// Include in the production folder_watch module under cfg(test). TestFolderWatch
// must wrap the actual production Win32 subscription, never a fake observer.
use super::*;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::Duration;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp").join("opencode")
            .join(format!("stack-external-watch-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&root).unwrap(); Self(root)
    }
    fn repo(&self, name: &str) -> PathBuf {
        let repo = self.0.join(name); std::fs::create_dir_all(repo.join("src")).unwrap();
        git(&repo, &["init", "-b", "main"]); git(&repo, &["config", "user.name", "Watch fixture"]);
        git(&repo, &["config", "user.email", "watch@example.invalid"]);
        std::fs::write(repo.join("tracked.txt"), "before\n").unwrap(); git(&repo, &["add", "."]); git(&repo, &["commit", "-m", "fixture"]); repo
    }
}
impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
fn git(cwd: &Path, args: &[&str]) {
    let output = std::process::Command::new("git").arg("-C").arg(cwd).args(args).env("GIT_TERMINAL_PROMPT", "0").output().unwrap();
    assert!(output.status.success(), "fixture git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
}
fn canonical(path: &Path) -> PathBuf { std::fs::canonicalize(path).unwrap() }
fn watch(path: &Path) -> (TestFolderWatch, mpsc::Receiver<()>) {
    let (tx, rx) = mpsc::channel();
    let subscription = TestFolderWatch::start(path, Arc::new(move || { let _ = tx.send(()); })).expect("actual native watcher must arm before return");
    (subscription, rx)
}
fn changed(rx: &mpsc::Receiver<()>, label: &str) { rx.recv_timeout(Duration::from_secs(3)).unwrap_or_else(|error| panic!("native watcher missed {label}: {error}")); }
fn quiet(rx: &mpsc::Receiver<()>, label: &str) { assert!(matches!(rx.recv_timeout(Duration::from_millis(400)), Err(mpsc::RecvTimeoutError::Timeout)), "unexpected native notification: {label}"); }

#[test]
fn real_edit_add_delete_rename_and_atomic_replace_notify() {
    let fixture = Fixture::new(); let repo = fixture.repo("repo");
    let (mut subscription, rx) = watch(&repo);
    std::fs::write(repo.join("tracked.txt"), "external edit\n").unwrap(); changed(&rx, "edit"); quiet(&rx, "duplicate edit");
    std::fs::write(repo.join("added.txt"), "added\n").unwrap(); changed(&rx, "add"); quiet(&rx, "duplicate add");
    std::fs::rename(repo.join("added.txt"), repo.join("renamed.txt")).unwrap(); changed(&rx, "rename"); quiet(&rx, "rename pair not coalesced");
    std::fs::remove_file(repo.join("renamed.txt")).unwrap(); changed(&rx, "delete"); quiet(&rx, "duplicate delete");
    std::fs::write(repo.join("replacement.tmp"), "atomic replacement\n").unwrap();
    std::fs::rename(repo.join("replacement.tmp"), repo.join("tracked.txt")).unwrap(); changed(&rx, "atomic replacement");
    subscription.stop();
}

#[test]
fn recursive_active_repository_observes_sibling_outside_visible_folder() {
    let fixture = Fixture::new(); let repo = fixture.repo("repo"); let visible = repo.join("src");
    let (mut subscription, rx) = watch(&visible);
    assert!(subscription.targets().contains(&canonical(&repo)), "native derives repo root, not just displayed folder");
    std::fs::write(repo.join("external-sibling.txt"), "LLM mutation outside src\n").unwrap(); changed(&rx, "repository sibling"); subscription.stop();
}

#[test]
fn nonrepository_watch_is_scoped_to_the_active_folder_not_its_parent() {
    let fixture = Fixture::new(); let folder = fixture.0.join("active-folder"); std::fs::create_dir_all(&folder).unwrap();
    let (mut subscription, rx) = watch(&folder); assert!(subscription.targets().contains(&canonical(&folder)));
    assert!(!subscription.targets().contains(&canonical(&fixture.0)));
    std::fs::write(fixture.0.join("outside.txt"), "outside\n").unwrap(); quiet(&rx, "nonrepo parent must not be watched");
    std::fs::write(folder.join("inside.txt"), "inside\n").unwrap(); changed(&rx, "nonrepo active folder"); subscription.stop();
}

#[test]
fn linked_worktree_watches_external_common_refs_and_working_tree() {
    let fixture = Fixture::new(); let repo = fixture.repo("main"); let linked = fixture.0.join("linked");
    git(&repo, &["worktree", "add", "-b", "linked", linked.to_str().unwrap()]); std::fs::create_dir_all(linked.join("src")).unwrap();
    let (mut subscription, rx) = watch(&linked.join("src")); let targets = subscription.targets();
    let common = canonical(&repo.join(".git"));
    assert!(targets.contains(&canonical(&linked)));
    assert!(targets.iter().any(|target| target == &common || target.starts_with(&common)), "linked worktree must observe actual common-dir metadata outside its root");
    git(&repo, &["update-ref", "refs/heads/external-agent", "HEAD"]); changed(&rx, "external common-dir branch ref"); quiet(&rx, "ref burst");
    std::fs::write(linked.join("tracked.txt"), "linked external edit\n").unwrap(); changed(&rx, "linked working file"); subscription.stop();
}

#[test]
fn burst_events_coalesce_and_status_queries_do_not_start_a_watch_scan_loop() {
    let fixture = Fixture::new(); let repo = fixture.repo("repo"); let (mut subscription, rx) = watch(&repo);
    for version in 0..8 { std::fs::write(repo.join("tracked.txt"), format!("external {version}\n")).unwrap(); }
    changed(&rx, "burst"); quiet(&rx, "burst must coalesce");
    for _ in 0..3 { query_watch_summary(&repo).expect("actual production summary read"); }
    quiet(&rx, "Git status/index-lock must not create self-sustaining refresh");
    std::fs::write(repo.join(".git/index.lock"), "temporary lock").unwrap(); std::fs::remove_file(repo.join(".git/index.lock")).unwrap();
    quiet(&rx, "index.lock-only churn"); subscription.stop();
}

#[test]
fn external_staging_index_change_remains_a_meaningful_notification() {
    let fixture = Fixture::new(); let repo = fixture.repo("repo");
    std::fs::write(repo.join("tracked.txt"), "external editor change before watching\n").unwrap();
    let (mut subscription, rx) = watch(&repo);
    git(&repo, &["add", "tracked.txt"]); changed(&rx, "external staging/index update"); quiet(&rx, "stage metadata burst");
    query_watch_summary(&repo).expect("summary after staging"); quiet(&rx, "read-only collector after staging must not trigger a loop"); subscription.stop();
}

#[test]
fn stop_and_drop_cancel_native_delivery() {
    let fixture = Fixture::new(); let repo = fixture.repo("repo");
    let (mut subscription, rx) = watch(&repo); subscription.stop(); subscription.stop();
    std::fs::write(repo.join("tracked.txt"), "after stop\n").unwrap();
    assert!(matches!(rx.recv_timeout(Duration::from_millis(350)), Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected)));
    let (subscription, rx) = watch(&repo); drop(subscription); std::fs::write(repo.join("tracked.txt"), "after drop\n").unwrap();
    assert!(matches!(rx.recv_timeout(Duration::from_millis(350)), Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected)));
}

#[cfg(windows)]
#[test]
fn repeated_native_watch_stop_releases_os_handles() {
    // Isolate this OS-wide handle oracle from the project's concurrently running
    // native tests. The child is this exact unit-test harness case, not the app.
    if std::env::var_os("JASONSHELL_WATCH_HANDLE_CHILD").is_none() {
        let name = std::thread::current().name().expect("named Rust test thread").to_owned();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name.as_str(), "--nocapture", "--test-threads=1"])
            .env("JASONSHELL_WATCH_HANDLE_CHILD", "1").output().unwrap();
        assert!(output.status.success(), "isolated watcher resource test failed: {} {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)); return;
    }
    fn handles() -> u32 {
        let mut count = 0;
        unsafe { windows::Win32::System::Threading::GetProcessHandleCount(windows::Win32::System::Threading::GetCurrentProcess(), &mut count).unwrap(); }
        count
    }
    let fixture = Fixture::new(); let repo = fixture.repo("repo");
    // Warm any process-wide Win32 setup before measuring repeated ownership.
    let (mut warm, _) = watch(&repo); warm.stop(); drop(warm);
    let before = handles();
    for _ in 0..8 {
        let (mut subscription, _) = watch(&repo); subscription.stop(); drop(subscription);
        let runtime = TestWatchRuntime::new(); runtime.start(&repo, Arc::new(|| {})).expect("actual runtime resource fixture");
        drop(runtime.retire());
    }
    let after = handles(); assert!(after <= before + 4, "native watch handles leaked across stop/drop: {before} -> {after}");
}

#[test]
fn generation_reservation_rejects_stale_stops_without_retiring_new_subscription() {
    let mut generation = WatchGeneration::default(); let old = generation.reserve(); let current = generation.reserve();
    assert_ne!(old, current); assert!(!generation.is_current(old)); assert!(generation.is_current(current));
    assert!(!generation.retire(old)); assert!(generation.is_current(current));
    assert!(generation.retire(current)); assert!(!generation.is_current(current));
}

#[cfg(windows)]
#[test]
fn recursive_watch_does_not_traverse_a_junction_outside_the_repository() {
    let fixture = Fixture::new(); let repo = fixture.repo("repo"); let outside = fixture.0.join("outside"); std::fs::create_dir_all(&outside).unwrap();
    let junction = repo.join("escape");
    let output = std::process::Command::new("cmd.exe").args(["/d", "/c", "mklink", "/J"])
        .arg(junction.to_string_lossy().replace('/', "\\")).arg(outside.to_string_lossy().replace('/', "\\")).output().unwrap();
    assert!(output.status.success(), "junction fixture failed: {}", String::from_utf8_lossy(&output.stderr));
    let (mut subscription, rx) = watch(&repo); std::fs::write(outside.join("escape.txt"), "outside mutation\n").unwrap(); quiet(&rx, "must not traverse junction");
    std::fs::write(repo.join("tracked.txt"), "inside mutation\n").unwrap(); changed(&rx, "real root remains watched"); subscription.stop(); std::fs::remove_dir(junction).unwrap();
}

#[test]
fn watch_commands_are_popup_only_and_deny_other_surface_callers() {
    use crate::stack_popup::auth::{allowed_stack_command_callers, authorize_stack_command_caller, STACK_GUARDED_COMMANDS};
    for command in [crate::contracts::commands::START_STACK_FOLDER_WATCH, crate::contracts::commands::STOP_STACK_FOLDER_WATCH] {
        let entry = STACK_GUARDED_COMMANDS.iter().copied().find(|entry| match entry { crate::stack_popup::StackCommandAuth::AllowedCallers { command: candidate, .. } | crate::stack_popup::StackCommandAuth::TerminalSessionTarget { command: candidate, .. } => *candidate == command }).expect("watch command must be in auth matrix");
        assert_eq!(allowed_stack_command_callers(entry), &[crate::contracts::surfaces::STACK_POPUP]);
        assert!(authorize_stack_command_caller(crate::contracts::surfaces::STACK_POPUP, entry).is_ok());
        assert!(authorize_stack_command_caller(crate::contracts::surfaces::TOP_BAR, entry).is_err());
        assert!(authorize_stack_command_caller("untrusted-renderer", entry).is_err());
    }
}

// This seam must share production admission, generation and install guards and
// own real ProductionWatch instances. Holding RetiredWatch delays the actual
// worker join/finish_retirement boundary without a GUI or fake watcher.
#[test]
fn runtime_reopen_waits_for_retirement_then_arms_one_real_watch() {
    let fixture = Fixture::new(); let repo = fixture.repo("repo");
    let runtime = Arc::new(TestWatchRuntime::new());
    let old = runtime.start(&repo, Arc::new(|| {})).expect("initial runtime start");
    let retirement = runtime.retire();
    assert!(retirement.is_some()); assert!(!runtime.is_current(old));
    let admitted = runtime.reserve_start().expect("normal reopen admission during retirement");
    assert!(runtime.reserve_start().is_err(), "only one pending start may be admitted");
    let (events_tx, events_rx) = mpsc::channel(); let (tx, rx) = mpsc::channel();
    let next_repo = repo.clone();
    let worker = std::thread::spawn(move || {
        let result = admitted.start(&next_repo, Arc::new(move || { let _ = events_tx.send(()); }));
        tx.send(result).unwrap();
    });
    assert!(matches!(rx.recv_timeout(Duration::from_millis(150)), Err(mpsc::RecvTimeoutError::Timeout)), "reopen must wait, not return busy or arm early");
    drop(retirement);
    let current = rx.recv_timeout(Duration::from_secs(3)).expect("retirement must wake reopen").expect("normal reopen must succeed without retry");
    worker.join().unwrap(); assert_ne!(old, current); assert!(runtime.is_current(current));
    std::fs::write(repo.join("tracked.txt"), "after actual runtime reopen\n").unwrap(); changed(&events_rx, "real watch after retirement wait");
    drop(runtime.retire()); assert!(!runtime.is_current(current));
}

#[test]
fn runtime_exit_wakes_waiting_start_without_resurrecting_worker() {
    let fixture = Fixture::new(); let repo = fixture.repo("repo");
    let runtime = Arc::new(TestWatchRuntime::new()); runtime.start(&repo, Arc::new(|| {})).unwrap();
    let retirement = runtime.retire(); let (tx, rx) = mpsc::channel();
    let admitted = runtime.reserve_start().unwrap(); let next_repo = repo.clone();
    let worker = std::thread::spawn(move || { tx.send(admitted.start(&next_repo, Arc::new(|| {}))).unwrap(); });
    assert!(matches!(rx.recv_timeout(Duration::from_millis(150)), Err(mpsc::RecvTimeoutError::Timeout)));
    drop(runtime.shutdown());
    assert!(rx.recv_timeout(Duration::from_secs(3)).expect("shutdown must wake waiter before held retirement completes").is_err());
    worker.join().unwrap(); drop(retirement);
    assert!(runtime.start(&repo, Arc::new(|| {})).is_err(), "shutdown cannot admit later starts");
}

#[test]
fn runtime_second_hide_cancels_waiting_reopen_without_resurrection() {
    let fixture = Fixture::new(); let repo = fixture.repo("repo");
    let runtime = Arc::new(TestWatchRuntime::new()); runtime.start(&repo, Arc::new(|| {})).unwrap();
    let retirement = runtime.retire(); let (tx, rx) = mpsc::channel();
    let admitted = runtime.reserve_start().unwrap(); let next_repo = repo.clone();
    let worker = std::thread::spawn(move || { tx.send(admitted.start(&next_repo, Arc::new(|| {}))).unwrap(); });
    assert!(matches!(rx.recv_timeout(Duration::from_millis(150)), Err(mpsc::RecvTimeoutError::Timeout)));
    drop(runtime.retire());
    assert!(rx.recv_timeout(Duration::from_secs(3)).expect("new hide must cancel and wake retired reopen").is_err());
    worker.join().unwrap(); drop(retirement);
    let current = runtime.start(&repo, Arc::new(|| {})).expect("cancelled waiter must not leak admission ownership");
    drop(runtime.retire()); assert!(!runtime.is_current(current));
}

#[test]
fn runtime_hide_invalidates_reserved_start_before_worker_begins() {
    let fixture = Fixture::new(); let repo = fixture.repo("repo"); let runtime = TestWatchRuntime::new();
    let admitted = runtime.reserve_start().expect("reserve before hide");
    assert!(runtime.reserve_start().is_err(), "admission is bounded before worker spawn");
    drop(runtime.retire());
    assert!(admitted.start(&repo, Arc::new(|| {})).is_err(), "pre-hide reserved start cannot install after hide even without an active watch");
    let current = runtime.start(&repo, Arc::new(|| {})).expect("stale startup must release admission");
    assert!(runtime.is_current(current)); drop(runtime.retire()); assert!(!runtime.is_current(current));
}
