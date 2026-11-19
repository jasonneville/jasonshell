//! Concrete Tauri instance identities and bounded initialization gates.
use super::coordinator::{Caller, Context, SnipError, Token, WindowRef};
use std::{collections::HashMap, sync::{Arc, Condvar, Mutex, OnceLock, mpsc, atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering}}, time::{Duration, Instant}};
use tauri::{Manager, WebviewWindow};
use windows::{core::w, Win32::{Foundation::{HANDLE, HWND}, UI::WindowsAndMessaging::{GetPropW, SetPropW}}};
/// Native adapter output, never deserialized or accepted from IPC. Each read
/// must resolve the current registration and HWND epoch afresh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FocusRegistration {
    pub hwnd: isize, pub pid: u32, pub thread: u32, pub instance: u64, pub token: Option<Token>,
}

type PreviewRetired = Arc<dyn Fn(&Caller) + Send + Sync>;

pub struct Entry {
    pub reference: WindowRef,
    pub token: Option<Token>,
    pub preview: bool,
    deadline: Instant,
    state: Mutex<State>,
    changed: Condvar,
    retired_generation: Arc<AtomicU64>,
    preview_retired: Option<PreviewRetired>,
}
struct State { window: Option<WebviewWindow>, retired_window: Option<WebviewWindow>, close_queued: bool, identity_failed: bool, hwnd: isize, dead: bool, context: Option<Context> }
#[derive(Default)]
pub struct NativeRegistry { entries: Mutex<HashMap<String, Arc<Entry>>>, retired: Mutex<Vec<Arc<Entry>>>, next: Mutex<u64>, stopped: AtomicBool, ui_in_flight: Arc<AtomicUsize>, origin_generation: Mutex<Option<Token>>, retired_generation: Arc<AtomicU64>, preview_retired: OnceLock<PreviewRetired> }
pub struct ConstructionPermit(Arc<AtomicUsize>);
impl Drop for ConstructionPermit { fn drop(&mut self) { self.0.fetch_sub(1, Ordering::AcqRel); } }
pub fn same_epoch(window: &WebviewWindow, id: u64) -> bool {
    window.hwnd().ok().is_some_and(|hwnd| hwnd_has_epoch(hwnd.0 as isize, id))
}
fn hwnd_has_epoch(hwnd: isize, id: u64) -> bool {
    hwnd != 0 && id != 0 && unsafe { GetPropW(HWND(hwnd as *mut _), w!("JasonShell.Snip.Instance")) }.0 as usize as u64 == id
}
impl NativeRegistry {
    pub fn admit_construction_callback(&self) -> Result<ConstructionPermit, SnipError> {
        let _entries = self.entries.lock().map_err(|_| SnipError::Inactive)?;
        if self.stopped.load(Ordering::Acquire) { return Err(SnipError::Inactive); }
        self.ui_in_flight.fetch_add(1, Ordering::AcqRel);
        Ok(ConstructionPermit(self.ui_in_flight.clone()))
    }
    pub(crate) fn on_preview_retired(&self, callback: impl Fn(&Caller) + Send + Sync + 'static) {
        let _ = self.preview_retired.set(Arc::new(callback));
    }
    pub fn retire_pending_generation(&self, token: &Token) -> Vec<(WebviewWindow, u64)> {
        let Some(generation) = token.generation.parse::<u64>().ok().filter(|generation| *generation > 0) else { return Vec::new(); };
        self.retired_generation.fetch_max(generation, Ordering::AcqRel);
        let entries = self.entries.lock().map(|entries| entries.values().filter(|entry| entry.token.as_ref() == Some(token)).cloned().collect::<Vec<_>>()).unwrap_or_default();
        entries.into_iter().filter_map(|entry| {
            let mut state = entry.state.lock().ok()?;
            if state.context.is_some() { return None; }
            let was_live = !state.dead;
            state.dead = true; let window = state.window.take(); if window.is_some() { state.retired_window = window.clone(); } entry.changed.notify_all();
            drop(state);
            if was_live { entry.notify_preview_retired(); }
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
        let mut retired = self.retired.lock().map_err(|_| SnipError::WindowFailed)?;
        retired.retain(|entry| entry.state.lock().map(|state| state.retired_window.is_some() || state.window.is_some()).unwrap_or(true));
        entries.retain(|_, entry| {
            if entry.state.lock().map(|state| state.dead).unwrap_or(false) { retired.push(entry.clone()); false } else { true }
        });
        if entries.len() >= 66 { return Err(SnipError::WindowFailed); }
        if entries.contains_key(&label) { return Err(SnipError::WindowFailed); }
        let mut next = self.next.lock().map_err(|_| SnipError::WindowFailed)?;
        *next = next.checked_add(1).ok_or(SnipError::WindowFailed)?;
        if usize::try_from(*next).is_err() { return Err(SnipError::WindowFailed); }
        let entry = Arc::new(Entry { reference: WindowRef { caller: Caller { label: label.clone(), window_id: *next }, monitor_id }, token, preview,
            deadline: Instant::now() + Duration::from_secs(5), state: Mutex::new(State { window: None, retired_window: None, close_queued: false, identity_failed: false, hwnd: 0, dead: false, context: None }), changed: Condvar::new(), retired_generation: self.retired_generation.clone(),
            preview_retired: if preview { self.preview_retired.get().cloned() } else { None } });
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
        Self::resolve_entry(window, &entry)
    }
    fn resolve_entry(window: &WebviewWindow, entry: &Entry) -> Result<Caller, SnipError> {
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
    pub(crate) fn resolve_image(&self, window: &WebviewWindow) -> Result<Caller, SnipError> {
        let entry = self.entries.lock().map_err(|_| SnipError::WindowFailed)?.get(window.label()).cloned().ok_or(SnipError::Unauthorized)?;
        if !entry.preview { return Self::resolve_entry(window, &entry); }
        // Label selects a candidate only. Query the *injected* instance on the
        // UI thread (inline HWND getter) with a bounded worker-side receive.
        let deadline = Instant::now() + Duration::from_secs(5);
        let (send, receive) = mpsc::sync_channel(1);
        let injected = window.clone();
        window.run_on_main_thread(move || {
            if Instant::now() < deadline {
                let hwnd = injected.hwnd().map(|hwnd| hwnd.0 as isize).map_err(|_| SnipError::Unauthorized);
                let _ = send.try_send(hwnd);
            }
        }).map_err(|_| SnipError::WindowFailed)?;
        let result = receive.recv_timeout(deadline.saturating_duration_since(Instant::now()));
        let hwnd = match result {
            _ if Instant::now() >= deadline => return Err(SnipError::ReadinessTimeout),
            Ok(hwnd) => hwnd?,
            Err(mpsc::RecvTimeoutError::Timeout) => return Err(SnipError::ReadinessTimeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err(SnipError::WindowFailed),
        };
        entry.verify_bound_hwnd(Some(hwnd))?;
        Ok(entry.reference.caller.clone())
    }
    pub(crate) fn image_window_is_live(&self, reference: &WindowRef) -> bool {
        self.entry(reference).is_ok_and(|entry| if entry.preview { entry.verify_bound_hwnd(None).is_ok() } else { entry.window().is_ok() })
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
    /// Drain exact registered helper instances, including previously queued closes.
    /// Ordinary Settings/Stack/bar entries are never destroyed by this operation.
    pub fn stop_and_drain_snips(&self) -> Result<(), String> {
        let mut entries = {
            let entries = self.entries.lock().map_err(|_| "Snipping registry unavailable")?;
            self.stopped.store(true, Ordering::Release);
            entries.values().filter(|entry| entry.token.is_some()).cloned().collect::<Vec<_>>()
        };
        entries.extend(self.retired.lock().map_err(|_| "Snipping retirement ledger unavailable")?.iter().filter(|entry| entry.token.is_some()).cloned());
        for entry in &entries {
            entry.retire();
            let window = {
                let mut state = entry.state.lock().map_err(|_| "Snipping window ownership unavailable")?;
                if state.close_queued { None } else {
                    let window = state.retired_window.clone();
                    state.close_queued = window.is_some(); window
                }
            };
            if let Some(window) = window {
                let app = window.app_handle().clone();
                let closing = entry.clone(); let id = entry.reference.caller.window_id;
                if app.run_on_main_thread(move || {
                    let failed_construction = closing.state.lock().map(|state| state.identity_failed).unwrap_or(false);
                    if failed_construction || same_epoch(&window, id) { let _ = window.destroy(); }
                    if let Ok(mut state) = closing.state.lock() { state.close_queued = false; }
                }).is_err() { entry.state.lock().map_err(|_| "Snipping close ledger unavailable")?.close_queued = false; return Err("Snipping close dispatch failed".into()); }
            }
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let mut pending = self.ui_in_flight.load(Ordering::Acquire) != 0;
            for entry in &entries {
                let (window, hwnd, identity_failed) = { let state = entry.state.lock().map_err(|_| "Snipping window ownership unavailable")?; (state.retired_window.clone().or_else(|| state.window.clone()), state.hwnd, state.identity_failed) };
                if let Some(window) = window {
                    if hwnd == 0 { pending = true; continue; }
                    // The original native epoch disappearing is a completion
                    // proof; a replacement HWND must never be acted on.
                    let live = unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindow(Some(HWND(hwnd as *mut _))) }.as_bool();
                    if live && (identity_failed || same_epoch(&window, entry.reference.caller.window_id)) { pending = true; }
                    else { let mut state = entry.state.lock().map_err(|_| "Snipping close ledger unavailable")?; state.window = None; state.retired_window = None; }
                }
            }
            if !pending { return Ok(()); }
            if Instant::now() >= deadline { return Err("Snipping native windows still retiring; ownership retained".into()); }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Entry {
    pub fn retain_failed_construction(self: &Arc<Self>, window: &WebviewWindow) {
        let weak = Arc::downgrade(self);
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                if let Some(entry) = weak.upgrade() { if let Ok(mut state) = entry.state.lock() { state.window = None; state.retired_window = None; } }
            }
        });
        let hwnd = window.hwnd().ok().map(|hwnd| hwnd.0 as isize).unwrap_or(0);
        let identified = hwnd != 0 && unsafe { SetPropW(HWND(hwnd as *mut _), w!("JasonShell.Snip.Instance"), Some(HANDLE(self.reference.caller.window_id as usize as *mut _))) }.is_ok();
        let mut state = self.state.lock().unwrap_or_else(|poison| poison.into_inner());
        state.dead = true; state.hwnd = hwnd; state.identity_failed = !identified; state.retired_window = Some(window.clone());
    }
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
        window.on_window_event(move |event| { if matches!(event, tauri::WindowEvent::Destroyed) { if let Some(entry) = weak.upgrade() { entry.invalidate(); if let Ok(mut state) = entry.state.lock() { state.window = None; state.retired_window = None; } } } });
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
        let was_live = !state.dead;
        state.dead = true; state.context = None;
        let window = state.window.take();
        if window.is_some() { state.retired_window = window.clone(); }
        self.changed.notify_all();
        drop(state);
        if was_live { self.notify_preview_retired(); }
        window.filter(|window| same_epoch(window, self.reference.caller.window_id))
    }
    pub fn commit(&self, context: &Context) -> Result<(), SnipError> {
        self.admit_construction()?;
        if self.token.as_ref() != Some(&context.token) || self.reference.monitor_id.as_ref() != Some(&context.monitor_id) { return Err(SnipError::Unauthorized); }
        let mut state = self.state.lock().map_err(|_| SnipError::WindowFailed)?;
        if state.dead || Instant::now() >= self.deadline { return Err(SnipError::ReadinessTimeout); }
        state.context = Some(context.clone()); self.changed.notify_all(); Ok(())
    }
    pub fn invalidate(&self) {
        if let Ok(mut state) = self.state.lock() {
            let was_live = !state.dead;
            state.dead = true; state.context = None; self.changed.notify_all();
            drop(state);
            if was_live { self.notify_preview_retired(); }
        }
    }
    // After resolving the concrete injected caller, use the registered HWND from
    // bind(). Its property disappears on destruction; a recycled HWND cannot
    // inherit the epoch. No Tauri/UI dispatch on preview image revalidation.
    fn verify_bound_hwnd(&self, injected_hwnd: Option<isize>) -> Result<(), SnipError> {
        let state = self.state.lock().map_err(|_| SnipError::WindowFailed)?;
        if state.dead || state.window.is_none() || injected_hwnd.is_some_and(|hwnd| hwnd != state.hwnd)
            || !hwnd_has_epoch(state.hwnd, self.reference.caller.window_id) { return Err(SnipError::Unauthorized); }
        Ok(())
    }
    // All callers publish death and release registry/entry locks before calling
    // into the coordinator's wait predicate.
    fn notify_preview_retired(&self) {
        if let Some(notify) = &self.preview_retired { notify(&self.reference.caller); }
    }
}
