use crate::shell_windows::{
    BOTTOM_BAR_LABEL, TASK_PREVIEW_HEIGHT_LOGICAL, TASK_PREVIEW_LABEL, TASK_PREVIEW_WIDTH_LOGICAL,
};
use crate::task_windows;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

const TASK_PREVIEW_UPDATE_EVENT: &str = "task-preview:update";
const TASK_PREVIEW_HIDE_EVENT: &str = "task-preview:hide";
const TASK_PREVIEW_MARGIN_PHYSICAL: i32 = 10;
const TASK_PREVIEW_EDGE_PADDING_PHYSICAL: i32 = 8;

#[derive(Default)]
pub struct TaskPreviewRuntimeState {
    pub latest_request_id: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowTaskPreviewRequest {
    pub request_id: u64,
    pub hwnd: String,
    pub title: String,
    pub process_name: String,
    pub icon_data_url: String,
    pub is_minimized: bool,
    pub anchor_left: f64,
    pub anchor_width: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskPreviewPayload {
    hwnd: String,
    title: String,
    process_name: String,
    icon_data_url: String,
    is_minimized: bool,
    image_data_url: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    error: Option<String>,
}

#[tauri::command]
pub fn show_task_window_preview(
    app_handle: AppHandle,
    state: tauri::State<'_, Mutex<TaskPreviewRuntimeState>>,
    request: ShowTaskPreviewRequest,
) -> Result<(), String> {
    let preview_window = app_handle
        .get_webview_window(TASK_PREVIEW_LABEL)
        .ok_or_else(|| "Task preview window is unavailable".to_string())?;
    let _ = preview_window.hide();

    {
        let mut state = state
            .lock()
            .expect("task preview runtime state is poisoned");
        state.latest_request_id = request.request_id;
    }

    let preview_image = task_windows::capture_task_window_preview(request.hwnd.clone());

    {
        let state = state
            .lock()
            .expect("task preview runtime state is poisoned");
        if state.latest_request_id != request.request_id {
            return Ok(());
        }
    }

    let bottom_bar = app_handle
        .get_webview_window(BOTTOM_BAR_LABEL)
        .ok_or_else(|| "Bottom bar window is unavailable".to_string())?;
    let scale_factor = bottom_bar
        .scale_factor()
        .map_err(|error| format!("Failed to read the taskbar scale factor: {error}"))?;
    let bottom_position = bottom_bar
        .outer_position()
        .map_err(|error| format!("Failed to read the taskbar position: {error}"))?;
    let bottom_size = bottom_bar
        .outer_size()
        .map_err(|error| format!("Failed to read the taskbar size: {error}"))?;
    let preview_width = (TASK_PREVIEW_WIDTH_LOGICAL * scale_factor).round() as i32;
    let preview_height = (TASK_PREVIEW_HEIGHT_LOGICAL * scale_factor).round() as i32;
    let anchor_midpoint = request.anchor_left + (request.anchor_width / 2.0);
    let anchor_midpoint_physical =
        bottom_position.x + (anchor_midpoint * scale_factor).round() as i32;
    let min_x = bottom_position.x + TASK_PREVIEW_EDGE_PADDING_PHYSICAL;
    let max_x = bottom_position.x + bottom_size.width as i32
        - preview_width
        - TASK_PREVIEW_EDGE_PADDING_PHYSICAL;
    let preview_x = (anchor_midpoint_physical - (preview_width / 2)).clamp(min_x, max_x.max(min_x));
    let preview_y = bottom_position.y - preview_height - TASK_PREVIEW_MARGIN_PHYSICAL;
    let payload = match preview_image {
        Ok(preview_image) => TaskPreviewPayload {
            hwnd: request.hwnd,
            title: request.title,
            process_name: request.process_name,
            icon_data_url: request.icon_data_url,
            is_minimized: request.is_minimized,
            image_data_url: Some(preview_image.image_data_url),
            width: Some(preview_image.width),
            height: Some(preview_image.height),
            error: None,
        },
        Err(error) => TaskPreviewPayload {
            hwnd: request.hwnd,
            title: request.title,
            process_name: request.process_name,
            icon_data_url: request.icon_data_url,
            is_minimized: request.is_minimized,
            image_data_url: None,
            width: None,
            height: None,
            error: Some(error),
        },
    };

    preview_window
        .emit(TASK_PREVIEW_UPDATE_EVENT, payload)
        .map_err(|error| format!("Failed to publish task preview data: {error}"))?;
    preview_window
        .set_position(PhysicalPosition::new(preview_x, preview_y))
        .map_err(|error| format!("Failed to position the task preview window: {error}"))?;
    preview_window
        .show()
        .map_err(|error| format!("Failed to show the task preview window: {error}"))
}

#[tauri::command]
pub fn hide_task_window_preview(
    app_handle: AppHandle,
    state: tauri::State<'_, Mutex<TaskPreviewRuntimeState>>,
    request_id: u64,
) -> Result<(), String> {
    {
        let mut state = state
            .lock()
            .expect("task preview runtime state is poisoned");
        state.latest_request_id = request_id;
    }

    let preview_window = app_handle
        .get_webview_window(TASK_PREVIEW_LABEL)
        .ok_or_else(|| "Task preview window is unavailable".to_string())?;
    preview_window
        .emit(TASK_PREVIEW_HIDE_EVENT, ())
        .map_err(|error| format!("Failed to clear task preview data: {error}"))?;
    preview_window
        .hide()
        .map_err(|error| format!("Failed to hide the task preview window: {error}"))
}
