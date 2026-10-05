//! Production ports for the existing coordinator. Not wired into startup here.
use super::{coordinator::*, geometry::{self, CroppedImage, PhysicalRect}, native_registry::NativeRegistry};
use std::{path::{Path, PathBuf}, sync::{Arc, Mutex, mpsc, atomic::{AtomicBool, AtomicUsize, Ordering}}, time::{Duration, Instant}};
use tauri::{AppHandle, Emitter, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

/// Runtime must bind native foreground intent and obtain the exact top-bar DOM ack,
/// then bounded DwmFlush and revalidate visibility/current token. No successful default.
/// Called on a worker immediately before GDI freeze; never hide/exclude the bars.
pub type PreparationBarrier = Arc<dyn Fn(Instant) -> Result<(), SnipError> + Send + Sync>;
pub struct NativeCapturePort { prepare: PreparationBarrier, release: Option<Arc<dyn Fn() + Send + Sync>> }
impl NativeCapturePort {
    pub fn new(prepare: PreparationBarrier) -> Self { Self { prepare, release: None } }
    pub fn with_release(prepare: PreparationBarrier, release: Arc<dyn Fn() + Send + Sync>) -> Self { Self { prepare, release: Some(release) } }
}

#[link(name = "shcore")]
unsafe extern "system" { fn GetDpiForMonitor(monitor: isize, kind: i32, x: *mut u32, y: *mut u32) -> i32; }
struct Inventory { monitors: Vec<MonitorSnapshot>, failed: bool }
unsafe extern "system" fn enumerate(handle: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> windows_core::BOOL {
    // SAFETY: EnumDisplayMonitors is synchronous; data points to its live stack owner.
    let inventory = unsafe { &mut *(data.0 as *mut Inventory) };
    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let (mut x, mut y) = (0, 0);
    if inventory.monitors.len() >= 32 || !unsafe { GetMonitorInfoW(handle, &mut info) }.as_bool()
        || unsafe { GetDpiForMonitor(handle.0 as isize, 0, &mut x, &mut y) } < 0 || x == 0 || x != y {
        inventory.failed = true; return windows_core::BOOL(0);
    }
    let rect = PhysicalRect { left: info.rcMonitor.left, top: info.rcMonitor.top, right: info.rcMonitor.right, bottom: info.rcMonitor.bottom };
    if geometry::checked_monitor_len(rect, geometry::MAX_FROZEN_BYTES).is_err() { inventory.failed = true; return windows_core::BOOL(0); }
    inventory.monitors.push(MonitorSnapshot { id: String::new(), rect, scale: f64::from(x) / 96.0, identity: handle.0 as usize as u64 });
    windows_core::BOOL(1)
}
impl CapturePort for NativeCapturePort {
    fn snapshot(&self) -> Result<Vec<MonitorSnapshot>, SnipError> {
        let _dpi = super::windows::PhysicalDpi::enter().map_err(|_| SnipError::CaptureFailed)?;
        let mut inventory = Inventory { monitors: Vec::with_capacity(32), failed: false };
        if !unsafe { EnumDisplayMonitors(None, None, Some(enumerate), LPARAM((&mut inventory as *mut Inventory) as isize)) }.as_bool()
            || inventory.failed || inventory.monitors.is_empty() { return Err(SnipError::CaptureFailed); }
        inventory.monitors.sort_by_key(|m| (m.rect.left, m.rect.top, m.identity));
        let mut total = 0usize;
        for (index, monitor) in inventory.monitors.iter_mut().enumerate() {
            monitor.id = format!("m{index}");
            total = total.checked_add(geometry::checked_monitor_len(monitor.rect, geometry::MAX_FROZEN_BYTES).map_err(|_| SnipError::CaptureFailed)?).ok_or(SnipError::CaptureFailed)?;
        }
        if total > geometry::MAX_FROZEN_BYTES { return Err(SnipError::CaptureFailed); }
        Ok(inventory.monitors)
    }
    fn freeze(&self, monitors: &[MonitorSnapshot]) -> Result<Vec<Frame>, SnipError> {
        struct Release<'a>(&'a Option<Arc<dyn Fn() + Send + Sync>>);
        impl Drop for Release<'_> { fn drop(&mut self) { if let Some(release) = self.0 { release(); } } }
        let _release = Release(&self.release);
        (self.prepare)(Instant::now() + Duration::from_secs(5))?;
        if self.snapshot()? != monitors { return Err(SnipError::TopologyChanged); }
        let _dpi = super::windows::PhysicalDpi::enter().map_err(|_| SnipError::CaptureFailed)?;
        let frozen = super::capture::freeze_monitors(&monitors.iter().map(|m| m.rect).collect::<Vec<_>>()).map_err(|_| SnipError::CaptureFailed)?;
        if self.snapshot()? != monitors { return Err(SnipError::TopologyChanged); }
        Ok(frozen.into_iter().zip(monitors).map(|(frame, monitor)| Frame { monitor: monitor.clone(), rgba: frame.pixels }).collect())
    }
}

/// Construct on the Tauri setup thread. All coordinator calls must run on workers.
pub struct NativeWindowPort { app: AppHandle, pub registry: Arc<NativeRegistry>, main_thread: std::thread::ThreadId, before_close: Option<Arc<dyn Fn(Token, isize) + Send + Sync>>, data_directory: Option<PathBuf> }
impl NativeWindowPort {
    pub fn new(app: AppHandle, registry: Arc<NativeRegistry>) -> Self { Self { app, registry, main_thread: std::thread::current().id(), before_close: None, data_directory: None } }
    pub fn with_close(app: AppHandle, registry: Arc<NativeRegistry>, before_close: Arc<dyn Fn(Token, isize) + Send + Sync>) -> Self { Self { app, registry, main_thread: std::thread::current().id(), before_close: Some(before_close), data_directory: None } }
    pub(crate) fn with_data_directory(mut self, directory: Option<PathBuf>) -> Self { self.data_directory = directory; self }
    fn dispatch<T: Send + 'static>(&self, effect: impl FnOnce() -> Result<T, SnipError> + Send + 'static) -> Result<T, SnipError> {
        // Refuse synchronous main-thread entry rather than deadlocking its event loop.
        if std::thread::current().id() == self.main_thread { return Err(SnipError::WindowFailed); }
        let (send, receive) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let pending = cancelled.clone();
        self.app.run_on_main_thread(move || { if pending.load(Ordering::Acquire) { return; } let result = effect(); let _ = send.send(result); }).map_err(|_| SnipError::WindowFailed)?;
        match receive.recv_timeout(Duration::from_secs(5)) { Ok(result) => result, Err(_) => { cancelled.store(true, Ordering::Release); Err(SnipError::ReadinessTimeout) } }
    }
    fn create(&self, token: &Token, monitor: &MonitorSnapshot, preview: bool) -> Result<WindowRef, SnipError> {
        let generation = token.generation.parse::<u64>().map_err(|_| SnipError::Unauthorized)?;
        if generation == 0 || generation.to_string() != token.generation || token.capture_id.len() != 32 || !token.capture_id.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) { return Err(SnipError::Unauthorized); }
        let index = monitor.id.strip_prefix('m').and_then(|s| s.parse::<u8>().ok()).filter(|n| *n < 32).ok_or(SnipError::Unauthorized)?;
        if monitor.id != format!("m{index}") { return Err(SnipError::Unauthorized); }
        let label = if preview { format!("snip-preview-{generation}") } else { format!("snip-overlay-{generation}-{}", monitor.id) };
        let entry = self.registry.reserve(label.clone(), Some(token.clone()), Some(monitor.id.clone()), preview)?;
        let app = self.app.clone(); let rect = monitor.rect; let scale = monitor.scale;
        let data_directory = self.data_directory.clone();
        let owned = entry.clone();
        let result = self.dispatch(move || {
            let _dpi = super::windows::PhysicalDpi::enter().map_err(|_| SnipError::WindowFailed)?;
            let builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("index.html".into()))
                .visible(false).focused(false).decorations(false).transparent(true).always_on_top(true).skip_taskbar(true).resizable(false)
                .inner_size(if preview { 320.0 } else { f64::from(rect.right - rect.left) / scale }, if preview { 264.0 } else { f64::from(rect.bottom - rect.top) / scale });
            let builder = if let Some(directory) = data_directory { builder.data_directory(directory) } else { builder };
            let window = builder.build().map_err(|_| SnipError::WindowFailed)?;
            let prepare = (|| {
                owned.bind(window.clone())?;
                let hwnd = HWND(window.hwnd().map_err(|_| SnipError::WindowFailed)?.0);
                let (mut left, mut top) = (rect.left, rect.top);
                let (width, height) = if preview { ((320.0 * scale).round() as i32, (264.0 * scale).round() as i32) } else { (rect.right - rect.left, rect.bottom - rect.top) };
                if preview {
                    // Select by the immutable monitor rectangle, not the initial window location.
                    let point = POINT { x: rect.left, y: rect.top };
                    let handle = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONULL) };
                    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
                    if !unsafe { GetMonitorInfoW(handle, &mut info) }.as_bool() { return Err(SnipError::WindowFailed); }
                    if width > info.rcWork.right - info.rcWork.left || height > info.rcWork.bottom - info.rcWork.top { return Err(SnipError::WindowFailed); }
                    left = info.rcWork.right - width; top = info.rcWork.top;
                    unsafe { let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE); SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_NOACTIVATE.0 as isize); }
                }
                unsafe { SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE).map_err(|_| SnipError::WindowFailed)?;
                    SetWindowPos(hwnd, Some(HWND_TOPMOST), left, top, width, height, SWP_NOACTIVATE | SWP_FRAMECHANGED).map_err(|_| SnipError::WindowFailed)?; }
                let mut bounds = RECT::default();
                unsafe { GetWindowRect(hwnd, &mut bounds) }.map_err(|_| SnipError::WindowFailed)?;
                if (bounds.left, bounds.top, bounds.right - bounds.left, bounds.bottom - bounds.top) != (left, top, width, height) { return Err(SnipError::WindowFailed); }
                Ok(owned.reference.clone())
            })();
            if prepare.is_err() { owned.invalidate(); let _ = window.destroy(); }
            prepare
        });
        if result.is_err() {
            let window = entry.retire();
            let _ = self.app.run_on_main_thread(move || { if let Some(window) = window { let _ = window.destroy(); } });
        }
        result
    }
}
impl WindowPort for NativeWindowPort {
    fn create_overlay(&self, token: &Token, monitor: &MonitorSnapshot) -> Result<WindowRef, SnipError> { self.create(token, monitor, false) }
    fn create_preview(&self, token: &Token, monitor: &MonitorSnapshot, _: u32, _: u32) -> Result<WindowRef, SnipError> { self.create(token, monitor, true) }
    fn emit_context(&self, reference: &WindowRef, context: &Context) -> Result<(), SnipError> {
        let entry = self.registry.entry(reference)?; entry.commit(context)?;
        entry.window()?.emit("snip:context", context_json(context)).map_err(|_| SnipError::WindowFailed)
    }
    fn emit_armed(&self, reference: &WindowRef, token: &Token) -> Result<(), SnipError> {
        let entry = self.registry.entry(reference)?;
        if entry.token.as_ref() != Some(token) { return Err(SnipError::Unauthorized); }
        entry.window()?.emit("snip:armed", serde_json::json!({"generation":token.generation,"captureId":token.capture_id})).map_err(|_| SnipError::WindowFailed)
    }
    fn show(&self, reference: &WindowRef) -> Result<(), SnipError> {
        let entry = self.registry.entry(reference)?;
        self.dispatch(move || { let window = entry.window()?; let hwnd = HWND(window.hwnd().map_err(|_| SnipError::WindowFailed)?.0);
            unsafe { let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE); }
            if !unsafe { IsWindowVisible(hwnd) }.as_bool() { return Err(SnipError::WindowFailed); } Ok(()) })
    }
    fn hide(&self, reference: &WindowRef) -> Result<(), SnipError> {
        let entry = self.registry.entry(reference)?;
        self.dispatch(move || { let window = entry.window()?; let hwnd = HWND(window.hwnd().map_err(|_| SnipError::WindowFailed)?.0);
            unsafe { let _ = ShowWindow(hwnd, SW_HIDE); }
            if unsafe { IsWindowVisible(hwnd) }.as_bool() { return Err(SnipError::WindowFailed); } Ok(()) })
    }
    fn close(&self, reference: WindowRef) {
        if let Ok(entry) = self.registry.entry(&reference) {
            // Cleanup must not be cancelled merely because the UI queue is late.
            let window = entry.retire();
            let before_close = self.before_close.clone();
            let token = entry.token.clone();
            let id = entry.reference.caller.window_id;
            if let Some(window) = window { let _ = self.app.run_on_main_thread(move || {
                if !super::native_registry::same_epoch(&window, id) { return; }
                if let (Some(before_close), Some(token), Ok(hwnd)) = (before_close, token, window.hwnd()) { before_close(token, hwnd.0 as isize); }
                let _ = window.destroy();
            }); }
        }
    }
    fn still_same(&self, reference: &WindowRef) -> bool { self.registry.entry(reference).and_then(|e| e.window()).is_ok() }
}
pub fn context_json(context: &Context) -> serde_json::Value {
    serde_json::json!({"generation":context.token.generation,"captureId":context.token.capture_id,"phase":match context.phase { Phase::Preparing=>"preparing",Phase::Selecting=>"selecting",Phase::Preview=>"preview",Phase::Saving=>"saving" },"monitorId":context.monitor_id,"width":context.width,"height":context.height,"scaleFactor":context.scale_factor,"originX":context.origin_x,"originY":context.origin_y})
}

// SAFETY: ProcessOwner contains owned kernel Job/Child/pipe handles, not an OLE
// apartment or UI handle. Kernel handles are thread-independent. The mutex gives
// exclusive access including shutdown/drop; no raw handle escapes this wrapper.
struct SerializedOwner(super::clipboard_process::ProcessOwner);
unsafe impl Send for SerializedOwner {}
pub struct NativeClipboardPort { owner: Mutex<SerializedOwner>, stopped: AtomicBool, attempts: AtomicUsize }
impl NativeClipboardPort {
    pub fn new() -> Result<Self, SnipError> {
        let executable = std::env::current_exe().map_err(|_| SnipError::ImageFailed)?;
        Ok(Self { owner: Mutex::new(SerializedOwner(super::clipboard_process::ProcessOwner::spawn(&executable).map_err(|_| SnipError::ImageFailed)?)), stopped: AtomicBool::new(false), attempts: AtomicUsize::new(0) })
    }
    pub fn shutdown(&self) {
        self.stopped.store(true, Ordering::Release);
        if let Ok(mut owner) = self.owner.try_lock() { let _ = owner.0.shutdown(); }
    }
    pub(crate) fn publication_attempts(&self) -> usize { self.attempts.load(Ordering::Acquire) }
}
impl ClipboardPort for NativeClipboardPort {
    fn publish(&self, crop: &CroppedImage, retained: usize) -> ClipboardOutcome {
        let Ok(mut owner) = self.owner.try_lock() else { return ClipboardOutcome::Rejected("clipboard-busy".into()); };
        if self.stopped.load(Ordering::Acquire) { return ClipboardOutcome::Rejected("clipboard-owner-stopped".into()); }
        let Some(budget) = retained.checked_add(crop.pixels.len()) else { return ClipboardOutcome::Rejected("clipboard-budget".into()); };
        if budget.checked_add(crop.pixels.len()).is_none_or(|n| n > geometry::MAX_STAGING_BYTES) { return ClipboardOutcome::Rejected("clipboard-budget".into()); }
        let mut pixels = Vec::new();
        if pixels.try_reserve_exact(crop.pixels.len()).is_err() { return ClipboardOutcome::Rejected("clipboard-budget".into()); }
        pixels.extend_from_slice(&crop.pixels);
        self.attempts.fetch_add(1, Ordering::AcqRel);
        let outcome = owner.0.publish(CroppedImage { width: crop.width, height: crop.height, pixels }, budget, super::clipboard::Failpoints::default());
        if self.stopped.load(Ordering::Acquire) { let _ = owner.0.shutdown(); }
        outcome
    }
}
pub struct NativePickerPort { app: AppHandle, main_thread: std::thread::ThreadId }
impl NativePickerPort { pub fn new(app: AppHandle) -> Self { Self { app, main_thread: std::thread::current().id() } } }
impl PickerPort for NativePickerPort {
    fn pick_png(&self) -> Result<Option<PathBuf>, SaveCode> {
        if std::thread::current().id() == self.main_thread { return Err(SaveCode::Failed); }
        self.app.dialog().file().add_filter("PNG image", &["png"]).set_file_name("Screen snip.png").blocking_save_file()
            .map(|file| file.into_path().map_err(|_| SaveCode::Failed)).transpose()
    }
}
pub struct NativePublisherPort;
impl PublisherPort for NativePublisherPort { fn publish_png(&self, destination: &Path, png: &[u8]) -> Result<(), SaveCode> { super::save::publish_png(destination, png) } }
pub struct NativeClock;
impl ClockPort for NativeClock { fn now(&self) -> Instant { Instant::now() } }
