use crate::layout::build_shell_preview_rects;
use std::error::Error;
use tauri::{App, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const TOP_BAR_LABEL: &str = "top-bar";
pub const BOTTOM_BAR_LABEL: &str = "bottom-bar";
pub const TASK_PREVIEW_LABEL: &str = "task-preview";
pub const SEARCH_PANEL_LABEL: &str = "search-panel";
pub const STACK_POPUP_LABEL: &str = "stack-popup";
pub const TOP_BAR_HEIGHT_LOGICAL: f64 = 26.0;
pub const BOTTOM_BAR_HEIGHT_LOGICAL: f64 = 42.0;
pub const TASK_PREVIEW_WIDTH_LOGICAL: f64 = 332.0;
pub const TASK_PREVIEW_HEIGHT_LOGICAL: f64 = 228.0;
pub const SEARCH_PANEL_WIDTH_LOGICAL: f64 = 420.0;
pub const SEARCH_PANEL_HEIGHT_LOGICAL: f64 = 320.0;
pub const STACK_POPUP_WIDTH_LOGICAL: f64 = 620.0;
pub const STACK_POPUP_HEIGHT_LOGICAL: f64 = 430.0;
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

    Ok(CreatedShellWindows { top, bottom })
}

fn build_shell_window(
    app: &App,
    label: &str,
    title: &str,
    logical_width: f64,
    logical_height: f64,
) -> AppResult<WebviewWindow> {
    Ok(
        WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
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
            .build()?,
    )
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

pub fn to_physical_height(logical_height: f64, scale_factor: f64) -> i32 {
    (logical_height * scale_factor).round() as i32
}
