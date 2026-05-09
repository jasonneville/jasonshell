use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

use crate::shell_windows::{TERMINAL_PANEL_LABEL, TERMINAL_PANEL_WIDTH_LOGICAL, TOP_BAR_LABEL};

pub const TERMINAL_PANEL_CLOSED_EVENT: &str = "terminal-panel:closed";
const TERMINAL_PANEL_MARGIN_PHYSICAL: i32 = 6;
const TERMINAL_PANEL_EDGE_PADDING_PHYSICAL: i32 = 8;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowTerminalPanelRequest {
    pub anchor_left: f64,
    pub anchor_width: f64,
}

#[tauri::command]
pub fn show_terminal_panel(
    app_handle: AppHandle,
    request: ShowTerminalPanelRequest,
) -> Result<(), String> {
    let panel = app_handle
        .get_webview_window(TERMINAL_PANEL_LABEL)
        .ok_or_else(|| "Terminal panel window is unavailable".to_string())?;
    let top_bar = app_handle
        .get_webview_window(TOP_BAR_LABEL)
        .ok_or_else(|| "Top bar window is unavailable".to_string())?;

    let scale_factor = top_bar
        .scale_factor()
        .map_err(|error| format!("Failed to read the top bar scale factor: {error}"))?;
    let top_position = top_bar
        .outer_position()
        .map_err(|error| format!("Failed to read the top bar position: {error}"))?;
    let top_size = top_bar
        .outer_size()
        .map_err(|error| format!("Failed to read the top bar size: {error}"))?;
    let panel_width = (TERMINAL_PANEL_WIDTH_LOGICAL * scale_factor).round() as i32;
    let panel_x = anchored_terminal_panel_x(
        top_position.x,
        top_size.width as i32,
        request.anchor_left,
        request.anchor_width,
        panel_width,
        scale_factor,
    );
    let panel_y = top_position.y + top_size.height as i32 + TERMINAL_PANEL_MARGIN_PHYSICAL;

    panel
        .set_position(PhysicalPosition::new(panel_x, panel_y))
        .map_err(|error| format!("Failed to position the terminal panel: {error}"))?;
    panel
        .show()
        .map_err(|error| format!("Failed to show the terminal panel: {error}"))?;
    panel
        .set_focus()
        .map_err(|error| format!("Failed to focus the terminal panel: {error}"))
}

#[tauri::command]
pub fn hide_terminal_panel(app_handle: AppHandle) -> Result<(), String> {
    let panel = app_handle
        .get_webview_window(TERMINAL_PANEL_LABEL)
        .ok_or_else(|| "Terminal panel window is unavailable".to_string())?;
    panel
        .hide()
        .map_err(|error| format!("Failed to hide the terminal panel: {error}"))?;
    app_handle
        .emit_to(TOP_BAR_LABEL, TERMINAL_PANEL_CLOSED_EVENT, ())
        .map_err(|error| format!("Failed to publish terminal panel closed event: {error}"))
}

fn anchored_terminal_panel_x(
    host_x: i32,
    host_width: i32,
    anchor_left: f64,
    anchor_width: f64,
    panel_width: i32,
    scale_factor: f64,
) -> i32 {
    let anchor_right = host_x + ((anchor_left + anchor_width) * scale_factor).round() as i32;
    let min_x = host_x + TERMINAL_PANEL_EDGE_PADDING_PHYSICAL;
    let max_x = host_x + host_width - panel_width - TERMINAL_PANEL_EDGE_PADDING_PHYSICAL;
    (anchor_right - panel_width).clamp(min_x, max_x.max(min_x))
}

#[cfg(test)]
mod tests {
    use super::anchored_terminal_panel_x;

    #[test]
    fn anchors_terminal_panel_to_button_right_edge() {
        assert_eq!(anchored_terminal_panel_x(0, 1_920, 1_420.0, 28.0, 860, 1.0), 588);
    }

    #[test]
    fn clamps_terminal_panel_inside_top_bar_edges() {
        assert_eq!(anchored_terminal_panel_x(0, 360, 4.0, 28.0, 860, 1.0), 8);
        assert_eq!(anchored_terminal_panel_x(0, 920, 910.0, 28.0, 860, 1.0), 52);
    }
}
