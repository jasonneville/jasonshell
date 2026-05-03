use crate::settings::{self, QuickIconEntry};
use crate::shell_paths;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::AppHandle;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuickIcon {
    pub id: String,
    pub name: String,
    pub target_path: String,
    pub icon_data_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PinTaskWindowQuickIconRequest {
    pub hwnd: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuickIconIdRequest {
    pub id: String,
}

#[tauri::command]
pub fn list_quick_icons(app_handle: AppHandle) -> Result<Vec<QuickIcon>, String> {
    let settings = settings::load_shell_settings_for_app(&app_handle)?;
    settings
        .quick_icons
        .entries
        .iter()
        .map(settings::validate_quick_icon_entry)
        .collect::<Result<Vec<_>, _>>()
        .map(map_quick_icon_entries)
}

#[tauri::command]
pub fn pin_task_window_quick_icon(
    app_handle: AppHandle,
    request: PinTaskWindowQuickIconRequest,
) -> Result<Vec<QuickIcon>, String> {
    let hwnd = request.hwnd.trim();
    if hwnd.is_empty() {
        return Err("task window handle is required".to_string());
    }

    let executable_path = task_window_executable_path(hwnd)?;
    crate::launchers::pin_executable_to_taskbar_shortcut(executable_path)?;
    list_quick_icons(app_handle)
}

#[tauri::command]
pub fn unpin_quick_icon(
    app_handle: AppHandle,
    request: QuickIconIdRequest,
) -> Result<Vec<QuickIcon>, String> {
    let id = request.id.trim();
    if id.is_empty() {
        return Err("quick icon id must not be empty".to_string());
    }

    let mut settings = settings::load_shell_settings_for_app(&app_handle)?;
    settings.quick_icons.entries = remove_quick_icon_entry(settings.quick_icons.entries, id);
    let saved = settings::save_shell_settings_for_app(&app_handle, settings)?;
    Ok(map_quick_icon_entries(saved.quick_icons.entries))
}

#[tauri::command]
pub fn launch_quick_icon(
    app_handle: AppHandle,
    request: QuickIconIdRequest,
) -> Result<(), String> {
    let id = request.id.trim();
    if id.is_empty() {
        return Err("quick icon id must not be empty".to_string());
    }

    let settings = settings::load_shell_settings_for_app(&app_handle)?;
    let Some(entry) = settings.quick_icons.entries.iter().find(|entry| entry.id == id) else {
        return Err(format!("quick icon '{}' is not configured", id));
    };
    let entry = settings::validate_quick_icon_entry(entry)?;
    if !Path::new(&entry.target_path).is_absolute() {
        return Err(format!(
            "quick icon '{}' target path must be absolute",
            entry.id
        ));
    }
    shell_paths::open_shell_path(entry.target_path)
}

fn map_quick_icon_entries(entries: Vec<QuickIconEntry>) -> Vec<QuickIcon> {
    entries
        .into_iter()
        .map(|entry| QuickIcon {
            id: entry.id,
            name: entry.name,
            target_path: entry.target_path,
            icon_data_url: entry.icon_data_url,
        })
        .collect()
}

fn upsert_quick_icon_entry(
    entries: Vec<QuickIconEntry>,
    entry: QuickIconEntry,
) -> Vec<QuickIconEntry> {
    let target_key = quick_icon_target_key(&entry.target_path);
    let mut retained = entries
        .into_iter()
        .filter(|candidate| {
            candidate.id != entry.id && quick_icon_target_key(&candidate.target_path) != target_key
        })
        .collect::<Vec<_>>();
    retained.insert(0, entry);
    retained
}

fn remove_quick_icon_entry(entries: Vec<QuickIconEntry>, id: &str) -> Vec<QuickIconEntry> {
    entries
        .into_iter()
        .filter(|entry| entry.id != id)
        .collect::<Vec<_>>()
}

#[cfg(target_os = "windows")]
fn quick_icon_entry_from_task_window(hwnd: &str) -> Result<QuickIconEntry, String> {
    let target_path = task_window_executable_path(hwnd)?;
    let executable_path = Path::new(&target_path);
    let icon_data_url = crate::task_windows::shell_file_icon_data_url(&executable_path)?;
    let name = executable_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .unwrap_or("Quick Icon")
        .to_string();

    settings::validate_quick_icon_entry(&QuickIconEntry {
        id: quick_icon_id_from_target_path(&target_path),
        name,
        target_path,
        icon_data_url,
    })
}

#[cfg(target_os = "windows")]
fn task_window_executable_path(hwnd: &str) -> Result<String, String> {
    crate::task_windows::task_window_process_path(hwnd)?
        .to_str()
        .ok_or_else(|| "task window executable path is not valid UTF-8".to_string())
        .map(str::to_string)
}

#[cfg(not(target_os = "windows"))]
fn task_window_executable_path(_hwnd: &str) -> Result<String, String> {
    Err("Taskbar pinning is only supported on Windows".to_string())
}

#[cfg(not(target_os = "windows"))]
fn quick_icon_entry_from_task_window(_hwnd: &str) -> Result<QuickIconEntry, String> {
    Err("Quick icons are only supported on Windows".to_string())
}

fn quick_icon_id_from_target_path(path: &str) -> String {
    quick_icon_target_key(path)
}

fn quick_icon_target_key(path: &str) -> String {
    path.trim().replace('/', "\\").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, path: &str) -> QuickIconEntry {
        QuickIconEntry {
            id: id.to_string(),
            name: id.to_string(),
            target_path: path.to_string(),
            icon_data_url: "data:image/png;base64,aaa".to_string(),
        }
    }

    #[test]
    fn upsert_quick_icon_replaces_duplicate_target_and_moves_to_front() {
        let existing = vec![
            entry("code", r"C:\Tools\Code.exe"),
            entry("terminal", r"C:\Windows\System32\WindowsTerminal.exe"),
        ];
        let updated = upsert_quick_icon_entry(existing, entry("code-next", r"c:/tools/code.exe"));

        assert_eq!(updated.len(), 2);
        assert_eq!(updated[0].id, "code-next");
        assert_eq!(updated[1].id, "terminal");
    }

    #[test]
    fn remove_quick_icon_by_id_is_non_destructive_for_non_matches() {
        let existing = vec![
            entry("code", r"C:\Tools\Code.exe"),
            entry("terminal", r"C:\Windows\System32\WindowsTerminal.exe"),
        ];
        let retained = remove_quick_icon_entry(existing.clone(), "missing");
        let removed = remove_quick_icon_entry(existing, "code");

        assert_eq!(retained.len(), 2);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].id, "terminal");
    }
}
