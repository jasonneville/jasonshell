//! Tier 1 RS-31: taskbar snapshot must do cheap window filtering before expensive
//! title/class/process/icon resolution and must not re-emit identical snapshots.
//!
//! Wiring (end of `task_windows/windows.rs`, after the existing `mod tests`):
//! ```ignore
//! #[cfg(test)]
//! #[path = "windows_tier1_tests.rs"]
//! mod tier1_tests;
//! ```
//!
//! Required seams. windows.rs (items `pub(super)` or private; this file is a child):
//! ```ignore
//! /// Flags obtainable without title/class/process-path/icon work. `ex_style` is the raw
//! /// WS_EX_* bits. `is_shell_process` = pid == current pid.
//! #[derive(Clone, Copy, Debug)]
//! struct CheapWindowFlags {
//!     is_visible: bool, is_minimized: bool, has_owner: bool, is_cloaked: bool,
//!     is_shell_process: bool, ex_style: u32,
//! }
//! /// Expensive details (title via GetWindowText, class, process path+name).
//! struct WindowDetail {
//!     title: String, class_name: String, process_name: String, process_path: Option<PathBuf>,
//! }
//! /// Final candidacy == `is_taskbar_candidate` for every input. `detail` MUST NOT be
//! /// called when a cheap flag already rejects; it is called at most once otherwise.
//! /// Returns the detail when accepted (so callers do not resolve it twice).
//! fn resolve_taskbar_candidacy(
//!     flags: &CheapWindowFlags,
//!     detail: impl FnOnce() -> WindowDetail,
//! ) -> Option<WindowDetail>;
//!
//! /// Pure comparator: true iff `next` differs from `prev` in ANY emitted field
//! /// (hwnd, title, process_id, process_name, icon_data_url, is_active, is_minimized,
//! /// activity_state, notification_count, attention_state, toast_count) or in order/length.
//! /// `prev == None` => true. Ignores sequence and timestamps.
//! fn should_emit_taskbar_snapshot(prev: Option<&[TaskbarWindow]>, next: &[TaskbarWindow]) -> bool;
//!
//! /// Pure publisher state used by refresh_taskbar_snapshot_now.
//! /// publish(): emit==true  => sequence += 1, `snapshot.sequence` = new sequence.
//! ///            emit==false => sequence unchanged, `snapshot.sequence` = last emitted sequence
//! ///            and `snapshot.windows` = the (equal) new windows. The caller ALWAYS stores
//! ///            `snapshot` into LAST_TASKBAR_SNAPSHOT and ALWAYS refreshes
//! ///            LAST_TASKBAR_SNAPSHOT_AT (so max-age 1120 ms freshness, diagnostics
//! ///            refreshed_at, and list_open_task_windows keep working); only `app.emit`
//! ///            is skipped when emit == false.
//! #[derive(Default)]
//! struct SnapshotPublisher { .. }
//! struct PublishOutcome { snapshot: TaskbarWindowsSnapshot, emit: bool }
//! impl SnapshotPublisher {
//!     fn new(start_sequence: u64) -> Self;
//!     fn publish(&mut self, windows: Vec<TaskbarWindow>) -> PublishOutcome;
//! }
//!
//! /// Process identity cache keyed by (pid, creation FILETIME ticks): pid reuse => miss.
//! struct ProcessInfoCache { .. }
//! impl ProcessInfoCache {
//!     fn new(capacity: usize) -> Self;
//!     fn get(&self, pid: u32, creation_time: u64) -> Option<(Option<PathBuf>, String)>;
//!     fn insert(&mut self, pid: u32, creation_time: u64, path: Option<PathBuf>, name: String);
//!     fn len(&self) -> usize;           // never exceeds capacity
//! }
//! ```
//! icons.rs:
//! ```ignore
//! /// Short backoff for hung windows (production: 5 s) keyed by (hwnd, pid).
//! pub(super) struct HungIconBackoff { .. }
//! impl HungIconBackoff {
//!     pub(super) fn new(ttl: Duration) -> Self;
//!     /// true while now < recorded_at + ttl for the same (hwnd, pid); false afterwards
//!     /// (retried, never permanently lost) and for any other hwnd/pid (HWND reuse).
//!     pub(super) fn should_skip(&self, hwnd: isize, pid: u32, now: Instant) -> bool;
//!     pub(super) fn record_unresponsive(&mut self, hwnd: isize, pid: u32, now: Instant);
//!     pub(super) fn record_responsive(&mut self, hwnd: isize, pid: u32);
//!     pub(super) fn len(&self) -> usize; // pruned/bounded (prune expired on record)
//! }
//! ```
//! A skipped lookup must NOT write the icon cache (so the retry after ttl resolves the real
//! icon) and returns the previously cached icon for that window if any, else the process
//! file icon / EMPTY_ICON_DATA_URL fallback.
//!
//! Emit targeting consumer proof (e2e-runner read): `taskbar:windows-snapshot` listeners are
//! exactly TopBar.svelte (:2020) and BottomBar.svelte (:1270) (grep over src/ + src-tauri/src);
//! both drop `sequence <= last`, and both catch up on mount via the pull command
//! `list_open_task_windows` (TopBar :540, BottomBar :471) which reads LAST_TASKBAR_SNAPSHOT
//! (<=1120 ms old) or forces a scan. No heartbeat dependency. Keep the broadcast `app.emit`
//! unless the fixer re-greps and proves nothing else (TaskPreview/TaskGallery are fed by
//! BottomBar, not by this event); do not change to emit_to without that proof.

use super::{
    is_taskbar_candidate, resolve_taskbar_candidacy, should_emit_taskbar_snapshot,
    CheapWindowFlags, PublishOutcome, SnapshotPublisher, ProcessInfoCache, WindowCandidate,
    WindowDetail,
};
use crate::task_windows::icons::HungIconBackoff;
use crate::task_windows::{
    TaskbarWindow, TaskbarWindowActivityState, TaskbarWindowAttentionState,
};
use std::cell::Cell;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    WINDOW_EX_STYLE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

// ---------------------------------------------------------------- cheap candidacy

fn detail(title: &str, class: &str, process: &str) -> WindowDetail {
    WindowDetail {
        title: title.to_string(),
        class_name: class.to_string(),
        process_name: process.to_string(),
        process_path: if process.is_empty() {
            None
        } else {
            Some(PathBuf::from(format!("C:\\x\\{process}.exe")))
        },
    }
}

fn candidate(flags: &CheapWindowFlags, d: &WindowDetail) -> WindowCandidate {
    WindowCandidate {
        class_name: d.class_name.clone(),
        title: d.title.clone(),
        process_name: d.process_name.clone(),
        process_path: d.process_path.clone(),
        hwnd: HWND(std::ptr::null_mut()),
        is_active: false,
        is_minimized: flags.is_minimized,
        has_owner: flags.has_owner,
        is_cloaked: flags.is_cloaked,
        is_shell_process: flags.is_shell_process,
        is_visible: flags.is_visible,
        ex_style: WINDOW_EX_STYLE(flags.ex_style),
        process_id: if flags.is_shell_process { 4242 } else { 1111 },
    }
}

#[test]
fn cheap_rejection_never_resolves_details_and_candidacy_matches_is_taskbar_candidate() {
    let styles = [
        0u32,
        WS_EX_TOOLWINDOW.0,
        WS_EX_TOOLWINDOW.0 | WS_EX_APPWINDOW.0,
        WS_EX_NOACTIVATE.0,
        WS_EX_APPWINDOW.0,
    ];
    let details = [
        detail("Notepad", "Notepad", "notepad"),
        detail("", "SomeClass", "app"),
        detail("", "SomeClass", ""),
        detail("Shell", "Shell_TrayWnd", "explorer"),
        detail("Wallpaper", "WorkerW", "explorer"),
        detail("DWM Notification Window", "Dwm", "dwm"),
        detail("Notification", "SomeClass", ""),
        detail("Notification", "xDwmx", "other"),
        detail("Title", "C", "DWM"),
    ];
    let mut cheap_rejects = 0;
    let mut accepted = 0;
    for is_visible in [false, true] {
        for is_minimized in [false, true] {
            for has_owner in [false, true] {
                for is_cloaked in [false, true] {
                    for is_shell_process in [false, true] {
                        for ex_style in styles {
                            let flags = CheapWindowFlags {
                                is_visible,
                                is_minimized,
                                has_owner,
                                is_cloaked,
                                is_shell_process,
                                ex_style,
                            };
                            let tool = ex_style & WS_EX_TOOLWINDOW.0 != 0
                                && ex_style & WS_EX_APPWINDOW.0 == 0;
                            let cheap_ok = (is_visible || is_minimized)
                                && !is_shell_process
                                && !has_owner
                                && !is_cloaked
                                && !tool
                                && ex_style & WS_EX_NOACTIVATE.0 == 0;
                            for d in &details {
                                let calls = Cell::new(0u32);
                                let got = resolve_taskbar_candidacy(&flags, || {
                                    calls.set(calls.get() + 1);
                                    detail(&d.title, &d.class_name, &d.process_name)
                                });
                                let expected = is_taskbar_candidate(&candidate(&flags, d), 4242);
                                assert_eq!(
                                    got.is_some(),
                                    expected,
                                    "flags={flags:?} detail={d:?}"
                                );
                                if !cheap_ok {
                                    assert_eq!(calls.get(), 0, "detail resolved for {flags:?}");
                                    cheap_rejects += 1;
                                } else {
                                    assert_eq!(calls.get(), 1, "detail resolved once for {flags:?}");
                                }
                                if let Some(out) = got {
                                    assert_eq!(out.title, d.title);
                                    accepted += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(cheap_rejects > 0 && accepted > 0, "grid must cover both outcomes");
}

// ---------------------------------------------------------------- emit comparator

fn win(hwnd: &str) -> TaskbarWindow {
    TaskbarWindow {
        hwnd: hwnd.to_string(),
        title: "T".to_string(),
        process_id: 10,
        process_name: "app".to_string(),
        icon_data_url: "data:image/png;base64,AAAA".to_string(),
        is_active: false,
        is_minimized: false,
        activity_state: TaskbarWindowActivityState::Idle,
        notification_count: 0,
        attention_state: TaskbarWindowAttentionState::Idle,
        toast_count: 0,
    }
}

#[test]
fn identical_snapshots_do_not_emit_and_first_snapshot_always_emits() {
    let a = vec![win("100"), win("200")];
    assert!(should_emit_taskbar_snapshot(None, &a));
    assert!(should_emit_taskbar_snapshot(None, &[]), "new renderer must still get an empty first snapshot");
    assert!(!should_emit_taskbar_snapshot(Some(&a[..]), &a.clone()));
    assert!(!should_emit_taskbar_snapshot(Some(&a[..0]), &[]));
}

#[test]
fn every_user_visible_field_change_emits() {
    let base = vec![win("100"), win("200")];
    let mut cases: Vec<(&str, Vec<TaskbarWindow>)> = Vec::new();
    let mut c = base.clone();
    c[1].title = "Other".into();
    cases.push(("title", c));
    let mut c = base.clone();
    c[0].icon_data_url = "data:image/png;base64,BBBB".into();
    cases.push(("icon", c));
    let mut c = base.clone();
    c[0].is_active = true;
    cases.push(("active", c));
    let mut c = base.clone();
    c[1].is_minimized = true;
    cases.push(("minimized", c));
    cases.push(("order", vec![base[1].clone(), base[0].clone()]));
    let mut c = base.clone();
    c[0].attention_state = TaskbarWindowAttentionState::Requested;
    cases.push(("attention", c));
    let mut c = base.clone();
    c[0].process_id = 11;
    cases.push(("pid", c));
    let mut c = base.clone();
    c[0].hwnd = "101".into();
    cases.push(("hwnd", c));
    let mut c = base.clone();
    c[0].process_name = "renamed".into();
    cases.push(("process_name", c));
    let mut c = base.clone();
    c[0].activity_state = TaskbarWindowActivityState::Busy;
    cases.push(("activity", c));
    let mut c = base.clone();
    c[0].notification_count = 2;
    cases.push(("notification_count", c));
    let mut c = base.clone();
    c[0].toast_count = 2;
    cases.push(("toast_count", c));
    cases.push(("removed", vec![base[0].clone()]));
    let mut c = base.clone();
    c.push(win("300"));
    cases.push(("added", c));
    for (label, next) in cases {
        assert!(should_emit_taskbar_snapshot(Some(&base[..]), &next), "{label} must emit");
    }
}

// ---------------------------------------------------------------- publisher

#[test]
fn publisher_skips_emit_for_unchanged_but_keeps_snapshot_and_sequence_semantics() {
    let mut p = SnapshotPublisher::new(0);
    let a = vec![win("100")];

    let first: PublishOutcome = p.publish(a.clone());
    assert!(first.emit);
    assert_eq!(first.snapshot.sequence, 1);

    let again = p.publish(a.clone());
    assert!(!again.emit, "unchanged => no emit");
    assert_eq!(again.snapshot.sequence, 1, "sequence not advanced without an emit");
    assert_eq!(again.snapshot.windows.len(), 1, "snapshot still carries windows for LAST_TASKBAR_SNAPSHOT");
    assert_eq!(again.snapshot.windows[0].hwnd, "100");

    let mut changed = a.clone();
    changed[0].title = "New".into();
    let third = p.publish(changed.clone());
    assert!(third.emit);
    assert_eq!(third.snapshot.sequence, 2, "strictly monotonic across emits");

    let fourth = p.publish(changed);
    assert!(!fourth.emit);
    assert_eq!(fourth.snapshot.sequence, 2);

    // change back to an earlier state is a change versus the LAST emitted one
    let back = p.publish(a);
    assert!(back.emit);
    assert_eq!(back.snapshot.sequence, 3);
}

#[test]
fn publisher_first_empty_snapshot_still_emits_and_resumes_from_start_sequence() {
    let mut p = SnapshotPublisher::new(41);
    let out = p.publish(Vec::new());
    assert!(out.emit);
    assert_eq!(out.snapshot.sequence, 42);
    assert!(!p.publish(Vec::new()).emit);
}

// ---------------------------------------------------------------- process cache

#[test]
fn process_cache_is_keyed_by_pid_and_creation_time_and_bounded() {
    let mut cache = ProcessInfoCache::new(2);
    cache.insert(10, 1_000, Some(PathBuf::from("C:\\a.exe")), "a".into());
    assert_eq!(cache.get(10, 1_000), Some((Some(PathBuf::from("C:\\a.exe")), "a".to_string())));
    assert_eq!(cache.get(10, 2_000), None, "pid reuse with new creation time must miss");
    assert_eq!(cache.get(11, 1_000), None);
    cache.insert(10, 2_000, None, String::new());
    assert_eq!(cache.get(10, 2_000), Some((None, String::new())));
    cache.insert(12, 3_000, None, "c".into());
    cache.insert(13, 4_000, None, "d".into());
    assert!(cache.len() <= 2, "bounded");
}

// ---------------------------------------------------------------- hung icon backoff

#[test]
fn hung_icon_backoff_skips_briefly_then_retries_and_is_keyed_by_hwnd_and_pid() {
    let t0 = Instant::now();
    let ttl = Duration::from_secs(5);
    let mut b = HungIconBackoff::new(ttl);
    assert!(!b.should_skip(0x10, 7, t0));
    b.record_unresponsive(0x10, 7, t0);
    assert!(b.should_skip(0x10, 7, t0 + Duration::from_millis(1)));
    assert!(b.should_skip(0x10, 7, t0 + ttl - Duration::from_millis(1)));
    assert!(!b.should_skip(0x10, 7, t0 + ttl), "retried after ttl, never permanently lost");
    assert!(!b.should_skip(0x10, 8, t0 + Duration::from_millis(1)), "same HWND, new pid");
    assert!(!b.should_skip(0x11, 7, t0 + Duration::from_millis(1)), "other HWND");

    b.record_unresponsive(0x20, 9, t0);
    b.record_responsive(0x20, 9);
    assert!(!b.should_skip(0x20, 9, t0 + Duration::from_millis(1)));

    // bounded: expired entries pruned on record
    for i in 0..500isize {
        b.record_unresponsive(1000 + i, 1, t0);
    }
    b.record_unresponsive(5000, 1, t0 + ttl * 2);
    assert!(b.len() < 50, "expired entries pruned, got {}", b.len());
}

// ---------------------------------------------------------------- heartbeat re-emit

#[test]
fn heartbeat_forces_emit_at_five_seconds_and_when_never_emitted() {
    use super::{heartbeat_must_emit, HEARTBEAT_FORCE_EMIT_AFTER};
    assert_eq!(HEARTBEAT_FORCE_EMIT_AFTER, Duration::from_secs(5));
    let base = Instant::now();
    assert!(heartbeat_must_emit(None, base));
    assert!(!heartbeat_must_emit(Some(base), base));
    assert!(!heartbeat_must_emit(
        Some(base),
        base + HEARTBEAT_FORCE_EMIT_AFTER - Duration::from_millis(1)
    ));
    assert!(heartbeat_must_emit(Some(base), base + HEARTBEAT_FORCE_EMIT_AFTER));
    assert!(heartbeat_must_emit(
        Some(base),
        base + HEARTBEAT_FORCE_EMIT_AFTER + Duration::from_secs(60)
    ));
}
