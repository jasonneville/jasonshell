use crate::settings::{self, TerminalProfile};
use crate::shell_windows;
use crate::stack_popup::models::StackPopupRuntimeState;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

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
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTerminalWriteRequest {
    pub session_id: String,
    pub input: String,
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
                stdin: None,
                output_rx: mpsc::channel().1,
                next_sequence: 1,
                running: true,
            },
        );
    }
}

struct StackTerminalSession {
    id: String,
    profile: TerminalProfile,
    cwd: PathBuf,
    child: Child,
    stdin: Option<ChildStdin>,
    output_rx: Receiver<TerminalReaderMessage>,
    next_sequence: u64,
    running: bool,
}

#[derive(Clone, Debug)]
struct TerminalReaderMessage {
    stream: StackTerminalOutputStream,
    text: String,
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
    let session =
        tauri::async_runtime::spawn_blocking(move || spawn_terminal_session(profile, cwd))
            .await
            .map_err(|error| format!("Failed to join terminal spawn task: {error}"))??;
    let snapshot = session.snapshot();
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
    emit_cwd_update(app_handle, &snapshot);
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
    let mut session = take_terminal_session(state, &request.session_id)?;
    let input = request.input;
    let write_result = tauri::async_runtime::spawn_blocking(move || {
        let stdin = match session.stdin.as_mut() {
            Some(stdin) => stdin,
            None => return Err((session, "Terminal session stdin is closed".to_string())),
        };
        if let Err(error) = stdin
            .write_all(input.as_bytes())
            .and_then(|_| stdin.flush())
        {
            return Err((session, format!("Failed to write terminal input: {error}")));
        }

        if let Some(cwd) = cwd_after_terminal_input(&session.cwd, &input) {
            session.cwd = cwd;
        }
        Ok::<StackTerminalSession, (StackTerminalSession, String)>(session)
    })
    .await
    .map_err(|error| format!("Failed to join terminal write task: {error}"))?;

    let session = match write_result {
        Ok(session) => session,
        Err((session, error)) => {
            if put_terminal_session(state, session)? {
                return Err(error);
            }
            return Err(error);
        }
    };
    let snapshot = session.snapshot();
    if put_terminal_session(state, session)? {
        emit_cwd_update(app_handle, &snapshot);
    } else {
        emit_terminal_closed(app_handle, &snapshot);
    }
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

pub(crate) fn read_stack_terminal(
    app_handle: &AppHandle,
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
) -> Result<StackTerminalReadResult, String> {
    let result = poll_stack_terminal_session(app_handle, state, session_id)?;
    Ok(StackTerminalReadResult {
        session_id: result.session_id,
        cwd: result.cwd,
        output: result
            .chunks
            .iter()
            .map(|chunk| chunk.text.as_str())
            .collect::<String>(),
        exited: !result.running,
        exit_code: None,
    })
}

pub(crate) fn poll_stack_terminal_session(
    app_handle: &AppHandle,
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
) -> Result<StackTerminalPollResult, String> {
    let mut session = take_terminal_session(state, &session_id)?;
    let mut chunks = drain_terminal_output(&mut session);
    let running = refresh_session_running(&mut session, &mut chunks);
    let snapshot = session.snapshot();
    let cwd = stack_terminal_cwd_string(&session.cwd);
    let returned_session_id = session.id.clone();
    if running {
        if !put_terminal_session(state, session)? {
            emit_terminal_closed(app_handle, &snapshot);
            return Ok(StackTerminalPollResult {
                session_id: returned_session_id,
                cwd,
                running: false,
                chunks,
            });
        }
    }
    for chunk in &chunks {
        let _ = app_handle.emit_to(
            shell_windows::STACK_POPUP_LABEL,
            crate::contracts::events::STACK_TERMINAL_OUTPUT,
            chunk,
        );
    }
    if !running {
        let _ = app_handle.emit_to(
            shell_windows::STACK_POPUP_LABEL,
            crate::contracts::events::STACK_TERMINAL_CLOSED,
            &snapshot,
        );
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
        };
        emit_terminal_closed(app_handle, &snapshot);
        return Ok(snapshot);
    };
    let _ = session.child.kill();
    let _ = session.child.wait();
    session.running = false;
    let snapshot = session.snapshot();
    emit_terminal_closed(app_handle, &snapshot);
    Ok(snapshot)
}

fn take_terminal_session(
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: &str,
) -> Result<StackTerminalSession, String> {
    let mut runtime = state
        .lock()
        .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
    runtime.terminal_sessions.remove(session_id)
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

fn put_terminal_session(
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    mut session: StackTerminalSession,
) -> Result<bool, String> {
    let mut runtime = state
        .lock()
        .map_err(|_| "Failed to lock stack popup runtime state".to_string())?;
    if runtime.terminal_sessions.should_drop_for_stop(&session.id) {
        drop(runtime);
        let _ = session.child.kill();
        let _ = session.child.wait();
        return Ok(false);
    }
    runtime.terminal_sessions.insert(session);
    Ok(true)
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
    profile: TerminalProfile,
    cwd: PathBuf,
) -> Result<StackTerminalSession, String> {
    let plan = terminal_process_plan(profile)?;
    let mut child = Command::new(&plan.executable)
        .args(&plan.args)
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "Failed to start Stack Browser terminal profile {:?}: {error}",
                profile
            )
        })?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (output_tx, output_rx) = mpsc::channel();
    if let Some(stdout) = stdout {
        spawn_terminal_reader(stdout, output_tx.clone(), StackTerminalOutputStream::Stdout);
    }
    if let Some(stderr) = stderr {
        spawn_terminal_reader(stderr, output_tx, StackTerminalOutputStream::Stderr);
    }
    Ok(StackTerminalSession {
        id: new_stack_terminal_session_id(),
        profile,
        cwd,
        child,
        stdin,
        output_rx,
        next_sequence: 1,
        running: true,
    })
}

fn spawn_terminal_reader<R>(
    mut reader: R,
    tx: mpsc::Sender<TerminalReaderMessage>,
    stream: StackTerminalOutputStream,
) where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let text = String::from_utf8_lossy(&buffer[..count]).to_string();
                    if tx.send(TerminalReaderMessage { stream, text }).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
}

fn drain_terminal_output(session: &mut StackTerminalSession) -> Vec<StackTerminalOutputChunk> {
    let mut chunks = Vec::new();
    while let Ok(message) = session.output_rx.try_recv() {
        chunks.push(session.next_chunk(message.stream, message.text));
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
}

pub(crate) fn terminal_process_plan(
    profile: TerminalProfile,
) -> Result<TerminalProcessPlan, String> {
    match profile {
        TerminalProfile::WindowsTerminal | TerminalProfile::PowerShell => Ok(TerminalProcessPlan {
            executable: trusted_powershell_path()?,
            args: vec![
                "-NoLogo".to_string(),
                "-NoProfile".to_string(),
                "-NoExit".to_string(),
            ],
            candidates: trusted_powershell_candidates(),
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

fn trusted_powershell_path() -> Result<PathBuf, String> {
    trusted_powershell_candidates()
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| "PowerShell was not found in a trusted Windows location".to_string())
}

fn trusted_powershell_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
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
    cwd.to_string_lossy().to_string()
}

fn emit_cwd_update(app_handle: &AppHandle, snapshot: &StackTerminalSessionSnapshot) {
    let payload = StackTerminalCwdUpdate {
        session_id: snapshot.session_id.clone(),
        cwd: snapshot.cwd.clone(),
    };
    let _ = app_handle.emit_to(
        shell_windows::STACK_POPUP_LABEL,
        crate::contracts::events::STACK_TERMINAL_CWD,
        payload,
    );
}

fn emit_terminal_closed(app_handle: &AppHandle, snapshot: &StackTerminalSessionSnapshot) {
    let _ = app_handle.emit_to(
        shell_windows::STACK_POPUP_LABEL,
        crate::contracts::events::STACK_TERMINAL_CLOSED,
        snapshot,
    );
}

#[cfg(test)]
fn test_child() -> Child {
    #[cfg(windows)]
    {
        Command::new("cmd.exe")
            .arg("/C")
            .arg("pause")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }
    #[cfg(not(windows))]
    {
        Command::new("sh")
            .arg("-c")
            .arg("sleep 60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }
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
        assert!(windows_terminal
            .executable
            .ends_with(r"WindowsPowerShell\v1.0\powershell.exe"));
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
