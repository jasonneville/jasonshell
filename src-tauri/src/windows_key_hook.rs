use crate::settings::{StandardHotkeySettings, MissingHotkeys};

#[cfg(windows)]
use std::sync::{mpsc, Mutex, OnceLock};
#[cfg(windows)]
use std::thread::{self, JoinHandle};
#[cfg(windows)]
use tauri::{AppHandle, Emitter};
#[cfg(windows)]
use windows::Win32::System::Threading::GetCurrentThreadId;
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, VK_1,
    VK_OEM_3, VK_SPACE,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, PeekMessageW, PostThreadMessageW, MSG, PM_NOREMOVE, PM_REMOVE, WM_APP, WM_HOTKEY,
    SetTimer, KillTimer, WM_TIMER, GetForegroundWindow, IsWindowVisible,
};

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
    ProcessManager,
    Settings,
    QuickCommands,
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

#[cfg(windows)]
type ActiveBindings = [Option<RegistryBinding>; 8];

#[derive(Clone, Debug)]
pub struct StandardHotkeyRegistry {
    bindings: [Option<RegistryBinding>; 8],
    settings: StandardHotkeySettings,
    inferred: [bool; 8],
}

impl StandardHotkeyRegistry {
    fn from_settings(settings: &StandardHotkeySettings) -> Result<Self, String> {
        Self::from_loaded_settings(settings, false)
    }
    fn from_loaded_settings(settings: &StandardHotkeySettings, missing_snipping: bool) -> Result<Self, String> {
        Self::from_migration(settings, MissingHotkeys { snipping: missing_snipping, ..Default::default() })
    }
    fn from_migration(settings: &StandardHotkeySettings, missing: MissingHotkeys) -> Result<Self, String> {
        let settings = crate::settings::validate_hotkeys_with_missing(settings.clone(), missing)?;
        let inferred = [false, false, false, false, missing.snipping, missing.process_manager, missing.settings, missing.quick_commands];
        let mut bindings = [
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
                Some(registry_binding(&settings.snipping.0, ConfiguredHotkeyAction::Snipping)?),
                Some(registry_binding(&settings.process_manager.0, ConfiguredHotkeyAction::ProcessManager)?),
                Some(registry_binding(&settings.settings.0, ConfiguredHotkeyAction::Settings)?),
                Some(registry_binding(&settings.quick_commands.0, ConfiguredHotkeyAction::QuickCommands)?),
            ];
        for index in 4..bindings.len() {
            if inferred[index] && bindings.iter().enumerate().any(|(other, binding)| other != index && (!inferred[other] || other < index)
                && binding.zip(bindings[index]).is_some_and(|(a,b)| a.modifier == b.modifier && a.key == b.key)) {
                bindings[index] = None;
            }
        }
        Ok(Self { bindings, settings, inferred })
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
const HOTKEY_IDS: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
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
    run: Box<dyn FnOnce(&mut NativeState, &AppHandle) + Send>,
}

#[cfg(windows)]
struct CaptureLease {
    id: String, revision: u64, sequence: u64, deadline: std::time::Instant,
    caller: crate::snipping::coordinator::Caller,
}
#[cfg(windows)]
struct NativeState {
    current: ActiveBindings, registry: StandardHotkeyRegistry, revision: u64,
    capture: Option<CaptureLease>, next_lease: u64, button_revision: u64,
    fence: u32,
    fence_time: std::time::Instant,
    button_owner: Option<crate::snipping::coordinator::Caller>,
}

#[cfg(windows)]
#[derive(Debug, PartialEq)]
enum HotkeyRoute { Discard, Capture(serde_json::Value), Dispatch(i32) }

/// A native observation, never IPC data. The registry comparison is exact
/// instance identity; visibility/focus alone cannot authorize a matching label.
#[cfg(windows)]
fn settings_eligibility(caller: &crate::snipping::coordinator::Caller, registered: Option<&crate::snipping::coordinator::Caller>, visible: bool, focused: bool) -> Result<(), String> {
    if caller.label != crate::contracts::surfaces::SETTINGS_PANEL || registered != Some(caller) { return Err("Unauthorized Settings instance".into()); }
    if !visible || !focused { return Err("Settings must be visible and focused".into()); }
    Ok(())
}

#[cfg(windows)]
impl NativeState {
    fn new(registry: StandardHotkeyRegistry, current: ActiveBindings) -> Self {
        Self { current, registry, revision: 1, capture: None, next_lease: 0, button_revision: 0, fence: 0, fence_time: std::time::Instant::now(), button_owner: None }
    }
    fn status(&self) -> serde_json::Value {
        let hotkeys = &self.registry.settings;
        let names = ["search", "terminal", "stackBrowser", "speechTranscription", "snipping", "processManager", "settings", "quickCommands"];
        let bindings = [&hotkeys.search, &hotkeys.terminal, &hotkeys.stack_browser, &hotkeys.speech_transcription, &hotkeys.snipping, &hotkeys.process_manager, &hotkeys.settings, &hotkeys.quick_commands];
        serde_json::json!({"revision": self.revision, "bindings": names.iter().enumerate().map(|(i, action)| {
            let active = self.current[i].is_some();
            serde_json::json!({"action": action, "id": HOTKEY_IDS[i], "binding": bindings[i].0, "active": active,
                "state": if self.capture.is_some() { "capturePaused" } else if active { "active" } else { "inactive" },
                "reason": if active { None } else { Some("Shortcut unavailable; repair conflicts and Save shortcuts") }})
        }).collect::<Vec<_>>()})
    }
    fn fence(&mut self) {
        let mut queued = MSG::default();
        unsafe { while PeekMessageW(&mut queued, None, WM_HOTKEY, WM_HOTKEY, PM_REMOVE).as_bool() {} }
        self.fence = unsafe { windows::Win32::System::SystemInformation::GetTickCount() };
        self.fence_time = std::time::Instant::now();
    }
    fn retire(&mut self) {
        if self.retire_observed() { self.fence(); }
    }
    fn retire_observed(&mut self) -> bool {
        if self.capture.take().is_some() { self.revision += 1; true } else { false }
    }
    fn expire_observed(&mut self, now: std::time::Instant, eligible: bool) -> bool {
        if self.capture.as_ref().is_some_and(|lease| lease.deadline <= now || !eligible) { return self.retire_observed(); }
        false
    }
    fn expire(&mut self, app: &AppHandle) -> bool {
        let eligible = self.capture.as_ref().is_some_and(|lease| registered_settings_window(app, &lease.caller).is_ok());
        let retired = self.expire_observed(std::time::Instant::now(), eligible);
        if retired { self.fence(); }
        retired
    }
    fn ack(&self) -> serde_json::Value {
        self.ack_at(std::time::Instant::now())
    }
    fn ack_at(&self, now: std::time::Instant) -> serde_json::Value {
        let lease = self.capture.as_ref().expect("ack requires admitted lease");
        let hotkeys = &self.registry.settings;
        let bindings = [&hotkeys.search, &hotkeys.terminal, &hotkeys.stack_browser, &hotkeys.speech_transcription, &hotkeys.snipping, &hotkeys.process_manager, &hotkeys.settings, &hotkeys.quick_commands];
        serde_json::json!({"leaseId": lease.id, "revision": lease.revision, "sequence": lease.sequence,
            "expiresInMs": lease.deadline.saturating_duration_since(now).as_millis() as u64,
            "registeredBindings": bindings.iter().enumerate().filter(|(i,_)| self.current[*i].is_some()).map(|(_, binding)| binding.0.clone()).collect::<Vec<_>>()})
    }
    // Production reducer: injected monotonic time and native eligibility make
    // every lease transition exercisable without a window or OS registration.
    fn capture_transition(&mut self, caller: crate::snipping::coordinator::Caller, operation: &str, revision: u64, lease_id: Option<&str>, now: std::time::Instant, eligible: bool) -> Result<serde_json::Value, String> {
        self.expire_observed(now, true);
        if !eligible {
            // A delayed call from a retired instance must not cancel the new
            // instance's lease. Native focus loss retires only its own lease.
            if self.capture.as_ref().is_some_and(|lease| lease.caller == caller) { self.retire_observed(); }
            return Err("Settings must be visible and focused".into());
        }
        if revision == 0 || revision > 9_007_199_254_740_991 { return Err("Invalid capture revision".into()); }
        if operation == "begin" {
            // A newly registered instance has its own button revision domain;
            // the globally increasing native lease ID still prevents ABA.
            if self.button_owner.as_ref() == Some(&caller) && revision <= self.button_revision { return Err("Stale capture revision".into()); }
            self.retire_observed();
            self.button_revision = revision; self.button_owner = Some(caller.clone());
            self.next_lease = self.next_lease.checked_add(1).ok_or("Capture lease exhausted")?;
            self.capture = Some(CaptureLease { id: self.next_lease.to_string(), revision, sequence: 0,
                deadline: now + std::time::Duration::from_secs(2), caller });
            self.revision += 1; return Ok(self.ack_at(now));
        }
        let lease = self.capture.as_mut().ok_or("Capture lease expired or retired")?;
        if lease.caller != caller || lease.revision != revision || lease_id != Some(lease.id.as_str()) { return Err("Stale capture lease".into()); }
        match operation {
            "renew" => { lease.deadline = now + std::time::Duration::from_secs(2); Ok(self.ack_at(now)) },
            "end" => { self.retire_observed(); Ok(serde_json::Value::Null) },
            _ => Err("Invalid capture operation".into()),
        }
    }
    fn route_hotkey_observed(&mut self, id: i32, message_time: u32, packed: u32, now: std::time::Instant, eligible: bool) -> HotkeyRoute {
        // Timestamp ordering is only unambiguous for half a 32-bit uptime
        // cycle. The queue was drained at the fence, so an older fence can be
        // retired rather than disabling hotkeys after 24.8 days of uptime.
        let fence_current = now.saturating_duration_since(self.fence_time).as_millis() < i32::MAX as u128;
        if self.expire_observed(now, eligible) || (fence_current && (message_time.wrapping_sub(self.fence) as i32) <= 0) { return HotkeyRoute::Discard; }
        let Some(index) = HOTKEY_IDS.iter().position(|candidate| *candidate == id) else { return HotkeyRoute::Discard; };
        let Some(binding) = self.current[index] else { return HotkeyRoute::Discard; };
        let modifier = match binding.modifier { HotkeyModifier::Ctrl => MOD_CONTROL.0, HotkeyModifier::Alt => MOD_ALT.0 };
        if packed >> 16 != binding.key || packed & 0xffff != modifier { return HotkeyRoute::Discard; }
        let Some(lease) = self.capture.as_mut() else { return HotkeyRoute::Dispatch(id); };
        lease.sequence += 1;
        let names = ["search", "terminal", "stackBrowser", "speechTranscription", "snipping", "processManager", "settings", "quickCommands"];
        let hotkeys = &self.registry.settings;
        let bindings = [&hotkeys.search, &hotkeys.terminal, &hotkeys.stack_browser, &hotkeys.speech_transcription, &hotkeys.snipping, &hotkeys.process_manager, &hotkeys.settings, &hotkeys.quick_commands];
        HotkeyRoute::Capture(serde_json::json!({"leaseId": lease.id, "revision": lease.revision, "sequence": lease.sequence, "action": names[index], "binding": bindings[index].0}))
    }
}

#[cfg(windows)]
fn registered_settings_window(app: &AppHandle, caller: &crate::snipping::coordinator::Caller) -> Result<tauri::WebviewWindow, String> {
    if caller.label != crate::contracts::surfaces::SETTINGS_PANEL { return Err("Unauthorized Settings instance".into()); }
    let runtime = app.try_state::<std::sync::Arc<crate::snipping::runtime::SnipRuntime>>().ok_or("Settings registry unavailable")?;
    let entry = runtime.registry.entry(&crate::snipping::coordinator::WindowRef { caller: caller.clone(), monitor_id: None }).map_err(|_| "Unauthorized Settings instance")?;
    let window = entry.window().map_err(|_| "Unauthorized Settings instance")?;
    let hwnd = window.hwnd().map_err(|_| "Settings window unavailable")?;
    let hwnd = windows::Win32::Foundation::HWND(hwnd.0);
    settings_eligibility(caller, Some(&entry.reference.caller), unsafe { IsWindowVisible(hwnd) }.as_bool(), unsafe { GetForegroundWindow() } == hwnd)?;
    Ok(window)
}

#[cfg(windows)]
pub(crate) fn authorize_settings_instance(window: &tauri::WebviewWindow, app: &AppHandle) -> Result<crate::snipping::coordinator::Caller, String> {
    if window.label() != crate::contracts::surfaces::SETTINGS_PANEL { return Err("Unauthorized Settings instance".into()); }
    let runtime = app.try_state::<std::sync::Arc<crate::snipping::runtime::SnipRuntime>>().ok_or("Settings registry unavailable")?;
    let caller = runtime.registry.resolve(window).map_err(|_| "Unauthorized Settings instance")?;
    registered_settings_window(app, &caller)?;
    Ok(caller)
}

#[cfg(windows)]
static HOTKEY_THREAD: OnceLock<Mutex<Option<HotkeyThread>>> = OnceLock::new();
// Separate short-held wake handle: UI lifecycle callbacks never contend with a
// command holding HOTKEY_THREAD while it waits for the owning native thread.
#[cfg(windows)]
static LIFECYCLE_WAKE: OnceLock<Mutex<Option<(u32, mpsc::Sender<ReplaceRequest>)>>> = OnceLock::new();

#[cfg(windows)]
fn hotkey_thread() -> &'static Mutex<Option<HotkeyThread>> {
    HOTKEY_THREAD.get_or_init(|| Mutex::new(None))
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

// Startup alone tolerates OS rejection of inferred additions. Existing and
// explicit owners retain strict registration semantics; no duplicate is tried.
fn install_migrated_registrations(
    current: &mut ActiveBindings, registry: &StandardHotkeyRegistry,
    mut register: impl FnMut(usize, RegistryBinding) -> Result<(), String>,
    mut unregister: impl FnMut(usize),
) -> Result<(), String> {
    for index in 0..HOTKEY_IDS.len() {
        let Some(binding) = registry.bindings[index] else { continue; };
        match register(index, binding) {
            Ok(()) => current[index] = Some(binding),
            Err(_) if registry.inferred[index] => {},
            Err(error) => {
                for (slot, binding) in current.iter_mut().enumerate() {
                    if binding.take().is_some() { unregister(slot); }
                }
                return Err(error);
            }
        }
    }
    Ok(())
}

// Persistence rollback restores verified slots, not inferred configured wishes.
// Failed old slots stay vacant; never roll a failed restore back to the draft.
fn restore_registrations(current: &mut ActiveBindings, previous: &StandardHotkeyRegistry,
    mut register: impl FnMut(usize, RegistryBinding) -> Result<(), String>, mut unregister: impl FnMut(usize),
) -> Result<(), String> {
    let changed: Vec<_> = (0..HOTKEY_IDS.len()).filter(|i| current[*i] != previous.bindings[*i]).collect();
    for &i in &changed { if current[i].take().is_some() { unregister(i); } }
    let mut errors = Vec::new();
    for i in changed {
        if let Some(binding) = previous.bindings[i] {
            match register(i, binding) { Ok(()) => current[i] = Some(binding), Err(error) => errors.push(error) }
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(format!("previous hotkeys only partially restored: {}", errors.join("; "))) }
}

#[cfg(windows)]
fn native_request<T: Send + 'static>(run: impl FnOnce(&mut NativeState, &AppHandle) -> Result<T, String> + Send + 'static) -> Result<T, String> {
    let mut guard = hotkey_thread().lock().map_err(|_| "hotkey thread lock is poisoned")?;
    reap_finished_thread(&mut guard)?;
    let thread = guard.as_ref().filter(|thread| !thread.stopping).ok_or("hotkey message thread is not running")?;
    request_on_thread(thread, run)
}

#[cfg(windows)]
pub(crate) fn standard_hotkey_status(caller: crate::snipping::coordinator::Caller) -> Result<serde_json::Value, String> {
    native_request(move |state, app| { registered_settings_window(app, &caller)?; state.expire(app); Ok(state.status()) })
}

/// Trusted post-commit response only, never a renderer command. Save already
/// authorized its exact caller before mutation; losing focus after disk commit
/// cannot revoke that successful result. Public status reads remain authorized.
#[cfg(windows)]
pub(crate) fn committed_standard_hotkey_status() -> Result<serde_json::Value, String> {
    native_request(|state, app| { state.expire(app); Ok(state.status()) })
}

#[cfg(windows)]
pub(crate) fn register_standard_hotkeys(caller: crate::snipping::coordinator::Caller, hotkeys: StandardHotkeySettings, revision: u64) -> Result<StandardHotkeyRegistry, String> {
    let next = StandardHotkeyRegistry::from_settings(&hotkeys)?;
    native_request(move |state, app| {
        registered_settings_window(app, &caller)?; state.expire(app);
        if state.revision != revision { return Err("Shortcut status changed; refresh and retry Save".into()); }
        if state.capture.is_some() { return Err("End shortcut capture before saving".into()); }
        let mut previous = state.registry.clone(); previous.bindings = state.current;
        state.fence();
        let result = replace_registrations(&mut state.current, next.clone(), register_binding, unregister_binding);
        state.revision += 1;
        if result.is_ok() { state.registry = next; }
        result.map(|_| previous)
    })
}

#[cfg(windows)]
pub(crate) fn restore_standard_hotkey_snapshot(previous: StandardHotkeyRegistry) -> Result<(), String> {
    native_request(move |state, _| {
        state.retire(); state.fence();
        let result = restore_registrations(&mut state.current, &previous, register_binding, unregister_binding);
        state.registry = previous; state.revision += 1; result
    })
}

#[cfg(windows)]
pub(crate) fn capture_command(caller: crate::snipping::coordinator::Caller, operation: &'static str, revision: u64, lease_id: Option<String>) -> Result<serde_json::Value, String> {
    native_request(move |state, app| {
        state.expire(app);
        let eligible = registered_settings_window(app, &caller).is_ok();
        let previous_revision = state.revision;
        let result = state.capture_transition(caller, operation, revision, lease_id.as_deref(), std::time::Instant::now(), eligible);
        if state.revision != previous_revision { state.fence(); }
        result
    })
}

/// Lifecycle callbacks only enqueue retirement: never wait on the UI thread.
#[cfg(windows)]
pub(crate) fn retire_settings_capture() {
    let wake = LIFECYCLE_WAKE.get().and_then(|wake| wake.lock().ok()?.clone());
    if let Some((id, sender)) = wake {
        let _ = sender.send(ReplaceRequest { run: Box::new(|state, _| state.retire()) });
        let _ = unsafe { PostThreadMessageW(id, WM_REPLACE_HOTKEYS, Default::default(), Default::default()) };
    }
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
            let _ = app_handle.emit_to(
                crate::shell_windows::TOP_BAR_LABEL,
                crate::contracts::events::SEARCH_TOGGLE_CENTERED,
                (),
            );
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
        ConfiguredHotkeyAction::ProcessManager => {
            let _ = app_handle.emit_to(tauri::EventTarget::webview_window(crate::contracts::surfaces::BOTTOM_BAR), crate::contracts::events::PROCESS_MANAGER_OPEN_REQUESTED, ());
        }
        ConfiguredHotkeyAction::Settings => {
            let _ = app_handle.emit_to(tauri::EventTarget::webview_window(crate::contracts::surfaces::TOP_BAR), crate::contracts::events::SETTINGS_OPEN_REQUESTED, ());
        }
        ConfiguredHotkeyAction::QuickCommands => {
            let _ = app_handle.emit_to(tauri::EventTarget::webview_window(crate::contracts::surfaces::TOP_BAR), crate::contracts::events::QUICK_COMMANDS_TOGGLE, ());
        }
    }
}

#[cfg(windows)]
fn run_hotkey_thread(
    app_handle: AppHandle,
    requests: mpsc::Receiver<ReplaceRequest>,
    ready: mpsc::SyncSender<u32>,
) {
    let mut message = MSG::default();
    // Force creation of this thread's message queue before publishing its ID.
    unsafe {
        let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
    }
    if ready.send(unsafe { GetCurrentThreadId() }).is_err() {
        return;
    }
    let mut state = NativeState::new(StandardHotkeyRegistry::from_settings(&StandardHotkeySettings::default()).expect("valid defaults"), [None; 8]);
    // Thread-owned timer wakes even when every renderer stalls or disappears.
    let timer = unsafe { SetTimer(None, 0, 100, None) };
    if timer == 0 { return; }
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        let expired = state.expire(&app_handle);
        match message.message {
            WM_REPLACE_HOTKEYS | WM_TIMER => {
                while let Ok(request) = requests.try_recv() {
                    (request.run)(&mut state, &app_handle);
                }
            }
            WM_HOTKEY => {
                // A queued event predating capture/retirement/remap is consumed,
                // never replayed as a later action. Wrapping subtraction handles
                // Windows' 32-bit uptime rollover.
                if expired { continue; }
                let window = state.capture.as_ref().and_then(|lease| registered_settings_window(&app_handle, &lease.caller).ok());
                let previous_revision = state.revision;
                let route = state.route_hotkey_observed(message.wParam.0 as i32, message.time, message.lParam.0 as u32, std::time::Instant::now(), window.is_some());
                if state.revision != previous_revision { state.fence(); }
                match route {
                    HotkeyRoute::Discard => {},
                    HotkeyRoute::Capture(payload) => {
                        if let Some(window) = window {
                            if window.emit_to(tauri::EventTarget::webview_window(window.label()), crate::contracts::events::STANDARD_HOTKEYS_CAPTURE, payload).is_err() { state.retire(); }
                        } else { state.retire(); }
                    }
                    HotkeyRoute::Dispatch(id) => dispatch_hotkey(&app_handle, id, &state.current),
                }
            }
            WM_STOP_HOTKEYS => break,
            _ => {}
        }
    }
    state.retire();
    let _ = unsafe { KillTimer(None, timer) };
    for (index, binding) in state.current.iter().enumerate() {
        if binding.is_some() {
            unregister_binding(index);
        }
    }
}

#[cfg(windows)]
fn request_replacement(
    thread: &HotkeyThread,
    registry: StandardHotkeyRegistry,
) -> Result<(), String> {
    request_on_thread(thread, move |state, _| {
        state.retire(); state.fence();
        let result = replace_registrations(&mut state.current, registry.clone(), register_binding, unregister_binding);
        if result.is_ok() { state.registry = registry; }
        state.revision += 1;
        result
    })
}

#[cfg(windows)]
fn request_on_thread<T: Send + 'static>(thread: &HotkeyThread, run: impl FnOnce(&mut NativeState, &AppHandle) -> Result<T, String> + Send + 'static) -> Result<T, String> {
    let (result_tx, result_rx) = mpsc::sync_channel(1);
    let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let request_cancelled = cancelled.clone();
    thread
        .sender
        .send(ReplaceRequest {
            run: Box::new(move |state, app| {
                if request_cancelled.load(std::sync::atomic::Ordering::Acquire) { return; }
                if result_tx.send(run(state, app)).is_err() { state.retire(); }
            }),
        })
        .map_err(|_| "hotkey message thread stopped".to_string())?;
    let notified = unsafe {
        PostThreadMessageW(
            thread.id,
            WM_REPLACE_HOTKEYS,
            Default::default(),
            Default::default(),
        )
    };
    if let Err(error) = notified {
        cancelled.store(true, std::sync::atomic::Ordering::Release);
        // The independent timer also drains requests if notification fails.
        let _ = thread.sender.send(ReplaceRequest { run: Box::new(|state, _| state.retire()) });
        return Err(format!("failed to notify hotkey message thread: {error}"));
    }
    result_rx
        .recv()
        .map_err(|_| "hotkey message thread stopped".to_string())?
}

#[cfg(windows)]
pub fn install_windows_key_hook(
    app_handle: AppHandle,
    hotkeys: StandardHotkeySettings,
) -> Result<(), String> {
    install_loaded_windows_key_hook(app_handle, hotkeys, MissingHotkeys::default())
}
#[cfg(windows)]
pub(crate) fn install_loaded_windows_key_hook(app_handle: AppHandle, hotkeys: StandardHotkeySettings, missing: MissingHotkeys) -> Result<(), String> {
    let registry = StandardHotkeyRegistry::from_migration(&hotkeys, missing)?;
    let mut guard = hotkey_thread()
        .lock()
        .map_err(|_| "hotkey thread lock is poisoned".to_string())?;
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
    let id = ready_rx
        .recv()
        .map_err(|_| "hotkey message thread failed to start".to_string())?;
    let state = HotkeyThread {
        id,
        sender,
        worker,
        stopping: false,
    };
    if let Err(error) = request_on_thread(&state, move |state, _| {
        let result = install_migrated_registrations(&mut state.current, &registry, register_binding, unregister_binding);
        state.registry = registry; state.fence(); state.revision += 1;
        result
    }) {
        *guard = Some(state);
        let cleanup = stop_hotkey_thread(&mut guard);
        return Err(match cleanup {
            Ok(()) => error,
            Err(cleanup_error) => format!("{error}; {cleanup_error}"),
        });
    }
    *guard = Some(state);
    if let Some(state) = guard.as_ref() {
        *LIFECYCLE_WAKE.get_or_init(|| Mutex::new(None)).lock().map_err(|_| "hotkey lifecycle lock is poisoned")? = Some((state.id, state.sender.clone()));
    }
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
    let mut guard = hotkey_thread()
        .lock()
        .map_err(|_| "hotkey thread lock is poisoned".to_string())?;
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
include!("../tests/fixtures/seven_hotkey_registry_acceptance.rs");

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
    if let Some(wake) = LIFECYCLE_WAKE.get() { if let Ok(mut wake) = wake.lock() { *wake = None; } }
    match hotkey_thread().lock() {
        Ok(mut guard) => {
            if let Err(error) = stop_hotkey_thread(&mut guard) {
                eprintln!("{error}");
            }
        }
        Err(_) => eprintln!("hotkey thread lock is poisoned; cannot stop hotkey message thread"),
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
