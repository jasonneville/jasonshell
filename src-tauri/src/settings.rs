use crate::workspaces::{normalize_workspace, WorkspaceProfile};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const SETTINGS_SCHEMA: &str = "jasonshell.settings";
const SETTINGS_VERSION: u32 = 1;
const SETTINGS_FILE: &str = "jasonshell-settings-v1.json";
const DEFAULT_TERMINAL_THEME: &str = "base-dark";
const VALID_TERMINAL_THEMES: &[&str] = &[
    "base-dark",
    "base-light",
    "monokai",
    "atom-one-dark",
    "atom-one-light",
    "nord",
    "dracula",
    "solarized-dark",
    "solarized-light",
    "github-dark",
    "github-light",
    "gruvbox-dark",
    "gruvbox-light",
    "tokyo-night",
    "catppuccin-mocha",
    "ayu-dark",
];
static SETTINGS_WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShellSettings {
    pub schema: String,
    pub version: u32,
    #[serde(default)]
    pub ui: ShellUiSettings,
    #[serde(default)]
    pub search: SearchSettings,
    #[serde(default)]
    pub stack_browser: StackBrowserSettings,
    #[serde(default)]
    pub workspaces: Vec<WorkspaceProfile>,
    #[serde(default)]
    pub task_history: Vec<Value>,
    #[serde(default)]
    pub quick_commands: QuickCommandsSettings,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShellUiSettings {
    pub active_workspace_id: Option<String>,
    pub enable_diagnostics_export: bool,
    #[serde(default)]
    pub search_mode: SearchMode,
    #[serde(default = "default_true")]
    pub lock_top_bar_height: bool,
    #[serde(default = "default_true")]
    pub lock_bottom_bar_height: bool,
    #[serde(default = "default_top_bar_height_logical")]
    pub top_bar_height_logical: f64,
    #[serde(default = "default_bottom_bar_height_logical")]
    pub bottom_bar_height_logical: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchSettings {
    #[serde(default = "default_search_result_limit")]
    pub result_limit: usize,
    #[serde(default)]
    pub everything: EverythingSearchSettings,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StackBrowserSettings {
    #[serde(default)]
    pub terminal_profile: TerminalProfile,
    #[serde(default = "default_terminal_theme")]
    pub terminal_theme: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TerminalProfile {
    #[default]
    WindowsTerminal,
    GitBash,
    #[serde(rename = "powershell")]
    PowerShell,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EverythingSearchSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub install_mode: EverythingInstallMode,
    #[serde(default)]
    pub sdk_source: EverythingSdkSource,
    #[serde(default = "default_everything_max_results")]
    pub max_results: usize,
    #[serde(default = "default_true")]
    pub full_path_search: bool,
    #[serde(default)]
    pub sort: EverythingSortMode,
    #[serde(default)]
    pub content_search_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuickCommandsSettings {
    #[serde(default)]
    pub entries: Vec<QuickCommandEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuickCommandEntry {
    pub id: String,
    pub label: String,
    pub mode: QuickCommandMode,
    pub target_path: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    pub cwd: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuickCommandMode {
    #[default]
    Direct,
    CommandBlock,
    PowershellFile,
    CmdFile,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SearchMode {
    TopRight,
    #[default]
    CenteredHotkey,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EverythingInstallMode {
    #[default]
    Ask,
    Disabled,
    Managed,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EverythingSdkSource {
    Bundled,
    #[default]
    System,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EverythingSortMode {
    #[default]
    NameAsc,
    PathAsc,
    DateModifiedDesc,
    RunCountDesc,
}

impl Default for ShellSettings {
    fn default() -> Self {
        Self {
            schema: SETTINGS_SCHEMA.to_string(),
            version: SETTINGS_VERSION,
            ui: ShellUiSettings::default(),
            search: SearchSettings::default(),
            stack_browser: StackBrowserSettings::default(),
            workspaces: Vec::new(),
            task_history: Vec::new(),
            quick_commands: QuickCommandsSettings::default(),
        }
    }
}

impl Default for ShellUiSettings {
    fn default() -> Self {
        Self {
            active_workspace_id: None,
            enable_diagnostics_export: false,
            search_mode: SearchMode::CenteredHotkey,
            lock_top_bar_height: true,
            lock_bottom_bar_height: true,
            top_bar_height_logical: default_top_bar_height_logical(),
            bottom_bar_height_logical: default_bottom_bar_height_logical(),
        }
    }
}

impl Default for StackBrowserSettings {
    fn default() -> Self {
        Self {
            terminal_profile: TerminalProfile::WindowsTerminal,
            terminal_theme: default_terminal_theme(),
        }
    }
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            result_limit: default_search_result_limit(),
            everything: EverythingSearchSettings::default(),
        }
    }
}

impl Default for EverythingSearchSettings {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            install_mode: EverythingInstallMode::Ask,
            sdk_source: EverythingSdkSource::System,
            max_results: default_everything_max_results(),
            full_path_search: default_true(),
            sort: EverythingSortMode::NameAsc,
            content_search_enabled: false,
        }
    }
}

impl Default for QuickCommandsSettings {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

fn default_terminal_theme() -> String {
    DEFAULT_TERMINAL_THEME.to_string()
}

fn normalize_terminal_theme(value: &str) -> String {
    if VALID_TERMINAL_THEMES.contains(&value) {
        value.to_string()
    } else {
        default_terminal_theme()
    }
}

fn default_search_result_limit() -> usize {
    50
}

fn default_everything_max_results() -> usize {
    100
}

fn default_true() -> bool {
    true
}

fn default_top_bar_height_logical() -> f64 {
    crate::shell_windows::TOP_BAR_HEIGHT_LOGICAL
}

fn default_bottom_bar_height_logical() -> f64 {
    crate::shell_windows::BOTTOM_BAR_HEIGHT_LOGICAL
}

#[tauri::command]
pub fn load_shell_settings(app_handle: AppHandle) -> Result<ShellSettings, String> {
    load_shell_settings_for_app(&app_handle)
}

#[tauri::command]
pub fn save_shell_settings(
    app_handle: AppHandle,
    settings: ShellSettings,
) -> Result<ShellSettings, String> {
    save_shell_settings_for_app(&app_handle, settings)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveShellBarHeightRequest {
    pub edge: crate::appbar::ShellBarResizeEdge,
    pub height_logical: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveShellBarLockRequest {
    pub edge: crate::appbar::ShellBarResizeEdge,
    pub locked: bool,
}

#[tauri::command]
pub fn save_shell_bar_height(
    app_handle: AppHandle,
    request: SaveShellBarHeightRequest,
) -> Result<ShellSettings, String> {
    save_shell_bar_height_for_app(&app_handle, request)
}

#[tauri::command]
pub fn save_shell_bar_lock(
    app_handle: AppHandle,
    request: SaveShellBarLockRequest,
) -> Result<ShellSettings, String> {
    save_shell_bar_lock_for_app(&app_handle, request)
}

pub(crate) fn load_shell_settings_for_app(app_handle: &AppHandle) -> Result<ShellSettings, String> {
    let path = settings_path(app_handle)?;
    load_settings_from_path(&path)
}

pub(crate) fn save_shell_settings_for_app(
    app_handle: &AppHandle,
    settings: ShellSettings,
) -> Result<ShellSettings, String> {
    let path = settings_path(app_handle)?;
    let _guard = SETTINGS_WRITE_LOCK
        .lock()
        .map_err(|_| "settings write lock is poisoned".to_string())?;
    save_settings_to_path(&path, settings)
}

pub(crate) fn save_shell_bar_height_for_app(
    app_handle: &AppHandle,
    request: SaveShellBarHeightRequest,
) -> Result<ShellSettings, String> {
    let path = settings_path(app_handle)?;
    save_shell_bar_height_to_path(&path, request)
}

pub(crate) fn save_shell_bar_lock_for_app(
    app_handle: &AppHandle,
    request: SaveShellBarLockRequest,
) -> Result<ShellSettings, String> {
    let path = settings_path(app_handle)?;
    save_shell_bar_lock_to_path(&path, request)
}

fn save_shell_bar_height_to_path(
    path: &Path,
    request: SaveShellBarHeightRequest,
) -> Result<ShellSettings, String> {
    let _guard = SETTINGS_WRITE_LOCK
        .lock()
        .map_err(|_| "settings write lock is poisoned".to_string())?;
    let mut settings = load_settings_from_path(&path)?;
    match request.edge {
        crate::appbar::ShellBarResizeEdge::Top => {
            settings.ui.top_bar_height_logical = request.height_logical;
        }
        crate::appbar::ShellBarResizeEdge::Bottom => {
            settings.ui.bottom_bar_height_logical = request.height_logical;
        }
    }
    save_settings_to_path(&path, settings)
}

fn save_shell_bar_lock_to_path(
    path: &Path,
    request: SaveShellBarLockRequest,
) -> Result<ShellSettings, String> {
    let _guard = SETTINGS_WRITE_LOCK
        .lock()
        .map_err(|_| "settings write lock is poisoned".to_string())?;
    let mut settings = load_settings_from_path(&path)?;
    match request.edge {
        crate::appbar::ShellBarResizeEdge::Top => {
            settings.ui.lock_top_bar_height = request.locked;
        }
        crate::appbar::ShellBarResizeEdge::Bottom => {
            settings.ui.lock_bottom_bar_height = request.locked;
        }
    }
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
    validate_settings(settings)
}

fn save_settings_to_path(
    path: &Path,
    mut settings: ShellSettings,
) -> Result<ShellSettings, String> {
    settings.schema = SETTINGS_SCHEMA.to_string();
    settings.version = SETTINGS_VERSION;
    settings = validate_settings(settings)?;
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

fn validate_settings(mut settings: ShellSettings) -> Result<ShellSettings, String> {
    settings.search.result_limit = settings.search.result_limit.clamp(1, 100);
    settings.search.everything.max_results = settings.search.everything.max_results.clamp(1, 200);
    settings.ui.top_bar_height_logical = clamp_shell_bar_height_logical(
        settings.ui.top_bar_height_logical,
        crate::shell_windows::MIN_TOP_BAR_HEIGHT_LOGICAL,
    );
    settings.ui.bottom_bar_height_logical = clamp_shell_bar_height_logical(
        settings.ui.bottom_bar_height_logical,
        crate::shell_windows::MIN_BOTTOM_BAR_HEIGHT_LOGICAL,
    );
    if settings.search.everything.content_search_enabled {
        settings.search.everything.content_search_enabled = false;
    }
    settings.stack_browser.terminal_theme = normalize_terminal_theme(&settings.stack_browser.terminal_theme);

    let mut workspaces = Vec::with_capacity(settings.workspaces.len());
    for workspace in settings.workspaces {
        workspaces.push(normalize_workspace(workspace)?);
    }
    settings.workspaces = workspaces;
    settings.quick_commands = validate_quick_commands_settings(settings.quick_commands)?;
    Ok(settings)
}

pub(crate) fn clamp_shell_bar_height_logical(value: f64, minimum: f64) -> f64 {
    if !value.is_finite() {
        return minimum;
    }
    value.clamp(minimum, 120.0)
}

fn validate_quick_commands_settings(
    mut quick_commands: QuickCommandsSettings,
) -> Result<QuickCommandsSettings, String> {
    let mut seen_ids = HashSet::new();
    let mut normalized = Vec::with_capacity(quick_commands.entries.len());
    for entry in quick_commands.entries {
        let entry = validate_quick_command_entry(&entry)?;
        if !seen_ids.insert(entry.id.clone()) {
            return Err(format!("quick command id must be unique: {}", entry.id));
        }
        normalized.push(entry);
    }
    quick_commands.entries = normalized;
    Ok(quick_commands)
}

pub(crate) fn validate_quick_command_entry(
    entry: &QuickCommandEntry,
) -> Result<QuickCommandEntry, String> {
    let id = entry.id.trim();
    if !is_slug_safe_id(id) {
        return Err(format!(
            "quick command id must be slug-safe lowercase text: {}",
            entry.id
        ));
    }

    let label = entry.label.trim();
    if label.is_empty() {
        return Err("quick command label must not be empty".to_string());
    }

    let target_path = entry.target_path.trim();
    let target_is_absolute = Path::new(target_path).is_absolute();
    let (mode, commands) = match entry.mode {
        QuickCommandMode::Direct => {
            if target_path.is_empty() {
                return Err(format!(
                    "quick command '{}' target path must not be empty",
                    id
                ));
            }
            if !target_is_absolute && !is_safe_command_token(target_path) {
                return Err(format!(
                    "quick command '{}' direct mode target must be an absolute path or safe command token",
                    id
                ));
            }
            (QuickCommandMode::Direct, Vec::new())
        }
        QuickCommandMode::CommandBlock => {
            let commands = validate_quick_command_commands(&entry.commands, id)?;
            (QuickCommandMode::CommandBlock, commands)
        }
        QuickCommandMode::PowershellFile => (
            QuickCommandMode::CommandBlock,
            vec![legacy_powershell_file_command(target_path, &entry.args)],
        ),
        QuickCommandMode::CmdFile => (
            QuickCommandMode::CommandBlock,
            vec![legacy_cmd_file_command(target_path, &entry.args)],
        ),
    };

    let args = validate_quick_command_args(&entry.args, id)?;
    let cwd = normalize_optional_absolute_dir(entry.cwd.as_deref(), id)?;
    Ok(QuickCommandEntry {
        id: id.to_string(),
        label: label.to_string(),
        mode,
        target_path: if mode == QuickCommandMode::Direct {
            target_path.to_string()
        } else {
            String::new()
        },
        args: if mode == QuickCommandMode::Direct {
            args
        } else {
            Vec::new()
        },
        commands,
        cwd,
    })
}

pub(crate) fn validate_quick_command_args(
    args: &[String],
    command_id: &str,
) -> Result<Vec<String>, String> {
    let mut normalized = Vec::with_capacity(args.len());
    for arg in args {
        let arg = arg.trim();
        if arg.is_empty() {
            return Err(format!(
                "quick command '{}' argument must not be empty",
                command_id
            ));
        }
        reject_control_chars(arg, command_id)?;
        if contains_secret_like_arg(arg) {
            return Err(format!(
                "quick command '{}' contains secret-like argument content",
                command_id
            ));
        }
        normalized.push(arg.to_string());
    }
    Ok(normalized)
}

pub(crate) fn validate_quick_command_commands(
    commands: &[String],
    command_id: &str,
) -> Result<Vec<String>, String> {
    let mut normalized = Vec::with_capacity(commands.len());
    for command in commands {
        let command = command.trim();
        if command.is_empty() {
            return Err(format!(
                "quick command '{}' command block must not include empty commands",
                command_id
            ));
        }
        reject_control_chars(command, command_id)?;
        if contains_secret_like_arg(command) {
            return Err(format!(
                "quick command '{}' contains secret-like command content",
                command_id
            ));
        }
        normalized.push(command.to_string());
    }
    if normalized.is_empty() {
        return Err(format!(
            "quick command '{}' command block must include at least one command",
            command_id
        ));
    }
    Ok(normalized)
}

fn legacy_powershell_file_command(target_path: &str, args: &[String]) -> String {
    format!(
        "pwsh.exe -NoLogo -NoProfile -File {}{}",
        quote_command_part(target_path),
        format_inline_args(args)
    )
}

fn legacy_cmd_file_command(target_path: &str, args: &[String]) -> String {
    format!(
        "cmd.exe /C {}{}",
        quote_command_part(target_path),
        format_inline_args(args)
    )
}

fn format_inline_args(args: &[String]) -> String {
    if args.is_empty() {
        String::new()
    } else {
        format!(
            " {}",
            args.iter()
                .map(|arg| quote_command_part(arg))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

fn quote_command_part(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':' | '/' | '\\'))
    {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('"', "\\\""))
    }
}

fn normalize_optional_absolute_dir(
    value: Option<&str>,
    command_id: &str,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if !Path::new(value).is_absolute() {
        return Err(format!(
            "quick command '{}' cwd must be an absolute path when present",
            command_id
        ));
    }
    Ok(Some(value.to_string()))
}

fn is_slug_safe_id(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let bytes = value.as_bytes();
    if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit() {
        return false;
    }
    if matches!(bytes.last(), Some(b'-' | b'_')) {
        return false;
    }
    let mut previous_was_separator = false;
    for ch in bytes {
        let is_separator = matches!(*ch, b'-' | b'_');
        let is_allowed = ch.is_ascii_lowercase() || ch.is_ascii_digit() || is_separator;
        if !is_allowed {
            return false;
        }
        if is_separator && previous_was_separator {
            return false;
        }
        previous_was_separator = is_separator;
    }
    true
}

pub(crate) fn is_safe_command_token(value: &str) -> bool {
    if value.is_empty() || value.contains(['\\', '/', ':']) {
        return false;
    }
    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

fn reject_control_chars(value: &str, command_id: &str) -> Result<(), String> {
    if value.chars().any(|ch| ch.is_control()) {
        return Err(format!(
            "quick command '{}' contains control characters in argument text",
            command_id
        ));
    }
    Ok(())
}

fn contains_secret_like_arg(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    if lowered.contains("bearer ") {
        return true;
    }
    if lowered.starts_with("sk-")
        || lowered.starts_with("ghp_")
        || lowered.starts_with("gho_")
        || lowered.starts_with("github_pat_")
        || lowered.starts_with("xoxb-")
        || lowered.starts_with("akia")
    {
        return true;
    }
    if let Some((left, _)) = lowered.split_once('=') {
        let key = left.trim().trim_start_matches('-');
        if is_secret_key(key) {
            return true;
        }
    }
    if lowered.starts_with("--") {
        let flag = lowered
            .trim_start_matches('-')
            .split_whitespace()
            .next()
            .unwrap_or_default();
        if is_secret_key(flag) {
            return true;
        }
    }
    false
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
        assert!(settings.quick_commands.entries.is_empty());
        assert_eq!(settings.ui.search_mode, SearchMode::CenteredHotkey);
        assert_eq!(settings.search, SearchSettings::default());
        assert_eq!(settings.stack_browser, StackBrowserSettings::default());
    }

    #[test]
    fn default_settings_include_search_settings_without_bumping_v1() {
        let value = serde_json::to_value(ShellSettings::default()).unwrap();

        assert_eq!(value["version"], SETTINGS_VERSION);
        assert_eq!(value["ui"]["searchMode"], "centeredHotkey");
        assert_eq!(value["ui"]["lockTopBarHeight"], true);
        assert_eq!(value["ui"]["lockBottomBarHeight"], true);
        assert_eq!(value["ui"]["topBarHeightLogical"], 23.4);
        assert_eq!(value["ui"]["bottomBarHeightLogical"], 32.4);
        assert_eq!(value["search"]["resultLimit"], 50);
        assert_eq!(value["search"]["everything"]["enabled"], true);
        assert_eq!(value["search"]["everything"]["installMode"], "ask");
        assert_eq!(value["search"]["everything"]["sdkSource"], "system");
        assert_eq!(value["search"]["everything"]["contentSearchEnabled"], false);
        assert_eq!(value["stackBrowser"]["terminalProfile"], "windowsTerminal");
        assert!(value.get("terminal").is_none());
        assert_eq!(value["quickCommands"]["entries"], json!([]));
        assert!(value.get("quickIcons").is_none());
    }

    #[test]
    fn clamps_search_result_limits_and_forces_content_search_off() {
        let mut settings = ShellSettings::default();
        settings.search.result_limit = 10_000;
        settings.search.everything.max_results = 10_000;
        settings.search.everything.content_search_enabled = true;

        let settings = validate_settings(settings).unwrap();

        assert_eq!(settings.search.result_limit, 100);
        assert_eq!(settings.search.everything.max_results, 200);
        assert!(!settings.search.everything.content_search_enabled);
    }

    #[test]
    fn stack_browser_terminal_theme_defaults_and_normalizes_unknown_ids() {
        let value = serde_json::to_value(ShellSettings::default()).unwrap();
        assert_eq!(
            value
                .get("stackBrowser")
                .and_then(|stack_browser| stack_browser.get("terminalTheme"))
                .and_then(Value::as_str),
            Some(DEFAULT_TERMINAL_THEME)
        );

        let missing_theme: ShellSettings = serde_json::from_value(json!({
            "schema": SETTINGS_SCHEMA,
            "version": SETTINGS_VERSION,
            "stackBrowser": { "terminalProfile": "gitBash" }
        }))
        .unwrap();
        assert_eq!(missing_theme.stack_browser.terminal_profile, TerminalProfile::GitBash);
        assert_eq!(missing_theme.stack_browser.terminal_theme, DEFAULT_TERMINAL_THEME);

        let mut invalid = ShellSettings::default();
        invalid.stack_browser.terminal_profile = TerminalProfile::PowerShell;
        invalid.stack_browser.terminal_theme = "unknown-theme".to_string();
        let validated = validate_settings(invalid).unwrap();
        assert_eq!(validated.stack_browser.terminal_profile, TerminalProfile::PowerShell);
        assert_eq!(validated.stack_browser.terminal_theme, DEFAULT_TERMINAL_THEME);
    }

    #[test]
    fn clamps_shell_bar_heights_while_locks_default_on() {
        let mut settings = ShellSettings::default();
        settings.ui.lock_top_bar_height = false;
        settings.ui.lock_bottom_bar_height = false;
        settings.ui.top_bar_height_logical = 500.0;
        settings.ui.bottom_bar_height_logical = f64::NAN;

        let settings = validate_settings(settings).unwrap();

        assert_eq!(settings.ui.lock_top_bar_height, false);
        assert_eq!(settings.ui.lock_bottom_bar_height, false);
        assert_eq!(settings.ui.top_bar_height_logical, 120.0);
        assert_eq!(settings.ui.bottom_bar_height_logical, 24.0);
    }

    #[test]
    fn shell_bar_height_save_preserves_unrelated_settings() {
        let path = test_dir("height-merge").join(SETTINGS_FILE);
        let mut settings = ShellSettings::default();
        settings.ui.lock_top_bar_height = false;
        settings.ui.lock_bottom_bar_height = false;
        settings.ui.search_mode = SearchMode::TopRight;
        settings.ui.top_bar_height_logical = 44.0;
        settings.ui.bottom_bar_height_logical = 55.0;
        settings.search.result_limit = 77;
        save_settings_to_path(&path, settings).unwrap();

        let saved = save_shell_bar_height_to_path(
            &path,
            SaveShellBarHeightRequest {
                edge: crate::appbar::ShellBarResizeEdge::Bottom,
                height_logical: 66.0,
            },
        )
        .unwrap();

        assert_eq!(saved.ui.top_bar_height_logical, 44.0);
        assert_eq!(saved.ui.bottom_bar_height_logical, 66.0);
        assert_eq!(saved.ui.search_mode, SearchMode::TopRight);
        assert_eq!(saved.search.result_limit, 77);
        assert_eq!(load_settings_from_path(&path).unwrap(), saved);
    }

    #[test]
    fn shell_bar_lock_save_preserves_current_heights() {
        let path = test_dir("lock-merge").join(SETTINGS_FILE);
        let mut settings = ShellSettings::default();
        settings.ui.lock_top_bar_height = false;
        settings.ui.lock_bottom_bar_height = false;
        settings.ui.top_bar_height_logical = 44.0;
        settings.ui.bottom_bar_height_logical = 55.0;
        save_settings_to_path(&path, settings).unwrap();

        let saved = save_shell_bar_lock_to_path(
            &path,
            SaveShellBarLockRequest {
                edge: crate::appbar::ShellBarResizeEdge::Top,
                locked: true,
            },
        )
        .unwrap();

        assert_eq!(saved.ui.lock_top_bar_height, true);
        assert_eq!(saved.ui.lock_bottom_bar_height, false);
        assert_eq!(saved.ui.top_bar_height_logical, 44.0);
        assert_eq!(saved.ui.bottom_bar_height_logical, 55.0);
        assert_eq!(load_settings_from_path(&path).unwrap(), saved);
    }

    #[test]
    fn partial_nested_search_settings_default_missing_fields() {
        let path = test_dir("partial-search").join(SETTINGS_FILE);
        fs::write(
            &path,
            json!({
                "schema": SETTINGS_SCHEMA,
                "version": SETTINGS_VERSION,
                "ui": { "activeWorkspaceId": null, "enableDiagnosticsExport": false },
                "search": { "everything": { "enabled": false } },
                "workspaces": [],
                "taskHistory": []
            })
            .to_string(),
        )
        .unwrap();

        let settings = load_settings_from_path(&path).unwrap();

        assert_eq!(settings.search.result_limit, 50);
        assert!(!settings.search.everything.enabled);
        assert_eq!(settings.search.everything.max_results, 100);
        assert!(settings.search.everything.full_path_search);
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

    #[test]
    fn validates_quick_command_entries_and_rejects_secret_like_args() {
        let mut settings = ShellSettings::default();
        settings.quick_commands.entries = vec![QuickCommandEntry {
            id: "git-status".to_string(),
            label: "Git Status".to_string(),
            mode: QuickCommandMode::Direct,
            target_path: "git.exe".to_string(),
            args: vec!["status".to_string()],
            commands: Vec::new(),
            cwd: Some("C:\\dev\\jasonshell".to_string()),
        }];

        let validated = validate_settings(settings).unwrap();
        assert_eq!(validated.quick_commands.entries.len(), 1);
        assert_eq!(validated.quick_commands.entries[0].id, "git-status");

        let mut invalid = ShellSettings::default();
        invalid.quick_commands.entries = vec![QuickCommandEntry {
            id: "secret".to_string(),
            label: "Bad".to_string(),
            mode: QuickCommandMode::Direct,
            target_path: "git.exe".to_string(),
            args: vec!["--token".to_string(), "abc".to_string()],
            commands: Vec::new(),
            cwd: None,
        }];
        assert!(validate_settings(invalid)
            .unwrap_err()
            .contains("secret-like"));
    }

    #[test]
    fn validates_quick_command_unique_ids_and_command_blocks() {
        let mut settings = ShellSettings::default();
        settings.quick_commands.entries = vec![
            QuickCommandEntry {
                id: "dup".to_string(),
                label: "One".to_string(),
                mode: QuickCommandMode::Direct,
                target_path: "git.exe".to_string(),
                args: vec!["status".to_string()],
                commands: Vec::new(),
                cwd: None,
            },
            QuickCommandEntry {
                id: "dup".to_string(),
                label: "Two".to_string(),
                mode: QuickCommandMode::Direct,
                target_path: "git.exe".to_string(),
                args: vec!["status".to_string()],
                commands: Vec::new(),
                cwd: None,
            },
        ];
        assert!(validate_settings(settings).unwrap_err().contains("unique"));

        let mut block = ShellSettings::default();
        block.quick_commands.entries = vec![QuickCommandEntry {
            id: "block".to_string(),
            label: "Block".to_string(),
            mode: QuickCommandMode::CommandBlock,
            target_path: String::new(),
            args: Vec::new(),
            commands: vec![
                "cd C:\\dev\\jasonshell".to_string(),
                "python app.py".to_string(),
            ],
            cwd: None,
        }];
        let validated = validate_settings(block).unwrap();
        assert_eq!(
            validated.quick_commands.entries[0].commands,
            vec!["cd C:\\dev\\jasonshell", "python app.py"]
        );
    }
}
