use crate::shell_windows::{STACK_POPUP_LABEL, TOP_BAR_LABEL};
use crate::stack_popup::models::{ShowStackPopupRequest, StackPopupRuntimeState};
use crate::stack_popup::paths::normalize_existing_dir;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State};

const STACK_POPUP_WIDTH_LOGICAL: f64 = 980.0;
const STACK_POPUP_HEIGHT_RATIO: f64 = 0.35;
const EDGE_PADDING_PHYSICAL: i32 = 8;

pub(crate) fn show_stack_popup_window(
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    request: ShowStackPopupRequest,
) -> Result<(), String> {
    let request = normalize_show_stack_popup_request(request)?;
    store_latest_request(&state, request.clone());

    let popup = app_handle
        .get_webview_window(STACK_POPUP_LABEL)
        .ok_or_else(|| "Stack popup window is unavailable".to_string())?;
    let top = app_handle
        .get_webview_window(TOP_BAR_LABEL)
        .ok_or_else(|| "Top bar window is unavailable".to_string())?;
    let monitor = top
        .current_monitor()
        .map_err(|error| format!("Failed to inspect current monitor: {error}"))?
        .or_else(|| app_handle.primary_monitor().ok().flatten())
        .ok_or_else(|| "Primary monitor is unavailable".to_string())?;
    let scale_factor = monitor.scale_factor();
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let top_position = top
        .outer_position()
        .map_err(|error| format!("Failed to read top bar position: {error}"))?;
    let top_size = top
        .outer_size()
        .map_err(|error| format!("Failed to read top bar size: {error}"))?;

    let width = ((STACK_POPUP_WIDTH_LOGICAL * scale_factor).round() as u32).min(
        monitor_size
            .width
            .saturating_sub((EDGE_PADDING_PHYSICAL * 2) as u32),
    );
    let height = ((monitor_size.height as f64 * STACK_POPUP_HEIGHT_RATIO).round() as u32)
        .max((240.0 * scale_factor).round() as u32);
    let anchor_right = top_position.x
        + ((request.anchor_left + request.anchor_width) * scale_factor).round() as i32;
    let min_x = monitor_position.x + EDGE_PADDING_PHYSICAL;
    let max_x =
        monitor_position.x + monitor_size.width as i32 - width as i32 - EDGE_PADDING_PHYSICAL;
    let x = (anchor_right - width as i32).clamp(min_x, max_x.max(min_x));
    let y = top_position.y + top_size.height as i32;

    popup
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| format!("Failed to size the stack popup: {error}"))?;
    popup
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| format!("Failed to position the stack popup: {error}"))?;
    popup
        .show()
        .map_err(|error| format!("Failed to show the stack popup: {error}"))?;
    popup
        .set_focus()
        .map_err(|error| format!("Failed to focus the stack popup: {error}"))?;
    popup
        .emit("stack-popup:open", request)
        .map_err(|error| format!("Failed to publish stack popup path: {error}"))
}

pub(crate) fn hide_stack_popup_window(app_handle: AppHandle) -> Result<(), String> {
    app_handle
        .get_webview_window(STACK_POPUP_LABEL)
        .ok_or_else(|| "Stack popup window is unavailable".to_string())?
        .hide()
        .map_err(|error| format!("Failed to hide the stack popup: {error}"))
}

pub(crate) fn latest_stack_popup_request(
    state: State<'_, Mutex<StackPopupRuntimeState>>,
) -> Option<ShowStackPopupRequest> {
    state
        .lock()
        .expect("stack popup runtime state is poisoned")
        .latest_request
        .clone()
}

fn store_latest_request(
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    request: ShowStackPopupRequest,
) {
    state
        .lock()
        .expect("stack popup runtime state is poisoned")
        .latest_request = Some(request);
}

pub(crate) fn normalize_show_stack_popup_request(
    request: ShowStackPopupRequest,
) -> Result<ShowStackPopupRequest, String> {
    let path = normalize_existing_dir(&request.path)?;
    Ok(ShowStackPopupRequest { path, ..request })
}
