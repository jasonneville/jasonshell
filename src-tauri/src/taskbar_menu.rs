use crate::launchers;
use crate::process_manager;
use crate::shell_windows::{BOTTOM_BAR_LABEL, TOP_BAR_LABEL};
use crate::task_windows::{self, TaskWindowAction};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL;
use base64::Engine;
use serde::Deserialize;
use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager};

const TASKBAR_REFRESH_WINDOWS_EVENT: &str = "taskbar:refresh-windows";
const TASKBAR_REFRESH_LAUNCHERS_EVENT: &str = "taskbar:refresh-launchers";
const TOP_BAR_PIN_MENU_ACTION_EVENT: &str = "top-bar:pin-menu-action";
const TASK_WINDOW_MENU_PREFIX: &str = "task-window";
const LAUNCHER_MENU_PREFIX: &str = "launcher";
const TOP_BAR_PIN_MENU_PREFIX: &str = "top-bar-pin";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowTaskWindowContextMenuRequest {
    pub hwnd: String,
    pub process_id: u32,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowTopBarPinContextMenuRequest {
    pub path: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TopBarPinMenuActionPayload {
    action: String,
    path: String,
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
    .map_err(|error| format!("Failed to build task window close item: {error}"))?;
    let close_item = MenuItem::with_id(
        &app_handle,
        format!("{TASK_WINDOW_MENU_PREFIX}:close:{}", request.hwnd),
        "Close window",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build task window pin item: {error}"))?;
    let process_item = MenuItem::with_id(
        &app_handle,
        format!(
            "{TASK_WINDOW_MENU_PREFIX}:process:{}:{}",
            request.process_id, request.hwnd
        ),
        format!("PID {} - open in Process Manager", request.process_id),
        request.process_id != 0,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build task window process item: {error}"))?;
    let pin_enabled = launchers::can_pin_task_window_to_taskbar(&request.hwnd).unwrap_or(false);
    let pin_item = MenuItem::with_id(
        &app_handle,
        format!("{TASK_WINDOW_MENU_PREFIX}:pin:{}", request.hwnd),
        "Pin to taskbar",
        pin_enabled,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build task window pin item: {error}"))?;
    let menu = Menu::with_items(
        &app_handle,
        &[
            &focus_item,
            &minimize_item,
            &process_item,
            &pin_item,
            &close_item,
        ],
    )
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
    let admin_item = MenuItem::with_id(
        &app_handle,
        format!("{LAUNCHER_MENU_PREFIX}:runas:{encoded_shortcut}"),
        "Run as administrator",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build launcher administrator item: {error}"))?;
    let properties_item = MenuItem::with_id(
        &app_handle,
        format!("{LAUNCHER_MENU_PREFIX}:properties:{encoded_shortcut}"),
        "Properties",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build launcher properties item: {error}"))?;
    let reveal_target_item = MenuItem::with_id(
        &app_handle,
        format!("{LAUNCHER_MENU_PREFIX}:reveal-target:{encoded_shortcut}"),
        "Open target location",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build launcher target reveal item: {error}"))?;
    let copy_path_item = MenuItem::with_id(
        &app_handle,
        format!("{LAUNCHER_MENU_PREFIX}:copy-path:{encoded_shortcut}"),
        "Copy shortcut path",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build launcher copy path item: {error}"))?;
    let unpin_item = MenuItem::with_id(
        &app_handle,
        format!("{LAUNCHER_MENU_PREFIX}:unpin:{encoded_shortcut}"),
        "Unpin from taskbar",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build launcher unpin item: {error}"))?;
    let menu = Menu::with_items(
        &app_handle,
        &[
            &launch_item,
            &admin_item,
            &properties_item,
            &reveal_item,
            &reveal_target_item,
            &copy_path_item,
            &unpin_item,
        ],
    )
    .map_err(|error| format!("Failed to build launcher context menu: {error}"))?;

    bottom_bar
        .popup_menu_at(&menu, LogicalPosition::new(request.x, request.y))
        .map_err(|error| format!("Failed to show launcher context menu: {error}"))
}

#[tauri::command]
pub fn show_top_bar_pin_context_menu(
    app_handle: AppHandle,
    request: ShowTopBarPinContextMenuRequest,
) -> Result<(), String> {
    let top_bar = app_handle
        .get_webview_window(TOP_BAR_LABEL)
        .ok_or_else(|| "Top bar window is unavailable".to_string())?;
    let encoded_path = BASE64_URL.encode(&request.path);
    let open_item = MenuItem::with_id(
        &app_handle,
        format!("{TOP_BAR_PIN_MENU_PREFIX}:open:{encoded_path}"),
        "Open",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build top-bar pin open item: {error}"))?;
    let open_in_vscode_item = MenuItem::with_id(
        &app_handle,
        format!("{TOP_BAR_PIN_MENU_PREFIX}:open-in-vscode:{encoded_path}"),
        "Open in VS Code",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build top-bar pin vscode item: {error}"))?;
    let unpin_item = MenuItem::with_id(
        &app_handle,
        format!("{TOP_BAR_PIN_MENU_PREFIX}:unpin:{encoded_path}"),
        "Unpin",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("Failed to build top-bar pin unpin item: {error}"))?;
    let menu = Menu::with_items(
        &app_handle,
        &[&open_item, &open_in_vscode_item, &unpin_item],
    )
    .map_err(|error| format!("Failed to build top-bar pin context menu: {error}"))?;

    top_bar
        .popup_menu_at(&menu, LogicalPosition::new(request.x, request.y))
        .map_err(|error| format!("Failed to show top-bar pin context menu: {error}"))
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
            "pin" => launchers::pin_task_window_to_taskbar(hwnd.to_string()),
            "process" => {
                let Some((pid, _)) = hwnd.split_once(':') else {
                    eprintln!("task window process menu payload missing pid");
                    return;
                };
                let Ok(focus_pid) = pid.parse::<u32>() else {
                    eprintln!("task window process menu payload invalid pid: {pid}");
                    return;
                };
                process_manager::show_process_manager(
                    app_handle.clone(),
                    process_manager::ShowProcessManagerRequest {
                        anchor_left: 0.0,
                        anchor_width: 0.0,
                        focus_pid: Some(focus_pid),
                    },
                )
            }
            _ => return,
        };

        if let Err(error) = result {
            eprintln!("task window menu action failed: {error}");
        }

        let _ = app_handle.emit_to(BOTTOM_BAR_LABEL, TASKBAR_REFRESH_LAUNCHERS_EVENT, ());
        let _ = app_handle.emit_to(BOTTOM_BAR_LABEL, TASKBAR_REFRESH_WINDOWS_EVENT, ());
        return;
    }

    if let Some((action, encoded_shortcut)) = parse_menu_payload(id, LAUNCHER_MENU_PREFIX) {
        let shortcut_path = match decode_menu_payload(encoded_shortcut) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("launcher menu decode failed: {error}");
                return;
            }
        };

        let result = match action {
            "launch" => launchers::launch_pinned_taskbar_app(shortcut_path.clone()),
            "runas" => launchers::run_pinned_taskbar_app_as_admin(shortcut_path.clone()),
            "properties" => launchers::open_pinned_shortcut_properties(shortcut_path.clone()),
            "reveal" => launchers::reveal_pinned_shortcut(shortcut_path.clone()),
            "reveal-target" => launchers::reveal_pinned_shortcut_target(shortcut_path.clone()),
            "copy-path" => launchers::copy_pinned_shortcut_path(shortcut_path.clone()),
            "unpin" => launchers::unpin_pinned_taskbar_app(shortcut_path.clone()),
            _ => return,
        };

        if let Err(error) = result {
            eprintln!("launcher menu action failed: {error}");
        }

        let _ = app_handle.emit_to(BOTTOM_BAR_LABEL, TASKBAR_REFRESH_LAUNCHERS_EVENT, ());
        let _ = app_handle.emit_to(BOTTOM_BAR_LABEL, TASKBAR_REFRESH_WINDOWS_EVENT, ());
        return;
    }

    if let Some((action, encoded_path)) = parse_menu_payload(id, TOP_BAR_PIN_MENU_PREFIX) {
        let path = match decode_menu_payload(encoded_path) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("top-bar pin menu decode failed: {error}");
                return;
            }
        };

        let payload = TopBarPinMenuActionPayload {
            action: if action == "open-in-vscode" {
                "openInVscode".to_string()
            } else {
                action.to_string()
            },
            path,
        };
        let _ = app_handle.emit_to(TOP_BAR_LABEL, TOP_BAR_PIN_MENU_ACTION_EVENT, payload);
    }
}

fn parse_menu_payload<'a>(id: &'a str, prefix: &str) -> Option<(&'a str, &'a str)> {
    let (menu_prefix, rest) = id.split_once(':')?;
    if menu_prefix != prefix {
        return None;
    }

    rest.split_once(':')
}

fn decode_menu_payload(encoded_shortcut: &str) -> Result<String, String> {
    let decoded = BASE64_URL
        .decode(encoded_shortcut)
        .map_err(|error| format!("Failed to decode launcher payload: {error}"))?;

    String::from_utf8(decoded)
        .map_err(|error| format!("Launcher payload was not valid UTF-8: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{
        decode_menu_payload, parse_menu_payload, LAUNCHER_MENU_PREFIX, TASK_WINDOW_MENU_PREFIX,
        TOP_BAR_PIN_MENU_PREFIX,
    };
    use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL;
    use base64::Engine;

    #[test]
    fn parses_known_menu_prefix_payloads() {
        assert_eq!(
            parse_menu_payload("task-window:focus:1234", TASK_WINDOW_MENU_PREFIX),
            Some(("focus", "1234"))
        );
        assert_eq!(
            parse_menu_payload("launcher:launch:abcd", LAUNCHER_MENU_PREFIX),
            Some(("launch", "abcd"))
        );
        assert_eq!(
            parse_menu_payload("top-bar-pin:open-in-vscode:abcd", TOP_BAR_PIN_MENU_PREFIX),
            Some(("open-in-vscode", "abcd"))
        );
    }

    #[test]
    fn rejects_wrong_prefix_payloads() {
        assert_eq!(
            parse_menu_payload("quick-icon:launch:abcd", LAUNCHER_MENU_PREFIX),
            None
        );
        assert_eq!(
            parse_menu_payload("invalid-payload", TASK_WINDOW_MENU_PREFIX),
            None
        );
    }

    #[test]
    fn decodes_menu_payload_values() {
        let encoded = BASE64_URL.encode(r"C:\Apps\Code.exe");
        assert_eq!(decode_menu_payload(&encoded).unwrap(), r"C:\Apps\Code.exe");
    }
}
