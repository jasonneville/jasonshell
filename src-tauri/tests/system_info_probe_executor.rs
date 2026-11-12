//! Actual production executor against isolated test-owned helper children.
//! No Tauri builder/window, hardware enumeration, shell startup or user-process kill.
#![cfg(windows)]
#[path = "../src/process_manager/probe_executor.rs"]
mod probe_executor;
#[path = "../src/process_manager/system_info.rs"]
mod system_info;
use probe_executor::{execute_command_observed,ProbeError,ProbeEvent};
use std::{path::PathBuf,process::Command,sync::{mpsc,Arc,Mutex,OnceLock},time::{Duration,Instant}};
use windows::Win32::{Foundation::{CloseHandle,HANDLE,WAIT_OBJECT_0},System::Threading::{OpenProcess,WaitForSingleObject,PROCESS_SYNCHRONIZE}};

fn helper() -> &'static PathBuf {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        let directory = PathBuf::from(std::env::var_os("LOCALAPPDATA").expect("Windows local application data directory")).join("Temp/opencode");
        std::fs::create_dir_all(&directory).expect("test artifact directory");
        let output = directory.join(format!("system-info-probe-child-{}.exe",std::process::id()));
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/system_info_probe_child.rs");
        let built = Command::new("rustc").arg("--edition=2021").arg(source).arg("-o").arg(&output).output().expect("installed rustc required; no installation attempted");
        assert!(built.status.success(),"helper compile: {}",String::from_utf8_lossy(&built.stderr)); output
    })
}
struct ProcessWatch(HANDLE);
impl ProcessWatch {
    fn open(pid:u32) -> Self { Self(unsafe {OpenProcess(PROCESS_SYNCHRONIZE,false,pid)}.expect("hold actual child's process object before executor releases its handle")) }
    fn assert_exited(&self) { assert_eq!(unsafe {WaitForSingleObject(self.0,0)},WAIT_OBJECT_0,"actual child must be signaled/exited, not merely marked reaped"); }
}
impl Drop for ProcessWatch {fn drop(&mut self) {unsafe {let _=CloseHandle(self.0);}}}
struct Execution { result:Result<Vec<u8>,ProbeError>, events:Vec<ProbeEvent>, elapsed:Duration }
fn run(mode:&str,deadline:Duration,cap:usize) -> Execution {
    let mut events=Vec::new(); let mut watch=None; let started=Instant::now();
    let result=execute_command_observed(Command::new(helper()).arg(mode),||false,deadline,cap,|event| {
        if let ProbeEvent::Spawned(pid)=event {watch=Some(ProcessWatch::open(pid));} events.push(event);
    });
    watch.as_ref().expect("helper spawned").assert_exited();
    Execution {result,events,elapsed:started.elapsed()}
}
fn assert_reaped(events:&[ProbeEvent],killed:bool) {
    let pid=match events.first() {Some(ProbeEvent::Spawned(pid))=>*pid,other=>panic!("spawn event first: {other:?}")};
    let expected=if killed {vec![ProbeEvent::Spawned(pid),ProbeEvent::KillRequested(pid),ProbeEvent::BeforeWait(pid),ProbeEvent::Reaped(pid)]}
        else {vec![ProbeEvent::Spawned(pid),ProbeEvent::BeforeWait(pid),ProbeEvent::Reaped(pid)]};
    assert_eq!(events,expected,"kill/wait/reap order and exactly-once completion");
}

#[test]
fn actual_child_deadline_kills_waits_and_exits_even_without_stdout() {
    let execution=run("stall",Duration::from_millis(1500),8192);
    assert_eq!(execution.result,Err(ProbeError::TimedOut)); assert_reaped(&execution.events,true);
    assert!(execution.elapsed>=Duration::from_millis(1500)); assert!(execution.elapsed<Duration::from_secs(4),"deadline is not a blocking stdout read");
}
#[test]
fn stdout_flood_and_one_byte_over_cap_are_killed_and_reaped_not_returned() {
    for mode in ["flood","one-over"] {
        let execution=run(mode,Duration::from_millis(1500),8192);
        assert_eq!(execution.result,Err(ProbeError::OutputLimit),"{mode}"); assert_reaped(&execution.events,true);
        assert!(execution.elapsed<Duration::from_secs(4));
    }
}
#[test]
fn exact_output_cap_and_empty_output_succeed_without_unnecessary_kill() {
    for (mode,cap,length) in [("exact",8192,8192),("empty",0,0)] {
        let execution=run(mode,Duration::from_millis(1500),cap); assert_eq!(execution.result.unwrap().len(),length); assert_reaped(&execution.events,false);
    }
}
#[test]
fn nonzero_child_is_reaped_and_does_not_return_success_bytes() {
    let execution=run("nonzero",Duration::from_millis(1500),8192); assert_eq!(execution.result,Err(ProbeError::Exit)); assert_reaped(&execution.events,false);
}
#[test]
fn successful_and_malformed_output_use_actual_storage_wire_decoder() {
    let valid=run("valid",Duration::from_millis(1500),8192); assert_reaped(&valid.events,false);
    let root:system_info::StorageRoot=serde_json::from_slice(&valid.result.unwrap()).unwrap();
    assert_eq!(root.root,"C:\\"); assert_eq!(root.total_free_bytes.value,Some(700)); assert_eq!(root.caller_available_bytes.value,Some(300));
    let malformed=run("malformed",Duration::from_millis(1500),8192); assert_reaped(&malformed.events,false);
    assert!(serde_json::from_slice::<system_info::StorageRoot>(&malformed.result.unwrap()).is_err());
}
#[test]
fn pre_cancelled_and_spawn_failure_never_create_a_child() {
    let mut events=Vec::new();
    assert_eq!(execute_command_observed(&mut Command::new(helper()),||true,Duration::from_millis(1500),8192,|e|events.push(e)),Err(ProbeError::Cancelled));
    assert!(events.is_empty());
    assert_eq!(execute_command_observed(&mut Command::new(helper().with_file_name("nonexistent-system-info-test-helper.exe")),||false,Duration::from_millis(1500),8192,|e|events.push(e)),Err(ProbeError::Spawn));
    assert!(events.is_empty());
}

// Always release/join the observer barrier, including when an assertion panics.
struct WorkerGuard {release:Option<mpsc::Sender<()>>,thread:Option<std::thread::JoinHandle<Execution>>}
impl WorkerGuard {fn finish(mut self)->Execution {let _=self.release.take().unwrap().send(());self.thread.take().unwrap().join().unwrap()}}
impl Drop for WorkerGuard {fn drop(&mut self){if let Some(release)=self.release.take(){let _=release.send(());}if let Some(thread)=self.thread.take(){let _=thread.join();}}}
#[test]
fn retired_epoch_keeps_real_child_slot_through_before_wait_until_observed_reap() {
    use system_info::{RuntimeInner,ProcessManagerTab as Tab,Lane};
    let mut state=RuntimeInner::default(); let c=state.coordinator.open(None).unwrap();
    let c=state.coordinator.activate(c.open_generation,c.tab_revision+1,Tab::SystemInfo,true,true).unwrap(); state.warm(100).unwrap();
    let storage=state.coordinator.admit(Lane::Storage,c.open_generation,c.tab_revision,0,true,true).unwrap();
    let token=state.probes.reserve("C:\\",c.open_generation,c.tab_revision,0).unwrap();
    let other=state.probes.reserve("D:\\",c.open_generation,c.tab_revision,0).unwrap();
    let state=Arc::new(Mutex::new(state)); let worker_state=state.clone();
    let (events_tx,events_rx)=mpsc::channel(); let (release_tx,release_rx)=mpsc::channel();
    let executable=helper().clone();
    let thread=std::thread::spawn(move|| {
        let mut events=Vec::new(); let mut watch=None; let started=Instant::now();
        let result=execute_command_observed(Command::new(executable).arg("stall"),|| {
            let current=worker_state.lock().unwrap().coordinator.context(); !current.visible || current.open_generation!=c.open_generation || current.tab_revision!=c.tab_revision
        },Duration::from_millis(1500),8192,|event| {
            if let ProbeEvent::Spawned(pid)=event {watch=Some(ProcessWatch::open(pid));}
            events.push(event); let _=events_tx.send(event);
            if matches!(event,ProbeEvent::BeforeWait(_)) {let _=release_rx.recv_timeout(Duration::from_secs(2));}
        });
        watch.as_ref().unwrap().assert_exited(); Execution{result,events,elapsed:started.elapsed()}
    });
    let guard=WorkerGuard{release:Some(release_tx),thread:Some(thread)};
    assert!(matches!(events_rx.recv_timeout(Duration::from_secs(2)).unwrap(),ProbeEvent::Spawned(_)));
    {
        let mut state=state.lock().unwrap(); state.invalidate(); state.coordinator.hide().unwrap(); let next=state.coordinator.open(None).unwrap();
        let next=state.coordinator.activate(next.open_generation,next.tab_revision+1,Tab::SystemInfo,true,true).unwrap();state.warm(200).unwrap();
        assert!(!state.probes.may_publish(&token,c.open_generation,c.tab_revision));
        assert!(state.probes.reserve("E:\\",next.open_generation,next.tab_revision,1).is_none());
        assert!(state.coordinator.admit(Lane::Storage,next.open_generation,next.tab_revision,1,true,true).is_none());
        assert!(state.coordinator.admit(Lane::Live,next.open_generation,next.tab_revision,1,true,true).is_some());
    }
    assert!(matches!(events_rx.recv_timeout(Duration::from_secs(2)).unwrap(),ProbeEvent::KillRequested(_)));
    assert!(matches!(events_rx.recv_timeout(Duration::from_secs(2)).unwrap(),ProbeEvent::BeforeWait(_)));
    {
        let mut state=state.lock().unwrap();let next=state.coordinator.context();
        assert!(state.probes.reserve("C:\\",next.open_generation,next.tab_revision,2).is_none());
        assert!(state.probes.reserve("E:\\",next.open_generation,next.tab_revision,2).is_none(),"kill request/before-wait is not slot release");
    }
    let execution=guard.finish(); assert_eq!(execution.result,Err(ProbeError::Cancelled)); assert_reaped(&execution.events,true);
    let mut state=state.lock().unwrap();let next=state.coordinator.context();
    assert!(state.probes.reserve("E:\\",next.open_generation,next.tab_revision,3).is_none(),"executor completion alone does not release caller-owned reservation");
    assert!(state.probes.reaped(&token));assert!(!state.probes.reaped(&token));
    assert!(state.probes.reserve("C:\\",next.open_generation,next.tab_revision,3).is_some());assert!(state.probes.reaped(&other));
    assert!(!state.coordinator.complete(storage,3),"stale storage cannot publish");
    assert!(state.coordinator.admit(Lane::Storage,next.open_generation,next.tab_revision,3,true,true).is_some());
}
