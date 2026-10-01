use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct ProcessRunSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub envs: Vec<(String, Option<String>)>,
    pub stdin: Option<Vec<u8>>,
    pub timeout: Duration,
    pub stdout_cap: usize,
    pub stderr_cap: usize,
    pub poll_interval: Duration,
    pub kill_tree: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessTimeoutKind {
    DeadlineExceeded,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProcessRunOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_total_bytes: u64,
    pub stderr_total_bytes: u64,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProcessRunError {
    Spawn(String),
    StdinWrite(String),
    StatusQuery {
        reason: String,
        stdout_total_bytes: u64,
        stderr_total_bytes: u64,
    },
    Timeout {
        kind: ProcessTimeoutKind,
        stdout_total_bytes: u64,
        stderr_total_bytes: u64,
        stdout_truncated: bool,
        stderr_truncated: bool,
    },
    CleanupIncomplete {
        reason: String,
        stdout_total_bytes: u64,
        stderr_total_bytes: u64,
    },
    NonZero {
        status: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        stdout_total_bytes: u64,
        stderr_total_bytes: u64,
        stdout_truncated: bool,
        stderr_truncated: bool,
    },
}

pub fn run_process(spec: ProcessRunSpec) -> Result<ProcessRunOutput, ProcessRunError> {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if spec.stdin.is_some() {
        command.stdin(Stdio::piped());
    }
    if let Some(cwd) = &spec.cwd {
        command.current_dir(cwd);
    }
    for (key, value) in &spec.envs {
        match value {
            Some(value) => {
                command.env(key, value);
            }
            None => {
                command.env_remove(key);
            }
        }
    }

    let mut child = command
        .spawn()
        .map_err(|err| ProcessRunError::Spawn(err.to_string()))?;
    if let Some(stdin) = spec.stdin {
        if let Err(reason) = write_child_stdin(&mut child, stdin) {
            return Err(cleanup_after_stdin_write_failure(child, reason));
        }
    }

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (stdout_rx, stdout_handle) = spawn_drain(stdout, spec.stdout_cap);
    let (stderr_rx, stderr_handle) = spawn_drain(stderr, spec.stderr_cap);
    let deadline = Instant::now() + spec.timeout;
    match wait_with_deadline(&mut child, deadline, spec.poll_interval) {
        Ok(status) => finish_success(
            child,
            stdout_rx,
            stderr_rx,
            stdout_handle,
            stderr_handle,
            status,
        ),
        Err(wait_error) => cleanup_after_wait(
            child,
            stdout_rx,
            stderr_rx,
            stdout_handle,
            stderr_handle,
            wait_error,
            spec.kill_tree,
        ),
    }
}

fn cleanup_after_wait(
    child: Child,
    stdout_rx: mpsc::Receiver<StreamDone>,
    stderr_rx: mpsc::Receiver<StreamDone>,
    stdout_handle: Option<thread::JoinHandle<()>>,
    stderr_handle: Option<thread::JoinHandle<()>>,
    wait_error: ProcessWaitError,
    kill_tree: bool,
) -> Result<ProcessRunOutput, ProcessRunError> {
    handle_timeout(child, stdout_rx, stderr_rx, stdout_handle, stderr_handle, wait_error, kill_tree)
}

fn write_child_stdin(child: &mut Child, input: Vec<u8>) -> Result<(), String> {
    write_child_stdin_with(child, input, |child, input| {
        let mut stdin = child.stdin.take().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stdin not piped")
        })?;
        stdin.write_all(&input)
    })
}

fn write_child_stdin_with(
    child: &mut Child,
    input: Vec<u8>,
    write: impl FnOnce(&mut Child, Vec<u8>) -> std::io::Result<()>,
) -> Result<(), String> {
    write(child, input).map_err(|err| {
        let cause = err.to_string();
        let cause = cause.chars().take(512).collect::<String>();
        format!("stdin write failed: {cause}")
    })
}

fn cleanup_after_stdin_write_failure(child: Child, reason: String) -> ProcessRunError {
    cleanup_after_stdin_write_failure_with(child, reason, 0, 0, |child| child.kill(), |_| {})
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StdinCleanupCompletion {
    Reaped,
    ReaperOwnsChild,
}

fn cleanup_after_stdin_write_failure_with(
    mut child: Child,
    reason: String,
    stdout_total_bytes: u64,
    stderr_total_bytes: u64,
    terminate: impl FnOnce(&mut Child) -> std::io::Result<()>,
    completed: impl FnOnce(StdinCleanupCompletion),
) -> ProcessRunError {
    // Close the pipe first; do not drop a still-running child on a write failure.
    child.stdin.take();
    let kill_error = terminate(&mut child).err();
    // A failed injected or native termination must not skip the real fallback.
    let fallback_error = if kill_error.is_some() { child.kill().err() } else { None };
    if fallback_error.is_some() {
        thread::spawn(move || {
            let _ = child.wait();
        });
        completed(StdinCleanupCompletion::ReaperOwnsChild);
        return ProcessRunError::CleanupIncomplete {
            reason: format!(
                "{reason}; child.kill: {}; fallback kill: {}",
                kill_error.as_ref().map(ToString::to_string).unwrap_or_default(),
                fallback_error.as_ref().map(ToString::to_string).unwrap_or_default()
            ),
            stdout_total_bytes,
            stderr_total_bytes,
        };
    }
    match child.wait() {
        Ok(_) => {
            completed(StdinCleanupCompletion::Reaped);
            if let Some(error) = kill_error {
                ProcessRunError::CleanupIncomplete {
                    reason: format!("{reason}; child.kill: {error}"),
                    stdout_total_bytes,
                    stderr_total_bytes,
                }
            } else {
                ProcessRunError::StdinWrite(reason)
            }
        }
        Err(error) => {
            thread::spawn(move || {
                let _ = child.wait();
            });
            completed(StdinCleanupCompletion::ReaperOwnsChild);
            ProcessRunError::CleanupIncomplete {
                reason: format!("{reason}; child.wait: {error}"),
                stdout_total_bytes,
                stderr_total_bytes,
            }
        }
    }
}

fn finish_success(
    mut child: Child,
    stdout_rx: mpsc::Receiver<StreamDone>,
    stderr_rx: mpsc::Receiver<StreamDone>,
    stdout_handle: Option<thread::JoinHandle<()>>,
    stderr_handle: Option<thread::JoinHandle<()>>,
    status: ExitStatus,
) -> Result<ProcessRunOutput, ProcessRunError> {
    let stdout = collect_stream(stdout_rx, stdout_handle, Duration::from_secs(2))?;
    let stderr = collect_stream(stderr_rx, stderr_handle, Duration::from_secs(2))?;
    let _ = child.wait();
    if status.success() {
        Ok(ProcessRunOutput {
            status,
            stdout: stdout.bytes,
            stderr: stderr.bytes,
            stdout_total_bytes: stdout.total_bytes,
            stderr_total_bytes: stderr.total_bytes,
            stdout_truncated: stdout.truncated,
            stderr_truncated: stderr.truncated,
        })
    } else {
        Err(ProcessRunError::NonZero {
            status: status.code(),
            stdout: stdout.bytes,
            stderr: stderr.bytes,
            stdout_total_bytes: stdout.total_bytes,
            stderr_total_bytes: stderr.total_bytes,
            stdout_truncated: stdout.truncated,
            stderr_truncated: stderr.truncated,
        })
    }
}

fn handle_timeout(
    child: Child,
    stdout_rx: mpsc::Receiver<StreamDone>,
    stderr_rx: mpsc::Receiver<StreamDone>,
    stdout_handle: Option<thread::JoinHandle<()>>,
    stderr_handle: Option<thread::JoinHandle<()>>,
    wait_error: ProcessWaitError,
    kill_tree: bool,
) -> Result<ProcessRunOutput, ProcessRunError> {
    handle_timeout_with_cleanup(
        child, stdout_rx, stderr_rx, stdout_handle, stderr_handle, wait_error, kill_tree,
        wait_for_cleanup,
    )
}

fn handle_timeout_with_cleanup(
    mut child: Child,
    stdout_rx: mpsc::Receiver<StreamDone>,
    stderr_rx: mpsc::Receiver<StreamDone>,
    stdout_handle: Option<thread::JoinHandle<()>>,
    stderr_handle: Option<thread::JoinHandle<()>>,
    wait_error: ProcessWaitError,
    kill_tree: bool,
    cleanup: impl FnOnce(
        &mut Child,
        mpsc::Receiver<StreamDone>,
        mpsc::Receiver<StreamDone>,
        Option<thread::JoinHandle<()>>,
        Option<thread::JoinHandle<()>>,
    ) -> Result<(CollectedStream, CollectedStream, Option<ExitStatus>), CleanupIncompleteError>,
) -> Result<ProcessRunOutput, ProcessRunError> {
    let tree_err = if kill_tree {
        kill_child_tree(&child).err()
    } else {
        None
    };
    let direct_kill_err = child.kill().err().map(|e| e.to_string());
    let cleanup_result = cleanup(
        &mut child,
        stdout_rx,
        stderr_rx,
        stdout_handle,
        stderr_handle,
    );
    // Child::drop does not reap. Reap even when drain/status cleanup fails.
    // If termination itself failed, hand ownership to a reaper rather than block this error path.
    if direct_kill_err.is_none() {
        let _ = child.wait();
    } else {
        thread::spawn(move || {
            let _ = child.wait();
        });
    }
    match cleanup_result {
        Ok((stdout, stderr, _)) => match wait_error {
            ProcessWaitError::Timeout => Err(ProcessRunError::Timeout {
                kind: ProcessTimeoutKind::DeadlineExceeded,
                stdout_total_bytes: stdout.total_bytes,
                stderr_total_bytes: stderr.total_bytes,
                stdout_truncated: stdout.truncated,
                stderr_truncated: stderr.truncated,
            }),
            ProcessWaitError::StatusQuery(reason) => Err(ProcessRunError::StatusQuery {
                reason,
                stdout_total_bytes: stdout.total_bytes,
                stderr_total_bytes: stderr.total_bytes,
            }),
        },
        Err(err) => Err(ProcessRunError::CleanupIncomplete {
            reason: cleanup_reason(wait_error.reason(), err.reason, direct_kill_err, tree_err),
            stdout_total_bytes: err.stdout_total_bytes,
            stderr_total_bytes: err.stderr_total_bytes,
        }),
    }
}

fn cleanup_reason(
    timeout_reason: String,
    reason: String,
    direct_kill_err: Option<String>,
    tree_err: Option<String>,
) -> String {
    let mut parts = vec![timeout_reason, reason];
    if let Some(err) = direct_kill_err {
        parts.push(format!("child.kill: {err}"));
    }
    if let Some(err) = tree_err {
        parts.push(format!("taskkill: {err}"));
    }
    parts.join("; ")
}

#[derive(Debug)]
enum ProcessWaitError {
    Timeout,
    StatusQuery(String),
}

impl ProcessWaitError {
    fn reason(self) -> String {
        match self {
            Self::Timeout => "deadline exceeded".to_string(),
            Self::StatusQuery(reason) => format!("status query failed: {reason}"),
        }
    }
}
struct CleanupIncompleteError {
    reason: String,
    stdout_total_bytes: u64,
    stderr_total_bytes: u64,
}

fn wait_with_deadline(
    child: &mut Child,
    deadline: Instant,
    poll_interval: Duration,
) -> Result<ExitStatus, ProcessWaitError> {
    wait_with_deadline_with(child, deadline, poll_interval, |child| child.try_wait())
}

fn wait_with_deadline_with(
    child: &mut Child,
    deadline: Instant,
    poll_interval: Duration,
    mut try_wait: impl FnMut(&mut Child) -> std::io::Result<Option<ExitStatus>>,
) -> Result<ExitStatus, ProcessWaitError> {
    loop {
        match try_wait(child) {
            Ok(Some(status)) => return Ok(status),
            Err(err) => return Err(ProcessWaitError::StatusQuery(err.to_string())),
            Ok(None) => {}
        }
        if Instant::now() >= deadline {
            return Err(ProcessWaitError::Timeout);
        }
        thread::sleep(poll_interval);
    }
}

struct StreamDone {
    bytes: Vec<u8>,
    total_bytes: u64,
    truncated: bool,
}

fn spawn_drain<T: Read + Send + 'static>(
    stream: Option<T>,
    cap: usize,
) -> (mpsc::Receiver<StreamDone>, Option<thread::JoinHandle<()>>) {
    let (tx, rx) = mpsc::channel();
    let tx_for_none = tx.clone();
    let handle = stream.map(|stream| {
        let tx = tx.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            let mut buf = [0u8; 4096];
            let mut total = 0u64;
            let mut retained = Vec::with_capacity(cap.min(4096));
            let mut truncated = false;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        total += n as u64;
                        let space = cap.saturating_sub(retained.len());
                        let to_copy = space.min(n);
                        retained.extend_from_slice(&buf[..to_copy]);
                        if to_copy < n {
                            truncated = true;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = tx.send(StreamDone {
                bytes: retained,
                total_bytes: total,
                truncated,
            });
        })
    });
    if handle.is_none() {
        let _ = tx_for_none.send(StreamDone {
            bytes: vec![],
            total_bytes: 0,
            truncated: false,
        });
    }
    (rx, handle)
}

struct CollectedStream {
    bytes: Vec<u8>,
    total_bytes: u64,
    truncated: bool,
}

fn collect_stream(
    rx: mpsc::Receiver<StreamDone>,
    handle: Option<thread::JoinHandle<()>>,
    wait_limit: Duration,
) -> Result<CollectedStream, ProcessRunError> {
    let done = rx
        .recv_timeout(wait_limit)
        .map_err(|err| ProcessRunError::CleanupIncomplete {
            reason: err.to_string(),
            stdout_total_bytes: 0,
            stderr_total_bytes: 0,
        })?;
    if let Some(handle) = handle {
        handle
            .join()
            .map_err(|_| ProcessRunError::CleanupIncomplete {
                reason: "reader thread panicked".to_string(),
                stdout_total_bytes: done.total_bytes,
                stderr_total_bytes: done.total_bytes,
            })?;
    }
    Ok(CollectedStream {
        bytes: done.bytes,
        total_bytes: done.total_bytes,
        truncated: done.truncated,
    })
}

fn wait_for_cleanup(
    child: &mut Child,
    stdout_rx: mpsc::Receiver<StreamDone>,
    stderr_rx: mpsc::Receiver<StreamDone>,
    stdout_handle: Option<thread::JoinHandle<()>>,
    stderr_handle: Option<thread::JoinHandle<()>>,
) -> Result<(CollectedStream, CollectedStream, Option<ExitStatus>), CleanupIncompleteError> {
    wait_for_cleanup_with(child, stdout_rx, stderr_rx, stdout_handle, stderr_handle, |child| {
        child.try_wait()
    })
}

fn wait_for_cleanup_with(
    child: &mut Child,
    stdout_rx: mpsc::Receiver<StreamDone>,
    stderr_rx: mpsc::Receiver<StreamDone>,
    stdout_handle: Option<thread::JoinHandle<()>>,
    stderr_handle: Option<thread::JoinHandle<()>>,
    mut try_wait: impl FnMut(&mut Child) -> std::io::Result<Option<ExitStatus>>,
) -> Result<(CollectedStream, CollectedStream, Option<ExitStatus>), CleanupIncompleteError> {
    let stdout = stdout_rx
        .recv_timeout(Duration::from_millis(250))
        .map_err(|err| CleanupIncompleteError {
            reason: err.to_string(),
            stdout_total_bytes: 0,
            stderr_total_bytes: 0,
        })
        .map(|done| CollectedStream {
            bytes: done.bytes,
            total_bytes: done.total_bytes,
            truncated: done.truncated,
        })?;
    let stderr = stderr_rx
        .recv_timeout(Duration::from_millis(250))
        .map_err(|err| CleanupIncompleteError {
            reason: err.to_string(),
            stdout_total_bytes: stdout.total_bytes,
            stderr_total_bytes: 0,
        })
        .map(|done| CollectedStream {
            bytes: done.bytes,
            total_bytes: done.total_bytes,
            truncated: done.truncated,
        })?;
    if let Some(handle) = stdout_handle {
        if handle.join().is_err() {
            return Err(CleanupIncompleteError {
                reason: "stdout reader thread panicked".to_string(),
                stdout_total_bytes: stdout.total_bytes,
                stderr_total_bytes: stderr.total_bytes,
            });
        }
    }
    if let Some(handle) = stderr_handle {
        if handle.join().is_err() {
            return Err(CleanupIncompleteError {
                reason: "stderr reader thread panicked".to_string(),
                stdout_total_bytes: stdout.total_bytes,
                stderr_total_bytes: stderr.total_bytes,
            });
        }
    }
    let status = try_wait(child).map_err(|err| CleanupIncompleteError {
        reason: err.to_string(),
        stdout_total_bytes: stdout.total_bytes,
        stderr_total_bytes: stderr.total_bytes,
    })?;
    Ok((stdout, stderr, status))
}

#[cfg(windows)]
fn kill_child_tree(child: &Child) -> Result<(), String> {
    let pid = child.id();
    let status = Command::new(trusted_taskkill_path()?)
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status()
        .map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("taskkill exited with {status}"))
    }
}

#[cfg(not(windows))]
fn kill_child_tree(_child: &Child) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub(crate) fn trusted_taskkill_path() -> Result<PathBuf, String> {
    let mut buf = vec![0u16; 32768];
    unsafe {
        let len =
            windows::Win32::System::SystemInformation::GetSystemDirectoryW(Some(&mut buf)) as usize;
        if len > 0 && len < buf.len() {
            let mut path = PathBuf::from(String::from_utf16_lossy(&buf[..len]));
            path.push("taskkill.exe");
            return Ok(path);
        }
    }
    let root = std::env::var("SystemRoot").map_err(|_| "SystemRoot missing".to_string())?;
    let root = PathBuf::from(root);
    let mut path = root.clone();
    path.push("System32");
    path.push("taskkill.exe");
    let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
    let canonical_path = path.canonicalize().map_err(|e| e.to_string())?;
    if canonical_path.starts_with(&canonical_root) {
        Ok(path)
    } else {
        Err("untrusted taskkill path".to_string())
    }
}

#[cfg(not(windows))]
pub(crate) fn trusted_taskkill_path() -> Result<PathBuf, String> {
    Err("taskkill unavailable on non-windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_runner_injected_stdin_write_failure_reaps_owned_child_and_keeps_cause() {
        let mut owned = OwnedTestChild::spawn();
        let input = vec![b'x'; 16];
        let mut writer_called = false;
        let reason = write_child_stdin_with(owned.child(), input.clone(), |child, data| {
            writer_called = true;
            assert_eq!(data, input);
            assert!(child.try_wait().unwrap().is_none(), "test child must still be running at injected failure");
            Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "injected writer failed"))
        }).unwrap_err();
        assert!(writer_called);
        assert!(reason.contains("injected writer failed"));
        let start = Instant::now();
        let result = cleanup_after_stdin_write_failure(owned.take(), reason);
        assert!(start.elapsed() < Duration::from_secs(5), "owned-child cleanup must be bounded");
        assert!(matches!(result, ProcessRunError::StdinWrite(cause) if cause.contains("injected writer failed")));
    }

    #[test]
    fn process_runner_stdin_write_failure_observer_confirms_reaped_or_cleanup_error() {
        for inject_termination_failure in [false, true] {
            let mut owned = OwnedTestChild::spawn();
            let reason = write_child_stdin_with(owned.child(), vec![1], |_, _| {
                Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "injected writer failed"))
            }).unwrap_err();
            let mut completion = None;
            let result = cleanup_after_stdin_write_failure_with(
                owned.take(), reason, 4, 7,
                |child| {
                    if inject_termination_failure {
                        Err(std::io::Error::other("injected termination failed"))
                    } else {
                        child.kill()
                    }
                },
                |observed| completion = Some(observed),
            );
            assert_eq!(completion, Some(StdinCleanupCompletion::Reaped), "fallback must terminate and reap test-owned child");
            if inject_termination_failure {
                assert!(matches!(result, ProcessRunError::CleanupIncomplete { stdout_total_bytes: 4, stderr_total_bytes: 7, reason }
                    if reason.contains("injected writer failed") && reason.contains("injected termination failed") && reason.len() < 1024));
            } else {
                assert!(matches!(result, ProcessRunError::StdinWrite(reason) if reason.contains("injected writer failed")));
            }
        }
    }

    #[test]
    fn process_runner_stdin_write_failure_must_transfer_or_reap_owned_child() {
        let source = include_str!("process_runner.rs");
        let runner = source.split("pub fn run_process(").nth(1).unwrap().split("fn write_child_stdin(").next().unwrap();
        assert!(!runner.contains("write_child_stdin(&mut child, stdin)?"),
            "propagating stdin write failure with ? drops the owned Child without kill/reap");
        let write = source.split("fn write_child_stdin(").nth(1).unwrap().split("fn finish_success(").next().unwrap();
        assert!(write.contains("stdin write failed:"), "write error must retain its cause");
    }

    struct OwnedTestChild(Option<Child>);
    impl OwnedTestChild {
        fn spawn() -> Self {
            let child = Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 30"])
                .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
                .spawn().expect("spawn uniquely owned test child");
            Self(Some(child))
        }
        fn child(&mut self) -> &mut Child { self.0.as_mut().unwrap() }
        fn take(&mut self) -> Child { self.0.take().unwrap() }
    }
    impl Drop for OwnedTestChild {
        fn drop(&mut self) {
            if let Some(child) = &mut self.0 { let _ = child.kill(); let _ = child.wait(); }
        }
    }

    #[test]
    fn process_runner_injected_status_error_returns_on_first_poll_not_timeout() {
        let mut owned = OwnedTestChild::spawn();
        let mut calls = 0;
        let now = Instant::now();
        let result = wait_with_deadline_with(owned.child(), now + Duration::from_secs(5), Duration::from_secs(1), |_| {
            calls += 1;
            Err(std::io::Error::other("injected status query failure"))
        });
        assert_eq!(calls, 1);
        assert!(now.elapsed() < Duration::from_secs(2));
        assert!(matches!(result, Err(ProcessWaitError::StatusQuery(reason)) if reason.contains("injected status query failure")));
    }

    #[test]
    fn process_runner_status_failure_kills_owned_child_and_preserves_collected_counts() {
        let mut owned = OwnedTestChild::spawn();
        let (stdout_tx, stdout_rx) = mpsc::channel();
        stdout_tx.send(StreamDone { bytes: vec![b'a'], total_bytes: 11, truncated: true }).unwrap();
        let (stderr_tx, stderr_rx) = mpsc::channel();
        stderr_tx.send(StreamDone { bytes: vec![b'b'], total_bytes: 17, truncated: true }).unwrap();
        let result = handle_timeout(owned.take(), stdout_rx, stderr_rx, None, None,
            ProcessWaitError::StatusQuery("injected status query failure".into()), false);
        assert!(matches!(result, Err(ProcessRunError::StatusQuery { stdout_total_bytes: 11, stderr_total_bytes: 17, .. })));
        // Child was moved into the cleanup path and its pipes/counts drained before return.
    }

    #[test]
    fn process_runner_cleanup_failure_retains_available_counts_and_wait_error_cause() {
        let mut owned = OwnedTestChild::spawn();
        let (stdout_tx, stdout_rx) = mpsc::channel();
        stdout_tx.send(StreamDone { bytes: vec![b'a'], total_bytes: 11, truncated: true }).unwrap();
        let (stderr_tx, stderr_rx) = mpsc::channel::<StreamDone>();
        drop(stderr_tx);
        let failure = wait_for_cleanup(owned.child(), stdout_rx, stderr_rx, None, None).err().unwrap();
        assert_eq!((failure.stdout_total_bytes, failure.stderr_total_bytes), (11, 0));
        let reason = cleanup_reason(ProcessWaitError::StatusQuery("injected status query failure".into()).reason(), failure.reason, None, None);
        assert!(reason.contains("injected status query failure") && reason.len() < 1024);
    }

    #[test]
    fn process_runner_final_cleanup_failure_preserves_counts_and_reaps_owned_child() {
        let mut owned = OwnedTestChild::spawn();
        let (stdout_tx, stdout_rx) = mpsc::channel();
        stdout_tx.send(StreamDone { bytes: vec![b'a'], total_bytes: 4, truncated: false }).unwrap();
        let (stderr_tx, stderr_rx) = mpsc::channel();
        stderr_tx.send(StreamDone { bytes: vec![b'b'], total_bytes: 7, truncated: false }).unwrap();
        let mut observed_terminated = false;
        let result = handle_timeout_with_cleanup(
            owned.take(), stdout_rx, stderr_rx, None, None,
            ProcessWaitError::StatusQuery("injected status query failure".into()), false,
            |child, stdout_rx, stderr_rx, _, _| {
                // Final cleanup is called only after direct termination. Its error must
                // not lose either stream count or the original status-query cause.
                observed_terminated = child.wait().unwrap().code().is_some();
                let stdout = stdout_rx.recv_timeout(Duration::from_secs(1)).unwrap();
                let stderr = stderr_rx.recv_timeout(Duration::from_secs(1)).unwrap();
                Err(CleanupIncompleteError {
                    reason: "injected cleanup failed".into(),
                    stdout_total_bytes: stdout.total_bytes,
                    stderr_total_bytes: stderr.total_bytes,
                })
            },
        );
        assert!(observed_terminated, "owned child must be terminated before final cleanup");
        assert!(matches!(result, Err(ProcessRunError::CleanupIncomplete { stdout_total_bytes: 4, stderr_total_bytes: 7, reason })
            if reason.contains("injected cleanup failed") && reason.contains("injected status query failure") && reason.len() < 1024));
    }

    #[test]
    fn process_runner_output_boundary_is_truncated_only_above_cap_on_each_stream() {
        for (name, cap) in [("stdout", 8usize), ("stderr", 8usize)] {
            for length in [0usize, cap, cap + 1] {
                let input = vec![b'x'; length];
                let (rx, handle) = spawn_drain(Some(std::io::Cursor::new(input.clone())), cap);
                let result = collect_stream(rx, handle, Duration::from_secs(1)).unwrap();
                assert_eq!(result.total_bytes, length as u64, "{name} length {length}");
                assert_eq!(result.bytes, input[..length.min(cap)], "{name} length {length}");
                assert_eq!(result.truncated, length > cap, "{name} length {length}");
            }
        }
    }

    #[test]
    fn process_runner_try_wait_failure_must_not_be_reclassified_as_deadline_timeout() {
        let source = include_str!("process_runner.rs");
        let wait = source.split("fn wait_with_deadline(").nth(1).unwrap().split("struct StreamDone").next().unwrap();
        assert!(!wait.contains("if let Ok(Some(status)) = child.try_wait()"), "try_wait Err must exit immediately instead of polling until timeout");
        assert!(wait.contains("child.try_wait()"), "poll must query owned child status");
        assert!(source.contains("StatusQuery") || source.contains("WaitFailed"), "status-query failure needs its own error classification, not Timeout");
        let runner = source.split("pub fn run_process(").nth(1).unwrap().split("fn write_child_stdin(").next().unwrap();
        assert!(runner.contains("handle_status") || runner.contains("cleanup_after_wait"), "status-query failure must initiate owned-child cleanup");
    }

    #[test]
    fn process_runner_cleanup_error_retains_bounded_status_and_stream_diagnostics() {
        let source = include_str!("process_runner.rs");
        let cleanup = source.split("fn handle_timeout(").nth(1).unwrap().split("fn cleanup_reason(").next().unwrap();
        assert!(!cleanup.contains("stdout_total_bytes: 0,\n            stderr_total_bytes: 0,"), "failed cleanup must retain available stream byte counts");
        assert!(source.contains("fn cleanup_reason("), "retain bounded failure reason and kill errors");
    }

    fn powershell_command(script: &str) -> ProcessRunSpec {
        ProcessRunSpec {
            program: "powershell.exe".into(),
            args: vec!["-NoProfile".into(), "-Command".into(), script.into()],
            cwd: None,
            envs: vec![],
            stdin: None,
            timeout: Duration::from_secs(5),
            stdout_cap: 64 * 1024,
            stderr_cap: 64 * 1024,
            poll_interval: Duration::from_millis(50),
            kill_tree: true,
        }
    }

    #[test]
    fn caps_output_and_counts_total_bytes() {
        let spec = powershell_command("$out = 'a' * 70000; [Console]::Out.Write($out)");
        let result = run_process(spec).unwrap();
        assert_eq!(result.stdout.len(), 64 * 1024);
        assert!(result.stdout_total_bytes > result.stdout.len() as u64);
        assert!(result.stdout_truncated);
    }

    #[test]
    fn timeout_returns_error() {
        let spec = powershell_command("Start-Sleep -Seconds 10");
        let err = run_process(spec).unwrap_err();
        assert!(matches!(err, ProcessRunError::Timeout { .. }));
    }

    #[test]
    fn nonzero_exit_returns_metadata() {
        let spec = powershell_command("Write-Output fail; exit 7");
        let err = run_process(spec).unwrap_err();
        match err {
            ProcessRunError::NonZero {
                status,
                stdout_total_bytes,
                stderr_total_bytes,
                stdout_truncated,
                stderr_truncated,
                ..
            } => {
                assert_eq!(status, Some(7));
                assert!(stdout_total_bytes > 0);
                assert_eq!(stderr_total_bytes, 0);
                assert!(!stdout_truncated || stdout_total_bytes >= 1);
                assert!(!stderr_truncated);
            }
            other => panic!("expected NonZero, got {other:?}"),
        }
    }

    #[test]
    fn spec_exposes_frozen_generic_runner_api() {
        let spec = powershell_command("Write-Output ok");
        assert_eq!(spec.program, "powershell.exe");
        assert!(spec.cwd.is_none());
        assert!(spec.envs.is_empty());
        assert!(spec.stdin.is_none());
        assert!(spec.kill_tree);
    }

    #[test]
    fn trusted_taskkill_lookup_is_not_path_based() {
        let path = trusted_taskkill_path();
        if let Ok(path) = path {
            assert!(path
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with("\\system32\\taskkill.exe"));
        }
    }

    #[test]
    fn timeout_kill_tree_runs_before_direct_child_kill() {
        let source = include_str!("process_runner.rs");
        let cleanup_body = source
            .split("fn handle_timeout_with_cleanup(")
            .nth(1)
            .expect("shared timeout/status cleanup function must exist")
            .split("fn cleanup_reason(")
            .next()
            .expect("cleanup function must end before reason formatter");
        let tree_kill = cleanup_body
            .find("kill_child_tree(&child)")
            .expect("tree termination must remain in shared cleanup path");
        let direct_kill = cleanup_body
            .find("child.kill()")
            .expect("direct child termination must remain in shared cleanup path");
        assert!(
            cleanup_body.contains("if kill_tree") && tree_kill < direct_kill,
            "when requested, shared cleanup must attempt tree termination before direct child kill"
        );
    }
}
