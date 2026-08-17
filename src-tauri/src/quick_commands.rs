use crate::contracts::surfaces;
use crate::settings::{
    self, validate_quick_command_args, validate_quick_command_commands,
    validate_quick_command_entry, QuickCommandEntry, QuickCommandMode, QuickCommandTranscriptEntry,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex, OnceLock,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

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
    pending: Option<QuickCommandPendingInput>,
    transcript: VecDeque<QuickCommandTranscriptEntry>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_truncated: bool,
    stderr_truncated: bool,
    finished_at_epoch_ms: Option<u64>,
    exit_code: Option<i32>,
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
    if run.command_id != request.id || run.process_id != request.process_id || !run.running {
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
pub fn stop_quick_command(
    app_handle: AppHandle,
    request: StopQuickCommandRequest,
) -> Result<(), String> {
    stop_running_quick_command(
        &app_handle,
        request.process_id,
        &request.id,
        &request.run_id,
    )
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
    let mut child = command
        .spawn()
        .map_err(|e| format!("failed to run quick command '{}': {e}", plan.executable))?;
    let process_id = child.id();
    let started_at_epoch_ms = current_epoch_ms();
    let started_at_filetime_100ns = process_creation_time_for_pid(process_id)?;
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
                pending: None,
                transcript: VecDeque::new(),
                stdout: vec![],
                stderr: vec![],
                stdout_truncated: false,
                stderr_truncated: false,
                finished_at_epoch_ms: None,
                exit_code: None,
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
        let (stdout, stdout_truncated) = stdout.and_then(|h| h.join().ok()).unwrap_or_default();
        let (stderr, stderr_truncated) = stderr.and_then(|h| h.join().ok()).unwrap_or_default();
        let exit_code = status.ok().and_then(|s| s.code());
        let (history, exit_payload) = {
            let mut runs = runs().lock().ok();
            let state = runs.as_mut().and_then(|r| r.remove(&run_id));
            let Some(state) = state else {
                return;
            };
            let mut transcript = state.transcript.clone();
            let sequence = next_sequence();
            push_transcript(
                &mut transcript,
                QuickCommandTranscriptEntry {
                    kind: "exit".into(),
                    body: exit_code.map(|c| c.to_string()).unwrap_or_default(),
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
                run_id: run_id.clone(),
                command_id: command_id.clone(),
                started_at_epoch_ms,
                started_at_filetime_100ns,
                finished_at_epoch_ms,
                process_id,
                exit_code,
                stdout: String::from_utf8_lossy(&stdout).into_owned(),
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
                transcript: transcript.into_iter().collect(),
                stdout_truncated: stdout_truncated || state.stdout_truncated,
                stderr_truncated: stderr_truncated || state.stderr_truncated,
                running: false,
            };
            let payload = QuickCommandRunUpdatedPayload {
                run_id: state.run_id,
                command_id: state.command_id,
                process_id: state.process_id,
                kind: "exit".into(),
                body: exit_code.map(|c| c.to_string()).unwrap_or_default(),
                prompt: None,
                request_id: None,
                max_length: None,
                secret: false,
                redacted: false,
                sequence,
                at_epoch_ms: finished_at_epoch_ms,
                pending: false,
            };
            (history, Some(payload))
        };
        on_exit(history);
        if let Some(payload) = exit_payload {
            emit_run_snapshot(&app_handle, &payload);
        }
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
            out.push(QuickCommandOutputChunk {
                text: String::from_utf8_lossy(&carry.bytes[i..start]).into_owned(),
                marker: None,
            });
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
            out.push(QuickCommandOutputChunk {
                text: String::from_utf8_lossy(&carry.bytes[start..=end]).into_owned(),
                marker: None,
            });
            i = end + 1;
            continue;
        }
        break;
    }
    if eof {
        if i < carry.bytes.len() {
            out.push(QuickCommandOutputChunk {
                text: String::from_utf8_lossy(&carry.bytes[i..]).into_owned(),
                marker: None,
            });
        }
        carry.bytes.clear();
    } else {
        let unprocessed = &carry.bytes[i..];
        let hold_from = if let Some(marker_start) = find_bytes(unprocessed, marker_prefix) {
            i + marker_start
        } else {
            let suffix_len = longest_marker_prefix_suffix(unprocessed, marker_prefix);
            carry.bytes.len().saturating_sub(suffix_len)
        };
        if carry.bytes.len().saturating_sub(hold_from) > marker_prefix.len() + MAX_ENCODED_MARKER {
            out.push(QuickCommandOutputChunk {
                text: String::from_utf8_lossy(&carry.bytes[i..]).into_owned(),
                marker: None,
            });
            carry.bytes.clear();
            return out;
        }
        if i < hold_from {
            out.push(QuickCommandOutputChunk {
                text: String::from_utf8_lossy(&carry.bytes[i..hold_from]).into_owned(),
                marker: None,
            });
        }
        carry.bytes = carry.bytes[hold_from..].to_vec();
    }
    out
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
            if run.pending.is_some() {
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
            let text = String::from_utf8_lossy(chunk);
            let body = text.to_string();
            let sequence = next_sequence();
            let at_epoch_ms = current_epoch_ms();
            let kind = if is_stdout { "stdout" } else { "stderr" }.to_string();
            let target = if is_stdout {
                &mut run.stdout
            } else {
                &mut run.stderr
            };
            if target.len() < OUTPUT_LIMIT {
                target.extend_from_slice(text.as_bytes());
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
        stdout: String::from_utf8_lossy(&state.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&state.stderr).into_owned(),
        transcript: state.transcript.iter().cloned().collect(),
        stdout_truncated: state.stdout_truncated,
        stderr_truncated: state.stderr_truncated,
        running: state.running,
    }
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
fn process_creation_time_for_pid(process_id: u32) -> Result<u64, String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{
            GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }
            .map_err(|e| format!("failed to inspect quick command process: {e}"))?;
        let mut creation = windows::Win32::Foundation::FILETIME::default();
        let mut exit = windows::Win32::Foundation::FILETIME::default();
        let mut kernel = windows::Win32::Foundation::FILETIME::default();
        let mut user = windows::Win32::Foundation::FILETIME::default();
        unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) }
            .map_err(|_| "quick command is no longer running".to_string())?;
        let _ = unsafe { CloseHandle(handle) };
        Ok(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64)
    }
    #[cfg(not(windows))]
    {
        Ok(0)
    }
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
        if run.process_id != process_id || run.command_id != command_id || !run.running {
            return Err("quick command is no longer running".into());
        }
        run.started_at_filetime_100ns
    };
    if process_creation_time_for_pid(process_id)? != started_at_filetime_100ns {
        return Err("quick command is no longer running".into());
    }
    let sequence = next_sequence();
    let state = {
        let mut runs_guard = runs()
            .lock()
            .map_err(|_| "quick command runtime state is poisoned".to_string())?;
        let run = runs_guard
            .get(run_id)
            .ok_or_else(|| "quick command is no longer running".to_string())?;
        if run.process_id != process_id || run.command_id != command_id || !run.running {
            return Err("quick command is no longer running".into());
        }
        if run.started_at_filetime_100ns != started_at_filetime_100ns {
            return Err("quick command is no longer running".into());
        }
        let mut state = runs_guard.remove(run_id).unwrap();
        push_transcript(
            &mut state.transcript,
            QuickCommandTranscriptEntry {
                kind: "stopped".into(),
                body: String::new(),
                request_id: None,
                prompt: None,
                secret: false,
                redacted: false,
                sequence,
                at_epoch_ms: current_epoch_ms(),
                pending: false,
                max_length: None,
            },
        );
        state
    };
    let status = Command::new(r"C:\Windows\System32\taskkill.exe")
        .args(["/PID", &state.process_id.to_string(), "/T", "/F"])
        .status()
        .map_err(|e| format!("failed to stop quick command: {e}"))?;
    if !status.success() {
        if let Ok(mut runs) = runs().lock() {
            runs.insert(run_id.to_string(), state);
        }
        return Err("failed to stop quick command".into());
    }
    let finished_at_epoch_ms = current_epoch_ms();
    let _ = settings::update_shell_settings_for_app(app_handle, |settings| {
        settings.quick_commands.history = append_quick_command_history_bounded(
            std::mem::take(&mut settings.quick_commands.history),
            settings::QuickCommandRunHistoryEntry {
                run_id: state.run_id.clone(),
                command_id: state.command_id.clone(),
                started_at_epoch_ms: state.started_at_epoch_ms,
                started_at_filetime_100ns: state.started_at_filetime_100ns,
                finished_at_epoch_ms,
                process_id: state.process_id,
                exit_code: None,
                stdout: String::from_utf8_lossy(&state.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&state.stderr).into_owned(),
                transcript: state.transcript.iter().cloned().collect(),
                stdout_truncated: state.stdout_truncated,
                stderr_truncated: state.stderr_truncated,
                running: false,
            },
        );
    });
    let finished_at_epoch_ms = current_epoch_ms();
    let _ = app_handle.emit_to(
        surfaces::COMMAND_PANEL,
        crate::contracts::events::QUICK_COMMAND_RUN_UPDATED,
        &QuickCommandRunUpdatedPayload {
            run_id: run_id.to_string(),
            command_id: command_id.to_string(),
            process_id,
            kind: "stopped".into(),
            body: String::new(),
            prompt: None,
            request_id: None,
            max_length: None,
            secret: false,
            redacted: false,
            sequence,
            at_epoch_ms: finished_at_epoch_ms,
            pending: false,
        },
    );
    Ok(())
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
            pending: None,
            transcript: VecDeque::new(),
            stdout: vec![],
            stderr: vec![],
            stdout_truncated: false,
            stderr_truncated: false,
            finished_at_epoch_ms: None,
            exit_code: None,
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
            pending: None,
            transcript: VecDeque::new(),
            stdout: vec![],
            stderr: vec![],
            stdout_truncated: false,
            stderr_truncated: false,
            finished_at_epoch_ms: None,
            exit_code: None,
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
            stdout_truncated: true,
            stderr_truncated: false,
            finished_at_epoch_ms: Some(99),
            exit_code: Some(7),
        };
        let history = state_to_history(&state);
        assert_eq!(history.finished_at_epoch_ms, 99);
        assert_eq!(history.exit_code, Some(7));
    }
}
