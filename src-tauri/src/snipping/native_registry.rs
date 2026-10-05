//! Concrete Tauri instance identities and bounded initialization gates.
use super::coordinator::{Caller, Context, SnipError, Token, WindowRef};
use std::{collections::HashMap, sync::{Arc, Condvar, Mutex, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
use tauri::{Manager, WebviewWindow};
use windows::{core::w, Win32::{Foundation::{HANDLE, HWND}, UI::WindowsAndMessaging::{GetPropW, SetPropW}}};

pub struct Entry {
    pub reference: WindowRef,
    pub token: Option<Token>,
    pub preview: bool,
    deadline: Instant,
    state: Mutex<State>,
    changed: Condvar,
}
struct State { window: Option<WebviewWindow>, hwnd: isize, dead: bool, context: Option<Context> }
#[derive(Default)]
pub struct NativeRegistry { entries: Mutex<HashMap<String, Arc<Entry>>>, next: Mutex<u64>, stopped: AtomicBool }
pub fn same_epoch(window: &WebviewWindow, id: u64) -> bool {
    window.hwnd().ok().is_some_and(|hwnd| unsafe { GetPropW(HWND(hwnd.0), w!("JasonShell.Snip.Instance")) }.0 as usize as u64 == id)
}
impl NativeRegistry {
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
        entries.retain(|_, entry| entry.state.lock().map(|state| !state.dead).unwrap_or(false));
        if entries.len() >= 66 { return Err(SnipError::WindowFailed); }
        if entries.contains_key(&label) { return Err(SnipError::WindowFailed); }
        let mut next = self.next.lock().map_err(|_| SnipError::WindowFailed)?;
        *next = next.checked_add(1).ok_or(SnipError::WindowFailed)?;
        if usize::try_from(*next).is_err() { return Err(SnipError::WindowFailed); }
        let entry = Arc::new(Entry { reference: WindowRef { caller: Caller { label: label.clone(), window_id: *next }, monitor_id }, token, preview,
            deadline: Instant::now() + Duration::from_secs(5), state: Mutex::new(State { window: None, hwnd: 0, dead: false, context: None }), changed: Condvar::new() });
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
    pub fn bind(self: &Arc<Self>, window: WebviewWindow) -> Result<(), SnipError> {
        let hwnd = window.hwnd().map_err(|_| SnipError::WindowFailed)?.0 as isize;
        // Window properties are destroyed with the HWND. A recycled HWND cannot
        // inherit this registry epoch, even before Tauri delivers Destroyed.
        unsafe { SetPropW(HWND(hwnd as *mut _), w!("JasonShell.Snip.Instance"), Some(HANDLE(self.reference.caller.window_id as usize as *mut _))) }.map_err(|_| SnipError::WindowFailed)?;
        let weak = Arc::downgrade(self);
        window.on_window_event(move |event| { if matches!(event, tauri::WindowEvent::Destroyed) { if let Some(entry) = weak.upgrade() { entry.invalidate(); } } });
        let mut state = self.state.lock().map_err(|_| SnipError::WindowFailed)?;
        if state.dead || state.window.is_some() { return Err(SnipError::WindowFailed); }
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
        let window = self.window().ok();
        self.invalidate();
        window
    }
    pub fn commit(&self, context: &Context) -> Result<(), SnipError> {
        if self.token.as_ref() != Some(&context.token) || self.reference.monitor_id.as_ref() != Some(&context.monitor_id) { return Err(SnipError::Unauthorized); }
        let mut state = self.state.lock().map_err(|_| SnipError::WindowFailed)?;
        if state.dead || Instant::now() >= self.deadline { return Err(SnipError::ReadinessTimeout); }
        state.context = Some(context.clone()); self.changed.notify_all(); Ok(())
    }
    pub fn invalidate(&self) { if let Ok(mut state) = self.state.lock() { state.dead = true; state.context = None; self.changed.notify_all(); } }
}
