use crate::launchers;
use crate::shell_windows::BOTTOM_BAR_LABEL;
use crate::task_windows::{self, TaskWindowAction};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL;
use base64::Engine;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager};

const TASKBAR_REFRESH_WINDOWS_EVENT: &str = "taskbar:refresh-windows";
const TASKBAR_REFRESH_LAUNCHERS_EVENT: &str = "taskbar:refresh-launchers";
const TASK_WINDOW_MENU_PREFIX: &str = "task-window";
const LAUNCHER_MENU_PREFIX: &str = "launcher";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowTaskWindowContextMenuRequest {
    pub hwnd: String,
    pub is_minimized: bool,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowLauncherContextMenuRequest {
    pub shortcut_path: String,
    pub x: f64,
    pub y: f64,
}

#[tauri::command]
pub fn show_task_window_context_menu(
    app_handle: AppHandle,
    request: ShowTaskWindowContextMenuRequest,
) -> Result<(), String> {
    let bottom_bar = app_handle
        .get_webview_window(BOTTOM_BAR_LABEL)
        .ok_or_else(|| "Bottom bar window is unavailable".to_string())?;
    let focus_label = if request.is_minimized {
        "Restore window"
    } else {
        "Switch to window"
    };
    let focus_item = MenuItem::with_id(
        &app_handle,
        format!("{TASK_WINDOW_MENU_PREFIX}:focus:{}", request.hwnd),
        focus_label,
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build task window focus item: {error}"))?;
    let minimize_item = MenuItem::with_id(
        &app_handle,
        format!("{TASK_WINDOW_MENU_PREFIX}:minimize:{}", request.hwnd),
        "Minimize",
        !request.is_minimized,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build task window minimize item: {error}"))?;
    let close_item = MenuItem::with_id(
        &app_handle,
        format!("{TASK_WINDOW_MENU_PREFIX}:close:{}", request.hwnd),
        "Close window",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build task window close item: {error}"))?;
    let menu = Menu::with_items(&app_handle, &[&focus_item, &minimize_item, &close_item])
        .map_err(|error| format!("Failed to build task window context menu: {error}"))?;

    bottom_bar
        .popup_menu_at(&menu, LogicalPosition::new(request.x, request.y))
        .map_err(|error| format!("Failed to show task window context menu: {error}"))
}

#[tauri::command]
pub fn show_launcher_context_menu(
    app_handle: AppHandle,
    request: ShowLauncherContextMenuRequest,
) -> Result<(), String> {
    let bottom_bar = app_handle
        .get_webview_window(BOTTOM_BAR_LABEL)
        .ok_or_else(|| "Bottom bar window is unavailable".to_string())?;
    let encoded_shortcut = BASE64_URL.encode(request.shortcut_path);
    let launch_item = MenuItem::with_id(
        &app_handle,
        format!("{LAUNCHER_MENU_PREFIX}:launch:{encoded_shortcut}"),
        "Launch",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build launcher launch item: {error}"))?;
    let reveal_item = MenuItem::with_id(
        &app_handle,
        format!("{LAUNCHER_MENU_PREFIX}:reveal:{encoded_shortcut}"),
        "Open shortcut location",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build launcher reveal item: {error}"))?;
    let menu = Menu::with_items(&app_handle, &[&launch_item, &reveal_item])
        .map_err(|error| format!("Failed to build launcher context menu: {error}"))?;

    bottom_bar
        .popup_menu_at(&menu, LogicalPosition::new(request.x, request.y))
        .map_err(|error| format!("Failed to show launcher context menu: {error}"))
}

pub fn handle_taskbar_menu_event(app_handle: &AppHandle, event: MenuEvent) {
    let id = &event.id().0;

    if let Some((action, hwnd)) = parse_menu_payload(id, TASK_WINDOW_MENU_PREFIX) {
        let result = match action {
            "focus" => {
                task_windows::perform_task_window_action(hwnd.to_string(), TaskWindowAction::Focus)
            }
            "minimize" => task_windows::perform_task_window_action(
                hwnd.to_string(),
                TaskWindowAction::Minimize,
            ),
            "close" => {
                task_windows::perform_task_window_action(hwnd.to_string(), TaskWindowAction::Close)
            }
            _ => return,
        };

        if let Err(error) = result {
            eprintln!("task window menu action failed: {error}");
        }

        let _ = app_handle.emit_to(BOTTOM_BAR_LABEL, TASKBAR_REFRESH_WINDOWS_EVENT, ());
        return;
    }

    if let Some((action, encoded_shortcut)) = parse_menu_payload(id, LAUNCHER_MENU_PREFIX) {
        let shortcut_path = match decode_shortcut_path(encoded_shortcut) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("launcher menu decode failed: {error}");
                return;
            }
        };

        let result = match action {
            "launch" => launchers::launch_pinned_taskbar_app(shortcut_path.clone()),
            "reveal" => reveal_pinned_shortcut(&shortcut_path),
            _ => return,
        };

        if let Err(error) = result {
            eprintln!("launcher menu action failed: {error}");
        }

        let _ = app_handle.emit_to(BOTTOM_BAR_LABEL, TASKBAR_REFRESH_LAUNCHERS_EVENT, ());
        let _ = app_handle.emit_to(BOTTOM_BAR_LABEL, TASKBAR_REFRESH_WINDOWS_EVENT, ());
    }
}

fn parse_menu_payload<'a>(id: &'a str, prefix: &str) -> Option<(&'a str, &'a str)> {
    let (menu_prefix, rest) = id.split_once(':')?;
    if menu_prefix != prefix {
        return None;
    }

    rest.split_once(':')
}

fn decode_shortcut_path(encoded_shortcut: &str) -> Result<String, String> {
    let decoded = BASE64_URL
        .decode(encoded_shortcut)
        .map_err(|error| format!("Failed to decode launcher payload: {error}"))?;

    String::from_utf8(decoded)
        .map_err(|error| format!("Launcher payload was not valid UTF-8: {error}"))
}

fn reveal_pinned_shortcut(shortcut_path: &str) -> Result<(), String> {
    let shortcut_path = validate_shortcut_path(shortcut_path)?;

    std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", shortcut_path.display()))
        .spawn()
        .map_err(|error| format!("Failed to reveal pinned shortcut: {error}"))?;

    Ok(())
}

fn validate_shortcut_path(shortcut_path: &str) -> Result<PathBuf, String> {
    let requested_path = PathBuf::from(shortcut_path);
    if !has_lnk_extension(&requested_path) {
        return Err("Only pinned .lnk shortcuts may be revealed".to_string());
    }

    let canonical_dir = fs::canonicalize(pinned_taskbar_dir()?)
        .map_err(|error| format!("Failed to resolve pinned taskbar directory: {error}"))?;
    let canonical_shortcut = fs::canonicalize(&requested_path)
        .map_err(|error| format!("Failed to resolve pinned shortcut path: {error}"))?;
    let Some(parent) = canonical_shortcut.parent() else {
        return Err("Pinned shortcut parent directory is unavailable".to_string());
    };

    if parent != canonical_dir {
        return Err("Pinned shortcut path is outside the taskbar pin directory".to_string());
    }

    Ok(canonical_shortcut)
}

fn pinned_taskbar_dir() -> Result<PathBuf, String> {
    let Some(appdata) = std::env::var_os("APPDATA") else {
        return Err("APPDATA is unavailable".to_string());
    };

    Ok(PathBuf::from(appdata).join(
        Path::new("Microsoft")
            .join("Internet Explorer")
            .join("Quick Launch")
            .join("User Pinned")
            .join("TaskBar"),
    ))
}

fn has_lnk_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("lnk"))
        .unwrap_or(false)
}
