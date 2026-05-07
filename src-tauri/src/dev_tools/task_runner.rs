use crate::settings::{self, ShellSettings};
use crate::workspaces::WorkspaceTaskDeclaration;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

pub const TASK_STARTED_EVENT: &str = "task:started";
pub const TASK_OUTPUT_EVENT: &str = "task:output";
pub const TASK_COMPLETED_EVENT: &str = "task:completed";
const TASK_HISTORY_SCHEMA: &str = "jasonshell.taskHistory";
const TASK_HISTORY_VERSION: u32 = 1;
const TASK_HISTORY_FILE: &str = "jasonshell-task-history-v1.json";
const MAX_TASK_HISTORY_ENTRIES: usize = 50;
const MAX_TASK_OUTPUT_CHUNK_BYTES: usize = 8 * 1024;

static NEXT_TASK_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static RUNNING_TASKS: OnceLock<Mutex<HashMap<String, RunningTask>>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct WorkspaceTaskRequest {
    pub workspace_id: Option<String>,
    pub task_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedWorkspaceTask {
    workspace_id: String,
    workspace_path: String,
    label: String,
    executable: String,
    args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskSpawnResponse {
    pub task_id: String,
    pub process_id: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskProcessMetadata {
    pub task_id: String,
    pub process_id: u32,
    pub workspace_id: Option<String>,
    pub workspace_path: String,
    pub label: String,
    pub started_at_epoch_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryEntry {
    pub task_id: String,
    pub workspace_id: Option<String>,
    pub workspace_path: String,
    pub label: String,
    pub executable: String,
    pub args: Vec<String>,
    pub process_id: u32,
    pub started_at_epoch_ms: u64,
    pub finished_at_epoch_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub canceled: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskOutputEvent {
    task_id: String,
    stream: &'static str,
    chunk: String,
    sequence: u64,
    timestamp_epoch_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskCompletedEvent {
    task_id: String,
    exit_code: Option<i32>,
    canceled: bool,
    success: bool,
    timestamp_epoch_ms: u64,
}

struct RunningTask {
    child: Arc<Mutex<Child>>,
    canceled: Arc<AtomicBool>,
    metadata: TaskProcessMetadata,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskHistoryFile {
    schema: String,
    version: u32,
    entries: Vec<TaskHistoryEntry>,
}

impl Default for TaskHistoryFile {
    fn default() -> Self {
        Self {
            schema: TASK_HISTORY_SCHEMA.to_string(),
            version: TASK_HISTORY_VERSION,
            entries: Vec::new(),
        }
    }
}

#[tauri::command]
pub fn spawn_workspace_task(
    app_handle: AppHandle,
    request: WorkspaceTaskRequest,
) -> Result<TaskSpawnResponse, String> {
    validate_task_request(&request)?;
    let settings = settings::load_shell_settings_for_app(&app_handle)?;
    let resolved = resolve_declared_task(&settings, &request)?;
    let task_id = next_task_id();
    let started_at_epoch_ms = current_epoch_ms();
    let mut child = spawn_task_child(&resolved)?;
    let process_id = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let child = Arc::new(Mutex::new(child));
    let canceled = Arc::new(AtomicBool::new(false));
    let metadata = TaskProcessMetadata {
        task_id: task_id.clone(),
        process_id,
        workspace_id: Some(resolved.workspace_id.clone()),
        workspace_path: resolved.workspace_path.clone(),
        label: resolved.label.clone(),
        started_at_epoch_ms,
    };
    insert_running_task(
        task_id.clone(),
        RunningTask {
            child: Arc::clone(&child),
            canceled: Arc::clone(&canceled),
            metadata: metadata.clone(),
        },
    )?;

    let _ = app_handle.emit(TASK_STARTED_EVENT, &metadata);
    let output_sequence = Arc::new(AtomicU64::new(1));
    if let Some(stdout) = stdout {
        spawn_output_reader(
            app_handle.clone(),
            task_id.clone(),
            "stdout",
            stdout,
            Arc::clone(&output_sequence),
        );
    }
    if let Some(stderr) = stderr {
        spawn_output_reader(
            app_handle.clone(),
            task_id.clone(),
            "stderr",
            stderr,
            Arc::clone(&output_sequence),
        );
    }
    spawn_waiter(
        app_handle,
        resolved,
        task_id.clone(),
        child,
        canceled,
        metadata,
    );

    Ok(TaskSpawnResponse {
        task_id,
        process_id,
    })
}

#[tauri::command]
pub fn cancel_workspace_task(task_id: String) -> Result<(), String> {
    let running = {
        let tasks = running_tasks();
        let tasks = tasks
            .lock()
            .map_err(|_| "running task registry is unavailable".to_string())?;
        tasks
            .get(&task_id)
            .map(|task| (Arc::clone(&task.child), Arc::clone(&task.canceled)))
    };
    let Some((child, canceled)) = running else {
        return Err(format!("task '{task_id}' is not running"));
    };
    canceled.store(true, Ordering::SeqCst);
    let mut child = child
        .lock()
        .map_err(|_| "running task process is unavailable".to_string())?;
    // Cancellation is intentionally capped to the direct child process. Full
    // process-tree termination needs separate OS-specific guardrails.
    child
        .kill()
        .map_err(|error| format!("failed to cancel task '{task_id}': {error}"))
}

#[tauri::command]
pub fn list_workspace_task_history(app_handle: AppHandle) -> Result<Vec<TaskHistoryEntry>, String> {
    let path = history_path(&app_handle)?;
    Ok(load_history_file(&path)?.entries)
}

#[tauri::command]
pub fn list_jasonshell_task_process_metadata() -> Result<Vec<TaskProcessMetadata>, String> {
    let tasks = running_tasks();
    let tasks = tasks
        .lock()
        .map_err(|_| "running task registry is unavailable".to_string())?;
    let mut entries = tasks
        .values()
        .map(|task| task.metadata.clone())
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.started_at_epoch_ms.cmp(&right.started_at_epoch_ms));
    Ok(entries)
}

pub fn validate_task_request(request: &WorkspaceTaskRequest) -> Result<(), String> {
    if request.task_id.trim().is_empty() {
        return Err("workspace task id must not be empty".to_string());
    }
    if let Some(workspace_id) = &request.workspace_id {
        if workspace_id.trim().is_empty() {
            return Err("workspace id must not be empty".to_string());
        }
    }
    Ok(())
}

pub fn append_history_entry_bounded(
    mut entries: Vec<TaskHistoryEntry>,
    entry: TaskHistoryEntry,
) -> Vec<TaskHistoryEntry> {
    entries.push(entry);
    if entries.len() > MAX_TASK_HISTORY_ENTRIES {
        entries.drain(0..entries.len() - MAX_TASK_HISTORY_ENTRIES);
    }
    entries
}

fn spawn_task_child(task: &ResolvedWorkspaceTask) -> Result<Child, String> {
    validate_resolved_task(task)?;
    Command::new(&task.executable)
        .args(&task.args)
        .current_dir(&task.workspace_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn task '{}': {error}", task.label))
}

fn spawn_output_reader<R>(
    app_handle: AppHandle,
    task_id: String,
    stream: &'static str,
    reader: R,
    sequence: Arc<AtomicU64>,
) where
    R: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = vec![0_u8; MAX_TASK_OUTPUT_CHUNK_BYTES];
        loop {
            let Ok(bytes) = reader.read(&mut buffer) else {
                break;
            };
            if bytes == 0 {
                break;
            }
            let event = TaskOutputEvent {
                task_id: task_id.clone(),
                stream,
                chunk: String::from_utf8_lossy(&buffer[..bytes]).to_string(),
                sequence: next_output_sequence(&sequence),
                timestamp_epoch_ms: current_epoch_ms(),
            };
            let _ = app_handle.emit(TASK_OUTPUT_EVENT, event);
        }
    });
}

fn spawn_waiter(
    app_handle: AppHandle,
    task: ResolvedWorkspaceTask,
    task_id: String,
    child: Arc<Mutex<Child>>,
    canceled: Arc<AtomicBool>,
    metadata: TaskProcessMetadata,
) {
    thread::spawn(move || {
        let exit_code = loop {
            let try_wait_result = child
                .lock()
                .ok()
                .and_then(|mut child| child.try_wait().ok());
            match try_wait_result {
                Some(Some(status)) => break status.code(),
                Some(None) => thread::sleep(Duration::from_millis(50)),
                None => break None,
            }
        };
        let canceled = canceled.load(Ordering::SeqCst);
        let finished_at_epoch_ms = current_epoch_ms();
        remove_running_task(&task_id);
        let entry = TaskHistoryEntry {
            task_id: task_id.clone(),
            workspace_id: metadata.workspace_id.clone(),
            workspace_path: metadata.workspace_path.clone(),
            label: task.label,
            executable: task.executable,
            args: task.args,
            process_id: metadata.process_id,
            started_at_epoch_ms: metadata.started_at_epoch_ms,
            finished_at_epoch_ms: Some(finished_at_epoch_ms),
            exit_code,
            canceled,
        };
        let _ = append_task_history(&app_handle, entry);
        let _ = app_handle.emit(
            TASK_COMPLETED_EVENT,
            TaskCompletedEvent {
                task_id,
                exit_code,
                canceled,
                success: exit_code == Some(0) && !canceled,
                timestamp_epoch_ms: finished_at_epoch_ms,
            },
        );
    });
}

fn append_task_history(app_handle: &AppHandle, entry: TaskHistoryEntry) -> Result<(), String> {
    let path = history_path(app_handle)?;
    let mut file = load_history_file(&path)?;
    file.entries = append_history_entry_bounded(file.entries, entry);
    save_history_file(&path, &file)
}

fn history_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    app_handle
        .path()
        .app_local_data_dir()
        .map(|dir| dir.join(TASK_HISTORY_FILE))
        .map_err(|error| format!("failed to resolve task history directory: {error}"))
}

fn load_history_file(path: &Path) -> Result<TaskHistoryFile, String> {
    if !path.exists() {
        return Ok(TaskHistoryFile::default());
    }
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read task history: {error}"))?;
    let mut file = serde_json::from_str::<TaskHistoryFile>(&raw)
        .map_err(|error| format!("failed to parse task history: {error}"))?;
    if file.schema != TASK_HISTORY_SCHEMA || file.version != TASK_HISTORY_VERSION {
        return Err("unsupported task history file version".to_string());
    }
    if file.entries.len() > MAX_TASK_HISTORY_ENTRIES {
        file.entries = file
            .entries
            .split_off(file.entries.len() - MAX_TASK_HISTORY_ENTRIES);
    }
    Ok(file)
}

fn save_history_file(path: &Path, file: &TaskHistoryFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create task history directory: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(file)
        .map_err(|error| format!("failed to serialize task history: {error}"))?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, bytes)
        .map_err(|error| format!("failed to write task history temp file: {error}"))?;
    fs::rename(&temp_path, path).map_err(|error| {
        format!(
            "failed to replace task history file {}: {error}",
            json!({ "path": path.display().to_string() })
        )
    })
}

fn insert_running_task(task_id: String, task: RunningTask) -> Result<(), String> {
    let tasks = running_tasks();
    let mut tasks = tasks
        .lock()
        .map_err(|_| "running task registry is unavailable".to_string())?;
    tasks.insert(task_id, task);
    Ok(())
}

fn remove_running_task(task_id: &str) {
    let Ok(mut tasks) = running_tasks().lock() else {
        return;
    };
    tasks.remove(task_id);
}

fn running_tasks() -> &'static Mutex<HashMap<String, RunningTask>> {
    RUNNING_TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_task_id() -> String {
    let sequence = NEXT_TASK_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    format!("task-{}-{sequence}", current_epoch_ms())
}

fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn resolve_declared_task(
    settings: &ShellSettings,
    request: &WorkspaceTaskRequest,
) -> Result<ResolvedWorkspaceTask, String> {
    let active_workspace_id = settings
        .ui
        .active_workspace_id
        .as_deref()
        .ok_or_else(|| "no active workspace is available for task execution".to_string())?;
    let requested_workspace_id = request
        .workspace_id
        .as_deref()
        .unwrap_or(active_workspace_id)
        .trim();
    if requested_workspace_id != active_workspace_id {
        return Err(format!(
            "workspace task execution is limited to the active workspace: {active_workspace_id}"
        ));
    }
    let workspace = settings
        .workspaces
        .iter()
        .find(|workspace| workspace.id == active_workspace_id)
        .ok_or_else(|| format!("active workspace is not persisted: {active_workspace_id}"))?;
    let task = workspace
        .tasks
        .iter()
        .find(|task| task.id == request.task_id)
        .ok_or_else(|| {
            format!(
                "workspace task is not declared in active workspace '{}': {}",
                workspace.id, request.task_id
            )
        })?;
    resolved_task_from_declaration(&workspace.id, &workspace.root_path, task)
}

fn resolved_task_from_declaration(
    workspace_id: &str,
    workspace_root: &str,
    task: &WorkspaceTaskDeclaration,
) -> Result<ResolvedWorkspaceTask, String> {
    let workspace_path = task.cwd.as_deref().unwrap_or(workspace_root).to_string();
    let resolved = ResolvedWorkspaceTask {
        workspace_id: workspace_id.to_string(),
        workspace_path,
        label: task.name.clone(),
        executable: task.command.clone(),
        args: task.args.clone(),
    };
    validate_resolved_task(&resolved)?;
    Ok(resolved)
}

fn validate_resolved_task(task: &ResolvedWorkspaceTask) -> Result<(), String> {
    if task.workspace_path.trim().is_empty() {
        return Err("workspace path must not be empty".to_string());
    }
    if !Path::new(&task.workspace_path).is_dir() {
        return Err("workspace path must be an existing directory".to_string());
    }
    if task.label.trim().is_empty() {
        return Err("task label must not be empty".to_string());
    }
    validate_executable(&task.executable)?;
    for arg in &task.args {
        reject_control_chars(arg, "task argument")?;
    }
    Ok(())
}

fn next_output_sequence(sequence: &AtomicU64) -> u64 {
    sequence.fetch_add(1, Ordering::SeqCst)
}

fn validate_executable(executable: &str) -> Result<(), String> {
    if executable.trim().is_empty() {
        return Err("task executable must not be empty".to_string());
    }
    if executable.chars().any(|ch| {
        matches!(
            ch,
            '"' | '\'' | '&' | '|' | ';' | '<' | '>' | '\n' | '\r' | '\0'
        )
    }) {
        return Err(
            "task executable must be a literal program path without shell metacharacters"
                .to_string(),
        );
    }
    Ok(())
}

fn reject_control_chars(value: &str, label: &str) -> Result<(), String> {
    if value.chars().any(|ch| matches!(ch, '\n' | '\r' | '\0')) {
        return Err(format!("{label} must not contain control characters"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{ShellSettings, ShellUiSettings};
    use crate::workspaces::{
        WorkspaceEnvValueSource, WorkspaceRestorationReservation, WorkspaceStartupSafety,
        WorkspaceTaskDeclaration, WorkspaceToolDefaults,
    };
    use std::env;

    #[test]
    fn rejects_direct_arbitrary_command_request_payloads() {
        let payload = json!({
            "workspaceId": "workspace-a",
            "taskId": "validate",
            "executable": "cmd.exe",
            "args": ["/C", "whoami"]
        });

        let error = serde_json::from_value::<WorkspaceTaskRequest>(payload).unwrap_err();

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn resolves_only_declared_tasks_from_active_workspace_settings() {
        let settings = task_settings();
        let request = WorkspaceTaskRequest {
            workspace_id: Some("workspace-a".to_string()),
            task_id: "validate".to_string(),
        };

        let resolved = resolve_declared_task(&settings, &request).unwrap();

        assert_eq!(resolved.executable, "cmd.exe");
        assert_eq!(
            resolved.args,
            vec!["/C".to_string(), "echo validate".to_string()]
        );
        assert_eq!(
            resolved.workspace_path,
            env::temp_dir().to_string_lossy().to_string()
        );
    }

    #[test]
    fn rejects_inactive_or_undeclared_workspace_task_identity() {
        let settings = task_settings();
        let inactive = WorkspaceTaskRequest {
            workspace_id: Some("workspace-b".to_string()),
            task_id: "validate".to_string(),
        };
        let undeclared = WorkspaceTaskRequest {
            workspace_id: Some("workspace-a".to_string()),
            task_id: "whoami".to_string(),
        };

        assert!(resolve_declared_task(&settings, &inactive)
            .unwrap_err()
            .contains("active workspace"));
        assert!(resolve_declared_task(&settings, &undeclared)
            .unwrap_err()
            .contains("not declared"));
    }

    #[test]
    fn bounds_task_history_to_latest_entries() {
        let entries = (0..60)
            .map(|index| TaskHistoryEntry {
                task_id: format!("task-{index}"),
                workspace_id: None,
                workspace_path: env::temp_dir().to_string_lossy().to_string(),
                label: "test".to_string(),
                executable: "cmd".to_string(),
                args: Vec::new(),
                process_id: index,
                started_at_epoch_ms: index as u64,
                finished_at_epoch_ms: Some(index as u64),
                exit_code: Some(0),
                canceled: false,
            })
            .collect::<Vec<_>>();

        let bounded = append_history_entry_bounded(
            entries,
            TaskHistoryEntry {
                task_id: "task-60".to_string(),
                workspace_id: None,
                workspace_path: env::temp_dir().to_string_lossy().to_string(),
                label: "test".to_string(),
                executable: "cmd".to_string(),
                args: Vec::new(),
                process_id: 60,
                started_at_epoch_ms: 60,
                finished_at_epoch_ms: Some(60),
                exit_code: Some(0),
                canceled: false,
            },
        );

        assert_eq!(bounded.len(), 50);
        assert_eq!(bounded.first().unwrap().task_id, "task-11");
        assert_eq!(bounded.last().unwrap().task_id, "task-60");
    }

    #[test]
    fn spawn_and_cancel_task_process_without_shell() {
        let request = long_running_task();
        let mut child = spawn_task_child(&request).unwrap();
        let pid = child.id();

        child.kill().unwrap();
        let status = child.wait().unwrap();

        assert!(pid > 0);
        assert!(!status.success());
    }

    #[test]
    fn output_sequences_are_task_monotonic_and_chunks_are_bounded() {
        let sequence = AtomicU64::new(1);

        assert_eq!(next_output_sequence(&sequence), 1);
        assert_eq!(next_output_sequence(&sequence), 2);
        assert!(MAX_TASK_OUTPUT_CHUNK_BYTES <= 8 * 1024);
    }

    fn long_running_task() -> ResolvedWorkspaceTask {
        #[cfg(target_os = "windows")]
        {
            ResolvedWorkspaceTask {
                workspace_id: "workspace-a".to_string(),
                workspace_path: env::temp_dir().to_string_lossy().to_string(),
                label: "sleep".to_string(),
                executable: "cmd.exe".to_string(),
                args: vec!["/C".to_string(), "ping 127.0.0.1 -n 6 > nul".to_string()],
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            ResolvedWorkspaceTask {
                workspace_id: "workspace-a".to_string(),
                workspace_path: env::temp_dir().to_string_lossy().to_string(),
                label: "sleep".to_string(),
                executable: "sh".to_string(),
                args: vec!["-c".to_string(), "sleep 5".to_string()],
            }
        }
    }

    fn task_settings() -> ShellSettings {
        ShellSettings {
            schema: "jasonshell.settings".to_string(),
            version: 1,
            ui: ShellUiSettings {
                active_workspace_id: Some("workspace-a".to_string()),
                enable_diagnostics_export: false,
                search_mode: Default::default(),
            },
            search: Default::default(),
            workspaces: vec![crate::workspaces::WorkspaceProfile {
                id: "workspace-a".to_string(),
                name: "Workspace A".to_string(),
                root_path: env::temp_dir().to_string_lossy().to_string(),
                aliases: Vec::new(),
                pins: Vec::new(),
                tool_defaults: WorkspaceToolDefaults::default(),
                tasks: vec![WorkspaceTaskDeclaration {
                    id: "validate".to_string(),
                    name: "Validate".to_string(),
                    command: "cmd.exe".to_string(),
                    args: vec!["/C".to_string(), "echo validate".to_string()],
                    cwd: None,
                    env: vec![crate::workspaces::WorkspaceEnvDeclaration {
                        name: "NODE_ENV".to_string(),
                        value: Some("development".to_string()),
                        value_source: WorkspaceEnvValueSource::Literal,
                    }],
                    expose_in_search: true,
                    pinned: true,
                }],
                startup: WorkspaceStartupSafety::default(),
                restoration: WorkspaceRestorationReservation::default(),
            }],
            task_history: Vec::new(),
            quick_commands: Default::default(),
        }
    }
}
