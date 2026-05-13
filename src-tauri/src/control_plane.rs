use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize};

use crate::shell_windows::{
    ensure_shell_window,
    CONTROL_PLANE_HEIGHT_LOGICAL, CONTROL_PLANE_LABEL, CONTROL_PLANE_WIDTH_LOGICAL,
};

const CONTROL_PLANE_EDGE_PADDING_PHYSICAL: i32 = 24;

#[tauri::command]
pub fn show_control_plane(app_handle: AppHandle) -> Result<(), String> {
    let panel = ensure_shell_window(&app_handle, CONTROL_PLANE_LABEL)
        .map_err(|error| format!("Control-plane window is unavailable: {error}"))?;
    let monitor = panel
        .current_monitor()
        .map_err(|error| format!("Failed to inspect control-plane monitor: {error}"))?
        .or_else(|| app_handle.primary_monitor().ok().flatten())
        .ok_or_else(|| "Primary monitor is unavailable".to_string())?;
    let scale_factor = monitor.scale_factor();
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let width = clamped_logical_size(
        CONTROL_PLANE_WIDTH_LOGICAL,
        scale_factor,
        monitor_size.width,
    );
    let height = clamped_logical_size(
        CONTROL_PLANE_HEIGHT_LOGICAL,
        scale_factor,
        monitor_size.height,
    );
    let x = centered_axis(
        monitor_position.x,
        monitor_size.width,
        width,
        CONTROL_PLANE_EDGE_PADDING_PHYSICAL,
    );
    let y = centered_axis(
        monitor_position.y,
        monitor_size.height,
        height,
        CONTROL_PLANE_EDGE_PADDING_PHYSICAL,
    );

    panel
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| format!("Failed to size the control plane: {error}"))?;
    panel
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| format!("Failed to position the control plane: {error}"))?;
    panel
        .show()
        .map_err(|error| format!("Failed to show the control plane: {error}"))?;
    panel
        .set_focus()
        .map_err(|error| format!("Failed to focus the control plane: {error}"))
}

#[tauri::command]
pub fn hide_control_plane(app_handle: AppHandle) -> Result<(), String> {
    if let Some(panel) = app_handle.get_webview_window(CONTROL_PLANE_LABEL) {
        panel
            .hide()
            .map_err(|error| format!("Failed to hide the control plane: {error}"))?;
    }
    Ok(())
}

fn clamped_logical_size(logical_size: f64, scale_factor: f64, monitor_size: u32) -> u32 {
    let requested = (logical_size * scale_factor).round() as u32;
    let max_size = monitor_size.saturating_sub((CONTROL_PLANE_EDGE_PADDING_PHYSICAL * 2) as u32);
    requested.min(max_size).max(1)
}

fn centered_axis(monitor_position: i32, monitor_size: u32, window_size: u32, padding: i32) -> i32 {
    let centered = monitor_position + ((monitor_size.saturating_sub(window_size)) / 2) as i32;
    let min = monitor_position + padding;
    let max = monitor_position + monitor_size as i32 - window_size as i32 - padding;
    centered.clamp(min, max.max(min))
}

#[cfg(test)]
mod tests {
    use super::{centered_axis, clamped_logical_size};

    #[test]
    fn control_plane_size_is_clamped_to_monitor_bounds() {
        assert_eq!(clamped_logical_size(860.0, 1.0, 1_920), 860);
        assert_eq!(clamped_logical_size(860.0, 1.0, 80), 32);
        assert_eq!(clamped_logical_size(860.0, 1.0, 1), 1);
    }

    #[test]
    fn control_plane_position_is_centered_inside_monitor_padding() {
        assert_eq!(centered_axis(0, 1_920, 860, 24), 530);
        assert_eq!(centered_axis(0, 120, 100, 24), 24);
        assert_eq!(centered_axis(-1_920, 1_920, 860, 24), -1_390);
    }
}
