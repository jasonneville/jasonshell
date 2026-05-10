use crate::settings::{self, TerminalProfile};
use crate::shell_windows;
use crate::stack_popup::models::StackPopupRuntimeState;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use portable_pty::{native_pty_system, Child as PtyChild, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};
#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::GetShortPathNameW;

pub(crate) const MAX_STACK_TERMINAL_SESSIONS: usize = 4;
pub(crate) const MAX_STACK_TERMINAL_SESSION_ID_LEN: usize = 48;
const MAX_STACK_TERMINAL_WRITE_BYTES: usize = 16 * 1024;
const STACK_TERMINAL_SESSION_PREFIX: &str = "stack-term-";

static STACK_TERMINAL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTerminalStartRequest {
    pub folder_path: String,
    #[serde(default)]
    pub profile: Option<TerminalProfile>,
    #[serde(default)]
    pub target_label: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTerminalWriteRequest {
    pub session_id: String,
    pub input: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTerminalResizeRequest {
    pub session_id: String,
    pub cols: u16,
    pub rows: u16,
    pub pixel_width: Option<u16>,
    pub pixel_height: Option<u16>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTerminalStopRequest {
    pub session_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTerminalSessionSnapshot {
    pub session_id: String,
    pub profile: TerminalProfile,
    pub cwd: String,
    pub running: bool,
    pub cols: u16,
    pub rows: u16,
    pub pixel_width: Option<u16>,
    pub pixel_height: Option<u16>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTerminalPollResult {
    pub session_id: String,
    pub cwd: String,
    pub running: bool,
    pub chunks: Vec<StackTerminalOutputChunk>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTerminalReadResult {
    pub session_id: String,
    pub cwd: String,
    pub output: String,
    pub chunks: Vec<StackTerminalOutputChunk>,
    pub exited: bool,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTerminalOutputChunk {
    pub session_id: String,
    pub stream: StackTerminalOutputStream,
    pub text: String,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StackTerminalOutputStream {
    Stdout,
    #[allow(dead_code)]
    Stderr,
    System,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTerminalCwdUpdate {
    pub session_id: String,
    pub cwd: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TerminalProcessPlan {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub candidates: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StackTerminalSize {
    cols: u16,
    rows: u16,
    pixel_width: Option<u16>,
    pixel_height: Option<u16>,
}

pub(crate) struct StackTerminalRegistry {
    sessions: HashMap<String, StackTerminalSession>,
    stop_requested: HashSet<String>,
}

impl Default for StackTerminalRegistry {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            stop_requested: HashSet::new(),
        }
    }
}

impl StackTerminalRegistry {
    pub(crate) fn can_start_session(&self) -> bool {
        self.sessions.len() < MAX_STACK_TERMINAL_SESSIONS
    }

    fn insert(&mut self, session: StackTerminalSession) {
        self.stop_requested.remove(&session.id);
        self.sessions.insert(session.id.clone(), session);
    }

    fn request_stop(&mut self, session_id: &str) -> Result<Option<StackTerminalSession>, String> {
        validate_stack_terminal_session_id(session_id)?;
        self.stop_requested.insert(session_id.to_string());
        Ok(self.sessions.remove(session_id))
    }

    #[cfg(test)]
    fn should_drop_for_stop(&mut self, session_id: &str) -> bool {
        self.stop_requested.remove(session_id)
    }

    fn session_mut(&mut self, session_id: &str) -> Result<&mut StackTerminalSession, String> {
        validate_stack_terminal_session_id(session_id)?;
        self.sessions
            .get_mut(session_id)
            .ok_or_else(|| "Terminal session not found".to_string())
    }

    fn remove(&mut self, session_id: &str) -> Result<StackTerminalSession, String> {
        validate_stack_terminal_session_id(session_id)?;
        self.sessions
            .remove(session_id)
            .ok_or_else(|| "Terminal session not found".to_string())
    }

    #[cfg(test)]
    pub(crate) fn insert_test_session(&mut self, id: String, cwd: PathBuf) {
        self.sessions.insert(
            id.clone(),
            StackTerminalSession {
                id,
                profile: TerminalProfile::PowerShell,
                cwd,
                child: test_child(),
                master: None,
                writer: None,
                input_buffer: String::new(),
                output_rx: mpsc::sync_channel(1024).1,
                target_label: shell_windows::STACK_POPUP_LABEL.to_string(),
                next_sequence: 1,
                running: true,
                size: StackTerminalSize {
                    cols: 120,
                    rows: 30,
                    pixel_width: None,
                    pixel_height: None,
                },
                test_size: None,
            },
        );
    }

    #[cfg(test)]
    fn resize_test_session(
        &mut self,
        session_id: &str,
        cols: u16,
        rows: u16,
        pixel_width: Option<u16>,
        pixel_height: Option<u16>,
    ) -> Result<(), String> {
        validate_stack_terminal_resize_request(session_id, cols, rows, pixel_width, pixel_height)?;
        let session = self.session_mut(session_id)?;
        session.test_size = Some(StackTerminalSize {
            cols,
            rows,
            pixel_width,
            pixel_height,
        });
        Ok(())
    }

    #[cfg(test)]
    fn test_session_size(&self, session_id: &str) -> Option<StackTerminalSize> {
        self.sessions
            .get(session_id)
            .and_then(|session| session.test_size.clone())
    }

    #[cfg(test)]
    fn begin_test_operation(
        &mut self,
        session_id: &str,
    ) -> Result<StackTerminalTestOperation, String> {
        validate_stack_terminal_session_id(session_id)?;
        if self.sessions.contains_key(session_id) {
            Ok(StackTerminalTestOperation {})
        } else {
            Err("Terminal session not found".to_string())
        }
    }
}

#[cfg(test)]
struct StackTerminalTestOperation {}

struct StackTerminalSession {
    id: String,
    profile: TerminalProfile,
    cwd: PathBuf,
    child: Box<dyn PtyChild + Send>,
    #[allow(dead_code)]
    master: Option<Arc<Mutex<Box<dyn MasterPty + Send>>>>,
    writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    input_buffer: String,
    output_rx: Receiver<TerminalReaderMessage>,
    target_label: String,
    next_sequence: u64,
    running: bool,
    size: StackTerminalSize,
    #[cfg(test)]
    test_size: Option<StackTerminalSize>,
}

#[derive(Clone, Debug)]
struct TerminalReaderMessage {
    stream: StackTerminalOutputStream,
    text: String,
    sequence: u64,
}

pub(crate) async fn start_stack_terminal_session(
    app_handle: &AppHandle,
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    request: StackTerminalStartRequest,
) -> Result<StackTerminalSessionSnapshot, String> {
    let cwd = PathBuf::from(super::paths::normalize_existing_dir(&request.folder_path)?);
    let profile = request.profile.unwrap_or(
        settings::load_shell_settings_for_app(app_handle)?
            .stack_browser
            .terminal_profile,
    );
    {
        let runtime = state
            .lock()
            .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
        if !runtime.terminal_sessions.can_start_session() {
            return Err(format!(
                "Stack Browser terminal sessions are limited to {MAX_STACK_TERMINAL_SESSIONS}"
            ));
        }
    }
    let target_label = terminal_event_target_label(request.target_label.as_deref())?;
    let app_handle_for_spawn = app_handle.clone();
    let session = tauri::async_runtime::spawn_blocking(move || {
        spawn_terminal_session(Some(app_handle_for_spawn), profile, cwd, target_label)
    })
    .await
    .map_err(|error| format!("Failed to join terminal spawn task: {error}"))??;
    let snapshot = session.snapshot();
    let target_label = session.target_label.clone();
    let mut runtime = state
        .lock()
        .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
    if !runtime.terminal_sessions.can_start_session() {
        drop(runtime);
        let mut session = session;
        let _ = session.child.kill();
        let _ = session.child.wait();
        return Err(format!(
            "Stack Browser terminal sessions are limited to {MAX_STACK_TERMINAL_SESSIONS}"
        ));
    }

    runtime.terminal_sessions.insert(session);
    drop(runtime);
    emit_cwd_update(app_handle, &target_label, &snapshot);
    Ok(snapshot)
}

pub(crate) async fn write_stack_terminal_session(
    app_handle: &AppHandle,
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    request: StackTerminalWriteRequest,
) -> Result<StackTerminalSessionSnapshot, String> {
    if request.input.len() > MAX_STACK_TERMINAL_WRITE_BYTES {
        return Err(format!(
            "Terminal input is limited to {MAX_STACK_TERMINAL_WRITE_BYTES} bytes per write"
        ));
    }
    let input = request.input;
    let writer = {
        let mut runtime = state
            .lock()
            .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
        let session = runtime.terminal_sessions.session_mut(&request.session_id)?;
        session
            .writer
            .clone()
            .ok_or_else(|| "Terminal session writer is closed".to_string())?
    };
    let input_for_write = input.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut writer = writer
            .lock()
            .map_err(|_| "Failed to lock terminal writer".to_string())?;
        writer
            .write_all(input_for_write.as_bytes())
            .and_then(|_| writer.flush())
            .map_err(|error| format!("Failed to write terminal input: {error}"))
    })
    .await
    .map_err(|error| format!("Failed to join terminal write task: {error}"))??;
    let (target_label, snapshot) = {
        let mut runtime = state
            .lock()
            .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
        let session = runtime.terminal_sessions.session_mut(&request.session_id)?;
        if let Some(cwd) = session.observe_terminal_input_for_cwd(&input) {
            session.cwd = cwd;
        }
        (session.target_label.clone(), session.snapshot())
    };
    emit_cwd_update(app_handle, &target_label, &snapshot);
    Ok(snapshot)
}

pub(crate) async fn write_stack_terminal(
    app_handle: &AppHandle,
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    request: StackTerminalWriteRequest,
) -> Result<(), String> {
    write_stack_terminal_session(app_handle, state, request)
        .await
        .map(|_| ())
}

pub(crate) fn resize_stack_terminal_session(
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    request: StackTerminalResizeRequest,
) -> Result<(), String> {
    validate_stack_terminal_session_id(&request.session_id)?;
    validate_stack_terminal_size(request.cols, request.rows)?;
    let master = {
        let mut runtime = state
            .lock()
            .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
        let session = runtime.terminal_sessions.session_mut(&request.session_id)?;
        session
            .master
            .clone()
            .ok_or_else(|| "Terminal PTY is unavailable".to_string())?
    };
    master
        .lock()
        .map_err(|_| "Failed to lock terminal PTY".to_string())?
        .resize(PtySize {
            rows: request.rows,
            cols: request.cols,
            pixel_width: request.pixel_width.unwrap_or(0),
            pixel_height: request.pixel_height.unwrap_or(0),
        })
        .map_err(|error| format!("Failed to resize Stack Browser terminal: {error}"))?;
    let mut runtime = state
        .lock()
        .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
    let session = runtime.terminal_sessions.session_mut(&request.session_id)?;
    session.size = StackTerminalSize {
        cols: request.cols,
        rows: request.rows,
        pixel_width: request.pixel_width,
        pixel_height: request.pixel_height,
    };
    Ok(())
}

pub(crate) fn read_stack_terminal(
    app_handle: &AppHandle,
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
) -> Result<StackTerminalReadResult, String> {
    let result = poll_stack_terminal_session(app_handle, state, session_id)?;
    let output = result
        .chunks
        .iter()
        .map(|chunk| chunk.text.as_str())
        .collect::<String>();
    Ok(StackTerminalReadResult {
        session_id: result.session_id,
        cwd: result.cwd,
        output,
        chunks: result.chunks,
        exited: !result.running,
        exit_code: None,
    })
}

pub(crate) fn poll_stack_terminal_session(
    app_handle: &AppHandle,
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
) -> Result<StackTerminalPollResult, String> {
    let (target_label, snapshot, returned_session_id, cwd, running, chunks) = {
        let mut runtime = state
            .lock()
            .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
        let session = runtime.terminal_sessions.session_mut(&session_id)?;
        let mut chunks = drain_terminal_output(session);
        let running = refresh_session_running(session, &mut chunks);
        (
            session.target_label.clone(),
            session.snapshot(),
            session.id.clone(),
            stack_terminal_cwd_string(&session.cwd),
            running,
            chunks,
        )
    };
    if !running {
        let mut runtime = state
            .lock()
            .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
        let _ = runtime.terminal_sessions.remove(&session_id);
        drop(runtime);
        emit_terminal_closed(app_handle, &target_label, &snapshot);
    }
    Ok(StackTerminalPollResult {
        session_id: returned_session_id,
        cwd,
        running,
        chunks,
    })
}

pub(crate) fn stop_stack_terminal_session(
    app_handle: &AppHandle,
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    request: StackTerminalStopRequest,
) -> Result<StackTerminalSessionSnapshot, String> {
    let Some(mut session) = request_terminal_stop(state, &request.session_id)? else {
        let snapshot = StackTerminalSessionSnapshot {
            session_id: request.session_id,
            profile: TerminalProfile::WindowsTerminal,
            cwd: String::new(),
            running: false,
            cols: 0,
            rows: 0,
            pixel_width: None,
            pixel_height: None,
        };
        emit_terminal_closed(app_handle, shell_windows::STACK_POPUP_LABEL, &snapshot);
        return Ok(snapshot);
    };
    let _ = session.child.kill();
    let _ = session.child.wait();
    session.running = false;
    let target_label = session.target_label.clone();
    let snapshot = session.snapshot();
    emit_terminal_closed(app_handle, &target_label, &snapshot);
    Ok(snapshot)
}

fn request_terminal_stop(
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: &str,
) -> Result<Option<StackTerminalSession>, String> {
    let mut runtime = state
        .lock()
        .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
    runtime.terminal_sessions.request_stop(session_id)
}

pub(crate) fn stop_stack_terminal(
    app_handle: &AppHandle,
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    request: StackTerminalStopRequest,
) -> Result<(), String> {
    stop_stack_terminal_session(app_handle, state, request).map(|_| ())
}

pub(crate) fn get_stack_terminal_cwd(
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
) -> Result<StackTerminalSessionSnapshot, String> {
    let mut runtime = state
        .lock()
        .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
    let session = runtime.terminal_sessions.session_mut(&session_id)?;
    Ok(session.snapshot())
}

fn spawn_terminal_session(
    app_handle: Option<AppHandle>,
    profile: TerminalProfile,
    cwd: PathBuf,
    target_label: String,
) -> Result<StackTerminalSession, String> {
    let plan = terminal_process_plan(profile)?;
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Failed to create Stack Browser terminal PTY: {error}"))?;
    let mut command = CommandBuilder::new(plan.executable.to_string_lossy().to_string());
    command.args(&plan.args);
    apply_terminal_capability_environment(&mut command);
    if matches!(
        profile,
        TerminalProfile::WindowsTerminal | TerminalProfile::PowerShell
    ) {
        if let Some(path) = powershell_augmented_path() {
            command.env("PATH", path);
        }
    }
    if shell_integration_enabled() && matches!(profile, TerminalProfile::GitBash) {
        apply_git_bash_shell_integration(&mut command);
    }
    command.cwd(&cwd);
    let child = pty_pair.slave.spawn_command(command).map_err(|error| {
        format!(
            "Failed to start Stack Browser terminal profile {:?}: {error}",
            profile
        )
    })?;
    let writer = pty_pair
        .master
        .take_writer()
        .map_err(|error| format!("Failed to attach terminal input: {error}"))?;
    let reader = pty_pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("Failed to attach terminal output: {error}"))?;
    let (output_tx, output_rx) = mpsc::sync_channel(1024);
    let session_id = new_stack_terminal_session_id();
    spawn_terminal_reader(
        reader,
        output_tx,
        StackTerminalOutputStream::Stdout,
        app_handle,
        session_id.clone(),
        target_label.clone(),
    );
    Ok(StackTerminalSession {
        id: session_id,
        profile,
        cwd,
        child,
        master: Some(Arc::new(Mutex::new(pty_pair.master))),
        writer: Some(Arc::new(Mutex::new(writer))),
        input_buffer: String::new(),
        output_rx,
        target_label,
        next_sequence: 1,
        running: true,
        size: StackTerminalSize {
            cols: 120,
            rows: 30,
            pixel_width: None,
            pixel_height: None,
        },
        #[cfg(test)]
        test_size: None,
    })
}

fn apply_terminal_capability_environment(command: &mut CommandBuilder) {
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("TERM_PROGRAM", "JasonShell");
}

fn shell_integration_enabled() -> bool {
    std::env::var("JASONSHELL_TERMINAL_SHELL_INTEGRATION")
        .map(|value| value != "0" && !value.eq_ignore_ascii_case("false"))
        .unwrap_or(true)
}

fn apply_git_bash_shell_integration(command: &mut CommandBuilder) {
    command.env(
        "PROMPT_COMMAND",
        r#"__js_ec=$?; printf '\033]133;D;%s\a\033]133;CurrentDir;%s\a\033]133;A\a' "$__js_ec" "$PWD"; printf '\033]133;B\a'"#,
    );
}

fn spawn_terminal_reader<R>(
    mut reader: R,
    tx: SyncSender<TerminalReaderMessage>,
    stream: StackTerminalOutputStream,
    app_handle: Option<AppHandle>,
    session_id: String,
    target_label: String,
) where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        let mut pending_utf8 = Vec::new();
        let mut sequence = 1_u64;
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let text = decode_terminal_output_chunk(&mut pending_utf8, &buffer[..count]);
                    if text.is_empty() {
                        continue;
                    }
                    let chunk = StackTerminalOutputChunk {
                        session_id: session_id.clone(),
                        stream,
                        text: text.clone(),
                        sequence,
                    };
                    if let Some(app_handle) = &app_handle {
                        let _ = app_handle.emit_to(
                            target_label.as_str(),
                            crate::contracts::events::STACK_TERMINAL_OUTPUT,
                            &chunk,
                        );
                    }
                    sequence = sequence.saturating_add(1);
                    if tx
                        .send(TerminalReaderMessage {
                            stream,
                            text,
                            sequence: chunk.sequence,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let text = flush_terminal_output_decoder(&mut pending_utf8);
        if !text.is_empty() {
            let chunk = StackTerminalOutputChunk {
                session_id: session_id.clone(),
                stream,
                text: text.clone(),
                sequence,
            };
            if let Some(app_handle) = &app_handle {
                let _ = app_handle.emit_to(
                    target_label.as_str(),
                    crate::contracts::events::STACK_TERMINAL_OUTPUT,
                    &chunk,
                );
            }
            let _ = tx.send(TerminalReaderMessage {
                stream,
                text,
                sequence: chunk.sequence,
            });
        }
    });
}

fn decode_terminal_output_chunk(pending: &mut Vec<u8>, bytes: &[u8]) -> String {
    pending.extend_from_slice(bytes);
    match std::str::from_utf8(pending) {
        Ok(valid) => {
            let text = valid.to_string();
            pending.clear();
            text
        }
        Err(error) => {
            let valid_up_to = error.valid_up_to();
            if valid_up_to == 0 {
                if error.error_len().is_none() {
                    return String::new();
                }
                let text = String::from_utf8_lossy(pending).to_string();
                pending.clear();
                return text;
            }
            let text = String::from_utf8_lossy(&pending[..valid_up_to]).to_string();
            let remainder = pending[valid_up_to..].to_vec();
            *pending = remainder;
            if error.error_len().is_some() && !pending.is_empty() {
                let replacement = String::from_utf8_lossy(pending).to_string();
                pending.clear();
                return format!("{text}{replacement}");
            }
            text
        }
    }
}

fn flush_terminal_output_decoder(pending: &mut Vec<u8>) -> String {
    if pending.is_empty() {
        return String::new();
    }
    let text = String::from_utf8_lossy(pending).to_string();
    pending.clear();
    text
}

fn drain_terminal_output(session: &mut StackTerminalSession) -> Vec<StackTerminalOutputChunk> {
    let mut chunks = Vec::new();
    while let Ok(message) = session.output_rx.try_recv() {
        chunks.push(StackTerminalOutputChunk {
            session_id: session.id.clone(),
            stream: message.stream,
            text: message.text,
            sequence: message.sequence,
        });
    }
    chunks
}

fn refresh_session_running(
    session: &mut StackTerminalSession,
    chunks: &mut Vec<StackTerminalOutputChunk>,
) -> bool {
    let running = match session.child.try_wait() {
        Ok(Some(status)) => {
            if session.running {
                chunks.push(session.next_chunk(
                    StackTerminalOutputStream::System,
                    format!("Terminal exited with status {status}\n"),
                ));
            }
            false
        }
        Ok(None) => true,
        Err(error) => {
            if session.running {
                chunks.push(session.next_chunk(
                    StackTerminalOutputStream::System,
                    format!("Failed to inspect terminal status: {error}\n"),
                ));
            }
            false
        }
    };
    session.running = running;
    running
}

impl StackTerminalSession {
    fn snapshot(&self) -> StackTerminalSessionSnapshot {
        StackTerminalSessionSnapshot {
            session_id: self.id.clone(),
            profile: self.profile,
            cwd: stack_terminal_cwd_string(&self.cwd),
            running: self.running,
            cols: self.size.cols,
            rows: self.size.rows,
            pixel_width: self.size.pixel_width,
            pixel_height: self.size.pixel_height,
        }
    }

    fn next_chunk(
        &mut self,
        stream: StackTerminalOutputStream,
        text: String,
    ) -> StackTerminalOutputChunk {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        StackTerminalOutputChunk {
            session_id: self.id.clone(),
            stream,
            text,
            sequence,
        }
    }

    fn observe_terminal_input_for_cwd(&mut self, input: &str) -> Option<PathBuf> {
        let mut cwd = None;
        for ch in input.chars() {
            match ch {
                '\r' | '\n' => {
                    let line = std::mem::take(&mut self.input_buffer);
                    if let Some(next) = cwd_after_terminal_line(&self.cwd, &line) {
                        cwd = Some(next);
                    }
                }
                '\u{3}' => {
                    self.input_buffer.clear();
                }
                '\u{8}' | '\u{7f}' => {
                    self.input_buffer.pop();
                }
                '\u{1b}' => {}
                ch if !ch.is_control() => self.input_buffer.push(ch),
                _ => {}
            }
        }
        cwd
    }
}

pub(crate) fn terminal_process_plan(
    profile: TerminalProfile,
) -> Result<TerminalProcessPlan, String> {
    match profile {
        TerminalProfile::WindowsTerminal | TerminalProfile::PowerShell => Ok(TerminalProcessPlan {
            executable: trusted_cmd_path()?,
            args: vec![
                "/K".to_string(),
                powershell_cmd_launch_line(trusted_powershell_path()?),
            ],
            candidates: trusted_cmd_candidates(),
        }),
        TerminalProfile::GitBash => {
            let candidates = git_bash_candidates();
            let executable = candidates
                .iter()
                .find(|candidate| candidate.is_file())
                .cloned()
                .ok_or_else(|| {
                    "Git Bash was not found in a trusted install location".to_string()
                })?;
            Ok(TerminalProcessPlan {
                executable,
                args: vec!["--login".to_string(), "-i".to_string()],
                candidates,
            })
        }
    }
}

fn powershell_cmd_launch_line(powershell: PathBuf) -> String {
    let trusted_path =
        short_windows_path(&powershell).unwrap_or_else(|| powershell.to_string_lossy().to_string());
    let encoded_startup = powershell_encoded_command(&powershell_startup_script());
    format!(
        "{} {}",
        trusted_path,
        [
            "-NoLogo".to_string(),
            "-NoProfile".to_string(),
            "-ExecutionPolicy".to_string(),
            "Bypass".to_string(),
            "-NoExit".to_string(),
            "-EncodedCommand".to_string(),
            encoded_startup,
        ]
        .join(" ")
    )
}

#[cfg(windows)]
fn short_windows_path(path: &Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let required = unsafe { GetShortPathNameW(PCWSTR(wide.as_ptr()), None) };
    if required == 0 {
        return None;
    }
    let mut buffer = vec![0u16; required as usize + 1];
    let written = unsafe { GetShortPathNameW(PCWSTR(wide.as_ptr()), Some(&mut buffer)) };
    if written == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..written as usize]))
}

#[cfg(not(windows))]
fn short_windows_path(path: &Path) -> Option<String> {
    Some(path.to_string_lossy().to_string())
}

fn powershell_augmented_path() -> Option<String> {
    let powershell = trusted_powershell_path().ok()?;
    let parent = powershell.parent()?;
    let existing = std::env::var_os("PATH").unwrap_or_default();
    let mut next = parent.as_os_str().to_os_string();
    next.push(";");
    next.push(existing);
    Some(next.to_string_lossy().to_string())
}

fn powershell_startup_script() -> String {
    [
        "$ErrorActionPreference = 'SilentlyContinue'",
        "if (Get-Module -ListAvailable -Name PSReadLine) { Import-Module PSReadLine; Set-PSReadLineOption -Colors @{ InlinePrediction = \"`e[38;5;240m\"; ListPrediction = \"`e[38;5;244m\"; ListPredictionSelected = \"`e[48;5;238m\" }; Set-PSReadLineKeyHandler -Key RightArrow -Function AcceptSuggestion; Set-PSReadLineKeyHandler -Key Tab -Function TabCompleteNext; Set-PSReadLineKeyHandler -Key Shift+Tab -Function TabCompletePrevious; Set-PSReadLineKeyHandler -Key Ctrl+Spacebar -Function MenuComplete }",
        "Set-Alias -Name ls -Value Get-ChildItem -Force -ErrorAction SilentlyContinue",
        "Set-Alias -Name ll -Value Get-ChildItem -Force -ErrorAction SilentlyContinue",
        "Set-Alias -Name clear -Value Clear-Host -Force -ErrorAction SilentlyContinue",
        "Set-Alias -Name cat -Value Get-Content -Force -ErrorAction SilentlyContinue",
        "Set-Alias -Name grep -Value Select-String -Force -ErrorAction SilentlyContinue",
        "function which { Get-Command @args }",
        r#"function prompt { $last = if ($global:LASTEXITCODE -is [int]) { $global:LASTEXITCODE } elseif ($?) { 0 } else { 1 }; $cwd = $executionContext.SessionState.Path.CurrentLocation.Path; if ($env:JASONSHELL_TERMINAL_SHELL_INTEGRATION -ne '0' -and $env:JASONSHELL_TERMINAL_SHELL_INTEGRATION -ne 'false') { Write-Host -NoNewline "`e]133;D;$last`a`e]133;CurrentDir;$cwd`a`e]133;A`a" }; $gitLine = $null; if (Get-Command git -ErrorAction SilentlyContinue) { $inside = git rev-parse --is-inside-work-tree 2>$null; if ($LASTEXITCODE -eq 0 -and $inside -eq 'true') { $branch = git symbolic-ref --quiet --short HEAD 2>$null; if (-not $branch) { $branch = git rev-parse --short HEAD 2>$null }; $raw = git status --porcelain=v1 -z 2>$null; $modified = 0; $deleted = 0; $untracked = 0; if ($raw) { foreach ($entry in ($raw -split "`0")) { if (-not $entry) { continue }; if ($entry.StartsWith('??')) { $untracked += 1; continue }; $xy = $entry.Substring(0, [Math]::Min(2, $entry.Length)); if ($xy.Contains('D')) { $deleted += 1 } else { $modified += 1 } } }; $parts = @($branch); if ($modified -gt 0) { $parts += "+$modified" }; if ($deleted -gt 0) { $parts += "-$deleted" }; if ($untracked -gt 0) { $parts += "?$untracked" }; $gitLine = $parts -join ' ' } }; Write-Host -NoNewline "$($executionContext.SessionState.Path.CurrentLocation)$('>' * ($nestedPromptLevel + 1))"; if ($gitLine) { Write-Host -NoNewline " ($gitLine)" -ForegroundColor Cyan }; " " }"#,
    ]
    .join("; ")
}

fn powershell_encoded_command(script: &str) -> String {
    let bytes = script
        .encode_utf16()
        .flat_map(|unit| unit.to_le_bytes())
        .collect::<Vec<u8>>();
    BASE64.encode(bytes)
}

fn trusted_cmd_path() -> Result<PathBuf, String> {
    trusted_cmd_candidates()
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| "cmd.exe was not found in a trusted Windows location".to_string())
}

fn trusted_cmd_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        candidates.push(PathBuf::from(system_root).join("System32").join("cmd.exe"));
    }
    candidates.push(PathBuf::from(r"C:\Windows\System32\cmd.exe"));
    candidates
}

fn trusted_powershell_path() -> Result<PathBuf, String> {
    // PowerShell 7 is preferred; fallback is WindowsPowerShell from System32.
    trusted_powershell_candidates()
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| "PowerShell was not found in a trusted Windows location".to_string())
}

fn trusted_powershell_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        candidates.push(
            PathBuf::from(program_files)
                .join("PowerShell")
                .join("7")
                .join("pwsh.exe"),
        );
    }
    candidates.push(PathBuf::from(r"C:\Program Files\PowerShell\7\pwsh.exe"));
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        candidates.push(
            PathBuf::from(system_root)
                .join("System32")
                .join("WindowsPowerShell")
                .join("v1.0")
                .join("powershell.exe"),
        );
    }
    candidates.push(PathBuf::from(
        r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
    ));
    candidates
}

fn git_bash_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        candidates.push(
            PathBuf::from(program_files)
                .join("Git")
                .join("bin")
                .join("bash.exe"),
        );
    }
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        candidates.push(
            PathBuf::from(program_files_x86)
                .join("Git")
                .join("bin")
                .join("bash.exe"),
        );
    }
    candidates.push(PathBuf::from(r"C:\Program Files\Git\bin\bash.exe"));
    candidates
}

pub(crate) fn new_stack_terminal_session_id() -> String {
    let counter = STACK_TERMINAL_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{STACK_TERMINAL_SESSION_PREFIX}{epoch_ms:x}-{counter:x}")
}

pub(crate) fn validate_stack_terminal_session_id(value: &str) -> Result<(), String> {
    if value.len() > MAX_STACK_TERMINAL_SESSION_ID_LEN
        || !value.starts_with(STACK_TERMINAL_SESSION_PREFIX)
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err("Terminal session id is invalid".to_string());
    }
    Ok(())
}

fn validate_stack_terminal_size(cols: u16, rows: u16) -> Result<(), String> {
    if cols == 0 || rows == 0 || cols > 600 || rows > 300 {
        return Err("Terminal size is outside supported bounds".to_string());
    }
    Ok(())
}

#[cfg(test)]
fn validate_stack_terminal_resize_request(
    session_id: &str,
    cols: u16,
    rows: u16,
    _pixel_width: Option<u16>,
    _pixel_height: Option<u16>,
) -> Result<(), String> {
    validate_stack_terminal_session_id(session_id)?;
    validate_stack_terminal_size(cols, rows)
}

#[cfg(test)]
pub(crate) fn cwd_after_terminal_input(current: &Path, input: &str) -> Option<PathBuf> {
    let mut cwd = None;
    let mut base = current.to_path_buf();
    for line in input.lines() {
        if let Some(next) = cwd_after_terminal_line(&base, line) {
            base = next.clone();
            cwd = Some(next);
        }
    }
    cwd
}

fn cwd_after_terminal_line(current: &Path, line: &str) -> Option<PathBuf> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    let rest = if lower == "cd" || lower == "chdir" {
        return Some(home_dir().unwrap_or_else(|| current.to_path_buf()));
    } else if lower.starts_with("cd ") {
        &trimmed[3..]
    } else if lower.starts_with("chdir ") {
        &trimmed[6..]
    } else if lower.starts_with("set-location ") {
        &trimmed[13..]
    } else if lower.starts_with("sl ") {
        &trimmed[3..]
    } else {
        return None;
    };
    let target = normalize_cd_target(rest)?;
    let next = if Path::new(&target).is_absolute() {
        PathBuf::from(target)
    } else {
        current.join(target)
    };
    if next.is_dir() {
        Some(std::fs::canonicalize(&next).unwrap_or(next))
    } else {
        None
    }
}

fn normalize_cd_target(rest: &str) -> Option<String> {
    let mut value = rest.trim();
    if value.starts_with("/d ") || value.starts_with("/D ") {
        value = value[3..].trim();
    }
    for flag in ["-LiteralPath", "-Path"] {
        if value
            .get(..flag.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(flag))
        {
            value = value[flag.len()..].trim();
        }
    }
    if value.is_empty() || value.starts_with("/?") {
        return None;
    }
    Some(strip_matching_quotes(value).to_string())
}

fn strip_matching_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

fn stack_terminal_cwd_string(cwd: &Path) -> String {
    display_stack_terminal_path(cwd)
}

fn display_stack_terminal_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(rest) = value.strip_prefix("\\\\?\\UNC\\") {
        return format!("\\\\{rest}");
    }
    value
        .strip_prefix("\\\\?\\")
        .unwrap_or(value.as_ref())
        .to_string()
}

fn terminal_event_target_label(target_label: Option<&str>) -> Result<String, String> {
    match target_label.unwrap_or(shell_windows::STACK_POPUP_LABEL) {
        shell_windows::STACK_POPUP_LABEL => Ok(shell_windows::STACK_POPUP_LABEL.to_string()),
        shell_windows::TERMINAL_PANEL_LABEL => Ok(shell_windows::TERMINAL_PANEL_LABEL.to_string()),
        other => Err(format!("Unsupported terminal event target: {other}")),
    }
}

fn emit_cwd_update(
    app_handle: &AppHandle,
    target_label: &str,
    snapshot: &StackTerminalSessionSnapshot,
) {
    let payload = StackTerminalCwdUpdate {
        session_id: snapshot.session_id.clone(),
        cwd: snapshot.cwd.clone(),
    };
    let _ = app_handle.emit_to(
        target_label,
        crate::contracts::events::STACK_TERMINAL_CWD,
        payload,
    );
}

fn emit_terminal_closed(
    app_handle: &AppHandle,
    target_label: &str,
    snapshot: &StackTerminalSessionSnapshot,
) {
    let _ = app_handle.emit_to(
        target_label,
        crate::contracts::events::STACK_TERMINAL_CLOSED,
        snapshot,
    );
}

#[cfg(test)]
fn test_child() -> Box<dyn PtyChild + Send> {
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize {
            rows: 1,
            cols: 1,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    #[cfg(windows)]
    let command = CommandBuilder::new("cmd.exe");
    #[cfg(not(windows))]
    let command = CommandBuilder::new("sh");
    pty_pair.slave.spawn_command(command).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn terminal_profiles_have_fixed_process_plans() {
        let windows_terminal = terminal_process_plan(TerminalProfile::WindowsTerminal).unwrap();
        let powershell = terminal_process_plan(TerminalProfile::PowerShell).unwrap();
        let git_bash_candidates = git_bash_candidates();

        assert!(windows_terminal.executable.is_absolute());
        assert!(powershell.executable.is_absolute());
        assert!(windows_terminal.executable.ends_with(r"System32\cmd.exe"));
        assert!(
            !windows_terminal.executable.ends_with("powershell.exe")
                || windows_terminal.executable.is_absolute()
        );
        assert!(!windows_terminal
            .candidates
            .iter()
            .any(|candidate| candidate == Path::new("powershell.exe")));
        assert!(git_bash_candidates
            .iter()
            .any(|candidate| candidate.ends_with(r"Git\bin\bash.exe")));
        assert!(!git_bash_candidates
            .iter()
            .any(|candidate| candidate == Path::new("bash.exe")));
        assert!(!windows_terminal
            .args
            .iter()
            .any(|arg| arg.contains("cmd /c")));
        assert!(windows_terminal.args.iter().any(|arg| arg == "/K"));
        assert!(windows_terminal
            .args
            .iter()
            .any(|arg| arg.contains("-ExecutionPolicy Bypass")));
        assert!(windows_terminal
            .args
            .iter()
            .any(|arg| arg.contains("pwsh.exe ")));
        assert!(windows_terminal
            .args
            .iter()
            .any(|arg| arg.contains("-EncodedCommand")));
        assert!(windows_terminal
            .args
            .iter()
            .any(|arg| arg.contains("-NoProfile")));
        assert!(!windows_terminal
            .args
            .iter()
            .any(|arg| arg.contains("Set-Alias")));
        assert!(!windows_terminal
            .args
            .iter()
            .any(|arg| arg.contains("Set-PSReadLineKeyHandler")));
    }

    #[test]
    fn powershell_startup_is_hidden_and_right_arrow_accepts_muted_suggestions() {
        let startup_script = powershell_startup_script();
        assert!(startup_script.contains("InlinePrediction = \"`e[38;5;240m\""));
        assert!(startup_script.contains("ListPrediction = \"`e[38;5;244m\""));
        assert!(startup_script
            .contains("Set-PSReadLineKeyHandler -Key RightArrow -Function AcceptSuggestion"));
        assert!(
            startup_script.contains("Set-PSReadLineKeyHandler -Key Tab -Function TabCompleteNext")
        );
        assert!(startup_script
            .contains("Set-PSReadLineKeyHandler -Key Shift+Tab -Function TabCompletePrevious"));
        assert!(startup_script.contains(
            "Set-Alias -Name ls -Value Get-ChildItem -Force -ErrorAction SilentlyContinue"
        ));
        assert!(!startup_script.contains("\"Clear-Host\","));
        assert!(startup_script.contains("function prompt"));
        assert!(startup_script.contains("git rev-parse --is-inside-work-tree"));
        assert!(startup_script.contains("git status --porcelain=v1 -z"));
        assert!(startup_script.contains("$parts = @($branch)"));
        assert!(startup_script.contains(
            "Write-Host -NoNewline \"$($executionContext.SessionState.Path.CurrentLocation)"
        ));
        assert!(!startup_script.contains(
            "Write-Host -NoNewline \"PS $($executionContext.SessionState.Path.CurrentLocation)"
        ));
        assert!(
            startup_script.contains("Write-Host -NoNewline \" ($gitLine)\" -ForegroundColor Cyan")
        );
        assert!(!startup_script.contains("Write-Host ''"));
        assert!(!startup_script.contains("$parts = @(\"git\", $branch)"));
        assert!(startup_script.contains("$($executionContext.SessionState.Path.CurrentLocation)"));
        assert!(
            !startup_script.contains("PS $($executionContext.SessionState.Path.CurrentLocation)")
        );

        let encoded = powershell_encoded_command(&startup_script);
        assert!(!encoded.contains("Set-Alias"));
        assert!(!encoded.contains("Set-PSReadLineKeyHandler"));

        let launch_line =
            powershell_cmd_launch_line(PathBuf::from(r"C:\Program Files\PowerShell\7\pwsh.exe"));
        assert!(launch_line.contains("-NoProfile"));
        assert!(launch_line.contains("-EncodedCommand"));
        assert!(!launch_line.contains("Set-Alias"));
        assert!(!launch_line.contains("Set-PSReadLineKeyHandler"));
    }

    #[test]
    fn powershell_conpty_session_stays_running_after_start() {
        let root = test_dir("conpty-start");
        let mut session = spawn_terminal_session(
            None,
            TerminalProfile::PowerShell,
            root.clone(),
            shell_windows::STACK_POPUP_LABEL.to_string(),
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let mut chunks = drain_terminal_output(&mut session);
        assert!(
            refresh_session_running(&mut session, &mut chunks),
            "PowerShell ConPTY session exited early with output: {:?}",
            chunks
        );
        let output = chunks
            .iter()
            .map(|chunk| chunk.text.as_str())
            .collect::<String>();
        assert!(
            !output.contains("is not recognized as an internal or external command"),
            "PowerShell launch command was quoted incorrectly: {output:?}"
        );
        let _ = session.child.kill();
        let _ = session.child.wait();
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn cmd_conpty_spawn_smoke() {
        let root = test_dir("conpty-cmd");
        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows: 30,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut command = CommandBuilder::new("cmd.exe");
        command.cwd(&root);
        let mut child = pty_pair.slave.spawn_command(command).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(child.try_wait().unwrap().is_none());
        let _ = child.kill();
        let _ = child.wait();
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn cmd_launches_pwsh_in_conpty() {
        let root = test_dir("conpty-cmd-pwsh");
        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows: 30,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut command = CommandBuilder::new("cmd.exe");
        command.args([
            "/K",
            r#""C:\Program Files\PowerShell\7\pwsh.exe" -NoLogo -ExecutionPolicy Bypass -NoExit"#,
        ]);
        command.cwd(&root);
        let mut child = pty_pair.slave.spawn_command(command).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(child.try_wait().unwrap().is_none());
        let _ = child.kill();
        let _ = child.wait();
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn terminal_output_decoder_preserves_split_utf8_sequences() {
        let mut pending = Vec::new();
        assert_eq!(decode_terminal_output_chunk(&mut pending, &[0xE2, 0x94]), "");
        assert_eq!(decode_terminal_output_chunk(&mut pending, &[0x80, b' ', b'O', b'K']), "─ OK");
        assert!(pending.is_empty());

        assert_eq!(decode_terminal_output_chunk(&mut pending, b"bad"), "bad");
        assert_eq!(decode_terminal_output_chunk(&mut pending, &[0xF0, 0x9F]), "");
        assert_eq!(flush_terminal_output_decoder(&mut pending), "�");
        assert!(pending.is_empty());
    }

    #[test]
    fn display_paths_strip_extended_windows_prefix() {
        assert_eq!(
            display_stack_terminal_path(Path::new(r"\\?\C:\dev")),
            r"C:\dev"
        );
        assert_eq!(
            display_stack_terminal_path(Path::new(r"\\?\UNC\server\share")),
            r"\\server\share"
        );
        assert_eq!(display_stack_terminal_path(Path::new(r"C:\dev")), r"C:\dev");
    }

    #[test]
    fn generated_terminal_session_ids_are_bounded() {
        let first = new_stack_terminal_session_id();
        let second = new_stack_terminal_session_id();

        assert_ne!(first, second);
        assert!(validate_stack_terminal_session_id(&first).is_ok());
        assert!(first.len() <= MAX_STACK_TERMINAL_SESSION_ID_LEN);
        assert!(validate_stack_terminal_session_id("../bad").is_err());
        assert!(validate_stack_terminal_session_id(&"x".repeat(80)).is_err());
    }

    #[test]
    fn cd_commands_update_tracked_terminal_cwd() {
        let root = test_dir("cwd");
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).unwrap();

        let updated =
            cwd_after_terminal_input(&root, &format!("cd {}\n", quote_arg(&nested))).unwrap();
        assert_eq!(
            std::fs::canonicalize(updated).unwrap(),
            std::fs::canonicalize(&nested).unwrap()
        );

        let relative = cwd_after_terminal_input(&nested, "cd ..\n").unwrap();
        assert_eq!(
            std::fs::canonicalize(relative).unwrap(),
            std::fs::canonicalize(&root).unwrap()
        );

        assert!(cwd_after_terminal_input(&root, "npm run check\n").is_none());
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn terminal_runtime_state_caps_active_sessions() {
        let mut registry = StackTerminalRegistry::default();
        for index in 0..(MAX_STACK_TERMINAL_SESSIONS - 1) {
            registry.insert_test_session(format!("stack-term-{index}"), PathBuf::from(r"C:\dev"));
        }

        assert!(registry.can_start_session());
        registry.insert_test_session("stack-term-live".to_string(), PathBuf::from(r"C:\dev"));
        assert!(!registry.can_start_session());
    }

    #[test]
    fn terminal_stop_request_blocks_late_reinsert() {
        let mut registry = StackTerminalRegistry::default();
        let session_id = "stack-term-race";

        assert!(registry.request_stop(session_id).unwrap().is_none());
        assert!(registry.should_drop_for_stop(session_id));
        assert!(!registry.should_drop_for_stop(session_id));
    }

    #[test]
    fn terminal_resize_request_rejects_invalid_session_and_bounds() {
        assert!(
            validate_stack_terminal_resize_request("../bad", 80, 24, Some(800), Some(400)).is_err()
        );
        assert!(validate_stack_terminal_resize_request(
            "stack-term-good",
            0,
            24,
            Some(800),
            Some(400)
        )
        .is_err());
        assert!(validate_stack_terminal_resize_request(
            "stack-term-good",
            80,
            0,
            Some(800),
            Some(400)
        )
        .is_err());
        assert!(validate_stack_terminal_resize_request(
            "stack-term-good",
            601,
            24,
            Some(800),
            Some(400)
        )
        .is_err());
        assert!(validate_stack_terminal_resize_request(
            "stack-term-good",
            80,
            301,
            Some(800),
            Some(400)
        )
        .is_err());
        assert!(
            validate_stack_terminal_resize_request("stack-term-good", 80, 24, None, None).is_ok()
        );
    }

    #[test]
    fn terminal_resize_test_session_records_requested_size() {
        let mut registry = StackTerminalRegistry::default();
        registry.insert_test_session("stack-term-resize".to_string(), PathBuf::from(r"C:\dev"));

        registry
            .resize_test_session("stack-term-resize", 132, 43, Some(1200), Some(700))
            .unwrap();

        assert_eq!(
            registry.test_session_size("stack-term-resize").unwrap(),
            StackTerminalSize {
                cols: 132,
                rows: 43,
                pixel_width: Some(1200),
                pixel_height: Some(700)
            }
        );
    }

    #[test]
    fn terminal_registry_keeps_live_session_visible_during_operations() {
        let mut registry = StackTerminalRegistry::default();
        registry.insert_test_session("stack-term-live".to_string(), PathBuf::from(r"C:\dev"));

        let _operation = registry.begin_test_operation("stack-term-live").unwrap();

        assert!(
            registry.session_mut("stack-term-live").is_ok(),
            "live terminal sessions must not disappear from registry during poll/write/resize"
        );
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "jasonshell-stack-terminal-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn quote_arg(path: &Path) -> String {
        format!("\"{}\"", path.to_string_lossy())
    }
}
