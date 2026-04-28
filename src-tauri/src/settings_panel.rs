use serde::Deserialize;
use tauri::{AppHandle, Manager, PhysicalPosition};

use crate::shell_windows::{
    SETTINGS_PANEL_LABEL, SETTINGS_PANEL_WIDTH_LOGICAL, TOP_BAR_LABEL,
};

const SETTINGS_PANEL_MARGIN_PHYSICAL: i32 = 6;
const SETTINGS_PANEL_EDGE_PADDING_PHYSICAL: i32 = 8;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowSettingsPanelRequest {
    pub anchor_left: f64,
    pub anchor_width: f64,
}

#[tauri::command]
pub fn show_settings_panel(
    app_handle: AppHandle,
    request: ShowSettingsPanelRequest,
) -> Result<(), String> {
    let panel = app_handle
        .get_webview_window(SETTINGS_PANEL_LABEL)
        .ok_or_else(|| "Settings panel window is unavailable".to_string())?;
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
    let panel_width = (SETTINGS_PANEL_WIDTH_LOGICAL * scale_factor).round() as i32;
    let panel_x = anchored_panel_x(
        top_position.x,
        top_size.width as i32,
        request.anchor_left,
        request.anchor_width,
        panel_width,
        scale_factor,
    );
    let panel_y = top_position.y + top_size.height as i32 + SETTINGS_PANEL_MARGIN_PHYSICAL;

    panel
        .set_position(PhysicalPosition::new(panel_x, panel_y))
        .map_err(|error| format!("Failed to position the settings panel: {error}"))?;
    panel
        .show()
        .map_err(|error| format!("Failed to show the settings panel: {error}"))?;
    panel
        .set_focus()
        .map_err(|error| format!("Failed to focus the settings panel: {error}"))
}

#[tauri::command]
pub fn hide_settings_panel(app_handle: AppHandle) -> Result<(), String> {
    let panel = app_handle
        .get_webview_window(SETTINGS_PANEL_LABEL)
        .ok_or_else(|| "Settings panel window is unavailable".to_string())?;
    panel
        .hide()
        .map_err(|error| format!("Failed to hide the settings panel: {error}"))
}

fn anchored_panel_x(
    host_x: i32,
    host_width: i32,
    anchor_left: f64,
    _anchor_width: f64,
    panel_width: i32,
    scale_factor: f64,
) -> i32 {
    let anchor_left = host_x + (anchor_left * scale_factor).round() as i32;
    let min_x = host_x + SETTINGS_PANEL_EDGE_PADDING_PHYSICAL;
    let max_x = host_x + host_width - panel_width - SETTINGS_PANEL_EDGE_PADDING_PHYSICAL;
    anchor_left.clamp(min_x, max_x.max(min_x))
}

#[cfg(test)]
mod tests {
    use super::anchored_panel_x;

    #[test]
    fn anchors_settings_panel_to_button_left_edge() {
        assert_eq!(anchored_panel_x(0, 1_920, 12.0, 96.0, 440, 1.0), 12);
    }

    #[test]
    fn clamps_settings_panel_inside_top_bar_edges() {
        assert_eq!(anchored_panel_x(0, 420, 4.0, 96.0, 440, 1.0), 8);
        assert_eq!(anchored_panel_x(0, 800, 760.0, 40.0, 440, 1.0), 352);
    }
}
