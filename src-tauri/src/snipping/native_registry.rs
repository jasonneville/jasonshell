//! Concrete Tauri instance identities and bounded initialization gates.
use super::coordinator::{Caller, Context, SnipError, Token, WindowRef};
use std::{collections::HashMap, sync::{Arc, Condvar, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}}, time::{Duration, Instant}};
use tauri::{Manager, WebviewWindow};
use windows::{core::w, Win32::{Foundation::{HANDLE, HWND}, UI::WindowsAndMessaging::{GetPropW, SetPropW}}};
/// Native adapter output, never deserialized or accepted from IPC. Each read
/// must resolve the current registration and HWND epoch afresh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FocusRegistration {
    pub hwnd: isize, pub pid: u32, pub thread: u32, pub instance: u64, pub token: Option<Token>,
}

pub struct Entry {
    pub reference: WindowRef,
    pub token: Option<Token>,
    pub preview: bool,
    deadline: Instant,
    state: Mutex<State>,
    changed: Condvar,
    retired_generation: Arc<AtomicU64>,
}
struct State { window: Option<WebviewWindow>, hwnd: isize, dead: bool, context: Option<Context> }
#[derive(Default)]
pub struct NativeRegistry { entries: Mutex<HashMap<String, Arc<Entry>>>, next: Mutex<u64>, stopped: AtomicBool, origin_generation: Mutex<Option<Token>>, retired_generation: Arc<AtomicU64> }
pub fn same_epoch(window: &WebviewWindow, id: u64) -> bool {
    window.hwnd().ok().is_some_and(|hwnd| unsafe { GetPropW(HWND(hwnd.0), w!("JasonShell.Snip.Instance")) }.0 as usize as u64 == id)
}
impl NativeRegistry {
    pub fn retire_pending_generation(&self, token: &Token) -> Vec<(WebviewWindow, u64)> {
        let Some(generation) = token.generation.parse::<u64>().ok().filter(|generation| *generation > 0) else { return Vec::new(); };
        self.retired_generation.fetch_max(generation, Ordering::AcqRel);
        let entries = self.entries.lock().map(|entries| entries.values().filter(|entry| entry.token.as_ref() == Some(token)).cloned().collect::<Vec<_>>()).unwrap_or_default();
        entries.into_iter().filter_map(|entry| {
            let mut state = entry.state.lock().ok()?;
            if state.context.is_some() { return None; }
            state.dead = true; let window = state.window.take(); entry.changed.notify_all();
            drop(state);
            window.filter(|window| same_epoch(window, entry.reference.caller.window_id)).map(|window| (window, entry.reference.caller.window_id))
        }).collect()
    }
    pub(crate) fn focus_registration(&self, hwnd: isize) -> Option<FocusRegistration> {
        use windows::Win32::{System::Threading::GetCurrentProcessId, UI::WindowsAndMessaging::{GetWindowThreadProcessId, IsWindowVisible}};
        let entries = self.entries.lock().ok()?.values().cloned().collect::<Vec<_>>();
        entries.into_iter().find_map(|entry| {
            let window = entry.window().ok()?;
            if window.hwnd().ok()?.0 as isize != hwnd || !unsafe { IsWindowVisible(HWND(hwnd as *mut _)) }.as_bool() { return None; }
            let mut pid = 0; let thread = unsafe { GetWindowThreadProcessId(HWND(hwnd as *mut _), Some(&mut pid)) };
            if pid != unsafe { GetCurrentProcessId() } || thread == 0 { return None; }
            // Recheck after native metadata acquisition, outside every registry lock.
            entry.window().ok()?;
            Some(FocusRegistration { hwnd, pid, thread, instance: entry.reference.caller.window_id, token: entry.token.clone() })
        })
    }
    pub fn bind_origin_generation(&self, token: Token) -> Result<(), SnipError> {
        *self.origin_generation.lock().map_err(|_| SnipError::Inactive)? = Some(token);
        Ok(())
    }
    pub fn helper_token(&self, hwnd: isize) -> Option<Token> {
        let entries = self.entries.lock().ok()?.values().filter(|entry| entry.token.is_some()).cloned().collect::<Vec<_>>();
        entries.into_iter().find_map(|entry| {
            let window = entry.window().ok()?;
            (window.hwnd().ok()?.0 as isize == hwnd).then(|| entry.token.clone()).flatten()
        })
    }
    /// Resolve only a live concrete registration, never a label or process alone.
    pub fn foreground_identity(&self, hwnd: isize) -> Option<(u64, bool)> {
        let entries = self.entries.lock().ok()?.values().cloned().collect::<Vec<_>>();
        let generation = self.origin_generation.lock().ok()?.clone();
        entries.into_iter().find_map(|entry| {
            if entry.token.is_some() && entry.token != generation { return None; }
            let window = entry.window().ok()?;
            (window.hwnd().ok()?.0 as isize == hwnd).then_some((entry.reference.caller.window_id, entry.token.is_some()))
        })
    }
    pub fn owns_foreground(&self, id: u64, hwnd: isize) -> bool {
        let entry = self.entries.lock().ok().and_then(|entries| entries.values().find(|entry| entry.reference.caller.window_id == id && entry.token.is_some()).cloned());
        entry.and_then(|entry| entry.window().ok()).and_then(|window| window.hwnd().ok()).is_some_and(|handle| handle.0 as isize == hwnd)
    }
    pub fn close_snips(&self) {
        self.stopped.store(true, Ordering::Release);
        let entries = self.entries.lock().map(|entries| entries.values().filter(|entry| entry.token.is_some()).cloned().collect::<Vec<_>>()).unwrap_or_default();
        for entry in entries { if let Some(window) = entry.retire() { let app = window.app_handle().clone(); let id = entry.reference.caller.window_id; let _ = app.run_on_main_thread(move || { if same_epoch(&window, id) { let _ = window.destroy(); } }); } }
    }
    pub fn reserve(&self, label: String, token: Option<Token>, monitor_id: Option<String>, preview: bool) -> Result<Arc<Entry>, SnipError> {
        let mut entries = self.entries.lock().map_err(|_| SnipError::WindowFailed)?;
        if self.stopped.load(Ordering::Acquire) { return Err(SnipError::Inactive); }
        if token.as_ref().is_some_and(|token| token.generation.parse::<u64>().ok().is_none_or(|generation| generation <= self.retired_generation.load(Ordering::Acquire))) { return Err(SnipError::Stale); }
        entries.retain(|_, entry| entry.state.lock().map(|state| !state.dead).unwrap_or(false));
        if entries.len() >= 66 { return Err(SnipError::WindowFailed); }
        if entries.contains_key(&label) { return Err(SnipError::WindowFailed); }
        let mut next = self.next.lock().map_err(|_| SnipError::WindowFailed)?;
        *next = next.checked_add(1).ok_or(SnipError::WindowFailed)?;
        if usize::try_from(*next).is_err() { return Err(SnipError::WindowFailed); }
        let entry = Arc::new(Entry { reference: WindowRef { caller: Caller { label: label.clone(), window_id: *next }, monitor_id }, token, preview,
            deadline: Instant::now() + Duration::from_secs(5), state: Mutex::new(State { window: None, hwnd: 0, dead: false, context: None }), changed: Condvar::new(), retired_generation: self.retired_generation.clone() });
        entries.insert(label, entry.clone());
        Ok(entry)
    }
    pub fn entry(&self, reference: &WindowRef) -> Result<Arc<Entry>, SnipError> {
        let entries = self.entries.lock().map_err(|_| SnipError::WindowFailed)?;
        let entry = entries.get(&reference.caller.label).ok_or(SnipError::Unauthorized)?;
        if entry.reference.caller != reference.caller || reference.monitor_id.as_ref().is_some_and(|id| entry.reference.monitor_id.as_ref() != Some(id)) { return Err(SnipError::Unauthorized); }
        Ok(entry.clone())
    }
    /// Only injected native windows may be resolved; no renderer IDs are accepted.
    pub fn resolve(&self, window: &WebviewWindow) -> Result<Caller, SnipError> {
        let entry = self.entries.lock().map_err(|_| SnipError::WindowFailed)?.get(window.label()).cloned().ok_or(SnipError::Unauthorized)?;
        let hwnd = window.hwnd().map_err(|_| SnipError::Unauthorized)?.0 as isize;
        let mut state = entry.state.lock().map_err(|_| SnipError::WindowFailed)?;
        while state.window.is_none() && !state.dead {
            let remaining = entry.deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() { return Err(SnipError::ReadinessTimeout); }
            state = entry.changed.wait_timeout(state, remaining).map_err(|_| SnipError::WindowFailed)?.0;
        }
        if state.dead || state.hwnd != hwnd { return Err(SnipError::Unauthorized); }
        drop(state);
        entry.window()?;
        Ok(entry.reference.caller.clone())
    }
    pub fn context(&self, caller: Caller) -> Result<Context, SnipError> {
        let entry = self.entry(&WindowRef { caller, monitor_id: None })?;
        let mut state = entry.state.lock().map_err(|_| SnipError::WindowFailed)?;
        loop {
            if state.dead { return Err(SnipError::Inactive); }
            if let Some(context) = &state.context { return Ok(context.clone()); }
            let remaining = entry.deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() { return Err(SnipError::ReadinessTimeout); }
            state = entry.changed.wait_timeout(state, remaining).map_err(|_| SnipError::WindowFailed)?.0;
        }
    }
}
impl Entry {
    pub fn admit_construction(&self) -> Result<(), SnipError> { self.admit_construction_at(Instant::now()) }
    pub(crate) fn admit_construction_at(&self, now: Instant) -> Result<(), SnipError> {
        if self.token.as_ref().is_some_and(|token| token.generation.parse::<u64>().ok().is_none_or(|generation| generation <= self.retired_generation.load(Ordering::Acquire))) { return Err(SnipError::Stale); }
        let state = self.state.lock().map_err(|_| SnipError::WindowFailed)?;
        if state.dead { return Err(SnipError::Stale); }
        if self.token.is_some() && now >= self.deadline { return Err(SnipError::ReadinessTimeout); }
        Ok(())
    }
    pub fn bind(self: &Arc<Self>, window: WebviewWindow) -> Result<(), SnipError> {
        self.admit_construction()?;
        { let state = self.state.lock().map_err(|_| SnipError::WindowFailed)?;
          if state.dead || state.window.is_some() || (self.token.is_some() && Instant::now() >= self.deadline) { return Err(SnipError::ReadinessTimeout); } }
        let hwnd = window.hwnd().map_err(|_| SnipError::WindowFailed)?.0 as isize;
        // Window properties are destroyed with the HWND. A recycled HWND cannot
        // inherit this registry epoch, even before Tauri delivers Destroyed.
        unsafe { SetPropW(HWND(hwnd as *mut _), w!("JasonShell.Snip.Instance"), Some(HANDLE(self.reference.caller.window_id as usize as *mut _))) }.map_err(|_| SnipError::WindowFailed)?;
        let weak = Arc::downgrade(self);
        window.on_window_event(move |event| { if matches!(event, tauri::WindowEvent::Destroyed) { if let Some(entry) = weak.upgrade() { entry.invalidate(); } } });
        let mut state = self.state.lock().map_err(|_| SnipError::WindowFailed)?;
        if state.dead || state.window.is_some() || (self.token.is_some() && Instant::now() >= self.deadline) { return Err(SnipError::WindowFailed); }
        state.hwnd = hwnd; state.window = Some(window); self.changed.notify_all(); Ok(())
    }
    pub fn window(&self) -> Result<WebviewWindow, SnipError> {
        let state = self.state.lock().map_err(|_| SnipError::WindowFailed)?;
        if state.dead { return Err(SnipError::Unauthorized); }
        let window = state.window.clone().ok_or(SnipError::WindowFailed)?;
        let hwnd = state.hwnd;
        drop(state);
        if window.hwnd().map_err(|_| SnipError::Unauthorized)?.0 as isize != hwnd
            || unsafe { GetPropW(HWND(hwnd as *mut _), w!("JasonShell.Snip.Instance")) }.0 as usize as u64 != self.reference.caller.window_id { return Err(SnipError::Unauthorized); }
        Ok(window)
    }
    pub fn retire(&self) -> Option<WebviewWindow> {
        let mut state = self.state.lock().ok()?;
        state.dead = true; state.context = None;
        let window = state.window.take();
        self.changed.notify_all();
        drop(state);
        window.filter(|window| same_epoch(window, self.reference.caller.window_id))
    }
    pub fn commit(&self, context: &Context) -> Result<(), SnipError> {
        self.admit_construction()?;
        if self.token.as_ref() != Some(&context.token) || self.reference.monitor_id.as_ref() != Some(&context.monitor_id) { return Err(SnipError::Unauthorized); }
        let mut state = self.state.lock().map_err(|_| SnipError::WindowFailed)?;
        if state.dead || Instant::now() >= self.deadline { return Err(SnipError::ReadinessTimeout); }
        state.context = Some(context.clone()); self.changed.notify_all(); Ok(())
    }
    pub fn invalidate(&self) { if let Ok(mut state) = self.state.lock() { state.dead = true; state.context = None; self.changed.notify_all(); } }
}
