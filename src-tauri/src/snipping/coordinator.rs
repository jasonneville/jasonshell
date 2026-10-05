//! Adapter-facing production lifecycle. Native effects occur outside the state lock.
use super::{geometry::{self, CroppedImage, LogicalRect, PhysicalRect, MAX_FROZEN_BYTES, MAX_STAGING_BYTES}, save::SaveError};
pub use super::session::Caller;
pub use super::clipboard_process::Outcome as ClipboardOutcome;
use std::{path::{Path, PathBuf}, sync::{Arc, Mutex, MutexGuard}, time::{Duration, Instant}};

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
    fn create_overlay(&self, token: &Token, monitor: &MonitorSnapshot) -> Result<WindowRef, SnipError>;
    fn create_preview(&self, token: &Token, monitor: &MonitorSnapshot, width: u32, height: u32) -> Result<WindowRef, SnipError>;
    fn emit_context(&self, window: &WindowRef, context: &Context) -> Result<(), SnipError>;
    fn emit_armed(&self, window: &WindowRef, token: &Token) -> Result<(), SnipError>;
    fn show(&self, window: &WindowRef) -> Result<(), SnipError>;
    fn hide(&self, window: &WindowRef) -> Result<(), SnipError>;
    fn close(&self, window: WindowRef);
    fn still_same(&self, window: &WindowRef) -> bool;
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

struct Preview { token: Token, window: WindowRef, crop: Arc<CroppedImage>, monitor: MonitorSnapshot, topology: Vec<MonitorSnapshot> }
struct Current {
    token: Token, phase: Phase, deadline: Instant, frames: Vec<Arc<Frame>>, windows: Vec<WindowRef>,
    ready: Vec<String>, armed: bool, selected: Option<String>, preview: Option<Preview>, busy: bool,
}
#[derive(Default)]
struct Inner { next: u64, active: Option<Current>, previous: Option<Preview>, last_publication: Option<Publication> }
pub struct Coordinator {
    capture: Arc<dyn CapturePort>, windows: Arc<dyn WindowPort>, clipboard: Arc<dyn ClipboardPort>,
    picker: Arc<dyn PickerPort>, publisher: Arc<dyn PublisherPort>, clock: Arc<dyn ClockPort>, inner: Mutex<Inner>,
}
impl Coordinator {
    pub fn new(capture: Arc<dyn CapturePort>, windows: Arc<dyn WindowPort>, clipboard: Arc<dyn ClipboardPort>, picker: Arc<dyn PickerPort>, publisher: Arc<dyn PublisherPort>, clock: Arc<dyn ClockPort>) -> Self {
        Self { capture, windows, clipboard, picker, publisher, clock, inner: Mutex::new(Inner::default()) }
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
            let retained = inner.previous.as_ref().map_or(0, |previous| previous.crop.pixels.capacity());
            let old_window = inner.previous.as_ref().map(|previous| previous.window.clone());
            let token = Token { generation: generation.to_string(), capture_id };
            inner.active = Some(Current { token: token.clone(), phase: Phase::Preparing, deadline: now + Duration::from_secs(5),
                frames: Vec::new(), windows: Vec::new(), ready: Vec::new(), armed: false, selected: None, preview: None, busy: false });
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
            let mut inner = self.lock()?;
            active_mut(&mut inner, &token)?.phase = Phase::Selecting;
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
        self.live(&caller)?;
        enum Source { Frame(Arc<Frame>), Crop(Arc<CroppedImage>) }
        let (source, retained) = {
            let mut inner = self.lock()?;
            let retained = retained_bytes(&inner);
            let current = active_mut(&mut inner, &token)?;
            if current.busy { return Err(SnipError::Busy); }
            let source = if let Some(preview) = &current.preview {
                if preview.window.caller != caller || preview.monitor.id != monitor_id { return Err(SnipError::Unauthorized); }
                Source::Crop(preview.crop.clone())
            } else {
                authorize_overlay(current, &caller, &monitor_id)?;
                Source::Frame(current.frames.iter().find(|frame| frame.monitor.id == monitor_id).cloned().ok_or(SnipError::Unauthorized)?)
            };
            current.busy = true;
            (source, retained)
        };
        let encoded = match &source {
            Source::Frame(frame) => frame.monitor.rect.dimensions().map_err(|_| SnipError::ImageFailed)
                .and_then(|(width, height)| super::clipboard::png_rgba(width, height, &frame.rgba, retained).map_err(|_| SnipError::ImageFailed)),
            Source::Crop(crop) => super::clipboard::png_rgba(crop.width, crop.height, &crop.pixels, retained).map_err(|_| SnipError::ImageFailed),
        };
        self.unpin(&token);
        self.expire();
        { let mut inner = self.lock()?; active_mut(&mut inner, &token)?; }
        encoded
    }
    pub fn ready(&self, caller: Caller, token: Token, monitor_id: String) -> Result<bool, SnipError> {
        self.live(&caller)?;
        self.check_deadline(&token)?;
        let windows = {
            let mut inner = self.lock()?;
            let current = active_mut(&mut inner, &token)?;
            authorize_overlay(current, &caller, &monitor_id)?;
            if current.armed { return Ok(true); }
            if current.phase != Phase::Selecting || current.busy { return Err(SnipError::Busy); }
            if !current.ready.contains(&monitor_id) { current.ready.push(monitor_id); }
            if current.ready.len() != current.windows.len() { return Ok(false); }
            current.busy = true;
            current.windows.clone()
        };
        let result = (|| {
            for window in &windows { self.windows.show(window)?; }
            for window in &windows { self.windows.emit_armed(window, &token)?; }
            Ok(())
        })();
        self.unpin(&token);
        if let Err(error) = result { self.rollback(&token); return Err(error); }
        self.check_deadline(&token)?;
        let mut inner = self.lock()?;
        active_mut(&mut inner, &token)?.armed = true;
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
                        current.preview = Some(Preview { token: token.clone(), window: window.clone(), crop: crop.clone(), monitor: frame.monitor.clone(), topology: topology.clone() });
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
                current.frames.clear();
                let overlays = std::mem::take(&mut current.windows);
                (inner.previous.take(), overlays)
            };
            for overlay in overlays { self.windows.close(overlay); }
            if let Some(old) = old { self.windows.close(old.window); }
            Ok(true)
        })();
        self.unpin(&token);
        if result.is_err() { self.rollback(&token); }
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
            let outcome = self.clipboard.publish(&crop, crop.pixels.capacity());
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
            let png = super::clipboard::png_rgba(crop.width, crop.height, &crop.pixels, crop.pixels.capacity()).map_err(|_| SaveCode::Failed)?;
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
        Ok(crop)
    }
    fn unpin(&self, token: &Token) {
        if let Ok(mut inner) = self.lock() { if let Ok(current) = active_mut(&mut inner, token) {
            current.busy = false;
            if current.phase == Phase::Saving { current.phase = Phase::Preview; }
        } }
    }
    fn check_deadline(&self, token: &Token) -> Result<(), SnipError> {
        self.expire();
        let mut inner = self.lock()?;
        let current = active_mut(&mut inner, token)?;
        if self.clock.now() >= current.deadline { return Err(SnipError::ReadinessTimeout); }
        Ok(())
    }
    pub fn expire(&self) {
        let now = self.clock.now();
        let token = self.lock().ok().and_then(|inner| inner.active.as_ref()
            .filter(|current| !current.busy && (current.phase == Phase::Preparing || (current.phase == Phase::Selecting && !current.armed)) && now >= current.deadline)
            .map(|current| current.token.clone()));
        if let Some(token) = token { self.rollback(&token); }
    }
    fn rollback(&self, token: &Token) {
        let removed = {
            let Ok(mut inner) = self.lock() else { return; };
            if !inner.active.as_ref().is_some_and(|current| &current.token == token && !current.busy) { return; }
            let removed = inner.active.take();
            let restored = inner.previous.take();
            let restore_window = restored.as_ref().map(|previous| previous.window.clone());
            if let Some(previous) = restored {
                inner.active = Some(Current { token: previous.token.clone(), phase: Phase::Preview, deadline: self.clock.now(),
                    frames: Vec::new(), windows: Vec::new(), ready: Vec::new(), armed: false, selected: None, preview: Some(previous), busy: false });
            }
            (removed, restore_window)
        };
        if let Some(current) = removed.0 { self.dispose(current); }
        if let Some(window) = removed.1 { let _ = self.windows.show(&window); }
    }
    fn dispose(&self, current: Current) {
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
        + inner.previous.as_ref().map_or(0, |preview| preview.crop.pixels.capacity())
}
fn context_for(token: &Token, phase: Phase, monitor: &MonitorSnapshot, (width, height): (u32, u32)) -> Context {
    Context { token: token.clone(), phase, monitor_id: monitor.id.clone(), width, height,
        scale_factor: monitor.scale, origin_x: monitor.rect.left, origin_y: monitor.rect.top }
}
fn validate_monitors(monitors: &[MonitorSnapshot], retained: usize) -> Result<(), SnipError> {
    if monitors.is_empty() || monitors.len() > 32 { return Err(SnipError::CaptureFailed); }
    let mut total = 0usize;
    for (index, monitor) in monitors.iter().enumerate() {
        if monitor.id != format!("m{index}") || monitor.identity == 0 || !monitor.scale.is_finite() || monitor.scale <= 0.0 { return Err(SnipError::CaptureFailed); }
        total = total.checked_add(geometry::checked_monitor_len(monitor.rect, MAX_FROZEN_BYTES).map_err(|_| SnipError::CaptureFailed)?).ok_or(SnipError::CaptureFailed)?;
    }
    if total > MAX_FROZEN_BYTES || total.checked_add(retained).is_none_or(|n| n > MAX_STAGING_BYTES) { return Err(SnipError::CaptureFailed); }
    Ok(())
}
fn random_token() -> Result<String, SnipError> {
    #[link(name = "bcrypt")]
    unsafe extern "system" { fn BCryptGenRandom(algorithm: *mut std::ffi::c_void, buffer: *mut u8, len: u32, flags: u32) -> i32; }
    let mut bytes = [0u8;16];
    if unsafe { BCryptGenRandom(std::ptr::null_mut(), bytes.as_mut_ptr(), 16, 2) } < 0 { return Err(SnipError::Inactive); }
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
