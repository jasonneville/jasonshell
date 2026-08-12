use super::{actions::resolve_activation_target, TaskbarWindow, TaskbarWindowActivityState};
use super::{
    actions::{should_fallback_post_close, should_minimize_window, should_use_foreground_handoff},
    windows::{
        infer_activity_state, is_activity_indicator_eligible, is_internal_notification_window,
        is_taskbar_candidate, sort_windows_stably, ActivitySnapshot, WindowCandidate,
    },
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    WINDOW_EX_STYLE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

fn candidate() -> WindowCandidate {
    WindowCandidate {
        class_name: "CabinetWClass".to_string(),
        title: "File Explorer".to_string(),
        process_name: "explorer".to_string(),
        process_path: None,
        hwnd: HWND(std::ptr::null_mut()),
        is_active: false,
        is_minimized: false,
        has_owner: false,
        is_cloaked: false,
        is_shell_process: false,
        is_visible: true,
        ex_style: WINDOW_EX_STYLE(0),
        process_id: 100,
    }
}

#[test]
fn excludes_tool_windows() {
    let mut window = candidate();
    window.ex_style = WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0);

    assert!(!is_taskbar_candidate(&window, 999));
}

#[test]
fn excludes_no_activate_windows() {
    let mut window = candidate();
    window.ex_style = WINDOW_EX_STYLE(WS_EX_NOACTIVATE.0);

    assert!(!is_taskbar_candidate(&window, 999));
}

#[test]
fn excludes_shell_tray_windows() {
    let mut window = candidate();
    window.class_name = "Shell_TrayWnd".to_string();

    assert!(!is_taskbar_candidate(&window, 999));
}

#[test]
fn excludes_dwm_process_windows() {
    let mut window = candidate();
    window.process_name = "dwm".to_string();
    window.title = "Notification".to_string();

    assert!(!is_taskbar_candidate(&window, 999));
}

#[test]
fn excludes_dwm_notification_window_even_when_process_name_is_missing() {
    let mut window = candidate();
    window.process_name = String::new();
    window.class_name = "Dwm".to_string();
    window.title = "DWM Notification Window".to_string();

    assert!(is_internal_notification_window(&window));
    assert!(!is_taskbar_candidate(&window, 999));
}

#[test]
fn includes_minimized_windows_with_identity() {
    let mut window = candidate();
    window.is_visible = false;
    window.is_minimized = true;

    assert!(is_taskbar_candidate(&window, 999));
}

#[test]
fn includes_non_primary_visible_windows_with_identity() {
    let window = candidate();

    assert!(is_taskbar_candidate(&window, 999));
}

#[test]
fn excludes_empty_title_helper_windows_without_explicit_taskbar_style() {
    let mut window = candidate();
    window.title = String::new();
    window.process_name = "helper".to_string();

    assert!(!is_taskbar_candidate(&window, 999));
}

#[test]
fn allows_empty_title_windows_that_force_taskbar_presence() {
    let mut window = candidate();
    window.title = String::new();
    window.process_name = "tool".to_string();
    window.ex_style = WINDOW_EX_STYLE(WS_EX_APPWINDOW.0);

    assert!(is_taskbar_candidate(&window, 999));
}

#[test]
fn sorts_windows_by_handle_for_stable_taskbar_order() {
    let mut windows = vec![
        TaskbarWindow {
            hwnd: "42".to_string(),
            title: "Third".to_string(),
            process_id: 42,
            process_name: "third".to_string(),
            icon_data_url: String::new(),
            is_active: true,
            is_minimized: false,
            activity_state: TaskbarWindowActivityState::Idle,
        },
        TaskbarWindow {
            hwnd: "7".to_string(),
            title: "First".to_string(),
            process_id: 7,
            process_name: "first".to_string(),
            icon_data_url: String::new(),
            is_active: false,
            is_minimized: false,
            activity_state: TaskbarWindowActivityState::Idle,
        },
        TaskbarWindow {
            hwnd: "15".to_string(),
            title: "Second".to_string(),
            process_id: 15,
            process_name: "second".to_string(),
            icon_data_url: String::new(),
            is_active: false,
            is_minimized: true,
            activity_state: TaskbarWindowActivityState::Idle,
        },
    ];

    sort_windows_stably(&mut windows);

    assert_eq!(
        windows
            .iter()
            .map(|window| window.hwnd.as_str())
            .collect::<Vec<_>>(),
        vec!["7", "15", "42"]
    );
}

#[test]
fn minimizes_only_from_live_foreground_state() {
    assert!(should_minimize_window(false, true, false, false));
    assert!(should_minimize_window(true, false, false, true));
    assert!(!should_minimize_window(true, false, false, false));
}

#[test]
fn does_not_minimize_already_minimized_window() {
    assert!(!should_minimize_window(true, false, true, true));
}

#[test]
fn retries_close_with_post_message_when_window_remains() {
    assert!(should_fallback_post_close(false, true));
    assert!(should_fallback_post_close(true, true));
    assert!(!should_fallback_post_close(false, false));
}

#[test]
fn uses_foreground_handoff_when_set_foreground_is_denied_for_non_minimized_windows() {
    assert!(should_use_foreground_handoff(false));
}

#[test]
fn skips_foreground_handoff_when_set_foreground_succeeds_or_window_is_minimized() {
    assert!(!should_use_foreground_handoff(true));
}

#[test]
fn resolves_activation_target_to_visible_last_active_popup_or_root_owner() {
    let hwnd = HWND(0x1234 as *mut _);
    let target = resolve_activation_target(hwnd);
    assert_eq!(target, hwnd);
}

#[test]
fn marks_window_busy_when_title_changes_between_refreshes() {
    let previous = ActivitySnapshot {
        process_id: 10,
        title: "OpenCode".to_string(),
        cpu_time_ticks: Some(1_000),
    };

    assert_eq!(
        infer_activity_state(
            Some(&previous),
            10,
            "OpenCode - running",
            "WindowsTerminal",
            Some(1_000)
        ),
        TaskbarWindowActivityState::Busy
    );
}

#[test]
fn marks_window_busy_when_process_cpu_advances_enough_between_refreshes() {
    let previous = ActivitySnapshot {
        process_id: 10,
        title: "Terminal".to_string(),
        cpu_time_ticks: Some(1_000),
    };

    assert_eq!(
        infer_activity_state(
            Some(&previous),
            10,
            "Terminal",
            "WindowsTerminal",
            Some(300_000)
        ),
        TaskbarWindowActivityState::Busy
    );
}

#[test]
fn keeps_window_idle_without_previous_activity_delta() {
    let previous = ActivitySnapshot {
        process_id: 10,
        title: "Terminal".to_string(),
        cpu_time_ticks: Some(1_000),
    };

    assert_eq!(
        infer_activity_state(
            Some(&previous),
            10,
            "Terminal",
            "WindowsTerminal",
            Some(1_100)
        ),
        TaskbarWindowActivityState::Idle
    );
}

#[test]
fn suppresses_generic_window_activity_indicator_even_when_title_changes() {
    let previous = ActivitySnapshot {
        process_id: 10,
        title: "Quarterly Report".to_string(),
        cpu_time_ticks: Some(1_000),
    };

    assert_eq!(
        infer_activity_state(
            Some(&previous),
            10,
            "Quarterly Report - Edited",
            "notepad",
            Some(300_000)
        ),
        TaskbarWindowActivityState::Idle
    );
}

#[test]
fn allows_llm_and_terminal_activity_indicators() {
    assert!(is_activity_indicator_eligible(
        "OpenCode",
        "Generating response"
    ));
    assert!(is_activity_indicator_eligible("pwsh", "cargo test"));
    assert!(is_activity_indicator_eligible(
        "WindowsTerminal",
        "Claude prompt"
    ));
}

#[test]
fn suppresses_generic_windows_with_llm_text_in_title() {
    assert!(!is_activity_indicator_eligible(
        "notepad",
        "OpenCode planning notes"
    ));
    assert!(!is_activity_indicator_eligible(
        "notepad",
        "Claude prompt notes"
    ));
}

#[test]
fn allows_browser_activity_indicators_only_for_downloads() {
    assert!(is_activity_indicator_eligible(
        "firefox",
        "Downloads - example.zip"
    ));
    assert!(is_activity_indicator_eligible("chrome", "Downloading file"));
    assert!(!is_activity_indicator_eligible("firefox", "News"));
    assert!(!is_activity_indicator_eligible("chrome", "Docs"));
    assert!(!is_activity_indicator_eligible(
        "notepad",
        "Chrome download notes"
    ));
    assert!(!is_activity_indicator_eligible("", "Chrome download notes"));
    assert!(is_activity_indicator_eligible("chrome", "Download notes"));
}
