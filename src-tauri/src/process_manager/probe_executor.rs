//! Native-only bounded child executor. No renderer-selected executable or arguments.
use std::os::windows::{io::AsRawHandle, process::CommandExt};
use std::{
    io::Read,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::{JobObjects::*, Pipes::PeekNamedPipe},
};
struct Job(HANDLE);
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
fn bind_job(child: &std::process::Child) -> Result<Job, ProbeError> {
    let job = Job(unsafe { CreateJobObjectW(None, None) }.map_err(|_| ProbeError::Spawn)?);
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    unsafe {
        SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&info) as u32,
        )
        .map_err(|_| ProbeError::Spawn)?;
        AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle()))
            .map_err(|_| ProbeError::Spawn)?;
    }
    Ok(job)
}
fn kill_and_reap(child: &mut std::process::Child, observe: &mut impl FnMut(ProbeEvent)) {
    observe(ProbeEvent::KillRequested(child.id()));
    let _ = child.kill();
    observe(ProbeEvent::BeforeWait(child.id()));
    loop {
        match child.wait() {
            Ok(_) => break,
            Err(_) => {
                let _ = child.kill();
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    observe(ProbeEvent::Reaped(child.id()));
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProbeError {
    Spawn,
    Io,
    TimedOut,
    Cancelled,
    OutputLimit,
    Exit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeEvent {
    Spawned(u32),
    KillRequested(u32),
    BeforeWait(u32),
    Reaped(u32),
}
/// Returns only after the direct child has actually been waited/reaped.
/// If Windows cannot terminate/reap a child this call retains ownership and its
/// caller must retain its budget slot; a deadline is not a claim of OS reaping.
pub fn execute_command(
    command: &mut Command,
    cancelled: impl FnMut() -> bool,
    deadline: Duration,
    max_stdout: usize,
) -> Result<Vec<u8>, ProbeError> {
    execute_command_observed(command, cancelled, deadline, max_stdout, |_| {})
}
/// Native-only observer enables deterministic teardown barriers without a second
/// executor. Production uses the same implementation with a no-op observer.
pub fn execute_command_observed(
    command: &mut Command,
    mut cancelled: impl FnMut() -> bool,
    deadline: Duration,
    max_stdout: usize,
    mut observe: impl FnMut(ProbeEvent),
) -> Result<Vec<u8>, ProbeError> {
    let started = Instant::now();
    if cancelled() {
        return Err(ProbeError::Cancelled);
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|_| ProbeError::Spawn)?;
    let _job = match bind_job(&child) {
        Ok(job) => job,
        Err(error) => {
            kill_and_reap(&mut child, &mut observe);
            return Err(error);
        }
    };
    observe(ProbeEvent::Spawned(child.id()));
    let mut output = Vec::with_capacity(max_stdout.min(8192));
    let mut failure = None;
    let mut exited = None;
    loop {
        if cancelled() {
            failure = Some(ProbeError::Cancelled);
        }
        if started.elapsed() >= deadline {
            failure = Some(ProbeError::TimedOut);
        }
        if failure.is_some() {
            break;
        }
        if let Some(pipe) = child.stdout.as_mut() {
            let mut available = 0;
            // Peek never waits for output. Read only bytes confirmed available.
            if unsafe {
                PeekNamedPipe(
                    HANDLE(pipe.as_raw_handle()),
                    None,
                    0,
                    None,
                    Some(&mut available),
                    None,
                )
            }
            .is_ok()
                && available > 0
            {
                let count = (available as usize)
                    .min(max_stdout.saturating_sub(output.len()).saturating_add(1))
                    .min(8192);
                let mut buffer = vec![0; count];
                match pipe.read(&mut buffer) {
                    Ok(n) => {
                        if n > max_stdout.saturating_sub(output.len()) {
                            failure = Some(ProbeError::OutputLimit);
                            break;
                        }
                        output.extend_from_slice(&buffer[..n]);
                    }
                    Err(_) => {
                        failure = Some(ProbeError::Io);
                        break;
                    }
                }
                if output.len() > max_stdout {
                    failure = Some(ProbeError::OutputLimit);
                    break;
                }
                continue;
            }
        }
        if let Some(status) = exited {
            return if status {
                Ok(output)
            } else {
                Err(ProbeError::Exit)
            };
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                observe(ProbeEvent::BeforeWait(child.id()));
                let _ = child.wait();
                observe(ProbeEvent::Reaped(child.id()));
                exited = Some(status.success());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => {
                failure = Some(ProbeError::Io);
                break;
            }
        }
    }
    // Never report completion or free a probe slot before wait succeeds.
    kill_and_reap(&mut child, &mut observe);
    Err(failure.unwrap_or(ProbeError::Io))
}
