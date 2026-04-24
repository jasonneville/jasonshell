use super::TaskbarWindow;
use super::{
    actions::should_minimize_window,
    windows::{is_taskbar_candidate, sort_windows_stably, WindowCandidate},
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{WINDOW_EX_STYLE, WS_EX_TOOLWINDOW};

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
        is_primary_monitor: true,
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
fn excludes_shell_tray_windows() {
    let mut window = candidate();
    window.class_name = "Shell_TrayWnd".to_string();

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
fn sorts_windows_by_handle_for_stable_taskbar_order() {
    let mut windows = vec![
        TaskbarWindow {
            hwnd: "42".to_string(),
            title: "Third".to_string(),
            process_name: "third".to_string(),
            icon_data_url: String::new(),
            is_active: true,
            is_minimized: false,
        },
        TaskbarWindow {
            hwnd: "7".to_string(),
            title: "First".to_string(),
            process_name: "first".to_string(),
            icon_data_url: String::new(),
            is_active: false,
            is_minimized: false,
        },
        TaskbarWindow {
            hwnd: "15".to_string(),
            title: "Second".to_string(),
            process_name: "second".to_string(),
            icon_data_url: String::new(),
            is_active: false,
            is_minimized: true,
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
fn minimizes_rendered_active_window_even_after_shell_focuses() {
    assert!(should_minimize_window(true, false, false));
}

#[test]
fn does_not_minimize_already_minimized_window() {
    assert!(!should_minimize_window(true, false, true));
}
