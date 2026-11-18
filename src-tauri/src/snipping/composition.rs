//! Process-bounded DwmFlush: terminate only our exact child, never a UI thread.
use super::coordinator::SnipError;
use std::{os::windows::io::AsRawHandle, process::{Command, Stdio}, time::{Duration, Instant}};
use windows::Win32::{Foundation::{CloseHandle, HANDLE}, System::JobObjects::*};
struct Job(HANDLE);
impl Drop for Job { fn drop(&mut self) { unsafe { let _ = CloseHandle(self.0); } } }
pub fn helper() -> bool { unsafe { windows::Win32::Graphics::Dwm::DwmFlush() }.is_ok() }
fn reap_owned(child: &mut std::process::Child, job: Option<&Job>) {
    loop {
        if let Some(job) = job { let _termination = unsafe { TerminateJobObject(job.0, 1) }; }
        let _kill = child.kill();
        if matches!(child.try_wait(), Ok(Some(_))) && child.wait().is_ok() {
            let empty = job.map(|job| {
                let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
                unsafe { QueryInformationJobObject(Some(job.0), JobObjectBasicAccountingInformation, (&mut info as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(), std::mem::size_of_val(&info) as u32, None) }.is_ok() && info.ActiveProcesses == 0
            }).unwrap_or(true);
            if empty { return; }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
pub fn flush(deadline: Instant) -> Result<(), SnipError> {
    if Instant::now() >= deadline { return Err(SnipError::ReadinessTimeout); }
    let executable = std::env::current_exe().map_err(|_| SnipError::CaptureFailed)?;
    let mut child = Command::new(executable).arg("--internal-snip-composition-flush").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|_| SnipError::CaptureFailed)?;
    let job = (|| {
        let job = Job(unsafe { CreateJobObjectW(None, None) }.map_err(|_| SnipError::CaptureFailed)?);
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        unsafe {
            SetInformationJobObject(job.0, JobObjectExtendedLimitInformation, (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(), std::mem::size_of_val(&info) as u32).map_err(|_| SnipError::CaptureFailed)?;
            AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle())).map_err(|_| SnipError::CaptureFailed)?;
        }
        Ok::<_, SnipError>(job)
    })();
    let job = match job { Ok(job) => job, Err(error) => { reap_owned(&mut child, None); return Err(error); } };
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if child.wait().is_err() { reap_owned(&mut child, Some(&job)); return Err(SnipError::CaptureFailed); }
                return if status.success() { Ok(()) } else { Err(SnipError::CaptureFailed) };
            },
            Err(_) => { reap_owned(&mut child, Some(&job)); return Err(SnipError::CaptureFailed); },
            _ => {}
        }
        if Instant::now() >= deadline { reap_owned(&mut child, Some(&job)); return Err(SnipError::ReadinessTimeout); }
        std::thread::sleep(Duration::from_millis(2));
    }
}
