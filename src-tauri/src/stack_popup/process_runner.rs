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
        write_child_stdin(&mut child, stdin)?;
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
        Err(timeout) => handle_timeout(
            child,
            stdout_rx,
            stderr_rx,
            stdout_handle,
            stderr_handle,
            timeout,
            spec.kill_tree,
        ),
    }
}

fn write_child_stdin(child: &mut Child, input: Vec<u8>) -> Result<(), ProcessRunError> {
    match child.stdin.take() {
        Some(mut stdin) => {
            stdin
                .write_all(&input)
                .map_err(|err| ProcessRunError::CleanupIncomplete {
                    reason: format!("stdin write failed: {err}"),
                    stdout_total_bytes: 0,
                    stderr_total_bytes: 0,
                })
        }
        None => Err(ProcessRunError::CleanupIncomplete {
            reason: "stdin not piped".to_string(),
            stdout_total_bytes: 0,
            stderr_total_bytes: 0,
        }),
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
    mut child: Child,
    stdout_rx: mpsc::Receiver<StreamDone>,
    stderr_rx: mpsc::Receiver<StreamDone>,
    stdout_handle: Option<thread::JoinHandle<()>>,
    stderr_handle: Option<thread::JoinHandle<()>>,
    timeout: ProcessTimeoutError,
    kill_tree: bool,
) -> Result<ProcessRunOutput, ProcessRunError> {
    let tree_err = if kill_tree {
        kill_child_tree(&child).err()
    } else {
        None
    };
    let direct_kill_err = child.kill().err().map(|e| e.to_string());
    match wait_for_cleanup(
        &mut child,
        stdout_rx,
        stderr_rx,
        stdout_handle,
        stderr_handle,
    ) {
        Ok((stdout, stderr, _)) => Err(ProcessRunError::Timeout {
            kind: ProcessTimeoutKind::DeadlineExceeded,
            stdout_total_bytes: stdout.total_bytes,
            stderr_total_bytes: stderr.total_bytes,
            stdout_truncated: stdout.truncated,
            stderr_truncated: stderr.truncated,
        }),
        Err(err) => Err(ProcessRunError::CleanupIncomplete {
            reason: cleanup_reason(timeout.reason, err.reason, direct_kill_err, tree_err),
            stdout_total_bytes: 0,
            stderr_total_bytes: 0,
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
struct ProcessTimeoutError {
    reason: String,
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
) -> Result<ExitStatus, ProcessTimeoutError> {
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            return Err(ProcessTimeoutError {
                reason: "deadline exceeded".to_string(),
            });
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
                        if to_copy < n || retained.len() >= cap {
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
    let status = child.try_wait().map_err(|err| CleanupIncompleteError {
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
        let timeout_body = source.split("fn handle_timeout").nth(1).unwrap_or("");
        assert!(
            timeout_body.find("kill_child_tree(&child)").unwrap()
                < timeout_body.find("child.kill()").unwrap()
        );
    }
}
