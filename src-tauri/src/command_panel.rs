use serde::Deserialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

use crate::settings;
use crate::shell_windows::{COMMAND_PANEL_LABEL, TOP_BAR_LABEL};

pub const COMMAND_PANEL_CLOSED_EVENT: &str = "command-panel:closed";
const COMMAND_PANEL_MARGIN_PHYSICAL: i32 = 6;
static COMMAND_PANEL_FOCUS_LOSS_NONCE: AtomicU64 = AtomicU64::new(1);
static COMMAND_PANEL_SUPPRESS_NEXT_RESIZE_SAVE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowCommandPanelRequest {
    pub anchor_left: f64,
    pub anchor_width: f64,
}

#[tauri::command]
pub fn show_command_panel(
    app_handle: AppHandle,
    request: ShowCommandPanelRequest,
) -> Result<(), String> {
    invalidate_command_panel_focus_loss_nonce();
    let panel = app_handle
        .get_webview_window(COMMAND_PANEL_LABEL)
        .ok_or_else(|| "Command panel window is unavailable".to_string())?;
    let top_bar = app_handle
        .get_webview_window(TOP_BAR_LABEL)
        .ok_or_else(|| "Top bar window is unavailable".to_string())?;

    let top_position = top_bar
        .outer_position()
        .map_err(|error| format!("Failed to read the top bar position: {error}"))?;
    let top_size = top_bar
        .outer_size()
        .map_err(|error| format!("Failed to read the top bar size: {error}"))?;
    let monitor = top_bar
        .current_monitor()
        .map_err(|error| format!("Failed to inspect current monitor: {error}"))?
        .or_else(|| app_handle.primary_monitor().ok().flatten())
        .ok_or_else(|| "Primary monitor is unavailable".to_string())?;
    let work_area = monitor.work_area();
    let scale_factor = monitor.scale_factor();
    let shell_settings = settings::load_shell_settings_for_app(&app_handle)?;
    let saved_width = shell_settings.ui.command_panel_width_logical;
    let saved_height = shell_settings.ui.command_panel_height_logical;
    let desired_width = ((saved_width * scale_factor).round() as i32).max(1);
    let desired_height = ((saved_height * scale_factor).round() as i32).max(1);
    let desired_x = top_position.x
        + ((request.anchor_left + request.anchor_width) * scale_factor).round() as i32
        - desired_width;
    let (panel_x, panel_y, panel_width, panel_height) = command_panel_work_area(
        work_area.position,
        work_area.size,
        desired_width,
        desired_height,
        desired_x,
        top_position.y + top_size.height as i32 + COMMAND_PANEL_MARGIN_PHYSICAL,
    );

    let target_size = PhysicalSize::new(panel_width as u32, panel_height as u32);
    let current_size = panel
        .outer_size()
        .map_err(|error| format!("Failed to read the command panel size: {error}"))?;
    if current_size != target_size {
        suppress_next_command_panel_resize_save();
    }
    panel
        .set_size(target_size)
        .map_err(|error| format!("Failed to size the command panel: {error}"))?;
    panel
        .set_position(PhysicalPosition::new(panel_x, panel_y))
        .map_err(|error| format!("Failed to position the command panel: {error}"))?;
    panel
        .show()
        .map_err(|error| format!("Failed to show the command panel: {error}"))?;
    panel
        .set_focus()
        .map_err(|error| format!("Failed to focus the command panel: {error}"))
}

pub fn save_command_panel_size_for_app(
    app_handle: &AppHandle,
    width_physical: u32,
    height_physical: u32,
) -> Result<settings::ShellSettings, String> {
    if take_command_panel_resize_save_suppression() {
        return settings::load_shell_settings_for_app(app_handle);
    }
    let panel = app_handle
        .get_webview_window(COMMAND_PANEL_LABEL)
        .ok_or_else(|| "Command panel window is unavailable".to_string())?;
    let scale_factor = panel.scale_factor().unwrap_or(1.0);
    let width_logical = (width_physical as f64 / scale_factor).max(1.0);
    let height_logical = (height_physical as f64 / scale_factor).max(1.0);
    settings::update_shell_settings_for_app(app_handle, |settings| {
        settings.ui.command_panel_width_logical = width_logical;
        settings.ui.command_panel_height_logical = height_logical;
    })
}

#[tauri::command]
pub fn save_command_panel_size(
    app_handle: AppHandle,
    width_physical: u32,
    height_physical: u32,
) -> Result<settings::ShellSettings, String> {
    save_command_panel_size_for_app(&app_handle, width_physical, height_physical)
}

#[tauri::command]
pub fn hide_command_panel(app_handle: AppHandle) -> Result<(), String> {
    invalidate_command_panel_focus_loss_nonce();
    let panel = app_handle
        .get_webview_window(COMMAND_PANEL_LABEL)
        .ok_or_else(|| "Command panel window is unavailable".to_string())?;
    panel
        .hide()
        .map_err(|error| format!("Failed to hide the command panel: {error}"))?;
    app_handle
        .emit_to(TOP_BAR_LABEL, COMMAND_PANEL_CLOSED_EVENT, ())
        .map_err(|error| format!("Failed to publish command panel closed event: {error}"))
}

pub fn invalidate_command_panel_focus_loss_nonce() -> u64 {
    COMMAND_PANEL_FOCUS_LOSS_NONCE.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn current_command_panel_focus_loss_nonce() -> u64 {
    COMMAND_PANEL_FOCUS_LOSS_NONCE.load(Ordering::SeqCst)
}

pub fn suppress_next_command_panel_resize_save() {
    COMMAND_PANEL_SUPPRESS_NEXT_RESIZE_SAVE.store(true, Ordering::SeqCst);
}

fn take_command_panel_resize_save_suppression() -> bool {
    COMMAND_PANEL_SUPPRESS_NEXT_RESIZE_SAVE.swap(false, Ordering::SeqCst)
}

pub fn command_panel_focus_loss_nonce_is_current(nonce: u64) -> bool {
    current_command_panel_focus_loss_nonce() == nonce
}

pub fn command_panel_work_area(
    work_area_position: PhysicalPosition<i32>,
    work_area_size: PhysicalSize<u32>,
    panel_width: i32,
    panel_height: i32,
    x: i32,
    below_y: i32,
) -> (i32, i32, i32, i32) {
    let work_area_left = work_area_position.x;
    let work_area_top = work_area_position.y;
    let work_area_right = work_area_left + work_area_size.width as i32;
    let work_area_bottom = work_area_top + work_area_size.height as i32;
    let available_width = (work_area_right - work_area_left).max(1);
    let available_height = (work_area_bottom - work_area_top).max(1);
    let panel_width = panel_width.max(1).min(available_width);
    let panel_height = panel_height.max(1).min(available_height);
    let min_x = work_area_left;
    let max_x = work_area_right - panel_width;
    let panel_x = x.clamp(min_x, max_x);

    let below_fits = below_y + panel_height <= work_area_bottom;
    let above_y = (below_y - panel_height).max(work_area_top);
    let panel_y = if below_fits {
        below_y.clamp(work_area_top, work_area_bottom - panel_height)
    } else if above_y >= work_area_top {
        above_y
    } else {
        work_area_bottom - panel_height
    }
    .clamp(work_area_top, work_area_bottom - panel_height);

    (panel_x, panel_y, panel_width, panel_height)
}

#[cfg(test)]
mod tests {
    use super::{
        command_panel_focus_loss_nonce_is_current, command_panel_work_area,
        current_command_panel_focus_loss_nonce, invalidate_command_panel_focus_loss_nonce,
        take_command_panel_resize_save_suppression,
        suppress_next_command_panel_resize_save,
    };
    use tauri::{PhysicalPosition, PhysicalSize};

    #[test]
    fn places_command_panel_within_work_area_and_keeps_width_when_it_fits() {
        assert_eq!(
            command_panel_work_area(
                PhysicalPosition::new(100, 50),
                PhysicalSize::new(1_920, 1_080),
                460,
                320,
                1_650,
                80,
            ),
            (1_560, 80, 460, 320)
        );
    }

    #[test]
    fn shrinks_command_panel_before_positioning_when_work_area_is_too_narrow() {
        assert_eq!(
            command_panel_work_area(
                PhysicalPosition::new(-320, 0),
                PhysicalSize::new(280, 800),
                460,
                900,
                -200,
                20,
            ),
            (-320, 0, 280, 800)
        );
    }

    #[test]
    fn clamps_command_panel_height_to_work_area() {
        assert_eq!(
            command_panel_work_area(
                PhysicalPosition::new(10, -40),
                PhysicalSize::new(400, 180),
                220,
                260,
                180,
                110,
            ),
            (180, -40, 220, 180)
        );
    }

    #[test]
    fn clamps_saved_command_panel_size_to_monitor_work_area() {
        assert_eq!(
            command_panel_work_area(
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(320, 240),
                900,
                800,
                10,
                20,
            ),
            (0, 0, 320, 240)
        );
    }

    #[test]
    fn invalidate_command_panel_focus_loss_nonce_returns_current_nonce_until_next_invalidation() {
        let first = invalidate_command_panel_focus_loss_nonce();
        assert_eq!(current_command_panel_focus_loss_nonce(), first);
        assert!(command_panel_focus_loss_nonce_is_current(first));

        let second = invalidate_command_panel_focus_loss_nonce();
        assert_eq!(current_command_panel_focus_loss_nonce(), second);
        assert_ne!(first, second);
        assert!(!command_panel_focus_loss_nonce_is_current(first));
        assert!(command_panel_focus_loss_nonce_is_current(second));
    }

    #[test]
    fn suppress_next_resize_save_consumes_single_resize_event() {
        suppress_next_command_panel_resize_save();
        assert!(take_command_panel_resize_save_suppression());
        assert!(!take_command_panel_resize_save_suppression());
    }
}
