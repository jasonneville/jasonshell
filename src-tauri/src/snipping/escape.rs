//! Passive native Escape observation. No key is consumed or renderer-authorized.
use super::{coordinator::SnipError, runtime::SnipRuntime};
use super::diagnostics::{Diagnostics, Event as DiagnosticEvent};
use super::diagnostics::{record_callback, CallbackEvent};
use std::{cell::RefCell, sync::{Arc, Weak, Mutex, mpsc, atomic::{AtomicBool, AtomicU64, Ordering}}, time::Duration};
use windows::Win32::{Foundation::{LPARAM, WPARAM, LRESULT}, UI::WindowsAndMessaging::*};

pub(crate) struct EventRouter {
    runtime: Weak<SnipRuntime>, stopped: AtomicBool, pressed: AtomicBool,
    mailbox: AtomicU64,
    diagnostics: Option<Arc<Diagnostics>>,
    cleanup: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}
pub(crate) struct PendingEscape { owner: Weak<EventRouter>, generation: u64 }
impl EventRouter {
    pub(crate) fn new(runtime: Weak<SnipRuntime>, cleanup: impl FnOnce() + Send + 'static) -> Arc<Self> {
        let diagnostics = runtime.upgrade().and_then(|runtime| runtime.coordinator.diagnostics());
        Arc::new(Self { runtime, diagnostics, stopped: AtomicBool::new(false), pressed: AtomicBool::new(false), mailbox: AtomicU64::new(0), cleanup: Mutex::new(Some(Box::new(cleanup))) })
    }
    fn diagnostic(&self, event: DiagnosticEvent) { if let Some(diagnostics) = &self.diagnostics { diagnostics.record(event); } }
    pub(crate) fn route(self: &Arc<Self>, message: u32, key: u32) -> Option<PendingEscape> {
        if key != 0x1b || self.stopped.load(Ordering::Acquire) { return None; }
        if matches!(message, WM_KEYUP | WM_SYSKEYUP) { self.diagnostic(DiagnosticEvent::EscapeUp); self.pressed.store(false, Ordering::Release); return None; }
        if !matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) { return None; }
        self.diagnostic(DiagnosticEvent::EscapeDown);
        if self.pressed.swap(true, Ordering::AcqRel) { self.diagnostic(DiagnosticEvent::EscapeRepeat); return None; }
        let runtime = self.runtime.upgrade()?;
        let generation = runtime.coordinator.observed_selection_generation();
        self.diagnostic(DiagnosticEvent::ObservedGeneration(generation));
        if generation == 0 { self.diagnostic(DiagnosticEvent::RouteNoGeneration); return None; }
        self.diagnostic(DiagnosticEvent::RouteAdmitted);
        Some(PendingEscape { owner: Arc::downgrade(self), generation })
    }
    /// Work lives in the atomic mailbox, not in the wakeup channel. A full
    /// channel already guarantees a wakeup; newer generations replace stale work.
    pub(crate) fn enqueue(self: &Arc<Self>, pending: PendingEscape, wake: &mpsc::SyncSender<()>) {
        if !Weak::ptr_eq(&pending.owner, &Arc::downgrade(self)) || self.stopped.load(Ordering::Acquire) { return; }
        self.mailbox.fetch_max(pending.generation, Ordering::AcqRel);
        self.diagnostic(DiagnosticEvent::Enqueued);
        let _ = wake.try_send(());
    }
    /// Drain exactly the production mailbox after consuming a wakeup. Delivery
    /// still validates shutdown and the captured generation, never a fresh target.
    pub(crate) fn drain_pending(self: &Arc<Self>) -> bool {
        let generation = self.mailbox.swap(0, Ordering::AcqRel);
        if generation != 0 { self.diagnostic(DiagnosticEvent::Drained); }
        generation != 0 && PendingEscape { owner: Arc::downgrade(self), generation }.deliver()
    }
    pub(crate) fn shutdown(&self) {
        if self.stopped.swap(true, Ordering::AcqRel) { return; }
        if let Ok(mut cleanup) = self.cleanup.lock() { if let Some(cleanup) = cleanup.take() { cleanup(); } }
        if let Some(diagnostics) = &self.diagnostics { diagnostics.shutdown(); }
    }
}
impl Drop for EventRouter { fn drop(&mut self) { self.shutdown(); } }
impl PendingEscape {
    pub(crate) fn deliver(self) -> bool {
        let Some(owner) = self.owner.upgrade() else { return false; };
        if owner.stopped.load(Ordering::Acquire) { owner.diagnostic(DiagnosticEvent::DeliveryRejected); return false; }
        let accepted = owner.runtime.upgrade().is_some_and(|runtime| {
            runtime.coordinator.selection_token(self.generation)
                .is_some_and(|token| !owner.stopped.load(Ordering::Acquire) && runtime.native_escape(&token, true))
        });
        owner.diagnostic(if accepted { DiagnosticEvent::DeliveryAccepted } else { DiagnosticEvent::DeliveryRejected });
        accepted
    }
}
thread_local! { static LISTENER: RefCell<Option<(Weak<EventRouter>, mpsc::SyncSender<()>)>> = const { RefCell::new(None) }; }
unsafe extern "system" fn keyboard(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
    if code == 0 && data.0 != 0 {
        // SAFETY: Windows supplies KBDLLHOOKSTRUCT for HC_ACTION for this callback.
        let key = unsafe { (*(data.0 as *const KBDLLHOOKSTRUCT)).vkCode };
        if key == 0x1b {
            match message.0 as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => record_callback(CallbackEvent::EscapeDown),
                WM_KEYUP | WM_SYSKEYUP => record_callback(CallbackEvent::EscapeUp),
                _ => {},
            }
            LISTENER.with(|slot| {
            if let Some((owner, send)) = slot.borrow().as_ref() {
                if let Some(owner) = owner.upgrade() {
                    if let Some(pending) = owner.route(message.0 as u32, key) {
                        owner.enqueue(pending, send);
                    }
                } else { record_callback(CallbackEvent::OwnerMissing); }
            } else { record_callback(CallbackEvent::TlsMissing); }
        }); }
    }
    // Always forward, including Escape and negative hook codes.
    unsafe { CallNextHookEx(None, code, message, data) }
}
pub(crate) fn install(runtime: Weak<SnipRuntime>) -> Result<Arc<EventRouter>, SnipError> {
    // Consent marker is read once at listener startup, never on an input path.
    if let Some(directory) = std::env::var_os("LOCALAPPDATA").map(|root| std::path::PathBuf::from(root).join("Temp/opencode")) {
        if let Some(diagnostics) = Diagnostics::start(&directory) {
            if let Some(runtime) = runtime.upgrade() { runtime.coordinator.set_diagnostics(diagnostics); }
        }
    }
    let stop = Arc::new(AtomicBool::new(false)); let stopping = stop.clone();
    let router = EventRouter::new(runtime, move || { stopping.store(true, Ordering::Release); });
    let weak = Arc::downgrade(&router);
    let delivery_owner = weak.clone();
    let (send, receive) = mpsc::sync_channel::<()>(1);
    let (ready, registered) = mpsc::sync_channel(1);
    let hook_stop = stop.clone();
    let hook_diagnostics = router.diagnostics.clone();
    std::thread::Builder::new().name("snip-escape-hook".into()).spawn(move || {
        LISTENER.with(|slot| *slot.borrow_mut() = Some((weak, send)));
        let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard), None, 0) };
        let Ok(hook) = hook else { if let Some(d) = &hook_diagnostics { d.record(DiagnosticEvent::HookInstallFailed); } let _ = ready.send(false); LISTENER.with(|slot| *slot.borrow_mut() = None); return; };
        if let Some(d) = &hook_diagnostics { d.record(DiagnosticEvent::HookRegistered); }
        let _ = ready.send(true);
        while !hook_stop.load(Ordering::Acquire) {
            if let Some(d) = &hook_diagnostics { d.record(DiagnosticEvent::PumpTick); }
            let mut message = MSG::default();
            while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
                unsafe { let _ = TranslateMessage(&message); DispatchMessageW(&message); }
            }
            unsafe { let _ = MsgWaitForMultipleObjects(None, false, 25, QS_ALLINPUT); }
        }
        unsafe { let _ = UnhookWindowsHookEx(hook); }
        if let Some(d) = &hook_diagnostics { d.record(DiagnosticEvent::HookExit); }
        LISTENER.with(|slot| *slot.borrow_mut() = None);
    }).map_err(|_| SnipError::Inactive)?;
    if registered.recv_timeout(Duration::from_secs(2)).ok() != Some(true) { router.shutdown(); return Err(SnipError::Inactive); }
    let worker_diagnostics = router.diagnostics.clone();
    if std::thread::Builder::new().name("snip-escape-delivery".into()).spawn(move || {
        while !stop.load(Ordering::Acquire) {
            match receive.recv_timeout(Duration::from_millis(25)) {
                Ok(()) => {
                    let Some(owner) = delivery_owner.upgrade() else { break; };
                    owner.drain_pending();
                },
                Err(mpsc::RecvTimeoutError::Timeout) => {},
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        if let Some(d) = &worker_diagnostics { d.record(DiagnosticEvent::WorkerExit); }
    }).is_err() { router.shutdown(); return Err(SnipError::Inactive); }
    Ok(router)
}
