use crate::settings::StandardHotkeySettings;

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
    GetMessageW, PeekMessageW, PostThreadMessageW, MSG, PM_NOREMOVE, WM_APP, WM_HOTKEY,
};

pub const SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT: &str =
    crate::contracts::events::SEARCH_TOGGLE_CENTERED;
#[rustfmt::skip]
pub const TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT: &str = crate::contracts::events::TERMINAL_TOGGLE_PANEL;
pub const STACK_BROWSER_HOTKEY_TOGGLE_STACK_BROWSER_EVENT: &str =
    crate::contracts::events::STACK_BROWSER_TOGGLE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfiguredHotkeyAction {
    Search,
    Terminal,
    StackBrowser,
    Speech,
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
type ActiveBindings = [Option<RegistryBinding>; 4];

#[derive(Clone, Debug)]
pub struct StandardHotkeyRegistry {
    bindings: [RegistryBinding; 4],
}

impl StandardHotkeyRegistry {
    fn from_settings(settings: &StandardHotkeySettings) -> Result<Self, String> {
        let settings = crate::settings::validate_standard_hotkey_settings(settings.clone())?;
        Ok(Self {
            bindings: [
                registry_binding(&settings.search.0, ConfiguredHotkeyAction::Search)?,
                registry_binding(&settings.terminal.0, ConfiguredHotkeyAction::Terminal)?,
                registry_binding(
                    &settings.stack_browser.0,
                    ConfiguredHotkeyAction::StackBrowser,
                )?,
                registry_binding(
                    &settings.speech_transcription.0,
                    ConfiguredHotkeyAction::Speech,
                )?,
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
const HOTKEY_IDS: [i32; 4] = [1, 2, 3, 4];
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
            current[index].is_none_or(|old| {
                old.key != next.bindings[index].key || old.modifier != next.bindings[index].modifier
            })
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
        if let Err(error) = register(index, next.bindings[index]) {
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
        current[index] = Some(next.bindings[index]);
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
        PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
    }
    if ready.send(unsafe { GetCurrentThreadId() }).is_err() {
        return;
    }
    let mut current: ActiveBindings = [None; 4];
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        match message.message {
            WM_REPLACE_HOTKEYS => {
                while let Ok(request) = requests.try_recv() {
                    let result = replace_registrations(
                        &mut current,
                        request.registry,
                        register_binding,
                        unregister_binding,
                    );
                    let _ = request.result.send(result);
                }
            }
            WM_HOTKEY => {
                dispatch_hotkey(&app_handle, message.wParam.0 as i32, &current);
            }
            WM_STOP_HOTKEYS => break,
            _ => {}
        }
    }
    for (index, binding) in current.iter().enumerate() {
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
    let registry = StandardHotkeyRegistry::from_settings(&hotkeys)?;
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
    fn defaults_map_all_four_distinct_actions_and_virtual_keys() {
        let registry =
            StandardHotkeyRegistry::from_settings(&StandardHotkeySettings::default()).unwrap();
        assert_eq!(registry.bindings[0].key, space_virtual_key());
        assert_eq!(registry.bindings[1].key, backquote_virtual_key());
        assert_eq!(registry.bindings[2].key, one_virtual_key());
        assert_eq!(registry.bindings[3].key, b'D' as u32);
        assert_eq!(registry.bindings[3].action, ConfiguredHotkeyAction::Speech);
    }

    #[test]
    fn remaps_ascii_keys_and_rejects_duplicate_chords() {
        let mut settings = StandardHotkeySettings::default();
        settings.search = CanonicalHotkeyBinding("ctrl+k".into());
        assert_eq!(
            StandardHotkeyRegistry::from_settings(&settings)
                .unwrap()
                .bindings[0]
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
        let mut active = old.bindings.map(Some);
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
        assert_eq!(active[1], Some(old.bindings[1]));
        assert_eq!(active[2], Some(old.bindings[2]));
        assert_eq!(active[3], Some(old.bindings[3]));
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
