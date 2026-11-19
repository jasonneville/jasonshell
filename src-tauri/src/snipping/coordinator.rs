//! Adapter-facing production lifecycle. Native effects occur outside the state lock.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/snipping_image_concurrency.rs"));
use super::{geometry::{self, CroppedImage, LogicalRect, PhysicalRect, MAX_FROZEN_BYTES, MAX_STAGING_BYTES}, save::SaveError};
pub use super::session::Caller;
pub use super::clipboard_process::Outcome as ClipboardOutcome;
use std::{path::{Path, PathBuf}, sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, atomic::{AtomicU64, Ordering}}, time::{Duration, Instant}};
use super::diagnostics::{Diagnostics, Event as DiagnosticEvent};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token { pub generation: String, pub capture_id: String }
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorSnapshot { pub id: String, pub rect: PhysicalRect, pub scale: f64, pub identity: u64 }
#[derive(Debug)]
pub struct Frame { pub monitor: MonitorSnapshot, pub rgba: Vec<u8> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowRef { pub caller: Caller, pub monitor_id: Option<String> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Preparing, Selecting, Preview, Saving }
#[derive(Clone, Debug)]
pub struct Context { pub token: Token, pub phase: Phase, pub monitor_id: String, pub width: u32, pub height: u32, pub scale_factor: f64, pub origin_x: i32, pub origin_y: i32 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnipError { Inactive, Unauthorized, Stale, Busy, InvalidSelection, TopologyChanged, CaptureFailed, ReadinessTimeout, WindowFailed, ImageFailed }
pub type SaveCode = SaveError;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveOutcome { Saved, Cancelled, Error(SaveCode) }

pub trait CapturePort: Send + Sync {
    fn snapshot(&self) -> Result<Vec<MonitorSnapshot>, SnipError>;
    fn freeze(&self, monitors: &[MonitorSnapshot]) -> Result<Vec<Frame>, SnipError>;
}
pub trait WindowPort: Send + Sync {
    /// Retire construction not yet returned/installed in the coordinator.
    fn retire_generation(&self, _token: &Token) {}
    fn create_overlay(&self, token: &Token, monitor: &MonitorSnapshot) -> Result<WindowRef, SnipError>;
    fn create_preview(&self, token: &Token, monitor: &MonitorSnapshot, width: u32, height: u32) -> Result<WindowRef, SnipError>;
    fn emit_context(&self, window: &WindowRef, context: &Context) -> Result<(), SnipError>;
    fn emit_armed(&self, window: &WindowRef, token: &Token) -> Result<(), SnipError>;
    fn show(&self, window: &WindowRef) -> Result<(), SnipError>;
    fn hide(&self, window: &WindowRef) -> Result<(), SnipError>;
    fn close(&self, window: WindowRef);
    fn still_same(&self, window: &WindowRef) -> bool;
    /// Preview image checks must not depend on an unbounded UI dispatcher wait.
    fn still_same_for_image(&self, window: &WindowRef) -> bool { self.still_same(window) }
}
pub trait ClipboardPort: Send + Sync { fn publish(&self, crop: &CroppedImage, retained_bytes: usize) -> ClipboardOutcome; }
pub trait PickerPort: Send + Sync { fn pick_png(&self) -> Result<Option<PathBuf>, SaveCode>; }
pub trait PublisherPort: Send + Sync { fn publish_png(&self, destination: &Path, png: &[u8]) -> Result<(), SaveCode>; }
pub trait ClockPort: Send + Sync { fn now(&self) -> Instant; }
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub phase: Option<Phase>, pub token: Option<Token>, pub frame_bytes: usize, pub image_bytes: usize,
    pub window_ids: Vec<u64>, pub readiness_count: usize, pub save_pin: bool,
    pub last_publication: Option<Publication>,
}
/// One token-bound actual native outcome; retained even if postcommit show fails.
#[derive(Clone, Debug)]
pub struct Publication { pub token: Token, pub outcome: ClipboardOutcome }

// Context is already committed when this wait matters. Allow the 2s clipboard
// operation and 5s show dispatch, with 1s for staging and cleanup overhead.
const PREVIEW_IMAGE_WAIT: Duration = Duration::from_secs(8);
#[cfg(test)]
thread_local! {
    static PREVIEW_WAIT_OBSERVER: std::cell::RefCell<Option<std::sync::mpsc::Sender<()>>> = const { std::cell::RefCell::new(None) };
}
#[derive(PartialEq, Eq)]
enum PreviewReadiness { Pending, Ready, Retired }
struct Preview { token: Token, window: WindowRef, crop: Arc<CroppedImage>, monitor: MonitorSnapshot, topology: Vec<MonitorSnapshot>, readiness: PreviewReadiness }
struct Current {
    token: Token, phase: Phase, deadline: Instant, frames: Vec<Arc<Frame>>, windows: Vec<WindowRef>,
    ready: Vec<String>, armed: bool, selected: Option<String>, preview: Option<Preview>, busy: bool, escape_after_arming: bool,
}
#[derive(Default)]
struct Inner { next: u64, active: Option<Current>, previous: Option<Preview>, last_publication: Option<Publication>, image_reservations: usize, images_stopped: bool }
enum ImageSource { Frame(Arc<Frame>), Crop(Arc<CroppedImage>) }
/// Independent read ownership survives cancellation, without pinning mutation busy.
/// Charge both the borrowed source and PNG output, conservatively even when the
/// source is also retained by the active generation. Retired generations stay charged.
struct ImageRead<'a> { coordinator: &'a Coordinator, source: Option<ImageSource>, charge: usize }
impl Drop for ImageRead<'_> {
    fn drop(&mut self) {
        drop(self.source.take());
        if let Ok(mut inner) = self.coordinator.inner.lock() { inner.image_reservations -= self.charge; }
    }
}
pub struct Coordinator {
    diagnostics: OnceLock<Arc<Diagnostics>>,
    selecting_generation: AtomicU64,
    capture: Arc<dyn CapturePort>, windows: Arc<dyn WindowPort>, clipboard: Arc<dyn ClipboardPort>,
    picker: Arc<dyn PickerPort>, publisher: Arc<dyn PublisherPort>, clock: Arc<dyn ClockPort>, inner: Mutex<Inner>, preview_changed: Condvar,
}
impl Coordinator {
    pub fn new(capture: Arc<dyn CapturePort>, windows: Arc<dyn WindowPort>, clipboard: Arc<dyn ClipboardPort>, picker: Arc<dyn PickerPort>, publisher: Arc<dyn PublisherPort>, clock: Arc<dyn ClockPort>) -> Self {
        Self { diagnostics: OnceLock::new(), selecting_generation: AtomicU64::new(0), capture, windows, clipboard, picker, publisher, clock, inner: Mutex::new(Inner::default()), preview_changed: Condvar::new() }
    }
    pub(crate) fn set_diagnostics(&self, diagnostics: Arc<Diagnostics>) { let _ = self.diagnostics.set(diagnostics); }
    pub(crate) fn diagnostics(&self) -> Option<Arc<Diagnostics>> { self.diagnostics.get().cloned() }
    fn diagnostic(&self, event: DiagnosticEvent) { if let Some(diagnostics) = self.diagnostics.get() { diagnostics.record(event); } }
    fn diagnostic_state(&self, current: Option<&Current>) {
        self.diagnostic(DiagnosticEvent::State { phase: current.map_or(0, |c| match c.phase { Phase::Preparing=>1,Phase::Selecting=>2,Phase::Preview=>3,Phase::Saving=>4 }),
            busy: current.is_some_and(|c|c.busy), armed: current.is_some_and(|c|c.armed), deferred: current.is_some_and(|c|c.escape_after_arming) });
    }
    fn lock(&self) -> Result<MutexGuard<'_, Inner>, SnipError> { self.inner.lock().map_err(|_| SnipError::Inactive) }
    pub fn start(&self, initiator: Caller) -> Result<Token, SnipError> {
        if initiator.label != "top-bar" || initiator.window_id == 0 || !self.windows.still_same(&WindowRef { caller: initiator, monitor_id: None }) {
            return Err(SnipError::Unauthorized);
        }
        let now = self.clock.now();
        let capture_id = random_token()?;
        let (token, old_window, retained) = {
            let mut inner = self.lock()?;
            if inner.active.as_ref().is_some_and(|current| current.busy || current.phase != Phase::Preview) { return Err(SnipError::Busy); }
            let generation = inner.next.checked_add(1).ok_or(SnipError::Inactive)?;
            inner.next = generation;
            inner.previous = inner.active.take().and_then(|mut current| current.preview.take());
            let retained = inner.previous.as_ref().map_or(0, |previous| previous.crop.pixels.capacity())
                .checked_add(inner.image_reservations).ok_or(SnipError::ImageFailed)?;
            let old_window = inner.previous.as_ref().map(|previous| previous.window.clone());
            let token = Token { generation: generation.to_string(), capture_id };
            inner.active = Some(Current { token: token.clone(), phase: Phase::Preparing, deadline: now + Duration::from_secs(5),
                frames: Vec::new(), windows: Vec::new(), ready: Vec::new(), armed: false, selected: None, preview: None, busy: false, escape_after_arming: false });
            self.diagnostic(DiagnosticEvent::Begin); self.diagnostic_state(inner.active.as_ref());
            (token, old_window, retained)
        };
        let result = (|| {
            if let Some(window) = &old_window { self.windows.hide(window)?; }
            let monitors = self.capture.snapshot()?;
            validate_monitors(&monitors, retained)?;
            let frames = self.capture.freeze(&monitors)?;
            if frames.len() != monitors.len() { return Err(SnipError::CaptureFailed); }
            let mut bytes = 0usize;
            for (frame, monitor) in frames.iter().zip(&monitors) {
                let expected = geometry::checked_monitor_len(monitor.rect, MAX_FROZEN_BYTES).map_err(|_| SnipError::CaptureFailed)?;
                if &frame.monitor != monitor || frame.rgba.len() != expected { return Err(SnipError::CaptureFailed); }
                bytes = bytes.checked_add(frame.rgba.capacity()).ok_or(SnipError::CaptureFailed)?;
            }
            if bytes > MAX_FROZEN_BYTES || bytes.checked_add(retained).is_none_or(|n| n > MAX_STAGING_BYTES) { return Err(SnipError::CaptureFailed); }
            {
                let mut inner = self.lock()?;
                let current = active_mut(&mut inner, &token)?;
                current.frames = frames.into_iter().map(Arc::new).collect();
            }
            for monitor in &monitors {
                self.check_deadline(&token)?;
                let window = self.windows.create_overlay(&token, monitor)?;
                let context = context_for(&token, Phase::Selecting, monitor, monitor.rect.dimensions().map_err(|_| SnipError::CaptureFailed)?);
                let registered = {
                    let mut inner = self.lock()?;
                    match active_mut(&mut inner, &token) {
                        Ok(current) => { current.windows.push(window.clone()); true }, Err(_) => false,
                    }
                };
                if !registered { self.windows.close(window); return Err(SnipError::Stale); }
                self.windows.emit_context(&window, &context)?;
            }
            self.check_deadline(&token)?;
            {
                let mut inner = self.lock()?;
                active_mut(&mut inner, &token)?.phase = Phase::Selecting;
                self.selecting_generation.store(inner.next, Ordering::Release);
                self.diagnostic_state(inner.active.as_ref());
            }
            self.arm_if_ready(&token)?;
            Ok(token.clone())
        })();
        if result.is_err() { self.rollback(&token); }
        result
    }
    pub fn context(&self, caller: Caller) -> Result<Context, SnipError> {
        let window = WindowRef { caller: caller.clone(), monitor_id: None };
        if !self.windows.still_same(&window) { return Err(SnipError::Unauthorized); }
        let inner = self.lock()?;
        let current = inner.active.as_ref().ok_or(SnipError::Inactive)?;
        if let Some(preview) = &current.preview {
            if preview.window.caller == caller { return Ok(context_for(&current.token, Phase::Preview, &preview.monitor, (preview.crop.width, preview.crop.height))); }
        }
        let owned = current.windows.iter().find(|window| window.caller == caller).ok_or(SnipError::Unauthorized)?;
        let frame = current.frames.iter().find(|frame| owned.monitor_id.as_ref() == Some(&frame.monitor.id)).ok_or(SnipError::Inactive)?;
        Ok(context_for(&current.token, Phase::Selecting, &frame.monitor, frame.monitor.rect.dimensions().map_err(|_| SnipError::ImageFailed)?))
    }
    pub fn image(&self, caller: Caller, token: Token, monitor_id: String) -> Result<Vec<u8>, SnipError> {
        self.image_with_preview_wait(caller, token, monitor_id, PREVIEW_IMAGE_WAIT)
    }
    #[cfg(test)]
    pub(crate) fn observe_next_preview_wait(sender: std::sync::mpsc::Sender<()>) {
        PREVIEW_WAIT_OBSERVER.with(|observer| *observer.borrow_mut() = Some(sender));
    }
    // The explicit interval lets coordinator fixtures exercise timeout without a
    // production override or depending on the capture clock advancing.
    fn image_with_preview_wait(&self, caller: Caller, token: Token, monitor_id: String, wait: Duration) -> Result<Vec<u8>, SnipError> {
        self.live_for_image(&caller)?;
        // Expiry can run native rollback effects. Only an owning overlay image
        // should drive them, never a preview read for a replaced generation.
        self.expire_matching(Some((&caller, &token)));
        let (read, output_limit) = {
            let mut inner = self.wait_for_image(&caller, &token, &monitor_id, wait)?;
            let retained = retained_bytes(&inner);
            let current = active_mut(&mut inner, &token)?;
            if current.busy { return Err(SnipError::Busy); }
            let source = if let Some(preview) = &current.preview {
                if preview.window.caller != caller || preview.monitor.id != monitor_id { return Err(SnipError::Unauthorized); }
                if preview.readiness != PreviewReadiness::Ready { return Err(SnipError::Unauthorized); }
                ImageSource::Crop(preview.crop.clone())
            } else {
                authorize_overlay(current, &caller, &monitor_id)?;
                ImageSource::Frame(current.frames.iter().find(|frame| frame.monitor.id == monitor_id).cloned().ok_or(SnipError::Unauthorized)?)
            };
            let source_bytes = match &source { ImageSource::Frame(frame) => frame.rgba.capacity(), ImageSource::Crop(crop) => crop.pixels.capacity() };
            // The stored-block PNG codec borrows RGBA and adds at most one filter
            // byte per row, five bytes per 65535-byte block, and 63 fixed bytes.
            // With validated dimensions <=16384, 64KiB bounds all that overhead.
            let output_limit = source_bytes.checked_add(65_536).ok_or(SnipError::ImageFailed)?;
            let charge = source_bytes.checked_add(output_limit).ok_or(SnipError::ImageFailed)?;
            if retained.checked_add(charge).is_none_or(|bytes| bytes > MAX_STAGING_BYTES) { return Err(SnipError::ImageFailed); }
            inner.image_reservations += charge;
            (ImageRead { coordinator: self, source: Some(source), charge }, output_limit)
        };
        #[cfg(test)]
        image_concurrency_verification::before_encode(&token, &monitor_id);
        // Give the codec only this reservation's output allowance, including its
        // actual Vec capacity check. No state lock is held during allocation/encode.
        let codec_retained = MAX_STAGING_BYTES - output_limit;
        let encoded = match read.source.as_ref().ok_or(SnipError::Inactive)? {
            ImageSource::Frame(frame) => frame.monitor.rect.dimensions().map_err(|_| SnipError::ImageFailed)
                .and_then(|(width, height)| super::clipboard::png_rgba_transport(width, height, &frame.rgba, codec_retained).map_err(|_| SnipError::ImageFailed)),
            ImageSource::Crop(crop) => super::clipboard::png_rgba_transport(crop.width, crop.height, &crop.pixels, codec_retained).map_err(|_| SnipError::ImageFailed),
        };
        if matches!(read.source, Some(ImageSource::Frame(_))) { self.expire_matching(Some((&caller, &token))); }
        {
            let mut inner = self.lock()?;
            if inner.images_stopped { return Err(SnipError::Inactive); }
            let current = active_mut(&mut inner, &token)?;
            if (current.phase == Phase::Preparing || (current.phase == Phase::Selecting && !current.armed))
                && self.clock.now() >= current.deadline { return Err(SnipError::ReadinessTimeout); }
        }
        self.live_for_image(&caller)?;
        encoded
    }
    fn live_for_image(&self, caller: &Caller) -> Result<(), SnipError> {
        if caller.window_id == 0 || !self.windows.still_same_for_image(&WindowRef { caller: caller.clone(), monitor_id: None }) { return Err(SnipError::Unauthorized); }
        Ok(())
    }
    fn wait_for_image(&self, caller: &Caller, token: &Token, monitor: &str, wait: Duration) -> Result<MutexGuard<'_, Inner>, SnipError> {
        let mut inner = self.lock()?;
        let mut deadline = None;
        let mut checked_ready = false;
        loop {
            if inner.images_stopped { return Err(SnipError::Inactive); }
            let current = active_mut(&mut inner, token)?;
            let Some(preview) = current.preview.as_ref() else {
                return if deadline.is_none() { Ok(inner) } else { Err(SnipError::Stale) };
            };
            if preview.readiness == PreviewReadiness::Pending {
                if &preview.window.caller != caller {
                    // A bound overlay still gets Busy during completion, but
                    // neither it nor an invalid binding can enter this wait.
                    authorize_overlay(current, caller, monitor)?;
                    return Err(SnipError::Busy);
                }
                if preview.monitor.id != monitor { return Err(SnipError::Unauthorized); }
            } else if deadline.is_none() {
                return Ok(inner);
            }
            // A waiting request must still name the same preview after wake.
            if &preview.window.caller != caller || preview.monitor.id != monitor || preview.readiness == PreviewReadiness::Retired {
                return Err(SnipError::Unauthorized);
            }
            let deadline = *deadline.get_or_insert_with(|| Instant::now() + wait);
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() { return Err(SnipError::ReadinessTimeout); }
            if preview.readiness == PreviewReadiness::Ready {
                if checked_ready { return Ok(inner); }
                // Terminal state and deadline were checked before leaving the
                // lock. Revalidate both again after the concrete-instance check.
                drop(inner);
                self.live_for_image(caller)?;
                checked_ready = true;
                inner = self.lock()?;
                continue;
            }
            #[cfg(test)]
            PREVIEW_WAIT_OBSERVER.with(|observer| {
                if let Some(sender) = observer.borrow_mut().take() { let _ = sender.send(()); }
            });
            inner = self.preview_changed.wait_timeout(inner, remaining).map_err(|_| SnipError::Inactive)?.0;
        }
    }
    pub(crate) fn preview_retired(&self, caller: &Caller) {
        if let Ok(mut inner) = self.lock() {
            if let Some(preview) = inner.active.as_mut().and_then(|current| current.preview.as_mut()).filter(|preview| &preview.window.caller == caller) {
                preview.readiness = PreviewReadiness::Retired;
                self.preview_changed.notify_all();
            }
        }
    }
    pub(crate) fn stop_images(&self) {
        if let Ok(mut inner) = self.lock() {
            inner.images_stopped = true;
            self.preview_changed.notify_all();
        }
    }
    pub fn ready(&self, caller: Caller, token: Token, monitor_id: String) -> Result<bool, SnipError> {
        self.live(&caller)?;
        self.check_deadline(&token)?;
        {
            let mut inner = self.lock()?;
            let current = active_mut(&mut inner, &token)?;
            authorize_overlay(current, &caller, &monitor_id)?;
            if current.armed { return Ok(true); }
            if current.busy { return Err(SnipError::Busy); }
            if !current.ready.contains(&monitor_id) { current.ready.push(monitor_id); }
            // A fast first renderer may finish while later overlays are still
            // being constructed. Start will arm after the full set is installed.
            if current.phase == Phase::Preparing { return Ok(false); }
        }
        self.arm_if_ready(&token)
    }
    fn arm_if_ready(&self, token: &Token) -> Result<bool, SnipError> {
        let windows = {
            let mut inner = self.lock()?;
            let current = active_mut(&mut inner, token)?;
            if current.armed { return Ok(true); }
            if current.phase != Phase::Selecting || current.busy { return Ok(false); }
            if current.ready.len() != current.windows.len() { return Ok(false); }
            current.busy = true;
            self.diagnostic(DiagnosticEvent::ArmBegin); self.diagnostic_state(Some(current));
            current.windows.clone()
        };
        let result = (|| {
            for window in &windows { self.windows.show(window)?; }
            for window in &windows { self.windows.emit_armed(window, token)?; }
            Ok(())
        })();
        if let Err(error) = result.and_then(|_| self.check_deadline(token)) {
            self.unpin(token); self.rollback(token); return Err(error);
        }
        // Finalize arming atomically with Escape admission. If Escape was
        // observed, retain the pin until rollback removes the generation; if
        // not, publish armed and release the pin in the same critical section.
        let cancelled = {
            let mut inner = self.lock()?;
            let current = active_mut(&mut inner, token)?;
            let cancelled = if current.escape_after_arming { true } else {
                current.armed = true; current.busy = false; false
            };
            self.diagnostic_state(Some(current)); cancelled
        };
        self.diagnostic(DiagnosticEvent::ArmEnd);
        if cancelled { self.rollback_matching(token, true, true); return Err(SnipError::Stale); }
        Ok(true)
    }
    pub fn begin_selection(&self, caller: Caller, token: Token, monitor_id: String) -> Result<bool, SnipError> {
        self.live(&caller)?;
        let mut inner = self.lock()?;
        let current = active_mut(&mut inner, &token)?;
        authorize_overlay(current, &caller, &monitor_id)?;
        if !current.armed || current.busy { return Err(SnipError::Busy); }
        if current.selected.is_some() { return Ok(false); }
        current.selected = Some(monitor_id);
        Ok(true)
    }
    pub fn complete(&self, caller: Caller, token: Token, monitor_id: String, local: LogicalRect) -> Result<bool, SnipError> {
        self.live(&caller)?;
        let (frame, retained, topology) = {
            let mut inner = self.lock()?;
            let retained = retained_bytes(&inner);
            let current = active_mut(&mut inner, &token)?;
            authorize_overlay(current, &caller, &monitor_id)?;
            if current.busy || !current.armed { return Err(SnipError::Busy); }
            if current.selected.as_ref() != Some(&monitor_id) { return Err(SnipError::Unauthorized); }
            let frame = current.frames.iter().find(|frame| frame.monitor.id == monitor_id).cloned().ok_or(SnipError::Unauthorized)?;
            let topology = current.frames.iter().map(|frame| frame.monitor.clone()).collect::<Vec<_>>();
            current.busy = true;
            self.diagnostic_state(Some(current));
            (frame, retained, topology)
        };
        let result = (|| {
            if self.capture.snapshot()? != topology { return Err(SnipError::TopologyChanged); }
            let rect = geometry::physical_rect_from_logical(frame.monitor.rect, frame.monitor.scale, local).map_err(|_| SnipError::InvalidSelection)?;
            let (width, height) = rect.dimensions().map_err(|_| SnipError::InvalidSelection)?;
            let crop_bytes = geometry::checked_rgba_len(width, height, MAX_STAGING_BYTES).map_err(|_| SnipError::InvalidSelection)?;
            if retained.checked_add(crop_bytes).is_none_or(|n| n > MAX_STAGING_BYTES) { return Err(SnipError::ImageFailed); }
            let crop = Arc::new(geometry::crop_rgba(frame.monitor.rect, rect, &frame.rgba, MAX_STAGING_BYTES).map_err(|_| SnipError::ImageFailed)?);
            let window = self.windows.create_preview(&token, &frame.monitor, width, height)?;
            let installed = {
                let mut inner = self.lock()?;
                match active_mut(&mut inner, &token) {
                    Ok(current) => {
                        current.preview = Some(Preview { token: token.clone(), window: window.clone(), crop: crop.clone(), monitor: frame.monitor.clone(), topology: topology.clone(), readiness: PreviewReadiness::Pending });
                        true
                    }, Err(_) => false,
                }
            };
            if !installed { self.windows.close(window); return Err(SnipError::Stale); }
            self.windows.emit_context(&window, &context_for(&token, Phase::Preview, &frame.monitor, (width, height)))?;
            // Hidden creation/context are precommit. Once publication is attempted,
            // show failure cannot erase or reinterpret its actual clipboard outcome.
            if self.capture.snapshot()? != topology { return Err(SnipError::TopologyChanged); }
            if !self.windows.still_same(&window) { return Err(SnipError::Unauthorized); }
            let publication_retained = {
                let mut inner = self.lock()?;
                active_mut(&mut inner, &token)?;
                retained_bytes(&inner)
            };
            let outcome = self.clipboard.publish(&crop, publication_retained);
            self.record_publication(&token, outcome)?;
            self.windows.show(&window)?;
            let (old, overlays) = {
                let mut inner = self.lock()?;
                let current = active_mut(&mut inner, &token)?;
                current.phase = Phase::Preview;
                self.selecting_generation.store(0, Ordering::Release);
                self.diagnostic_state(Some(current));
                current.frames.clear();
                let overlays = std::mem::take(&mut current.windows);
                (inner.previous.take(), overlays)
            };
            for overlay in overlays { self.windows.close(overlay); }
            if let Some(old) = old { self.windows.close(old.window); }
            Ok(true)
        })();
        if result.is_ok() {
            if let Ok(mut inner) = self.lock() { if let Ok(current) = active_mut(&mut inner, &token) {
                current.busy = false;
                if let Some(preview) = current.preview.as_mut().filter(|preview| preview.readiness == PreviewReadiness::Pending) {
                    preview.readiness = PreviewReadiness::Ready;
                    self.preview_changed.notify_all();
                }
                self.diagnostic_state(Some(current));
            } }
        } else {
            // Remove the failed generation while still pinned. No interval in
            // which a reader could observe an unpinned, unshown crop.
            self.rollback_completion(&token);
        }
        result
    }
    pub fn cancel(&self, caller: Caller, token: Token) -> Result<bool, SnipError> {
        self.live(&caller)?;
        {
            let mut inner = self.lock()?;
            let current = active_mut(&mut inner, &token)?;
            if !current.windows.iter().any(|window| window.caller == caller) { return Err(SnipError::Unauthorized); }
            if current.busy { return Err(SnipError::Busy); }
        }
        self.rollback(&token);
        Ok(true)
    }
    pub fn copy(&self, caller: Caller, token: Token) -> Result<ClipboardOutcome, SnipError> {
        let crop = self.pin_preview(&caller, &token, false)?;
        let result = (|| {
            self.validate_pinned_preview(&caller, &token)?;
            let retained = retained_bytes(&*self.lock()?);
            let outcome = self.clipboard.publish(&crop, retained);
            self.record_publication(&token, outcome.clone())?;
            Ok(outcome)
        })();
        self.unpin(&token);
        result
    }
    pub fn save(&self, caller: Caller, token: Token) -> Result<SaveOutcome, SnipError> {
        let crop = self.pin_preview(&caller, &token, true)?;
        let result = (|| {
            self.validate_pinned_preview(&caller, &token).map_err(|_| SaveCode::Failed)?;
            let Some(path) = self.picker.pick_png()? else { return Ok(SaveOutcome::Cancelled); };
            let retained = retained_bytes(&*self.lock().map_err(|_| SaveCode::Failed)?);
            let png = super::clipboard::png_rgba(crop.width, crop.height, &crop.pixels, retained).map_err(|_| SaveCode::Failed)?;
            // Check immediately before publication; save pin prevents replacement.
            self.validate_pinned_preview(&caller, &token).map_err(|_| SaveCode::Failed)?;
            self.publisher.publish_png(&path, &png)?;
            Ok(SaveOutcome::Saved)
        })().unwrap_or_else(SaveOutcome::Error);
        self.unpin(&token);
        Ok(result)
    }
    fn validate_pinned_preview(&self, caller: &Caller, token: &Token) -> Result<(), SnipError> {
        let (window, topology) = {
            let mut inner = self.lock()?;
            let current = active_mut(&mut inner, token)?;
            let preview = current.preview.as_ref().ok_or(SnipError::Inactive)?;
            if !current.busy || &preview.window.caller != caller { return Err(SnipError::Unauthorized); }
            (preview.window.clone(), preview.topology.clone())
        };
        if self.capture.snapshot()? != topology { return Err(SnipError::TopologyChanged); }
        if !self.windows.still_same(&window) { return Err(SnipError::Unauthorized); }
        // No adapter called while locked. Pin prevents coordinator replacement;
        // fresh native identity/topology checks precede this final token check.
        let mut inner = self.lock()?;
        active_mut(&mut inner, token)?;
        Ok(())
    }
    fn record_publication(&self, token: &Token, outcome: ClipboardOutcome) -> Result<(), SnipError> {
        let mut inner = self.lock()?;
        let current = active_mut(&mut inner, token)?;
        if !current.busy { return Err(SnipError::Stale); }
        inner.last_publication = Some(Publication { token: token.clone(), outcome });
        Ok(())
    }
    pub fn dismiss(&self, caller: Caller, token: Token) -> Result<bool, SnipError> {
        let _crop = self.pin_preview(&caller, &token, false)?;
        let current = {
            let mut inner = self.lock()?;
            active_mut(&mut inner, &token)?;
            self.diagnostic_state(None);
            inner.active.take()
        };
        if let Some(current) = current { self.dispose(current); }
        Ok(true)
    }
    fn live(&self, caller: &Caller) -> Result<(), SnipError> {
        if caller.window_id == 0 || !self.windows.still_same(&WindowRef { caller: caller.clone(), monitor_id: None }) { return Err(SnipError::Unauthorized); }
        Ok(())
    }
    fn pin_preview(&self, caller: &Caller, token: &Token, save: bool) -> Result<Arc<CroppedImage>, SnipError> {
        self.live(caller)?;
        let mut inner = self.lock()?;
        let current = active_mut(&mut inner, token)?;
        if current.busy { return Err(SnipError::Busy); }
        if current.phase != Phase::Preview { return Err(SnipError::Unauthorized); }
        let preview = current.preview.as_ref().ok_or(SnipError::Inactive)?;
        if &preview.window.caller != caller { return Err(SnipError::Unauthorized); }
        let crop = preview.crop.clone();
        current.busy = true;
        if save { current.phase = Phase::Saving; }
        self.diagnostic_state(Some(current));
        Ok(crop)
    }
    fn unpin(&self, token: &Token) {
        if let Ok(mut inner) = self.lock() { if let Ok(current) = active_mut(&mut inner, token) {
            current.busy = false;
            if current.phase == Phase::Saving { current.phase = Phase::Preview; }
            self.diagnostic_state(Some(current));
        } }
    }
    fn check_deadline(&self, token: &Token) -> Result<(), SnipError> {
        self.expire();
        let mut inner = self.lock()?;
        let current = active_mut(&mut inner, token)?;
        if self.clock.now() >= current.deadline { return Err(SnipError::ReadinessTimeout); }
        Ok(())
    }
    pub fn expire(&self) { self.expire_matching(None); }
    fn expire_matching(&self, overlay: Option<(&Caller, &Token)>) {
        let now = self.clock.now();
        let token = self.lock().ok().and_then(|mut inner| {
            let current = match overlay {
                Some((caller, expected)) => {
                    let current = active_mut(&mut inner, expected).ok()?;
                    if !current.windows.iter().any(|window| &window.caller == caller) { return None; }
                    current
                }
                None => inner.active.as_mut()?,
            };
            (!current.busy && (current.phase == Phase::Preparing || (current.phase == Phase::Selecting && !current.armed)) && now >= current.deadline)
                .then(|| current.token.clone())
        });
        if let Some(token) = token { self.rollback(&token); }
    }
    /// Native-only cancellation: phase/token admission and removal share one lock.
    /// A completion already holding the mutation pin wins over cancellation.
    pub(crate) fn cancel_selecting(&self, token: &Token) -> bool {
        let removed = {
            let Ok(mut inner) = self.lock() else { return false; };
            let Ok(current) = active_mut(&mut inner, token) else { self.diagnostic(DiagnosticEvent::CancelRejected); return false; };
            self.diagnostic_state(Some(current));
            if current.phase != Phase::Selecting { self.diagnostic(DiagnosticEvent::CancelRejected); return false; }
            if current.busy {
                // Only arming has Selecting + !armed + busy. Completion requires
                // armed=true and must continue to win over cancellation.
                if current.armed || current.escape_after_arming { self.diagnostic(DiagnosticEvent::CancelRejected); return false; }
                current.escape_after_arming = true;
                self.diagnostic(DiagnosticEvent::CancelDeferred); self.diagnostic_state(Some(current));
                return true;
            }
            // No unlock between admission and extraction: arming/completion
            // cannot acquire a new pin after Escape accepted this idle selection.
            self.extract_rollback(&mut inner)
        };
        self.diagnostic(DiagnosticEvent::CancelAccepted);
        self.finish_rollback(removed);
        true
    }
    /// Lock-free observation only, never cancellation authority. Published under
    /// the same lock as phase changes; generations are never reused.
    pub(crate) fn observed_selection_generation(&self) -> u64 {
        self.selecting_generation.load(Ordering::Acquire)
    }
    /// Deferred native work resolves the token only for its observed generation.
    pub(crate) fn selection_token(&self, generation: u64) -> Option<Token> {
        let inner = self.lock().ok()?;
        inner.active.as_ref().filter(|current| current.phase == Phase::Selecting
            && current.token.generation.parse::<u64>().ok() == Some(generation)).map(|current| current.token.clone())
    }
    fn rollback_completion(&self, token: &Token) {
        let removed = {
            let Ok(mut inner) = self.lock() else { return; };
            let Ok(current) = active_mut(&mut inner, token) else { return; };
            if current.phase != Phase::Selecting || !current.busy || !current.armed { return; }
            self.extract_rollback(&mut inner)
        };
        self.finish_rollback(removed);
    }
    fn rollback(&self, token: &Token) { self.rollback_matching(token, false, false); }
    fn rollback_matching(&self, token: &Token, selecting_only: bool, finishing_arming: bool) -> bool {
        let removed = {
            let Ok(mut inner) = self.lock() else { return false; };
            if !inner.active.as_ref().is_some_and(|current| &current.token == token
                && (if finishing_arming { current.busy && !current.armed && current.escape_after_arming } else { !current.busy })
                && (!selecting_only || current.phase == Phase::Selecting)) { return false; }
            self.extract_rollback(&mut inner)
        };
        if finishing_arming { self.diagnostic(DiagnosticEvent::CancelAccepted); }
        self.finish_rollback(removed);
        true
    }
    /// Caller retains the authoritative state lock across admission/extraction.
    fn extract_rollback(&self, inner: &mut Inner) -> (Option<Current>, Option<WindowRef>) {
        let removed = inner.active.take();
        self.selecting_generation.store(0, Ordering::Release);
        let restored = inner.previous.take();
        let restore_window = restored.as_ref().map(|previous| previous.window.clone());
        if let Some(previous) = restored {
            inner.active = Some(Current { token: previous.token.clone(), phase: Phase::Preview, deadline: self.clock.now(),
                frames: Vec::new(), windows: Vec::new(), ready: Vec::new(), armed: false, selected: None, preview: Some(previous), busy: false, escape_after_arming: false });
        }
        self.diagnostic_state(inner.active.as_ref());
        self.preview_changed.notify_all();
        (removed, restore_window)
    }
    /// Native retirement/close/show effects run only after releasing state lock.
    fn finish_rollback(&self, removed: (Option<Current>, Option<WindowRef>)) {
        if let Some(current) = removed.0 { self.dispose(current); }
        if let Some(window) = removed.1 { let _ = self.windows.show(&window); }
    }
    fn dispose(&self, current: Current) {
        self.windows.retire_generation(&current.token);
        for window in current.windows { self.windows.close(window); }
        if let Some(preview) = current.preview { self.windows.close(preview.window); }
    }
    pub fn inspect(&self) -> Snapshot {
        let Ok(inner) = self.lock() else { return Snapshot { phase: None, token: None, frame_bytes: 0, image_bytes: 0, window_ids: Vec::new(), readiness_count: 0, save_pin: false, last_publication: None }; };
        let current = inner.active.as_ref();
        Snapshot { phase: current.map(|c| c.phase), token: current.map(|c| c.token.clone()),
            frame_bytes: current.map_or(0, |c| c.frames.iter().map(|frame| frame.rgba.len()).sum()),
            image_bytes: current.and_then(|c| c.preview.as_ref()).map_or(0, |p| p.crop.pixels.len()) + inner.previous.as_ref().map_or(0, |p| p.crop.pixels.len()),
            window_ids: current.into_iter().flat_map(|c| c.windows.iter().map(|w| w.caller.window_id).chain(c.preview.iter().map(|p| p.window.caller.window_id)))
                .chain(inner.previous.iter().map(|p| p.window.caller.window_id)).collect(),
            readiness_count: current.map_or(0, |c| c.ready.len()), save_pin: current.is_some_and(|c| c.phase == Phase::Saving), last_publication: inner.last_publication.clone() }
    }
    pub fn retire_for_shutdown(&self) -> Result<(), SnipError> {
        let (active, previous) = {
            let mut inner = self.lock()?;
            if inner.image_reservations != 0 || inner.active.as_ref().is_some_and(|current| current.busy) { return Err(SnipError::Busy); }
            (inner.active.take(), inner.previous.take())
        };
        if let Some(current) = active { self.dispose(current); }
        if let Some(previous) = previous { self.windows.retire_generation(&previous.token); self.windows.close(previous.window); }
        Ok(())
    }
}
impl Drop for Coordinator {
    fn drop(&mut self) {
        if let Ok(inner) = self.inner.get_mut() {
            if let Some(current) = inner.active.take() {
                for window in current.windows { self.windows.close(window); }
                if let Some(preview) = current.preview { self.windows.close(preview.window); }
            }
            if let Some(previous) = inner.previous.take() { self.windows.close(previous.window); }
        }
    }
}
fn active_mut<'a>(inner: &'a mut Inner, token: &Token) -> Result<&'a mut Current, SnipError> {
    if token.generation.is_empty() || token.generation.starts_with('0') || token.generation.len() > 20
        || !token.generation.bytes().all(|b| b.is_ascii_digit()) || token.generation.parse::<u64>().is_err()
        || token.capture_id.len() != 32 || !token.capture_id.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) { return Err(SnipError::Stale); }
    let current = inner.active.as_mut().ok_or(SnipError::Inactive)?;
    let difference = current.token.capture_id.bytes().zip(token.capture_id.bytes()).fold(0u8, |difference, (a,b)| difference | (a ^ b));
    if current.token.generation != token.generation || difference != 0 { return Err(SnipError::Stale); }
    Ok(current)
}
fn authorize_overlay(current: &Current, caller: &Caller, monitor: &str) -> Result<(), SnipError> {
    if current.phase != Phase::Preparing && current.phase != Phase::Selecting { return Err(SnipError::Unauthorized); }
    if monitor.len() > 3 || !current.windows.iter().any(|window| &window.caller == caller && window.monitor_id.as_deref() == Some(monitor)) {
        return Err(SnipError::Unauthorized);
    }
    Ok(())
}
fn retained_bytes(inner: &Inner) -> usize {
    inner.active.as_ref().map_or(0, |current| current.frames.iter().map(|frame| frame.rgba.capacity()).sum::<usize>()
        + current.preview.as_ref().map_or(0, |preview| preview.crop.pixels.capacity()))
        + inner.previous.as_ref().map_or(0, |preview| preview.crop.pixels.capacity()) + inner.image_reservations
}
fn context_for(token: &Token, phase: Phase, monitor: &MonitorSnapshot, (width, height): (u32, u32)) -> Context {
    Context { token: token.clone(), phase, monitor_id: monitor.id.clone(), width, height,
        scale_factor: monitor.scale, origin_x: monitor.rect.left, origin_y: monitor.rect.top }
}
fn validate_monitors(monitors: &[MonitorSnapshot], retained: usize) -> Result<(), SnipError> {
    if monitors.is_empty() || monitors.len() > 32 { return Err(SnipError::CaptureFailed); }
    let mut total = 0usize;
    let mut largest = 0usize;
    for (index, monitor) in monitors.iter().enumerate() {
        if monitor.id != format!("m{index}") || monitor.identity == 0 || !monitor.scale.is_finite() || monitor.scale <= 0.0 { return Err(SnipError::CaptureFailed); }
        let bytes = geometry::checked_monitor_len(monitor.rect, MAX_FROZEN_BYTES).map_err(|_| SnipError::CaptureFailed)?;
        total = total.checked_add(bytes).ok_or(SnipError::CaptureFailed)?;
        largest = largest.max(bytes);
    }
    // Native capture retains the frozen buffers while one GDI DIB is live.
    // Include that transient alongside retired image-read reservations BEFORE freeze.
    if total > MAX_FROZEN_BYTES || total.checked_add(largest).and_then(|n| n.checked_add(retained)).is_none_or(|n| n > MAX_STAGING_BYTES) { return Err(SnipError::CaptureFailed); }
    Ok(())
}
fn random_token() -> Result<String, SnipError> {
    #[link(name = "bcrypt")]
    unsafe extern "system" { fn BCryptGenRandom(algorithm: *mut std::ffi::c_void, buffer: *mut u8, len: u32, flags: u32) -> i32; }
    let mut bytes = [0u8;16];
    if unsafe { BCryptGenRandom(std::ptr::null_mut(), bytes.as_mut_ptr(), 16, 2) } < 0 { return Err(SnipError::Inactive); }
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
