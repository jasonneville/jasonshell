#![cfg(target_os = "windows")]

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetWindowThreadProcessId,
    MsgWaitForMultipleObjectsEx, PeekMessageW, PostThreadMessageW, RegisterClassW,
    RegisterShellHookWindow, RegisterWindowMessageW, TranslateMessage, UnregisterClassW,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY,
    EVENT_OBJECT_HIDE, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW, EVENT_SYSTEM_FOREGROUND,
    EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, MSG, MWMO_INPUTAVAILABLE, OBJID_WINDOW,
    PM_REMOVE, QS_ALLINPUT, WINDOW_EX_STYLE, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
    WM_APP, WM_DISPLAYCHANGE, WM_QUIT, WNDCLASSW, WS_OVERLAPPED,
};

fn class_name() -> PCWSTR {
    PCWSTR::from_raw(wide_null("JasonShellNativeHooksWindow").as_ptr())
}

fn shellhook_name() -> PCWSTR {
    PCWSTR::from_raw(wide_null("SHELLHOOK").as_ptr())
}

const HSHELL_FLASH: i32 = 0x8006;
const WM_TASKBAR_FLASH: u32 = WM_APP + 1;
const EVENT_SYSTEM_FOREGROUND_ID: u32 = 3;
const WINEVENT_OUTOFCONTEXT_FLAG: u32 = 0;

static STATE: OnceLock<Mutex<State>> = OnceLock::new();
static SHELL_MSG_ID: AtomicU32 = AtomicU32::new(0);
static CLASS_NAME_BUF: OnceLock<Vec<u16>> = OnceLock::new();
static SHELLHOOK_NAME_BUF: OnceLock<Vec<u16>> = OnceLock::new();

fn wide_null(s: &str) -> &'static [u16] {
    let buf = if s == "JasonShellNativeHooksWindow" {
        CLASS_NAME_BUF.get_or_init(|| s.encode_utf16().chain(Some(0)).collect())
    } else {
        SHELLHOOK_NAME_BUF.get_or_init(|| s.encode_utf16().chain(Some(0)).collect())
    };
    buf
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeTaskbarLifecycleEvent {
    Flash,
    Foreground,
    Create,
    Destroy,
    Show,
    Hide,
    NameChange,
    MinimizeStart,
    MinimizeEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeHooksHealth {
    Disabled,
    Healthy,
    Degraded,
    Unhealthy,
}

struct State {
    running: bool,
    stop: AtomicBool,
    worker: Option<JoinHandle<()>>,
    worker_thread_id: AtomicU32,
    hwnd: Option<isize>,
    sender: Option<SyncSender<HookEvent>>,
    shell_msg_id: u32,
    health: NativeHooksHealth,
    shell_hook_status: &'static str,
    win_event_status: &'static str,
    last_signal: Option<HookSignalSnapshot>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            running: false,
            stop: AtomicBool::new(false),
            worker: None,
            worker_thread_id: AtomicU32::new(0),
            hwnd: None,
            sender: None,
            shell_msg_id: 0,
            health: NativeHooksHealth::Disabled,
            shell_hook_status: "disabled",
            win_event_status: "disabled",
            last_signal: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HookSignalSnapshot {
    pub signal: String,
    pub timestamp_ms: u64,
}

#[derive(Clone, Copy)]
struct HookEvent {
    event: NativeTaskbarLifecycleEvent,
    hwnd: isize,
    timestamp_ms: u128,
}

pub fn explorer_suppression_v2_enabled_from_env() -> bool {
    matches!(std::env::var("JASONSHELL_EXPLORER_SUPPRESSION_V2"), Ok(v) if v == "1")
}

pub fn taskbar_native_hooks_enabled_from_env() -> bool {
    std::env::var("JASONSHELL_TASKBAR_NATIVE_HOOKS").map_or(true, |value| value != "0")
}

pub fn native_hooks_health() -> NativeHooksHealth {
    STATE
        .get()
        .and_then(|state| state.lock().ok())
        .map(|guard| guard.health)
        .unwrap_or(NativeHooksHealth::Disabled)
}

pub fn native_hooks_diagnostics_snapshot() -> super::diagnostics::NativeHooksDiagnosticsSnapshot {
    let (shell_hook, win_event, last_signal) = STATE
        .get()
        .and_then(|state| state.lock().ok())
        .map(|guard| {
            (
                guard.shell_hook_status,
                guard.win_event_status,
                guard.last_signal.clone(),
            )
        })
        .unwrap_or(("disabled", "disabled", None));
    super::diagnostics::NativeHooksDiagnosticsSnapshot {
        health: super::diagnostics::NativeHooksHealthSnapshot {
            shell_hook: shell_hook.to_string(),
            win_event: win_event.to_string(),
        },
        last_signal: last_signal.map(|signal| super::diagnostics::NativeHooksSignalSnapshot {
            signal: signal.signal,
            timestamp_ms: signal.timestamp_ms,
        }),
    }
}

pub fn start_native_hooks() -> Result<(), String> {
    if !explorer_suppression_v2_enabled_from_env() && !taskbar_native_hooks_enabled_from_env() {
        return Ok(());
    }
    let state = STATE.get_or_init(|| Mutex::new(State::default()));
    let mut guard = state
        .lock()
        .map_err(|_| "native hooks state poisoned".to_string())?;
    if guard.running {
        return Ok(());
    }

    let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<WorkerInit, String>>(1);
    guard.stop.store(false, Ordering::SeqCst);
    guard.running = true;
    guard.worker = Some(thread::spawn(move || worker_thread(ready_tx)));
    drop(guard);

    let init = ready_rx
        .recv_timeout(std::time::Duration::from_secs(3))
        .map_err(|_| "native hooks worker failed to initialize".to_string());
    let mut guard = state
        .lock()
        .map_err(|_| "native hooks state poisoned".to_string())?;
    match init {
        Ok(Ok(worker)) => {
            guard
                .worker_thread_id
                .store(worker.thread_id, Ordering::SeqCst);
            guard.hwnd = Some(worker.hwnd);
            guard.sender = Some(worker.sender);
            guard.shell_msg_id = worker.shell_msg_id;
            Ok(())
        }
        Ok(Err(err)) | Err(err) => {
            guard.running = false;
            guard.stop.store(true, Ordering::SeqCst);
            let join = guard.worker.take();
            let thread_id = guard.worker_thread_id.load(Ordering::SeqCst);
            drop(guard);
            if thread_id != 0 {
                let _ = unsafe { PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
            }
            if let Some(join) = join {
                let _ = join.join();
            }
            if let Ok(mut guard) = state.lock() {
                guard.worker_thread_id.store(0, Ordering::SeqCst);
                guard.hwnd = None;
                guard.sender = None;
                guard.shell_msg_id = 0;
                guard.health = NativeHooksHealth::Unhealthy;
                guard.stop.store(false, Ordering::SeqCst);
            }
            Err(err)
        }
    }
}

pub fn stop_native_hooks() {
    let Some(state) = STATE.get() else {
        return;
    };
    let (join, thread_id, hwnd) = {
        let mut guard = match state.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if !guard.running {
            return;
        }
        guard.stop.store(true, Ordering::SeqCst);
        let thread_id = guard.worker_thread_id.load(Ordering::SeqCst);
        let hwnd = guard.hwnd;
        if thread_id != 0 {
            let _ = unsafe { PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        }
        (guard.worker.take(), thread_id, hwnd)
    };
    let _ = thread_id;
    let _ = hwnd;
    if let Some(join) = join {
        let _ = join.join();
    }
    if let Ok(mut guard) = state.lock() {
        guard.running = false;
        guard.worker_thread_id.store(0, Ordering::SeqCst);
        guard.hwnd = None;
        guard.sender = None;
        guard.shell_msg_id = 0;
        guard.health = NativeHooksHealth::Disabled;
        guard.shell_hook_status = "disabled";
        guard.win_event_status = "disabled";
        guard.stop.store(false, Ordering::SeqCst);
    }
}

fn worker_thread(ready_tx: SyncSender<Result<WorkerInit, String>>) {
    let thread_id = unsafe { GetCurrentThreadId() };
    if let Some(state) = STATE.get() {
        if let Ok(guard) = state.lock() {
            guard.worker_thread_id.store(thread_id, Ordering::SeqCst);
        }
    }
    let shell_msg_id = unsafe { RegisterWindowMessageW(shellhook_name()) };
    if shell_msg_id == 0 {
        let _ = ready_tx.send(Err("RegisterWindowMessageW failed".to_string()));
        return;
    }
    let (signal_tx, signal_rx) = mpsc::sync_channel::<HookEvent>(32);
    let hinstance = unsafe { GetModuleHandleW(None) };
    if hinstance.is_err() {
        let _ = ready_tx.send(Err("GetModuleHandleW failed".to_string()));
        return;
    }
    let hinstance = hinstance.unwrap();

    let class_name = class_name();
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        hInstance: hinstance.into(),
        lpszClassName: class_name,
        style: CS_HREDRAW | CS_VREDRAW,
        ..Default::default()
    };
    let atom = unsafe { RegisterClassW(&wc) };
    if atom == 0 {
        let _ = ready_tx.send(Err("RegisterClassW failed".to_string()));
        return;
    }

    let hwnd = match unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            PCWSTR::null(),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
    } {
        Ok(hwnd) => hwnd,
        Err(_) => {
            let _ = unsafe { UnregisterClassW(class_name, Some(hinstance.into())) };
            let _ = ready_tx.send(Err("CreateWindowExW failed".to_string()));
            return;
        }
    };

    let object_hook = unsafe {
        SetWinEventHook(
            EVENT_OBJECT_CREATE,
            EVENT_OBJECT_NAMECHANGE,
            None,
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
    };
    let foreground_hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
    };
    let minimize_hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_MINIMIZESTART,
            EVENT_SYSTEM_MINIMIZEEND,
            None,
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
    };
    let object_registered = !object_hook.0.is_null();
    let foreground_registered = !foreground_hook.0.is_null();
    let minimize_registered = !minimize_hook.0.is_null();
    let hooks: Vec<_> = [object_hook, foreground_hook, minimize_hook]
        .into_iter()
        .filter(|hook| !hook.0.is_null())
        .collect();
    SHELL_MSG_ID.store(shell_msg_id, Ordering::SeqCst);
    let shell_registered = unsafe { RegisterShellHookWindow(hwnd) }.as_bool();

    if let Some(state) = STATE.get() {
        if let Ok(mut guard) = state.lock() {
            guard.health = if !shell_registered {
                NativeHooksHealth::Unhealthy
            } else if foreground_registered && object_registered && minimize_registered {
                NativeHooksHealth::Healthy
            } else {
                NativeHooksHealth::Degraded
            };
            guard.shell_hook_status = if shell_registered {
                "registered"
            } else {
                "failed"
            };
            guard.win_event_status =
                if foreground_registered && object_registered && minimize_registered {
                    "registered"
                } else if foreground_registered || object_registered || minimize_registered {
                    "partial"
                } else {
                    "failed"
                };
        }
    }
    let _ = ready_tx.send(Ok(WorkerInit {
        thread_id,
        hwnd: hwnd.0 as isize,
        sender: signal_tx.clone(),
        shell_msg_id,
    }));
    println!(
        "{{\"kind\":\"nativeHookInit\",\"shellMessageId\":{},\"hwnd\":{},\"threadId\":{}}}",
        shell_msg_id, hwnd.0 as isize, thread_id
    );
    let _ = io::stdout().flush();

    let mut msg = MSG::default();
    'message_loop: loop {
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            if msg.message == WM_QUIT {
                break 'message_loop;
            }
            if msg.message == WM_TASKBAR_FLASH {
                handle_event(HookEvent {
                    event: NativeTaskbarLifecycleEvent::Flash,
                    hwnd: msg.lParam.0 as isize,
                    timestamp_ms: now_ms(),
                });
                continue;
            }
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        drain_signals(&signal_rx);
        if should_stop() {
            break;
        }
        unsafe {
            let _ = MsgWaitForMultipleObjectsEx(None, 50, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        }
    }

    cleanup_worker(hwnd, hooks, hinstance.into());
}

fn should_stop() -> bool {
    STATE
        .get()
        .and_then(|s| s.lock().ok())
        .map(|g| g.stop.load(Ordering::SeqCst))
        .unwrap_or(true)
}

fn drain_signals(rx: &Receiver<HookEvent>) {
    loop {
        match rx.try_recv() {
            Ok(event) => handle_event(event),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
        }
    }
}

fn handle_event(event: HookEvent) {
    let signal = match event.event {
        NativeTaskbarLifecycleEvent::Flash => "Flash",
        NativeTaskbarLifecycleEvent::Foreground => "Foreground",
        NativeTaskbarLifecycleEvent::Create => "Create",
        NativeTaskbarLifecycleEvent::Destroy => "Destroy",
        NativeTaskbarLifecycleEvent::Show => "Show",
        NativeTaskbarLifecycleEvent::Hide => "Hide",
        NativeTaskbarLifecycleEvent::NameChange => "NameChange",
        NativeTaskbarLifecycleEvent::MinimizeStart => "MinimizeStart",
        NativeTaskbarLifecycleEvent::MinimizeEnd => "MinimizeEnd",
    };
    if taskbar_native_hooks_enabled_from_env() {
        process_native_signal(event.event, event.hwnd);
        super::windows::request_taskbar_snapshot_refresh_native(Instant::now());
    }
    if let Some(state) = STATE.get() {
        if let Ok(mut guard) = state.lock() {
            guard.last_signal = Some(HookSignalSnapshot {
                signal: signal.to_string(),
                timestamp_ms: event.timestamp_ms as u64,
            });
        }
    }
    if explorer_suppression_v2_enabled_from_env()
        && matches!(
            event.event,
            NativeTaskbarLifecycleEvent::Create
                | NativeTaskbarLifecycleEvent::Destroy
                | NativeTaskbarLifecycleEvent::Show
                | NativeTaskbarLifecycleEvent::Hide
                | NativeTaskbarLifecycleEvent::NameChange
        )
    {
        crate::explorer::request_taskbar_reconcile();
    }
    println!(
        "{{\"kind\":\"nativeHook\",\"signal\":\"{}\",\"hwnd\":{},\"timestampMs\":{}}}",
        signal, event.hwnd, event.timestamp_ms
    );
    let _ = io::stdout().flush();
}

fn process_native_signal(signal: NativeTaskbarLifecycleEvent, hwnd: isize) {
    let mut pid = 0;
    unsafe {
        let _ =
            GetWindowThreadProcessId(windows::Win32::Foundation::HWND(hwnd as _), Some(&mut pid));
    }
    let identity = super::windows::attention_identity_for_hwnd(
        windows::Win32::Foundation::HWND(hwnd as _),
        pid,
    )
    .unwrap_or(super::attention::TaskbarAttentionIdentity {
        root_owner_hwnd: hwnd,
        process_id: pid,
        creation_time: None,
    });
    match signal {
        NativeTaskbarLifecycleEvent::Flash => {
            super::attention::record_taskbar_attention(identity, true)
        }
        NativeTaskbarLifecycleEvent::Foreground => {
            super::attention::clear_taskbar_attention_if_matches(&identity)
        }
        NativeTaskbarLifecycleEvent::Destroy
            if identity.root_owner_hwnd == hwnd && identity.process_id != 0 =>
        {
            super::windows::remove_root_owner_taskbar_attention(identity.root_owner_hwnd)
        }
        NativeTaskbarLifecycleEvent::Destroy => {}
        NativeTaskbarLifecycleEvent::Create
        | NativeTaskbarLifecycleEvent::Show
        | NativeTaskbarLifecycleEvent::Hide
        | NativeTaskbarLifecycleEvent::NameChange
        | NativeTaskbarLifecycleEvent::MinimizeStart
        | NativeTaskbarLifecycleEvent::MinimizeEnd => {}
    }
}

fn post_taskbar_flash_or_fallback(
    thread_id: u32,
    lparam: LPARAM,
    mut post: impl FnMut(u32, u32, WPARAM, LPARAM) -> bool,
    mut fallback: impl FnMut(LPARAM),
) {
    if post(thread_id, WM_TASKBAR_FLASH, WPARAM(0), lparam) {
        return;
    }
    fallback(lparam);
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn coalesce_hook_wake_timeout(
    last_wake: Option<Instant>,
    now: Instant,
    timeout: std::time::Duration,
) -> bool {
    last_wake
        .map(|wake| now.duration_since(wake) >= timeout)
        .unwrap_or(true)
}

fn cleanup_worker(hwnd: HWND, hooks: Vec<HWINEVENTHOOK>, hinstance: HINSTANCE) {
    SHELL_MSG_ID.store(0, Ordering::SeqCst);
    unsafe {
        let _ = deregister_shell_hook_window(hwnd);
        for hook in hooks {
            let _ = UnhookWinEvent(hook);
        }
        let _ = DestroyWindow(hwnd);
        let _ = UnregisterClassW(class_name(), Some(hinstance));
    }
    println!("{{\"kind\":\"nativeHookCleanup\"}}");
    let _ = io::stdout().flush();
}

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_DISPLAYCHANGE && explorer_suppression_v2_enabled_from_env() {
        crate::explorer::request_taskbar_reconcile();
        return LRESULT(0);
    }
    if msg == SHELL_MSG_ID.load(Ordering::SeqCst) && wparam.0 as i32 == HSHELL_FLASH {
        post_taskbar_flash_or_fallback(
            unsafe { GetCurrentThreadId() },
            lparam,
            |thread_id, message, wparam, lparam| unsafe {
                PostThreadMessageW(thread_id, message, wparam, lparam).is_ok()
            },
            |lparam| {
                handle_event(HookEvent {
                    event: NativeTaskbarLifecycleEvent::Flash,
                    hwnd: lparam.0 as isize,
                    timestamp_ms: now_ms(),
                });
            },
        );
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

unsafe extern "system" fn win_event_proc(
    _: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _: u32,
    _: u32,
) {
    if id_object != OBJID_WINDOW.0 || id_child != 0 {
        return;
    }
    let event = match event {
        EVENT_OBJECT_CREATE => NativeTaskbarLifecycleEvent::Create,
        EVENT_OBJECT_DESTROY => NativeTaskbarLifecycleEvent::Destroy,
        EVENT_OBJECT_SHOW => NativeTaskbarLifecycleEvent::Show,
        EVENT_OBJECT_HIDE => NativeTaskbarLifecycleEvent::Hide,
        EVENT_OBJECT_NAMECHANGE => NativeTaskbarLifecycleEvent::NameChange,
        EVENT_SYSTEM_FOREGROUND => NativeTaskbarLifecycleEvent::Foreground,
        EVENT_SYSTEM_MINIMIZESTART => NativeTaskbarLifecycleEvent::MinimizeStart,
        EVENT_SYSTEM_MINIMIZEEND => NativeTaskbarLifecycleEvent::MinimizeEnd,
        _ => return,
    };
    enqueue_signal(event, hwnd.0 as isize);
}

fn enqueue_signal(event: NativeTaskbarLifecycleEvent, hwnd: isize) {
    let Some(state) = STATE.get() else {
        return;
    };
    let sender = {
        let guard = match state.try_lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        guard.sender.clone()
    };
    if let Some(sender) = sender {
        let _ = sender.try_send(HookEvent {
            event,
            hwnd,
            timestamp_ms: now_ms(),
        });
    }
}

#[link(name = "user32")]
unsafe extern "system" {
    #[link_name = "DeregisterShellHookWindow"]
    fn deregister_shell_hook_window(hwnd: HWND) -> i32;
}

struct WorkerInit {
    thread_id: u32,
    hwnd: isize,
    sender: SyncSender<HookEvent>,
    shell_msg_id: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn native_attention_defaults_on_with_exact_zero_kill_switch() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        std::env::remove_var("JASONSHELL_TASKBAR_NATIVE_HOOKS");
        assert!(taskbar_native_hooks_enabled_from_env());
        std::env::set_var("JASONSHELL_TASKBAR_NATIVE_HOOKS", "0");
        assert!(!taskbar_native_hooks_enabled_from_env());
        std::env::set_var("JASONSHELL_TASKBAR_NATIVE_HOOKS", "1");
        assert!(taskbar_native_hooks_enabled_from_env());
        std::env::remove_var("JASONSHELL_TASKBAR_NATIVE_HOOKS");

        std::env::remove_var("JASONSHELL_EXPLORER_SUPPRESSION_V2");
        assert!(!explorer_suppression_v2_enabled_from_env());
        std::env::set_var("JASONSHELL_EXPLORER_SUPPRESSION_V2", "1");
        assert!(explorer_suppression_v2_enabled_from_env());
        std::env::set_var("JASONSHELL_EXPLORER_SUPPRESSION_V2", "true");
        assert!(!explorer_suppression_v2_enabled_from_env());
        std::env::remove_var("JASONSHELL_EXPLORER_SUPPRESSION_V2");
    }

    #[test]
    fn json_line_formatting_stable() {
        let line = format!(
            "{{\"kind\":\"nativeHook\",\"signal\":\"{}\",\"hwnd\":{},\"timestampMs\":{}}}",
            "Flash", 12, 34
        );
        assert_eq!(
            line,
            "{\"kind\":\"nativeHook\",\"signal\":\"Flash\",\"hwnd\":12,\"timestampMs\":34}"
        );
    }

    #[test]
    fn coalesce_hook_wake_timeout_blocks_duplicate_wakes() {
        let now = Instant::now();
        assert!(coalesce_hook_wake_timeout(
            None,
            now,
            std::time::Duration::from_millis(1500)
        ));
        assert!(!coalesce_hook_wake_timeout(
            Some(now),
            now,
            std::time::Duration::from_millis(1500)
        ));
    }

    #[test]
    fn real_worker_stops_and_clears_native_resources() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        std::env::set_var("JASONSHELL_EXPLORER_SUPPRESSION_V2", "1");

        start_native_hooks().expect("native hook worker should start");
        stop_native_hooks();

        let state = STATE.get().unwrap().lock().unwrap();
        assert!(!state.running);
        assert!(state.worker.is_none());
        assert_eq!(state.worker_thread_id.load(Ordering::SeqCst), 0);
        assert!(state.hwnd.is_none());
        assert!(state.sender.is_none());
        assert_eq!(state.shell_msg_id, 0);
        drop(state);
        std::env::remove_var("JASONSHELL_EXPLORER_SUPPRESSION_V2");
    }

    #[test]
    fn post_taskbar_flash_or_fallback_uses_fallback_exactly_once_on_failure() {
        let mut post_calls = 0;
        let mut fallback_calls = 0;
        post_taskbar_flash_or_fallback(
            1,
            LPARAM(77),
            |_, _, _, _| {
                post_calls += 1;
                false
            },
            |_| {
                fallback_calls += 1;
            },
        );
        assert_eq!(post_calls, 1);
        assert_eq!(fallback_calls, 1);
    }

    #[test]
    fn post_taskbar_flash_or_fallback_skips_fallback_on_success() {
        let mut post_calls = 0;
        let mut fallback_calls = 0;
        post_taskbar_flash_or_fallback(
            1,
            LPARAM(77),
            |_, _, _, _| {
                post_calls += 1;
                true
            },
            |_| {
                fallback_calls += 1;
            },
        );
        assert_eq!(post_calls, 1);
        assert_eq!(fallback_calls, 0);
    }
}
