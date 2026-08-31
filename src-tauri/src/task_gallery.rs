use crate::shell_windows::{
    BOTTOM_BAR_LABEL, TASK_GALLERY_HEIGHT_LOGICAL, TASK_GALLERY_LABEL, TASK_GALLERY_WIDTH_LOGICAL,
};
use crate::{task_preview, task_windows, taskbar_menu};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, Window};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE};

const TASK_GALLERY_EDGE_PADDING_LOGICAL: f64 = 0.0;
const TASK_GALLERY_TAB_GAP_LOGICAL: f64 = 0.0;
const MAX_CANCELLED_GALLERY_NONCES: usize = 256;

pub const TASK_GALLERY_CLOSED_EVENT: &str = "task-gallery:closed";
pub const TASK_GALLERY_OPEN_EVENT: &str = "task-gallery:open";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryShowArgs {
    pub anchor_left: f64,
    pub anchor_width: f64,
    pub nonce: String,
    pub group_key: String,
    pub label: String,
    pub windows: Vec<TaskGalleryWindowRow>,
    #[serde(default)]
    pub focus_gallery: bool,
    #[serde(default)]
    pub refresh_existing: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryActivateArgs {
    pub nonce: String,
    pub hwnd: String,
    pub minimize_if_active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryClosePreviewedWindowArgs {
    pub nonce: String,
    pub hwnd: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryContextMenuArgs {
    pub nonce: String,
    pub hwnd: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryPreviewArgs {
    pub nonce: String,
    #[serde(rename = "requestId")]
    pub request_id: u64,
    pub hwnd: String,
    pub title: String,
    pub process_name: String,
    pub icon_data_url: String,
    pub is_minimized: bool,
    pub anchor_left: f64,
    pub anchor_width: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryOpenPayload {
    pub nonce: String,
    pub group_key: String,
    pub label: String,
    pub focus_gallery: bool,
    pub windows: Vec<TaskGalleryWindowRow>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryWindowRow {
    pub hwnd: String,
    pub title: String,
    pub process_id: Option<u32>,
    pub process_name: String,
    pub icon_data_url: String,
    pub is_active: bool,
    pub is_minimized: bool,
}

#[derive(Default)]
struct TaskGalleryRuntimeState {
    nonce: Option<String>,
    cancelled_nonces: VecDeque<String>,
    windows_by_hwnd: BTreeMap<String, TaskGalleryAuthorizedWindow>,
    focus_loss_hold_count: u32,
}

#[derive(Clone)]
struct TaskGalleryAuthorizedWindow {
    row: TaskGalleryWindowRow,
    #[cfg(target_os = "windows")]
    identity: task_windows::TaskWindowIdentity,
}

fn state() -> &'static Mutex<TaskGalleryRuntimeState> {
    static STATE: OnceLock<Mutex<TaskGalleryRuntimeState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(TaskGalleryRuntimeState::default()))
}

fn reset_state() {
    let mut state = state().lock().unwrap();
    state.nonce = None;
    state.windows_by_hwnd.clear();
    state.focus_loss_hold_count = 0;
}

fn record_cancelled_nonce(runtime: &mut TaskGalleryRuntimeState, nonce: String) {
    if runtime.cancelled_nonces.contains(&nonce) {
        return;
    }
    if runtime.cancelled_nonces.len() == MAX_CANCELLED_GALLERY_NONCES {
        runtime.cancelled_nonces.pop_front();
    }
    runtime.cancelled_nonces.push_back(nonce);
}

fn take_cancelled_nonce(runtime: &mut TaskGalleryRuntimeState, nonce: &str) -> bool {
    let Some(index) = runtime
        .cancelled_nonces
        .iter()
        .position(|cancelled| cancelled == nonce)
    else {
        return false;
    };
    runtime.cancelled_nonces.remove(index);
    true
}

fn emit_closed_event(app_handle: &AppHandle, nonce: Option<String>) {
    let payload = json!({"nonce": nonce});
    let _ = app_handle.emit_to(BOTTOM_BAR_LABEL, TASK_GALLERY_CLOSED_EVENT, payload.clone());
    let _ = app_handle.emit_to(TASK_GALLERY_LABEL, TASK_GALLERY_CLOSED_EVENT, payload);
}

fn begin_focus_hold() {
    state().lock().unwrap().focus_loss_hold_count += 1;
}
fn end_focus_hold() {
    let mut state = state().lock().unwrap();
    state.focus_loss_hold_count = state.focus_loss_hold_count.saturating_sub(1);
}
fn focus_hold_active() -> bool {
    state().lock().unwrap().focus_loss_hold_count > 0
}

fn validate(args_nonce: &str, hwnd: &str) -> Result<(), String> {
    snapshot_window(args_nonce, hwnd).map(|_| ())
}

fn validate_nonce(args_nonce: &str) -> Result<(), String> {
    if state().lock().unwrap().nonce.as_deref() != Some(args_nonce) {
        return Err("Stale task gallery nonce".to_string());
    }
    Ok(())
}

fn snapshot_window(args_nonce: &str, hwnd: &str) -> Result<TaskGalleryAuthorizedWindow, String> {
    let authorized = {
        let state = state().lock().unwrap();
        if state.nonce.as_deref() != Some(args_nonce) {
            return Err("Stale task gallery nonce".to_string());
        }
        state.windows_by_hwnd.get(hwnd).cloned()
    }
    .ok_or_else(|| "Task gallery hwnd not allowed".to_string())?;
    #[cfg(target_os = "windows")]
    {
        let current = task_windows::task_window_identity(hwnd)?;
        if current != authorized.identity {
            return Err("Task gallery window identity changed".to_string());
        }
    }
    Ok(authorized)
}

fn hide_task_gallery_window(gallery: &WebviewWindow) {
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = gallery.hwnd() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        return;
    }
    let _ = gallery.hide();
}

fn hide_gallery_and_reset(app_handle: &AppHandle) {
    if let Some(gallery) = app_handle.get_webview_window(TASK_GALLERY_LABEL) {
        hide_task_gallery_window(&gallery);
    }
    let nonce = {
        let mut state = state().lock().unwrap();
        let nonce = state.nonce.take();
        state.windows_by_hwnd.clear();
        state.focus_loss_hold_count = 0;
        nonce
    };
    if nonce.is_some() {
        let preview_state = app_handle.state::<Mutex<task_preview::TaskPreviewRuntimeState>>();
        if let Ok(request_id) = task_preview::next_task_preview_request_id(&preview_state) {
            let _ = task_preview::hide_task_window_preview(
                app_handle.clone(),
                preview_state,
                request_id,
            );
        }
        emit_closed_event(app_handle, nonce);
    }
}

fn task_gallery_width_logical(window_count: usize, monitor_width_logical: f64) -> f64 {
    let tab_count = window_count.max(1) as f64;
    let derived_width = tab_count * TASK_GALLERY_WIDTH_LOGICAL
        + ((tab_count - 1.0).max(0.0) * TASK_GALLERY_TAB_GAP_LOGICAL)
        + (TASK_GALLERY_EDGE_PADDING_LOGICAL * 2.0);
    let min_width = TASK_GALLERY_WIDTH_LOGICAL + (TASK_GALLERY_EDGE_PADDING_LOGICAL * 2.0);
    let max_width = monitor_width_logical.max(min_width);
    derived_width.clamp(min_width, max_width)
}

fn show_task_gallery_window(gallery: &WebviewWindow, activate: bool) -> Result<(), String> {
    if activate {
        return gallery
            .show()
            .map_err(|error| format!("Failed to show task gallery: {error}"));
    }
    #[cfg(target_os = "windows")]
    unsafe {
        let hwnd = gallery
            .hwnd()
            .map_err(|error| format!("Failed to read task gallery HWND: {error}"))?;
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    gallery
        .show()
        .map_err(|error| format!("Failed to show task gallery: {error}"))
}

#[tauri::command]
pub fn show_task_gallery(
    window: WebviewWindow,
    app_handle: AppHandle,
    args: TaskGalleryShowArgs,
) -> Result<(), String> {
    if window.label() != BOTTOM_BAR_LABEL {
        return Err("Unauthorized caller for command show_task_gallery".to_string());
    }
    {
        let mut runtime = state().lock().unwrap();
        if take_cancelled_nonce(&mut runtime, &args.nonce) {
            return Ok(());
        }
        if args.refresh_existing && runtime.nonce.as_deref() != Some(args.nonce.as_str()) {
            return Ok(());
        }
    }
    let gallery = app_handle
        .get_webview_window(TASK_GALLERY_LABEL)
        .ok_or_else(|| "Task gallery window is unavailable".to_string())?;
    let bottom_bar = app_handle
        .get_webview_window(BOTTOM_BAR_LABEL)
        .ok_or_else(|| "Bottom bar window is unavailable".to_string())?;
    let allowed_hwnds = args
        .windows
        .iter()
        .cloned()
        .map(|row| {
            let hwnd = row.hwnd.clone();
            #[cfg(target_os = "windows")]
            let identity = task_windows::task_window_identity(&hwnd)?;
            Ok((
                hwnd,
                TaskGalleryAuthorizedWindow {
                    row,
                    #[cfg(target_os = "windows")]
                    identity,
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let same_session = state().lock().unwrap().nonce.as_deref() == Some(args.nonce.as_str());
    {
        let mut runtime = state().lock().unwrap();
        if take_cancelled_nonce(&mut runtime, &args.nonce) {
            return Ok(());
        }
        if args.refresh_existing && runtime.nonce.as_deref() != Some(args.nonce.as_str()) {
            return Ok(());
        }
        if !same_session && runtime.nonce.is_some() {
            return Err("Another task gallery session is active".to_string());
        }
        runtime.nonce = Some(args.nonce.clone());
        runtime.windows_by_hwnd = allowed_hwnds.clone();
    }
    let scale_factor = match bottom_bar.scale_factor() {
        Ok(scale_factor) => scale_factor,
        Err(error) => {
            reset_state();
            return Err(format!("Failed to read bottom-bar scale factor: {error}"));
        }
    };
    let bottom_position = match bottom_bar.outer_position() {
        Ok(position) => position,
        Err(error) => {
            reset_state();
            return Err(format!("Failed to read bottom-bar position: {error}"));
        }
    };
    let anchor_mid = (args.anchor_left + (args.anchor_width / 2.0)) * scale_factor;
    let monitor = bottom_bar
        .current_monitor()
        .ok()
        .and_then(|monitor| monitor);
    let monitor_x = monitor
        .as_ref()
        .map(|monitor| monitor.position().x)
        .unwrap_or(bottom_position.x);
    let monitor_width = monitor
        .as_ref()
        .map(|monitor| monitor.size().width)
        .unwrap_or((TASK_GALLERY_WIDTH_LOGICAL * scale_factor).round() as u32);
    let monitor_width_logical = monitor
        .as_ref()
        .map(|monitor| monitor.size().width as f64 / scale_factor)
        .unwrap_or(
            bottom_bar
                .outer_size()
                .map(|size| size.width as f64 / scale_factor)
                .unwrap_or(TASK_GALLERY_WIDTH_LOGICAL * 3.0),
        );
    let width = (task_gallery_width_logical(args.windows.len(), monitor_width_logical)
        * scale_factor)
        .round() as u32;
    let height = bottom_bar
        .outer_size()
        .map(|size| size.height)
        .unwrap_or_else(|_| (TASK_GALLERY_HEIGHT_LOGICAL * scale_factor).round() as u32);
    let min_x = monitor_x;
    let max_x = monitor_x + monitor_width.saturating_sub(width) as i32;
    let x = (bottom_position.x + anchor_mid.round() as i32 - (width as i32 / 2))
        .clamp(min_x, max_x.max(min_x));
    let monitor_y = monitor
        .as_ref()
        .map(|monitor| monitor.position().y)
        .unwrap_or(bottom_position.y - height as i32);
    let y = (bottom_position.y - height as i32).max(monitor_y);
    if let Err(e) = gallery.set_size(PhysicalSize::new(width, height)) {
        reset_state();
        return Err(format!("Failed to size task gallery: {e}"));
    }
    if let Err(e) = gallery.set_position(PhysicalPosition::new(x, y)) {
        reset_state();
        return Err(format!("Failed to position task gallery: {e}"));
    }
    if state().lock().unwrap().nonce.as_deref() != Some(args.nonce.as_str()) {
        return Ok(());
    }
    if same_session {
        return app_handle
            .emit_to(
                TASK_GALLERY_LABEL,
                TASK_GALLERY_OPEN_EVENT,
                TaskGalleryOpenPayload {
                    nonce: args.nonce,
                    group_key: args.group_key,
                    label: args.label,
                    focus_gallery: args.focus_gallery,
                    windows: args.windows,
                },
            )
            .map_err(|e| format!("Failed to refresh task gallery: {e}"));
    }
    if let Err(e) = show_task_gallery_window(&gallery, args.focus_gallery) {
        reset_state();
        return Err(e);
    }
    if args.focus_gallery {
        if let Err(e) = gallery.set_focus() {
            hide_task_gallery_window(&gallery);
            reset_state();
            return Err(format!("Failed to focus task gallery: {e}"));
        }
    }
    if let Err(e) = app_handle.emit_to(
        TASK_GALLERY_LABEL,
        TASK_GALLERY_OPEN_EVENT,
        TaskGalleryOpenPayload {
            nonce: args.nonce,
            group_key: args.group_key,
            label: args.label,
            focus_gallery: args.focus_gallery,
            windows: args.windows,
        },
    ) {
        hide_task_gallery_window(&gallery);
        reset_state();
        return Err(format!("Failed to publish task gallery open event: {e}"));
    }
    Ok(())
}

#[tauri::command]
pub fn hide_task_gallery(
    window: Window,
    app_handle: AppHandle,
    nonce: Option<String>,
) -> Result<(), String> {
    if window.label() != BOTTOM_BAR_LABEL && window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command hide_task_gallery".to_string());
    }
    if let Some(nonce) = nonce {
        record_cancelled_nonce(&mut state().lock().unwrap(), nonce);
    }
    hide_gallery_and_reset(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn hide_task_gallery_on_focus_loss(
    window: Window,
    app_handle: AppHandle,
) -> Result<(), String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command hide_task_gallery_on_focus_loss".to_string());
    }
    if focus_hold_active() {
        return Ok(());
    }
    hide_task_gallery(window, app_handle, None)
}

#[tauri::command]
pub fn activate_task_gallery_window(
    window: WebviewWindow,
    app_handle: AppHandle,
    args: TaskGalleryActivateArgs,
) -> Result<(), String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command activate_task_gallery_window".to_string());
    }
    validate(&args.nonce, &args.hwnd)?;
    let result = task_windows::activate_task_window(args.hwnd, args.minimize_if_active);
    hide_gallery_and_reset(&app_handle);
    result
}

#[tauri::command]
pub fn show_task_gallery_window_context_menu(
    window: WebviewWindow,
    app_handle: AppHandle,
    args: TaskGalleryContextMenuArgs,
) -> Result<(), String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err(
            "Unauthorized caller for command show_task_gallery_window_context_menu".to_string(),
        );
    }
    let authorized = snapshot_window(&args.nonce, &args.hwnd)?;
    begin_focus_hold();
    let result = taskbar_menu::show_task_window_context_menu_for_owner(
        &app_handle,
        TASK_GALLERY_LABEL,
        taskbar_menu::ShowTaskWindowContextMenuRequest {
            hwnd: authorized.row.hwnd,
            process_id: Some(authorized.identity.process_id),
            is_minimized: authorized.row.is_minimized,
            x: args.x,
            y: args.y,
        },
    );
    end_focus_hold();
    let gallery_focused = app_handle
        .get_webview_window(TASK_GALLERY_LABEL)
        .and_then(|gallery| gallery.is_focused().ok())
        .unwrap_or(false);
    if !gallery_focused {
        hide_gallery_and_reset(&app_handle);
    }
    result
}

#[tauri::command]
pub fn show_task_gallery_window_preview(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: tauri::State<'_, Mutex<task_preview::TaskPreviewRuntimeState>>,
    args: TaskGalleryPreviewArgs,
) -> Result<(), String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command show_task_gallery_window_preview".to_string());
    }
    validate(&args.nonce, &args.hwnd)?;
    let gallery = app_handle
        .get_webview_window(TASK_GALLERY_LABEL)
        .ok_or_else(|| "Task gallery window is unavailable".to_string())?;
    task_preview::show_task_window_preview_with_host(
        &app_handle,
        &gallery,
        state,
        task_preview::ShowTaskPreviewRequest {
            request_id: args.request_id,
            hwnd: args.hwnd,
            title: args.title,
            process_name: args.process_name,
            icon_data_url: args.icon_data_url,
            is_minimized: args.is_minimized,
            anchor_left: args.anchor_left,
            anchor_width: args.anchor_width,
            gallery_nonce: Some(args.nonce),
        },
    )
}

#[tauri::command]
pub fn close_task_gallery_previewed_window(
    window: WebviewWindow,
    args: TaskGalleryClosePreviewedWindowArgs,
) -> Result<(), String> {
    if window.label() != crate::shell_windows::TASK_PREVIEW_LABEL {
        return Err(
            "Unauthorized caller for command close_task_gallery_previewed_window".to_string(),
        );
    }
    let authorized = snapshot_window(&args.nonce, &args.hwnd)?;
    task_windows::close_task_window_with_identity(authorized.row.hwnd, authorized.identity)?;
    state().lock().unwrap().windows_by_hwnd.remove(&args.hwnd);
    Ok(())
}

#[tauri::command]
pub fn hide_task_gallery_window_preview(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: tauri::State<'_, Mutex<task_preview::TaskPreviewRuntimeState>>,
    nonce: String,
    hwnd: String,
    request_id: u64,
) -> Result<(), String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command hide_task_gallery_window_preview".to_string());
    }
    let _ = hwnd;
    validate_nonce(&nonce)?;
    task_preview::hide_task_window_preview(app_handle, state, request_id)
}

#[cfg(test)]
mod tests {
    use super::task_gallery_width_logical;

    #[test]
    fn task_gallery_width_scales_from_tab_count_and_clamps_to_monitor() {
        assert_eq!(task_gallery_width_logical(1, 1920.0), 160.0);
        assert_eq!(task_gallery_width_logical(4, 1920.0), 640.0);
        assert_eq!(task_gallery_width_logical(20, 900.0), 900.0);
    }
}
