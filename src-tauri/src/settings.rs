use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const SETTINGS_SCHEMA: &str = "jasonshell.settings";
const SETTINGS_VERSION: u32 = 1;
const SETTINGS_FILE: &str = "jasonshell-settings-v1.json";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShellSettings {
    pub schema: String,
    pub version: u32,
    pub ui: ShellUiSettings,
    pub workspaces: Vec<Value>,
    pub task_history: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellUiSettings {
    pub active_workspace_id: Option<String>,
    pub enable_diagnostics_export: bool,
}

impl Default for ShellSettings {
    fn default() -> Self {
        Self {
            schema: SETTINGS_SCHEMA.to_string(),
            version: SETTINGS_VERSION,
            ui: ShellUiSettings {
                active_workspace_id: None,
                enable_diagnostics_export: false,
            },
            workspaces: Vec::new(),
            task_history: Vec::new(),
        }
    }
}

#[tauri::command]
pub fn load_shell_settings(app_handle: AppHandle) -> Result<ShellSettings, String> {
    let path = settings_path(&app_handle)?;
    load_settings_from_path(&path)
}

#[tauri::command]
pub fn save_shell_settings(
    app_handle: AppHandle,
    settings: ShellSettings,
) -> Result<ShellSettings, String> {
    let path = settings_path(&app_handle)?;
    save_settings_to_path(&path, settings)
}

fn settings_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    app_handle
        .path()
        .app_local_data_dir()
        .map(|dir| dir.join(SETTINGS_FILE))
        .map_err(|error| format!("failed to resolve settings directory: {error}"))
}

fn load_settings_from_path(path: &Path) -> Result<ShellSettings, String> {
    if !path.exists() {
        return Ok(ShellSettings::default());
    }

    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read shell settings: {error}"))?;
    let value = match serde_json::from_str::<Value>(&raw) {
        Ok(value) => value,
        Err(error) => {
            backup_corrupt_settings(path).map_err(|backup_error| {
                format!("failed to back up corrupt settings: {backup_error}")
            })?;
            let _ = error;
            return Ok(ShellSettings::default());
        }
    };
    let settings = migrate_settings_value(value)?;
    reject_secret_setting_keys(
        &serde_json::to_value(&settings)
            .map_err(|error| format!("failed to inspect shell settings: {error}"))?,
        &[],
    )?;
    Ok(settings)
}

fn save_settings_to_path(
    path: &Path,
    mut settings: ShellSettings,
) -> Result<ShellSettings, String> {
    settings.schema = SETTINGS_SCHEMA.to_string();
    settings.version = SETTINGS_VERSION;
    reject_secret_setting_keys(
        &serde_json::to_value(&settings)
            .map_err(|error| format!("failed to inspect shell settings: {error}"))?,
        &[],
    )?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create settings directory: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(&settings)
        .map_err(|error| format!("failed to serialize shell settings: {error}"))?;
    write_file_atomic(path, &bytes)
        .map_err(|error| format!("failed to write shell settings: {error}"))?;
    Ok(settings)
}

fn migrate_settings_value(value: Value) -> Result<ShellSettings, String> {
    let version = value.get("version").and_then(Value::as_u64);
    match version {
        Some(1) => serde_json::from_value(value)
            .map_err(|error| format!("failed to parse shell settings v1: {error}")),
        None | Some(0) => migrate_legacy_settings(value),
        Some(other) => Err(format!("unsupported shell settings version: {other}")),
    }
}

fn migrate_legacy_settings(value: Value) -> Result<ShellSettings, String> {
    let mut settings = ShellSettings::default();
    if let Some(active_workspace_id) = value
        .get("ui")
        .and_then(|ui| ui.get("activeWorkspaceId"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        settings.ui.active_workspace_id = Some(active_workspace_id.to_string());
    }
    Ok(settings)
}

fn reject_secret_setting_keys(value: &Value, path: &[String]) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let mut next_path = path.to_vec();
                next_path.push(key.clone());
                if is_secret_key(key) {
                    return Err(format!(
                        "shell settings must not store secret-like key: {}",
                        next_path.join(".")
                    ));
                }
                reject_secret_setting_keys(child, &next_path)?;
            }
            Ok(())
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let mut next_path = path.to_vec();
                next_path.push(index.to_string());
                reject_secret_setting_keys(child, &next_path)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn is_secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "credential",
        "api_key",
        "apikey",
        "authorization",
        "cookie",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn backup_corrupt_settings(path: &Path) -> io::Result<PathBuf> {
    let backup = path.with_extension(format!("corrupt-{}.bak", current_epoch_secs()));
    fs::rename(path, &backup)?;
    Ok(backup)
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, bytes)?;
    fs::rename(&temp_path, path)
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::env;

    fn test_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!(
            "jasonshell-settings-{name}-{}",
            current_epoch_secs()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn loads_default_settings_when_file_is_missing() {
        let path = test_dir("missing").join(SETTINGS_FILE);

        assert_eq!(
            load_settings_from_path(&path).unwrap(),
            ShellSettings::default()
        );
    }

    #[test]
    fn saves_and_loads_versioned_settings() {
        let path = test_dir("roundtrip").join(SETTINGS_FILE);
        let mut settings = ShellSettings::default();
        settings.ui.enable_diagnostics_export = true;

        save_settings_to_path(&path, settings.clone()).unwrap();

        assert_eq!(load_settings_from_path(&path).unwrap(), settings);
    }

    #[test]
    fn migrates_unversioned_settings_to_v1_defaults() {
        let path = test_dir("legacy").join(SETTINGS_FILE);
        fs::write(
            &path,
            json!({ "ui": { "activeWorkspaceId": "workspace-a" } }).to_string(),
        )
        .unwrap();

        let settings = load_settings_from_path(&path).unwrap();

        assert_eq!(settings.version, SETTINGS_VERSION);
        assert_eq!(
            settings.ui.active_workspace_id.as_deref(),
            Some("workspace-a")
        );
        assert!(settings.workspaces.is_empty());
        assert!(settings.task_history.is_empty());
    }

    #[test]
    fn backs_up_corrupt_settings_and_recovers_defaults() {
        let dir = test_dir("corrupt");
        let path = dir.join(SETTINGS_FILE);
        fs::write(&path, b"not-json").unwrap();

        let settings = load_settings_from_path(&path).unwrap();

        assert_eq!(settings, ShellSettings::default());
        assert!(!path.exists());
        assert!(fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().contains("corrupt")));
    }

    #[test]
    fn rejects_secret_like_settings_keys() {
        let value = json!({ "workspaces": [{ "apiToken": "abc" }] });

        let error = reject_secret_setting_keys(&value, &[]).unwrap_err();

        assert!(error.contains("workspaces.0.apiToken"));
    }
}
