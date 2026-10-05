//! Private process-isolated STA owner. Only image stdin is binary; stdout is metadata.
//! Whole-child termination bounds an uninterruptible OLE call without killing a thread.
use super::{clipboard::{self, Failpoints, PublicationStage, Publisher, ShutdownOutcome}, geometry::{checked_rgba_len, CroppedImage, MAX_STAGING_BYTES}};
use serde_json::{json, Value};
use std::{io::{Read, Write}, os::windows::io::AsRawHandle, path::Path, process::{Child, ChildStdin, ChildStdout, Command, Stdio}, sync::mpsc, thread::JoinHandle, time::{Duration, Instant}};
use windows::Win32::{Foundation::{CloseHandle, HANDLE}, System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject, JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE}};

const MAGIC: &[u8; 8] = b"JSCLIP01";
const HEADER: usize = 40;
pub const OPERATION_TIMEOUT: Duration = Duration::from_secs(2);
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(500);
const EXIT_TIMEOUT: Duration = Duration::from_millis(250);

#[link(name = "kernel32")]
unsafe extern "system" {
    fn PeekNamedPipe(pipe: *mut std::ffi::c_void, buffer: *mut std::ffi::c_void, size: u32, read: *mut u32, available: *mut u32, remaining: *mut u32) -> i32;
    fn GetCurrentProcess() -> isize;
    fn GetProcessHandleCount(process: isize, count: *mut u32) -> i32;
    fn CancelSynchronousIo(thread: *mut std::ffi::c_void) -> i32;
}
#[link(name = "user32")]
unsafe extern "system" { fn GetGuiResources(process: isize, flags: u32) -> u32; }

struct Job(HANDLE);
impl Drop for Job { fn drop(&mut self) { unsafe { let _ = CloseHandle(self.0); } } }

/// All outcomes distinguish commitment from durability. Unknown never promises
/// old-content preservation, even if the last helper message preceded commitment.
#[derive(Clone, Debug)]
pub enum Outcome { Rejected(String), Committed { durable: bool }, Unknown, DurabilityLost, Superseded }
impl Outcome {
    pub fn metadata(&self) -> Value { match self {
        Self::Rejected(code) => json!({"status":"rejected","code":code,"committed":false,"durable":false}),
        Self::Committed { durable: true } => json!({"status":"committed","committed":true,"durable":true}),
        Self::Committed { durable: false } => json!({"status":"committed-warning","code":"clipboard-committed-not-durable","committed":true,"durable":false}),
        Self::Unknown => json!({"status":"publication-unknown","code":"clipboard-publication-unknown","committed":null,"durable":null}),
        Self::DurabilityLost => json!({"status":"committed-warning","code":"clipboard-durability-lost","committed":true,"durable":false}),
        Self::Superseded => json!({"status":"committed-warning","code":"clipboard-owner-superseded","committed":true,"durable":false}),
    } }
}

#[derive(Debug)]
pub struct ShutdownReport { pub outcome: Outcome, pub forced: bool, pub exited: bool, pub writer_drained: bool }

/// Keep this single owner for app lifetime; &mut serializes commands (no queue).
/// Program is a Rust-selected bundled helper path, never a renderer-supplied path.
pub struct ProcessOwner {
    child: Child, job: Option<Job>, stdin: Option<ChildStdin>, stdout: ChildStdout,
    writer: Option<JoinHandle<Result<ChildStdin, ()>>>, line: Vec<u8>,
    commitment: Option<bool>, durable: bool, stopped: bool, forced_termination: bool, superseded: bool,
    redundant_flush_fault_armed: bool, redundant_flush_fault_used: bool,
}
impl ProcessOwner {
    pub fn spawn(program: &Path) -> Result<Self, &'static str> {
        let mut child = Command::new(program).arg("--internal-snip-clipboard-owner")
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|_| "clipboard-helper-spawn")?;
        let job = match bind_job(&child) {
            Ok(job) => job,
            Err(error) => { let _ = child.kill(); return Err(error); }
        };
        let stdin = child.stdin.take().ok_or("clipboard-helper-stdin")?;
        let stdout = child.stdout.take().ok_or("clipboard-helper-stdout")?;
        let mut owner = Self { child, job: Some(job), stdin: Some(stdin), stdout, writer: None,
            line: Vec::with_capacity(4096), commitment: Some(false), durable: false, stopped: false, forced_termination: false, superseded: false,
            redundant_flush_fault_armed: false, redundant_flush_fault_used: false };
        let value = owner.receive(Instant::now() + OPERATION_TIMEOUT)?;
        if value != json!({"protocol":"snipping-clipboard-owner-v1","ready":true}) { return Err("clipboard-helper-handshake"); }
        Ok(owner)
    }
    pub fn publish(&mut self, image: CroppedImage, retained: usize, fail: Failpoints) -> Outcome {
        if self.stopped { return Outcome::Rejected("clipboard-owner-stopped".into()); }
        if self.commitment == Some(true) && !self.durable { return Outcome::Rejected("clipboard-owner-undurable".into()); }
        // Parent plus helper input coexist during pipe transfer. No third payload copy.
        if retained < image.pixels.capacity() || retained.checked_add(image.pixels.len()).is_none_or(|bytes| bytes > MAX_STAGING_BYTES) {
            return Outcome::Rejected("clipboard-budget".into());
        }
        let flags = u8::from(fail.allocation) | (u8::from(fail.set)<<1) | (u8::from(fail.flush)<<2)
            | (u8::from(fail.stall_before_set)<<3) | (u8::from(fail.stall_after_commit)<<4) | (u8::from(fail.stall_shutdown)<<5);
        let header = header(1, flags, image.width, image.height, retained as u64, image.pixels.len() as u64);
        self.redundant_flush_fault_armed = false; self.redundant_flush_fault_used = false;
        self.commitment = None; self.durable = false; self.superseded = false;
        let deadline = Instant::now() + OPERATION_TIMEOUT;
        if self.send(header, Some(image)).is_err() { return self.timeout(); }
        self.operation(deadline)
    }
    pub fn flush(&mut self) -> Outcome {
        // Durability is an acknowledged historical fact, independent of helper
        // reachability or retirement. Never issue redundant transport/native work.
        if self.commitment == Some(true) && self.durable { return Outcome::Committed { durable: true }; }
        if self.stopped { return if self.commitment == Some(true) { Outcome::DurabilityLost } else { Outcome::Unknown }; }
        if self.commitment != Some(true) { return Outcome::Rejected("clipboard-no-committed-owner".into()); }
        let deadline = Instant::now() + OPERATION_TIMEOUT;
        if self.send(header(2, 0, 0, 0, 0, 0), None).is_err() { return self.timeout(); }
        self.operation(deadline)
    }
    pub fn metrics(&mut self) -> Result<Value, &'static str> {
        self.send(header(3, 0, 0, 0, 0, 0), None)?;
        let result = self.receive(Instant::now() + OPERATION_TIMEOUT);
        self.reclaim_writer();
        if result.is_err() { self.terminate(); }
        result
    }
    fn operation(&mut self, deadline: Instant) -> Outcome {
        loop {
            let value = match self.receive(deadline) { Ok(value) => value, Err(_) => return self.timeout() };
            match value["stage"].as_str() {
                Some("publishing") => continue,
                Some("committed") => { self.commitment = Some(true); continue; }
                _ => {}
            }
            self.reclaim_writer();
            match value["status"].as_str() {
                Some("rejected") if self.commitment != Some(true) => {
                    self.commitment = Some(false);
                    return Outcome::Rejected(value["code"].as_str().unwrap_or("clipboard-rejected").to_string());
                }
                Some("committed") => { self.commitment = Some(true); self.durable = true; return Outcome::Committed { durable: true }; }
                Some("committed-warning") => { self.commitment = Some(true); return Outcome::Committed { durable: false }; }
                _ => return self.timeout(),
            }
        }
    }
    fn timeout(&mut self) -> Outcome {
        let outcome = if self.commitment == Some(true) && self.durable { Outcome::Committed { durable: true } }
            else if self.commitment == Some(true) { Outcome::DurabilityLost } else { Outcome::Unknown };
        self.terminate(); outcome
    }
    pub fn shutdown(&mut self) -> ShutdownReport {
        if self.stopped { self.reclaim_writer(); return ShutdownReport { outcome: self.last_outcome(), forced: self.forced_termination, exited: self.child.try_wait().ok().flatten().is_some(), writer_drained: self.writer.is_none() }; }
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
        let mut forced = self.send(header(4, 0, 0, 0, 0, 0), None).is_err();
        if !forced {
            match self.receive(deadline) {
                Ok(value) if value["shutdown"] == "durable" => { self.durable = true; }
                Ok(value) if value["shutdown"] == "superseded" => { self.superseded = true; }
                Ok(value) if value["shutdown"] == "durability-lost" => {}
                _ => forced = true,
            }
        }
        while !forced && Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() { break; }
            std::thread::sleep(Duration::from_millis(5));
        }
        if self.child.try_wait().ok().flatten().is_none() { forced = true; }
        let exited = if forced { self.terminate() } else { self.stopped = true; self.reclaim_writer(); self.job.take(); true };
        ShutdownReport { outcome: self.last_outcome(), forced, exited, writer_drained: self.writer.is_none() }
    }
    fn last_outcome(&self) -> Outcome {
        if self.commitment == Some(true) && self.durable { return Outcome::Committed { durable: true }; }
        if self.superseded && self.commitment == Some(true) { return Outcome::Superseded; }
        match (self.commitment, self.durable) {
            (Some(true), true) => Outcome::Committed { durable: true },
            (Some(true), false) => Outcome::DurabilityLost,
            (Some(false), _) => Outcome::Rejected("clipboard-not-current-owner".into()),
            (None, _) => Outcome::Unknown,
        }
    }
    fn terminate(&mut self) -> bool {
        // Retained Child/Job handles only; never OpenProcess or lookup by PID.
        self.stopped = true;
        self.forced_termination = true;
        if let Some(writer) = &self.writer {
            // Cancel only our pipe writer. It owns no COM state or clipboard data.
            unsafe { let _ = CancelSynchronousIo(writer.as_raw_handle()); }
        }
        self.job.take(); // KILL_ON_JOB_CLOSE covers private helper even if kill fails.
        let _ = self.child.kill();
        self.stdin.take();
        let deadline = Instant::now() + EXIT_TIMEOUT;
        loop {
            self.reclaim_writer();
            if self.child.try_wait().ok().flatten().is_some() && self.writer.is_none() { return true; }
            if Instant::now() >= deadline { return false; }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn send(&mut self, header: [u8; HEADER], image: Option<CroppedImage>) -> Result<(), &'static str> {
        if header[8] == 2 && self.redundant_flush_fault_armed {
            self.redundant_flush_fault_used = true;
            return Err("clipboard-test-redundant-flush-response-unavailable");
        }
        let deadline = Instant::now() + Duration::from_millis(25);
        loop {
            self.reclaim_writer();
            if self.writer.is_none() || Instant::now() >= deadline { break; }
            std::thread::yield_now();
        }
        if self.writer.is_some() { return Err("clipboard-command-pending"); }
        let mut stdin = self.stdin.take().ok_or("clipboard-helper-closed")?;
        self.writer = Some(std::thread::Builder::new().name("snip-clipboard-pipe-write".into()).spawn(move || {
            stdin.write_all(&header).map_err(|_| ())?;
            if let Some(image) = image { stdin.write_all(&image.pixels).map_err(|_| ())?; }
            stdin.flush().map_err(|_| ())?;
            Ok(stdin)
        }).map_err(|_| "clipboard-pipe-worker")?);
        Ok(())
    }
    fn reclaim_writer(&mut self) {
        if self.writer.as_ref().is_some_and(|writer| writer.is_finished()) {
            if let Some(writer) = self.writer.take() { if let Ok(Ok(stdin)) = writer.join() { if !self.stopped { self.stdin = Some(stdin); } } }
        }
    }
    fn receive(&mut self, deadline: Instant) -> Result<Value, &'static str> {
        loop {
            if let Some(index) = self.line.iter().position(|byte| *byte == b'\n') {
                let result = serde_json::from_slice(&self.line[..index]).map_err(|_| "clipboard-helper-report");
                self.line.drain(..=index); return result;
            }
            if Instant::now() >= deadline { return Err("clipboard-helper-timeout"); }
            let mut available = 0u32;
            let ok = unsafe { PeekNamedPipe(self.stdout.as_raw_handle(), std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut available, std::ptr::null_mut()) };
            if ok == 0 { return Err("clipboard-helper-disconnected"); }
            if available > 0 {
                let mut bytes = [0u8; 512];
                let count = (available as usize).min(bytes.len());
                let read = self.stdout.read(&mut bytes[..count]).map_err(|_| "clipboard-helper-read")?;
                if read == 0 || self.line.len() + read > 4096 { return Err("clipboard-helper-report-budget"); }
                self.line.extend_from_slice(&bytes[..read]);
            } else { self.reclaim_writer(); std::thread::sleep(Duration::from_millis(5)); }
        }
    }
    /// Narrow opt-in harness dependency fault. It cannot alter initial publication
    /// or binary protocol validation. Normal flush must bypass it when durable.
    pub fn arm_redundant_flush_response_fault_for_test(&mut self) -> Result<(), &'static str> {
        if self.commitment != Some(true) || !self.durable { return Err("clipboard-test-requires-confirmed-durability"); }
        self.redundant_flush_fault_armed = true;
        Ok(())
    }
    pub fn redundant_flush_response_fault_used_for_test(&self) -> bool { self.redundant_flush_fault_used }
}
impl Drop for ProcessOwner { fn drop(&mut self) { if !self.stopped { let result = self.shutdown(); if matches!(result.outcome, Outcome::DurabilityLost | Outcome::Unknown) { eprintln!("clipboard-owner-shutdown: publication unknown or durability lost"); } } } }

fn bind_job(child: &Child) -> Result<Job, &'static str> {
    let job = Job(unsafe { CreateJobObjectW(None, None) }.map_err(|_| "clipboard-job-create")?);
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    unsafe {
        SetInformationJobObject(job.0, JobObjectExtendedLimitInformation, (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(), std::mem::size_of_val(&info) as u32).map_err(|_| "clipboard-job-configure")?;
        AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle())).map_err(|_| "clipboard-job-assign")?;
    }
    Ok(job)
}
fn header(op: u8, flags: u8, width: u32, height: u32, retained: u64, len: u64) -> [u8; HEADER] {
    let mut bytes = [0u8; HEADER]; bytes[..8].copy_from_slice(MAGIC); bytes[8] = op; bytes[9] = flags;
    bytes[12..16].copy_from_slice(&width.to_le_bytes()); bytes[16..20].copy_from_slice(&height.to_le_bytes());
    bytes[20..28].copy_from_slice(&retained.to_le_bytes()); bytes[28..36].copy_from_slice(&len.to_le_bytes()); bytes
}
struct Request { op: u8, flags: u8, image: Option<CroppedImage>, retained: usize }
fn read_request(input: &mut impl Read) -> Result<Request, ()> {
    let mut bytes = [0u8; HEADER]; input.read_exact(&mut bytes).map_err(|_| ())?;
    if &bytes[..8] != MAGIC || bytes[10..12] != [0,0] || bytes[36..] != [0,0,0,0] || bytes[9] & !63 != 0 { return Err(()); }
    let u32_at = |start| u32::from_le_bytes(bytes[start..start+4].try_into().unwrap_or([0;4]));
    let u64_at = |start| u64::from_le_bytes(bytes[start..start+8].try_into().unwrap_or([0;8]));
    let op = bytes[8]; let retained = usize::try_from(u64_at(20)).map_err(|_| ())?;
    let len = usize::try_from(u64_at(28)).map_err(|_| ())?;
    if op != 1 {
        if !(2..=4).contains(&op) || bytes[9] != 0 || bytes[12..36].iter().any(|byte| *byte != 0) { return Err(()); }
        return Ok(Request { op, flags: 0, image: None, retained: 0 });
    }
    let (width, height) = (u32_at(12), u32_at(16));
    // Truncated input is intentionally allowed for invalid-image acceptance; no
    // allocation may exceed valid dimension/product limits or combined input budget.
    let expected = checked_rgba_len(width, height, MAX_STAGING_BYTES).map_err(|_| ())?;
    if width > 16_384 || height > 16_384 || u64::from(width)*u64::from(height) > 33_600_000 || len > expected || len > MAX_STAGING_BYTES { return Err(()); }
    let mut pixels = Vec::new(); pixels.try_reserve_exact(len).map_err(|_| ())?; pixels.resize(len, 0);
    input.read_exact(&mut pixels).map_err(|_| ())?;
    Ok(Request { op, flags: bytes[9], image: Some(CroppedImage { width, height, pixels }), retained })
}
fn emit(value: Value) -> Result<(), ()> {
    let line = value.to_string(); if line.len() > 4096 { return Err(()); }
    let mut stdout = std::io::stdout().lock(); writeln!(stdout, "{line}").map_err(|_| ())?; stdout.flush().map_err(|_| ())
}
pub fn helper_main() -> Result<(), &'static str> {
    let mut publisher = Publisher::new().map_err(|_| "clipboard-helper-sta")?;
    // Only pipe bytes belong to this reader; all COM state remains on this main STA.
    // Single bounded queue; receiving another image requires explicit parent command.
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::Builder::new().name("snip-clipboard-pipe-read".into()).spawn(move || { let mut stdin = std::io::stdin().lock(); while let Ok(request) = read_request(&mut stdin) { if tx.send(request).is_err() { break; } } }).map_err(|_| "clipboard-helper-reader")?;
    emit(json!({"protocol":"snipping-clipboard-owner-v1","ready":true})).map_err(|_| "clipboard-helper-output")?;
    let mut stall_shutdown = false;
    loop {
        clipboard::pump();
        let request = match rx.recv_timeout(Duration::from_millis(5)) {
            Ok(request) => request,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        };
        match request.op {
            1 => {
                let image = request.image.ok_or("clipboard-helper-image")?;
                let flags = request.flags;
                stall_shutdown = flags & 32 != 0;
                let fail = Failpoints { allocation: flags&1 != 0, set: flags&2 != 0, flush: flags&4 != 0,
                    stall_before_set: flags&8 != 0, stall_after_commit: flags&16 != 0, stall_shutdown };
                let retained = request.retained.checked_add(image.pixels.capacity()).ok_or("clipboard-helper-budget")?;
                let result = publisher.publish_observed(&image, retained, fail, |stage| {
                    let _ = emit(json!({"stage": match stage { PublicationStage::Publishing => "publishing", PublicationStage::Committed => "committed" }}));
                });
                let outcome = match result { Ok(durable) => Outcome::Committed { durable },
                    Err(clipboard::Failure::SetUnknown) => Outcome::Unknown, Err(error) => Outcome::Rejected(error.code().into()) };
                emit(outcome.metadata()).map_err(|_| "clipboard-helper-output")?;
            }
            2 => { let durable = publisher.flush().is_ok(); emit(Outcome::Committed { durable }.metadata()).map_err(|_| "clipboard-helper-output")?; }
            3 => {
                let process = unsafe { GetCurrentProcess() }; let mut handles = 0;
                if unsafe { GetProcessHandleCount(process, &mut handles) } == 0 { return Err("clipboard-helper-counts"); }
                let mut counts = serde_json::to_value(clipboard::owned_counts()).map_err(|_| "clipboard-helper-counts")?;
                counts["gdi"] = json!(unsafe { GetGuiResources(process,0) }); counts["user"] = json!(unsafe { GetGuiResources(process,1) }); counts["processHandles"] = json!(handles);
                emit(counts).map_err(|_| "clipboard-helper-output")?;
            }
            4 => {
                if stall_shutdown { loop { std::thread::park(); } }
                let outcome = publisher.shutdown();
                emit(json!({"shutdown": match outcome { ShutdownOutcome::Durable => "durable", ShutdownOutcome::Superseded => "superseded", ShutdownOutcome::DurabilityLost => "durability-lost" }})).map_err(|_| "clipboard-helper-output")?;
                break;
            }
            _ => return Err("clipboard-helper-command"),
        }
    }
    drop(publisher);
    Ok(())
}
