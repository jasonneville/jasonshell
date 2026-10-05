//! Dedicated out-of-context WinEvent observer; no renderer supplies HWND authority.
use super::coordinator::SnipError;
use std::{cell::RefCell, sync::{Arc, Mutex, mpsc, atomic::{AtomicBool, Ordering}}, time::Duration};
use windows::Win32::{Foundation::*, System::Threading::GetCurrentProcessId, UI::{Accessibility::*, WindowsAndMessaging::*}};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intent { hwnd: isize, pid: u32, thread: u32, epoch: u64 }
#[cfg(test)]
impl Intent {
    /// Synthetic native metadata only; absent from every production IPC surface.
    pub(crate) fn test_identity(hwnd: isize, pid: u32, thread: u32, epoch: u64) -> Self { Self { hwnd, pid, thread, epoch } }
}
struct Observed { candidate: Option<Intent>, epoch: u64, healthy: bool }
impl Observed {
    fn apply(&mut self, event: u32, hwnd: isize, object: i32, external: Option<(u32, u32)>, own_window: bool) {
        if !self.healthy { return; }
        if event == EVENT_SYSTEM_FOREGROUND {
            if let Some((pid, thread)) = external {
                self.epoch = match self.epoch.checked_add(1) { Some(epoch) => epoch, None => { self.healthy = false; self.candidate = None; return; } };
                self.candidate = Some(Intent { hwnd, pid, thread, epoch: self.epoch });
            } else if !own_window { self.candidate = None; }
        } else if object == OBJID_WINDOW.0 && (event == EVENT_OBJECT_DESTROY || event == EVENT_OBJECT_CREATE)
            && self.candidate.is_some_and(|intent| intent.hwnd == hwnd) { self.candidate = None; }
    }
    fn proves(&self, intent: Intent, current_identity: Option<(u32, u32)>) -> bool {
        self.healthy && self.candidate == Some(intent) && current_identity == Some((intent.pid, intent.thread))
    }
}
thread_local! { static OBSERVER: RefCell<Option<Arc<Mutex<Observed>>>> = const { RefCell::new(None) }; }
unsafe extern "system" fn event(_: HWINEVENTHOOK, event: u32, hwnd: HWND, object: i32, _: i32, _: u32, _: u32) {
    OBSERVER.with(|slot| {
        let slot = slot.borrow(); let Some(shared) = slot.as_ref() else { return; };
        let external = if event == EVENT_SYSTEM_FOREGROUND { identity(hwnd) } else { None };
        let mut pid = 0;
        let own_window = event == EVENT_SYSTEM_FOREGROUND && !hwnd.0.is_null()
            && unsafe { IsWindow(Some(hwnd)) }.as_bool()
            && unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) } != 0 && pid == unsafe { GetCurrentProcessId() };
        let Ok(mut state) = shared.lock() else { return; };
        // Preserve external intent only across a proven live own-window transition.
        state.apply(event, hwnd.0 as isize, object, external, own_window);
    });
}
fn identity(hwnd: HWND) -> Option<(u32, u32)> {
    if hwnd.0.is_null() || !unsafe { IsWindow(Some(hwnd)) }.as_bool() { return None; }
    let mut pid = 0; let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 || thread == 0 || pid == unsafe { GetCurrentProcessId() } { None } else { Some((pid, thread)) }
}
pub struct ForegroundObserver { shared: Arc<Mutex<Observed>>, stop: Arc<AtomicBool>, queries: mpsc::Sender<mpsc::SyncSender<Option<Intent>>>, scoped_pid: Option<u32> }
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
        let shared = Arc::new(Mutex::new(Observed { candidate: None, epoch: 0, healthy: false }));
        let stop = Arc::new(AtomicBool::new(false)); let state = shared.clone(); let stopping = stop.clone();
        let (send, receive) = mpsc::sync_channel(1);
        let (queries, requests) = mpsc::channel::<mpsc::SyncSender<Option<Intent>>>();
        std::thread::spawn(move || {
            OBSERVER.with(|slot| *slot.borrow_mut() = Some(state.clone()));
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
                while let Ok(reply) = requests.try_recv() {
                    let candidate = state.lock().ok().and_then(|state| if state.healthy { state.candidate } else { None });
                    let candidate = candidate.filter(|intent| identity(HWND(intent.hwnd as *mut _)) == Some((intent.pid, intent.thread)));
                    let _ = reply.send(candidate);
                }
                // Wait only for observer work; never used as a capture paint barrier.
                unsafe { let _ = MsgWaitForMultipleObjects(None, false, 25, QS_ALLINPUT); }
            }
            if let Ok(mut observed) = state.lock() { observed.healthy = false; observed.candidate = None; }
            unsafe { let _ = UnhookWinEvent(foreground); let _ = UnhookWinEvent(lifecycle); }
            OBSERVER.with(|slot| *slot.borrow_mut() = None);
        });
        if receive.recv_timeout(Duration::from_secs(2)).ok() != Some(true) { stop.store(true, Ordering::Release); return Err(SnipError::Inactive); }
        Ok(Self { shared, stop, queries, scoped_pid })
    }
    pub fn snapshot(&self) -> Result<Intent, SnipError> {
        let intent = self.observed().ok_or(SnipError::Inactive)?;
        if self.valid(intent) { Ok(intent) } else { Err(SnipError::Inactive) }
    }
    fn observed(&self) -> Option<Intent> {
        if self.stop.load(Ordering::Acquire) { return None; }
        let (send, receive) = mpsc::sync_channel(1);
        self.queries.send(send).ok()?;
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
        if identity(HWND(intent.hwnd as *mut _)) != Some((intent.pid, intent.thread)) { return false; }
        if self.observed() != Some(intent) { return false; }
        let current_identity = identity(HWND(intent.hwnd as *mut _));
        self.shared.lock().map(|state| state.proves(intent, current_identity)).unwrap_or(false)
    }
    /// Caller must additionally prove a live snip instance still owns foreground.
    pub fn restore_validated(&self, intent: Intent, owned_foreground: isize) {
        let current = self.shared.lock().map(|state| state.healthy && state.candidate == Some(intent)).unwrap_or(false);
        if current && identity(HWND(intent.hwnd as *mut _)) == Some((intent.pid, intent.thread))
            && unsafe { GetForegroundWindow() }.0 as isize == owned_foreground {
            unsafe { let _ = SetForegroundWindow(HWND(intent.hwnd as *mut _)); }
        }
    }
    pub fn stop(&self) { self.stop.store(true, Ordering::Release); if let Ok(mut state) = self.shared.lock() { state.healthy = false; state.candidate = None; } }
}
#[cfg(test)]
pub(crate) mod verification {
    use super::*;
    pub(crate) struct Tracker(Observed);
    impl Tracker {
        pub(crate) fn new() -> Self { Self(Observed { candidate: None, epoch: 0, healthy: true }) }
        pub(crate) fn foreground(&mut self, hwnd: isize, external: Option<(u32, u32)>, own: bool) { self.0.apply(EVENT_SYSTEM_FOREGROUND, hwnd, OBJID_WINDOW.0, external, own); }
        pub(crate) fn destroyed(&mut self, hwnd: isize) { self.0.apply(EVENT_OBJECT_DESTROY, hwnd, OBJID_WINDOW.0, None, false); }
        pub(crate) fn created(&mut self, hwnd: isize) { self.0.apply(EVENT_OBJECT_CREATE, hwnd, OBJID_WINDOW.0, None, false); }
        pub(crate) fn candidate(&self) -> Option<Intent> { self.0.candidate }
        pub(crate) fn valid(&self, intent: Intent, current_identity: Option<(u32, u32)>) -> bool { self.0.proves(intent, current_identity) }
        pub(crate) fn lose_continuity(&mut self) { self.0.healthy = false; self.0.candidate = None; }
    }
}
impl Drop for ForegroundObserver { fn drop(&mut self) { self.stop(); } }
