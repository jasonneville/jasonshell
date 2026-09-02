use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(windows)]
use std::sync::{mpsc, Arc, Mutex, OnceLock};
#[cfg(windows)]
use tauri::{AppHandle, Emitter};
#[cfg(windows)]
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_1, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_MENU, VK_OEM_3, VK_RCONTROL,
    VK_RMENU, VK_SPACE,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

pub const SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT: &str = "search:toggle-centered";
pub const TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT: &str = "terminal:toggle-panel";
pub const STACK_BROWSER_HOTKEY_TOGGLE_STACK_BROWSER_EVENT: &str = "stack-browser:toggle";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchHotkeyCode {
    LeftControl,
    RightControl,
    Space,
    Backquote,
    LeftAlt,
    RightAlt,
    Other(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchHotkeyEventKind {
    KeyDown,
    KeyUp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchHotkeyEvent {
    pub key: SearchHotkeyCode,
    pub kind: SearchHotkeyEventKind,
    pub repeat: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchHotkeyDecision {
    ToggleSearch,
    ToggleTerminal,
    ToggleStackBrowser,
    Suppress,
    PassThrough,
}

pub fn unavailable_hook_state_decision(_event: SearchHotkeyEvent) -> SearchHotkeyDecision {
    SearchHotkeyDecision::PassThrough
}

#[derive(Default)]
pub struct SearchHotkeyClassifier {
    left_control_down: bool,
    right_control_down: bool,
    emitted_for_chord: bool,
    space_down: bool,
    left_alt_down: bool,
    right_alt_down: bool,
    backquote_down: bool,
    one_down: bool,
}

impl SearchHotkeyClassifier {
    fn any_control_down(&self) -> bool {
        self.left_control_down || self.right_control_down
    }

    fn set_control_down(&mut self, key: SearchHotkeyCode, is_down: bool) {
        match key {
            SearchHotkeyCode::LeftControl => self.left_control_down = is_down,
            SearchHotkeyCode::RightControl => self.right_control_down = is_down,
            SearchHotkeyCode::Space
            | SearchHotkeyCode::Backquote
            | SearchHotkeyCode::LeftAlt
            | SearchHotkeyCode::RightAlt
            | SearchHotkeyCode::Other(_) => {}
        }
    }

    fn any_alt_down(&self) -> bool {
        self.left_alt_down || self.right_alt_down
    }

    fn set_alt_down(&mut self, key: SearchHotkeyCode, is_down: bool) {
        match key {
            SearchHotkeyCode::LeftAlt => self.left_alt_down = is_down,
            SearchHotkeyCode::RightAlt => self.right_alt_down = is_down,
            SearchHotkeyCode::LeftControl
            | SearchHotkeyCode::RightControl
            | SearchHotkeyCode::Space
            | SearchHotkeyCode::Backquote
            | SearchHotkeyCode::Other(_) => {}
        }
    }

    #[cfg(test)]
    pub fn handle_event(&mut self, event: SearchHotkeyEvent) -> SearchHotkeyDecision {
        self.handle_event_with_control_override(event, None)
    }

    pub fn handle_event_with_control_override(
        &mut self,
        event: SearchHotkeyEvent,
        control_down_override: Option<bool>,
    ) -> SearchHotkeyDecision {
        match (event.key, event.kind) {
            (
                SearchHotkeyCode::LeftControl | SearchHotkeyCode::RightControl,
                SearchHotkeyEventKind::KeyDown,
            ) => {
                self.set_control_down(event.key, true);
                SearchHotkeyDecision::PassThrough
            }
            (
                SearchHotkeyCode::LeftControl | SearchHotkeyCode::RightControl,
                SearchHotkeyEventKind::KeyUp,
            ) => {
                self.set_control_down(event.key, false);
                if !self.any_control_down() {
                    self.emitted_for_chord = false;
                }
                SearchHotkeyDecision::PassThrough
            }
            (
                SearchHotkeyCode::LeftAlt | SearchHotkeyCode::RightAlt,
                SearchHotkeyEventKind::KeyDown,
            ) => {
                self.set_alt_down(event.key, true);
                SearchHotkeyDecision::PassThrough
            }
            (
                SearchHotkeyCode::LeftAlt | SearchHotkeyCode::RightAlt,
                SearchHotkeyEventKind::KeyUp,
            ) => {
                self.set_alt_down(event.key, false);
                if !self.any_alt_down() {
                    self.backquote_down = false;
                }
                SearchHotkeyDecision::PassThrough
            }
            (SearchHotkeyCode::Backquote, SearchHotkeyEventKind::KeyDown) => {
                let repeated = event.repeat || self.backquote_down;
                self.backquote_down = true;
                if self.any_alt_down() {
                    if !repeated {
                        SearchHotkeyDecision::ToggleTerminal
                    } else {
                        SearchHotkeyDecision::Suppress
                    }
                } else {
                    SearchHotkeyDecision::PassThrough
                }
            }
            (SearchHotkeyCode::Backquote, SearchHotkeyEventKind::KeyUp) => {
                self.backquote_down = false;
                if self.any_alt_down() {
                    SearchHotkeyDecision::Suppress
                } else {
                    SearchHotkeyDecision::PassThrough
                }
            }
            (SearchHotkeyCode::Other(code), SearchHotkeyEventKind::KeyDown)
                if code == VK_1.0 as u32 =>
            {
                let repeated = event.repeat || self.one_down;
                self.one_down = true;
                if self.any_alt_down() && !self.any_control_down() {
                    if !repeated {
                        SearchHotkeyDecision::ToggleStackBrowser
                    } else {
                        SearchHotkeyDecision::Suppress
                    }
                } else {
                    SearchHotkeyDecision::PassThrough
                }
            }
            (SearchHotkeyCode::Other(code), SearchHotkeyEventKind::KeyUp)
                if code == VK_1.0 as u32 =>
            {
                self.one_down = false;
                if self.any_alt_down() && !self.any_control_down() {
                    SearchHotkeyDecision::Suppress
                } else {
                    SearchHotkeyDecision::PassThrough
                }
            }
            (SearchHotkeyCode::Space, SearchHotkeyEventKind::KeyDown) => {
                self.space_down = true;
                let control_down = control_down_override.unwrap_or_else(|| self.any_control_down());
                if control_down {
                    if !self.emitted_for_chord && !event.repeat {
                        self.emitted_for_chord = true;
                        SearchHotkeyDecision::ToggleSearch
                    } else {
                        SearchHotkeyDecision::Suppress
                    }
                } else {
                    self.emitted_for_chord = false;
                    SearchHotkeyDecision::PassThrough
                }
            }
            (SearchHotkeyCode::Space, SearchHotkeyEventKind::KeyUp) => {
                self.space_down = false;
                if self.emitted_for_chord {
                    if !self.any_control_down() {
                        self.emitted_for_chord = false;
                    }
                    SearchHotkeyDecision::Suppress
                } else {
                    SearchHotkeyDecision::PassThrough
                }
            }
            (SearchHotkeyCode::Other(_), SearchHotkeyEventKind::KeyDown) => {
                if !self.space_down {
                    self.emitted_for_chord = false;
                }
                if !self.backquote_down {
                    self.backquote_down = false;
                }
                SearchHotkeyDecision::PassThrough
            }
            (SearchHotkeyCode::Other(_), SearchHotkeyEventKind::KeyUp) => {
                SearchHotkeyDecision::PassThrough
            }
        }
    }
}

#[derive(Default)]
#[cfg(test)]
pub struct SearchHotkeyHookLifecycle {
    installed: AtomicBool,
}

#[cfg(test)]
impl SearchHotkeyHookLifecycle {
    pub fn install_once(&self) -> bool {
        self.installed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn uninstall(&self) -> bool {
        self.installed
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn is_installed(&self) -> bool {
        self.installed.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
pub fn toggle_search_event_target_label() -> &'static str {
    crate::shell_windows::TOP_BAR_LABEL
}

#[cfg(windows)]
struct NativeHookState {
    classifier: SearchHotkeyClassifier,
    hook: isize,
    action_tx: mpsc::SyncSender<SearchHotkeyDecision>,
    worker_stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

#[cfg(windows)]
static NATIVE_HOOK_STATE: OnceLock<Mutex<Option<NativeHookState>>> = OnceLock::new();

#[cfg(windows)]
fn native_hook_state() -> &'static Mutex<Option<NativeHookState>> {
    NATIVE_HOOK_STATE.get_or_init(|| Mutex::new(None))
}

#[cfg(windows)]
pub fn install_windows_key_hook(app_handle: AppHandle) -> Result<(), String> {
    let mut guard = native_hook_state()
        .lock()
        .map_err(|_| "search hotkey hook state is poisoned".to_string())?;
    if guard.is_some() {
        return Ok(());
    }

    let (action_tx, action_rx) = mpsc::sync_channel(8);
    let worker_stop = Arc::new(AtomicBool::new(false));
    let worker_stop_signal = Arc::clone(&worker_stop);
    let worker_app_handle = app_handle.clone();
    let worker = std::thread::spawn(move || {
        while !worker_stop_signal.load(Ordering::Relaxed) {
            match action_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                Ok(SearchHotkeyDecision::ToggleSearch) => {
                    let _ = worker_app_handle.emit_to(
                        crate::shell_windows::TOP_BAR_LABEL,
                        SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT,
                        (),
                    );
                }
                Ok(SearchHotkeyDecision::ToggleTerminal) => {
                    let _ = worker_app_handle.emit_to(
                        crate::shell_windows::TOP_BAR_LABEL,
                        TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT,
                        (),
                    );
                }
                Ok(SearchHotkeyDecision::ToggleStackBrowser) => {
                    let _ = worker_app_handle.emit_to(
                        crate::shell_windows::TOP_BAR_LABEL,
                        STACK_BROWSER_HOTKEY_TOGGLE_STACK_BROWSER_EVENT,
                        (),
                    );
                }
                Ok(SearchHotkeyDecision::PassThrough | SearchHotkeyDecision::Suppress) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        while let Ok(action) = action_rx.try_recv() {
            match action {
                SearchHotkeyDecision::ToggleSearch => {
                    let _ = worker_app_handle.emit_to(
                        crate::shell_windows::TOP_BAR_LABEL,
                        SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT,
                        (),
                    );
                }
                SearchHotkeyDecision::ToggleTerminal => {
                    let _ = worker_app_handle.emit_to(
                        crate::shell_windows::TOP_BAR_LABEL,
                        TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT,
                        (),
                    );
                }
                SearchHotkeyDecision::ToggleStackBrowser => {
                    let _ = worker_app_handle.emit_to(
                        crate::shell_windows::TOP_BAR_LABEL,
                        STACK_BROWSER_HOTKEY_TOGGLE_STACK_BROWSER_EVENT,
                        (),
                    );
                }
                SearchHotkeyDecision::PassThrough | SearchHotkeyDecision::Suppress => {}
            }
        }
    });

    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(windows_key_hook_proc), None, 0) }
        .map_err(|error| format!("failed to install search hotkey hook: {error}"))?;
    *guard = Some(NativeHookState {
        classifier: SearchHotkeyClassifier::default(),
        hook: hook.0 as isize,
        action_tx,
        worker_stop,
        worker: Some(worker),
    });
    Ok(())
}

#[cfg(not(windows))]
pub fn install_windows_key_hook(_app_handle: ()) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub fn uninstall_windows_key_hook() {
    let Ok(mut guard) = native_hook_state().lock() else {
        return;
    };
    let Some(state) = guard.take() else {
        return;
    };
    state.worker_stop.store(true, Ordering::Relaxed);
    let _ = unsafe { UnhookWindowsHookEx(HHOOK(state.hook as *mut _)) };
    drop(state.action_tx);
    if let Some(worker) = state.worker {
        let _ = worker.join();
    }
}

#[cfg(not(windows))]
pub fn uninstall_windows_key_hook() {}

#[cfg(windows)]
unsafe extern "system" fn windows_key_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code == HC_ACTION as i32 {
        let event = keyboard_hook_event(wparam, lparam);
        if let Some(event) = event {
            let decision = if let Ok(mut guard) = native_hook_state().lock() {
                if let Some(state) = guard.as_mut() {
                    let decision = state
                        .classifier
                        .handle_event_with_control_override(event, control_key_is_down());
                    if matches!(
                        decision,
                        SearchHotkeyDecision::ToggleSearch
                            | SearchHotkeyDecision::ToggleTerminal
                            | SearchHotkeyDecision::ToggleStackBrowser
                    ) {
                        let _ = state.action_tx.try_send(decision);
                    }
                    decision
                } else {
                    unavailable_hook_state_decision(event)
                }
            } else {
                unavailable_hook_state_decision(event)
            };

            match decision {
                SearchHotkeyDecision::ToggleSearch => {
                    return LRESULT(1);
                }
                SearchHotkeyDecision::ToggleTerminal => {
                    return LRESULT(1);
                }
                SearchHotkeyDecision::ToggleStackBrowser => return LRESULT(1),
                SearchHotkeyDecision::Suppress => return LRESULT(1),
                SearchHotkeyDecision::PassThrough => {}
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(windows)]
fn control_key_is_down() -> Option<bool> {
    Some(
        unsafe { GetAsyncKeyState(VK_CONTROL.0.into()) } < 0
            || unsafe { GetAsyncKeyState(VK_LCONTROL.0.into()) } < 0
            || unsafe { GetAsyncKeyState(VK_RCONTROL.0.into()) } < 0,
    )
}

#[cfg(windows)]
fn keyboard_hook_event(wparam: WPARAM, lparam: LPARAM) -> Option<SearchHotkeyEvent> {
    let kind = match wparam.0 as u32 {
        WM_KEYDOWN | WM_SYSKEYDOWN => SearchHotkeyEventKind::KeyDown,
        WM_KEYUP | WM_SYSKEYUP => SearchHotkeyEventKind::KeyUp,
        _ => return None,
    };
    let info = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let key = if info.vkCode == VK_LCONTROL.0 as u32 || info.vkCode == VK_CONTROL.0 as u32 {
        SearchHotkeyCode::LeftControl
    } else if info.vkCode == VK_RCONTROL.0 as u32 {
        SearchHotkeyCode::RightControl
    } else if info.vkCode == VK_SPACE.0 as u32 {
        SearchHotkeyCode::Space
    } else if info.vkCode == VK_OEM_3.0 as u32 {
        SearchHotkeyCode::Backquote
    } else if info.vkCode == VK_MENU.0 as u32 || info.vkCode == VK_LMENU.0 as u32 {
        SearchHotkeyCode::LeftAlt
    } else if info.vkCode == VK_RMENU.0 as u32 {
        SearchHotkeyCode::RightAlt
    } else {
        SearchHotkeyCode::Other(info.vkCode)
    };
    Some(SearchHotkeyEvent {
        key,
        kind,
        repeat: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down(key: SearchHotkeyCode) -> SearchHotkeyEvent {
        SearchHotkeyEvent {
            key,
            kind: SearchHotkeyEventKind::KeyDown,
            repeat: false,
        }
    }

    fn up(key: SearchHotkeyCode) -> SearchHotkeyEvent {
        SearchHotkeyEvent {
            key,
            kind: SearchHotkeyEventKind::KeyUp,
            repeat: false,
        }
    }

    #[test]
    fn ctrl_space_toggles_search_and_suppresses_space() {
        let mut classifier = SearchHotkeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::LeftControl)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::ToggleSearch
        );
        assert_eq!(
            classifier.handle_event(up(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(SearchHotkeyCode::LeftControl)),
            SearchHotkeyDecision::PassThrough
        );
    }

    #[test]
    fn right_ctrl_space_toggles_search() {
        let mut classifier = SearchHotkeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::RightControl)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::ToggleSearch
        );
    }

    #[test]
    fn alt_backquote_toggles_terminal_and_suppresses_backquote() {
        let mut classifier = SearchHotkeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::LeftAlt)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Backquote)),
            SearchHotkeyDecision::ToggleTerminal
        );
        assert_eq!(
            classifier.handle_event(up(SearchHotkeyCode::Backquote)),
            SearchHotkeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(SearchHotkeyCode::LeftAlt)),
            SearchHotkeyDecision::PassThrough
        );
    }

    #[test]
    fn repeated_alt_backquote_does_not_duplicate_terminal_toggle() {
        let mut classifier = SearchHotkeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::LeftAlt)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Backquote)),
            SearchHotkeyDecision::ToggleTerminal
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Backquote)),
            SearchHotkeyDecision::Suppress
        );
    }

    #[test]
    fn alt_1_toggles_stack_browser_and_suppresses_repeat() {
        let mut classifier = SearchHotkeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::LeftAlt)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Other(VK_1.0 as u32))),
            SearchHotkeyDecision::ToggleStackBrowser
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Other(VK_1.0 as u32))),
            SearchHotkeyDecision::Suppress
        );

        let mut classifier = SearchHotkeyClassifier::default();
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Other(VK_1.0 as u32))),
            SearchHotkeyDecision::PassThrough
        );
    }

    #[test]
    fn ctrl_alt_1_passes_through() {
        let mut classifier = SearchHotkeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::LeftControl)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::RightAlt)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Other(VK_1.0 as u32))),
            SearchHotkeyDecision::PassThrough
        );
    }

    #[test]
    fn bare_space_and_other_ctrl_chords_pass_through() {
        let mut classifier = SearchHotkeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(up(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::LeftControl)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Other(u32::from(b'R')))),
            SearchHotkeyDecision::PassThrough
        );
    }

    #[test]
    fn repeated_space_down_does_not_duplicate_open_search() {
        let mut classifier = SearchHotkeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::LeftControl)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::ToggleSearch
        );
        assert_eq!(
            classifier.handle_event(SearchHotkeyEvent {
                key: SearchHotkeyCode::Space,
                kind: SearchHotkeyEventKind::KeyDown,
                repeat: true
            }),
            SearchHotkeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::Suppress
        );
    }

    #[test]
    fn ctrl_release_resets_chord() {
        let mut classifier = SearchHotkeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::LeftControl)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::ToggleSearch
        );
        assert_eq!(
            classifier.handle_event(up(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(SearchHotkeyCode::LeftControl)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::LeftControl)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::ToggleSearch
        );
    }

    #[test]
    fn async_control_state_opens_when_control_down_was_not_observed() {
        let mut classifier = SearchHotkeyClassifier::default();

        assert_eq!(
            classifier
                .handle_event_with_control_override(down(SearchHotkeyCode::Space), Some(true)),
            SearchHotkeyDecision::ToggleSearch
        );
        assert_eq!(
            classifier.handle_event(up(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::Suppress
        );
    }

    #[test]
    fn released_control_state_passes_through_stale_classifier_control() {
        let mut classifier = SearchHotkeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(SearchHotkeyCode::LeftControl)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            classifier
                .handle_event_with_control_override(down(SearchHotkeyCode::Space), Some(false)),
            SearchHotkeyDecision::PassThrough
        );
    }

    #[test]
    fn unavailable_hook_state_passes_through() {
        assert_eq!(
            unavailable_hook_state_decision(down(SearchHotkeyCode::LeftControl)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            unavailable_hook_state_decision(down(SearchHotkeyCode::Space)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            unavailable_hook_state_decision(down(SearchHotkeyCode::Backquote)),
            SearchHotkeyDecision::PassThrough
        );
        assert_eq!(
            unavailable_hook_state_decision(down(SearchHotkeyCode::Other(u32::from(b'R')))),
            SearchHotkeyDecision::PassThrough
        );
    }

    #[test]
    fn lifecycle_install_once_and_uninstall_are_idempotent() {
        let lifecycle = SearchHotkeyHookLifecycle::default();

        assert!(lifecycle.install_once());
        assert!(!lifecycle.install_once());
        assert!(lifecycle.is_installed());
        assert!(lifecycle.uninstall());
        assert!(!lifecycle.uninstall());
        assert!(!lifecycle.is_installed());
    }

    #[test]
    fn emitted_event_targets_top_bar_existing_open_path() {
        assert_eq!(SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT, "search:toggle-centered");
        assert_eq!(
            TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT,
            "terminal:toggle-panel"
        );
        assert_eq!(
            toggle_search_event_target_label(),
            crate::shell_windows::TOP_BAR_LABEL
        );
    }
}
