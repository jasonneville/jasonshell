use crate::settings::{
    self, validate_quick_command_args, validate_quick_command_commands,
    validate_quick_command_entry, QuickCommandEntry, QuickCommandMode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

const QUICK_COMMAND_HISTORY_LIMIT: usize = 20;
const QUICK_COMMAND_CAPTURE_LIMIT: usize = 16 * 1024;
static RUNNING_QUICK_COMMANDS: OnceLock<
    Mutex<HashMap<u32, settings::QuickCommandRunHistoryEntry>>,
> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunQuickCommandRequest {
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuickCommandSpawnResult {
    pub process_id: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QuickCommandSpawnPlan {
    pub(crate) label: String,
    pub(crate) executable: String,
    pub(crate) args: Vec<String>,
    pub(crate) cwd: Option<String>,
}

#[tauri::command]
pub fn run_quick_command(
    app_handle: AppHandle,
    request: RunQuickCommandRequest,
) -> Result<QuickCommandSpawnResult, String> {
    validate_run_request(&request)?;
    let settings = settings::load_shell_settings_for_app(&app_handle)?;
    let entry = resolve_quick_command_entry(&settings, &request.id)?;
    let plan = build_spawn_plan(&entry)?;
    let app_handle = app_handle.clone();
    let command_id = request.id.clone();
    let process_id = spawn_quick_command(&plan, command_id, move |run| {
        let _ = store_quick_command_history(&app_handle, run);
    })?;
    Ok(QuickCommandSpawnResult { process_id })
}

#[tauri::command]
pub fn list_quick_command_history(
    app_handle: AppHandle,
    request: RunQuickCommandRequest,
) -> Result<Vec<settings::QuickCommandRunHistoryEntry>, String> {
    validate_run_request(&request)?;
    let settings = settings::load_shell_settings_for_app(&app_handle)?;
    let mut history = running_quick_commands()
        .lock()
        .map_err(|_| "quick command runtime state is poisoned".to_string())?
        .values()
        .filter(|entry| entry.command_id == request.id)
        .cloned()
        .collect::<Vec<_>>();
    history.extend(
        settings
            .quick_commands
            .history
            .into_iter()
            .filter(|entry| entry.command_id == request.id),
    );
    history.sort_by(|left, right| right.started_at_epoch_ms.cmp(&left.started_at_epoch_ms));
    history.truncate(QUICK_COMMAND_HISTORY_LIMIT);
    Ok(history)
}

#[tauri::command]
pub fn save_quick_commands_settings(
    app_handle: AppHandle,
    quick_commands: settings::QuickCommandsSettings,
) -> Result<settings::QuickCommandsSettings, String> {
    settings::update_shell_settings_for_app(&app_handle, |settings| {
        replace_quick_command_entries(&mut settings.quick_commands, quick_commands.entries);
    })
    .map(|settings| settings.quick_commands)
}

fn validate_run_request(request: &RunQuickCommandRequest) -> Result<(), String> {
    if request.id.trim().is_empty() {
        return Err("quick command id must not be empty".to_string());
    }
    Ok(())
}

pub(crate) fn resolve_quick_command_entry(
    settings: &settings::ShellSettings,
    command_id: &str,
) -> Result<QuickCommandEntry, String> {
    let Some(entry) = settings
        .quick_commands
        .entries
        .iter()
        .find(|entry| entry.id == command_id)
    else {
        return Err(format!("quick command '{}' is not configured", command_id));
    };
    validate_quick_command_entry(entry)
}

pub(crate) fn build_spawn_plan(entry: &QuickCommandEntry) -> Result<QuickCommandSpawnPlan, String> {
    if let Some(cwd) = entry.cwd.as_deref() {
        if !Path::new(cwd).is_dir() {
            return Err(format!(
                "quick command '{}' cwd does not exist: {}",
                entry.id, cwd
            ));
        }
    }
    if !entry.args.is_empty() {
        let _ = validate_quick_command_args(&entry.args, &entry.id)?;
    }

    match entry.mode {
        QuickCommandMode::Direct => Ok(QuickCommandSpawnPlan {
            label: entry.label.clone(),
            executable: entry.target_path.clone(),
            args: entry.args.clone(),
            cwd: entry.cwd.clone(),
        }),
        QuickCommandMode::CommandBlock => {
            let commands = validate_quick_command_commands(&entry.commands, &entry.id)?;
            Ok(QuickCommandSpawnPlan {
                label: entry.label.clone(),
                executable: "pwsh.exe".to_string(),
                args: vec![
                    "-NoLogo".to_string(),
                    "-NoProfile".to_string(),
                    "-Command".to_string(),
                    commands.join("\r\n"),
                ],
                cwd: entry.cwd.clone(),
            })
        }
        QuickCommandMode::PowershellFile => {
            let mut commands = vec![
                "-NoLogo".to_string(),
                "-NoProfile".to_string(),
                "-File".to_string(),
                entry.target_path.clone(),
            ];
            commands.extend(entry.args.clone());
            build_spawn_plan(&QuickCommandEntry {
                mode: QuickCommandMode::CommandBlock,
                target_path: String::new(),
                args: Vec::new(),
                commands: vec![format!("pwsh.exe {}", commands.join(" "))],
                ..entry.clone()
            })
        }
        QuickCommandMode::CmdFile => {
            let mut commands = vec!["/C".to_string(), entry.target_path.clone()];
            commands.extend(entry.args.clone());
            build_spawn_plan(&QuickCommandEntry {
                mode: QuickCommandMode::CommandBlock,
                target_path: String::new(),
                args: Vec::new(),
                commands: vec![format!("cmd.exe {}", commands.join(" "))],
                ..entry.clone()
            })
        }
    }
}

pub(crate) fn spawn_quick_command(
    plan: &QuickCommandSpawnPlan,
    command_id: String,
    on_exit: impl FnOnce(settings::QuickCommandRunHistoryEntry) + Send + 'static,
) -> Result<u32, String> {
    let mut command = Command::new(&plan.executable);
    command
        .args(&plan.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = &plan.cwd {
        command.current_dir(cwd);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to run quick command '{}': {error}", plan.executable))?;
    let process_id = child.id();
    let started_at_epoch_ms = current_epoch_ms();
    let running = settings::QuickCommandRunHistoryEntry {
        command_id,
        started_at_epoch_ms,
        finished_at_epoch_ms: 0,
        process_id,
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        running: true,
    };
    running_quick_commands()
        .lock()
        .map_err(|_| "quick command runtime state is poisoned".to_string())?
        .insert(process_id, running);
    std::thread::spawn(move || {
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdout_reader = stdout.map(|stream| {
            std::thread::spawn(move || {
                capture_stream(stream, |chunk| {
                    append_running_output(process_id, true, chunk)
                })
            })
        });
        let stderr_reader = stderr.map(|stream| {
            std::thread::spawn(move || {
                capture_stream(stream, |chunk| {
                    append_running_output(process_id, false, chunk)
                })
            })
        });
        let status = child.wait();
        let finished_at_epoch_ms = current_epoch_ms();
        let (stdout, stdout_truncated) = stdout_reader
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default();
        let (stderr, stderr_truncated) = stderr_reader
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default();
        let exit_code = status.ok().and_then(|status| status.code());
        let mut completed = running_quick_commands()
            .lock()
            .ok()
            .and_then(|mut runs| runs.remove(&process_id))
            .unwrap_or(settings::QuickCommandRunHistoryEntry {
                command_id: String::new(),
                started_at_epoch_ms,
                finished_at_epoch_ms,
                process_id,
                exit_code,
                stdout: String::new(),
                stderr: String::new(),
                stdout_truncated: false,
                stderr_truncated: false,
                running: false,
            });
        completed.finished_at_epoch_ms = finished_at_epoch_ms;
        completed.exit_code = exit_code;
        completed.stdout = stdout;
        completed.stderr = stderr;
        completed.stdout_truncated = stdout_truncated;
        completed.stderr_truncated = stderr_truncated;
        completed.running = false;
        on_exit(completed);
    });
    Ok(process_id)
}

fn store_quick_command_history(
    app_handle: &AppHandle,
    entry: settings::QuickCommandRunHistoryEntry,
) -> Result<(), String> {
    settings::update_shell_settings_for_app(app_handle, |settings| {
        settings.quick_commands.history.insert(0, entry);
        settings
            .quick_commands
            .history
            .truncate(QUICK_COMMAND_HISTORY_LIMIT);
    })
    .map(|_| ())
}

fn replace_quick_command_entries(
    quick_commands: &mut settings::QuickCommandsSettings,
    entries: Vec<QuickCommandEntry>,
) {
    let active_ids = entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    quick_commands
        .history
        .retain(|entry| active_ids.contains(entry.command_id.as_str()));
    quick_commands.entries = entries;
}

fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn capture_stream(mut stream: impl Read, mut on_chunk: impl FnMut(&[u8])) -> (String, bool) {
    let mut captured = Vec::with_capacity(QUICK_COMMAND_CAPTURE_LIMIT);
    let mut buffer = [0_u8; 4096];
    let mut truncated = false;

    loop {
        let read = match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        let remaining = QUICK_COMMAND_CAPTURE_LIMIT.saturating_sub(captured.len());
        let kept = read.min(remaining);
        captured.extend_from_slice(&buffer[..kept]);
        if kept > 0 {
            on_chunk(&buffer[..kept]);
        }
        truncated |= kept < read;
    }

    (String::from_utf8_lossy(&captured).to_string(), truncated)
}

fn running_quick_commands() -> &'static Mutex<HashMap<u32, settings::QuickCommandRunHistoryEntry>> {
    RUNNING_QUICK_COMMANDS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn append_running_output(process_id: u32, is_stdout: bool, chunk: &[u8]) {
    let Ok(mut runs) = running_quick_commands().lock() else {
        return;
    };
    let Some(run) = runs.get_mut(&process_id) else {
        return;
    };
    let output = String::from_utf8_lossy(chunk);
    if is_stdout {
        run.stdout.push_str(&output);
    } else {
        run.stderr.push_str(&output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(mode: QuickCommandMode, target_path: String, args: Vec<&str>) -> QuickCommandEntry {
        QuickCommandEntry {
            id: "quick".to_string(),
            label: "Quick".to_string(),
            mode,
            target_path,
            args: args.into_iter().map(|value| value.to_string()).collect(),
            commands: Vec::new(),
            cwd: None,
        }
    }

    #[test]
    fn validates_run_request_requires_id() {
        let error =
            validate_run_request(&RunQuickCommandRequest { id: "".to_string() }).unwrap_err();
        assert!(error.contains("must not be empty"));
    }

    #[test]
    fn builds_direct_mode_spawn_plan_without_shell_wrapping() {
        let plan = build_spawn_plan(&entry(
            QuickCommandMode::Direct,
            "git.exe".to_string(),
            vec!["status", "--short"],
        ))
        .unwrap();
        assert_eq!(plan.executable, "git.exe");
        assert_eq!(plan.args, vec!["status", "--short"]);
    }

    #[test]
    fn builds_command_block_spawn_plan_as_powershell_command_text() {
        let mut command = entry(QuickCommandMode::CommandBlock, String::new(), vec![]);
        command.commands = vec![
            "cd C:\\dev\\jasonshell".to_string(),
            "python app.py".to_string(),
        ];
        let plan = build_spawn_plan(&command).unwrap();
        assert_eq!(plan.executable, "pwsh.exe");
        assert_eq!(plan.args[0], "-NoLogo");
        assert_eq!(plan.args[2], "-Command");
        assert_eq!(plan.args[3], "cd C:\\dev\\jasonshell\r\npython app.py");
    }

    #[test]
    fn rejects_secret_like_args_from_spawn_plan() {
        let error = build_spawn_plan(&entry(
            QuickCommandMode::Direct,
            "git.exe".to_string(),
            vec!["--token", "abc"],
        ))
        .unwrap_err();
        assert!(error.contains("secret-like"));
    }

    #[test]
    fn bounds_captured_output() {
        let output = vec![b'x'; QUICK_COMMAND_CAPTURE_LIMIT + 1];
        let (captured, truncated) = capture_stream(output.as_slice(), |_| {});
        assert_eq!(captured.len(), QUICK_COMMAND_CAPTURE_LIMIT);
        assert!(truncated);
    }

    #[test]
    fn replacing_entries_keeps_history_for_active_commands() {
        let run = settings::QuickCommandRunHistoryEntry {
            command_id: "quick".to_string(),
            started_at_epoch_ms: 1,
            finished_at_epoch_ms: 2,
            process_id: 1,
            exit_code: Some(0),
            stdout: "ok".to_string(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            running: false,
        };
        let mut quick_commands = settings::QuickCommandsSettings {
            entries: vec![entry(
                QuickCommandMode::Direct,
                "git.exe".to_string(),
                vec![],
            )],
            history: vec![
                run.clone(),
                settings::QuickCommandRunHistoryEntry {
                    command_id: "removed".to_string(),
                    ..run
                },
            ],
        };

        replace_quick_command_entries(
            &mut quick_commands,
            vec![entry(
                QuickCommandMode::Direct,
                "git.exe".to_string(),
                vec![],
            )],
        );

        assert_eq!(quick_commands.history.len(), 1);
        assert_eq!(quick_commands.history[0].command_id, "quick");
    }
}
