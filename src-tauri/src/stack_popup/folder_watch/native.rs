//! One event-driven worker owns all outstanding OVERLAPPED reads. Buffers and
//! OVERLAPPED addresses remain stable until completion, including cancellation.

use super::WatchTarget;
use std::path::{Component, Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING, ERROR_NOTIFY_ENUM_DIR, HANDLE,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED,
    FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_ATTRIBUTES, FILE_NOTIFY_CHANGE_DIR_NAME,
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Threading::{
    CreateEventW, ResetEvent, SetEvent, WaitForMultipleObjects, INFINITE,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

const DEBOUNCE: Duration = Duration::from_millis(200);
const BUFFER_WORDS: usize = 8192; // 32 KiB, DWORD aligned, below the network 64 KiB limit.

// Windows kernel event/directory handles are process-wide, not thread-affine.
// Store their integer representation so ownership can move to the worker without
// unsafe Send/Sync impls. Only the final owner closes a handle.
struct OwnedHandle(isize);
impl OwnedHandle {
    fn raw(&self) -> HANDLE {
        HANDLE(self.0 as *mut std::ffi::c_void)
    }
    fn event() -> Result<Self, String> {
        unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
            .map(|handle| Self(handle.0 as isize))
            .map_err(|error| format!("Failed to create folder watch event: {error}"))
    }
}
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.raw());
        }
    }
}

pub(super) struct ProductionWatch {
    stop_event: Arc<OwnedHandle>,
    worker: Option<JoinHandle<()>>,
    worker_failed: bool,
    #[cfg(test)]
    targets: Vec<PathBuf>,
}

impl ProductionWatch {
    pub(super) fn start(
        targets: Vec<WatchTarget>,
        on_change: Arc<dyn Fn() + Send + Sync>,
        on_error: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        if targets.is_empty() || targets.len() > 3 {
            return Err("Folder watch target limit exceeded".to_string());
        }
        let stop_event = Arc::new(OwnedHandle::event()?);
        let stop = stop_event.clone();
        #[cfg(test)]
        let paths = targets.iter().map(|target| target.path.clone()).collect();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("stack-folder-watch".to_string())
            .spawn(move || {
                let reads = targets
                    .into_iter()
                    .map(DirectoryRead::open)
                    .collect::<Result<Vec<_>, _>>();
                let mut reads = match reads {
                    Ok(reads) => reads,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                for read in &mut reads {
                    if let Err(error) = read.arm() {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                }
                if ready_tx.send(Ok(())).is_err() {
                    return;
                }
                if run(&stop, &mut reads, &on_change).is_err() {
                    on_error();
                }
                // DirectoryRead::drop cancels and waits before freeing either the
                // buffer or OVERLAPPED. No callback survives ProductionWatch::stop.
            })
            .map_err(|error| format!("Failed to start folder watch worker: {error}"))?;
        let mut watch = Self {
            stop_event,
            worker: Some(worker),
            worker_failed: false,
            #[cfg(test)]
            targets: paths,
        };
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(watch),
            Ok(Err(error)) => {
                watch.stop();
                Err(error)
            }
            Err(_) => {
                watch.stop();
                Err("Folder watch worker failed during startup".to_string())
            }
        }
    }

    pub(super) fn signal_stop(&self) {
        unsafe {
            let _ = SetEvent(self.stop_event.raw());
        }
    }
    pub(super) fn stop(&mut self) {
        self.signal_stop();
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                eprintln!("folder watch worker panicked during cleanup");
            }
        }
    }
    pub(super) fn stop_and_drain(&mut self) -> Result<(), String> {
        unsafe { SetEvent(self.stop_event.raw()) }.map_err(|_| "Folder watch cancellation failed")?;
        let deadline = Instant::now() + Duration::from_secs(3);
        while self.worker.as_ref().is_some_and(|worker| !worker.is_finished()) {
            if Instant::now() >= deadline { return Err("Folder watch still draining; native worker retained".into()); }
            thread::sleep(Duration::from_millis(10));
        }
        if let Some(worker) = self.worker.take() { if worker.join().is_err() { self.worker_failed = true; } }
        if self.worker_failed { return Err("Folder watch worker completion failed".into()); }
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn targets(&self) -> Vec<PathBuf> {
        self.targets.clone()
    }
}
impl Drop for ProductionWatch {
    fn drop(&mut self) {
        self.stop();
    }
}

struct DirectoryRead {
    target: WatchTarget,
    directory: OwnedHandle,
    event: OwnedHandle,
    overlapped: Box<OVERLAPPED>,
    buffer: Box<[u32; BUFFER_WORDS]>,
    pending: bool,
}

impl DirectoryRead {
    fn open(target: WatchTarget) -> Result<Self, String> {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = target
            .path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let directory = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_LIST_DIRECTORY.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                None,
            )
        }
        .map_err(|error| format!("Failed to open folder notifications: {error}"))?;
        let directory = OwnedHandle(directory.0 as isize);
        let event = OwnedHandle::event()?;
        Ok(Self {
            target,
            directory,
            event,
            overlapped: Box::default(),
            buffer: Box::new([0; BUFFER_WORDS]),
            pending: false,
        })
    }

    fn arm(&mut self) -> Result<(), String> {
        unsafe { ResetEvent(self.event.raw()) }.map_err(|error| error.to_string())?;
        *self.overlapped = OVERLAPPED {
            hEvent: self.event.raw(),
            ..Default::default()
        };
        // SAFETY: Both allocations are stable and exclusively worker-owned.
        // Every queued request is drained by complete or Drop before reuse/free.
        let result = unsafe {
            ReadDirectoryChangesW(
                self.directory.raw(),
                self.buffer.as_mut_ptr().cast(),
                (BUFFER_WORDS * 4) as u32,
                self.target.recursive,
                FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_DIR_NAME
                    | FILE_NOTIFY_CHANGE_LAST_WRITE
                    | FILE_NOTIFY_CHANGE_SIZE
                    | FILE_NOTIFY_CHANGE_ATTRIBUTES,
                None,
                Some(self.overlapped.as_mut()),
                None,
            )
        };
        match result {
            Ok(()) => {
                self.pending = true;
                Ok(())
            }
            Err(error)
                if error.code() == windows::core::HRESULT::from_win32(ERROR_IO_PENDING.0) =>
            {
                self.pending = true;
                Ok(())
            }
            Err(error) => Err(format!("Failed to arm folder notifications: {error}")),
        }
    }

    fn complete(&mut self) -> Result<bool, String> {
        let mut bytes = 0;
        // The completion event is signaled; bWait still guarantees retirement.
        let result = unsafe {
            GetOverlappedResult(
                self.directory.raw(),
                self.overlapped.as_ref(),
                &mut bytes,
                true,
            )
        };
        match result {
            Err(error)
                if error.code() == windows::core::HRESULT::from_win32(ERROR_NOTIFY_ENUM_DIR.0) =>
            {
                self.pending = false;
                return Ok(true);
            }
            Err(error)
                if error.code() == windows::core::HRESULT::from_win32(ERROR_IO_INCOMPLETE.0) =>
            {
                return Err("Folder notification completion incomplete".to_string());
            }
            result => {
                self.pending = false;
                result.map_err(|error| format!("Folder notification failed: {error}"))?;
            }
        }
        if bytes == 0 {
            return Ok(true);
        } // OS overflow: details lost, invalidate safely.
        if bytes as usize > BUFFER_WORDS * 4 {
            return Ok(true);
        }
        // SAFETY: Completed I/O initialized these bytes, within the allocation.
        let bytes = unsafe {
            std::slice::from_raw_parts(self.buffer.as_ptr().cast::<u8>(), bytes as usize)
        };
        Ok(has_relevant_change(&self.target, bytes))
    }
}

impl Drop for DirectoryRead {
    fn drop(&mut self) {
        if !self.pending {
            return;
        }
        // CancelIoEx is a request, NOT completion. Even ERROR_NOT_FOUND may mean
        // the operation raced to completion; always wait before closing/freeing.
        unsafe {
            let _ = CancelIoEx(self.directory.raw(), Some(self.overlapped.as_ref()));
        }
        loop {
            let mut bytes = 0;
            let result = unsafe {
                GetOverlappedResult(
                    self.directory.raw(),
                    self.overlapped.as_ref(),
                    &mut bytes,
                    true,
                )
            };
            match result {
                Err(error)
                    if error.code()
                        == windows::core::HRESULT::from_win32(ERROR_IO_INCOMPLETE.0) =>
                {
                    continue
                }
                _ => break, // success or terminal completion error (e.g. cancelled)
            }
        }
        self.pending = false;
    }
}

fn run(
    stop: &OwnedHandle,
    reads: &mut [DirectoryRead],
    on_change: &Arc<dyn Fn() + Send + Sync>,
) -> Result<(), String> {
    let handles: Vec<HANDLE> = std::iter::once(stop.raw())
        .chain(reads.iter().map(|read| read.event.raw()))
        .collect();
    let mut due: Option<Instant> = None;
    loop {
        let timeout = due
            .map(|due| {
                due.saturating_duration_since(Instant::now())
                    .as_millis()
                    .min(u32::MAX as u128 - 1) as u32
            })
            .unwrap_or(INFINITE);
        let result = unsafe { WaitForMultipleObjects(&handles, false, timeout) };
        if result == WAIT_OBJECT_0 {
            return Ok(());
        }
        if result == WAIT_TIMEOUT {
            due = None;
            on_change();
            continue;
        }
        let index = result.0.wrapping_sub(WAIT_OBJECT_0.0 + 1) as usize;
        let read = reads
            .get_mut(index)
            .ok_or_else(|| "Folder notification wait failed".to_string())?;
        let relevant = read.complete()?;
        read.arm()?; // rearm before publication; no lost async refresh window
        if relevant && due.is_none() {
            due = Some(Instant::now() + DEBOUNCE);
        }
        // A fixed first-event deadline bounds burst latency even under sustained
        // writes. Only one pending invalidation is retained, never a growing queue.
        if due.is_some_and(|due| Instant::now() >= due) {
            due = None;
            // Prioritize a concurrent stop before invoking the callback.
            if unsafe { WaitForMultipleObjects(&[stop.raw()], false, 0) } == WAIT_OBJECT_0 {
                return Ok(());
            }
            on_change();
        }
    }
}

fn has_relevant_change(target: &WatchTarget, bytes: &[u8]) -> bool {
    let mut offset = 0usize;
    loop {
        let Some(header) = bytes.get(offset..offset.saturating_add(12)) else {
            return true;
        };
        let field = |start: usize| {
            u32::from_le_bytes([
                header[start],
                header[start + 1],
                header[start + 2],
                header[start + 3],
            ]) as usize
        };
        let next = field(0);
        let length = field(8);
        if length % 2 != 0 {
            return true;
        }
        let Some(name) = bytes.get(offset + 12..(offset + 12).saturating_add(length)) else {
            return true;
        };
        let name: Vec<u16> = name
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        use std::os::windows::ffi::OsStringExt;
        let relative = PathBuf::from(std::ffi::OsString::from_wide(&name));
        let relevant = relevant_path(target, &relative, field(4));
        if relevant {
            return true;
        }
        if next == 0 {
            return false;
        }
        if next < 12 + length || next % 4 != 0 {
            return true;
        }
        let Some(next_offset) = offset.checked_add(next) else {
            return true;
        };
        offset = next_offset;
    }
}

fn relevant_path(target: &WatchTarget, relative: &Path, action: usize) -> bool {
    let components: Vec<_> = relative.components().collect();
    if components.is_empty()
        || components
            .iter()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return true;
    }
    // Do not follow linked/junction subtrees into unrelated roots. Inspect each
    // ancestor entry before touching deeper entries; the leaf itself can change
    // the displayed row (e.g. adding/removing a junction) and is not traversed.
    let mut ancestor = target.path.clone();
    for component in components.iter().take(components.len() - 1) {
        ancestor.push(component.as_os_str());
        if let Ok(metadata) = std::fs::symlink_metadata(&ancestor) {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return false;
            }
        }
    }
    let first = components[0].as_os_str().to_string_lossy();
    if !target.metadata {
        // Metadata has its own independently derived watch, including linked
        // worktree index/HEAD and external common refs. Ignore objects/log churn.
        if !first.eq_ignore_ascii_case(".git") {
            return true;
        }
        if components.len() > 1 {
            return false;
        }
        // Child lock/temp churn also changes the .git directory's timestamp.
        // Only membership changes or edits to a .git *file* change identity.
        return action != 3
            || std::fs::symlink_metadata(target.path.join(relative))
                .map(|metadata| !metadata.is_dir())
                .unwrap_or(true);
    }
    let leaf = components
        .last()
        .map(|part| part.as_os_str().to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if leaf.ends_with(".lock") || leaf.ends_with(".tmp") || leaf.starts_with("tmp_") {
        return false;
    }
    if action == 3
        && std::fs::symlink_metadata(target.path.join(relative))
            .is_ok_and(|metadata| metadata.is_dir())
    {
        return false;
    }
    matches!(
        first.to_ascii_lowercase().as_str(),
        "index"
            | "head"
            | "config"
            | "config.worktree"
            | "packed-refs"
            | "refs"
            | "reftable"
            | "commondir"
            | "gitdir"
    )
}
