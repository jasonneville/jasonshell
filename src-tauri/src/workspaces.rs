use crate::settings;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use tauri::AppHandle;

const ACTIVATION_SEARCH_BOOST: u16 = 32;
const RESERVED_RESTORATION_STATUS: &str = "reserved-not-implemented";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceProfile {
    pub id: String,
    pub name: String,
    pub root_path: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub pins: Vec<WorkspacePin>,
    #[serde(default)]
    pub tool_defaults: WorkspaceToolDefaults,
    #[serde(default)]
    pub tasks: Vec<WorkspaceTaskDeclaration>,
    #[serde(default)]
    pub startup: WorkspaceStartupSafety,
    #[serde(default)]
    pub restoration: WorkspaceRestorationReservation,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceToolDefaults {
    pub terminal: Option<String>,
    pub editor: Option<String>,
    pub shell: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePin {
    pub id: String,
    pub label: String,
    pub path: String,
    #[serde(default)]
    pub kind: WorkspacePinKind,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WorkspacePinKind {
    #[default]
    Folder,
    File,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTaskDeclaration {
    pub id: String,
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: Vec<WorkspaceEnvDeclaration>,
    #[serde(default)]
    pub expose_in_search: bool,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEnvDeclaration {
    pub name: String,
    pub value: Option<String>,
    #[serde(default)]
    pub value_source: WorkspaceEnvValueSource,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceEnvValueSource {
    #[default]
    Literal,
    Inherited,
    Prompt,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStartupSafety {
    #[serde(default)]
    pub mode: WorkspaceStartupMode,
    #[serde(default)]
    pub task_ids: Vec<String>,
    #[serde(default)]
    pub commands: Vec<WorkspaceStartupCommand>,
    #[serde(default)]
    pub env: Vec<WorkspaceEnvDeclaration>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceStartupMode {
    #[default]
    ManualOnly,
    SuggestOnly,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStartupCommand {
    pub id: String,
    pub label: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: Vec<WorkspaceEnvDeclaration>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRestorationReservation {
    #[serde(default = "reserved_restoration_status")]
    pub status: String,
}

impl Default for WorkspaceRestorationReservation {
    fn default() -> Self {
        Self {
            status: reserved_restoration_status(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceActivationPlan {
    pub workspace: WorkspaceProfile,
    pub layout: WorkspaceLayoutPlan,
    pub search: WorkspaceSearchPlan,
    pub pins: WorkspacePinsPlan,
    pub tasks: WorkspaceTasksPlan,
    pub startup: WorkspaceStartupPlan,
    pub restoration: WorkspaceRestorationPlan,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLayoutPlan {
    pub active_workspace_id: String,
    pub root_path: String,
    pub aliases: Vec<String>,
    pub window_app_restoration_status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchPlan {
    pub bias_roots: Vec<String>,
    pub aliases: Vec<String>,
    pub result_boost: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePinsPlan {
    pub top_bar: Vec<WorkspaceActivationPin>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceActivationPin {
    pub id: String,
    pub label: String,
    pub path: String,
    pub workspace_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTasksPlan {
    pub exposed: Vec<WorkspaceActivationTask>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceActivationTask {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub pinned: bool,
    pub will_execute_on_activation: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStartupPlan {
    pub mode: WorkspaceStartupMode,
    pub will_execute: bool,
    pub reason: String,
    pub task_ids: Vec<String>,
    pub commands: Vec<WorkspaceStartupCommand>,
    pub env: Vec<WorkspaceEnvDeclaration>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRestorationPlan {
    pub status: String,
}

#[tauri::command]
pub fn list_workspaces(app_handle: AppHandle) -> Result<Vec<WorkspaceProfile>, String> {
    Ok(settings::load_shell_settings_for_app(&app_handle)?.workspaces)
}

#[tauri::command]
pub fn create_workspace(
    app_handle: AppHandle,
    workspace: WorkspaceProfile,
) -> Result<WorkspaceProfile, String> {
    let workspace = normalize_workspace(workspace)?;
    let mut settings = settings::load_shell_settings_for_app(&app_handle)?;
    if settings
        .workspaces
        .iter()
        .any(|existing| existing.id == workspace.id)
    {
        return Err(format!("workspace already exists: {}", workspace.id));
    }
    if settings
        .workspaces
        .iter()
        .any(|existing| existing.name.eq_ignore_ascii_case(&workspace.name))
    {
        return Err(format!("context name must be unique: {}", workspace.name));
    }
    settings.workspaces.push(workspace.clone());
    settings::save_shell_settings_for_app(&app_handle, settings)?;
    Ok(workspace)
}

#[tauri::command]
pub fn update_workspace(
    app_handle: AppHandle,
    workspace: WorkspaceProfile,
) -> Result<WorkspaceProfile, String> {
    let workspace = normalize_workspace(workspace)?;
    let mut settings = settings::load_shell_settings_for_app(&app_handle)?;
    if settings.workspaces.iter().any(|existing| {
        existing.id != workspace.id && existing.name.eq_ignore_ascii_case(&workspace.name)
    }) {
        return Err(format!("context name must be unique: {}", workspace.name));
    }
    let Some(existing) = settings
        .workspaces
        .iter_mut()
        .find(|existing| existing.id == workspace.id)
    else {
        return Err(format!("workspace not found: {}", workspace.id));
    };
    *existing = workspace.clone();
    settings::save_shell_settings_for_app(&app_handle, settings)?;
    Ok(workspace)
}

#[tauri::command]
pub fn delete_workspace(
    app_handle: AppHandle,
    id: String,
) -> Result<Vec<WorkspaceProfile>, String> {
    let mut settings = settings::load_shell_settings_for_app(&app_handle)?;
    let initial_len = settings.workspaces.len();
    settings.workspaces.retain(|workspace| workspace.id != id);
    if settings.workspaces.len() == initial_len {
        return Err(format!("workspace not found: {id}"));
    }
    if settings.ui.active_workspace_id.as_deref() == Some(id.as_str()) {
        settings.ui.active_workspace_id = None;
    }
    let saved = settings::save_shell_settings_for_app(&app_handle, settings)?;
    Ok(saved.workspaces)
}

#[tauri::command]
pub fn activate_workspace(
    app_handle: AppHandle,
    id: String,
) -> Result<WorkspaceActivationPlan, String> {
    let mut settings = settings::load_shell_settings_for_app(&app_handle)?;
    let Some(workspace) = settings
        .workspaces
        .iter()
        .find(|workspace| workspace.id == id)
        .cloned()
    else {
        return Err(format!("workspace not found: {id}"));
    };
    settings.ui.active_workspace_id = Some(workspace.id.clone());
    settings::save_shell_settings_for_app(&app_handle, settings)?;
    Ok(build_activation_plan(&workspace))
}

pub(crate) fn normalize_workspace(
    mut workspace: WorkspaceProfile,
) -> Result<WorkspaceProfile, String> {
    workspace.id = workspace.id.trim().to_string();
    workspace.name = workspace.name.trim().to_string();
    workspace.root_path = workspace.root_path.trim().to_string();
    workspace.aliases = dedupe_trimmed(workspace.aliases);
    for pin in &mut workspace.pins {
        pin.id = pin.id.trim().to_string();
        pin.label = pin.label.trim().to_string();
        pin.path = pin.path.trim().to_string();
    }
    for task in &mut workspace.tasks {
        task.id = task.id.trim().to_string();
        task.name = task.name.trim().to_string();
        task.command = task.command.trim().to_string();
        task.cwd = task.cwd.as_ref().map(|cwd| cwd.trim().to_string());
    }
    workspace.startup.task_ids = dedupe_trimmed(workspace.startup.task_ids);
    for command in &mut workspace.startup.commands {
        command.id = command.id.trim().to_string();
        command.label = command.label.trim().to_string();
        command.command = command.command.trim().to_string();
        command.cwd = command.cwd.as_ref().map(|cwd| cwd.trim().to_string());
    }
    validate_workspace(&workspace)?;
    Ok(workspace)
}

pub(crate) fn validate_workspace(workspace: &WorkspaceProfile) -> Result<(), String> {
    validate_id("workspace id", &workspace.id)?;
    validate_label("workspace name", &workspace.name)?;
    validate_absolute_path("workspace root path", &workspace.root_path)?;
    validate_unique_strings("workspace aliases", &workspace.aliases)?;

    let mut pin_ids = HashSet::new();
    for pin in &workspace.pins {
        validate_id("workspace pin id", &pin.id)?;
        validate_label("workspace pin label", &pin.label)?;
        validate_absolute_path("workspace pin path", &pin.path)?;
        insert_unique(&mut pin_ids, "workspace pin id", &pin.id)?;
    }

    let mut task_ids = HashSet::new();
    for task in &workspace.tasks {
        validate_id("workspace task id", &task.id)?;
        validate_label("workspace task name", &task.name)?;
        validate_command_token("workspace task command", &task.command)?;
        validate_command_args("workspace task args", &task.args)?;
        if let Some(cwd) = &task.cwd {
            validate_absolute_path("workspace task cwd", cwd)?;
        }
        validate_env_declarations(&task.env)?;
        insert_unique(&mut task_ids, "workspace task id", &task.id)?;
    }
    for task_id in &workspace.startup.task_ids {
        validate_id("workspace startup task id", task_id)?;
        if !task_ids.contains(task_id) {
            return Err(format!("workspace startup task is not declared: {task_id}"));
        }
    }
    validate_env_declarations(&workspace.startup.env)?;
    let mut startup_ids = HashSet::new();
    for command in &workspace.startup.commands {
        validate_id("workspace startup command id", &command.id)?;
        validate_label("workspace startup command label", &command.label)?;
        validate_command_token("workspace startup command", &command.command)?;
        validate_command_args("workspace startup command args", &command.args)?;
        if let Some(cwd) = &command.cwd {
            validate_absolute_path("workspace startup cwd", cwd)?;
        }
        validate_env_declarations(&command.env)?;
        insert_unique(
            &mut startup_ids,
            "workspace startup command id",
            &command.id,
        )?;
    }
    if workspace.restoration.status != RESERVED_RESTORATION_STATUS {
        return Err(
            "workspace restoration is reserved and must remain reserved-not-implemented"
                .to_string(),
        );
    }
    Ok(())
}

pub(crate) fn build_activation_plan(workspace: &WorkspaceProfile) -> WorkspaceActivationPlan {
    let top_bar = if workspace.pins.is_empty() {
        vec![WorkspaceActivationPin {
            id: format!("workspace-root:{}", workspace.id),
            label: workspace.name.clone(),
            path: workspace.root_path.clone(),
            workspace_id: workspace.id.clone(),
        }]
    } else {
        workspace
            .pins
            .iter()
            .map(|pin| WorkspaceActivationPin {
                id: pin.id.clone(),
                label: pin.label.clone(),
                path: pin.path.clone(),
                workspace_id: workspace.id.clone(),
            })
            .collect()
    };
    let exposed = workspace
        .tasks
        .iter()
        .filter(|task| task.expose_in_search || task.pinned)
        .map(|task| WorkspaceActivationTask {
            id: task.id.clone(),
            name: task.name.clone(),
            command: task.command.clone(),
            args: task.args.clone(),
            cwd: task.cwd.clone(),
            pinned: task.pinned,
            will_execute_on_activation: false,
        })
        .collect();

    WorkspaceActivationPlan {
        workspace: workspace.clone(),
        layout: WorkspaceLayoutPlan {
            active_workspace_id: workspace.id.clone(),
            root_path: workspace.root_path.clone(),
            aliases: workspace.aliases.clone(),
            window_app_restoration_status: RESERVED_RESTORATION_STATUS.to_string(),
        },
        search: WorkspaceSearchPlan {
            bias_roots: vec![workspace.root_path.clone()],
            aliases: workspace.aliases.clone(),
            result_boost: ACTIVATION_SEARCH_BOOST,
        },
        pins: WorkspacePinsPlan { top_bar },
        tasks: WorkspaceTasksPlan { exposed },
        startup: WorkspaceStartupPlan {
            mode: workspace.startup.mode.clone(),
            will_execute: false,
            reason: "workspace activation only returns a startup plan; command execution is reserved for a later task-runner phase".to_string(),
            task_ids: workspace.startup.task_ids.clone(),
            commands: workspace.startup.commands.clone(),
            env: workspace.startup.env.clone(),
        },
        restoration: WorkspaceRestorationPlan {
            status: RESERVED_RESTORATION_STATUS.to_string(),
        },
    }
}

fn reserved_restoration_status() -> String {
    RESERVED_RESTORATION_STATUS.to_string()
}

fn dedupe_trimmed(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_ascii_lowercase();
        if seen.insert(key) {
            deduped.push(trimmed.to_string());
        }
    }
    deduped
}

fn validate_id(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{label} is required"));
    }
    if value.len() > 64
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(format!(
            "{label} must use only letters, numbers, dash, underscore, or dot"
        ));
    }
    Ok(())
}

fn validate_label(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} is required"));
    }
    if value.chars().count() > 120 {
        return Err(format!("{label} must be 120 characters or shorter"));
    }
    Ok(())
}

fn validate_absolute_path(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} is required"));
    }
    if value.contains('\0') || value.contains("://") {
        return Err(format!("{label} must be a local filesystem path"));
    }
    if !Path::new(value).is_absolute() {
        return Err(format!("{label} must be absolute"));
    }
    Ok(())
}

fn validate_unique_strings(label: &str, values: &[String]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(format!("{label} must not include blank values"));
        }
        insert_unique(&mut seen, label, value)?;
    }
    Ok(())
}

fn insert_unique(seen: &mut HashSet<String>, label: &str, value: &str) -> Result<(), String> {
    if !seen.insert(value.to_ascii_lowercase()) {
        return Err(format!("{label} must be unique: {value}"));
    }
    Ok(())
}

fn validate_command_token(label: &str, command: &str) -> Result<(), String> {
    if command.trim().is_empty() {
        return Err(format!("{label} is required"));
    }
    if command
        .chars()
        .any(|character| matches!(character, '&' | '|' | ';' | '<' | '>' | '\r' | '\n'))
    {
        return Err(format!(
            "{label} must be a command token, not a shell command line"
        ));
    }
    Ok(())
}

fn validate_command_args(label: &str, args: &[String]) -> Result<(), String> {
    for arg in args {
        if arg
            .chars()
            .any(|character| matches!(character, '\0' | '\r' | '\n'))
        {
            return Err(format!("{label} must not contain control characters"));
        }
        if let Some(key) = secret_like_arg_key(arg) {
            return Err(format!("{label} must not include secret-like key: {key}"));
        }
        if looks_secret_like_value(arg) {
            return Err(format!("{label} must not include secret-like values"));
        }
    }
    Ok(())
}

fn validate_env_declarations(env: &[WorkspaceEnvDeclaration]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for entry in env {
        if !is_safe_env_name(&entry.name) {
            return Err(format!(
                "workspace env name is invalid or secret-like: {}",
                entry.name
            ));
        }
        if let Some(value) = &entry.value {
            if looks_secret_like_value(value) {
                return Err(format!(
                    "workspace env value for {} looks secret-like",
                    entry.name
                ));
            }
        }
        insert_unique(&mut seen, "workspace env name", &entry.name)?;
    }
    Ok(())
}

fn is_safe_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
        && !is_secret_like_name(name)
}

fn is_secret_like_name(value: &str) -> bool {
    let value = normalize_secret_scan(value);
    [
        "token",
        "secret",
        "password",
        "credential",
        "apikey",
        "authorization",
        "cookie",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}

fn secret_like_arg_key(arg: &str) -> Option<String> {
    let trimmed = arg.trim();
    let key_candidate = if trimmed.starts_with("--") {
        trimmed.trim_start_matches('-')
    } else if trimmed.starts_with('/') {
        trimmed.trim_start_matches('/')
    } else if trimmed.contains('=') {
        trimmed
    } else {
        return None;
    };
    let key = key_candidate
        .split(['=', ':'])
        .next()
        .unwrap_or_default()
        .trim();
    if !key.is_empty() && is_secret_like_name(key) {
        Some(key.to_string())
    } else {
        None
    }
}

fn normalize_secret_scan(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn looks_secret_like_value(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    normalized.contains("bearer ")
        || normalized.contains("ghp_")
        || normalized.contains("gho_")
        || normalized.contains("github_pat_")
        || normalized.contains("xoxb-")
        || normalized.contains("sk-")
        || normalized.contains("akia")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_workspace() -> WorkspaceProfile {
        WorkspaceProfile {
            id: "jasonshell".to_string(),
            name: "JasonShell".to_string(),
            root_path: "C:\\dev\\jasonshell".to_string(),
            aliases: vec!["shell".to_string(), "Jason".to_string()],
            pins: vec![WorkspacePin {
                id: "src".to_string(),
                label: "Source".to_string(),
                path: "C:\\dev\\jasonshell\\src".to_string(),
                kind: WorkspacePinKind::Folder,
            }],
            tool_defaults: WorkspaceToolDefaults {
                terminal: Some("Windows Terminal".to_string()),
                editor: Some("VS Code".to_string()),
                shell: Some("pwsh".to_string()),
            },
            tasks: vec![WorkspaceTaskDeclaration {
                id: "validate".to_string(),
                name: "Validate".to_string(),
                command: "npm".to_string(),
                args: vec!["run".to_string(), "validate".to_string()],
                cwd: Some("C:\\dev\\jasonshell".to_string()),
                env: vec![WorkspaceEnvDeclaration {
                    name: "NODE_ENV".to_string(),
                    value: Some("development".to_string()),
                    value_source: WorkspaceEnvValueSource::Literal,
                }],
                expose_in_search: true,
                pinned: true,
            }],
            startup: WorkspaceStartupSafety {
                mode: WorkspaceStartupMode::SuggestOnly,
                task_ids: vec!["validate".to_string()],
                commands: vec![WorkspaceStartupCommand {
                    id: "open-editor".to_string(),
                    label: "Open editor".to_string(),
                    command: "code".to_string(),
                    args: vec![".".to_string()],
                    cwd: Some("C:\\dev\\jasonshell".to_string()),
                    env: Vec::new(),
                }],
                env: Vec::new(),
            },
            restoration: WorkspaceRestorationReservation::default(),
        }
    }

    #[test]
    fn validates_workspace_schema_paths_and_startup_metadata() {
        let workspace = normalize_workspace(sample_workspace()).unwrap();

        assert_eq!(workspace.aliases, ["shell", "Jason"]);
        assert!(validate_workspace(&workspace).is_ok());
    }

    #[test]
    fn rejects_relative_root_and_pin_paths() {
        let mut workspace = sample_workspace();
        workspace.root_path = "dev\\jasonshell".to_string();

        let error = validate_workspace(&workspace).unwrap_err();

        assert!(error.contains("workspace root path must be absolute"));
    }

    #[test]
    fn rejects_secret_like_workspace_env_names_and_values() {
        let mut workspace = sample_workspace();
        workspace.tasks[0].env = vec![WorkspaceEnvDeclaration {
            name: "API_TOKEN".to_string(),
            value: None,
            value_source: WorkspaceEnvValueSource::Prompt,
        }];

        let error = validate_workspace(&workspace).unwrap_err();

        assert!(error.contains("secret-like"));

        workspace.tasks[0].env = vec![WorkspaceEnvDeclaration {
            name: "SAFE_FLAG".to_string(),
            value: Some("Bearer abc".to_string()),
            value_source: WorkspaceEnvValueSource::Literal,
        }];

        let error = validate_workspace(&workspace).unwrap_err();
        assert!(error.contains("looks secret-like"));
    }

    #[test]
    fn rejects_secret_like_workspace_task_and_startup_args() {
        let mut workspace = sample_workspace();
        workspace.tasks[0].args = vec!["--api-key=abc".to_string()];

        let error = validate_workspace(&workspace).unwrap_err();

        assert!(error.contains("workspace task args"));
        assert!(error.contains("secret-like key"));

        workspace = sample_workspace();
        workspace.tasks[0].args = vec!["Bearer abc".to_string()];

        let error = validate_workspace(&workspace).unwrap_err();

        assert!(error.contains("workspace task args"));
        assert!(error.contains("secret-like values"));

        workspace = sample_workspace();
        workspace.startup.commands[0].args = vec!["/password:abc".to_string()];

        let error = validate_workspace(&workspace).unwrap_err();

        assert!(error.contains("workspace startup command args"));
        assert!(error.contains("secret-like key"));
    }

    #[test]
    fn activation_plan_never_executes_startup_or_restores_windows() {
        let workspace = sample_workspace();

        let plan = build_activation_plan(&workspace);

        assert_eq!(plan.layout.active_workspace_id, "jasonshell");
        assert_eq!(plan.pins.top_bar[0].path, "C:\\dev\\jasonshell\\src");
        assert_eq!(plan.tasks.exposed[0].id, "validate");
        assert!(!plan.tasks.exposed[0].will_execute_on_activation);
        assert!(!plan.startup.will_execute);
        assert_eq!(plan.restoration.status, RESERVED_RESTORATION_STATUS);
    }

    #[test]
    fn activation_plan_uses_root_pin_when_workspace_has_no_pins() {
        let mut workspace = sample_workspace();
        workspace.pins.clear();

        let plan = build_activation_plan(&workspace);

        assert_eq!(plan.pins.top_bar[0].label, "JasonShell");
        assert_eq!(plan.pins.top_bar[0].path, "C:\\dev\\jasonshell");
    }
}
