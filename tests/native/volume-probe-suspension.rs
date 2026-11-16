//! Actual volume collector cancellation closure + actual executor; only roots/executable are test ports.
#![allow(dead_code, unused_imports)]
use std::{process::Command, sync::{Arc, Mutex, OnceLock}, time::{Duration, Instant}, path::PathBuf, cell::RefCell};
/* ACTUAL_MODULES */
use system_info::*;
use probe_executor::{ProbeError, ProbeEvent, execute_command_observed};
use windows::Win32::{Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0}, System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE}};
struct ProcessWatch(HANDLE);
impl ProcessWatch {
    fn open(pid: u32) -> Self { Self(unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }.unwrap()) }
    fn assert_exited(&self) { assert_eq!(unsafe { WaitForSingleObject(self.0, 0) }, WAIT_OBJECT_0, "test-owned helper must actually exit"); }
}
impl Drop for ProcessWatch { fn drop(&mut self) { unsafe { let _ = CloseHandle(self.0); } } }
fn now() -> u64 { 100 }
fn roots() -> Vec<String> { vec!["C:\\".into()] }
/* ACTUAL_UNAVAILABLE */
/* ACTUAL_COLLECT */

#[derive(Clone, Copy)] enum Mode { BeforeSpawn, SuspendRunning, RetireProbeRunning, CurrentValid, CurrentStall }
struct Hook {
    mode: Mode, state: Arc<Mutex<RuntimeInner>>, calls: usize,
    outcome: Option<String>, events: Vec<ProbeEvent>,
    slot_held_before_wait: bool, slot_held_after_reap: bool,
}
thread_local! { static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) }; }
fn helper() -> &'static PathBuf {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("probe-child.exe");
        let built = Command::new("rustc").arg("--edition=2021").arg(/* HELPER_SOURCE */).arg("-o").arg(&output).output().unwrap();
        assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr)); output
    })
}
fn execute_command(_: &mut Command, cancelled: impl FnMut() -> bool, deadline: Duration, cap: usize) -> Result<Vec<u8>, ProbeError> {
    let (mode, state) = HOOK.with(|slot| { let mut value = slot.borrow_mut(); let hook = value.as_mut().unwrap(); hook.calls += 1; (hook.mode, hook.state.clone()) });
    if matches!(mode, Mode::BeforeSpawn) { return Err(ProbeError::Cancelled); }
    let mut watch = None;
    let child_mode = if matches!(mode, Mode::CurrentValid) { "valid" } else { "stall" };
    let result = execute_command_observed(Command::new(helper()).arg(child_mode), cancelled, deadline, cap, |event| {
        if let ProbeEvent::Spawned(pid) = event {
            watch = Some(ProcessWatch::open(pid));
            let mut runtime = state.lock().unwrap();
            match mode { Mode::SuspendRunning => runtime.suspend(), Mode::RetireProbeRunning => runtime.invalidate(), Mode::CurrentValid | Mode::CurrentStall => {}, _ => unreachable!() }
        }
        let held = if matches!(event, ProbeEvent::BeforeWait(_) | ProbeEvent::Reaped(_)) {
            let mut runtime = state.lock().unwrap(); let context = runtime.coordinator.context();
            runtime.probes.reserve("E:\\", context.open_generation, context.tab_revision, 1).is_none()
        } else { false };
        HOOK.with(|slot| { let mut value = slot.borrow_mut(); let hook = value.as_mut().unwrap();
            hook.events.push(event);
            if matches!(event, ProbeEvent::BeforeWait(_)) { hook.slot_held_before_wait = held; }
            if matches!(event, ProbeEvent::Reaped(_)) { hook.slot_held_after_reap = held; }
        });
    });
    watch.as_ref().expect("helper spawned").assert_exited();
    HOOK.with(|slot| slot.borrow_mut().as_mut().unwrap().outcome = Some(format!("{result:?}")));
    result
}
fn active() -> (Arc<Mutex<RuntimeInner>>, WorkToken) {
    let mut state = RuntimeInner::default(); let c = state.coordinator.open(None).unwrap();
    let c = state.coordinator.activate(c.open_generation, c.tab_revision + 1, ProcessManagerTab::SystemInfo, true, true).unwrap();
    state.warm(100).unwrap();
    let work = state.coordinator.admit(Lane::Storage, c.open_generation, c.tab_revision, 0, true, true).unwrap();
    (Arc::new(Mutex::new(state)), work)
}
fn install(mode: Mode, state: Arc<Mutex<RuntimeInner>>) {
    HOOK.with(|slot| *slot.borrow_mut() = Some(Hook { mode, state, calls: 0, outcome: None, events: vec![], slot_held_before_wait: false, slot_held_after_reap: false }));
}
#[test]
fn suspended_storage_work_cannot_reserve_or_reach_executor() {
    let (state, work) = active();
    state.lock().unwrap().suspend(); install(Mode::BeforeSpawn, state.clone());
    collect(&state, Instant::now(), work);
    let hook = HOOK.with(|slot| slot.borrow_mut().take().unwrap());
    let mut runtime = state.lock().unwrap(); let context = runtime.coordinator.context();
    let next = runtime.probes.reserve("E:\\", context.open_generation, context.tab_revision, 0).unwrap();
    let expected = ProbeBudget::default().reserve("E:\\", context.open_generation, context.tab_revision, 0).unwrap();
    assert_eq!(next, expected, "suspended collector consumed a probe reservation ID before spawn");
    assert_eq!(hook.calls, 0, "suspended WorkToken reached executor");
}
#[test]
fn old_work_cannot_borrow_replacement_token_with_same_context_and_lane() {
    let (state, old) = active();
    let context = state.lock().unwrap().coordinator.context();
    let current = {
        let mut runtime = state.lock().unwrap(); runtime.suspend(); runtime.warm(200).unwrap();
        runtime.coordinator.admit(Lane::Storage, context.open_generation, context.tab_revision, 0, true, true).unwrap()
    };
    assert_ne!(old, current);
    assert!(!state.lock().unwrap().coordinator.owns_work(old));
    assert!(state.lock().unwrap().coordinator.owns_work(current));
    install(Mode::BeforeSpawn, state.clone()); collect(&state, Instant::now(), old);
    let hook = HOOK.with(|slot| slot.borrow_mut().take().unwrap());
    assert_eq!(hook.calls, 0, "same-context replacement must not authorize old token ID");
    let next = state.lock().unwrap().probes.reserve("E:\\", context.open_generation, context.tab_revision, 0).unwrap();
    assert_eq!(next, ProbeBudget::default().reserve("E:\\", context.open_generation, context.tab_revision, 0).unwrap());
}
fn running(mode: Mode) {
    let (state, work) = active();
    let context = state.lock().unwrap().coordinator.context();
    let other = state.lock().unwrap().probes.reserve("D:\\", context.open_generation, context.tab_revision, 0).unwrap();
    install(mode, state.clone()); collect(&state, Instant::now(), work);
    let hook = HOOK.with(|slot| slot.borrow_mut().take().unwrap());
    assert_eq!(hook.calls, 1); assert_eq!(hook.events.len(), 4);
    let pid = match hook.events[0] { ProbeEvent::Spawned(pid) => pid, _ => panic!("child must spawn") };
    assert_eq!(hook.events, vec![ProbeEvent::Spawned(pid), ProbeEvent::KillRequested(pid), ProbeEvent::BeforeWait(pid), ProbeEvent::Reaped(pid)]);
    assert!(hook.slot_held_before_wait && hook.slot_held_after_reap, "reservation released before collector observed actual reap");
    let mut runtime = state.lock().unwrap();
    assert_eq!(runtime.coordinator.context(), context, "suspend/retirement must retain context");
    assert!(runtime.probes.reserve("E:\\", context.open_generation, context.tab_revision, 1).is_some(), "collector must release its reservation after executor reap");
    assert!(runtime.probes.reaped(&other));
    assert_eq!(hook.outcome.as_deref(), Some("Err(Cancelled)"), "invalid work/retired probe must cancel promptly, not wait for deadline");
}
#[test] fn suspension_during_child_cancels_kills_waits_reaps_without_early_slot_release() { running(Mode::SuspendRunning); }
#[test] fn retired_probe_cancels_even_when_work_context_still_matches() { running(Mode::RetireProbeRunning); }
#[test]
fn current_work_and_probe_publish_valid_child_without_cancellation_or_kill() {
    let (state, work) = active(); let context = state.lock().unwrap().coordinator.context();
    let other = state.lock().unwrap().probes.reserve("D:\\", context.open_generation, context.tab_revision, 0).unwrap();
    install(Mode::CurrentValid, state.clone()); let output = collect(&state, Instant::now(), work);
    let hook = HOOK.with(|slot| slot.borrow_mut().take().unwrap());
    let pid = match hook.events[0] { ProbeEvent::Spawned(pid) => pid, _ => panic!("current helper must spawn") };
    assert_eq!(hook.calls, 1);
    assert_eq!(hook.events, vec![ProbeEvent::Spawned(pid), ProbeEvent::BeforeWait(pid), ProbeEvent::Reaped(pid)]);
    assert!(hook.slot_held_before_wait && hook.slot_held_after_reap);
    assert!(hook.outcome.as_deref().unwrap().starts_with("Ok("));
    assert_eq!(output.roots.len(), 1); assert_eq!(output.roots[0].root, "C:\\");
    assert!(matches!(output.state, SampleState::Ready));
    assert_eq!(output.roots[0].total_free_bytes.value, Some(700));
    let mut runtime = state.lock().unwrap();
    assert!(runtime.coordinator.owns_work(work));
    assert!(runtime.probes.reserve("E:\\", context.open_generation, context.tab_revision, 1).is_some());
    assert!(runtime.probes.reaped(&other));
}
#[test]
fn current_stalled_work_still_times_out_and_reaps_without_false_cancellation() {
    let (state, work) = active(); let context = state.lock().unwrap().coordinator.context();
    state.lock().unwrap().probes.reserve("D:\\", context.open_generation, context.tab_revision, 0).unwrap();
    install(Mode::CurrentStall, state.clone()); let output = collect(&state, Instant::now(), work);
    let hook = HOOK.with(|slot| slot.borrow_mut().take().unwrap());
    let pid = match hook.events[0] { ProbeEvent::Spawned(pid) => pid, _ => panic!("current helper must spawn") };
    assert_eq!(hook.events, vec![ProbeEvent::Spawned(pid), ProbeEvent::KillRequested(pid), ProbeEvent::BeforeWait(pid), ProbeEvent::Reaped(pid)]);
    assert!(hook.slot_held_before_wait && hook.slot_held_after_reap);
    assert_eq!(hook.outcome.as_deref(), Some("Err(TimedOut)"));
    assert_eq!(output.roots.len(), 1);
    assert!(matches!(output.roots[0].total_bytes.reason, Some(SampleReason::TimedOut)));
}
