use serde::{Deserialize, Serialize};

pub use diagnostics::{TaskbarRuntimeDiagnostics, ToastListenerStatus};

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
    pub notification_count: u32,
    pub attention_state: TaskbarWindowAttentionState,
    pub toast_count: u32,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskbarWindowAttentionState {
    #[default]
    Idle,
    Requested,
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskbarWindowsSnapshot {
    pub sequence: u64,
    pub windows: Vec<TaskbarWindow>,
}

#[cfg(target_os = "windows")]
mod actions;
pub(crate) use actions::TaskWindowIdentity;
#[cfg(target_os = "windows")]
mod attention;
pub(crate) mod bounded_string_cache;
#[cfg(target_os = "windows")]
mod diagnostics;
#[cfg(target_os = "windows")]
pub(crate) mod flash_fixture;
#[cfg(target_os = "windows")]
mod helper;
#[cfg(target_os = "windows")]
mod icons;
#[cfg(target_os = "windows")]
mod native_hooks;
#[cfg(target_os = "windows")]
mod notifications;
#[cfg(target_os = "windows")]
mod previews;
#[cfg(all(target_os = "windows", test))]
mod tests;
#[cfg(target_os = "windows")]
mod windows;

pub(crate) const TASKBAR_WINDOWS_SNAPSHOT_EVENT: &str = "taskbar:windows-snapshot";

#[cfg(target_os = "windows")]
pub(crate) fn start_notification_tracking() {
    notifications::start_notification_tracking();
}

#[cfg(target_os = "windows")]
pub(crate) fn start_taskbar_snapshot_pipeline(app: &tauri::AppHandle) {
    windows::refresh_taskbar_snapshot_now(Some(app)).ok();
    windows::ensure_taskbar_snapshot_worker_started(app.clone());
}

#[cfg(target_os = "windows")]
pub(crate) fn start_taskbar_hooks(app: tauri::AppHandle) -> Result<(), String> {
    let _ = app;
    if !native_hooks::explorer_suppression_v2_enabled_from_env()
        && !native_hooks::taskbar_native_hooks_enabled_from_env()
    {
        return Ok(());
    }
    native_hooks::start_native_hooks()
}

#[cfg(target_os = "windows")]
pub(crate) fn record_taskbar_attention(
    identity: attention::TaskbarAttentionIdentity,
    requested: bool,
) {
    attention::record_taskbar_attention(identity, requested);
}

#[cfg(target_os = "windows")]
pub(crate) fn clear_taskbar_attention_if_matches(identity: &attention::TaskbarAttentionIdentity) {
    attention::clear_taskbar_attention_if_matches(identity);
}

#[cfg(target_os = "windows")]
pub(crate) fn remove_root_owner_taskbar_attention(root_owner_hwnd: isize) {
    attention::remove_root_owner_taskbar_attention(root_owner_hwnd);
}

#[cfg(target_os = "windows")]
pub(crate) fn taskbar_native_hooks_health() -> native_hooks::NativeHooksHealth {
    native_hooks::native_hooks_health()
}

#[cfg(target_os = "windows")]
pub(crate) fn stop_taskbar_hooks() {
    native_hooks::stop_native_hooks();
}

#[tauri::command]
pub fn request_taskbar_windows_refresh() {
    #[cfg(target_os = "windows")]
    {
        windows::request_taskbar_snapshot_refresh();
    }
}

#[tauri::command]
pub fn get_taskbar_runtime_diagnostics() -> Result<TaskbarRuntimeDiagnostics, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(diagnostics::taskbar_runtime_diagnostics())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(TaskbarRuntimeDiagnostics::default())
    }
}

pub(crate) fn note_explorer_taskbar_reconcile(
    tracked: u64,
    hidden: u64,
    recreations: u64,
    hide_failures: u64,
    last_error: Option<&str>,
) {
    diagnostics::note_explorer_taskbar_reconcile(
        tracked,
        hidden,
        recreations,
        hide_failures,
        last_error,
    );
}

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
pub fn activate_task_window(hwnd: String, minimize_if_active: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let app_path = task_window_process_path(&hwnd).ok();
        actions::activate_task_window(hwnd, minimize_if_active)?;
        if let Some(app_path) = app_path {
            notifications::clear_notifications_for_process_path(&app_path);
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        let _ = minimize_if_active;
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
        actions::perform_task_window_action(hwnd, TaskWindowAction::Close)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        Err("Taskbar window integration is only supported on Windows".to_string())
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn task_window_identity(hwnd: &str) -> Result<actions::TaskWindowIdentity, String> {
    actions::current_task_window_identity(parse_hwnd(hwnd)?)
}

#[cfg(target_os = "windows")]
pub(crate) fn close_task_window_with_identity(
    hwnd: String,
    identity: TaskWindowIdentity,
) -> Result<(), String> {
    reject_internal_shell_hwnd(&hwnd)?;
    actions::close_task_window_with_identity(hwnd, identity)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn task_window_identity(_hwnd: &str) -> Result<(), String> {
    Err("Taskbar window integration is only supported on Windows".to_string())
}

#[cfg(target_os = "windows")]
pub(super) fn reject_internal_shell_hwnd(hwnd: &str) -> Result<(), String> {
    let target_path = task_window_process_path(hwnd)?;
    if reject_internal_shell_process_path(&target_path)? {
        return Err(
            "Refusing to close an internal JasonShell window from task preview".to_string(),
        );
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub(super) fn reject_internal_shell_process_path(
    target_path: &std::path::Path,
) -> Result<bool, String> {
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

#[cfg(target_os = "windows")]
pub(crate) fn handle_task_window_helper_args() -> Result<bool, String> {
    helper::handle_task_window_helper_args()
}

#[cfg(target_os = "windows")]
pub(crate) fn handle_taskbar_flash_fixture_args() -> Result<bool, String> {
    flash_fixture::handle_taskbar_flash_fixture_args()
}

#[cfg(target_os = "windows")]
pub(crate) fn spawn_task_window_helper(
    hwnd: String,
    pid: u32,
    creation_time: u64,
    canonical_image_path: std::path::PathBuf,
) -> Result<(), String> {
    windows::spawn_task_window_helper(hwnd, pid, creation_time, canonical_image_path)
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
