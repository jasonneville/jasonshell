//! Isolated Windows subprocess fixture. No shell setup, AppBar, or power APIs.
use super::*;
use std::{
    os::windows::{io::{AsRawHandle, FromRawHandle, OwnedHandle}, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
};

const NAME: &str = "graceful_exit::owned_e2e::actual_exit_drains_owned_trees_with_stalled_cleanup";
const POWERSHELL: &str = "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe";

#[test]
fn quick_command_job_is_part_of_windows_process_creation() {
    let source = include_str!("../../src/quick_commands.rs");
    assert!(source.contains("PROC_THREAD_ATTRIBUTE_JOB_LIST"), "the owned job must be supplied to CreateProcessW, not assigned after it returns");
}

struct ChildGuard(Child);
struct FixtureDir(PathBuf);
impl FixtureDir {
    fn path(&self) -> &Path { &self.0 }
}
impl Drop for FixtureDir {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn require_marker(path: &Path, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while !path.exists() {
        assert!(Instant::now() < deadline, "fixture marker missing: {path:?}");
        thread::sleep(Duration::from_millis(10));
    }
}

fn observation(root: &Path, name: &str) -> OwnedHandle {
    let marker = root.join(format!("{name}.pid"));
    require_marker(&marker, Duration::from_secs(15));
    let pid = std::fs::read_to_string(marker).unwrap().parse::<u32>().unwrap();
    // Retain the kernel handle before allowing exit; no PID re-open/reuse oracle.
    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }.unwrap();
    let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };
    assert_eq!(wait(&handle, 0), WAIT_TIMEOUT, "{name} not alive before exit");
    handle
}

fn wait(handle: &OwnedHandle, millis: u32) -> windows::Win32::Foundation::WAIT_EVENT {
    unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), millis) }
}

fn quote(path: &Path) -> String { format!("'{}'", path.to_string_lossy().replace('\'', "''")) }

fn driver(app: AppHandle, root: &Path) {
    use crate::quick_commands::{spawn_quick_command, QuickCommandSpawnPlan};
    let mut ordinary = None;
    for (extension, verbatim) in [("cmd", false), ("bat", false), ("cmd", true), ("bat", true)] {
        let script = root.join(format!("direct batch {verbatim}.{extension}"));
        let marker = root.join(format!("batch-{extension}-{verbatim}.txt"));
        std::fs::write(&script, "@echo off\r\n>\"%~2\" echo [%~1]\r\necho fixture-stdout\r\n>&2 echo fixture-stderr\r\n").unwrap();
        let plan = QuickCommandSpawnPlan {
            label: extension.into(), executable: if verbatim {
                let canonical = script.canonicalize().unwrap();
                let path = canonical.to_str().unwrap();
                assert!(path.starts_with(r"\\?\") && path.as_bytes().get(5) == Some(&b':'), "expected a local verbatim drive path: {path}");
                path.into()
            } else { script.to_string_lossy().into_owned() },
            cwd: Some(root.to_string_lossy().into_owned()),
            args: vec!["two words".into(), marker.to_string_lossy().into_owned()],
        };
        let (done, completed) = mpsc::channel();
        let (control, control_child) = mpsc::channel();
        crate::quick_commands::set_before_contained_create_for_test(move || {
            let child = Command::new(POWERSHELL).args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 20"])
                .creation_flags(0x08000000).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
            assert!(control.send(ChildGuard(child)).is_ok());
        });
        spawn_quick_command(app.clone(), &plan, extension.into(), format!("{extension}-{verbatim}"), move |history| { let _ = done.send(history); })
            .unwrap_or_else(|error| panic!("direct .{extension}, verbatim={verbatim}: launch failed: {error}"));
        ordinary = Some(control_child.recv_timeout(Duration::from_secs(1)).unwrap());
        let history = completed.recv_timeout(Duration::from_secs(5)).expect("both output pipes must reach EOF while ordinary spawn lives");
        assert_eq!(history.exit_code, Some(0), "direct .{extension}, verbatim={verbatim}: stdout={:?}, stderr={:?}", history.stdout, history.stderr);
        assert!(history.stdout.contains("fixture-stdout"), "direct .{extension}: stdout={:?}, stderr={:?}", history.stdout, history.stderr);
        assert!(history.stderr.contains("fixture-stderr"), "direct .{extension}: stderr={:?}", history.stderr);
        assert!(ordinary.as_mut().unwrap().0.try_wait().unwrap().is_none());
        assert_eq!(std::fs::read_to_string(marker).unwrap().trim(), "[two words]");
    }
    for tree in ["live", "root-first"] {
        for node in ["grandchild", "child", "root"] {
            let name = format!("{tree}-{node}");
            if tree == "live" && node == "grandchild" {
                let marker = root.join("late.pid");
                std::fs::write(root.join("late.ps1"), format!("[IO.File]::WriteAllText({}, [string]$PID)\n[IO.File]::Move({}, {})\nStart-Sleep -Seconds 25", quote(&root.join("late.tmp")), quote(&root.join("late.tmp")), quote(&marker))).unwrap();
            }
            let next = match node { "root" => Some("child"), "child" => Some("grandchild"), "grandchild" if tree == "live" => Some("late"), _ => None };
            let spawn = next.map(|next| {
                let script = if next == "late" { root.join("late.ps1") } else { root.join(format!("{tree}-{next}.ps1")) };
                let gate = if next == "late" { format!("while (!(Test-Path {})) {{ Start-Sleep -Milliseconds 10 }}\n", quote(&root.join("spawn-late"))) } else { String::new() };
                format!("{gate}$p = New-Object Diagnostics.ProcessStartInfo\n$p.FileName = '{POWERSHELL}'\n$p.Arguments = '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"{}\"'\n$p.UseShellExecute = $false\n$p.CreateNoWindow = $true\n[void][Diagnostics.Process]::Start($p)\n", script.to_string_lossy().replace('\'', "''"))
            }).unwrap_or_default();
            let marker = root.join(format!("{name}.pid"));
            let partial = root.join(format!("{name}.tmp"));
            let sleep = if name == "root-first-root" {
                format!("$until=(Get-Date).AddSeconds(25); while (!(Test-Path {}) -and (Get-Date) -lt $until) {{ Start-Sleep -Milliseconds 10 }}", quote(&root.join("release-root")))
            } else { "Start-Sleep -Seconds 25".into() };
            std::fs::write(root.join(format!("{name}.ps1")), format!("[IO.File]::WriteAllText({}, [string]$PID)\n[IO.File]::Move({}, {})\n{spawn}{sleep}", quote(&partial), quote(&partial), quote(&marker))).unwrap();
        }
        let plan = QuickCommandSpawnPlan {
            label: tree.into(), executable: POWERSHELL.into(), cwd: Some(root.to_string_lossy().into()),
            args: ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]
                .into_iter().map(str::to_owned).chain([root.join(format!("{tree}-root.ps1")).to_string_lossy().into_owned()]).collect(),
        };
        if tree == "root-first" {
            let barrier = root.to_owned();
            crate::quick_commands::set_before_root_exit_drain_for_test(move || {
                std::fs::write(barrier.join("root-worker-parked"), b"").unwrap();
                loop { thread::park(); }
            });
        }
        spawn_quick_command(app.clone(), &plan, tree.into(), tree.into(), |_| {}).unwrap();
    }
    let script = root.join("race.ps1");
    std::fs::write(&script, format!("[IO.File]::WriteAllText({}, 'executed')", quote(&root.join("race-executed")))).unwrap();
    let launch_app = app.clone();
    let launch_root = root.to_owned();
    let launch = thread::spawn(move || {
        let barrier_root = launch_root.clone();
        crate::quick_commands::set_after_suspended_create_for_test(move |pid| {
            std::fs::write(barrier_root.join("race.tmp"), pid.to_string()).unwrap();
            std::fs::rename(barrier_root.join("race.tmp"), barrier_root.join("race.pid")).unwrap();
            require_marker(&barrier_root.join("release-race"), Duration::from_secs(20));
        });
        spawn_quick_command(launch_app, &QuickCommandSpawnPlan {
            label: "race".into(), executable: POWERSHELL.into(), cwd: Some(launch_root.to_string_lossy().into()),
            args: ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]
                .into_iter().map(str::to_owned).chain([script.to_string_lossy().into_owned()]).collect(),
        }, "race".into(), "race".into(), |_| {})
    });
    require_marker(&root.join("accept"), Duration::from_secs(20));
    let state = app.state::<ExitCoordinator>();
    let cleanup_root = root.to_owned();
    state.request(
        || loop { thread::park(); },
        move |deadline| {
            require_marker(&cleanup_root.join("allow-drain"), Duration::from_secs(2));
            let barrier = cleanup_root.clone();
            crate::quick_commands::set_before_exit_job_termination_for_test(move || {
                std::fs::write(barrier.join("spawn-late"), b"").unwrap();
                require_marker(&barrier.join("late-observed"), Duration::from_secs(2));
            });
            assert_eq!(crate::quick_commands::drain_owned_quick_commands(deadline).unwrap_err(), "Owned command job termination could not be verified");
            std::fs::write(cleanup_root.join("drained"), b"").unwrap();
            loop { thread::park(); }
        },
        terminate_current_process,
    ).unwrap();
    std::fs::write(root.join("accepted"), b"").unwrap();
    std::fs::write(root.join("release-race"), b"").unwrap();
    let error = launch.join().unwrap().expect_err("exit must win before the suspended root resumes");
    assert_eq!(error, "JasonShell is exiting");
    std::fs::write(root.join("race-rejected"), b"").unwrap();
    std::fs::write(root.join("allow-drain"), b"").unwrap();
    thread::sleep(Duration::from_millis(1500));
    state.request(|| panic!("duplicate restore"), |_| panic!("duplicate cleanup"), || panic!("duplicate exit")).unwrap();
    std::fs::write(root.join("duplicate"), b"").unwrap();
    drop(ordinary);
}

#[test]
#[ignore = "explicit isolated fixture; terminates only its own test subprocess and owned process tree"]
fn actual_exit_drains_owned_trees_with_stalled_cleanup() {
    assert_eq!(std::env::var("JASONSHELL_GRACEFUL_EXIT_OWNED_TESTS").as_deref(), Ok("1"));
    if let Some(root) = std::env::var_os("JASONSHELL_OWNED_EXIT_CHILD") {
        let root = std::path::PathBuf::from(root);
        assert!(root.join("parent-owner").is_file());
        let mut context = tauri::generate_context!();
        context.config_mut().identifier = "com.jnev1.jasonshell.owned-exit-fixture".into();
        assert!(context.config().app.windows.is_empty());
        let app = tauri::Builder::default().any_thread().manage(ExitCoordinator::default())
            .build(context).unwrap();
        let handle = app.handle().clone();
        thread::spawn(move || driver(handle, &root));
        // Intentionally do not pump the Tauri loop or call AppHandle::exit.
        loop { thread::park(); }
    }
    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let root = FixtureDir(std::env::temp_dir().join("opencode").join(format!("owned-exit-{}-{id}", std::process::id())));
    std::fs::create_dir(root.path()).unwrap();
    std::fs::write(root.path().join("parent-owner"), b"").unwrap();
    let mut independent = ChildGuard(Command::new(POWERSHELL).args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 30"])
        .creation_flags(0x08000000).stdin(Stdio::null()).spawn().unwrap());
    let mut child = ChildGuard(Command::new(std::env::current_exe().unwrap()).args(["--ignored", "--exact", NAME, "--nocapture"])
        .env("JASONSHELL_OWNED_EXIT_CHILD", root.path()).spawn().unwrap());
    let handles = ["live-root", "live-child", "live-grandchild", "root-first-root", "root-first-child", "root-first-grandchild", "race"]
        .map(|node| observation(root.path(), node));
    std::fs::write(root.path().join("release-root"), b"").unwrap();
    assert_eq!(wait(&handles[3], 3000), WAIT_OBJECT_0);
    require_marker(&root.path().join("root-worker-parked"), Duration::from_secs(2));
    for handle in &handles[4..6] { assert_eq!(wait(handle, 0), WAIT_TIMEOUT, "root-first descendant must survive until shutdown drain"); }
    let start = Instant::now();
    std::fs::write(root.path().join("accept"), b"").unwrap();
    require_marker(&root.path().join("accepted"), Duration::from_millis(500));
    require_marker(&root.path().join("race-rejected"), Duration::from_secs(2));
    assert_eq!(wait(&handles[6], 500), WAIT_OBJECT_0, "suspended launch must unwind");
    assert!(!root.path().join("race-executed").exists());
    let late = observation(root.path(), "late");
    std::fs::write(root.path().join("late-observed"), b"").unwrap();
    require_marker(&root.path().join("drained"), Duration::from_secs(2));
    assert_eq!(wait(&late, 100), WAIT_OBJECT_0, "late owned process must still be terminated");
    for handle in &handles[..6] {
        assert_eq!(wait(handle, 0), WAIT_OBJECT_0, "owned process must signal before watchdog exit");
    }
    assert!(child.0.try_wait().unwrap().is_none(), "watchdog must still be pending after observed drain");
    assert!(start.elapsed() < DEADLINE, "drain missed first acceptance deadline");
    while child.0.try_wait().unwrap().is_none() {
        assert!(start.elapsed() < Duration::from_millis(4000), "watchdog did not honor the first acceptance deadline");
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(child.0.wait().unwrap().code(), Some(0));
    assert!(root.path().join("duplicate").is_file());
    assert!(root.path().join("drained").is_file());
    assert!(root.path().join("race-rejected").is_file());
    assert!(!root.path().join("race-executed").exists());
    for handle in handles { assert_eq!(wait(&handle, 500), WAIT_OBJECT_0); }
    assert!(independent.0.try_wait().unwrap().is_none());
}
