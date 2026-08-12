use super::{parse_hwnd, TaskWindowAction};
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, LPARAM, WPARAM};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, TerminateProcess, PROCESS_TERMINATE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetAncestor, GetForegroundWindow, GetLastActivePopup,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, PostMessageW,
    SendMessageTimeoutW, SetForegroundWindow, ShowWindowAsync, SwitchToThisWindow, GA_ROOTOWNER,
    SC_MINIMIZE, SC_RESTORE, SMTO_ABORTIFHUNG, SMTO_ERRORONEXIT, SW_MAXIMIZE, SW_MINIMIZE, SW_SHOW,
    WM_CLOSE, WM_SYSCOMMAND,
};

const CLOSE_TIMEOUT_MS: u32 = 750;
const CLOSE_VERIFY_ATTEMPTS: usize = 10;
const CLOSE_VERIFY_DELAY_MS: u64 = 50;
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
    let send_timeout_succeeded = close_status.0 != 0;

    if send_timeout_succeeded && wait_for_window_close(hwnd) {
        return Ok(());
    }

    if should_fallback_post_close(send_timeout_succeeded, window_exists(hwnd)) {
        unsafe {
            PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0))
                .map_err(|error| format!("Failed to close task window: {error}"))?;
        }
        if wait_for_window_close(hwnd) {
            return Ok(());
        }
    }

    terminate_window_process(hwnd)
}

fn terminate_window_process(hwnd: windows::Win32::Foundation::HWND) -> Result<(), String> {
    if !window_exists(hwnd) {
        return Ok(());
    }

    let mut process_id = 0_u32;
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    if process_id == 0 {
        return Err(
            "Task window did not close and its owning process could not be identified".to_string(),
        );
    }

    let process_handle =
        unsafe { OpenProcess(PROCESS_TERMINATE, false, process_id) }.map_err(|error| {
            format!("Failed to open task window process {process_id} for termination: {error}")
        })?;
    let result = unsafe { TerminateProcess(process_handle, 1) }
        .map_err(|error| format!("Failed to terminate task window process {process_id}: {error}"));
    unsafe {
        let _ = CloseHandle(process_handle);
    }
    result?;

    if wait_for_window_close(hwnd) {
        return Ok(());
    }

    Err(format!(
        "Task window process {process_id} was terminated but the window still exists"
    ))
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

pub(super) fn should_fallback_post_close(
    _send_timeout_succeeded: bool,
    window_still_exists: bool,
) -> bool {
    window_still_exists
}

fn wait_for_window_close(hwnd: windows::Win32::Foundation::HWND) -> bool {
    for _ in 0..CLOSE_VERIFY_ATTEMPTS {
        if !window_exists(hwnd) {
            return true;
        }
        thread::sleep(Duration::from_millis(CLOSE_VERIFY_DELAY_MS));
    }
    !window_exists(hwnd)
}

fn window_exists(hwnd: windows::Win32::Foundation::HWND) -> bool {
    unsafe { IsWindow(Some(hwnd)).as_bool() }
}
