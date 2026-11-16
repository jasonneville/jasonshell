use crate::settings::StandardHotkeySettings;

#[cfg(windows)]
use std::sync::{mpsc, Mutex, OnceLock};
#[cfg(windows)]
use std::{cell::RefCell, sync::{Arc, atomic::{AtomicBool, AtomicU8, Ordering}}};
#[cfg(windows)]
use std::thread::{self, JoinHandle};
#[cfg(windows)]
use tauri::{AppHandle, Emitter};
#[cfg(windows)]
use windows::Win32::System::Threading::GetCurrentThreadId;
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, VK_1,
    VK_OEM_3, VK_SPACE,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, KillTimer, PeekMessageW, PostThreadMessageW,
    SetTimer, SetWindowsHookExW, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, LLKHF_INJECTED,
    MSG, PM_NOREMOVE, WH_KEYBOARD_LL, WM_APP, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER,
};
#[cfg(windows)]
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};

#[cfg(test)]
pub const SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT: &str =
    crate::contracts::events::SEARCH_TOGGLE_CENTERED;
#[rustfmt::skip]
#[cfg(test)]
pub const TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT: &str = crate::contracts::events::TERMINAL_TOGGLE_PANEL;
#[cfg(test)]
pub const STACK_BROWSER_HOTKEY_TOGGLE_STACK_BROWSER_EVENT: &str =
    crate::contracts::events::STACK_BROWSER_TOGGLE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfiguredHotkeyAction {
    Search,
    Terminal,
    StackBrowser,
    Speech,
    Snipping,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HotkeyModifier {
    Ctrl,
    Alt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RegistryBinding {
    modifier: HotkeyModifier,
    key: u32,
    action: ConfiguredHotkeyAction,
}

#[derive(Clone, Copy, Default)]
struct SearchModifiers {
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
}

// Fixed virtual-key space; no allocation or foreground-window dependency.
// Separate held/owned bits preserve unowned cycles and releases across rebinds.
struct SearchKeyOwnership {
    binding: Option<RegistryBinding>,
    held: [bool; 256],
    owned: [bool; 256],
}

impl Default for SearchKeyOwnership {
    fn default() -> Self {
        Self { binding: None, held: [false; 256], owned: [false; 256] }
    }
}

impl SearchKeyOwnership {
    fn set_binding(&mut self, binding: Option<RegistryBinding>, trigger_preheld: bool) {
        let binding = binding.filter(|b| b.action == ConfiguredHotkeyAction::Search);
        if let Some(binding) = binding {
            if let Some(held) = self.held.get_mut(binding.key as usize) {
                *held |= trigger_preheld;
            }
        }
        self.binding = binding;
    }

    fn handle_event(&mut self, code: i32, key: u32, is_down: bool, modifiers: SearchModifiers,
        mut queue: impl FnMut() -> bool) -> bool {
        if code < 0 || key >= 256 { return false; }
        let index = key as usize;
        let was_held = self.held[index];
        self.held[index] = is_down;
        if self.owned[index] {
            if !is_down { self.owned[index] = false; }
            return true;
        }
        if !is_down || was_held || modifiers.shift || modifiers.win { return false; }
        let Some(binding) = self.binding else { return false; };
        let matches = binding.key == key && match binding.modifier {
            HotkeyModifier::Ctrl => modifiers.ctrl && !modifiers.alt,
            HotkeyModifier::Alt => modifiers.alt && !modifiers.ctrl,
        };
        if matches && queue() {
            self.owned[index] = true;
            return true;
        }
        false
    }
}

fn reconcile_search_startup_keys(mut snapshot: [bool; 256], observed: [Option<bool>; 256]) -> [bool; 256] {
    for (sampled, transition) in snapshot.iter_mut().zip(observed) {
        if let Some(down) = transition { *sampled = down; }
    }
    snapshot
}

#[cfg(windows)]
const WM_SEARCH_TOGGLE: u32 = WM_APP + 43;

#[cfg(windows)]
struct SearchDispatch {
    id: u32,
    pending: AtomicBool,
    retired: AtomicBool,
    // Bit 0 retires admission permanently; bit 1 owns one emission permit.
    emission_gate: AtomicU8,
}

#[cfg(windows)]
struct SearchEmissionPermit<'a>(&'a SearchDispatch);

#[cfg(windows)]
impl Drop for SearchEmissionPermit<'_> {
    fn drop(&mut self) {
        self.0.pending.store(false, Ordering::Release);
        // Never erase concurrent retirement when releasing the admitted work.
        self.0.emission_gate.fetch_and(!2, Ordering::AcqRel);
    }
}

#[cfg(windows)]
impl SearchDispatch {
    fn try_admit_emission(&self) -> Option<SearchEmissionPermit<'_>> {
        if !self.pending.load(Ordering::Acquire) { return None; }
        self.emission_gate.compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
            .ok().map(|_| SearchEmissionPermit(self))
    }

    fn retire(&self) {
        // Linearizes against permit acquisition, without waiting for IPC. Only
        // an emission admitted before this RMW may complete after retirement.
        self.emission_gate.fetch_or(1, Ordering::AcqRel);
        self.retired.store(true, Ordering::Release);
    }

    // One outstanding delivery. Admission failure passes the entire key cycle.
    fn queue(&self) -> bool {
        if self.retired.load(Ordering::Acquire) || self.emission_gate.load(Ordering::Acquire) & 1 != 0
            || self.pending.compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed).is_err() {
            return false;
        }
        if unsafe { PostThreadMessageW(self.id, WM_SEARCH_TOGGLE, WPARAM(0), LPARAM(0)) }.is_err() {
            self.pending.store(false, Ordering::Release);
            return false;
        }
        true
    }
}

#[cfg(windows)]
struct SearchHookState {
    ownership: SearchKeyOwnership,
    // LL callbacks run before asynchronous state updates. Seed once, then track
    // left/right modifier transitions locally, rather than querying in callback.
    pressed: [bool; 256],
    startup_observed: Option<[Option<bool>; 256]>,
    dispatch: Arc<SearchDispatch>,
}

#[cfg(windows)]
thread_local! {
    static SEARCH_HOOK: RefCell<Option<SearchHookState>> = const { RefCell::new(None) };
}

#[cfg(windows)]
unsafe extern "system" fn search_keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }
    let transition = wparam.0 as u32;
    if !matches!(transition, WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP) {
        return CallNextHookEx(None, code, wparam, lparam);
    }
    // Windows owns this pointer for this callback only. Never retain it.
    let event = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
    // Injected input passes unchanged and cannot release physical ownership.
    if event.flags.contains(LLKHF_INJECTED) || event.vkCode >= 256 {
        return CallNextHookEx(None, code, wparam, lparam);
    }
    let consumed = SEARCH_HOOK.with(|slot| {
        let Ok(mut slot) = slot.try_borrow_mut() else { return false; };
        let Some(state) = slot.as_mut() else { return false; };
        let down = matches!(transition, WM_KEYDOWN | WM_SYSKEYDOWN);
        state.pressed[event.vkCode as usize] = down;
        if let Some(observed) = state.startup_observed.as_mut() {
            observed[event.vkCode as usize] = Some(down);
        }
        let keys = &state.pressed;
        let modifiers = SearchModifiers {
            ctrl: keys[0xa2] || keys[0xa3],
            alt: keys[0xa4] || keys[0xa5],
            shift: keys[0xa0] || keys[0xa1],
            win: keys[0x5b] || keys[0x5c],
        };
        state.ownership.handle_event(code, event.vkCode, down, modifiers, || state.dispatch.queue())
    });
    if consumed { LRESULT(1) } else { CallNextHookEx(None, code, wparam, lparam) }
}

#[cfg(windows)]
fn refresh_search_binding(binding: Option<RegistryBinding>) {
    SEARCH_HOOK.with(|slot| {
        if let Some(state) = slot.borrow_mut().as_mut() {
            let preheld = binding.is_some_and(|b| {
                state.pressed.get(b.key as usize).copied().unwrap_or(false)
                    || unsafe { GetAsyncKeyState(b.key as i32) } < 0
            });
            state.ownership.set_binding(binding, preheld);
        }
    });
}

// Emission is deliberately not on the hook thread. A timer provides a bounded
// retirement wake even if posting a stop message would fail/full-queue.
#[cfg(windows)]
fn start_search_dispatch(app: AppHandle) -> Result<(Arc<SearchDispatch>, JoinHandle<()>), String> {
    let (ready, wait) = mpsc::sync_channel(1);
    let worker = thread::Builder::new().name("jasonshell-search-dispatch".into()).spawn(move || {
        let mut message = MSG::default();
        unsafe { let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE); }
        let timer = unsafe { SetTimer(None, 0, 100, None) };
        if timer == 0 {
            let _ = ready.send(Err("failed to create Search dispatch retirement timer".to_string()));
            return;
        }
        let dispatch = Arc::new(SearchDispatch {
            id: unsafe { GetCurrentThreadId() }, pending: AtomicBool::new(false), retired: AtomicBool::new(false),
            emission_gate: AtomicU8::new(0),
        });
        if ready.send(Ok(dispatch.clone())).is_ok() {
            while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
                if dispatch.retired.load(Ordering::Acquire) { break; }
                if message.message == WM_SEARCH_TOGGLE && dispatch.pending.load(Ordering::Acquire) {
                    let Some(permit) = dispatch.try_admit_emission() else { continue; };
                    let _ = app.emit_to(crate::shell_windows::TOP_BAR_LABEL,
                        crate::contracts::events::SEARCH_TOGGLE_CENTERED, ());
                    drop(permit);
                }
            }
        }
        dispatch.retire();
        dispatch.pending.store(false, Ordering::Release);
        unsafe { let _ = KillTimer(None, timer); }
    }).map_err(|error| format!("failed to start Search dispatcher: {error}"))?;
    match wait.recv() {
        Ok(Ok(dispatch)) => Ok((dispatch, worker)),
        result => {
            let _ = worker.join();
            Err(match result {
                Ok(Err(error)) => error,
                _ => "Search dispatcher failed to start".into(),
            })
        }
    }
}

#[cfg(windows)]
type ActiveBindings = [Option<RegistryBinding>; 5];

#[derive(Clone, Debug)]
pub struct StandardHotkeyRegistry {
    bindings: [Option<RegistryBinding>; 5],
}

impl StandardHotkeyRegistry {
    fn from_settings(settings: &StandardHotkeySettings) -> Result<Self, String> {
        Self::from_loaded_settings(settings, false)
    }
    fn from_loaded_settings(settings: &StandardHotkeySettings, missing_snipping: bool) -> Result<Self, String> {
        let settings = crate::settings::validate_loaded_hotkeys(settings.clone(), missing_snipping)?;
        let disabled = missing_snipping && [&settings.search, &settings.terminal, &settings.stack_browser, &settings.speech_transcription]
            .iter().any(|binding| binding.0 == settings.snipping.0);
        Ok(Self {
            bindings: [
                Some(registry_binding(&settings.search.0, ConfiguredHotkeyAction::Search)?),
                Some(registry_binding(&settings.terminal.0, ConfiguredHotkeyAction::Terminal)?),
                Some(registry_binding(
                    &settings.stack_browser.0,
                    ConfiguredHotkeyAction::StackBrowser,
                )?),
                Some(registry_binding(
                    &settings.speech_transcription.0,
                    ConfiguredHotkeyAction::Speech,
                )?),
                if disabled { None } else { Some(registry_binding(&settings.snipping.0, ConfiguredHotkeyAction::Snipping)?) },
            ],
        })
    }
}

fn registry_binding(
    value: &str,
    action: ConfiguredHotkeyAction,
) -> Result<RegistryBinding, String> {
    let canonical = crate::settings::canonicalize_hotkey_binding(value)?.0;
    let (modifier, key) = canonical
        .split_once('+')
        .ok_or_else(|| "invalid canonical hotkey".to_string())?;
    let modifier = match modifier {
        "Ctrl" => HotkeyModifier::Ctrl,
        "Alt" => HotkeyModifier::Alt,
        _ => return Err("unsupported hotkey modifier".to_string()),
    };
    // VK_OEM_3 is layout-sensitive: Backquote means this physical Windows virtual key.
    let key = match key {
        "Space" => space_virtual_key(),
        "Backquote" => backquote_virtual_key(),
        "1" => one_virtual_key(),
        _ if key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric() => {
            key.as_bytes()[0] as u32
        }
        _ => return Err("unsupported canonical hotkey key".to_string()),
    };
    Ok(RegistryBinding {
        modifier,
        key,
        action,
    })
}

#[cfg(windows)]
fn space_virtual_key() -> u32 {
    VK_SPACE.0 as u32
}
#[cfg(not(windows))]
fn space_virtual_key() -> u32 {
    0x20
}
#[cfg(windows)]
fn backquote_virtual_key() -> u32 {
    VK_OEM_3.0 as u32
}
#[cfg(not(windows))]
fn backquote_virtual_key() -> u32 {
    0xc0
}
#[cfg(windows)]
fn one_virtual_key() -> u32 {
    VK_1.0 as u32
}
#[cfg(not(windows))]
fn one_virtual_key() -> u32 {
    b'1' as u32
}

#[cfg(windows)]
const HOTKEY_IDS: [i32; 5] = [1, 2, 3, 4, 5];
#[cfg(windows)]
const WM_REPLACE_HOTKEYS: u32 = WM_APP + 41;
#[cfg(windows)]
const WM_STOP_HOTKEYS: u32 = WM_APP + 42;

#[cfg(windows)]
struct HotkeyThread {
    id: u32,
    sender: mpsc::Sender<ReplaceRequest>,
    worker: JoinHandle<()>,
    stopping: bool,
}

#[cfg(windows)]
struct ReplaceRequest {
    registry: StandardHotkeyRegistry,
    result: mpsc::SyncSender<Result<(), String>>,
}

#[cfg(windows)]
static HOTKEY_THREAD: OnceLock<Mutex<Option<HotkeyThread>>> = OnceLock::new();

#[cfg(windows)]
fn hotkey_thread() -> &'static Mutex<Option<HotkeyThread>> {
    HOTKEY_THREAD.get_or_init(|| Mutex::new(None))
}

// Serialize lifecycle operations without retaining a global mutex during native
// registration, worker startup, IPC dispatch, or bounded shutdown waits.
#[cfg(windows)]
static HOTKEY_OPERATION: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
struct HotkeyAccess { state: Option<HotkeyThread> }
#[cfg(windows)]
impl std::ops::Deref for HotkeyAccess {
    type Target = Option<HotkeyThread>;
    fn deref(&self) -> &Self::Target { &self.state }
}
#[cfg(windows)]
impl std::ops::DerefMut for HotkeyAccess {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.state }
}
#[cfg(windows)]
impl Drop for HotkeyAccess {
    fn drop(&mut self) {
        *hotkey_thread().lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = self.state.take();
        HOTKEY_OPERATION.store(false, Ordering::Release);
    }
}
#[cfg(windows)]
fn hotkey_access() -> Result<HotkeyAccess, String> {
    HOTKEY_OPERATION.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .map_err(|_| "hotkey lifecycle operation is already in progress".to_string())?;
    let mut access = HotkeyAccess { state: None };
    access.state = hotkey_thread().lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
    Ok(access)
}

#[cfg(windows)]
fn modifiers(binding: RegistryBinding) -> HOT_KEY_MODIFIERS {
    MOD_NOREPEAT
        | match binding.modifier {
            HotkeyModifier::Ctrl => MOD_CONTROL,
            HotkeyModifier::Alt => MOD_ALT,
        }
}

#[cfg(windows)]
fn register_binding(index: usize, binding: RegistryBinding) -> Result<(), String> {
    unsafe { RegisterHotKey(None, HOTKEY_IDS[index], modifiers(binding), binding.key) }
        .map_err(|error| format!("failed to register {:?} hotkey: {error}", binding.action))
}

#[cfg(windows)]
fn unregister_binding(index: usize) {
    let _ = unsafe { UnregisterHotKey(None, HOTKEY_IDS[index]) };
}

// All OS registrations and their rollback live on the owning message thread.
// Unchanged chords stay registered, avoiding needless conflicts with other apps.
fn replace_registrations(
    current: &mut ActiveBindings,
    next: StandardHotkeyRegistry,
    mut register: impl FnMut(usize, RegistryBinding) -> Result<(), String>,
    mut unregister: impl FnMut(usize),
) -> Result<(), String> {
    let changed: Vec<usize> = (0..HOTKEY_IDS.len())
        .filter(|&index| {
            current[index] != next.bindings[index]
        })
        .collect();
    let previous = *current;
    for &index in &changed {
        if current[index].is_some() {
            unregister(index);
            current[index] = None;
        }
    }
    for &index in &changed {
        let Some(binding) = next.bindings[index] else { continue; };
        if let Err(error) = register(index, binding) {
            for &registered_index in &changed {
                if current[registered_index].is_some() {
                    unregister(registered_index);
                    current[registered_index] = None;
                }
            }
            let mut restore_errors = Vec::new();
            for &old_index in &changed {
                if let Some(binding) = previous[old_index] {
                    if let Err(restore_error) = register(old_index, binding) {
                        restore_errors.push(restore_error);
                    } else {
                        current[old_index] = Some(binding);
                    }
                }
            }
            if !restore_errors.is_empty() {
                return Err(format!(
                    "{error}; rollback failed; remaining hotkeys retained: {}",
                    restore_errors.join("; ")
                ));
            }
            return Err(error);
        }
        current[index] = Some(binding);
    }
    Ok(())
}

#[cfg(windows)]
use crate::speech_runtime::SpeechRuntimeState;
#[cfg(windows)]
use tauri::Manager;

#[cfg(windows)]
#[rustfmt::skip]
fn dispatch_hotkey(app_handle: &AppHandle, id: i32, registry: &ActiveBindings) {
    let Some(index) = HOTKEY_IDS
        .iter()
        .position(|registered_id| *registered_id == id)
    else {
        return;
    };
    let Some(binding) = registry[index] else { return; };
    match binding.action {
        ConfiguredHotkeyAction::Search => {
            // Registration reserves/conflict-checks the chord; only the native
            // trigger owner queues Search. Never double-toggle via WM_HOTKEY.
        }
        ConfiguredHotkeyAction::Terminal => {
            let _ = app_handle.emit_to(crate::shell_windows::TOP_BAR_LABEL, crate::contracts::events::TERMINAL_TOGGLE_PANEL, ());
        }
        ConfiguredHotkeyAction::StackBrowser => {
            let _ = app_handle.emit_to(
                crate::shell_windows::TOP_BAR_LABEL,
                crate::contracts::events::STACK_BROWSER_TOGGLE,
                (),
            );
        }
        ConfiguredHotkeyAction::Speech => {
            // Capture on the hotkey message thread, before UI delivery can change focus.
            let Some(reservation) = app_handle
                .state::<SpeechRuntimeState>()
                .prepare_speech_paste_target() else { return; };
            if app_handle.emit_to(
                crate::shell_windows::TOP_BAR_LABEL,
                crate::contracts::events::SPEECH_TOGGLE,
                reservation,
            ).is_err() {
                if let crate::speech::SpeechHotkeyActivation::Start { reservation_id } = reservation {
                    app_handle.state::<SpeechRuntimeState>().cancel_speech_preparation(reservation_id);
                }
            }
        }
        ConfiguredHotkeyAction::Snipping => {
            // Bind foreground intent on the WM_HOTKEY thread, before scheduling
            // can change focus. Capture and the renderer barrier stay off-thread.
            let Some(runtime) = app_handle.try_state::<std::sync::Arc<crate::snipping::runtime::SnipRuntime>>() else { return; };
            let runtime = runtime.inner().clone();
            let Ok(ticket) = runtime.reserve_native_start() else { return; };
            // The runtime owns the blocking task; the RAII ticket releases its
            // reservation on completion or if task creation drops the closure.
            tauri::async_runtime::spawn_blocking(move || { let _ = runtime.run_native_start(ticket); });
        }
    }
}

#[cfg(windows)]
fn run_hotkey_thread(
    app_handle: AppHandle,
    requests: mpsc::Receiver<ReplaceRequest>,
    ready: mpsc::SyncSender<Result<u32, String>>,
) {
    let mut message = MSG::default();
    // Force creation of this thread's message queue before publishing its ID.
    unsafe {
        let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
    }
    let (dispatch, dispatch_worker) = match start_search_dispatch(app_handle.clone()) {
        Ok(dispatcher) => dispatcher,
        Err(error) => { let _ = ready.send(Err(error)); return; }
    };
    // Install inactive observation first. A pre-hook sample could miss a Ctrl
    // release in the installation gap and misclassify later ordinary Space.
    SEARCH_HOOK.with(|slot| *slot.borrow_mut() = Some(SearchHookState {
        ownership: SearchKeyOwnership::default(), pressed: [false; 256],
        startup_observed: Some([None; 256]), dispatch: dispatch.clone(),
    }));
    // The callback is in the process image, whose module remains loaded until
    // process exit. Supply its handle explicitly for this desktop-wide hook.
    let hook = match unsafe { GetModuleHandleW(None) }.and_then(|module| unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(search_keyboard_hook),
            Some(windows::Win32::Foundation::HINSTANCE(module.0)), 0)
    }) {
        Ok(hook) => hook,
        Err(error) => {
            SEARCH_HOOK.with(|slot| *slot.borrow_mut() = None);
            dispatch.retire();
            let _ = dispatch_worker.join();
            let _ = ready.send(Err(format!("failed to install Search keyboard hook: {error}")));
            return;
        }
    };
    let mut snapshot = [false; 256];
    for (key, down) in snapshot.iter_mut().enumerate() {
        *down = unsafe { GetAsyncKeyState(key as i32) } < 0;
    }
    SEARCH_HOOK.with(|slot| {
        if let Some(state) = slot.borrow_mut().as_mut() {
            // LL callbacks precede async-state updates: their observed transitions
            // must override any older per-key sample. No Win32 calls while borrowed.
            let pressed = reconcile_search_startup_keys(snapshot,
                state.startup_observed.take().unwrap_or([None; 256]));
            state.pressed = pressed;
            state.ownership.held = pressed;
        }
    });
    let published = ready.send(Ok(unsafe { GetCurrentThreadId() })).is_ok();
    let mut current: ActiveBindings = [None; 5];
    let mut stopping_at = None;
    let mut drain_timer = 0;
    while published && unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        match message.message {
            WM_REPLACE_HOTKEYS => {
                while let Ok(request) = requests.try_recv() {
                    if stopping_at.is_some() {
                        let _ = request.result.send(Err("hotkey message thread is stopping".into()));
                        continue;
                    }
                    // Do not acquire a chord while its OS ownership is changing.
                    // Already-owned releases still drain during replacement.
                    refresh_search_binding(None);
                    let result = replace_registrations(
                        &mut current,
                        request.registry,
                        register_binding,
                        unregister_binding,
                    );
                    // Even failed rollback can change the verified active set.
                    refresh_search_binding(current[0]);
                    let _ = request.result.send(result);
                }
            }
            WM_HOTKEY => {
                if stopping_at.is_none() {
                    dispatch_hotkey(&app_handle, message.wParam.0 as i32, &current);
                }
            }
            WM_STOP_HOTKEYS => {
                if stopping_at.is_none() {
                    dispatch.retire();
                    refresh_search_binding(None);
                    stopping_at = Some(std::time::Instant::now());
                    drain_timer = unsafe { SetTimer(None, 0, 10, None) };
                }
            }
            WM_TIMER => {}
            _ => {}
        }
        if let Some(start) = stopping_at {
            let owned = SEARCH_HOOK.with(|slot| slot.borrow().as_ref()
                .is_some_and(|s| s.ownership.owned.iter().any(|owned| *owned)));
            if !owned || drain_timer == 0 || start.elapsed() >= std::time::Duration::from_millis(250) { break; }
        }
    }
    dispatch.retire();
    if drain_timer != 0 { unsafe { let _ = KillTimer(None, drain_timer); } }
    if let Err(error) = unsafe { UnhookWindowsHookEx(hook) } {
        eprintln!("failed to unhook Search keyboard hook: {error}");
    }
    SEARCH_HOOK.with(|slot| *slot.borrow_mut() = None);
    for (index, binding) in current.iter().enumerate() {
        if binding.is_some() {
            unregister_binding(index);
        }
    }
    // Retired queued work cannot be admitted; a pre-retirement permit may
    // finish before this thread is reaped and another install can begin.
    let _ = dispatch_worker.join();
}

#[cfg(windows)]
fn request_replacement(
    thread: &HotkeyThread,
    registry: StandardHotkeyRegistry,
) -> Result<(), String> {
    let (result_tx, result_rx) = mpsc::sync_channel(1);
    thread
        .sender
        .send(ReplaceRequest {
            registry,
            result: result_tx,
        })
        .map_err(|_| "hotkey message thread stopped".to_string())?;
    unsafe {
        PostThreadMessageW(
            thread.id,
            WM_REPLACE_HOTKEYS,
            Default::default(),
            Default::default(),
        )
    }
    .map_err(|error| format!("failed to notify hotkey message thread: {error}"))?;
    result_rx
        .recv()
        .map_err(|_| "hotkey message thread stopped".to_string())?
}

#[cfg(windows)]
pub fn install_windows_key_hook(
    app_handle: AppHandle,
    hotkeys: StandardHotkeySettings,
) -> Result<(), String> {
    install_loaded_windows_key_hook(app_handle, hotkeys, false)
}
#[cfg(windows)]
pub fn install_loaded_windows_key_hook(app_handle: AppHandle, hotkeys: StandardHotkeySettings, missing_snipping: bool) -> Result<(), String> {
    let registry = StandardHotkeyRegistry::from_loaded_settings(&hotkeys, missing_snipping)?;
    let mut guard = hotkey_access()?;
    reap_finished_thread(&mut guard)?;
    if let Some(state) = guard.as_ref() {
        return if state.stopping {
            Err("hotkey message thread is still stopping; cannot install".into())
        } else {
            Ok(())
        };
    }
    let (sender, requests) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let worker = thread::Builder::new()
        .name("jasonshell-hotkeys".into())
        .spawn(move || run_hotkey_thread(app_handle, requests, ready_tx))
        .map_err(|error| format!("failed to start hotkey thread: {error}"))?;
    let id = match ready_rx.recv() {
        Ok(Ok(id)) => id,
        result => {
            let _ = worker.join();
            return Err(match result {
                Ok(Err(error)) => error,
                _ => "hotkey message thread failed to start".into(),
            });
        }
    };
    let state = HotkeyThread {
        id,
        sender,
        worker,
        stopping: false,
    };
    if let Err(error) = request_replacement(&state, registry) {
        *guard = Some(state);
        let cleanup = stop_hotkey_thread(&mut guard);
        return Err(match cleanup {
            Ok(()) => error,
            Err(cleanup_error) => format!("{error}; {cleanup_error}"),
        });
    }
    *guard = Some(state);
    Ok(())
}

#[cfg(not(windows))]
pub fn install_windows_key_hook(
    _app_handle: (),
    hotkeys: StandardHotkeySettings,
) -> Result<(), String> {
    StandardHotkeyRegistry::from_settings(&hotkeys).map(|_| ())
}

#[cfg(windows)]
pub fn configure_standard_hotkeys(
    _app_handle: &AppHandle,
    hotkeys: &StandardHotkeySettings,
) -> Result<(), String> {
    let registry = StandardHotkeyRegistry::from_settings(hotkeys)?;
    configure_registry(registry)
}
/// Only trusted transaction rollback invokes the legacy exception; normal updates
/// always build a strict registry before requesting any native registration.
#[cfg(windows)]
pub(crate) fn restore_loaded_standard_hotkeys(_app: &AppHandle, hotkeys: &StandardHotkeySettings) -> Result<(), String> {
    configure_registry(StandardHotkeyRegistry::from_loaded_settings(hotkeys, true)?)
}
#[cfg(windows)]
fn configure_registry(registry: StandardHotkeyRegistry) -> Result<(), String> {
    let mut guard = hotkey_access()?;
    reap_finished_thread(&mut guard)?;
    let thread = guard
        .as_ref()
        .ok_or_else(|| "hotkey message thread is not running".to_string())?;
    if thread.stopping {
        return Err("hotkey message thread is stopping".into());
    }
    request_replacement(thread, registry)
}
#[cfg(not(windows))]
pub(crate) fn restore_loaded_standard_hotkeys(_app: &tauri::AppHandle, hotkeys: &StandardHotkeySettings) -> Result<(), String> {
    StandardHotkeyRegistry::from_loaded_settings(hotkeys, true).map(|_| ())
}

#[cfg(test)]
include!("../tests/fixtures/snipping_registry_acceptance.rs");

#[cfg(test)]
include!("../tests/fixtures/search_hotkey_ownership.rs");

#[cfg(not(windows))]
pub fn configure_standard_hotkeys(
    _app_handle: &tauri::AppHandle,
    hotkeys: &StandardHotkeySettings,
) -> Result<(), String> {
    StandardHotkeyRegistry::from_settings(hotkeys).map(|_| ())
}

#[cfg(windows)]
fn reap_finished_thread(guard: &mut Option<HotkeyThread>) -> Result<(), String> {
    if guard
        .as_ref()
        .is_some_and(|state| state.worker.is_finished())
    {
        let state = guard.take().expect("finished worker was present");
        state
            .worker
            .join()
            .map_err(|_| "hotkey message thread panicked".to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
fn stop_hotkey_thread(guard: &mut Option<HotkeyThread>) -> Result<(), String> {
    reap_finished_thread(guard)?;
    let Some(state) = guard.as_mut() else {
        return Ok(());
    };
    // On post failure or timeout, retain ownership for another attempt. Joining an
    // unnotified GetMessageW thread would hang shutdown indefinitely.
    unsafe {
        PostThreadMessageW(
            state.id,
            WM_STOP_HOTKEYS,
            Default::default(),
            Default::default(),
        )
    }
    .map_err(|error| format!("failed to stop hotkey message thread: {error}"))?;
    state.stopping = true;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !state.worker.is_finished() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if !state.worker.is_finished() {
        return Err(
            "hotkey message thread did not stop within two seconds; retaining handle for retry"
                .into(),
        );
    }
    reap_finished_thread(guard)
}

#[cfg(windows)]
pub fn uninstall_windows_key_hook() {
    match hotkey_access() {
        Ok(mut guard) => {
            if let Err(error) = stop_hotkey_thread(&mut guard) {
                eprintln!("{error}");
            }
        }
        Err(error) => eprintln!("cannot stop hotkey message thread: {error}"),
    }
}

#[cfg(not(windows))]
pub fn uninstall_windows_key_hook() {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::CanonicalHotkeyBinding;

    #[test]
    fn defaults_map_all_five_distinct_actions_and_virtual_keys() {
        let registry =
            StandardHotkeyRegistry::from_settings(&StandardHotkeySettings::default()).unwrap();
        assert_eq!(registry.bindings[0].unwrap().key, space_virtual_key());
        assert_eq!(registry.bindings[1].unwrap().key, backquote_virtual_key());
        assert_eq!(registry.bindings[2].unwrap().key, one_virtual_key());
        assert_eq!(registry.bindings[3].unwrap().key, b'D' as u32);
        assert_eq!(registry.bindings[3].unwrap().action, ConfiguredHotkeyAction::Speech);
        assert_eq!(registry.bindings[4].unwrap().key, b'S' as u32);
        assert_eq!(registry.bindings[4].unwrap().action, ConfiguredHotkeyAction::Snipping);
    }

    #[test]
    fn remaps_ascii_keys_and_rejects_duplicate_chords() {
        let mut settings = StandardHotkeySettings::default();
        settings.search = CanonicalHotkeyBinding("ctrl+k".into());
        assert_eq!(
            StandardHotkeyRegistry::from_settings(&settings)
                .unwrap()
                .bindings[0]
                .unwrap()
                .key,
            b'K' as u32
        );
        settings.speech_transcription = CanonicalHotkeyBinding("CONTROL+K".into());
        assert!(StandardHotkeyRegistry::from_settings(&settings).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn failed_replacement_preserves_verified_restored_bindings_and_reports_missing_one() {
        let old =
            StandardHotkeyRegistry::from_settings(&StandardHotkeySettings::default()).unwrap();
        let mut next_settings = StandardHotkeySettings::default();
        next_settings.search = CanonicalHotkeyBinding("Ctrl+K".into());
        next_settings.terminal = CanonicalHotkeyBinding("Alt+9".into());
        let next = StandardHotkeyRegistry::from_settings(&next_settings).unwrap();
        let mut active = old.bindings;
        let mut attempts = 0;
        let error = replace_registrations(
            &mut active,
            next,
            |_, _| {
                attempts += 1;
                match attempts {
                    2 => Err("new chord occupied".into()),
                    3 => Err("old chord claimed during rollback".into()),
                    _ => Ok(()),
                }
            },
            |_| {},
        )
        .unwrap_err();
        assert!(error.contains("new chord occupied"));
        assert!(error.contains("old chord claimed during rollback"));
        assert_eq!(active[0], None);
        assert_eq!(active[1], old.bindings[1]);
        assert_eq!(active[2], old.bindings[2]);
        assert_eq!(active[3], old.bindings[3]);
        assert_eq!(active[4], old.bindings[4]);
    }

    #[cfg(windows)]
    #[test]
    fn timed_out_stop_followed_by_late_finish_is_reaped_before_reinstall() {
        let (release, wait) = mpsc::sync_channel::<()>(0);
        let (sender, _) = mpsc::channel();
        let worker = thread::spawn(move || {
            let _ = wait.recv();
        });
        let mut state = Some(HotkeyThread {
            id: 0,
            sender,
            worker,
            stopping: true,
        });
        assert!(reap_finished_thread(&mut state).is_ok());
        assert!(state.as_ref().is_some_and(|thread| thread.stopping));
        release.send(()).unwrap();
        // Synchronize without a sleep: join availability is observed by the same
        // predicate used in production before accepting a retained worker.
        while !state.as_ref().unwrap().worker.is_finished() {
            thread::yield_now();
        }
        reap_finished_thread(&mut state).unwrap();
        assert!(state.is_none(), "a new install must now spawn a worker");
    }
}
