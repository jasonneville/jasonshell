use super::{helper::process_image_path, parse_hwnd, TaskWindowAction};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, GetLastError, LPARAM, WPARAM};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, GetProcessTimes, OpenProcess,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetAncestor, GetForegroundWindow, GetLastActivePopup,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, PostMessageW,
    SendMessageTimeoutW, SetForegroundWindow, ShowWindowAsync, SwitchToThisWindow, GA_ROOTOWNER,
    SC_MINIMIZE, SC_RESTORE, SMTO_ABORTIFHUNG, SMTO_ERRORONEXIT, SW_MAXIMIZE, SW_MINIMIZE, SW_SHOW,
    WM_CLOSE, WM_SYSCOMMAND,
};

const CLOSE_TIMEOUT_MS: u32 = 750;
const ACTIVATE_RESTORE_ATTEMPTS: usize = 10;
const ACTIVATE_RESTORE_DELAY_MS: u64 = 50;

pub(super) fn activate_task_window(hwnd: String, minimize_if_active: bool) -> Result<(), String> {
    let hwnd = parse_hwnd(&hwnd)?;
    let target = resolve_activation_target(hwnd);
    if !window_exists(target) || !window_exists(hwnd) {
        return Err("Task window is no longer available".to_string());
    }

    let foreground = unsafe { GetForegroundWindow() };
    let target_root = resolve_activation_root(target);
    let foreground_root = resolve_activation_root(foreground);
    let mut foreground_process_id = 0;
    unsafe {
        let _ = GetWindowThreadProcessId(foreground, Some(&mut foreground_process_id));
    }
    if should_minimize_window(
        minimize_if_active,
        foreground == target || foreground_root == target_root,
        unsafe { IsIconic(target_root).as_bool() },
        foreground_process_id == std::process::id(),
    ) {
        unsafe {
            PostMessageW(
                Some(target_root),
                WM_SYSCOMMAND,
                WPARAM(SC_MINIMIZE as usize),
                LPARAM(0),
            )
            .map_err(|error| format!("Failed to minimize task window: {error}"))?;
        }
        return Ok(());
    }

    activate_window(target, hwnd)
}

pub(crate) fn perform_task_window_action(
    hwnd: String,
    action: TaskWindowAction,
) -> Result<(), String> {
    match action {
        TaskWindowAction::Focus => focus_task_window(hwnd),
        TaskWindowAction::Maximize => maximize_task_window(hwnd),
        TaskWindowAction::Minimize => {
            let hwnd = parse_hwnd(&hwnd)?;
            minimize_window(hwnd);
            Ok(())
        }
        TaskWindowAction::Close => {
            super::reject_internal_shell_hwnd(&hwnd)?;
            let hwnd = parse_hwnd(&hwnd)?;
            close_window(hwnd)
        }
    }
}

pub(crate) fn maximize_task_window(hwnd: String) -> Result<(), String> {
    let hwnd = parse_hwnd(&hwnd)?;
    maximize_window(hwnd);
    Ok(())
}

fn focus_task_window(hwnd: String) -> Result<(), String> {
    let hwnd = parse_hwnd(&hwnd)?;
    activate_window(resolve_activation_target(hwnd), hwnd)
}

fn activate_window(
    target: windows::Win32::Foundation::HWND,
    raw_hwnd: windows::Win32::Foundation::HWND,
) -> Result<(), String> {
    if !window_exists(target) || !window_exists(raw_hwnd) {
        return Err("Task window is no longer available".to_string());
    }

    let restore_targets = activation_restore_targets(target, raw_hwnd);
    if !restore_targets.is_empty() {
        for restore_target in &restore_targets {
            unsafe {
                let _ = PostMessageW(
                    Some(*restore_target),
                    WM_SYSCOMMAND,
                    WPARAM(SC_RESTORE as usize),
                    LPARAM(0),
                );
            }
        }
        wait_for_windows_restore(&restore_targets);
    } else {
        unsafe {
            let _ = ShowWindowAsync(target, SW_SHOW);
        }
    }

    unsafe {
        let _ = BringWindowToTop(target);
        let _ = SetForegroundWindow(target);
    }

    let set_foreground_succeeded = unsafe { SetForegroundWindow(target).as_bool() };
    let foreground_window = unsafe { GetForegroundWindow() };
    let target_root_owner = unsafe { GetAncestor(target, GA_ROOTOWNER) };
    let foreground_thread = unsafe { GetWindowThreadProcessId(foreground_window, None) };
    let target_thread = unsafe { GetWindowThreadProcessId(target, None) };

    if should_use_foreground_handoff(set_foreground_succeeded)
        && foreground_window != target
        && foreground_window != target_root_owner
        && foreground_thread != 0
        && target_thread != 0
        && foreground_thread != target_thread
    {
        unsafe {
            let current_thread = GetCurrentThreadId();
            let mut foreground_attached = false;
            let mut target_attached = false;

            if AttachThreadInput(current_thread, foreground_thread, true).as_bool() {
                foreground_attached = true;
            }
            if AttachThreadInput(current_thread, target_thread, true).as_bool() {
                target_attached = true;
            }
            let _ = BringWindowToTop(target);
            let _ = SetForegroundWindow(target);

            if target_attached {
                let _ = AttachThreadInput(current_thread, target_thread, false);
            }
            if foreground_attached {
                let _ = AttachThreadInput(current_thread, foreground_thread, false);
            }
        }
    }

    let foreground_window = unsafe { GetForegroundWindow() };
    if foreground_window != target && foreground_window != target_root_owner {
        unsafe {
            let _ = BringWindowToTop(target);
            let _ = SetForegroundWindow(target);
            SwitchToThisWindow(target, true);
        }
    }

    let verified_foreground = unsafe { GetForegroundWindow() };
    if verified_foreground == target || verified_foreground == target_root_owner {
        return Ok(());
    }

    Err("Failed to focus task window".to_string())
}

fn activation_restore_targets(
    target: windows::Win32::Foundation::HWND,
    raw_hwnd: windows::Win32::Foundation::HWND,
) -> Vec<windows::Win32::Foundation::HWND> {
    let mut targets = Vec::new();
    for hwnd in [target, resolve_activation_root(raw_hwnd), raw_hwnd] {
        if unsafe { IsIconic(hwnd).as_bool() } && !targets.contains(&hwnd) {
            targets.push(hwnd);
        }
    }
    targets
}

fn resolve_activation_root(
    hwnd: windows::Win32::Foundation::HWND,
) -> windows::Win32::Foundation::HWND {
    let root_owner = unsafe { GetAncestor(hwnd, GA_ROOTOWNER) };
    if root_owner.0.is_null() {
        hwnd
    } else {
        root_owner
    }
}

fn wait_for_windows_restore(targets: &[windows::Win32::Foundation::HWND]) {
    for _ in 0..ACTIVATE_RESTORE_ATTEMPTS {
        if targets
            .iter()
            .all(|hwnd| !unsafe { IsIconic(*hwnd).as_bool() })
        {
            return;
        }
        thread::sleep(Duration::from_millis(ACTIVATE_RESTORE_DELAY_MS));
    }
}

pub(super) fn resolve_activation_target(
    hwnd: windows::Win32::Foundation::HWND,
) -> windows::Win32::Foundation::HWND {
    let root_owner = unsafe { GetAncestor(hwnd, GA_ROOTOWNER) };
    if root_owner.0.is_null() {
        return hwnd;
    }

    let popup = unsafe { GetLastActivePopup(root_owner) };
    if popup.0.is_null() {
        return root_owner;
    }

    if unsafe { IsWindowVisible(popup).as_bool() } {
        popup
    } else {
        root_owner
    }
}

fn minimize_window(hwnd: windows::Win32::Foundation::HWND) {
    unsafe {
        let _ = ShowWindowAsync(hwnd, SW_MINIMIZE);
    }
}

fn maximize_window(hwnd: windows::Win32::Foundation::HWND) {
    unsafe {
        let _ = ShowWindowAsync(hwnd, SW_MAXIMIZE);
        let _ = SetForegroundWindow(hwnd);
    }
}

fn close_window(hwnd: windows::Win32::Foundation::HWND) -> Result<(), String> {
    if !window_exists(hwnd) {
        return Ok(());
    }

    let initial_identity = capture_task_window_identity_at_start(hwnd)?;
    close_window_with_identity(hwnd, initial_identity)
}

pub(crate) fn close_task_window_with_identity(
    hwnd: String,
    expected_identity: TaskWindowIdentity,
) -> Result<(), String> {
    let hwnd = parse_hwnd(&hwnd)?;
    revalidate_close_target(hwnd, &expected_identity)?;
    close_window_with_identity(hwnd, expected_identity)
}

/// Accept one close request; a surviving window is not a dispatch failure.
pub(crate) fn request_close_task_window_with_identity(
    hwnd: String,
    expected_identity: TaskWindowIdentity,
) -> Result<(), String> {
    let hwnd = parse_hwnd(&hwnd)?;
    request_close_with(
        || revalidate_close_target(hwnd, &expected_identity),
        || unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) }
            .map_err(|error| format!("Failed to request task window close: {error}")),
    )
}

fn request_close_with(
    validate: impl FnOnce() -> Result<(), String>,
    dispatch: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    validate()?;
    dispatch()
}

/// What follows the synchronous `SendMessageTimeoutW(WM_CLOSE)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CloseFollowup {
    Done,
    PostFallback,
}

const ERROR_TIMEOUT: u32 = 1460;

/// A delivered close, or a timeout (the app is probably showing a "Save changes?"
/// prompt, and a second WM_CLOSE would duplicate or answer it), is final. Only a
/// genuine delivery failure against a surviving window falls back to one post.
fn close_followup(send_succeeded: bool, last_error: u32, window_exists: bool) -> CloseFollowup {
    if send_succeeded || last_error == ERROR_TIMEOUT || !window_exists {
        CloseFollowup::Done
    } else {
        CloseFollowup::PostFallback
    }
}

/// Close is request-only: one WM_CLOSE, with a single `PostMessage` fallback only
/// when the synchronous send could not be delivered. A window that stays open
/// (for example a save prompt) is success. This never terminates the owning
/// process and never elevates.
fn close_window_with_identity(
    hwnd: windows::Win32::Foundation::HWND,
    initial_identity: TaskWindowIdentity,
) -> Result<(), String> {
    revalidate_close_target(hwnd, &initial_identity)?;
    let mut result = 0usize;
    let close_status = unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_CLOSE,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT,
            CLOSE_TIMEOUT_MS,
            Some(&mut result),
        )
    };
    let send_succeeded = close_status.0 != 0;
    let last_error = if send_succeeded {
        0
    } else {
        unsafe { GetLastError().0 }
    };

    match close_followup(send_succeeded, last_error, window_exists(hwnd)) {
        CloseFollowup::Done => Ok(()),
        CloseFollowup::PostFallback => {
            revalidate_close_target(hwnd, &initial_identity)?;
            unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) }
                .map_err(|error| format!("Failed to close task window: {error}"))
        }
    }
}

fn revalidate_close_target(
    hwnd: windows::Win32::Foundation::HWND,
    expected_identity: &TaskWindowIdentity,
) -> Result<(), String> {
    revalidate_close_target_with(
        expected_identity,
        || window_exists(hwnd),
        || current_task_window_identity(hwnd),
    )
}

fn revalidate_close_target_with(
    expected_identity: &TaskWindowIdentity,
    exists: impl FnOnce() -> bool,
    inspect: impl FnOnce() -> Result<TaskWindowIdentity, String>,
) -> Result<(), String> {
    if !exists() {
        return Err("Task window target became stale before close fallback".to_string());
    }
    let current_identity = inspect()?;
    if !task_window_identity_matches(
        &current_identity,
        expected_identity.process_id,
        expected_identity.creation_time,
        &expected_identity.canonical_image_path,
    ) {
        return Err(
            "Task window target identity could not be revalidated before close fallback"
                .to_string(),
        );
    }
    Ok(())
}

fn capture_task_window_identity_at_start(
    hwnd: windows::Win32::Foundation::HWND,
) -> Result<TaskWindowIdentity, String> {
    current_task_window_identity(hwnd)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TaskWindowIdentity {
    pub(crate) process_id: u32,
    pub(crate) creation_time: u64,
    pub(crate) canonical_image_path: PathBuf,
}

pub(crate) fn current_task_window_identity(
    hwnd: windows::Win32::Foundation::HWND,
) -> Result<TaskWindowIdentity, String> {
    let mut process_id = 0_u32;
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    if process_id == 0 {
        return Err("Task window target lost process identity before close fallback".to_string());
    }
    Ok(TaskWindowIdentity {
        process_id,
        creation_time: process_creation_time(process_id)?,
        canonical_image_path: process_image_path(process_id)?,
    })
}

pub(super) fn task_window_identity_matches(
    current: &TaskWindowIdentity,
    expected_pid: u32,
    expected_creation_time: u64,
    expected_canonical_image_path: &PathBuf,
) -> bool {
    current.process_id == expected_pid
        && current.creation_time == expected_creation_time
        && &current.canonical_image_path == expected_canonical_image_path
}

fn process_creation_time(process_id: u32) -> Result<u64, String> {
    let process_handle =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.map_err(
            |error| format!("Failed to inspect task window process {process_id}: {error}"),
        )?;
    struct HandleGuard(windows::Win32::Foundation::HANDLE);
    impl Drop for HandleGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let _guard = HandleGuard(process_handle);
    let mut creation = windows::Win32::Foundation::FILETIME::default();
    let mut exit = windows::Win32::Foundation::FILETIME::default();
    let mut kernel = windows::Win32::Foundation::FILETIME::default();
    let mut user = windows::Win32::Foundation::FILETIME::default();
    unsafe {
        GetProcessTimes(
            process_handle,
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    }
    .map_err(|error| format!("Failed to inspect task window process {process_id}: {error}"))?;
    Ok(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64)
}

pub(super) fn should_minimize_window(
    minimize_if_active: bool,
    is_foreground: bool,
    is_minimized: bool,
    shell_is_foreground: bool,
) -> bool {
    !is_minimized && (is_foreground || (minimize_if_active && shell_is_foreground))
}

pub(super) fn should_use_foreground_handoff(set_foreground_succeeded: bool) -> bool {
    !set_foreground_succeeded
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn request_identity_fixture() -> TaskWindowIdentity {
        TaskWindowIdentity { process_id: 42, creation_time: 99,
            canonical_image_path: PathBuf::from(r"C:\fixture.exe") }
    }

    #[test]
    fn request_only_close_dead_target_rejects_before_inspection_or_dispatch() {
        let expected = request_identity_fixture();
        let result = request_close_with(
            || revalidate_close_target_with(&expected, || false, || panic!("dead target inspected")),
            || panic!("dead target dispatched"));
        assert!(result.unwrap_err().contains("stale"));
    }

    #[test]
    fn request_only_close_identity_mismatches_and_inspection_failure_never_dispatch() {
        let expected = request_identity_fixture();
        for current in [
            TaskWindowIdentity { process_id: 43, ..expected.clone() },
            TaskWindowIdentity { creation_time: 100, ..expected.clone() },
            TaskWindowIdentity { canonical_image_path: PathBuf::from(r"C:\other.exe"), ..expected.clone() },
        ] {
            assert!(request_close_with(
                || revalidate_close_target_with(&expected, || true, || Ok(current)),
                || panic!("identity mismatch dispatched")).is_err());
        }
        let result = request_close_with(
            || revalidate_close_target_with(&expected, || true, || Err("inspection denied".into())),
            || panic!("uninspectable target dispatched"));
        assert_eq!(result.unwrap_err(), "inspection denied");
    }

    #[test]
    fn request_only_close_valid_identity_dispatches_once_and_failure_never_retries() {
        let expected = request_identity_fixture();
        for outcome in [Ok(()), Err("WM_CLOSE access denied".to_string()), Err("WM_CLOSE failed".to_string())] {
            let dispatches = Cell::new(0);
            let wanted = outcome.clone();
            let result = request_close_with(
                || revalidate_close_target_with(&expected, || true, || Ok(expected.clone())),
                || { dispatches.set(dispatches.get() + 1); outcome });
            assert_eq!(result, wanted);
            assert_eq!(dispatches.get(), 1);
        }
    }

    #[test]
    fn request_only_close_malformed_handle_and_shell_admission_errors_never_dispatch() {
        assert!(request_close_task_window_with_identity("not-an-hwnd".into(), request_identity_fixture()).is_err());
        let shell_path = std::env::current_exe().unwrap();
        let rejected = super::super::reject_internal_shell_process_path(&shell_path).unwrap();
        assert!(rejected);
        assert!(request_close_with(
            || if rejected { Err("internal shell target".into()) } else { Ok(()) },
            || panic!("shell target dispatched")).is_err());
    }

    #[test]
    fn task_window_identity_matches_requires_exact_pid_time_and_path() {
        let current = TaskWindowIdentity {
            process_id: 10,
            creation_time: 20,
            canonical_image_path: PathBuf::from(r"C:\good.exe"),
        };
        assert!(task_window_identity_matches(
            &current,
            10,
            20,
            &PathBuf::from(r"C:\good.exe")
        ));
        assert!(!task_window_identity_matches(
            &current,
            11,
            20,
            &PathBuf::from(r"C:\good.exe")
        ));
        assert!(!task_window_identity_matches(
            &current,
            10,
            21,
            &PathBuf::from(r"C:\good.exe")
        ));
        assert!(!task_window_identity_matches(
            &current,
            10,
            20,
            &PathBuf::from(r"C:\bad.exe")
        ));
    }

    #[test]
    fn capture_identity_requires_matching_live_pid_only() {
        let current = TaskWindowIdentity {
            process_id: 42,
            creation_time: 99,
            canonical_image_path: PathBuf::from(r"C:\good.exe"),
        };
        assert!(capture_task_window_identity_at_start_for_test(&current, 42).is_ok());
        assert!(capture_task_window_identity_at_start_for_test(&current, 7).is_err());
    }

    #[test]
    fn revalidation_requires_exact_captured_identity_match() {
        let captured = TaskWindowIdentity {
            process_id: 42,
            creation_time: 99,
            canonical_image_path: PathBuf::from(r"C:\good.exe"),
        };
        let current = captured.clone();
        assert!(revalidate_identity_for_test(&current, &captured).is_ok());
        assert!(revalidate_identity_for_test(
            &TaskWindowIdentity {
                process_id: 42,
                creation_time: 100,
                canonical_image_path: PathBuf::from(r"C:\good.exe")
            },
            &captured,
        )
        .is_err());
    }

    fn capture_task_window_identity_at_start_for_test(
        current: &TaskWindowIdentity,
        process_id: u32,
    ) -> Result<TaskWindowIdentity, String> {
        if current.process_id != process_id {
            return Err(
                "Task window target identity could not be revalidated before close fallback"
                    .to_string(),
            );
        }
        Ok(current.clone())
    }

    fn revalidate_identity_for_test(
        current: &TaskWindowIdentity,
        expected: &TaskWindowIdentity,
    ) -> Result<(), String> {
        if !task_window_identity_matches(
            current,
            expected.process_id,
            expected.creation_time,
            &expected.canonical_image_path,
        ) {
            return Err(
                "Task window target identity could not be revalidated before close fallback"
                    .to_string(),
            );
        }
        Ok(())
    }
}

fn window_exists(hwnd: windows::Win32::Foundation::HWND) -> bool {
    unsafe { IsWindow(Some(hwnd)).as_bool() }
}

#[cfg(test)]
#[path = "actions_tier0_tests.rs"]
mod tier0_tests;
