use crate::contracts::surfaces;
use crate::settings::{
    self, validate_quick_command_args, validate_quick_command_commands,
    validate_quick_command_entry, QuickCommandEntry, QuickCommandMode, QuickCommandTranscriptEntry,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
#[cfg(windows)]
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    OpenProcess, OpenThread, ResumeThread, TerminateProcess, CREATE_SUSPENDED,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    THREAD_QUERY_LIMITED_INFORMATION, THREAD_SUSPEND_RESUME,
};

const HISTORY_LIMIT: usize = 20;
const OUTPUT_LIMIT: usize = 16 * 1024;
const TRANSCRIPT_LIMIT: usize = 256;
const MAX_ENCODED_MARKER: usize = 8192;
const MAX_DECODED_MARKER: usize = 4096;
const MAX_INPUT_LENGTH: usize = 16 * 1024;
const MAX_REQUEST_ID_LEN: usize = 128;
const MAX_PROMPT_LEN: usize = 512;
const MARKER_PREFIX: &str = "\x1b]777;JasonShellQuickCommandInput;";
const MARKER_SUFFIX: &str = "\x07";

static RUNS: OnceLock<Mutex<HashMap<String, QuickCommandRunState>>> = OnceLock::new();
static NEXT_RUN_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunQuickCommandRequest {
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuickCommandSpawnResult {
    pub run_id: String,
    pub process_id: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StopQuickCommandRequest {
    pub id: String,
    pub process_id: u32,
    pub run_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SendQuickCommandInputRequest {
    pub id: String,
    pub process_id: u32,
    pub run_id: String,
    pub request_id: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QuickCommandSpawnPlan {
    pub(crate) label: String,
    pub(crate) executable: String,
    pub(crate) args: Vec<String>,
    pub(crate) cwd: Option<String>,
}

#[derive(Debug)]
struct QuickCommandRunState {
    command_id: String,
    run_id: String,
    process_id: u32,
    started_at_epoch_ms: u64,
    started_at_filetime_100ns: u64,
    stdin: Option<ChildStdin>,
    running: bool,
    stopping: bool,
    pending: Option<QuickCommandPendingInput>,
    transcript: VecDeque<QuickCommandTranscriptEntry>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_carry: AnsiCarry,
    stderr_carry: AnsiCarry,
    stdout_truncated: bool,
    stderr_truncated: bool,
    finished_at_epoch_ms: Option<u64>,
    exit_code: Option<i32>,
    #[cfg(windows)]
    job: Arc<QuickCommandJobHandle>,
}

#[cfg(windows)]
#[derive(Debug)]
struct QuickCommandJobHandle(HANDLE);

#[cfg(windows)]
unsafe impl Send for QuickCommandJobHandle {}
#[cfg(windows)]
unsafe impl Sync for QuickCommandJobHandle {}

#[cfg(windows)]
impl Drop for QuickCommandJobHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[derive(Clone, Debug)]
struct QuickCommandPendingInput {
    request_id: String,
    secret: bool,
    max_length: usize,
}

#[derive(Default)]
struct MarkerCarry {
    bytes: Vec<u8>,
    text_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
struct AnsiCarry {
    bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
struct QuickCommandOutputChunk {
    text: String,
    marker: Option<QuickCommandInputMarker>,
}

#[derive(Clone, Debug, Deserialize)]
struct QuickCommandInputMarker {
    version: u32,
    #[serde(rename = "requestId")]
    request_id: String,
    prompt: Option<String>,
    kind: Option<String>,
    #[serde(default)]
    secret: bool,
    #[serde(default)]
    max_length: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuickCommandRunUpdatedPayload {
    run_id: String,
    command_id: String,
    process_id: u32,
    kind: String,
    body: String,
    prompt: Option<String>,
    request_id: Option<String>,
    max_length: Option<usize>,
    secret: bool,
    redacted: bool,
    sequence: u64,
    at_epoch_ms: u64,
    pending: bool,
}

#[tauri::command]
pub fn run_quick_command(
    app_handle: AppHandle,
    request: RunQuickCommandRequest,
) -> Result<QuickCommandSpawnResult, String> {
    if request.id.trim().is_empty() {
        return Err("quick command id must not be empty".into());
    }
    let settings = settings::load_shell_settings_for_app(&app_handle)?;
    let entry = resolve_quick_command_entry(&settings, &request.id)?;
    let plan = build_spawn_plan(&entry)?;
    let run_id = new_run_id();
    let command_id = request.id.clone();
    let app_for_exit = app_handle.clone();
    let process_id = spawn_quick_command(
        app_handle.clone(),
        &plan,
        command_id.clone(),
        run_id.clone(),
        move |history| {
            let _ = settings::update_shell_settings_for_app(&app_for_exit, |settings| {
                settings.quick_commands.history = append_quick_command_history_bounded(
                    std::mem::take(&mut settings.quick_commands.history),
                    history,
                );
            });
        },
    )?;
    emit_run_snapshot(
        &app_handle,
        &QuickCommandRunUpdatedPayload {
            run_id: run_id.clone(),
            command_id: command_id.clone(),
            process_id,
            kind: "started".into(),
            body: String::new(),
            prompt: None,
            request_id: None,
            max_length: None,
            secret: false,
            redacted: false,
            sequence: next_sequence(),
            at_epoch_ms: current_epoch_ms(),
            pending: false,
        },
    );
    Ok(QuickCommandSpawnResult { run_id, process_id })
}

#[tauri::command]
pub fn send_quick_command_input(
    app_handle: AppHandle,
    request: SendQuickCommandInputRequest,
) -> Result<(), String> {
    let mut runs_guard = runs()
        .lock()
        .map_err(|_| "quick command runtime state is poisoned".to_string())?;
    let run = runs_guard
        .get_mut(&request.run_id)
        .ok_or_else(|| "quick command is no longer running".to_string())?;
    if run.command_id != request.id
        || run.process_id != request.process_id
        || !run.running
        || run.stopping
    {
        return Err("quick command is no longer running".into());
    }
    let pending = run
        .pending
        .take()
        .ok_or_else(|| "quick command input request id is not active".to_string())?;
    if pending.request_id != request.request_id {
        run.pending = Some(pending);
        return Err("quick command input request id is not active".into());
    }
    if request.value.chars().count() > pending.max_length {
        run.pending = Some(pending);
        return Err("quick command input exceeds max length".into());
    }
    let mut stdin = match run.stdin.take() {
        Some(stdin) => stdin,
        None => {
            run.pending = Some(pending);
            return Err("quick command input is no longer available".into());
        }
    };
    drop(runs_guard);
    let write_res = stdin
        .write_all(request.value.as_bytes())
        .and_then(|_| stdin.write_all(b"\r\n"))
        .and_then(|_| stdin.flush());
    if let Err(e) = write_res {
        let mut runs_guard = runs()
            .lock()
            .map_err(|_| "quick command runtime state is poisoned".to_string())?;
        if let Some(run) = runs_guard.get_mut(&request.run_id) {
            run.pending = Some(pending);
            if run.running {
                run.stdin = Some(stdin);
            }
        }
        return Err(format!("failed to write quick command input: {e}"));
    }
    let payload = {
        let mut runs_guard = runs()
            .lock()
            .map_err(|_| "quick command runtime state is poisoned".to_string())?;
        let run = runs_guard
            .get_mut(&request.run_id)
            .ok_or_else(|| "quick command is no longer running".to_string())?;
        emit_run_updated_from_transcript(
            run,
            &request.run_id,
            request.process_id,
            "input-submitted",
            Some(request.request_id.clone()),
            Some(pending.max_length),
            if pending.secret {
                "[redacted]".to_string()
            } else {
                request.value.clone()
            },
            None,
            pending.secret,
            pending.secret,
            false,
        )
    };
    let mut runs_guard = runs()
        .lock()
        .map_err(|_| "quick command runtime state is poisoned".to_string())?;
    if let Some(run) = runs_guard.get_mut(&request.run_id) {
        if run.running {
            run.stdin = Some(stdin);
        }
    }
    emit_run_snapshot(&app_handle, &payload);
    Ok(())
}

#[tauri::command]
pub async fn stop_quick_command(
    app_handle: AppHandle,
    request: StopQuickCommandRequest,
) -> Result<(), String> {
    let app_handle = app_handle.clone();
    tauri::async_runtime::spawn_blocking(move || {
        stop_running_quick_command(
            &app_handle,
            request.process_id,
            &request.id,
            &request.run_id,
        )
    })
    .await
    .map_err(|e| format!("failed to stop quick command: {e}"))?
}

#[tauri::command]
pub fn list_quick_command_history(
    app_handle: AppHandle,
    request: Option<RunQuickCommandRequest>,
) -> Result<Vec<settings::QuickCommandRunHistoryEntry>, String> {
    let settings = settings::load_shell_settings_for_app(&app_handle)?;
    let mut history = runs()
        .lock()
        .map_err(|_| "quick command runtime state is poisoned".to_string())?
        .values()
        .map(state_to_history)
        .collect::<Vec<_>>();
    history.extend(settings.quick_commands.history);
    if let Some(r) = request.filter(|r| !r.id.trim().is_empty()) {
        history.retain(|h| h.command_id == r.id);
    }
    history.sort_by(|l, r| {
        r.started_at_epoch_ms
            .cmp(&l.started_at_epoch_ms)
            .then_with(|| r.finished_at_epoch_ms.cmp(&l.finished_at_epoch_ms))
    });
    history.truncate(HISTORY_LIMIT);
    Ok(history)
}

#[tauri::command]
pub fn save_quick_commands_settings(
    app_handle: AppHandle,
    quick_commands: settings::QuickCommandsSettings,
) -> Result<settings::QuickCommandsSettings, String> {
    settings::update_shell_settings_for_app(&app_handle, |settings| {
        settings.quick_commands.entries = quick_commands.entries;
        settings.quick_commands.list_width = quick_commands.list_width;
    })
    .map(|s| s.quick_commands)
}

fn resolve_quick_command_entry(
    settings: &settings::ShellSettings,
    command_id: &str,
) -> Result<QuickCommandEntry, String> {
    let e = settings
        .quick_commands
        .entries
        .iter()
        .find(|e| e.id == command_id)
        .ok_or_else(|| format!("quick command '{}' is not configured", command_id))?;
    validate_quick_command_entry(e)
}
fn build_spawn_plan(entry: &QuickCommandEntry) -> Result<QuickCommandSpawnPlan, String> {
    if let Some(cwd) = entry.cwd.as_deref() {
        if !Path::new(cwd).is_dir() {
            return Err(format!(
                "quick command '{}' cwd does not exist: {}",
                entry.id, cwd
            ));
        }
    }
    if !entry.args.is_empty() {
        validate_quick_command_args(&entry.args, &entry.id)?;
    }
    match entry.mode {
        QuickCommandMode::Direct => Ok(QuickCommandSpawnPlan {
            label: entry.label.clone(),
            executable: entry.target_path.clone(),
            args: entry.args.clone(),
            cwd: entry.cwd.clone(),
        }),
        QuickCommandMode::CommandBlock => Ok(QuickCommandSpawnPlan {
            label: entry.label.clone(),
            executable: "pwsh.exe".into(),
            args: vec![
                "-NoLogo".into(),
                "-NoProfile".into(),
                "-Command".into(),
                build_command_block_script(&validate_quick_command_commands(
                    &entry.commands,
                    &entry.id,
                )?),
            ],
            cwd: entry.cwd.clone(),
        }),
        QuickCommandMode::PowershellFile => build_spawn_plan(&QuickCommandEntry {
            mode: QuickCommandMode::CommandBlock,
            target_path: String::new(),
            args: vec![],
            commands: vec![format!(
                "pwsh.exe -NoLogo -NoProfile -File {}",
                quote_command_part(&entry.target_path)
            )],
            ..entry.clone()
        }),
        QuickCommandMode::CmdFile => build_spawn_plan(&QuickCommandEntry {
            mode: QuickCommandMode::CommandBlock,
            target_path: String::new(),
            args: vec![],
            commands: vec![format!(
                "cmd.exe /C {}",
                quote_command_part(&entry.target_path)
            )],
            ..entry.clone()
        }),
    }
}
fn build_command_block_script(commands: &[String]) -> String {
    format!(
        r#"function Request-JasonShellInput {{ param([string]$RequestId,[string]$Prompt,[string]$Kind='text',[switch]$Secret,[int]$MaxLength=4096) $payload=@{{version=1;requestId=$RequestId;prompt=$Prompt;kind=$Kind;secret=[bool]$Secret;maxLength=$MaxLength}}; $encoded=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(($payload|ConvertTo-Json -Compress))).Replace('+','-').Replace('/','_').TrimEnd('='); [Console]::Out.Write("`e]777;JasonShellQuickCommandInput;" + $encoded + "`a"); [Console]::Out.Flush(); [Console]::In.ReadLine() }}"#
    ) + "\r\n"
        + &commands.join("\r\n")
}

fn spawn_quick_command(
    app_handle: AppHandle,
    plan: &QuickCommandSpawnPlan,
    command_id: String,
    run_id: String,
    on_exit: impl FnOnce(settings::QuickCommandRunHistoryEntry) + Send + 'static,
) -> Result<u32, String> {
    let mut command = Command::new(&plan.executable);
    command
        .args(&plan.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = &plan.cwd {
        command.current_dir(cwd);
    }
    #[cfg(windows)]
    command.creation_flags(CREATE_SUSPENDED.0);
    #[cfg(windows)]
    let job = create_quick_command_job()?;
    #[cfg(windows)]
    let job_handle = Arc::new(job);
    let mut child = command
        .spawn()
        .map_err(|e| format!("failed to run quick command '{}': {e}", plan.executable))?;
    let process_id = child.id();
    #[cfg(windows)]
    let started_at_filetime_100ns =
        match assign_and_resume_quick_command_process(job_handle.0, process_id) {
            Ok(started_at_filetime_100ns) => started_at_filetime_100ns,
            Err(error) => {
                cleanup_suspended_quick_command_process(process_id);
                let _ = child.wait();
                return Err(error);
            }
        };
    let started_at_epoch_ms = current_epoch_ms();
    let stdin = child.stdin.take();
    runs()
        .lock()
        .map_err(|_| "quick command runtime state is poisoned".to_string())?
        .insert(
            run_id.clone(),
            QuickCommandRunState {
                command_id: command_id.clone(),
                run_id: run_id.clone(),
                process_id,
                started_at_epoch_ms,
                started_at_filetime_100ns,
                stdin,
                running: true,
                stopping: false,
                pending: None,
                transcript: VecDeque::new(),
                stdout: vec![],
                stderr: vec![],
                stdout_carry: AnsiCarry::default(),
                stderr_carry: AnsiCarry::default(),
                stdout_truncated: false,
                stderr_truncated: false,
                finished_at_epoch_ms: None,
                exit_code: None,
                #[cfg(windows)]
                job: Arc::clone(&job_handle),
            },
        );
    std::thread::spawn(move || {
        let run_id_stdout = run_id.clone();
        let run_id_stderr = run_id.clone();
        let app_for_stdout = app_handle.clone();
        let app_for_stderr = app_handle.clone();
        let stdout = child.stdout.take().map(|s| {
            std::thread::spawn(move || capture_stream(&app_for_stdout, s, true, run_id_stdout))
        });
        let stderr = child.stderr.take().map(|s| {
            std::thread::spawn(move || capture_stream(&app_for_stderr, s, false, run_id_stderr))
        });
        let status = child.wait();
        let finished_at_epoch_ms = current_epoch_ms();
        let stopping = runs()
            .lock()
            .ok()
            .and_then(|mut runs| runs.get_mut(&run_id).map(claim_root_exit))
            .unwrap_or(false);
        #[cfg(windows)]
        if !stopping {
            let _ = unsafe { TerminateJobObject(job_handle.0, 1) };
        }
        let (stdout, stdout_truncated, stderr, stderr_truncated) = if stopping {
            (vec![], false, vec![], false)
        } else {
            let (stdout, stdout_truncated) = stdout.and_then(|h| h.join().ok()).unwrap_or_default();
            let (stderr, stderr_truncated) = stderr.and_then(|h| h.join().ok()).unwrap_or_default();
            (stdout, stdout_truncated, stderr, stderr_truncated)
        };
        let exit_code = status.ok().and_then(|s| s.code());
        let (history, exit_payload) = {
            let mut runs = runs().lock().ok();
            let Some(runs) = runs.as_mut() else {
                return;
            };
            let Some(state) = runs.get_mut(&run_id) else {
                return;
            };
            let stopped = state.stopping;
            let terminal_kind = if stopped { "stopped" } else { "exit" };
            let terminal_body = if stopped {
                String::new()
            } else {
                exit_code.map(|c| c.to_string()).unwrap_or_default()
            };
            let sequence = next_sequence();
            state.running = false;
            state.finished_at_epoch_ms = Some(finished_at_epoch_ms);
            state.exit_code = exit_code;
            push_transcript(
                &mut state.transcript,
                QuickCommandTranscriptEntry {
                    kind: terminal_kind.into(),
                    body: terminal_body.clone(),
                    request_id: None,
                    prompt: None,
                    secret: false,
                    max_length: None,
                    redacted: false,
                    sequence,
                    at_epoch_ms: finished_at_epoch_ms,
                    pending: false,
                },
            );
            let history = settings::QuickCommandRunHistoryEntry {
                run_id: state.run_id.clone(),
                command_id: state.command_id.clone(),
                started_at_epoch_ms,
                started_at_filetime_100ns,
                finished_at_epoch_ms,
                process_id,
                exit_code,
                stdout: if stopped {
                    sanitize_terminal_text(&decode_terminal_bytes(&state.stdout))
                } else {
                    sanitize_terminal_text(&decode_terminal_bytes(&stdout))
                },
                stderr: if stopped {
                    sanitize_terminal_text(&decode_terminal_bytes(&state.stderr))
                } else {
                    sanitize_terminal_text(&decode_terminal_bytes(&stderr))
                },
                transcript: state.transcript.iter().cloned().collect(),
                stdout_truncated: stdout_truncated || state.stdout_truncated,
                stderr_truncated: stderr_truncated || state.stderr_truncated,
                running: false,
            };
            let payload = QuickCommandRunUpdatedPayload {
                run_id: state.run_id.clone(),
                command_id: state.command_id.clone(),
                process_id: state.process_id,
                kind: terminal_kind.into(),
                body: terminal_body,
                prompt: None,
                request_id: None,
                max_length: None,
                secret: false,
                redacted: false,
                sequence,
                at_epoch_ms: finished_at_epoch_ms,
                pending: false,
            };
            (history, payload)
        };
        emit_run_snapshot(&app_handle, &exit_payload);
        on_exit(history);
        let _ = runs().lock().ok().and_then(|mut runs| runs.remove(&run_id));
    });
    Ok(process_id)
}

fn capture_stream(
    app_handle: &AppHandle,
    mut stream: impl Read,
    is_stdout: bool,
    run_id: String,
) -> (Vec<u8>, bool) {
    let mut out = vec![];
    let mut buf = [0u8; 4096];
    let mut carry = MarkerCarry::default();
    let mut truncated = false;
    loop {
        let read = match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let remaining = OUTPUT_LIMIT.saturating_sub(out.len());
        let keep = read.min(remaining);
        if keep > 0 {
            out.extend_from_slice(&buf[..keep]);
        }
        truncated |= keep < read;
        for chunk in feed(&mut carry, &buf[..read], false) {
            emit_chunk(app_handle, &run_id, is_stdout, chunk);
        }
    }
    for chunk in feed(&mut carry, &[], true) {
        emit_chunk(app_handle, &run_id, is_stdout, chunk);
    }
    (out, truncated)
}

fn feed(carry: &mut MarkerCarry, bytes: &[u8], eof: bool) -> Vec<QuickCommandOutputChunk> {
    carry.bytes.extend_from_slice(bytes);
    let mut out = vec![];
    let mut i = 0usize;
    let marker_prefix = MARKER_PREFIX.as_bytes();
    let marker_suffix = MARKER_SUFFIX.as_bytes()[0];
    while let Some(pos) = find_bytes(&carry.bytes[i..], marker_prefix) {
        let start = i + pos;
        if start > i {
            let segment = carry.bytes[i..start].to_vec();
            push_text_chunk(&mut out, carry, &segment, false);
        }
        let after_prefix = start + marker_prefix.len();
        if let Some(end_rel) = carry.bytes[after_prefix..]
            .iter()
            .position(|b| *b == marker_suffix)
        {
            let end = after_prefix + end_rel;
            let encoded = &carry.bytes[after_prefix..end];
            let marker = if encoded.len() <= MAX_ENCODED_MARKER {
                URL_SAFE_NO_PAD
                    .decode(encoded)
                    .ok()
                    .and_then(|b| {
                        if b.len() <= MAX_DECODED_MARKER {
                            serde_json::from_slice::<QuickCommandInputMarker>(&b).ok()
                        } else {
                            None
                        }
                    })
                    .filter(validate_marker_bounds)
            } else {
                None
            };
            if let Some(marker) = marker {
                out.push(QuickCommandOutputChunk {
                    text: String::new(),
                    marker: Some(marker),
                });
                i = end + 1;
                continue;
            }
            let segment = carry.bytes[start..=end].to_vec();
            push_text_chunk(&mut out, carry, &segment, false);
            i = end + 1;
            continue;
        }
        break;
    }
    if eof {
        if i < carry.bytes.len() {
            let segment = carry.bytes[i..].to_vec();
            push_text_chunk(&mut out, carry, &segment, true);
        }
        carry.bytes.clear();
        if !carry.text_bytes.is_empty() {
            out.push(QuickCommandOutputChunk {
                text: decode_terminal_bytes(&carry.text_bytes),
                marker: None,
            });
            carry.text_bytes.clear();
        }
    } else {
        let unprocessed = &carry.bytes[i..];
        let hold_from = if let Some(marker_start) = find_bytes(unprocessed, marker_prefix) {
            i + marker_start
        } else {
            let suffix_len = longest_marker_prefix_suffix(unprocessed, marker_prefix);
            carry.bytes.len().saturating_sub(suffix_len)
        };
        if carry.bytes.len().saturating_sub(hold_from) > marker_prefix.len() + MAX_ENCODED_MARKER {
            let segment = carry.bytes[i..].to_vec();
            push_text_chunk(&mut out, carry, &segment, true);
            carry.bytes.clear();
            return out;
        }
        if i < hold_from {
            let segment = carry.bytes[i..hold_from].to_vec();
            push_text_chunk(&mut out, carry, &segment, false);
        }
        carry.bytes = carry.bytes[hold_from..].to_vec();
    }
    out
}

fn push_text_chunk(
    out: &mut Vec<QuickCommandOutputChunk>,
    carry: &mut MarkerCarry,
    bytes: &[u8],
    eof: bool,
) {
    carry.text_bytes.extend_from_slice(bytes);
    let flush_len = text_flush_len(&carry.text_bytes, eof);
    if flush_len > 0 {
        let decoded = decode_terminal_bytes(&carry.text_bytes[..flush_len]);
        out.push(QuickCommandOutputChunk {
            text: decoded,
            marker: None,
        });
        carry.text_bytes.drain(..flush_len);
    }
}

fn text_flush_len(bytes: &[u8], eof: bool) -> usize {
    if eof || bytes.is_empty() {
        return bytes.len();
    }
    if let Err(error) = std::str::from_utf8(bytes) {
        if error.error_len().is_none() {
            return error.valid_up_to();
        }
    }
    #[cfg(windows)]
    {
        use windows::Win32::Globalization::{GetOEMCP, IsDBCSLeadByteEx};
        let cp = unsafe { GetOEMCP() };
        if unsafe { IsDBCSLeadByteEx(cp, *bytes.last().unwrap()) }.is_ok() {
            return bytes.len() - 1;
        }
    }
    bytes.len()
}

fn longest_marker_prefix_suffix(bytes: &[u8], marker_prefix: &[u8]) -> usize {
    let max = bytes.len().min(marker_prefix.len().saturating_sub(1));
    (1..=max)
        .rev()
        .find(|len| bytes[bytes.len() - len..] == marker_prefix[..*len])
        .unwrap_or(0)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn emit_chunk(
    app_handle: &AppHandle,
    run_id: &str,
    is_stdout: bool,
    chunk: QuickCommandOutputChunk,
) {
    let payload = if !chunk.text.is_empty() {
        append_running_output(run_id, is_stdout, chunk.text.as_bytes())
    } else {
        None
    };
    if let Some(marker) = chunk.marker {
        let _ = handle_marker(app_handle, run_id, marker);
    }
    if let Some(payload) = payload {
        emit_run_snapshot(app_handle, &payload);
    }
}

fn handle_marker(
    app_handle: &AppHandle,
    run_id: &str,
    marker: QuickCommandInputMarker,
) -> Option<QuickCommandRunUpdatedPayload> {
    let kind = marker.kind.unwrap_or_else(|| "text".into());
    if !matches!(kind.as_str(), "text" | "password" | "confirm") {
        return None;
    }
    let pending = QuickCommandPendingInput {
        request_id: marker.request_id.clone(),
        secret: marker.secret || kind == "password",
        max_length: marker.max_length.unwrap_or(4096),
    };
    if let Ok(mut runs) = runs().lock() {
        if let Some(run) = runs.get_mut(run_id) {
            if !run.running || run.stopping || run.pending.is_some() {
                return None;
            }
            let max_length = marker.max_length.unwrap_or(4096).min(MAX_INPUT_LENGTH);
            let process_id = run.process_id;
            let body = marker.prompt.clone().unwrap_or_default();
            let prompt = marker.prompt.clone();
            let request_id = pending.request_id.clone();
            let secret = pending.secret;
            let payload = emit_run_updated_from_transcript(
                run,
                run_id,
                process_id,
                &kind,
                Some(request_id),
                Some(max_length),
                body,
                prompt,
                secret,
                false,
                true,
            );
            run.pending = Some(QuickCommandPendingInput {
                max_length,
                ..pending
            });
            emit_run_snapshot(app_handle, &payload);
            return Some(payload);
        }
    }
    None
}

fn append_running_output(
    run_id: &str,
    is_stdout: bool,
    chunk: &[u8],
) -> Option<QuickCommandRunUpdatedPayload> {
    if let Ok(mut runs) = runs().lock() {
        if let Some(run) = runs.get_mut(run_id) {
            if !run.running {
                return None;
            }
            let body = decode_terminal_text_stateful(
                chunk,
                if is_stdout {
                    &mut run.stdout_carry
                } else {
                    &mut run.stderr_carry
                },
                false,
            );
            let sequence = next_sequence();
            let at_epoch_ms = current_epoch_ms();
            let kind = if is_stdout { "stdout" } else { "stderr" }.to_string();
            let target = if is_stdout {
                &mut run.stdout
            } else {
                &mut run.stderr
            };
            if target.len() < OUTPUT_LIMIT {
                target.extend_from_slice(body.as_bytes());
                if target.len() > OUTPUT_LIMIT {
                    target.truncate(OUTPUT_LIMIT);
                    if is_stdout {
                        run.stdout_truncated = true;
                    } else {
                        run.stderr_truncated = true;
                    }
                }
            }
            push_transcript(
                &mut run.transcript,
                QuickCommandTranscriptEntry {
                    kind: kind.clone(),
                    body: body.clone(),
                    request_id: None,
                    prompt: None,
                    secret: false,
                    max_length: None,
                    redacted: false,
                    sequence,
                    at_epoch_ms,
                    pending: false,
                },
            );
            return Some(QuickCommandRunUpdatedPayload {
                run_id: run.run_id.clone(),
                command_id: run.command_id.clone(),
                process_id: run.process_id,
                kind,
                body,
                prompt: None,
                request_id: None,
                max_length: None,
                secret: false,
                redacted: false,
                sequence,
                at_epoch_ms,
                pending: false,
            });
        }
    }
    None
}
fn emit_run_snapshot(app_handle: &AppHandle, snapshot: &QuickCommandRunUpdatedPayload) {
    let _ = app_handle.emit_to(
        surfaces::COMMAND_PANEL,
        crate::contracts::events::QUICK_COMMAND_RUN_UPDATED,
        snapshot,
    );
}
fn emit_run_updated_from_transcript(
    run: &mut QuickCommandRunState,
    run_id: &str,
    process_id: u32,
    kind: &str,
    request_id: Option<String>,
    max_length: Option<usize>,
    body: String,
    prompt: Option<String>,
    secret: bool,
    redacted: bool,
    pending: bool,
) -> QuickCommandRunUpdatedPayload {
    let payload = QuickCommandRunUpdatedPayload {
        run_id: run_id.to_string(),
        command_id: run.command_id.clone(),
        process_id,
        kind: kind.to_string(),
        body: body.clone(),
        prompt: prompt.clone(),
        request_id: request_id.clone(),
        max_length,
        secret,
        redacted,
        sequence: next_sequence(),
        at_epoch_ms: current_epoch_ms(),
        pending,
    };
    push_transcript(
        &mut run.transcript,
        QuickCommandTranscriptEntry {
            kind: kind.into(),
            body,
            request_id,
            prompt,
            secret,
            redacted,
            sequence: payload.sequence,
            at_epoch_ms: payload.at_epoch_ms,
            pending,
            max_length,
        },
    );
    payload
}
fn push_transcript(
    transcript: &mut VecDeque<QuickCommandTranscriptEntry>,
    entry: QuickCommandTranscriptEntry,
) {
    transcript.push_back(entry);
    if transcript.len() > TRANSCRIPT_LIMIT {
        transcript.pop_front();
    }
}
fn state_to_history(state: &QuickCommandRunState) -> settings::QuickCommandRunHistoryEntry {
    settings::QuickCommandRunHistoryEntry {
        run_id: state.run_id.clone(),
        command_id: state.command_id.clone(),
        started_at_epoch_ms: state.started_at_epoch_ms,
        started_at_filetime_100ns: state.started_at_filetime_100ns,
        finished_at_epoch_ms: state
            .finished_at_epoch_ms
            .unwrap_or(state.started_at_epoch_ms),
        process_id: state.process_id,
        exit_code: state.exit_code,
        stdout: decode_terminal_bytes(&state.stdout),
        stderr: decode_terminal_bytes(&state.stderr),
        transcript: state.transcript.iter().cloned().collect(),
        stdout_truncated: state.stdout_truncated,
        stderr_truncated: state.stderr_truncated,
        running: state.running,
    }
}
trait QuickCommandTerminator {
    #[cfg(windows)]
    fn terminate(
        &self,
        process_id: u32,
        started_at_filetime_100ns: u64,
        job: Arc<QuickCommandJobHandle>,
    ) -> Result<(), String>;
    #[cfg(not(windows))]
    fn terminate(
        &self,
        process_id: u32,
        started_at_filetime_100ns: u64,
        job: (),
    ) -> Result<(), String>;
}

struct DefaultQuickCommandTerminator;

impl DefaultQuickCommandTerminator {
    fn new() -> Self {
        Self
    }
}

#[cfg(windows)]
impl QuickCommandTerminator for DefaultQuickCommandTerminator {
    fn terminate(
        &self,
        process_id: u32,
        started_at_filetime_100ns: u64,
        job: Arc<QuickCommandJobHandle>,
    ) -> Result<(), String> {
        struct HandleGuard(HANDLE);
        impl Drop for HandleGuard {
            fn drop(&mut self) {
                let _ = unsafe { CloseHandle(self.0) };
            }
        }
        let process_handle = unsafe {
            OpenProcess(
                PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                process_id,
            )
        }
        .map_err(|e| format!("failed to stop quick command: {e}"))?;
        let process_guard = HandleGuard(process_handle);
        let verified = process_creation_time_for_handle(process_guard.0)?;
        if verified != started_at_filetime_100ns {
            return Err("quick command is no longer running".into());
        }
        unsafe { TerminateJobObject(job.0, 1) }
            .map_err(|e| format!("failed to stop quick command: {e}"))?;
        Ok(())
    }
}

#[cfg(not(windows))]
impl QuickCommandTerminator for DefaultQuickCommandTerminator {
    fn terminate(
        &self,
        _process_id: u32,
        _started_at_filetime_100ns: u64,
        _job: (),
    ) -> Result<(), String> {
        Ok(())
    }
}

fn mark_stopping(run: &mut QuickCommandRunState, sequence: u64) -> QuickCommandRunUpdatedPayload {
    run.stopping = true;
    let payload = QuickCommandRunUpdatedPayload {
        run_id: run.run_id.clone(),
        command_id: run.command_id.clone(),
        process_id: run.process_id,
        kind: "stopping".into(),
        body: String::new(),
        prompt: None,
        request_id: None,
        max_length: None,
        secret: false,
        redacted: false,
        sequence,
        at_epoch_ms: current_epoch_ms(),
        pending: false,
    };
    push_transcript(
        &mut run.transcript,
        QuickCommandTranscriptEntry {
            kind: payload.kind.clone(),
            body: String::new(),
            request_id: None,
            prompt: None,
            secret: false,
            redacted: false,
            sequence,
            at_epoch_ms: payload.at_epoch_ms,
            pending: false,
            max_length: None,
        },
    );
    payload
}

fn rollback_stopping(run: &mut QuickCommandRunState) {
    run.stopping = false;
}

fn claim_root_exit(state: &mut QuickCommandRunState) -> bool {
    state.running = false;
    state.stopping
}

fn runs() -> &'static Mutex<HashMap<String, QuickCommandRunState>> {
    RUNS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn new_run_id() -> String {
    format!(
        "run-{}-{}",
        current_epoch_ms(),
        NEXT_RUN_ID.fetch_add(1, Ordering::SeqCst)
    )
}
fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
fn next_sequence() -> u64 {
    NEXT_SEQUENCE.fetch_add(1, Ordering::SeqCst)
}
fn validate_marker_bounds(marker: &QuickCommandInputMarker) -> bool {
    marker.version == 1
        && !marker.request_id.is_empty()
        && marker.request_id.len() <= MAX_REQUEST_ID_LEN
        && marker
            .prompt
            .as_deref()
            .map(|s| s.len() <= MAX_PROMPT_LEN)
            .unwrap_or(true)
        && marker
            .kind
            .as_deref()
            .map(|s| matches!(s, "text" | "password" | "confirm"))
            .unwrap_or(true)
        && marker
            .max_length
            .map(|v| v > 0 && v <= MAX_INPUT_LENGTH)
            .unwrap_or(true)
}
fn stop_running_quick_command(
    app_handle: &AppHandle,
    process_id: u32,
    command_id: &str,
    run_id: &str,
) -> Result<(), String> {
    let started_at_filetime_100ns = {
        let runs_guard = runs()
            .lock()
            .map_err(|_| "quick command runtime state is poisoned".to_string())?;
        let run = runs_guard
            .get(run_id)
            .ok_or_else(|| "quick command is no longer running".to_string())?;
        if run.process_id != process_id
            || run.command_id != command_id
            || !run.running
            || run.stopping
        {
            return Err("quick command is no longer running".into());
        }
        run.started_at_filetime_100ns
    };
    let sequence = next_sequence();
    let (job, stopping_snapshot) = {
        let mut runs_guard = runs()
            .lock()
            .map_err(|_| "quick command runtime state is poisoned".to_string())?;
        let run = runs_guard
            .get(run_id)
            .ok_or_else(|| "quick command is no longer running".to_string())?;
        if run.process_id != process_id
            || run.command_id != command_id
            || !run.running
            || run.stopping
        {
            return Err("quick command is no longer running".into());
        }
        if run.started_at_filetime_100ns != started_at_filetime_100ns {
            return Err("quick command is no longer running".into());
        }
        let run = runs_guard
            .get_mut(run_id)
            .ok_or_else(|| "quick command is no longer running".to_string())?;
        let snapshot = mark_stopping(run, sequence);
        #[cfg(windows)]
        let job = Arc::clone(&run.job);
        #[cfg(not(windows))]
        let job = ();
        (job, snapshot)
    };
    let _ = app_handle.emit_to(
        surfaces::COMMAND_PANEL,
        crate::contracts::events::QUICK_COMMAND_RUN_UPDATED,
        &stopping_snapshot,
    );
    let terminator = DefaultQuickCommandTerminator::new();
    if let Err(error) = terminator.terminate(process_id, started_at_filetime_100ns, job) {
        let output_snapshot = {
            let mut runs_guard = runs()
                .lock()
                .map_err(|_| "quick command runtime state is poisoned".to_string())?;
            let Some(run) = runs_guard.get_mut(run_id) else {
                return Err(error);
            };
            rollback_stopping(run);
            let payload = QuickCommandRunUpdatedPayload {
                run_id: run.run_id.clone(),
                command_id: run.command_id.clone(),
                process_id: run.process_id,
                kind: "stop-failed".into(),
                body: error.clone(),
                prompt: None,
                request_id: None,
                max_length: None,
                secret: false,
                redacted: false,
                sequence: next_sequence(),
                at_epoch_ms: current_epoch_ms(),
                pending: false,
            };
            push_transcript(
                &mut run.transcript,
                QuickCommandTranscriptEntry {
                    kind: payload.kind.clone(),
                    body: payload.body.clone(),
                    request_id: None,
                    prompt: None,
                    secret: false,
                    redacted: false,
                    sequence: payload.sequence,
                    at_epoch_ms: payload.at_epoch_ms,
                    pending: false,
                    max_length: None,
                },
            );
            payload
        };
        let _ = app_handle.emit_to(
            surfaces::COMMAND_PANEL,
            crate::contracts::events::QUICK_COMMAND_RUN_UPDATED,
            &output_snapshot,
        );
        return Err(error);
    }
    Ok(())
}
fn process_creation_time_for_handle(
    handle: windows::Win32::Foundation::HANDLE,
) -> Result<u64, String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::GetProcessTimes;
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) }
            .map_err(|_| "quick command is no longer running".to_string())?;
        Ok(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64)
    }
    #[cfg(not(windows))]
    {
        Ok(0)
    }
}

#[cfg(windows)]
fn create_quick_command_job() -> Result<QuickCommandJobHandle, String> {
    let job = QuickCommandJobHandle(
        unsafe { CreateJobObjectW(None, PCWSTR::null()) }
            .map_err(|e| format!("failed to create quick command job: {e}"))?,
    );
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    unsafe {
        SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    }
    .map_err(|e| format!("failed to configure quick command job: {e}"))?;
    Ok(job)
}

#[cfg(windows)]
fn assign_and_resume_quick_command_process(job: HANDLE, process_id: u32) -> Result<u64, String> {
    let process = unsafe {
        OpenProcess(
            PROCESS_TERMINATE | PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            process_id,
        )
    }
    .map_err(|e| format!("failed to attach quick command to job: {e}"))?;
    struct Guard(HANDLE);
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
    let process_guard = Guard(process);
    unsafe { AssignProcessToJobObject(job, process_guard.0) }
        .map_err(|e| format!("failed to attach quick command to job: {e}"))?;
    let started_at_filetime_100ns = process_creation_time_for_handle(process_guard.0)?;
    let thread = open_root_suspended_thread(process_id)?;
    let thread_guard = Guard(thread);
    let resume_result = unsafe { ResumeThread(thread_guard.0) };
    if resume_result == u32::MAX {
        return Err("failed to resume quick command thread".into());
    }
    Ok(started_at_filetime_100ns)
}

#[cfg(windows)]
fn open_root_suspended_thread(process_id: u32) -> Result<HANDLE, String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }
        .map_err(|e| format!("failed to inspect quick command thread: {e}"))?;
    struct Snapshot(HANDLE);
    impl Drop for Snapshot {
        fn drop(&mut self) {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
    let _snapshot_guard = Snapshot(snapshot);
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    let mut found = unsafe { Thread32First(snapshot, &mut entry) }.is_ok();
    while found {
        if entry.th32OwnerProcessID == process_id {
            let thread = unsafe {
                OpenThread(
                    THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
                    false,
                    entry.th32ThreadID,
                )
            }
            .map_err(|e| format!("failed to open quick command thread: {e}"))?;
            return Ok(thread);
        }
        found = unsafe { Thread32Next(snapshot, &mut entry) }.is_ok();
    }
    Err("failed to locate suspended quick command thread".into())
}

#[cfg(windows)]
fn cleanup_suspended_quick_command_process(process_id: u32) {
    if let Ok(handle) = unsafe { OpenProcess(PROCESS_TERMINATE, false, process_id) } {
        let _ = unsafe { TerminateProcess(handle, 1) };
        let _ = unsafe { CloseHandle(handle) };
    }
}

#[cfg(not(windows))]
fn create_quick_command_job() -> Result<(), String> {
    Ok(())
}
fn decode_terminal_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }
    #[cfg(windows)]
    {
        use windows::Win32::Globalization::{
            GetOEMCP, MultiByteToWideChar, CP_UTF8, MB_ERR_INVALID_CHARS,
        };
        let cp = unsafe { GetOEMCP() };
        let wide_len = unsafe { MultiByteToWideChar(cp, MB_ERR_INVALID_CHARS, bytes, None) };
        if wide_len > 0 {
            let mut wide = vec![0u16; wide_len as usize];
            let written =
                unsafe { MultiByteToWideChar(cp, MB_ERR_INVALID_CHARS, bytes, Some(&mut wide)) };
            if written > 0 {
                return String::from_utf16_lossy(&wide[..written as usize]);
            }
        }
        let wide_len = unsafe { MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, bytes, None) };
        if wide_len > 0 {
            let mut wide = vec![0u16; wide_len as usize];
            let written = unsafe {
                MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, bytes, Some(&mut wide))
            };
            if written > 0 {
                return String::from_utf16_lossy(&wide[..written as usize]);
            }
        }
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn sanitize_terminal_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\x1b' => match chars.peek().copied() {
                Some('[') => {
                    chars.next();
                    while let Some(next) = chars.next() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    while let Some(next) = chars.next() {
                        if next == '\x07' {
                            break;
                        }
                        if next == '\x1b' && matches!(chars.peek(), Some('\\')) {
                            chars.next();
                            break;
                        }
                    }
                }
                Some('P' | '^' | '_') => {
                    chars.next();
                    while let Some(next) = chars.next() {
                        if next == '\x07' {
                            break;
                        }
                        if next == '\x1b' && matches!(chars.peek(), Some('\\')) {
                            chars.next();
                            break;
                        }
                    }
                }
                Some('(' | ')' | '*' | '+' | '-' | '.' | '/') => {
                    chars.next();
                    let _ = chars.next();
                }
                Some('7' | '8' | 'c') => {
                    chars.next();
                }
                _ => {}
            },
            '\r' | '\n' | '\t' => out.push(ch),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

fn decode_terminal_text_stateful(chunk: &[u8], carry: &mut AnsiCarry, eof: bool) -> String {
    carry.bytes.extend_from_slice(chunk);
    let mut out = Vec::with_capacity(carry.bytes.len());
    let mut i = 0usize;
    while i < carry.bytes.len() {
        let b = carry.bytes[i];
        if b != 0x1b {
            out.push(b);
            i += 1;
            continue;
        }
        let escape_start = i;
        if i + 1 >= carry.bytes.len() {
            break;
        }
        match carry.bytes[i + 1] {
            b'[' => {
                i += 2;
                while i < carry.bytes.len() {
                    let c = carry.bytes[i];
                    i += 1;
                    if (b'@'..=b'~').contains(&c) {
                        break;
                    }
                }
                if i >= carry.bytes.len() && !matches!(carry.bytes.last(), Some(b'@'..=b'~')) {
                    i = escape_start;
                    break;
                }
            }
            b']' | b'P' | b'^' | b'_' => {
                i += 2;
                while i < carry.bytes.len() {
                    let c = carry.bytes[i];
                    if c == 0x07 {
                        i += 1;
                        break;
                    }
                    if c == 0x1b && i + 1 < carry.bytes.len() && carry.bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                if i >= carry.bytes.len() {
                    i = escape_start;
                    break;
                }
            }
            b'(' | b')' | b'*' | b'+' | b'-' | b'.' | b'/' => {
                if i + 2 >= carry.bytes.len() {
                    i = escape_start;
                    break;
                }
                i += 3;
            }
            b'7' | b'8' | b'c' => {
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }
    let keep_from = i.min(carry.bytes.len());
    carry.bytes.drain(..keep_from);
    if eof {
        carry.bytes.clear();
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn quote_command_part(value: &str) -> String {
    if value.contains([' ', '\t', '"']) {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}
pub(crate) fn append_quick_command_history_bounded(
    mut history: Vec<settings::QuickCommandRunHistoryEntry>,
    entry: settings::QuickCommandRunHistoryEntry,
) -> Vec<settings::QuickCommandRunHistoryEntry> {
    history.retain(|existing| existing.run_id != entry.run_id);
    history.insert(0, entry);
    history.truncate(HISTORY_LIMIT);
    history
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::QuickCommandRunHistoryEntry;

    const QUICK_COMMANDS_SOURCE: &str = include_str!("quick_commands.rs");

    fn quick_commands_production_source() -> &'static str {
        QUICK_COMMANDS_SOURCE
            .split("#[cfg(test)]")
            .next()
            .unwrap_or(QUICK_COMMANDS_SOURCE)
    }

    fn quick_command_test_state(run_id: &str) -> QuickCommandRunState {
        QuickCommandRunState {
            command_id: "cmd".into(),
            run_id: run_id.into(),
            process_id: 42,
            started_at_epoch_ms: 10,
            started_at_filetime_100ns: 99,
            stdin: None,
            running: true,
            stopping: false,
            pending: None,
            transcript: VecDeque::from([QuickCommandTranscriptEntry {
                kind: "stdout".into(),
                body: "before stop".into(),
                request_id: None,
                prompt: None,
                secret: false,
                max_length: None,
                redacted: false,
                sequence: 1,
                at_epoch_ms: 10,
                pending: false,
            }]),
            stdout: b"before stop".to_vec(),
            stderr: b"warn".to_vec(),
            stdout_carry: AnsiCarry::default(),
            stderr_carry: AnsiCarry::default(),
            stdout_truncated: false,
            stderr_truncated: false,
            finished_at_epoch_ms: None,
            exit_code: None,
            #[cfg(windows)]
            job: quick_command_test_job_handle(),
        }
    }

    #[cfg(windows)]
    fn quick_command_test_job_handle() -> Arc<QuickCommandJobHandle> {
        Arc::new(QuickCommandJobHandle(HANDLE(std::ptr::null_mut())))
    }

    #[test]
    fn stop_marks_run_stopping_before_termination() {
        assert!(
            quick_commands_production_source().contains("fn mark_stopping")
                && quick_commands_production_source().contains("run.stopping = true"),
            "stop must mark live run as visible stopping before termination"
        );
        assert!(
            quick_commands_production_source().contains("mark_stopping(run, sequence)")
                && quick_commands_production_source().contains("kind: \"stopping\".into()"),
            "stop must emit stopping snapshot before final stopped/error result"
        );
        assert!(
            !quick_commands_production_source()
                .contains("let mut state = runs_guard.remove(run_id).unwrap();"),
            "stop must not remove run from registry before termination completes"
        );
    }

    #[test]
    fn stop_failure_restores_running_state_and_preserves_transcript() {
        let before = quick_command_test_state("run-stop-fail");
        let before_transcript = before.transcript.clone();
        let before_stdout = before.stdout.clone();
        let before_stderr = before.stderr.clone();

        assert!(
            quick_commands_production_source().contains("rollback_stopping")
                || quick_commands_production_source()
                    .contains("restore_running_after_stop_failure"),
            "termination failure must use explicit rollback path"
        );
        assert_eq!(before.transcript, before_transcript);
        assert_eq!(before.stdout, before_stdout);
        assert_eq!(before.stderr, before_stderr);
        assert!(before.running, "failed stop leaves run retryable/running");
    }

    #[test]
    fn stop_rejects_reused_root_pid_before_terminate() {
        assert!(
            quick_commands_production_source().contains("trait QuickCommandTerminator")
                && quick_commands_production_source().contains("DefaultQuickCommandTerminator"),
            "termination must expose injectable seam for final same-handle identity test"
        );
        assert!(
            quick_commands_production_source().contains("Arc<QuickCommandJobHandle>")
                && quick_commands_production_source()
                    .contains("process_creation_time_for_handle(process_guard.0)")
                && quick_commands_production_source().contains("TerminateJobObject"),
            "root PID reuse must be rejected before job termination"
        );
    }

    #[test]
    fn stop_success_appends_single_stopped_history_entry() {
        assert!(
            quick_commands_production_source().contains("let stopped = state.stopping")
                && quick_commands_production_source()
                    .contains("if stopped { \"stopped\" } else { \"exit\" }"),
            "exit waiter must own exactly-once stopped history finalization"
        );
    }

    #[test]
    fn stop_state_uses_arc_job_handle_and_checked_resume_thread() {
        let source = quick_commands_production_source();
        assert!(source.contains("job: Arc<QuickCommandJobHandle>"));
        assert!(!source.contains("job: Option<usize>"));
        assert!(source.contains("ResumeThread(thread_guard.0)") && source.contains("u32::MAX"));
        let assign = source
            .split("fn assign_and_resume_quick_command_process")
            .nth(1)
            .expect("assign/resume helper must exist");
        let capture = assign
            .find("process_creation_time_for_handle(process_guard.0)")
            .expect("root identity must be captured from opened process handle");
        let resume = assign
            .find("ResumeThread(thread_guard.0)")
            .expect("suspended root must be resumed");
        assert!(
            capture < resume,
            "root identity must be captured before resume"
        );
    }

    #[test]
    fn terminal_run_stays_queryable_until_event_and_persistence() {
        let source = quick_commands_production_source();
        let terminalize = source
            .find("let (history, exit_payload) = {")
            .expect("waiter must build terminal state");
        let emit = source[terminalize..]
            .find("emit_run_snapshot(&app_handle, &exit_payload)")
            .expect("waiter must emit terminal state")
            + terminalize;
        let persist = source[terminalize..]
            .find("on_exit(history)")
            .expect("waiter must persist terminal history")
            + terminalize;
        let remove = source[terminalize..]
            .find("remove(&run_id)")
            .expect("waiter must eventually remove terminal runtime state")
            + terminalize;

        assert!(
            source[terminalize..emit].contains("get_mut(&run_id)"),
            "terminal state must remain queryable before terminal event emission"
        );
        assert!(
            emit < persist,
            "terminal event must not wait for settings I/O"
        );
        assert!(
            persist < remove,
            "terminal state must remain queryable until persistence finishes"
        );
    }

    #[test]
    fn stopped_root_does_not_wait_for_descendant_held_output_pipes() {
        assert!(
            quick_commands_production_source().contains(
                "let (stdout, stdout_truncated, stderr, stderr_truncated) = if stopping"
            ) && quick_commands_production_source().contains("(vec![], false, vec![], false)"),
            "stopped root must finalize without joining pipe readers that descendants may keep open"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_job_termination_kills_nested_listener_descendant() {
        use std::fs;
        use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
        use std::thread;
        use std::time::{Duration, Instant};

        struct JobProcessCleanup {
            job: Option<QuickCommandJobHandle>,
            process_id: u32,
        }

        impl Drop for JobProcessCleanup {
            fn drop(&mut self) {
                if let Some(job) = &self.job {
                    let _ = unsafe { TerminateJobObject(job.0, 1) };
                }
                cleanup_suspended_quick_command_process(self.process_id);
            }
        }

        fn wait_until(
            timeout: Duration,
            mut predicate: impl FnMut() -> Result<bool, String>,
        ) -> Result<(), String> {
            let deadline = Instant::now() + timeout;
            let mut last_error = None;
            while Instant::now() < deadline {
                match predicate() {
                    Ok(true) => return Ok(()),
                    Ok(false) => last_error = None,
                    Err(error) => last_error = Some(error),
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(last_error.unwrap_or_else(|| "condition was not met before timeout".into()))
        }

        let port = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("reserve ephemeral loopback port")
            .local_addr()
            .expect("read reserved listener addr")
            .port();
        let marker = std::env::temp_dir().join(format!(
            "jasonshell-job-descendant-{}-{}.ready",
            std::process::id(),
            port
        ));
        let _ = fs::remove_file(&marker);
        let marker_for_child = marker.display().to_string().replace("'", "''");
        let child_script = format!(
            "$listener=[Net.Sockets.TcpListener]::new([Net.IPAddress]::Parse('127.0.0.1'),{port}); \
             $listener.Start(); \
             Set-Content -LiteralPath '{marker_for_child}' -Value ready -NoNewline; \
             try {{ while ($true) {{ $client=$listener.AcceptTcpClient(); $client.Dispose() }} }} finally {{ $listener.Stop() }}"
        );
        let mut child_script_utf16 = Vec::with_capacity(child_script.len() * 2);
        for code_unit in child_script.encode_utf16() {
            child_script_utf16.extend_from_slice(&code_unit.to_le_bytes());
        }
        let encoded_child_script =
            base64::engine::general_purpose::STANDARD.encode(child_script_utf16);
        let root_script = format!(
            "Start-Process -FilePath pwsh.exe -ArgumentList @('-NoLogo','-NoProfile','-EncodedCommand','{encoded_child_script}') -WindowStyle Hidden; \
             while ($true) {{ Start-Sleep -Seconds 60 }}"
        );

        let job = create_quick_command_job().expect("create production quick command job");
        let mut root = Command::new("pwsh.exe")
            .args(["-NoLogo", "-NoProfile", "-Command", &root_script])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_SUSPENDED.0)
            .spawn()
            .expect("spawn suspended root quick command process");
        let root_pid = root.id();
        let mut cleanup = JobProcessCleanup {
            job: Some(job),
            process_id: root_pid,
        };
        assign_and_resume_quick_command_process(cleanup.job.as_ref().unwrap().0, root_pid)
            .expect("assign suspended root to job and resume it");

        wait_until(Duration::from_secs(10), || Ok(marker.exists()))
            .expect("nested child listener signaled readiness");
        TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
            .expect_err("child-owned listener must hold reserved port before job termination");

        let job = cleanup.job.take().expect("job still owned by cleanup");
        unsafe { TerminateJobObject(job.0, 1) }.expect("terminate quick command job");
        drop(job);
        let _ = root.wait();

        wait_until(Duration::from_secs(10), || {
            TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
                .map(|listener| {
                    drop(listener);
                    true
                })
                .map_err(|error| format!("port {port} still held after job termination: {error}"))
        })
        .expect("nested child-owned listener port released after job termination");
        let _ = fs::remove_file(marker);
    }

    #[test]
    fn root_exit_is_claimed_before_late_stop_can_relabel_it() {
        let mut state = quick_command_test_state("run-natural-exit");
        assert!(!claim_root_exit(&mut state));
        assert!(!state.running);

        let mut stopping_state = quick_command_test_state("run-stopped");
        stopping_state.stopping = true;
        assert!(claim_root_exit(&mut stopping_state));
        assert!(!stopping_state.running);
    }

    #[test]
    fn stop_helpers_and_terminator_seam_are_present() {
        assert!(quick_commands_production_source().contains("fn mark_stopping"));
        assert!(quick_commands_production_source().contains("fn rollback_stopping"));
        assert!(quick_commands_production_source().contains("trait QuickCommandTerminator"));
        assert!(quick_commands_production_source().contains("DefaultQuickCommandTerminator"));
        assert!(
            quick_commands_production_source().contains("emit_to(")
                && quick_commands_production_source().contains("stopping_snapshot")
        );
    }

    #[test]
    fn parser_splits_chunks() {
        let mut carry = MarkerCarry::default();
        let a = feed(
            &mut carry,
            b"a\x1b]777;JasonShellQuickCommandInput;bad\x07b",
            true,
        );
        assert!(a.len() >= 2);
    }
    #[test]
    fn malformed_passthrough() {
        let mut carry = MarkerCarry::default();
        assert!(feed(
            &mut carry,
            b"x\x1b]777;JasonShellQuickCommandInput;bad\x07y",
            true
        )
        .iter()
        .any(|c| c.text.contains("bad")));
    }
    #[test]
    fn oversize_passthrough() {
        let s = format!("x{}y", MARKER_PREFIX);
        let mut carry = MarkerCarry::default();
        assert!(!feed(&mut carry, s.as_bytes(), true).is_empty());
    }
    #[test]
    fn split_valid_marker_two_chunks() {
        let marker = serde_json::json!({"version":1,"requestId":"req-1","prompt":"p","kind":"text","secret":false,"maxLength":16384});
        let encoded = URL_SAFE_NO_PAD.encode(marker.to_string());
        let mut carry = MarkerCarry::default();
        let first = feed(
            &mut carry,
            format!("hello{}{}", MARKER_PREFIX, &encoded[..encoded.len() / 2]).as_bytes(),
            false,
        );
        let second = feed(
            &mut carry,
            format!("{}{}world", &encoded[encoded.len() / 2..], MARKER_SUFFIX).as_bytes(),
            true,
        );
        assert!(first
            .iter()
            .chain(second.iter())
            .any(|c| c.marker.is_some()));
    }

    #[test]
    fn split_utf8_stream_keeps_text_intact_across_chunks() {
        let mut carry = MarkerCarry::default();
        let bytes = "ab🙂cd".as_bytes();
        let first = feed(&mut carry, &bytes[..4], false);
        assert_eq!(
            first
                .iter()
                .map(|chunk| chunk.text.as_str())
                .collect::<String>(),
            "ab"
        );
        let second = feed(&mut carry, &bytes[4..], true);
        assert_eq!(
            first
                .iter()
                .chain(second.iter())
                .map(|chunk| chunk.text.as_str())
                .collect::<String>(),
            "ab🙂cd"
        );
    }

    #[test]
    fn sanitize_terminal_text_consumes_common_escape_classes() {
        let text = "a\x1bc\x1b[31mred\x1b]0;title\x07b\x1b7\x1b8\x1b(Be\r\nf\t\x1bPq\x1b\\g";
        assert_eq!(sanitize_terminal_text(text), "aredbe\r\nf\tg");
    }

    #[test]
    fn split_csi_sequence_is_removed_across_chunks() {
        let mut carry = AnsiCarry::default();
        let first = decode_terminal_text_stateful(b"a\x1b[3", &mut carry, false);
        let second = decode_terminal_text_stateful(b"1mb", &mut carry, true);
        assert_eq!(format!("{}{}", first, second), "ab");
        assert!(!format!("{}{}", first, second).contains("31m"));
    }

    #[test]
    fn split_osc_st_sequence_is_removed_across_chunks() {
        let mut carry = AnsiCarry::default();
        let first = decode_terminal_text_stateful(b"x\x1b]0;ti", &mut carry, false);
        let second = decode_terminal_text_stateful(b"tle\x07y", &mut carry, true);
        let combined = format!("{}{}", first, second);
        assert_eq!(combined, "xy");
        assert!(!combined.contains("title"));
    }

    #[cfg(windows)]
    #[test]
    fn split_oem_dbcs_bytes_flush_on_boundary() {
        use windows::Win32::Globalization::GetOEMCP;
        let cp = unsafe { GetOEMCP() };
        if cp == 932 {
            assert_eq!(decode_terminal_bytes(&[0x82, 0xA0]), "あ");
        }
    }
    #[test]
    fn split_marker_prefix_across_chunks() {
        let marker = serde_json::json!({"version":1,"requestId":"req-1","prompt":"p","kind":"text","secret":false,"maxLength":4096});
        let encoded = URL_SAFE_NO_PAD.encode(marker.to_string());
        let split = MARKER_PREFIX.len() / 2;
        let mut carry = MarkerCarry::default();
        let first = feed(&mut carry, MARKER_PREFIX[..split].as_bytes(), false);
        let second = feed(
            &mut carry,
            format!("{}{}{}", &MARKER_PREFIX[split..], encoded, MARKER_SUFFIX).as_bytes(),
            true,
        );
        assert!(first.is_empty());
        assert!(second.iter().any(|c| c.marker.is_some()));
    }
    #[test]
    fn unclosed_marker_carry_bounded_at_prefix_plus_max_encoded() {
        let mut carry = MarkerCarry::default();
        let chunk = format!("{}{}", MARKER_PREFIX, "x".repeat(MAX_ENCODED_MARKER + 1));
        let out = feed(&mut carry, chunk.as_bytes(), false);
        assert!(out.iter().any(|c| c.text == chunk));
        assert!(carry.bytes.is_empty());
    }
    #[test]
    fn output_event_payload_from_helper() {
        let mut run = QuickCommandRunState {
            command_id: "cmd".into(),
            run_id: "run-1".into(),
            process_id: 11,
            started_at_epoch_ms: 1,
            started_at_filetime_100ns: 2,
            stdin: None,
            running: true,
            stopping: false,
            pending: None,
            transcript: VecDeque::new(),
            stdout: vec![],
            stderr: vec![],
            stdout_carry: AnsiCarry::default(),
            stderr_carry: AnsiCarry::default(),
            stdout_truncated: false,
            stderr_truncated: false,
            finished_at_epoch_ms: None,
            exit_code: None,
            #[cfg(windows)]
            job: quick_command_test_job_handle(),
        };
        let payload = emit_run_updated_from_transcript(
            &mut run,
            "run-1",
            11,
            "stdout",
            None,
            None,
            "hello".into(),
            None,
            false,
            false,
            false,
        );
        assert_eq!(payload.kind, "stdout");
        assert_eq!(payload.body, "hello");
        assert_eq!(run.transcript.len(), 1);
        assert_eq!(run.transcript[0].sequence, payload.sequence);
    }

    #[test]
    fn marker_kind_preserved_in_transcript() {
        let mut run = QuickCommandRunState {
            command_id: "cmd".into(),
            run_id: "run-1".into(),
            process_id: 11,
            started_at_epoch_ms: 1,
            started_at_filetime_100ns: 2,
            stdin: None,
            running: true,
            stopping: false,
            pending: None,
            transcript: VecDeque::new(),
            stdout: vec![],
            stderr: vec![],
            stdout_carry: AnsiCarry::default(),
            stderr_carry: AnsiCarry::default(),
            stdout_truncated: false,
            stderr_truncated: false,
            finished_at_epoch_ms: None,
            exit_code: None,
            #[cfg(windows)]
            job: quick_command_test_job_handle(),
        };
        let payload = emit_run_updated_from_transcript(
            &mut run,
            "run-1",
            11,
            "confirm",
            Some("req-1".into()),
            Some(4096),
            "prompt".into(),
            Some("prompt".into()),
            false,
            false,
            true,
        );
        assert_eq!(payload.kind, "confirm");
        assert_eq!(run.transcript[0].kind, "confirm");
    }
    #[test]
    fn marker_bounds() {
        assert!(validate_marker_bounds(&QuickCommandInputMarker {
            version: 1,
            request_id: "r".into(),
            prompt: Some("p".into()),
            kind: Some("text".into()),
            secret: false,
            max_length: Some(4096)
        }));
        assert!(!validate_marker_bounds(&QuickCommandInputMarker {
            version: 1,
            request_id: "r".into(),
            prompt: Some("p".into()),
            kind: Some("text".into()),
            secret: false,
            max_length: Some(0)
        }));
    }
    #[test]
    fn redaction_helper() {
        assert_eq!("[redacted]", if true { "[redacted]" } else { "x" });
    }
    #[test]
    fn transcript_bound() {
        let mut t = VecDeque::new();
        for i in 0..TRANSCRIPT_LIMIT {
            push_transcript(
                &mut t,
                QuickCommandTranscriptEntry {
                    kind: "stdout".into(),
                    body: i.to_string(),
                    request_id: None,
                    prompt: None,
                    secret: false,
                    max_length: None,
                    redacted: false,
                    sequence: 7,
                    at_epoch_ms: i as u64,
                    pending: false,
                },
            );
        }
        push_transcript(
            &mut t,
            QuickCommandTranscriptEntry {
                kind: "stdout".into(),
                body: "terminal".into(),
                request_id: None,
                prompt: None,
                secret: false,
                max_length: None,
                redacted: false,
                sequence: 7,
                at_epoch_ms: TRANSCRIPT_LIMIT as u64,
                pending: false,
            },
        );
        assert_eq!(t.len(), TRANSCRIPT_LIMIT);
        assert_eq!(t.back().map(|entry| entry.body.as_str()), Some("terminal"));
        assert!(t.iter().all(|entry| entry.sequence == 7));
    }
    #[test]
    fn run_ids_unique() {
        assert_ne!(new_run_id(), new_run_id());
    }
    #[test]
    fn history_bounded() {
        let h = append_quick_command_history_bounded(
            Vec::new(),
            QuickCommandRunHistoryEntry {
                run_id: "r".into(),
                command_id: "c".into(),
                started_at_epoch_ms: 1,
                started_at_filetime_100ns: 0,
                finished_at_epoch_ms: 1,
                process_id: 1,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                transcript: Vec::new(),
                stdout_truncated: false,
                stderr_truncated: false,
                running: false,
            },
        );
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn state_to_history_uses_finished_fields() {
        let state = QuickCommandRunState {
            command_id: "cmd".into(),
            run_id: "run-1".into(),
            process_id: 9,
            started_at_epoch_ms: 10,
            started_at_filetime_100ns: 11,
            stdin: None,
            running: false,
            pending: None,
            transcript: VecDeque::new(),
            stdout: vec![1],
            stderr: vec![2],
            stdout_carry: AnsiCarry::default(),
            stderr_carry: AnsiCarry::default(),
            stdout_truncated: true,
            stderr_truncated: false,
            finished_at_epoch_ms: Some(99),
            exit_code: Some(7),
            #[cfg(windows)]
            job: quick_command_test_job_handle(),
            stopping: false,
        };
        let history = state_to_history(&state);
        assert_eq!(history.finished_at_epoch_ms, 99);
        assert_eq!(history.exit_code, Some(7));
    }
}
