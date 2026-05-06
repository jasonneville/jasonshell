#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(windows)]
use std::sync::{Mutex, OnceLock};
#[cfg(windows)]
use tauri::{AppHandle, Emitter};
#[cfg(windows)]
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_LWIN, VK_RWIN};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

pub const WINDOWS_KEY_OPEN_SEARCH_EVENT: &str = "search:open-centered";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowsKeyCode {
    LeftWin,
    RightWin,
    Other(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowsKeyEventKind {
    KeyDown,
    KeyUp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowsKeyEvent {
    pub key: WindowsKeyCode,
    pub kind: WindowsKeyEventKind,
    pub repeat: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowsKeyDecision {
    OpenSearch,
    Suppress,
    PassThrough,
}

fn is_windows_key(key: WindowsKeyCode) -> bool {
    matches!(key, WindowsKeyCode::LeftWin | WindowsKeyCode::RightWin)
}

pub fn unavailable_hook_state_decision(event: WindowsKeyEvent) -> WindowsKeyDecision {
    if is_windows_key(event.key) {
        WindowsKeyDecision::Suppress
    } else {
        WindowsKeyDecision::PassThrough
    }
}

#[derive(Default)]
pub struct WindowsKeyClassifier {
    left_win_down: bool,
    right_win_down: bool,
    chorded: bool,
    emitted_for_tap: bool,
}

impl WindowsKeyClassifier {
    fn any_windows_key_down(&self) -> bool {
        self.left_win_down || self.right_win_down
    }

    fn set_windows_key_down(&mut self, key: WindowsKeyCode, is_down: bool) {
        match key {
            WindowsKeyCode::LeftWin => self.left_win_down = is_down,
            WindowsKeyCode::RightWin => self.right_win_down = is_down,
            WindowsKeyCode::Other(_) => {}
        }
    }

    pub fn handle_event(&mut self, event: WindowsKeyEvent) -> WindowsKeyDecision {
        match (event.key, event.kind) {
            (WindowsKeyCode::LeftWin | WindowsKeyCode::RightWin, WindowsKeyEventKind::KeyDown) => {
                if !self.any_windows_key_down() {
                    self.chorded = false;
                    self.emitted_for_tap = false;
                }
                self.set_windows_key_down(event.key, true);
                WindowsKeyDecision::Suppress
            }
            (WindowsKeyCode::LeftWin | WindowsKeyCode::RightWin, WindowsKeyEventKind::KeyUp) => {
                let had_windows_key_down = self.any_windows_key_down();
                self.set_windows_key_down(event.key, false);
                if !had_windows_key_down {
                    self.emitted_for_tap = false;
                    self.chorded = false;
                    return WindowsKeyDecision::Suppress;
                }
                if self.any_windows_key_down() {
                    return WindowsKeyDecision::Suppress;
                }

                let should_open = !self.chorded && !self.emitted_for_tap;
                self.chorded = false;
                if should_open {
                    self.emitted_for_tap = true;
                    WindowsKeyDecision::OpenSearch
                } else {
                    self.emitted_for_tap = false;
                    WindowsKeyDecision::Suppress
                }
            }
            (WindowsKeyCode::Other(_), _) => {
                if self.any_windows_key_down() {
                    self.chorded = true;
                }
                WindowsKeyDecision::PassThrough
            }
        }
    }
}

#[derive(Default)]
#[cfg(test)]
pub struct WindowsKeyHookLifecycle {
    installed: AtomicBool,
}

#[cfg(test)]
impl WindowsKeyHookLifecycle {
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
pub fn open_search_event_target_label() -> &'static str {
    crate::shell_windows::TOP_BAR_LABEL
}

#[cfg(windows)]
struct NativeHookState {
    app_handle: AppHandle,
    classifier: WindowsKeyClassifier,
    hook: isize,
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
        .map_err(|_| "Windows-key hook state is poisoned".to_string())?;
    if guard.is_some() {
        return Ok(());
    }

    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(windows_key_hook_proc), None, 0) }
        .map_err(|error| format!("Failed to install Windows-key hook: {error}"))?;
    *guard = Some(NativeHookState {
        app_handle,
        classifier: WindowsKeyClassifier::default(),
        hook: hook.0 as isize,
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
    let _ = unsafe { UnhookWindowsHookEx(HHOOK(state.hook as *mut _)) };
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
            let (decision, app_handle) = if let Ok(mut guard) = native_hook_state().lock() {
                if let Some(state) = guard.as_mut() {
                    let decision = state.classifier.handle_event(event);
                    let app_handle = if matches!(decision, WindowsKeyDecision::OpenSearch) {
                        Some(state.app_handle.clone())
                    } else {
                        None
                    };
                    (decision, app_handle)
                } else {
                    (unavailable_hook_state_decision(event), None)
                }
            } else {
                (unavailable_hook_state_decision(event), None)
            };

            match decision {
                WindowsKeyDecision::OpenSearch => {
                    if let Some(app_handle) = app_handle {
                        let _ = app_handle.emit_to(
                            crate::shell_windows::TOP_BAR_LABEL,
                            WINDOWS_KEY_OPEN_SEARCH_EVENT,
                            (),
                        );
                    }
                    return LRESULT(1);
                }
                WindowsKeyDecision::Suppress => return LRESULT(1),
                WindowsKeyDecision::PassThrough => {}
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(windows)]
fn keyboard_hook_event(wparam: WPARAM, lparam: LPARAM) -> Option<WindowsKeyEvent> {
    let kind = match wparam.0 as u32 {
        WM_KEYDOWN | WM_SYSKEYDOWN => WindowsKeyEventKind::KeyDown,
        WM_KEYUP | WM_SYSKEYUP => WindowsKeyEventKind::KeyUp,
        _ => return None,
    };
    let info = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let key = if info.vkCode == VK_LWIN.0 as u32 {
        WindowsKeyCode::LeftWin
    } else if info.vkCode == VK_RWIN.0 as u32 {
        WindowsKeyCode::RightWin
    } else {
        WindowsKeyCode::Other(info.vkCode)
    };
    Some(WindowsKeyEvent {
        key,
        kind,
        repeat: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down(key: WindowsKeyCode) -> WindowsKeyEvent {
        WindowsKeyEvent {
            key,
            kind: WindowsKeyEventKind::KeyDown,
            repeat: false,
        }
    }

    fn up(key: WindowsKeyCode) -> WindowsKeyEvent {
        WindowsKeyEvent {
            key,
            kind: WindowsKeyEventKind::KeyUp,
            repeat: false,
        }
    }

    #[test]
    fn left_windows_tap_opens_search_and_suppresses_default() {
        let mut classifier = WindowsKeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::OpenSearch
        );
    }

    #[test]
    fn right_windows_tap_opens_search() {
        let mut classifier = WindowsKeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::RightWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::RightWin)),
            WindowsKeyDecision::OpenSearch
        );
    }

    #[test]
    fn windows_shortcuts_pass_through() {
        let mut classifier = WindowsKeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::Other(u32::from(b'R')))),
            WindowsKeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::Other(u32::from(b'R')))),
            WindowsKeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );

        let mut classifier = WindowsKeyClassifier::default();
        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::Other(u32::from(b'D')))),
            WindowsKeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
    }

    #[test]
    fn repeated_windows_keydown_does_not_duplicate_open_search() {
        let mut classifier = WindowsKeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(WindowsKeyEvent {
                key: WindowsKeyCode::LeftWin,
                kind: WindowsKeyEventKind::KeyDown,
                repeat: true
            }),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::OpenSearch
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
    }

    #[test]
    fn duplicate_bare_windows_key_up_events_do_not_leak_start_activation() {
        let mut classifier = WindowsKeyClassifier::default();

        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::OpenSearch
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );

        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::RightWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::RightWin)),
            WindowsKeyDecision::OpenSearch
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::RightWin)),
            WindowsKeyDecision::Suppress
        );
    }

    #[test]
    fn left_right_windows_overlap_opens_only_on_final_bare_release() {
        let mut classifier = WindowsKeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::RightWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::RightWin)),
            WindowsKeyDecision::OpenSearch
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::RightWin)),
            WindowsKeyDecision::Suppress
        );
    }

    #[test]
    fn left_right_windows_overlap_with_chord_never_opens_search() {
        let mut classifier = WindowsKeyClassifier::default();

        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::RightWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(down(WindowsKeyCode::Other(u32::from(b'R')))),
            WindowsKeyDecision::PassThrough
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            classifier.handle_event(up(WindowsKeyCode::RightWin)),
            WindowsKeyDecision::Suppress
        );
    }

    #[test]
    fn unavailable_hook_state_fails_closed_only_for_windows_key_events() {
        assert_eq!(
            unavailable_hook_state_decision(down(WindowsKeyCode::LeftWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            unavailable_hook_state_decision(up(WindowsKeyCode::RightWin)),
            WindowsKeyDecision::Suppress
        );
        assert_eq!(
            unavailable_hook_state_decision(down(WindowsKeyCode::Other(u32::from(b'R')))),
            WindowsKeyDecision::PassThrough
        );
    }

    #[test]
    fn lifecycle_install_once_and_uninstall_are_idempotent() {
        let lifecycle = WindowsKeyHookLifecycle::default();

        assert!(lifecycle.install_once());
        assert!(!lifecycle.install_once());
        assert!(lifecycle.is_installed());
        assert!(lifecycle.uninstall());
        assert!(!lifecycle.uninstall());
        assert!(!lifecycle.is_installed());
    }

    #[test]
    fn emitted_event_targets_top_bar_existing_open_path() {
        assert_eq!(WINDOWS_KEY_OPEN_SEARCH_EVENT, "search:open-centered");
        assert_eq!(
            open_search_event_target_label(),
            crate::shell_windows::TOP_BAR_LABEL
        );
    }
}
