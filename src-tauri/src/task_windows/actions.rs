use super::{parse_hwnd, TaskWindowAction};
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, IsIconic, IsWindow, PostMessageW, SendMessageTimeoutW,
    SetForegroundWindow, ShowWindowAsync, SMTO_ABORTIFHUNG, SMTO_ERRORONEXIT, SW_MAXIMIZE,
    SW_MINIMIZE, SW_RESTORE, WM_CLOSE,
};

const CLOSE_TIMEOUT_MS: u32 = 750;
const CLOSE_VERIFY_ATTEMPTS: usize = 10;
const CLOSE_VERIFY_DELAY_MS: u64 = 50;

pub(super) fn activate_task_window(hwnd: String, was_active: bool) -> Result<(), String> {
    let hwnd = parse_hwnd(&hwnd)?;
    let is_foreground = unsafe { GetForegroundWindow() == hwnd };
    let is_minimized = unsafe { IsIconic(hwnd).as_bool() };

    if should_minimize_window(was_active, is_foreground, is_minimized) {
        minimize_window(hwnd);
        return Ok(());
    }

    focus_window(hwnd);
    Ok(())
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
    focus_window(hwnd);
    Ok(())
}

fn focus_window(hwnd: windows::Win32::Foundation::HWND) {
    if unsafe { IsIconic(hwnd).as_bool() } {
        unsafe {
            let _ = ShowWindowAsync(hwnd, SW_RESTORE);
        }
    }

    unsafe {
        let _ = SetForegroundWindow(hwnd);
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

    Err(
        "Task window did not close after WM_CLOSE; it may be elevated, protected, or vetoing close"
            .to_string(),
    )
}

pub(super) fn should_minimize_window(
    was_active: bool,
    is_foreground: bool,
    is_minimized: bool,
) -> bool {
    !is_minimized && (was_active || is_foreground)
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
