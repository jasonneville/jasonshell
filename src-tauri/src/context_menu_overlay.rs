use crate::shell_windows::{BOTTOM_BAR_LABEL, CONTEXT_MENU_OVERLAY_LABEL, TOP_BAR_LABEL};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

const CONTEXT_MENU_OVERLAY_WIDTH_LOGICAL: f64 = 360.0;
const CONTEXT_MENU_OVERLAY_HEIGHT_LOGICAL: f64 = 360.0;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextMenuOverlayShowRequest {
    pub source: String,
    pub x: f64,
    #[allow(dead_code)]
    pub y: f64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextMenuOverlayHideRequest {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub restore_origin_focus: bool,
}

#[tauri::command]
pub fn show_context_menu_overlay(
    window: WebviewWindow,
    app_handle: AppHandle,
    request: ContextMenuOverlayShowRequest,
) -> Result<(), String> {
    if request.source != window.label()
        || !matches!(window.label(), TOP_BAR_LABEL | BOTTOM_BAR_LABEL)
    {
        return Err("Unauthorized caller for command show_context_menu_overlay".to_string());
    }

    let overlay = app_handle
        .get_webview_window(CONTEXT_MENU_OVERLAY_LABEL)
        .ok_or_else(|| "Context menu overlay window is unavailable".to_string())?;
    let scale_factor = window
        .scale_factor()
        .map_err(|error| format!("Failed to read context menu source scale factor: {error}"))?;
    let source_position = window
        .outer_position()
        .map_err(|error| format!("Failed to read context menu source position: {error}"))?;
    let source_size = window
        .outer_size()
        .map_err(|error| format!("Failed to read context menu source size: {error}"))?;
    let width = (CONTEXT_MENU_OVERLAY_WIDTH_LOGICAL * scale_factor).round() as u32;
    let height = (CONTEXT_MENU_OVERLAY_HEIGHT_LOGICAL * scale_factor).round() as u32;
    let monitor = window
        .current_monitor()
        .map_err(|error| format!("Failed to read context menu monitor: {error}"))?
        .ok_or_else(|| "Context menu monitor is unavailable".to_string())?;
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let max_x = monitor_position.x + monitor_size.width.saturating_sub(width) as i32;
    let max_y = monitor_position.y + monitor_size.height.saturating_sub(height) as i32;
    let requested_x = source_position.x + (request.x * scale_factor).round() as i32;
    let source_anchor_y = source_position.y + (request.y * scale_factor).round() as i32;
    let requested_y = if window.label() == TOP_BAR_LABEL {
        source_anchor_y + source_size.height as i32 - (request.y * scale_factor).round() as i32
    } else {
        source_anchor_y - height as i32
    };
    let x = requested_x.clamp(monitor_position.x, max_x.max(monitor_position.x));
    let y = requested_y.clamp(monitor_position.y, max_y.max(monitor_position.y));

    overlay
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| format!("Failed to size context menu overlay: {error}"))?;
    overlay
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| format!("Failed to position context menu overlay: {error}"))?;
    overlay
        .show()
        .map_err(|error| format!("Failed to show context menu overlay: {error}"))?;
    overlay
        .set_focus()
        .map_err(|error| format!("Failed to focus context menu overlay: {error}"))?;
    Ok(())
}

#[tauri::command]
pub fn hide_context_menu_overlay(
    window: WebviewWindow,
    app_handle: AppHandle,
    request: ContextMenuOverlayHideRequest,
) -> Result<(), String> {
    if window.label() != CONTEXT_MENU_OVERLAY_LABEL {
        return Err("Unauthorized caller for command hide_context_menu_overlay".to_string());
    }
    let overlay = app_handle
        .get_webview_window(CONTEXT_MENU_OVERLAY_LABEL)
        .ok_or_else(|| "Context menu overlay window is unavailable".to_string())?;
    overlay
        .hide()
        .map_err(|error| format!("Failed to hide context menu overlay: {error}"))?;

    if request.restore_origin_focus {
        let source = request
            .source
            .filter(|source| matches!(source.as_str(), TOP_BAR_LABEL | BOTTOM_BAR_LABEL))
            .ok_or_else(|| {
                "Invalid origin window for context menu focus restoration".to_string()
            })?;
        app_handle
            .get_webview_window(&source)
            .ok_or_else(|| "Context menu origin window is unavailable".to_string())?
            .set_focus()
            .map_err(|error| format!("Failed to restore context menu origin focus: {error}"))?;
    }
    Ok(())
}
