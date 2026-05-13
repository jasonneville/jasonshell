use crate::shell_windows::{ensure_shell_window, STACK_POPUP_LABEL, TOP_BAR_LABEL};
use crate::stack_popup::models::{
    ShowStackPopupRequest, StackPopupLogicalSize, StackPopupRuntimeState,
};
use crate::stack_popup::paths::normalize_existing_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State};

const STACK_POPUP_WIDTH_LOGICAL: f64 = 980.0;
const STACK_POPUP_HEIGHT_RATIO: f64 = 0.35;
const STACK_POPUP_MIN_WIDTH_LOGICAL: f64 = 560.0;
const STACK_POPUP_MIN_HEIGHT_LOGICAL: f64 = 280.0;
const STACK_POPUP_GEOMETRY_SCHEMA: &str = "jasonshell.stackPopupGeometry";
const STACK_POPUP_GEOMETRY_VERSION: u32 = 1;
const STACK_POPUP_GEOMETRY_FILE: &str = "stack-popup-geometry-v1.json";
const EDGE_PADDING_PHYSICAL: i32 = 8;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StackPopupGeometryFile {
    schema: String,
    version: u32,
    size: StackPopupLogicalSize,
}

pub(crate) fn show_stack_popup_window(
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    request: ShowStackPopupRequest,
) -> Result<(), String> {
    let request = normalize_show_stack_popup_request(request)?;
    store_latest_request(&state, request.clone());

    let popup = ensure_shell_window(&app_handle, STACK_POPUP_LABEL)
        .map_err(|error| format!("Stack popup window is unavailable: {error}"))?;
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

    let default_size = default_stack_popup_size(monitor_size.height, scale_factor);
    let requested_size = load_stack_popup_size(&app_handle).unwrap_or(default_size);
    let y = top_position.y + top_size.height as i32;
    let available_height = available_height_below_y(monitor_position.y, monitor_size.height, y);
    let clamped_size = clamp_stack_popup_size(
        requested_size,
        scale_factor,
        monitor_size.width,
        available_height,
    );
    let width = logical_to_physical(clamped_size.width, scale_factor);
    let height = logical_to_physical(clamped_size.height, scale_factor);
    let anchor_right = top_position.x
        + ((request.anchor_left + request.anchor_width) * scale_factor).round() as i32;
    let min_x = monitor_position.x + EDGE_PADDING_PHYSICAL;
    let max_x =
        monitor_position.x + monitor_size.width as i32 - width as i32 - EDGE_PADDING_PHYSICAL;
    let x = (anchor_right - width as i32).clamp(min_x, max_x.max(min_x));

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
    let state = app_handle.state::<Mutex<StackPopupRuntimeState>>();
    {
        let mut guard = state.lock().expect("stack popup runtime state is poisoned");
        guard.focus_loss_hold_count = 0;
        guard.restore_focus_after_hold = false;
    }

    if let Some(popup) = app_handle.get_webview_window(STACK_POPUP_LABEL) {
        popup
            .hide()
            .map_err(|error| format!("Failed to hide the stack popup: {error}"))?;
    }
    Ok(())
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

pub(crate) fn suppress_stack_popup_focus_loss(app_handle: &AppHandle) -> bool {
    let state = app_handle.state::<Mutex<StackPopupRuntimeState>>();
    let mut guard = state.lock().expect("stack popup runtime state is poisoned");
    if guard.focus_loss_hold_count == 0 {
        return false;
    }
    guard.restore_focus_after_hold = true;
    true
}

pub(crate) fn begin_stack_popup_focus_hold(state: &State<'_, Mutex<StackPopupRuntimeState>>) {
    let mut guard = state.lock().expect("stack popup runtime state is poisoned");
    guard.focus_loss_hold_count += 1;
}

pub(crate) fn end_stack_popup_focus_hold(
    app_handle: &AppHandle,
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
) {
    let should_restore_focus = {
        let mut guard = state.lock().expect("stack popup runtime state is poisoned");
        guard.focus_loss_hold_count = guard.focus_loss_hold_count.saturating_sub(1);
        if guard.focus_loss_hold_count == 0 && guard.restore_focus_after_hold {
            guard.restore_focus_after_hold = false;
            true
        } else {
            false
        }
    };

    if should_restore_focus {
        if let Some(popup) = app_handle.get_webview_window(STACK_POPUP_LABEL) {
            let _ = popup.show();
            let _ = popup.set_focus();
        }
    }
}

pub(crate) fn resize_stack_popup_window(
    app_handle: AppHandle,
    width: f64,
    height: f64,
    persist: bool,
) -> Result<StackPopupLogicalSize, String> {
    let popup = ensure_shell_window(&app_handle, STACK_POPUP_LABEL)
        .map_err(|error| format!("Stack popup window is unavailable: {error}"))?;
    let monitor = popup
        .current_monitor()
        .map_err(|error| format!("Failed to inspect current monitor: {error}"))?
        .or_else(|| app_handle.primary_monitor().ok().flatten())
        .ok_or_else(|| "Primary monitor is unavailable".to_string())?;
    let scale_factor = monitor.scale_factor();
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let popup_position = popup
        .outer_position()
        .map_err(|error| format!("Failed to read stack popup position: {error}"))?;
    let available_width =
        available_width_from_x(monitor_position.x, monitor_size.width, popup_position.x);
    let available_height =
        available_height_below_y(monitor_position.y, monitor_size.height, popup_position.y);
    let clamped_size = clamp_stack_popup_size(
        StackPopupLogicalSize { width, height },
        scale_factor,
        available_width,
        available_height,
    );

    popup
        .set_size(PhysicalSize::new(
            logical_to_physical(clamped_size.width, scale_factor),
            logical_to_physical(clamped_size.height, scale_factor),
        ))
        .map_err(|error| format!("Failed to resize the stack popup: {error}"))?;

    if persist {
        save_stack_popup_size(&app_handle, clamped_size)?;
    }

    Ok(clamped_size)
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

fn default_stack_popup_size(
    monitor_height_physical: u32,
    scale_factor: f64,
) -> StackPopupLogicalSize {
    StackPopupLogicalSize {
        width: STACK_POPUP_WIDTH_LOGICAL,
        height: (monitor_height_physical as f64 * STACK_POPUP_HEIGHT_RATIO / scale_factor)
            .max(STACK_POPUP_MIN_HEIGHT_LOGICAL),
    }
}

fn clamp_stack_popup_size(
    size: StackPopupLogicalSize,
    scale_factor: f64,
    available_width_physical: u32,
    available_height_physical: u32,
) -> StackPopupLogicalSize {
    let max_width = (available_width_physical.saturating_sub((EDGE_PADDING_PHYSICAL * 2) as u32)
        as f64
        / scale_factor)
        .max(STACK_POPUP_MIN_WIDTH_LOGICAL);
    let max_height = (available_height_physical.saturating_sub(EDGE_PADDING_PHYSICAL as u32)
        as f64
        / scale_factor)
        .max(STACK_POPUP_MIN_HEIGHT_LOGICAL);

    StackPopupLogicalSize {
        width: finite_or_default(size.width, STACK_POPUP_WIDTH_LOGICAL)
            .clamp(STACK_POPUP_MIN_WIDTH_LOGICAL, max_width),
        height: finite_or_default(size.height, STACK_POPUP_MIN_HEIGHT_LOGICAL)
            .clamp(STACK_POPUP_MIN_HEIGHT_LOGICAL, max_height),
    }
}

fn finite_or_default(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn logical_to_physical(value: f64, scale_factor: f64) -> u32 {
    ((value * scale_factor).round() as u32).max(1)
}

fn available_width_from_x(monitor_x: i32, monitor_width: u32, popup_x: i32) -> u32 {
    let monitor_right = monitor_x + monitor_width as i32;
    monitor_right
        .saturating_sub(popup_x)
        .max(EDGE_PADDING_PHYSICAL) as u32
}

fn available_height_below_y(monitor_y: i32, monitor_height: u32, popup_y: i32) -> u32 {
    let monitor_bottom = monitor_y + monitor_height as i32;
    monitor_bottom
        .saturating_sub(popup_y)
        .max(EDGE_PADDING_PHYSICAL) as u32
}

fn load_stack_popup_size(app_handle: &AppHandle) -> Option<StackPopupLogicalSize> {
    let path = stack_popup_geometry_path(app_handle)?;
    load_stack_popup_size_from_path(&path).ok()
}

fn save_stack_popup_size(
    app_handle: &AppHandle,
    size: StackPopupLogicalSize,
) -> Result<(), String> {
    let Some(path) = stack_popup_geometry_path(app_handle) else {
        return Ok(());
    };
    save_stack_popup_size_to_path(&path, size)
}

fn load_stack_popup_size_from_path(path: &Path) -> Result<StackPopupLogicalSize, String> {
    if !path.exists() {
        return Err("stack popup geometry is not persisted".to_string());
    }
    let bytes =
        fs::read(path).map_err(|error| format!("Failed to read stack popup geometry: {error}"))?;
    let file = match serde_json::from_slice::<StackPopupGeometryFile>(&bytes) {
        Ok(file) => file,
        Err(error) => {
            backup_corrupt_stack_popup_geometry(path)?;
            return Err(format!("Failed to parse stack popup geometry: {error}"));
        }
    };
    if file.schema != STACK_POPUP_GEOMETRY_SCHEMA || file.version != STACK_POPUP_GEOMETRY_VERSION {
        return Err("Unsupported stack popup geometry version".to_string());
    }
    Ok(file.size)
}

fn save_stack_popup_size_to_path(path: &Path, size: StackPopupLogicalSize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create stack popup geometry directory: {error}"))?;
    }
    let file = StackPopupGeometryFile {
        schema: STACK_POPUP_GEOMETRY_SCHEMA.to_string(),
        version: STACK_POPUP_GEOMETRY_VERSION,
        size,
    };
    let bytes = serde_json::to_vec_pretty(&file)
        .map_err(|error| format!("Failed to serialize stack popup geometry: {error}"))?;
    write_file_atomic(path, &bytes)
        .map_err(|error| format!("Failed to write stack popup geometry: {error}"))
}

fn backup_corrupt_stack_popup_geometry(path: &Path) -> Result<(), String> {
    let timestamp = UNIX_EPOCH
        .elapsed()
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let backup = path.with_extension(format!("json.corrupt-{timestamp}"));
    fs::rename(path, backup)
        .map_err(|error| format!("Failed to back up corrupt stack popup geometry: {error}"))
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, bytes)?;
    atomic_rename(&temp_path, path)
}

#[cfg(windows)]
fn atomic_rename(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: `source` and `destination` are NUL-terminated UTF-16 path buffers
    // that remain alive for the duration of the MoveFileExW call.
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(io::Error::other)
    }
}

#[cfg(not(windows))]
fn atomic_rename(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

fn stack_popup_geometry_path(app_handle: &AppHandle) -> Option<PathBuf> {
    app_handle
        .path()
        .app_local_data_dir()
        .ok()
        .map(|dir| dir.join(STACK_POPUP_GEOMETRY_FILE))
}

#[cfg(test)]
mod tests {
    use super::{
        clamp_stack_popup_size, load_stack_popup_size_from_path, save_stack_popup_size_to_path,
        StackPopupLogicalSize,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(name: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        std::env::temp_dir().join(format!("jasonshell-stack-popup-{name}-{timestamp}.json"))
    }

    #[test]
    fn stack_popup_size_clamps_to_sane_monitor_bounds() {
        assert_eq!(
            clamp_stack_popup_size(
                StackPopupLogicalSize {
                    width: 100.0,
                    height: 100.0,
                },
                1.0,
                1920,
                1080,
            ),
            StackPopupLogicalSize {
                width: 560.0,
                height: 280.0,
            }
        );
        assert_eq!(
            clamp_stack_popup_size(
                StackPopupLogicalSize {
                    width: 4000.0,
                    height: 3000.0,
                },
                1.0,
                1000,
                700,
            ),
            StackPopupLogicalSize {
                width: 984.0,
                height: 692.0,
            }
        );
    }

    #[test]
    fn stack_popup_size_roundtrips_through_geometry_file() {
        let path = test_path("geometry");
        let size = StackPopupLogicalSize {
            width: 1234.0,
            height: 567.0,
        };

        save_stack_popup_size_to_path(&path, size).unwrap();

        assert_eq!(load_stack_popup_size_from_path(&path).unwrap(), size);
        let _ = fs::remove_file(path);
    }
}
