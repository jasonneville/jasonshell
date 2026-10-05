//! Managed production coordinator and path-free, concrete-instance IPC boundary.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/snipping_runtime_acceptance.rs"));
use super::{coordinator::*, foreground::{ForegroundObserver, Intent}, native::*, native_registry::NativeRegistry};
use std::{collections::BTreeMap, sync::{Arc, Mutex, Condvar, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
use tauri::{AppHandle, Emitter, State, WebviewWindow};
use serde_json::{Value, json};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::{GetForegroundWindow, IsWindowVisible}};
struct Pending { token: Token, bar: Caller, deadline: Instant, ack: bool }
/// Narrow native boundaries shared by production and cfg(test) runtime fixtures.
/// Implementations must fail closed; runtime owns all token/ack/reservation state.
pub(crate) trait RuntimeEdges: Send + Sync {
    fn intent(&self) -> Result<Intent, SnipError>;
    fn valid(&self, intent: Intent) -> bool;
    fn bar(&self) -> Result<(), SnipError>;
    fn emit(&self, event: &str, token: &Token) -> Result<(), SnipError>;
    fn flush(&self, deadline: Instant) -> Result<(), SnipError>;
    fn now(&self) -> Instant;
    fn foreground(&self) -> isize;
    fn restore(&self, intent: Intent, owned_foreground: isize);
    fn shutdown(&self);
    fn publication_attempts(&self) -> Option<usize> { None }
}
struct NativeEdges { registry: Arc<NativeRegistry>, bar: Caller, observer: ForegroundObserver, clipboard: Arc<NativeClipboardPort> }
impl NativeEdges {
    fn window(&self) -> Result<WebviewWindow, SnipError> {
        self.registry.entry(&WindowRef { caller: self.bar.clone(), monitor_id: None })?.window()
    }
}
impl RuntimeEdges for NativeEdges {
    fn intent(&self) -> Result<Intent, SnipError> { self.observer.snapshot() }
    fn valid(&self, intent: Intent) -> bool { self.observer.valid(intent) }
    fn bar(&self) -> Result<(), SnipError> {
        let window = self.window()?;
        if !unsafe { IsWindowVisible(HWND(window.hwnd().map_err(|_| SnipError::Unauthorized)?.0)) }.as_bool() { return Err(SnipError::Inactive); }
        Ok(())
    }
    fn emit(&self, event: &str, token: &Token) -> Result<(), SnipError> { self.window()?.emit(event, token_json(token)).map_err(|_| SnipError::WindowFailed) }
    fn flush(&self, deadline: Instant) -> Result<(), SnipError> { super::composition::flush(deadline) }
    fn now(&self) -> Instant { Instant::now() }
    fn foreground(&self) -> isize { unsafe { GetForegroundWindow() }.0 as isize }
    fn restore(&self, intent: Intent, owned_foreground: isize) { self.observer.restore_validated(intent, owned_foreground); }
    fn shutdown(&self) { self.clipboard.shutdown(); self.observer.stop(); }
    fn publication_attempts(&self) -> Option<usize> { Some(self.clipboard.publication_attempts()) }
}
pub struct SnipRuntime {
    pub coordinator: Coordinator,
    pub registry: Arc<NativeRegistry>,
    bar: Caller,
    edges: Arc<dyn RuntimeEdges>,
    original: Mutex<Option<Intent>>,
    originals: Mutex<BTreeMap<String, (String, Intent)>>,
    pending: Mutex<Option<Pending>>,
    changed: Condvar,
    starting: AtomicBool,
    stopped: AtomicBool,
    preparation_deadline: Mutex<Option<Instant>>,
}
/// Native-only intent/reservation ticket. Drop releases repeat suppression even if
/// the hotkey owner cannot dispatch its worker. Never serialized or accepted by IPC.
pub struct NativeStartReservation { runtime: Arc<SnipRuntime>, intent: Intent, deadline: Instant }
impl Drop for NativeStartReservation { fn drop(&mut self) { self.runtime.starting.store(false, Ordering::Release); } }
impl SnipRuntime {
    pub(crate) fn publication_attempts(&self) -> Option<usize> { self.edges.publication_attempts() }
    fn assemble(coordinator: Coordinator, registry: Arc<NativeRegistry>, bar: Caller, edges: Arc<dyn RuntimeEdges>) -> Self {
        Self { coordinator, registry, bar, edges, original: Mutex::new(None), originals: Mutex::new(BTreeMap::new()), pending: Mutex::new(None), changed: Condvar::new(), starting: AtomicBool::new(false), stopped: AtomicBool::new(false), preparation_deadline: Mutex::new(None) }
    }
    /// Called after shell window construction and before normal activation.
    pub fn install(app: AppHandle, top: WebviewWindow) -> Result<Arc<Self>, SnipError> {
        let observer = ForegroundObserver::start()?;
        let picker = Arc::new(NativePickerPort::new(app.clone()));
        Self::install_native_ports(app, top, observer, picker, None, Box::new(|prepare, release| Arc::new(NativeCapturePort::with_release(prepare, release))))
    }
    /// Native construction seam for the standalone, synthetic-only example.
    /// Runtime, commands, registry, window ports, composition and clipboard remain
    /// production implementations. Only declared capture/picker boundaries vary.
    pub(crate) fn install_native_ports(app: AppHandle, top: WebviewWindow, observer: ForegroundObserver, picker: Arc<dyn PickerPort>, data_directory: Option<std::path::PathBuf>,
        capture_factory: Box<dyn FnOnce(PreparationBarrier, Arc<dyn Fn() + Send + Sync>) -> Arc<dyn CapturePort>>) -> Result<Arc<Self>, SnipError> {
        let registry = Arc::new(NativeRegistry::default());
        let entry = registry.reserve("top-bar".into(), None, None, false)?;
        entry.bind(top)?; let bar = entry.reference.caller.clone();
        let clipboard = Arc::new(NativeClipboardPort::new()?);
        let edges: Arc<dyn RuntimeEdges> = Arc::new(NativeEdges { registry: registry.clone(), bar: bar.clone(), observer, clipboard: clipboard.clone() });
        let runtime: Arc<Self> = Arc::new_cyclic(|weak: &std::sync::Weak<Self>| {
            let prepare = weak.clone(); let release = weak.clone();
            let closing = weak.clone();
            let windows = Arc::new(NativeWindowPort::with_close(app.clone(), registry.clone(), Arc::new(move |token, hwnd| {
                if let Some(runtime) = closing.upgrade() { runtime.restore_if_owned(token, hwnd); }
            })).with_data_directory(data_directory));
            let capture = capture_factory(Arc::new(move |deadline| prepare.upgrade().ok_or(SnipError::Inactive)?.prepare(deadline)),
                Arc::new(move || { if let Some(runtime) = release.upgrade() { runtime.release(); } }));
            Self::assemble(Coordinator::new(capture, windows, clipboard.clone(), picker, Arc::new(NativePublisherPort), Arc::new(NativeClock)), registry, bar, edges)
        });
        let weak = Arc::downgrade(&runtime);
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(50));
            let Some(runtime) = weak.upgrade() else { break; };
            if runtime.stopped.load(Ordering::Acquire) { break; }
            runtime.coordinator.expire();
        });
        Ok(runtime)
    }
    /// Native hotkey owner may call this from a worker without renderer mediation.
    pub fn start_native(&self) -> Result<Token, SnipError> { self.start(self.bar.clone()) }
    /// Hotkey message thread calls BEFORE spawning a worker or emitting UI events.
    pub fn reserve_native_start(self: &Arc<Self>) -> Result<NativeStartReservation, SnipError> {
        let (intent, deadline) = self.reserve_intent(self.bar.clone())?;
        Ok(NativeStartReservation { runtime: self.clone(), intent, deadline })
    }
    /// Worker consumes the exact native reservation; never recaches activated UI.
    pub fn run_native_start(self: &Arc<Self>, reservation: NativeStartReservation) -> Result<Token, SnipError> {
        if !Arc::ptr_eq(self, &reservation.runtime) { return Err(SnipError::Unauthorized); }
        self.run_reserved(self.bar.clone(), reservation.intent, reservation.deadline)
    }
    fn start(&self, caller: Caller) -> Result<Token, SnipError> {
        let (intent, deadline) = self.reserve_intent(caller.clone())?;
        struct Pin<'a>(&'a AtomicBool); impl Drop for Pin<'_> { fn drop(&mut self) { self.0.store(false, Ordering::Release); } }
        let _pin = Pin(&self.starting);
        self.run_reserved(caller, intent, deadline)
    }
    fn reserve_intent(&self, caller: Caller) -> Result<(Intent, Instant), SnipError> {
        if caller != self.bar || self.stopped.load(Ordering::Acquire) { return Err(SnipError::Unauthorized); }
        if self.starting.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() { return Err(SnipError::Busy); }
        let result = (|| {
            self.edges.bar()?;
            let snapshot = self.coordinator.inspect();
            if snapshot.phase.is_some_and(|phase| phase != Phase::Preview) || snapshot.save_pin { return Err(SnipError::Busy); }
            let deadline = self.edges.now() + Duration::from_secs(5);
            self.edges.intent().map(|intent| (intent, deadline))
        })();
        if result.is_err() { self.starting.store(false, Ordering::Release); }
        result
    }
    fn run_reserved(&self, caller: Caller, intent: Intent, deadline: Instant) -> Result<Token, SnipError> {
        if self.stopped.load(Ordering::Acquire) || !self.edges.valid(intent) { return Err(SnipError::Inactive); }
        if self.edges.now() >= deadline { return Err(SnipError::ReadinessTimeout); }
        *self.original.lock().map_err(|_| SnipError::Inactive)? = Some(intent);
        *self.preparation_deadline.lock().map_err(|_| SnipError::Inactive)? = Some(deadline);
        let result = self.coordinator.start(caller);
        self.release();
        result
    }
    fn prepare(&self, deadline: Instant) -> Result<(), SnipError> {
        let deadline = deadline.min(self.preparation_deadline.lock().map_err(|_| SnipError::Inactive)?.ok_or(SnipError::Inactive)?);
        let token = self.coordinator.inspect().token.ok_or(SnipError::Inactive)?;
        self.edges.bar()?;
        let original = self.original.lock().map_err(|_| SnipError::Inactive)?.ok_or(SnipError::Inactive)?;
        if !self.edges.valid(original) { return Err(SnipError::Inactive); }
        { let mut originals = self.originals.lock().map_err(|_| SnipError::Inactive)?;
            if originals.len() >= 64 { originals.pop_first(); }
            if originals.insert(token.generation.clone(), (token.capture_id.clone(), original)).is_some() { return Err(SnipError::Stale); }
        }
        { let mut pending = self.pending.lock().map_err(|_| SnipError::Inactive)?;
            if pending.is_some() { return Err(SnipError::Busy); }
            *pending = Some(Pending { token: token.clone(), bar: self.bar.clone(), deadline, ack: false }); }
        self.edges.emit("snip:prepare_bar", &token)?;
        let mut pending = self.pending.lock().map_err(|_| SnipError::Inactive)?;
        loop {
            if self.stopped.load(Ordering::Acquire) { return Err(SnipError::Inactive); }
            let current = pending.as_ref().ok_or(SnipError::Stale)?;
            if current.token != token { return Err(SnipError::Stale); }
            if current.ack { break; }
            let remaining = deadline.saturating_duration_since(self.edges.now());
            if remaining.is_zero() { return Err(SnipError::ReadinessTimeout); }
            pending = self.changed.wait_timeout(pending, remaining).map_err(|_| SnipError::Inactive)?.0;
        }
        drop(pending);
        self.edges.bar()?;
        if self.coordinator.inspect().token.as_ref() != Some(&token) || !self.edges.valid(original) { return Err(SnipError::Stale); }
        self.edges.flush(deadline)?;
        self.edges.bar()?;
        // The native observer may lose continuity or see a different external
        // foreground while composition is deferred. Never freeze using that old
        // intent simply because it was valid before entering the compositor.
        if !self.edges.valid(original) || self.stopped.load(Ordering::Acquire) { return Err(SnipError::Inactive); }
        if self.edges.now() >= deadline || self.coordinator.inspect().token.as_ref() != Some(&token) { return Err(SnipError::ReadinessTimeout); }
        Ok(())
    }
    fn ack(&self, caller: Caller, token: Token) -> Result<Value, SnipError> {
        if caller != self.bar { return Err(SnipError::Unauthorized); }
        self.edges.bar()?;
        if self.coordinator.inspect().token.as_ref() != Some(&token) { return Err(SnipError::Stale); }
        let mut pending = self.pending.lock().map_err(|_| SnipError::Inactive)?;
        let current = pending.as_mut().ok_or(SnipError::Stale)?;
        if current.bar != caller || current.token != token || current.ack || self.edges.now() >= current.deadline { return Err(SnipError::Stale); }
        current.ack = true; self.changed.notify_all(); Ok(ack_json(&token, "accepted", true))
    }
    fn release(&self) {
        let pending = self.pending.lock().ok().and_then(|mut pending| pending.take());
        self.changed.notify_all();
        if let Some(pending) = pending { let _ = self.edges.emit("snip:release_bar", &pending.token); }
    }
    fn restore_if_owned(&self, token: Token, closing_hwnd: isize) {
        // Only called for an exact owned instance being disposed by the coordinator.
        // Check again AFTER observer validation so a user's foreground change wins.
        if self.edges.foreground() != closing_hwnd { return; }
        let original = self.originals.lock().ok().and_then(|originals| originals.get(&token.generation).filter(|(capture_id, _)| *capture_id == token.capture_id).map(|(_, intent)| *intent));
        if let Some(intent) = original {
            if self.edges.valid(intent) && self.edges.foreground() == closing_hwnd {
                self.edges.restore(intent, closing_hwnd);
            }
        }
    }
    pub fn shutdown(&self) { self.stopped.store(true, Ordering::Release); self.release(); self.registry.close_snips(); self.edges.shutdown(); }
}
/// Test construction replaces only native edges and the six existing coordinator
/// ports. All reservation, prepare/ack/release, token checks and shutdown code above
/// remains the actual production SnipRuntime, not a second state machine.
#[cfg(test)]
pub(crate) mod verification {
    use super::*;
    pub(crate) type CaptureFactory = Box<dyn FnOnce(PreparationBarrier, Arc<dyn Fn() + Send + Sync>) -> Arc<dyn CapturePort>>;
    pub(crate) struct Ports {
        pub capture: CaptureFactory,
        pub windows: Arc<dyn WindowPort>,
        pub clipboard: Arc<dyn ClipboardPort>,
        pub picker: Arc<dyn PickerPort>,
        pub publisher: Arc<dyn PublisherPort>,
        pub clock: Arc<dyn ClockPort>,
    }
    pub(crate) fn construct(bar: Caller, registry: Arc<NativeRegistry>, edges: Arc<dyn RuntimeEdges>, ports: Ports) -> Arc<SnipRuntime> {
        Arc::new_cyclic(|weak: &std::sync::Weak<SnipRuntime>| {
            let prepare = weak.clone(); let release = weak.clone();
            let capture = (ports.capture)(Arc::new(move |deadline| prepare.upgrade().ok_or(SnipError::Inactive)?.prepare(deadline)),
                Arc::new(move || { if let Some(runtime) = release.upgrade() { runtime.release(); } }));
            SnipRuntime::assemble(Coordinator::new(capture, ports.windows, ports.clipboard, ports.picker, ports.publisher, ports.clock), registry, bar, edges)
        })
    }
    pub(crate) fn start(runtime: &SnipRuntime, caller: Caller) -> Result<Token, SnipError> { runtime.start(caller) }
    pub(crate) fn ack(runtime: &SnipRuntime, caller: Caller, token: Token) -> Result<Value, SnipError> { runtime.ack(caller, token) }
    pub(crate) fn pending(runtime: &SnipRuntime) -> Option<(Token, Caller, Instant, bool)> {
        runtime.pending.lock().ok().and_then(|pending| pending.as_ref().map(|pending| (pending.token.clone(), pending.bar.clone(), pending.deadline, pending.ack)))
    }
    pub(crate) fn wake(runtime: &SnipRuntime) { runtime.changed.notify_all(); }
    pub(crate) fn restore(runtime: &SnipRuntime, token: Token, owned_hwnd: isize) { runtime.restore_if_owned(token, owned_hwnd); }
}
fn token_json(token: &Token) -> Value { json!({"generation":token.generation,"captureId":token.capture_id}) }
fn ack_json(token: &Token, field: &str, accepted: bool) -> Value { let mut value = token_json(token); value[field] = json!(accepted); value }
fn token(generation: String, capture_id: String) -> Result<Token, SnipError> {
    if generation.len() > 20 || generation.parse::<u64>().ok().filter(|n| *n > 0).is_none_or(|n| n.to_string() != generation)
        || capture_id.len() != 32 || !capture_id.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) { return Err(SnipError::Stale); }
    Ok(Token { generation, capture_id })
}
fn monitor(id: String) -> Result<String, SnipError> {
    if id.len() > 3 || id.strip_prefix('m').and_then(|s| s.parse::<u8>().ok()).filter(|n| *n < 32).is_none_or(|n| format!("m{n}") != id) { Err(SnipError::Unauthorized) } else { Ok(id) }
}
fn error(error: SnipError) -> String { match error { SnipError::Inactive=>"inactive",SnipError::Unauthorized=>"unauthorized",SnipError::Stale=>"stale",SnipError::Busy=>"busy",SnipError::InvalidSelection=>"invalid-selection",SnipError::TopologyChanged=>"topology-changed",SnipError::CaptureFailed=>"capture-failed",SnipError::ReadinessTimeout=>"readiness-timeout",SnipError::WindowFailed=>"window-failed",SnipError::ImageFailed=>"image-failed" }.into() }
async fn worker<T: Send + 'static>(runtime: Arc<SnipRuntime>, window: WebviewWindow, action: impl FnOnce(&SnipRuntime, Caller) -> Result<T, SnipError> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if runtime.stopped.load(Ordering::Acquire) { return Err(SnipError::Inactive); }
        let caller = runtime.registry.resolve(&window)?; action(&runtime, caller)
    }).await.map_err(|_| "inactive".to_string())?.map_err(error)
}
#[tauri::command]
pub async fn start_snip(window: WebviewWindow, state: State<'_, Arc<SnipRuntime>>) -> Result<Value, String> { worker(state.inner().clone(), window, |runtime, caller| runtime.start(caller).map(|token| token_json(&token))).await }
#[tauri::command]
pub async fn get_snip_context(window: WebviewWindow, state: State<'_, Arc<SnipRuntime>>) -> Result<Value, String> { worker(state.inner().clone(), window, |runtime, caller| { runtime.registry.context(caller.clone())?; runtime.coordinator.context(caller).map(|context| context_json(&context)) }).await }
#[tauri::command]
pub async fn get_snip_image(window: WebviewWindow, state: State<'_, Arc<SnipRuntime>>, generation: String, capture_id: String, monitor_id: String) -> Result<tauri::ipc::Response, String> {
    worker(state.inner().clone(), window, move |runtime, caller| { runtime.registry.context(caller.clone())?; runtime.coordinator.image(caller, token(generation,capture_id)?, monitor(monitor_id)?).map(tauri::ipc::Response::new) }).await
}
#[tauri::command]
pub async fn snip_bar_ready(window: WebviewWindow, state: State<'_, Arc<SnipRuntime>>, generation: String, capture_id: String) -> Result<Value, String> { worker(state.inner().clone(), window, move |runtime, caller| runtime.ack(caller, token(generation,capture_id)?)).await }
#[tauri::command]
pub async fn snip_ready(window: WebviewWindow, state: State<'_, Arc<SnipRuntime>>, generation: String, capture_id: String, monitor_id: String) -> Result<Value, String> { worker(state.inner().clone(), window, move |runtime, caller| { runtime.registry.context(caller.clone())?; let token = token(generation,capture_id)?; runtime.coordinator.ready(caller,token.clone(),monitor(monitor_id)?).map(|ready| ack_json(&token,"ready",ready)) }).await }
#[tauri::command]
pub async fn snip_begin_selection(window: WebviewWindow, state: State<'_, Arc<SnipRuntime>>, generation: String, capture_id: String, monitor_id: String) -> Result<Value, String> { worker(state.inner().clone(), window, move |runtime, caller| { runtime.registry.context(caller.clone())?; let token = token(generation,capture_id)?; runtime.coordinator.begin_selection(caller,token.clone(),monitor(monitor_id)?).map(|accepted| ack_json(&token,"accepted",accepted)) }).await }
#[derive(serde::Deserialize)]
pub struct Rect { x: f64, y: f64, width: f64, height: f64 }
#[tauri::command]
pub async fn complete_snip(window: WebviewWindow, state: State<'_, Arc<SnipRuntime>>, generation: String, capture_id: String, monitor_id: String, rect: Rect) -> Result<Value, String> { worker(state.inner().clone(), window, move |runtime, caller| { let token = token(generation,capture_id)?;
    runtime.coordinator.complete(caller,token.clone(),monitor(monitor_id)?, super::geometry::LogicalRect { x:rect.x,y:rect.y,width:rect.width,height:rect.height }).map(|_| token_json(&token)) }).await }
#[tauri::command]
pub async fn cancel_snip(window: WebviewWindow, state: State<'_, Arc<SnipRuntime>>, generation: String, capture_id: String) -> Result<Value, String> { worker(state.inner().clone(), window, move |runtime, caller| { let token = token(generation,capture_id)?; runtime.coordinator.cancel(caller,token.clone()).map(|_| token_json(&token)) }).await }
#[tauri::command]
pub async fn dismiss_snip(window: WebviewWindow, state: State<'_, Arc<SnipRuntime>>, generation: String, capture_id: String) -> Result<Value, String> { worker(state.inner().clone(), window, move |runtime, caller| { let token = token(generation,capture_id)?; runtime.coordinator.dismiss(caller,token.clone()).map(|_| token_json(&token)) }).await }
#[tauri::command]
pub async fn copy_snip(window: WebviewWindow, state: State<'_, Arc<SnipRuntime>>, generation: String, capture_id: String) -> Result<Value, String> { worker(state.inner().clone(), window, move |runtime, caller| { let token = token(generation,capture_id)?; let outcome = runtime.coordinator.copy(caller,token.clone())?; let mut value = outcome.metadata(); value["generation"] = json!(token.generation); value["captureId"] = json!(token.capture_id); Ok(value) }).await }
#[tauri::command]
pub async fn save_snip(window: WebviewWindow, state: State<'_, Arc<SnipRuntime>>, generation: String, capture_id: String) -> Result<Value, String> { worker(state.inner().clone(), window, move |runtime, caller| { let token = token(generation,capture_id)?; let outcome = runtime.coordinator.save(caller,token.clone())?; let mut value = token_json(&token); match outcome { SaveOutcome::Saved => value["status"] = json!("saved"), SaveOutcome::Cancelled => value["status"] = json!("cancelled"), SaveOutcome::Error(code) => { value["status"] = json!("error"); value["code"] = json!(code.code()); } } Ok(value) }).await }
