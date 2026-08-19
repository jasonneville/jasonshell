use crate::launchers::canonicalize_pinned_taskbar_shortcut_path;
use crate::shell_windows::{
    BOTTOM_BAR_LABEL, QUICK_LAUNCH_PANEL_HEIGHT_LOGICAL, QUICK_LAUNCH_PANEL_LABEL,
    QUICK_LAUNCH_PANEL_WIDTH_LOGICAL,
};
use crate::taskbar_menu;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, Window};

pub const QUICK_LAUNCH_PANEL_CLOSED_EVENT: &str = "quick-launch-panel:closed";
pub const QUICK_LAUNCH_PANEL_OPEN_EVENT: &str = "quick-launch-panel:open";

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLaunchPanelShowArgs {
    pub anchor_left: f64,
    pub anchor_width: f64,
    pub nonce: String,
    pub rows: Vec<QuickLaunchPanelRow>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLaunchPanelSelectArgs {
    pub nonce: String,
    pub shortcut_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLaunchPanelRunAsAdminArgs {
    pub nonce: String,
    pub shortcut_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLaunchPanelContextMenuArgs {
    pub nonce: String,
    pub shortcut_path: String,
    pub x: f64,
    pub y: f64,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QuickLaunchPanelOpenPayload {
    pub nonce: String,
    pub rows: Vec<QuickLaunchPanelRow>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QuickLaunchPanelRow {
    pub shortcut_path: String,
    pub name: String,
    pub icon_data_url: String,
    pub target_path: Option<String>,
}

#[derive(Default)]
struct QuickLaunchPanelRuntimeState {
    nonce: Option<String>,
    allowed_shortcuts: BTreeSet<PathBuf>,
    focus_loss_hold_count: u32,
}

fn state() -> &'static Mutex<QuickLaunchPanelRuntimeState> {
    static STATE: OnceLock<Mutex<QuickLaunchPanelRuntimeState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(QuickLaunchPanelRuntimeState::default()))
}

fn quick_launch_panel_state_reset() {
    let mut state = state().lock().unwrap();
    state.nonce = None;
    state.allowed_shortcuts.clear();
    state.focus_loss_hold_count = 0;
}

fn current_quick_launch_panel_nonce() -> Option<String> {
    state().lock().unwrap().nonce.clone()
}

fn begin_quick_launch_panel_focus_hold() {
    state().lock().unwrap().focus_loss_hold_count += 1;
}

fn end_quick_launch_panel_focus_hold() {
    let mut state = state().lock().unwrap();
    state.focus_loss_hold_count = state.focus_loss_hold_count.saturating_sub(1);
}

fn quick_launch_panel_focus_loss_held() -> bool {
    state().lock().unwrap().focus_loss_hold_count > 0
}

fn validate_quick_launch_panel_shortcut(
    args_nonce: &str,
    shortcut_path: &str,
) -> Result<PathBuf, String> {
    let (nonce, allowed_shortcuts) = {
        let state = state().lock().unwrap();
        (state.nonce.clone(), state.allowed_shortcuts.clone())
    };
    if nonce.as_deref() != Some(args_nonce) {
        return Err("Stale quick launch panel nonce".to_string());
    }
    let canonical_shortcut_path = canonicalize_pinned_taskbar_shortcut_path(shortcut_path)?;
    if !allowed_shortcuts.contains(&canonical_shortcut_path) {
        return Err("Quick launch shortcut not allowed".to_string());
    }
    Ok(canonical_shortcut_path)
}

#[tauri::command]
pub fn show_quick_launch_panel(
    window: WebviewWindow,
    app_handle: AppHandle,
    args: QuickLaunchPanelShowArgs,
) -> Result<(), String> {
    if window.label() != BOTTOM_BAR_LABEL {
        return Err("Unauthorized caller for command show_quick_launch_panel".to_string());
    }
    let panel = app_handle
        .get_webview_window(QUICK_LAUNCH_PANEL_LABEL)
        .ok_or_else(|| "Quick launch panel window is unavailable".to_string())?;
    let bottom_bar = app_handle
        .get_webview_window(BOTTOM_BAR_LABEL)
        .ok_or_else(|| "Bottom bar window is unavailable".to_string())?;
    let allowed_shortcuts = args
        .rows
        .iter()
        .map(|row| canonicalize_pinned_taskbar_shortcut_path(&row.shortcut_path))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let bottom_position = bottom_bar
        .outer_position()
        .map_err(|error| format!("Failed to read bottom-bar position: {error}"))?;
    let scale_factor = bottom_bar
        .scale_factor()
        .map_err(|error| format!("Failed to read bottom-bar scale factor: {error}"))?;
    let width = (QUICK_LAUNCH_PANEL_WIDTH_LOGICAL * scale_factor).round() as u32;
    let height = (QUICK_LAUNCH_PANEL_HEIGHT_LOGICAL * scale_factor).round() as u32;
    let screen_width = bottom_bar
        .current_monitor()
        .ok()
        .and_then(|monitor| monitor)
        .map(|monitor| monitor.size().width)
        .unwrap_or(width);
    let anchor_left = (args.anchor_left * scale_factor).round() as i32;
    let anchor_width = (args.anchor_width * scale_factor).round() as i32;
    let max_x = screen_width.saturating_sub(width) as i32;
    let work_area_origin_x = bottom_bar
        .current_monitor()
        .ok()
        .and_then(|monitor| monitor)
        .map(|monitor| monitor.position().x)
        .unwrap_or(0);
    let work_area_origin_y = bottom_bar
        .current_monitor()
        .ok()
        .and_then(|monitor| monitor)
        .map(|monitor| monitor.position().y)
        .unwrap_or(0);
    let x = (work_area_origin_x + anchor_left)
        .clamp(work_area_origin_x, work_area_origin_x + max_x.max(0))
        .min(
            (work_area_origin_x + anchor_left + anchor_width - width as i32)
                .max(work_area_origin_x),
        );
    let y = (bottom_position.y - height as i32).max(work_area_origin_y);
    panel
        .set_size(PhysicalSize::new(width, height))
        .map_err(|e| format!("Failed to size quick launch panel: {e}"))?;
    panel
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| format!("Failed to position quick launch panel: {e}"))?;
    if let Err(error) = panel.show() {
        quick_launch_panel_state_reset();
        return Err(format!("Failed to show quick launch panel: {error}"));
    }
    if let Err(error) = panel.set_focus() {
        let _ = panel.hide();
        quick_launch_panel_state_reset();
        return Err(format!("Failed to focus quick launch panel: {error}"));
    }
    {
        let mut state = state().lock().unwrap();
        state.nonce = Some(args.nonce.clone());
        state.allowed_shortcuts = allowed_shortcuts;
    }
    if let Err(error) = app_handle.emit_to(
        QUICK_LAUNCH_PANEL_LABEL,
        QUICK_LAUNCH_PANEL_OPEN_EVENT,
        QuickLaunchPanelOpenPayload {
            nonce: args.nonce.clone(),
            rows: args.rows,
        },
    ) {
        let _ = panel.hide();
        quick_launch_panel_state_reset();
        return Err(format!(
            "Failed to publish quick launch panel open event: {error}"
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn hide_quick_launch_panel(window: Window, app_handle: AppHandle) -> Result<(), String> {
    if window.label() != BOTTOM_BAR_LABEL && window.label() != QUICK_LAUNCH_PANEL_LABEL {
        return Err("Unauthorized caller for command hide_quick_launch_panel".to_string());
    }
    let panel = app_handle
        .get_webview_window(QUICK_LAUNCH_PANEL_LABEL)
        .ok_or_else(|| "Quick launch panel window is unavailable".to_string())?;
    panel
        .hide()
        .map_err(|error| format!("Failed to hide quick launch panel: {error}"))?;
    let nonce = current_quick_launch_panel_nonce();
    quick_launch_panel_state_reset();
    app_handle
        .emit_to(
            BOTTOM_BAR_LABEL,
            QUICK_LAUNCH_PANEL_CLOSED_EVENT,
            serde_json::json!({"nonce": nonce }),
        )
        .map_err(|error| format!("Failed to publish quick launch panel closed event: {error}"))?;
    app_handle
        .emit_to(
            QUICK_LAUNCH_PANEL_LABEL,
            QUICK_LAUNCH_PANEL_CLOSED_EVENT,
            serde_json::json!({"nonce": nonce }),
        )
        .map_err(|error| format!("Failed to publish quick launch panel closed event: {error}"))
}

#[tauri::command]
pub fn hide_quick_launch_panel_on_focus_loss(
    window: Window,
    app_handle: AppHandle,
) -> Result<(), String> {
    if window.label() != QUICK_LAUNCH_PANEL_LABEL {
        return Err(
            "Unauthorized caller for command hide_quick_launch_panel_on_focus_loss".to_string(),
        );
    }
    if quick_launch_panel_focus_loss_held() {
        return Ok(());
    }
    hide_quick_launch_panel(window, app_handle)
}

#[tauri::command]
pub fn select_quick_launch_panel(
    window: WebviewWindow,
    app_handle: AppHandle,
    args: QuickLaunchPanelSelectArgs,
) -> Result<(), String> {
    if window.label() != QUICK_LAUNCH_PANEL_LABEL {
        return Err("Unauthorized caller for command select_quick_launch_panel".to_string());
    }
    let shortcut_path =
        validate_quick_launch_panel_shortcut(args.nonce.as_str(), args.shortcut_path.as_str())?;
    crate::launchers::launch_pinned_taskbar_app_internal(
        shortcut_path.to_string_lossy().into_owned(),
    )?;
    let panel = app_handle
        .get_webview_window(QUICK_LAUNCH_PANEL_LABEL)
        .ok_or_else(|| "Quick launch panel window is unavailable".to_string())?;
    panel
        .hide()
        .map_err(|error| format!("Failed to hide quick launch panel after selection: {error}"))?;
    quick_launch_panel_state_reset();
    app_handle
        .emit_to(
            BOTTOM_BAR_LABEL,
            QUICK_LAUNCH_PANEL_CLOSED_EVENT,
            serde_json::json!({"nonce": args.nonce}),
        )
        .map_err(|error| format!("Failed to publish quick launch panel closed event: {error}"))?;
    app_handle
        .emit_to(
            QUICK_LAUNCH_PANEL_LABEL,
            QUICK_LAUNCH_PANEL_CLOSED_EVENT,
            serde_json::json!({"nonce": args.nonce}),
        )
        .map_err(|error| format!("Failed to publish quick launch panel closed event: {error}"))?;
    Ok(())
}

#[tauri::command]
pub fn run_quick_launch_panel_as_admin(
    window: WebviewWindow,
    app_handle: AppHandle,
    args: QuickLaunchPanelRunAsAdminArgs,
) -> Result<(), String> {
    if window.label() != QUICK_LAUNCH_PANEL_LABEL {
        return Err("Unauthorized caller for command run_quick_launch_panel_as_admin".to_string());
    }
    let shortcut_path =
        validate_quick_launch_panel_shortcut(args.nonce.as_str(), args.shortcut_path.as_str())?;
    let result = crate::launchers::run_pinned_taskbar_app_as_admin(
        shortcut_path.to_string_lossy().into_owned(),
    );
    end_quick_launch_panel_focus_hold();
    result?;
    let panel = app_handle
        .get_webview_window(QUICK_LAUNCH_PANEL_LABEL)
        .ok_or_else(|| "Quick launch panel window is unavailable".to_string())?;
    panel.hide().map_err(|error| {
        format!("Failed to hide quick launch panel after admin launch: {error}")
    })?;
    quick_launch_panel_state_reset();
    app_handle
        .emit_to(
            BOTTOM_BAR_LABEL,
            QUICK_LAUNCH_PANEL_CLOSED_EVENT,
            serde_json::json!({"nonce": args.nonce}),
        )
        .map_err(|error| format!("Failed to publish quick launch panel closed event: {error}"))?;
    app_handle
        .emit_to(
            QUICK_LAUNCH_PANEL_LABEL,
            QUICK_LAUNCH_PANEL_CLOSED_EVENT,
            serde_json::json!({"nonce": args.nonce}),
        )
        .map_err(|error| format!("Failed to publish quick launch panel closed event: {error}"))?;
    Ok(())
}

#[tauri::command]
pub fn show_quick_launch_panel_context_menu(
    window: WebviewWindow,
    app_handle: AppHandle,
    args: QuickLaunchPanelContextMenuArgs,
) -> Result<(), String> {
    if window.label() != QUICK_LAUNCH_PANEL_LABEL {
        return Err(
            "Unauthorized caller for command show_quick_launch_panel_context_menu".to_string(),
        );
    }
    let shortcut_path =
        validate_quick_launch_panel_shortcut(args.nonce.as_str(), args.shortcut_path.as_str())?;
    begin_quick_launch_panel_focus_hold();
    struct FocusHoldGuard;
    impl Drop for FocusHoldGuard {
        fn drop(&mut self) {
            end_quick_launch_panel_focus_hold();
        }
    }
    let _focus_hold_guard = FocusHoldGuard;
    taskbar_menu::show_quick_launch_panel_context_menu(
        app_handle,
        taskbar_menu::ShowQuickLaunchPanelContextMenuRequest {
            nonce: args.nonce,
            shortcut_path: shortcut_path.to_string_lossy().into_owned(),
            x: args.x,
            y: args.y,
        },
    )
}
