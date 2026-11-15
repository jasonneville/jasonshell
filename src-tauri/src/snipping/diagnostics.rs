//! Temporary opt-in Escape/state counters, never an input/event history.
use std::{path::Path, sync::{Arc, atomic::{AtomicU64, AtomicBool, Ordering}}, time::Duration, io::{Seek, Write}};
use serde::Serialize;
// Process totals deliberately live independently of callback TLS/router lifetime.
// They contain Escape boundary counts only, never an owner/native identifier.
static CALLBACK_ACTIVE: AtomicU64 = AtomicU64::new(0);
static CALLBACK_ESCAPE_DOWN: AtomicU64 = AtomicU64::new(0);
static CALLBACK_ESCAPE_UP: AtomicU64 = AtomicU64::new(0);
static CALLBACK_TLS_MISSING: AtomicU64 = AtomicU64::new(0);
static CALLBACK_OWNER_MISSING: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Copy)]
pub(crate) enum CallbackEvent { EscapeDown, EscapeUp, TlsMissing, OwnerMissing }
/// Callable before TLS lookup: fixed process atomics, no ownership acquisition.
pub(crate) fn record_callback(event: CallbackEvent) {
    if CALLBACK_ACTIVE.load(Ordering::Relaxed) == 0 { return; }
    let counter = match event {
        CallbackEvent::EscapeDown => &CALLBACK_ESCAPE_DOWN,
        CallbackEvent::EscapeUp => &CALLBACK_ESCAPE_UP,
        CallbackEvent::TlsMissing => &CALLBACK_TLS_MISSING,
        CallbackEvent::OwnerMissing => &CALLBACK_OWNER_MISSING,
    };
    counter.fetch_add(1, Ordering::Relaxed);
}
#[derive(Clone, Copy)]
pub(crate) enum Event {
    EscapeDown, EscapeUp, EscapeRepeat, ObservedGeneration(u64), HookRegistered,
    HookInstallFailed, PumpTick, HookExit, WorkerExit, RouteAdmitted, RouteNoGeneration,
    Enqueued, Drained, DeliveryAccepted, DeliveryRejected, Begin, ArmBegin, ArmEnd,
    CancelAccepted, CancelRejected, CancelDeferred, Shutdown,
    State { phase: u8, busy: bool, armed: bool, deferred: bool },
}
macro_rules! counters {
    ($($field:ident),+ $(,)?) => {
        pub(crate) struct Diagnostics { $( $field: AtomicU64, )+ state: AtomicU64, stopped: AtomicBool }
        #[derive(Clone, Copy, Serialize)]
        pub(crate) struct Snapshot { schema: u64, process_id: u32, enabled: bool, $( $field: u64, )+ phase: u64, busy: bool, armed: bool, deferred: bool,
            callback_escape_down: u64, callback_escape_up: u64, callback_tls_missing: u64, callback_owner_missing: u64 }
        impl Diagnostics {
            fn new() -> Self { CALLBACK_ACTIVE.fetch_add(1, Ordering::Relaxed); Self { $( $field: AtomicU64::new(0), )+ state: AtomicU64::new(0), stopped: AtomicBool::new(false) } }
            /// Approximate cross-counter snapshot; flags share one atomic word.
            pub(crate) fn snapshot(&self) -> Snapshot {
                let state = self.state.load(Ordering::Relaxed);
                Snapshot { schema: 1, process_id: std::process::id(), enabled: true,
                    $( $field: self.$field.load(Ordering::Relaxed), )+
                    phase: state & 255, busy: state & 256 != 0, armed: state & 512 != 0, deferred: state & 1024 != 0,
                    callback_escape_down: CALLBACK_ESCAPE_DOWN.load(Ordering::Relaxed), callback_escape_up: CALLBACK_ESCAPE_UP.load(Ordering::Relaxed),
                    callback_tls_missing: CALLBACK_TLS_MISSING.load(Ordering::Relaxed), callback_owner_missing: CALLBACK_OWNER_MISSING.load(Ordering::Relaxed) }
            }
        }
    }
}
counters!(observed_generation, escape_down, escape_up, escape_repeat, hook_registered,
    hook_install_failed, pump_ticks, hook_exit, worker_exit, shutdown, route_admitted,
    route_no_generation, enqueued, drained, delivery_accepted, delivery_rejected,
    begin, arm_begin, arm_end, cancel_accepted, cancel_rejected, cancel_deferred);
impl Diagnostics {
    /// Hot path: bounded atomics only, with no allocation, I/O, locks or wakeup.
    pub(crate) fn record(&self, event: Event) {
        let counter = match event {
            Event::ObservedGeneration(value) => { self.observed_generation.store(value, Ordering::Relaxed); return; },
            Event::State { phase, busy, armed, deferred } => {
                self.state.store(u64::from(phase) | ((busy as u64) << 8) | ((armed as u64) << 9) | ((deferred as u64) << 10), Ordering::Relaxed); return;
            },
            Event::EscapeDown => &self.escape_down, Event::EscapeUp => &self.escape_up,
            Event::EscapeRepeat => &self.escape_repeat, Event::HookRegistered => &self.hook_registered,
            Event::HookInstallFailed => &self.hook_install_failed, Event::PumpTick => &self.pump_ticks,
            Event::HookExit => &self.hook_exit, Event::WorkerExit => &self.worker_exit,
            Event::Shutdown => &self.shutdown, Event::RouteAdmitted => &self.route_admitted,
            Event::RouteNoGeneration => &self.route_no_generation, Event::Enqueued => &self.enqueued,
            Event::Drained => &self.drained, Event::DeliveryAccepted => &self.delivery_accepted,
            Event::DeliveryRejected => &self.delivery_rejected, Event::Begin => &self.begin,
            Event::ArmBegin => &self.arm_begin, Event::ArmEnd => &self.arm_end,
            Event::CancelAccepted => &self.cancel_accepted, Event::CancelRejected => &self.cancel_rejected,
            Event::CancelDeferred => &self.cancel_deferred,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }
    pub(crate) fn start(directory: &Path) -> Option<Arc<Self>> {
        if !regular_directory(directory) || !regular_file(&directory.join("snip-escape-diagnostics.enable")) { return None; }
        let output = directory.join(format!("snip-escape-diagnostics-{}.json", std::process::id()));
        let diagnostic = Arc::new(Self::new()); let weak = Arc::downgrade(&diagnostic);
        std::thread::Builder::new().name("snip-diagnostic-writer".into()).spawn(move || {
            // Never follow/overwrite an existing output or reparse target. Only
            // this writer owns this newly created per-process file handle.
            let Ok(mut file) = std::fs::OpenOptions::new().write(true).create_new(true).open(output) else { return; };
            loop {
                std::thread::sleep(Duration::from_millis(250));
                let Some(owner) = weak.upgrade() else { break; };
                let snapshot = owner.snapshot(); let stopped = owner.stopped.load(Ordering::Acquire);
                drop(owner); // Filesystem stalls cannot retain runtime/diagnostics.
                if let Ok(bytes) = serde_json::to_vec(&snapshot) {
                    if bytes.len() <= 4096 {
                        if file.rewind().and_then(|_| file.write_all(&bytes)).and_then(|_| file.set_len(bytes.len() as u64)).is_err() { break; }
                    }
                }
                if stopped { break; }
            }
        }).ok()?;
        Some(diagnostic)
    }
    pub(crate) fn shutdown(&self) {
        if !self.stopped.swap(true, Ordering::AcqRel) { self.record(Event::Shutdown); }
    }
}
impl Drop for Diagnostics { fn drop(&mut self) { self.shutdown(); CALLBACK_ACTIVE.fetch_sub(1, Ordering::Relaxed); } }
fn non_reparse(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)] { use std::os::windows::fs::MetadataExt; metadata.file_attributes() & 0x400 == 0 }
    #[cfg(not(windows))] { !metadata.file_type().is_symlink() }
}
fn regular_file(path: &Path) -> bool { std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && non_reparse(&m)) }
fn regular_directory(path: &Path) -> bool { std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir() && non_reparse(&m)) }
