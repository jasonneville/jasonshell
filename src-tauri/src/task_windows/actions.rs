use super::{parse_hwnd, TaskWindowAction};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, IsIconic, PostMessageW, SetForegroundWindow, ShowWindowAsync, SW_MAXIMIZE,
    SW_MINIMIZE, SW_RESTORE, WM_CLOSE,
};

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
            unsafe {
                PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0))
                    .map_err(|error| format!("Failed to close task window: {error}"))
            }
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

pub(super) fn should_minimize_window(
    was_active: bool,
    is_foreground: bool,
    is_minimized: bool,
) -> bool {
    !is_minimized && (was_active || is_foreground)
}
