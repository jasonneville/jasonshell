use crate::layout::build_shell_preview_rects;
#[cfg(windows)]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::error::Error;
use tauri::{App, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
#[cfg(windows)]
use windows::Win32::Foundation::{GetLastError, SetLastError, ERROR_SUCCESS, HWND, WIN32_ERROR};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WINDOW_EX_STYLE, WS_EX_APPWINDOW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

pub const TOP_BAR_LABEL: &str = "top-bar";
pub const BOTTOM_BAR_LABEL: &str = "bottom-bar";
pub const TASK_PREVIEW_LABEL: &str = "task-preview";
pub const SEARCH_PANEL_LABEL: &str = "search-panel";
pub const STACK_POPUP_LABEL: &str = "stack-popup";
pub const PROCESS_MANAGER_LABEL: &str = "process-manager";
pub const CONTROL_PLANE_LABEL: &str = "control-plane";
pub const SETTINGS_PANEL_LABEL: &str = "settings-panel";
pub const TRAY_PANEL_LABEL: &str = "tray-panel";
pub const COMMAND_PANEL_LABEL: &str = "command-panel";
pub const AUDIO_PANEL_LABEL: &str = "audio-panel";
#[cfg(test)]
pub const ALL_LABELS: &[&str] = &[
    TOP_BAR_LABEL,
    BOTTOM_BAR_LABEL,
    TASK_PREVIEW_LABEL,
    SEARCH_PANEL_LABEL,
    STACK_POPUP_LABEL,
    PROCESS_MANAGER_LABEL,
    CONTROL_PLANE_LABEL,
    SETTINGS_PANEL_LABEL,
    TRAY_PANEL_LABEL,
    COMMAND_PANEL_LABEL,
    AUDIO_PANEL_LABEL,
];
pub const TOP_BAR_HEIGHT_LOGICAL: f64 = 26.0;
pub const BOTTOM_BAR_HEIGHT_LOGICAL: f64 = 36.0;
pub const TASK_PREVIEW_WIDTH_LOGICAL: f64 = 332.0;
pub const TASK_PREVIEW_HEIGHT_LOGICAL: f64 = 228.0;
pub const SEARCH_PANEL_WIDTH_LOGICAL: f64 = 420.0;
pub const SEARCH_PANEL_HEIGHT_LOGICAL: f64 = 320.0;
pub const STACK_POPUP_WIDTH_LOGICAL: f64 = 980.0;
pub const STACK_POPUP_HEIGHT_LOGICAL: f64 = 430.0;
pub const PROCESS_MANAGER_WIDTH_LOGICAL: f64 = 720.0;
pub const PROCESS_MANAGER_HEIGHT_LOGICAL: f64 = 520.0;
pub const CONTROL_PLANE_WIDTH_LOGICAL: f64 = 860.0;
pub const CONTROL_PLANE_HEIGHT_LOGICAL: f64 = 620.0;
pub const SETTINGS_PANEL_WIDTH_LOGICAL: f64 = 440.0;
pub const SETTINGS_PANEL_HEIGHT_LOGICAL: f64 = 520.0;
pub const TRAY_PANEL_WIDTH_LOGICAL: f64 = 252.0;
pub const TRAY_PANEL_HEIGHT_LOGICAL: f64 = 220.0;
pub const COMMAND_PANEL_WIDTH_LOGICAL: f64 = 460.0;
pub const COMMAND_PANEL_HEIGHT_LOGICAL: f64 = 420.0;
pub const AUDIO_PANEL_WIDTH_LOGICAL: f64 = 320.0;
pub const AUDIO_PANEL_HEIGHT_LOGICAL: f64 = 430.0;
const DISABLE_NATIVE_CONTEXT_MENU_SCRIPT: &str =
    "window.addEventListener('contextmenu', (event) => event.preventDefault());";

pub type AppResult<T> = Result<T, Box<dyn Error>>;

pub struct CreatedShellWindows {
    pub top: WebviewWindow,
    pub bottom: WebviewWindow,
}

pub fn create_shell_windows(app: &mut App) -> AppResult<CreatedShellWindows> {
    let primary_monitor = app
        .primary_monitor()?
        .ok_or_else(|| "Primary monitor is unavailable".to_string())?;

    let scale_factor = primary_monitor.scale_factor();
    let logical_width = f64::from(primary_monitor.size().width) / scale_factor;
    let preview_rects = build_shell_preview_rects(
        primary_monitor.position().x,
        primary_monitor.position().y,
        primary_monitor.size().width as i32,
        primary_monitor.size().height as i32,
        to_physical_height(TOP_BAR_HEIGHT_LOGICAL, scale_factor),
        to_physical_height(BOTTOM_BAR_HEIGHT_LOGICAL, scale_factor),
    );

    let top = build_shell_window(
        app,
        TOP_BAR_LABEL,
        "JasonShell Top Bar",
        logical_width,
        f64::from(preview_rects.top.height()) / scale_factor,
    )?;

    let bottom = build_shell_window(
        app,
        BOTTOM_BAR_LABEL,
        "JasonShell Bottom Bar",
        logical_width,
        f64::from(preview_rects.bottom.height()) / scale_factor,
    )?;

    let _preview = build_preview_window(app)?;
    let _search = build_search_panel_window(app)?;
    let _stack = build_stack_popup_window(app)?;
    let _process_manager = build_process_manager_window(app)?;
    let _control_plane = build_control_plane_window(app)?;
    let _settings_panel = build_settings_panel_window(app)?;
    let _tray_panel = build_tray_panel_window(app)?;
    let _command_panel = build_command_panel_window(app)?;
    let _audio_panel = build_audio_panel_window(app)?;

    Ok(CreatedShellWindows { top, bottom })
}

fn build_shell_window(
    app: &App,
    label: &str,
    title: &str,
    logical_width: f64,
    logical_height: f64,
) -> AppResult<WebviewWindow> {
    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .always_on_top(true)
        .devtools(false)
        .decorations(false)
        .focused(false)
        .initialization_script(DISABLE_NATIVE_CONTEXT_MENU_SCRIPT)
        .inner_size(logical_width, logical_height)
        .maximizable(false)
        .minimizable(false)
        .resizable(false)
        .shadow(false)
        .skip_taskbar(true)
        .theme(Some(Theme::Dark))
        .title(title)
        .visible(false)
        .build()?;

    apply_no_alt_tab_shell_style(&window)?;

    Ok(window)
}

fn build_preview_window(app: &App) -> AppResult<WebviewWindow> {
    Ok(WebviewWindowBuilder::new(
        app,
        TASK_PREVIEW_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .always_on_top(true)
    .devtools(false)
    .decorations(false)
    .focused(false)
    .initialization_script(DISABLE_NATIVE_CONTEXT_MENU_SCRIPT)
    .inner_size(TASK_PREVIEW_WIDTH_LOGICAL, TASK_PREVIEW_HEIGHT_LOGICAL)
    .maximizable(false)
    .minimizable(false)
    .resizable(false)
    .shadow(true)
    .skip_taskbar(true)
    .theme(Some(Theme::Dark))
    .title("JasonShell Task Preview")
    .transparent(true)
    .visible(false)
    .build()?)
}

fn build_search_panel_window(app: &App) -> AppResult<WebviewWindow> {
    Ok(WebviewWindowBuilder::new(
        app,
        SEARCH_PANEL_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .always_on_top(true)
    .devtools(false)
    .decorations(false)
    .focused(false)
    .initialization_script(DISABLE_NATIVE_CONTEXT_MENU_SCRIPT)
    .inner_size(SEARCH_PANEL_WIDTH_LOGICAL, SEARCH_PANEL_HEIGHT_LOGICAL)
    .maximizable(false)
    .minimizable(false)
    .resizable(false)
    .shadow(true)
    .skip_taskbar(true)
    .theme(Some(Theme::Dark))
    .title("JasonShell Search")
    .visible(false)
    .build()?)
}

fn build_stack_popup_window(app: &App) -> AppResult<WebviewWindow> {
    Ok(
        WebviewWindowBuilder::new(app, STACK_POPUP_LABEL, WebviewUrl::App("index.html".into()))
            .always_on_top(true)
            .devtools(false)
            .decorations(false)
            .focused(false)
            .initialization_script(DISABLE_NATIVE_CONTEXT_MENU_SCRIPT)
            .inner_size(STACK_POPUP_WIDTH_LOGICAL, STACK_POPUP_HEIGHT_LOGICAL)
            .maximizable(false)
            .minimizable(false)
            .resizable(false)
            .shadow(true)
            .skip_taskbar(true)
            .theme(Some(Theme::Dark))
            .title("JasonShell Stack")
            .visible(false)
            .build()?,
    )
}

fn build_process_manager_window(app: &App) -> AppResult<WebviewWindow> {
    Ok(WebviewWindowBuilder::new(
        app,
        PROCESS_MANAGER_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .always_on_top(true)
    .devtools(false)
    .decorations(false)
    .focused(false)
    .initialization_script(DISABLE_NATIVE_CONTEXT_MENU_SCRIPT)
    .inner_size(
        PROCESS_MANAGER_WIDTH_LOGICAL,
        PROCESS_MANAGER_HEIGHT_LOGICAL,
    )
    .maximizable(false)
    .minimizable(false)
    .resizable(false)
    .shadow(true)
    .skip_taskbar(true)
    .theme(Some(Theme::Dark))
    .title("JasonShell Process Manager")
    .visible(false)
    .build()?)
}

fn build_control_plane_window(app: &App) -> AppResult<WebviewWindow> {
    Ok(WebviewWindowBuilder::new(
        app,
        CONTROL_PLANE_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .always_on_top(true)
    .devtools(false)
    .decorations(false)
    .focused(false)
    .initialization_script(DISABLE_NATIVE_CONTEXT_MENU_SCRIPT)
    .inner_size(CONTROL_PLANE_WIDTH_LOGICAL, CONTROL_PLANE_HEIGHT_LOGICAL)
    .maximizable(false)
    .minimizable(false)
    .resizable(false)
    .shadow(true)
    .skip_taskbar(true)
    .theme(Some(Theme::Dark))
    .title("JasonShell Control Plane")
    .visible(false)
    .build()?)
}

fn build_settings_panel_window(app: &App) -> AppResult<WebviewWindow> {
    Ok(WebviewWindowBuilder::new(
        app,
        SETTINGS_PANEL_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .always_on_top(true)
    .devtools(false)
    .decorations(false)
    .focused(false)
    .initialization_script(DISABLE_NATIVE_CONTEXT_MENU_SCRIPT)
    .inner_size(SETTINGS_PANEL_WIDTH_LOGICAL, SETTINGS_PANEL_HEIGHT_LOGICAL)
    .maximizable(false)
    .minimizable(false)
    .resizable(false)
    .shadow(true)
    .skip_taskbar(true)
    .theme(Some(Theme::Dark))
    .title("JasonShell Settings")
    .visible(false)
    .build()?)
}

fn build_audio_panel_window(app: &App) -> AppResult<WebviewWindow> {
    Ok(
        WebviewWindowBuilder::new(app, AUDIO_PANEL_LABEL, WebviewUrl::App("index.html".into()))
            .always_on_top(true)
            .devtools(false)
            .decorations(false)
            .focused(false)
            .initialization_script(DISABLE_NATIVE_CONTEXT_MENU_SCRIPT)
            .inner_size(AUDIO_PANEL_WIDTH_LOGICAL, AUDIO_PANEL_HEIGHT_LOGICAL)
            .maximizable(false)
            .minimizable(false)
            .resizable(false)
            .shadow(true)
            .skip_taskbar(true)
            .theme(Some(Theme::Dark))
            .title("JasonShell Sound")
            .visible(false)
            .build()?,
    )
}

fn build_tray_panel_window(app: &App) -> AppResult<WebviewWindow> {
    Ok(WebviewWindowBuilder::new(
        app,
        TRAY_PANEL_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .always_on_top(true)
    .devtools(false)
    .decorations(false)
    .focused(false)
    .initialization_script(DISABLE_NATIVE_CONTEXT_MENU_SCRIPT)
    .inner_size(TRAY_PANEL_WIDTH_LOGICAL, TRAY_PANEL_HEIGHT_LOGICAL)
    .maximizable(false)
    .minimizable(false)
    .resizable(false)
    .shadow(true)
    .skip_taskbar(true)
    .theme(Some(Theme::Dark))
    .title("JasonShell Tray")
    .visible(false)
    .build()?)
}

fn build_command_panel_window(app: &App) -> AppResult<WebviewWindow> {
    Ok(WebviewWindowBuilder::new(
        app,
        COMMAND_PANEL_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .always_on_top(true)
    .devtools(false)
    .decorations(false)
    .focused(false)
    .initialization_script(DISABLE_NATIVE_CONTEXT_MENU_SCRIPT)
    .inner_size(COMMAND_PANEL_WIDTH_LOGICAL, COMMAND_PANEL_HEIGHT_LOGICAL)
    .maximizable(false)
    .minimizable(false)
    .resizable(false)
    .shadow(true)
    .skip_taskbar(true)
    .theme(Some(Theme::Dark))
    .title("JasonShell Commands")
    .visible(false)
    .build()?)
}

pub fn to_physical_height(logical_height: f64, scale_factor: f64) -> i32 {
    (logical_height * scale_factor).round() as i32
}

#[cfg(windows)]
fn apply_no_alt_tab_shell_style(window: &WebviewWindow) -> AppResult<()> {
    let hwnd = hwnd_from_tauri_window(window)?;
    apply_no_alt_tab_shell_style_to_hwnd(hwnd, window.label())
}

#[cfg(windows)]
pub(crate) fn apply_no_alt_tab_shell_style_to_hwnd(hwnd: HWND, context: &str) -> AppResult<()> {
    let current_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 };
    let desired_style = desired_shell_ex_style(current_style);

    if desired_style != current_style {
        unsafe {
            SetLastError(WIN32_ERROR(0));
            let previous_style = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired_style as isize);
            let last_error = GetLastError();

            if previous_style == 0 && last_error != ERROR_SUCCESS {
                return Err(format!(
                    "failed to apply shell Alt+Tab exclusion style to {context} {:?}: {:?}",
                    hwnd.0, last_error,
                )
                .into());
            }
        }
    }

    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
        )?;
    }

    Ok(())
}

#[cfg(not(windows))]
fn apply_no_alt_tab_shell_style(_window: &WebviewWindow) -> AppResult<()> {
    Ok(())
}

#[cfg(windows)]
fn hwnd_from_tauri_window(window: &WebviewWindow) -> AppResult<HWND> {
    let handle = window.window_handle()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut _)),
        other => Err(format!("unsupported window handle: {other:?}").into()),
    }
}

#[cfg(windows)]
fn desired_shell_ex_style(existing: u32) -> u32 {
    let shell_style = WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0);
    (existing | shell_style.0) & !(WS_EX_APPWINDOW.0 | WS_EX_NOACTIVATE.0)
}

#[cfg(all(test, windows))]
fn shell_ex_style_is_alt_tab_excluded(style: u32) -> bool {
    (style & WS_EX_TOOLWINDOW.0) != 0
        && (style & WS_EX_APPWINDOW.0) == 0
        && (style & WS_EX_NOACTIVATE.0) == 0
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn desired_shell_ex_style_hides_from_alt_tab() {
        let existing = WINDOW_EX_STYLE(WS_EX_APPWINDOW.0);

        let desired = desired_shell_ex_style(existing.0);

        assert!(shell_ex_style_is_alt_tab_excluded(desired));
    }

    #[test]
    fn desired_shell_ex_style_preserves_unrelated_bits() {
        let unrelated_style = 0x0000_0200;

        let desired = desired_shell_ex_style(unrelated_style);

        assert_eq!(desired & unrelated_style, unrelated_style);
        assert_eq!(desired & WS_EX_TOOLWINDOW.0, WS_EX_TOOLWINDOW.0);
        assert_eq!(desired & WS_EX_NOACTIVATE.0, 0);
    }

    #[test]
    fn alt_tab_exclusion_rejects_task_switcher_bits() {
        let toolwindow_only = WS_EX_TOOLWINDOW.0;
        let forced_appwindow = WS_EX_TOOLWINDOW.0 | WS_EX_APPWINDOW.0;
        let noactivate_helper = WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0;
        let regular_window = 0;

        assert!(shell_ex_style_is_alt_tab_excluded(toolwindow_only));
        assert!(!shell_ex_style_is_alt_tab_excluded(forced_appwindow));
        assert!(!shell_ex_style_is_alt_tab_excluded(noactivate_helper));
        assert!(!shell_ex_style_is_alt_tab_excluded(regular_window));
    }
}
