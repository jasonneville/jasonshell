//! Dedicated out-of-context WinEvent observer; no renderer supplies HWND authority.
use super::coordinator::SnipError;
use std::{cell::RefCell, sync::{Arc, Mutex, mpsc, atomic::{AtomicBool, Ordering}}, time::Duration};
use windows::Win32::{Foundation::*, System::Threading::GetCurrentProcessId, UI::{Accessibility::*, WindowsAndMessaging::*}};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intent { hwnd: isize, pid: u32, thread: u32, epoch: u64, surface: Option<u64> }
pub type SurfaceResolver = Arc<dyn Fn(isize) -> Option<(u64, bool)> + Send + Sync>;
#[cfg(test)]
impl Intent {
    /// Synthetic native metadata only; absent from every production IPC surface.
    pub(crate) fn test_identity(hwnd: isize, pid: u32, thread: u32, epoch: u64) -> Self { Self { hwnd, pid, thread, epoch, surface: None } }
}
struct Observed { candidate: Option<Intent>, epoch: u64, healthy: bool, bar_hwnd: isize, native_reservation: bool, bar_previous: Option<Intent> }
impl Observed {
    fn apply(&mut self, event: u32, hwnd: isize, object: i32, external: Option<(u32, u32)>, own_window: bool) {
        if !self.healthy { return; }
        if object == OBJID_WINDOW.0 && (event == EVENT_OBJECT_DESTROY || event == EVENT_OBJECT_CREATE)
            && self.bar_previous.is_some_and(|intent| intent.hwnd == hwnd) { self.bar_previous = None; }
        if event == EVENT_SYSTEM_FOREGROUND {
            if let Some((pid, thread)) = external {
                if self.candidate.is_some_and(|intent| intent.hwnd == hwnd && intent.pid == pid && intent.thread == thread && intent.surface.is_none()) { return; }
                self.epoch = match self.epoch.checked_add(1) { Some(epoch) => epoch, None => { self.healthy = false; self.candidate = None; return; } };
                self.candidate = Some(Intent { hwnd, pid, thread, epoch: self.epoch, surface: None });
            } else if !own_window { self.candidate = None; }
        } else if object == OBJID_WINDOW.0 && (event == EVENT_OBJECT_DESTROY || event == EVENT_OBJECT_CREATE)
            && self.candidate.is_some_and(|intent| intent.hwnd == hwnd) { self.candidate = None; }
    }
    fn registered(&mut self, hwnd: isize, pid: u32, thread: u32, instance: u64) {
        if self.candidate.is_some_and(|intent| intent.hwnd == hwnd && intent.pid == pid && intent.thread == thread && intent.surface == Some(instance)) { return; }
        self.apply(EVENT_SYSTEM_FOREGROUND, hwnd, OBJID_WINDOW.0, Some((pid, thread)), false);
        if let Some(intent) = self.candidate.as_mut() { intent.surface = Some(instance); }
    }
    fn surface_foreground(&mut self, hwnd: isize, pid: u32, thread: u32, instance: u64) {
        if hwnd == self.bar_hwnd {
            if self.candidate.is_some_and(|intent| intent.surface.is_none()) {
                self.bar_previous = self.candidate;
                if !self.native_reservation { return; }
            }
        } else { self.bar_previous = None; }
        self.registered(hwnd, pid, thread, instance);
    }
    fn admit(&mut self, native: bool, foreground: isize) {
        self.native_reservation = native;
        if native { self.bar_previous = None; }
        else if foreground == self.bar_hwnd {
            if let Some(previous) = self.bar_previous.take() {
                // New renderer admission, never resurrection of the native
                // reservation invalidated by the user's top-bar transition.
                self.apply(EVENT_SYSTEM_FOREGROUND, previous.hwnd, OBJID_WINDOW.0, Some((previous.pid, previous.thread)), false);
            }
        }
    }
    fn proves(&self, intent: Intent, current_identity: Option<(u32, u32)>) -> bool {
        self.healthy && self.candidate == Some(intent) && current_identity == Some((intent.pid, intent.thread))
    }
}
thread_local! { static OBSERVER: RefCell<Option<(Arc<Mutex<Observed>>, Arc<Mutex<Option<SurfaceResolver>>>)>> = const { RefCell::new(None) }; }
unsafe extern "system" fn event(_: HWINEVENTHOOK, event: u32, hwnd: HWND, object: i32, _: i32, _: u32, _: u32) {
    OBSERVER.with(|slot| {
        let slot = slot.borrow(); let Some((shared, resolver)) = slot.as_ref() else { return; };
        let external = if event == EVENT_SYSTEM_FOREGROUND { identity(hwnd) } else { None };
        let mut pid = 0;
        let own_window = event == EVENT_SYSTEM_FOREGROUND && !hwnd.0.is_null()
            && unsafe { IsWindow(Some(hwnd)) }.as_bool()
            && unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) } != 0 && pid == unsafe { GetCurrentProcessId() };
        let resolve = resolver.lock().ok().and_then(|resolver| resolver.clone());
        let surface = if own_window { resolve.and_then(|resolve| resolve(hwnd.0 as isize)) } else { None };
        let Ok(mut state) = shared.lock() else { return; };
        if let Some((instance, false)) = surface {
            let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
            // A bar click retains its pre-click target. A native hotkey query
            // below explicitly binds the bar itself when it owns foreground.
            state.surface_foreground(hwnd.0 as isize, pid, thread, instance);
        } else {
            // Unknown own-PID windows are not authority. Only exact registered
            // transient helpers preserve a previous origin across activation.
            if event == EVENT_SYSTEM_FOREGROUND && (external.is_some() || surface.is_none()) { state.bar_previous = None; }
            state.apply(event, hwnd.0 as isize, object, external, matches!(surface, Some((_, true))));
        }
    });
}
fn identity(hwnd: HWND) -> Option<(u32, u32)> {
    if hwnd.0.is_null() || !unsafe { IsWindow(Some(hwnd)) }.as_bool() { return None; }
    let mut pid = 0; let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 || thread == 0 || pid == unsafe { GetCurrentProcessId() } { None } else { Some((pid, thread)) }
}
pub struct ForegroundObserver { shared: Arc<Mutex<Observed>>, stop: Arc<AtomicBool>, queries: mpsc::Sender<(u8, mpsc::SyncSender<Option<Intent>>)>, scoped_pid: Option<u32>, resolver: Arc<Mutex<Option<SurfaceResolver>>>, workers: super::shutdown_workers::WorkerGroup }
impl ForegroundObserver {
    pub fn start() -> Result<Self, SnipError> {
        Self::start_filtered(None)
    }
    /// Standalone synthetic harness may observe only its owned external child PID.
    /// This does not install global hotkeys or keyboard hooks.
    pub(crate) fn start_scoped(pid: u32) -> Result<Self, SnipError> {
        if pid == 0 || pid == unsafe { GetCurrentProcessId() } { return Err(SnipError::Inactive); }
        Self::start_filtered(Some(pid))
    }
    fn start_filtered(scoped_pid: Option<u32>) -> Result<Self, SnipError> {
        let resolver = Arc::new(Mutex::new(None::<SurfaceResolver>)); let resolving = resolver.clone();
        let shared = Arc::new(Mutex::new(Observed { candidate: None, epoch: 0, healthy: false, bar_hwnd: 0, native_reservation: false, bar_previous: None }));
        let stop = Arc::new(AtomicBool::new(false)); let state = shared.clone(); let stopping = stop.clone();
        let (send, receive) = mpsc::sync_channel(1);
        let (queries, requests) = mpsc::channel::<(u8, mpsc::SyncSender<Option<Intent>>) >();
        let workers = super::shutdown_workers::WorkerGroup::default();
        workers.spawn(move || {
            OBSERVER.with(|slot| *slot.borrow_mut() = Some((state.clone(), resolving.clone())));
            let foreground = unsafe { SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(event), scoped_pid.unwrap_or(0), 0, WINEVENT_OUTOFCONTEXT) };
            let lifecycle = unsafe { SetWinEventHook(EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY, None, Some(event), scoped_pid.unwrap_or(0), 0, WINEVENT_OUTOFCONTEXT) };
            if foreground.0.is_null() || lifecycle.0.is_null() {
                unsafe { if !foreground.0.is_null() { let _ = UnhookWinEvent(foreground); } if !lifecycle.0.is_null() { let _ = UnhookWinEvent(lifecycle); } }
                let _ = send.send(false); return;
            }
            if let Ok(mut observed) = state.lock() { observed.healthy = true; }
            unsafe { event(foreground, EVENT_SYSTEM_FOREGROUND, GetForegroundWindow(), 0, 0, 0, 0); }
            let _ = send.send(true);
            while !stopping.load(Ordering::Acquire) {
                let mut message = MSG::default();
                while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() { unsafe { let _ = TranslateMessage(&message); DispatchMessageW(&message); } }
                // Answer only on the hook-owning thread after draining native events.
                // A stopped/unresponsive observer cannot authorize cached identity.
                while let Ok((admission, reply)) = requests.try_recv() {
                    if admission != 0 { if let Ok(mut state) = state.lock() { state.admit(admission == 1, unsafe { GetForegroundWindow() }.0 as isize); } }
                    unsafe { event(foreground, EVENT_SYSTEM_FOREGROUND, GetForegroundWindow(), 0, 0, 0, 0); }
                    if admission == 1 {
                        let hwnd = unsafe { GetForegroundWindow() };
                        let resolve = resolving.lock().ok().and_then(|resolver| resolver.clone());
                        if let Some((instance, false)) = resolve.and_then(|resolve| resolve(hwnd.0 as isize)) {
                            let mut pid = 0; let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
                            if let Ok(mut state) = state.lock() { state.registered(hwnd.0 as isize, pid, thread, instance); }
                        }
                    }
                    let candidate = state.lock().ok().and_then(|state| if state.healthy { state.candidate } else { None });
                    let resolve = resolving.lock().ok().and_then(|resolver| resolver.clone());
                    let candidate = candidate.filter(|intent| live_identity(*intent, resolve.as_ref()));
                    let _ = reply.send(candidate);
                }
                // Wait only for observer work; never used as a capture paint barrier.
                unsafe { let _ = MsgWaitForMultipleObjects(None, false, 25, QS_ALLINPUT); }
            }
            if let Ok(mut observed) = state.lock() { observed.healthy = false; observed.candidate = None; }
            for hook in [foreground, lifecycle] {
                while !unsafe { UnhookWinEvent(hook) }.as_bool() { std::thread::sleep(Duration::from_millis(25)); }
            }
            OBSERVER.with(|slot| *slot.borrow_mut() = None);
        }).map_err(|_| SnipError::Inactive)?;
        if receive.recv_timeout(Duration::from_secs(2)).ok() != Some(true) { stop.store(true, Ordering::Release); return Err(SnipError::Inactive); }
        Ok(Self { shared, stop, queries, scoped_pid, resolver, workers })
    }
    pub fn set_surface_resolver(&self, resolver: SurfaceResolver, bar_hwnd: isize) {
        if let Ok(mut state) = self.shared.lock() { state.bar_hwnd = bar_hwnd; }
        if let Ok(mut current) = self.resolver.lock() { *current = Some(resolver); }
    }
    pub fn snapshot_native(&self) -> Result<Intent, SnipError> {
        // This method is entered by WM_HOTKEY's owner. Bind its actual native
        // identity here, before waiting for the observer's drained continuity
        // proof; a later foreground must never silently become this attempt.
        let hwnd = unsafe { GetForegroundWindow() };
        let mut pid = 0; let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        let resolver = self.resolver.lock().ok().and_then(|resolver| resolver.clone());
        let surface = resolver.as_ref().and_then(|resolve| resolve(hwnd.0 as isize));
        if pid == 0 || thread == 0 || (pid == unsafe { GetCurrentProcessId() } && !matches!(surface, Some((_, false)))) { return Err(SnipError::Inactive); }
        let intent = self.observed_query(1).ok_or(SnipError::Inactive)?;
        if (intent.hwnd, intent.pid, intent.thread, intent.surface) != (hwnd.0 as isize, pid, thread, surface.map(|(instance, _)| instance)) { return Err(SnipError::Inactive); }
        if self.valid(intent) { Ok(intent) } else { Err(SnipError::Inactive) }
    }
    pub fn snapshot(&self) -> Result<Intent, SnipError> {
        let intent = self.observed_query(2).ok_or(SnipError::Inactive)?;
        if self.valid(intent) { Ok(intent) } else { Err(SnipError::Inactive) }
    }
    fn observed(&self) -> Option<Intent> {
        self.observed_query(0)
    }
    fn observed_query(&self, admission: u8) -> Option<Intent> {
        if self.stop.load(Ordering::Acquire) { return None; }
        let (send, receive) = mpsc::sync_channel(1);
        self.queries.send((admission, send)).ok()?;
        match receive.recv_timeout(Duration::from_millis(250)) {
            Ok(candidate) => candidate,
            Err(_) => { if let Ok(mut state) = self.shared.lock() { state.healthy = false; state.candidate = None; } None }
        }
    }
    pub fn valid(&self, intent: Intent) -> bool {
        if let Some(pid) = self.scoped_pid {
            if intent.pid != pid { return false; }
            let mut foreground_pid = 0;
            unsafe { GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut foreground_pid)); }
            if foreground_pid != pid && foreground_pid != unsafe { GetCurrentProcessId() } { return false; }
        }
        let resolver = self.resolver.lock().ok().and_then(|resolver| resolver.clone());
        if !live_identity(intent, resolver.as_ref()) { return false; }
        if self.observed() != Some(intent) { return false; }
        let current_identity = Some((intent.pid, intent.thread));
        self.shared.lock().map(|state| state.proves(intent, current_identity)).unwrap_or(false)
    }
    /// Caller must additionally prove a live snip instance still owns foreground.
    pub fn restore_validated(&self, intent: Intent, owned_foreground: isize) {
        let current = self.shared.lock().map(|state| state.healthy && state.candidate == Some(intent)).unwrap_or(false);
        let resolver = self.resolver.lock().ok().and_then(|resolver| resolver.clone());
        if current && live_identity(intent, resolver.as_ref())
            && unsafe { GetForegroundWindow() }.0 as isize == owned_foreground {
            unsafe { let _ = SetForegroundWindow(HWND(intent.hwnd as *mut _)); }
        }
    }
    pub fn stop(&self) { self.stop.store(true, Ordering::Release); if let Ok(mut state) = self.shared.lock() { state.healthy = false; state.candidate = None; } }
    pub fn stop_and_drain(&self) -> Result<(), String> { self.stop(); self.workers.drain() }
}
fn live_identity(intent: Intent, resolver: Option<&SurfaceResolver>) -> bool {
    let hwnd = HWND(intent.hwnd as *mut _);
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() || !unsafe { IsWindow(Some(hwnd)) }.as_bool() { return false; }
    let mut pid = 0; let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if (pid, thread) != (intent.pid, intent.thread) { return false; }
    match intent.surface {
        Some(instance) => resolver.and_then(|resolve| resolve(intent.hwnd)) == Some((instance, false)),
        None => pid != unsafe { GetCurrentProcessId() },
    }
}
#[cfg(test)]
pub(crate) mod verification {
    use super::*;
    pub(crate) struct Tracker(Observed);
    impl Tracker {
        pub(crate) fn new() -> Self { Self(Observed { candidate: None, epoch: 0, healthy: true, bar_hwnd: 0, native_reservation: false, bar_previous: None }) }
        pub(crate) fn foreground(&mut self, hwnd: isize, external: Option<(u32, u32)>, own: bool) { self.0.apply(EVENT_SYSTEM_FOREGROUND, hwnd, OBJID_WINDOW.0, external, own); }
        pub(crate) fn registered_foreground(&mut self, hwnd: isize, pid: u32, thread: u32, instance: u64) { self.0.registered(hwnd, pid, thread, instance); }
        pub(crate) fn set_bar(&mut self, hwnd: isize) { self.0.bar_hwnd = hwnd; }
        pub(crate) fn admit(&mut self, native: bool, foreground: isize) { self.0.admit(native, foreground); }
        pub(crate) fn surface_foreground(&mut self, hwnd: isize, pid: u32, thread: u32, instance: u64) { self.0.surface_foreground(hwnd, pid, thread, instance); }
        pub(crate) fn destroyed(&mut self, hwnd: isize) { self.0.apply(EVENT_OBJECT_DESTROY, hwnd, OBJID_WINDOW.0, None, false); }
        pub(crate) fn created(&mut self, hwnd: isize) { self.0.apply(EVENT_OBJECT_CREATE, hwnd, OBJID_WINDOW.0, None, false); }
        pub(crate) fn candidate(&self) -> Option<Intent> { self.0.candidate }
        pub(crate) fn valid(&self, intent: Intent, current_identity: Option<(u32, u32)>) -> bool { self.0.proves(intent, current_identity) }
        pub(crate) fn lose_continuity(&mut self) { self.0.healthy = false; self.0.candidate = None; }
    }
}
impl Drop for ForegroundObserver { fn drop(&mut self) { self.stop(); } }
