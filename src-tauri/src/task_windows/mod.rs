use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskbarWindow {
    pub hwnd: String,
    pub title: String,
    pub process_id: u32,
    pub process_name: String,
    pub icon_data_url: String,
    pub is_active: bool,
    pub is_minimized: bool,
    pub activity_state: TaskbarWindowActivityState,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskbarProcessWindow {
    pub hwnd: String,
    pub title: String,
    pub process_id: u32,
    pub is_active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskbarWindowActivityState {
    Idle,
    Busy,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskWindowAction {
    Focus,
    Maximize,
    Minimize,
    Close,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskWindowPreviewImage {
    pub image_data_url: String,
    pub width: u32,
    pub height: u32,
}

#[cfg(target_os = "windows")]
mod actions;
#[cfg(target_os = "windows")]
mod icons;
#[cfg(target_os = "windows")]
mod previews;
#[cfg(all(target_os = "windows", test))]
mod tests;
#[cfg(target_os = "windows")]
mod windows;

#[tauri::command]
pub fn list_open_task_windows() -> Result<Vec<TaskbarWindow>, String> {
    #[cfg(target_os = "windows")]
    {
        windows::list_open_task_windows()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(Vec::new())
    }
}

#[tauri::command]
pub fn list_taskbar_process_windows() -> Result<Vec<TaskbarProcessWindow>, String> {
    #[cfg(target_os = "windows")]
    {
        windows::list_taskbar_process_windows()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(Vec::new())
    }
}

#[tauri::command]
pub fn activate_task_window(hwnd: String, was_active: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        actions::activate_task_window(hwnd, was_active)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (hwnd, was_active);
        Err("Taskbar window integration is only supported on Windows".to_string())
    }
}

#[tauri::command]
pub fn maximize_task_window(hwnd: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        actions::maximize_task_window(hwnd)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        Err("Taskbar window integration is only supported on Windows".to_string())
    }
}

#[tauri::command]
pub fn close_task_window(hwnd: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        reject_internal_shell_hwnd(&hwnd)?;
        validate_task_window_preview_source(&hwnd)?;
        actions::perform_task_window_action(hwnd, TaskWindowAction::Close)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        Err("Taskbar window integration is only supported on Windows".to_string())
    }
}

#[cfg(target_os = "windows")]
fn reject_internal_shell_hwnd(hwnd: &str) -> Result<(), String> {
    if is_jasonshell_window(hwnd)? {
        return Err("Refusing to close an internal JasonShell window from task preview".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn is_jasonshell_window(hwnd: &str) -> Result<bool, String> {
    let target_path = task_window_process_path(hwnd)?;
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("Current JasonShell executable path is unavailable: {error}"))?;
    Ok(target_path == current_exe)
}

#[cfg(target_os = "windows")]
pub(crate) fn perform_task_window_action(
    hwnd: String,
    action: TaskWindowAction,
) -> Result<(), String> {
    actions::perform_task_window_action(hwnd, action)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn perform_task_window_action(
    hwnd: String,
    action: TaskWindowAction,
) -> Result<(), String> {
    let _ = (hwnd, action);
    Err("Taskbar window integration is only supported on Windows".to_string())
}

#[cfg(target_os = "windows")]
pub(crate) fn capture_task_window_preview(hwnd: String) -> Result<TaskWindowPreviewImage, String> {
    previews::capture_task_window_preview(hwnd)
}

#[cfg(target_os = "windows")]
pub(crate) fn validate_task_window_preview_source(
    hwnd: &str,
) -> Result<::windows::Win32::Foundation::HWND, String> {
    previews::validate_task_window_preview_source(hwnd)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn capture_task_window_preview(hwnd: String) -> Result<TaskWindowPreviewImage, String> {
    let _ = hwnd;
    Err("Taskbar window previews are only supported on Windows".to_string())
}

#[cfg(target_os = "windows")]
pub(crate) fn shell_file_icon_data_url(path: &std::path::Path) -> Result<String, String> {
    icons::file_icon_data_url(path)
}

#[cfg(target_os = "windows")]
pub(super) fn parse_hwnd(hwnd: &str) -> Result<::windows::Win32::Foundation::HWND, String> {
    let hwnd_value = hwnd
        .parse::<isize>()
        .map_err(|error| format!("Invalid window handle '{hwnd}': {error}"))?;

    Ok(::windows::Win32::Foundation::HWND(hwnd_value as *mut _))
}

#[cfg(target_os = "windows")]
pub(crate) fn task_window_process_path(hwnd: &str) -> Result<std::path::PathBuf, String> {
    let hwnd = parse_hwnd(hwnd)?;
    windows::process_image_path_for_hwnd(hwnd)
}
