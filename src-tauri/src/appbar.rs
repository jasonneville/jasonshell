#![cfg(target_os = "windows")]

use crate::explorer;
use crate::shell_windows::{
    apply_no_alt_tab_shell_style_to_hwnd, AppResult, CreatedShellWindows,
    BOTTOM_BAR_HEIGHT_LOGICAL, TOP_BAR_HEIGHT_LOGICAL,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::c_void;
use std::mem::size_of;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tauri::{App, AppHandle, Manager};
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, RegisterWindowMessageW, SetWindowPos,
    SystemParametersInfoW, GWL_STYLE, HWND_TOPMOST, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SPIF_SENDCHANGE, SPI_GETWORKAREA, SPI_SETWORKAREA,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    WS_CAPTION, WS_THICKFRAME,
};

#[derive(Clone, Debug, serde::Serialize)]
pub struct WindowRectSnapshot {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendSurfaceMetrics {
    pub label: String,
    pub outer_height: i32,
    pub inner_height: i32,
    pub client_height: i32,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellSurfaceRuntimeMetrics {
    pub label: String,
    pub native_rect: WindowRectSnapshot,
    pub outer_height: i32,
    pub inner_height: i32,
    pub client_height: i32,
    pub native_height_ok: bool,
    pub webview_height_ok: bool,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ShellBarResizeEdge {
    Top,
    Bottom,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeShellBarRequest {
    pub edge: ShellBarResizeEdge,
    pub height_logical: f64,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResizeShellBarResponse {
    pub edge: ShellBarResizeEdge,
    pub height_logical: f64,
}

#[derive(Default)]
pub struct ShellRuntimeState {
    pub cleaned_up: bool,
    pub hidden_explorer_taskbars: Vec<explorer::ExplorerTaskbarSnapshot>,
    pub hidden_explorer_taskbars_v2: Option<Arc<Mutex<Vec<explorer::ExplorerTaskbarSnapshot>>>>,
    pub baseline_work_area: Option<RECT>,
    pub registered_appbars: Vec<isize>,
    shell_layout: Option<ShellSurfaceLayout>,
    fullscreen_state: FullscreenAppBarState,
    fullscreen_guard_stop: Option<Arc<AtomicBool>>,
    fullscreen_guard: Option<JoinHandle<()>>,
    pub taskbar_guard_stop: Option<Arc<AtomicBool>>,
    pub taskbar_guard: Option<JoinHandle<()>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum FullscreenAppBarState {
    #[default]
    Reserved,
    Released,
    Parked,
}

#[derive(Clone, Copy, Debug)]
struct GuardRetryState {
    target: Option<FullscreenGuardTarget>,
    failures: u32,
    retry_deadline: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkAreaSyncResult {
    Ok,
    Mismatch,
    SetFailed,
    GetFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FullscreenSyncAction {
    Hide,
    Restore,
    Park,
    Noop,
}

impl Default for GuardRetryState {
    fn default() -> Self {
        Self {
            target: None,
            failures: 0,
            retry_deadline: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FullscreenGuardTarget {
    Hide,
    Restore,
}

const WORK_AREA_RETRY_ATTEMPTS: usize = 20;
const WORK_AREA_RETRY_DELAY: Duration = Duration::from_millis(50);
const STARTUP_STABILIZATION_POLLS: usize = 15;
const STARTUP_STABILIZATION_DELAY: Duration = Duration::from_millis(100);
const REQUIRED_STABLE_POLLS: usize = 3;
const FULLSCREEN_GUARD_POLL_DELAY: Duration = Duration::from_millis(250);
const FULLSCREEN_RECT_TOLERANCE: i32 = 2;
static FULLSCREEN_GUARD_WAKE_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum AppBarEdge {
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug)]
struct ShellSurfaceLayout {
    monitor_rect: RECT,
    top_hwnd: isize,
    bottom_hwnd: isize,
    top_rect: RECT,
    bottom_rect: RECT,
}

#[derive(Clone, Copy, Debug)]
struct FullscreenWindowCandidate {
    window_rect: RECT,
    monitor_rect: RECT,
    work_area_rect: RECT,
    is_shell_process: bool,
    is_desktop_shell_window: bool,
    is_visible: bool,
    is_minimized: bool,
    is_cloaked: bool,
    has_window_frame: bool,
}

impl AppBarEdge {
    fn as_u32(self) -> u32 {
        match self {
            AppBarEdge::Top => ABE_TOP,
            AppBarEdge::Bottom => ABE_BOTTOM,
        }
    }
}

pub fn activate_shell_surfaces(app: &mut App, windows: &CreatedShellWindows) -> AppResult<()> {
    let primary_monitor = app
        .primary_monitor()?
        .ok_or_else(|| "Primary monitor is unavailable".to_string())?;

    let monitor_rect = RECT {
        left: primary_monitor.position().x,
        top: primary_monitor.position().y,
        right: primary_monitor.position().x + primary_monitor.size().width as i32,
        bottom: primary_monitor.position().y + primary_monitor.size().height as i32,
    };

    let scale_factor = primary_monitor.scale_factor();
    let top_height = super::shell_windows::to_physical_height(TOP_BAR_HEIGHT_LOGICAL, scale_factor);
    let bottom_height =
        super::shell_windows::to_physical_height(BOTTOM_BAR_HEIGHT_LOGICAL, scale_factor);
    let top_rect = desired_rect_for_edge(monitor_rect, AppBarEdge::Top, top_height);
    let bottom_rect = desired_rect_for_edge(monitor_rect, AppBarEdge::Bottom, bottom_height);
    let state = app.state::<Mutex<ShellRuntimeState>>();
    let mut state = state.lock().expect("shell runtime state is poisoned");
    state.cleaned_up = false;
    state.baseline_work_area = None;
    state.hidden_explorer_taskbars.clear();
    state.hidden_explorer_taskbars_v2 = None;
    state.shell_layout = None;
    state.fullscreen_state = FullscreenAppBarState::Reserved;
    state.registered_appbars.clear();

    let activation_result = (|| -> AppResult<()> {
        let current_work_area = get_work_area()?;
        let explorer_taskbars = explorer::all_taskbar_snapshots()?;
        state.baseline_work_area = Some(resolve_baseline_work_area(
            monitor_rect,
            current_work_area,
            explorer_taskbars
                .iter()
                .copied()
                .find(|snapshot| snapshot.monitor_rect == monitor_rect),
        ));
        if taskbar_suppression_v2_enabled_from_env() {
            let owned = Arc::new(Mutex::new(explorer::hide_taskbars_if_needed(
                explorer_taskbars,
            )?));
            if !owned.lock().expect("taskbar v2 state poisoned").is_empty() {
                state.hidden_explorer_taskbars_v2 = Some(Arc::clone(&owned));
                start_taskbar_guard_v2(&mut state, owned);
                set_work_area_with_retry_or_warn(
                    monitor_rect,
                    "monitor work area while Explorer taskbar is hidden",
                )?;
            }
        } else {
            state.hidden_explorer_taskbars =
                explorer::hide_primary_taskbar_if_needed(monitor_rect)?
                    .filter(|snapshot| snapshot.hidden_by_jasonshell)
                    .into_iter()
                    .collect();

            if !state.hidden_explorer_taskbars.is_empty() {
                start_taskbar_guard(&mut state);
                set_work_area_with_retry_or_warn(
                    monitor_rect,
                    "monitor work area while Explorer taskbar is hidden",
                )?;
            }
        }

        let top_hwnd = hwnd_from_tauri_window(&windows.top)?;
        let bottom_hwnd = hwnd_from_tauri_window(&windows.bottom)?;

        let resolved_top_rect =
            register_tracked_appbar(&mut state, top_hwnd, register_appbar, |hwnd| {
                reserve_appbar(hwnd, AppBarEdge::Top, top_rect)
            })?;

        let resolved_bottom_rect =
            register_tracked_appbar(&mut state, bottom_hwnd, register_appbar, |hwnd| {
                reserve_appbar(hwnd, AppBarEdge::Bottom, bottom_rect)
            })?;

        set_work_area_with_retry_or_warn(
            reserved_work_area(monitor_rect, resolved_top_rect, resolved_bottom_rect),
            "reserved shell work area",
        )?;

        if let Some(snapshot) = state.hidden_explorer_taskbars.first().copied() {
            if !explorer::enforce_taskbar_hidden(snapshot)? {
                eprintln!(
                    "Explorer taskbar remained visible after startup retries; guard thread will continue enforcement"
                );
            }
        }

        move_window_to_rect(top_hwnd, resolved_top_rect)?;
        move_window_to_rect(bottom_hwnd, resolved_bottom_rect)?;
        state.shell_layout = Some(ShellSurfaceLayout {
            monitor_rect,
            top_hwnd: top_hwnd.0 as isize,
            bottom_hwnd: bottom_hwnd.0 as isize,
            top_rect: resolved_top_rect,
            bottom_rect: resolved_bottom_rect,
        });

        windows.top.show()?;
        windows.bottom.show()?;
        apply_no_alt_tab_shell_style_to_hwnd(top_hwnd, super::shell_windows::TOP_BAR_LABEL)?;
        apply_no_alt_tab_shell_style_to_hwnd(bottom_hwnd, super::shell_windows::BOTTOM_BAR_LABEL)?;

        stabilize_runtime_window_rect(
            top_hwnd,
            super::shell_windows::TOP_BAR_LABEL,
            resolved_top_rect,
        )?;
        stabilize_runtime_window_rect(
            bottom_hwnd,
            super::shell_windows::BOTTOM_BAR_LABEL,
            resolved_bottom_rect,
        )?;

        if let Some(snapshot) = state.hidden_explorer_taskbars.first().copied() {
            if !explorer::enforce_taskbar_hidden(snapshot)? {
                eprintln!(
                    "Explorer taskbar was still visible after shell surfaces finished showing"
                );
            }
        }

        start_fullscreen_guard(app.handle().clone(), &mut state);

        Ok(())
    })();

    if let Err(error) = activation_result {
        let _ = windows.top.hide();
        let _ = windows.bottom.hide();
        let rollback_error = cleanup_runtime_state(&mut state);

        return match rollback_error {
            Ok(()) => Err(error),
            Err(rollback_error) => Err(format!(
                "shell activation failed: {error}; rollback failed: {rollback_error}"
            )
            .into()),
        };
    }

    Ok(())
}

pub fn cleanup_shell_surfaces(app_handle: &AppHandle) -> AppResult<()> {
    let state = app_handle.state::<Mutex<ShellRuntimeState>>();
    let mut state = state.lock().expect("shell runtime state is poisoned");

    if state.cleaned_up {
        return Ok(());
    }

    cleanup_runtime_state(&mut state)
}

#[tauri::command]
pub fn resize_shell_bar(
    app_handle: AppHandle,
    request: ResizeShellBarRequest,
) -> Result<ResizeShellBarResponse, String> {
    resize_shell_bar_for_app(&app_handle, request).map_err(|error| error.to_string())
}

pub(crate) fn resize_shell_bar_for_app(
    app_handle: &AppHandle,
    request: ResizeShellBarRequest,
) -> AppResult<ResizeShellBarResponse> {
    let top_min = super::shell_windows::MIN_TOP_BAR_HEIGHT_LOGICAL;
    let bottom_min = super::shell_windows::MIN_BOTTOM_BAR_HEIGHT_LOGICAL;
    let min_height = match request.edge {
        ShellBarResizeEdge::Top => top_min,
        ShellBarResizeEdge::Bottom => bottom_min,
    };
    let height_logical =
        crate::settings::clamp_shell_bar_height_logical(request.height_logical, min_height);
    let scale_factor = app_handle
        .primary_monitor()?
        .ok_or_else(|| "Primary monitor is unavailable".to_string())?
        .scale_factor();
    let height = super::shell_windows::to_physical_height(height_logical, scale_factor);

    resize_shell_bar_runtime(app_handle, request.edge, height)?;

    Ok(ResizeShellBarResponse {
        edge: request.edge,
        height_logical,
    })
}

fn resize_shell_bar_runtime(
    app_handle: &AppHandle,
    edge: ShellBarResizeEdge,
    height: i32,
) -> AppResult<()> {
    let state = app_handle.state::<Mutex<ShellRuntimeState>>();
    let mut state = state.lock().expect("shell runtime state is poisoned");
    let layout = state
        .shell_layout
        .ok_or_else(|| "Shell AppBar layout is not active".to_string())?;
    ensure_shell_bar_resize_allowed(state.fullscreen_state)?;
    let top_hwnd = HWND(layout.top_hwnd as *mut _);
    let bottom_hwnd = HWND(layout.bottom_hwnd as *mut _);
    let mut top_rect = layout.top_rect;
    let mut bottom_rect = layout.bottom_rect;

    match edge {
        ShellBarResizeEdge::Top => {
            top_rect = desired_rect_for_edge(layout.monitor_rect, AppBarEdge::Top, height);
        }
        ShellBarResizeEdge::Bottom => {
            bottom_rect = desired_rect_for_edge(layout.monitor_rect, AppBarEdge::Bottom, height);
        }
    }

    let top_rect = reserve_appbar(top_hwnd, AppBarEdge::Top, top_rect)?;
    let bottom_rect = reserve_appbar(bottom_hwnd, AppBarEdge::Bottom, bottom_rect)?;
    set_work_area_with_retry_or_warn(
        reserved_work_area(layout.monitor_rect, top_rect, bottom_rect),
        "reserved shell work area after shell bar resize",
    )?;
    move_window_to_rect(top_hwnd, top_rect)?;
    move_window_to_rect(bottom_hwnd, bottom_rect)?;

    state.shell_layout = Some(ShellSurfaceLayout {
        top_rect,
        bottom_rect,
        ..layout
    });

    Ok(())
}

fn ensure_shell_bar_resize_allowed(fullscreen_state: FullscreenAppBarState) -> AppResult<()> {
    if fullscreen_state != FullscreenAppBarState::Reserved {
        return Err(
            "Shell AppBars are temporarily released while a fullscreen foreground app is active"
                .into(),
        );
    }

    Ok(())
}

fn restored_shell_surface_layout(
    layout: ShellSurfaceLayout,
    top_rect: RECT,
    bottom_rect: RECT,
) -> ShellSurfaceLayout {
    ShellSurfaceLayout {
        top_rect,
        bottom_rect,
        ..layout
    }
}

fn hwnd_from_tauri_window(window: &tauri::WebviewWindow) -> AppResult<HWND> {
    let handle = window.window_handle()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut _)),
        other => Err(format!("Unsupported window handle: {other:?}").into()),
    }
}

fn register_tracked_appbar<RegisterFn, ConfigureFn>(
    state: &mut ShellRuntimeState,
    hwnd: HWND,
    register: RegisterFn,
    configure: ConfigureFn,
) -> AppResult<RECT>
where
    RegisterFn: FnOnce(HWND) -> AppResult<()>,
    ConfigureFn: FnOnce(HWND) -> AppResult<RECT>,
{
    register(hwnd)?;
    state.registered_appbars.push(hwnd.0 as isize);
    configure(hwnd)
}

fn register_appbar(hwnd: HWND) -> AppResult<()> {
    let mut appbar = new_appbar_data(hwnd, RECT::default());
    let result = unsafe { SHAppBarMessage(ABM_NEW, &mut appbar) };

    if result == 0 {
        return Err(format!("ABM_NEW failed for window handle {:?}", hwnd.0).into());
    }

    Ok(())
}

fn reserve_appbar(hwnd: HWND, edge: AppBarEdge, desired_rect: RECT) -> AppResult<RECT> {
    let mut appbar = new_appbar_data(hwnd, desired_rect);
    appbar.uEdge = edge.as_u32();
    let desired_thickness = rect_thickness(desired_rect, edge);

    unsafe {
        SHAppBarMessage(ABM_QUERYPOS, &mut appbar);
    }

    apply_requested_thickness(&mut appbar.rc, edge, desired_thickness);

    let result = unsafe { SHAppBarMessage(ABM_SETPOS, &mut appbar) };

    if result == 0 {
        return Err(format!("ABM_SETPOS failed for window handle {:?}", hwnd.0).into());
    }

    Ok(normalize_rect_thickness(appbar.rc, edge, desired_thickness))
}

fn unregister_appbar(hwnd: HWND) -> AppResult<()> {
    let mut appbar = new_appbar_data(hwnd, RECT::default());
    unsafe {
        let result = SHAppBarMessage(ABM_REMOVE, &mut appbar);
        if result == 0 {
            return Err(format!("ABM_REMOVE failed for window handle {:?}", hwnd.0).into());
        }
    }

    Ok(())
}

/// Removes every tracked AppBar that Win32 accepts and retains only failed removals.
///
/// Retaining failures prevents a retry from issuing `ABM_NEW` for an HWND whose prior
/// registration may still be live.
fn unregister_tracked_appbars_with<UnregisterFn>(
    registered_appbars: &mut Vec<isize>,
    mut unregister: UnregisterFn,
) -> AppResult<()>
where
    UnregisterFn: FnMut(HWND) -> AppResult<()>,
{
    let tracked = std::mem::take(registered_appbars);
    let mut remaining = Vec::new();
    let mut errors = Vec::new();

    for hwnd_value in tracked.into_iter().rev() {
        if let Err(error) = unregister(HWND(hwnd_value as *mut _)) {
            remaining.push(hwnd_value);
            errors.push(error.to_string());
        }
    }

    remaining.reverse();
    *registered_appbars = remaining;

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "failed to remove tracked Shell AppBars: {}",
            errors.join("; ")
        )
        .into())
    }
}

fn desired_rect_for_edge(monitor_rect: RECT, edge: AppBarEdge, height: i32) -> RECT {
    match edge {
        AppBarEdge::Top => RECT {
            left: monitor_rect.left,
            top: monitor_rect.top,
            right: monitor_rect.right,
            bottom: monitor_rect.top + height,
        },
        AppBarEdge::Bottom => RECT {
            left: monitor_rect.left,
            top: monitor_rect.bottom - height,
            right: monitor_rect.right,
            bottom: monitor_rect.bottom,
        },
    }
}

fn rect_thickness(rect: RECT, edge: AppBarEdge) -> i32 {
    match edge {
        AppBarEdge::Top | AppBarEdge::Bottom => rect.bottom - rect.top,
    }
}

fn apply_requested_thickness(rect: &mut RECT, edge: AppBarEdge, thickness: i32) {
    match edge {
        AppBarEdge::Top => rect.bottom = rect.top + thickness,
        AppBarEdge::Bottom => rect.top = rect.bottom - thickness,
    }
}

fn normalize_rect_thickness(mut rect: RECT, edge: AppBarEdge, thickness: i32) -> RECT {
    apply_requested_thickness(&mut rect, edge, thickness);
    rect
}

fn new_appbar_data(hwnd: HWND, rect: RECT) -> APPBARDATA {
    APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uCallbackMessage: unsafe { RegisterWindowMessageW(w!("JasonShell.AppBar")) },
        uEdge: 0,
        rc: rect,
        lParam: LPARAM(0),
    }
}

fn reserved_work_area(monitor_rect: RECT, top_rect: RECT, bottom_rect: RECT) -> RECT {
    RECT {
        left: monitor_rect.left,
        top: top_rect.bottom,
        right: monitor_rect.right,
        bottom: bottom_rect.top,
    }
}

fn resolve_baseline_work_area(
    monitor_rect: RECT,
    current_work_area: RECT,
    taskbar_snapshot: Option<explorer::ExplorerTaskbarSnapshot>,
) -> RECT {
    if let Some(snapshot) = taskbar_snapshot {
        if let Some(taskbar_baseline) = snapshot.baseline_work_area(monitor_rect) {
            if snapshot.originally_visible || work_area_looks_dirty(current_work_area, monitor_rect)
            {
                return taskbar_baseline;
            }
        }
    }

    current_work_area
}

fn work_area_looks_dirty(current_work_area: RECT, monitor_rect: RECT) -> bool {
    current_work_area.left != monitor_rect.left
        || current_work_area.top != monitor_rect.top
        || current_work_area.right != monitor_rect.right
        || current_work_area.bottom != monitor_rect.bottom
}

fn get_work_area() -> AppResult<RECT> {
    let mut rect = RECT::default();
    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&mut rect as *mut RECT).cast::<c_void>()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )?;
    }

    Ok(rect)
}

fn set_work_area(rect: RECT) -> AppResult<()> {
    let mut rect = rect;
    unsafe {
        SystemParametersInfoW(
            SPI_SETWORKAREA,
            0,
            Some((&mut rect as *mut RECT).cast::<c_void>()),
            SPIF_SENDCHANGE,
        )?;
    }

    Ok(())
}

fn set_work_area_with_retry(rect: RECT, context: &str) -> AppResult<()> {
    for attempt in 0..WORK_AREA_RETRY_ATTEMPTS {
        set_work_area(rect)?;
        let observed = get_work_area()?;

        if rects_match(observed, rect) {
            return Ok(());
        }

        if attempt + 1 < WORK_AREA_RETRY_ATTEMPTS {
            thread::sleep(WORK_AREA_RETRY_DELAY);
        }
    }

    let observed = get_work_area()?;
    Err(format!(
        "failed to restore {context} after retries; expected {},{},{},{} but observed {},{},{},{}",
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        observed.left,
        observed.top,
        observed.right,
        observed.bottom
    )
    .into())
}

fn sync_work_area_best_effort(rect: RECT, context: &str) -> AppResult<()> {
    match sync_work_area_best_effort_with(rect, set_work_area, get_work_area) {
        WorkAreaSyncResult::SetFailed => {
            eprintln!("warning: could not set {context}; AppBar geometry remains authoritative");
        }
        WorkAreaSyncResult::GetFailed => {
            eprintln!("warning: could not read {context}; AppBar geometry remains authoritative");
        }
        WorkAreaSyncResult::Ok | WorkAreaSyncResult::Mismatch => {}
    }
    Ok(())
}

fn sync_work_area_best_effort_with<SetFn, GetFn>(
    rect: RECT,
    mut set_work_area_fn: SetFn,
    mut get_work_area_fn: GetFn,
) -> WorkAreaSyncResult
where
    SetFn: FnMut(RECT) -> AppResult<()>,
    GetFn: FnMut() -> AppResult<RECT>,
{
    match set_work_area_fn(rect) {
        Err(_) => return WorkAreaSyncResult::SetFailed,
        Ok(()) => {}
    }
    match get_work_area_fn() {
        Err(_) => WorkAreaSyncResult::GetFailed,
        Ok(observed) if !rects_match(observed, rect) => WorkAreaSyncResult::Mismatch,
        Ok(_) => WorkAreaSyncResult::Ok,
    }
}

fn fullscreen_guard_retry_delay(failures: u32) -> Duration {
    let steps = failures.saturating_sub(1).min(4);
    Duration::from_millis((250 * (1 << steps)).min(4000) as u64)
}

fn fullscreen_guard_target_or_restore(
    target: Option<FullscreenGuardTarget>,
) -> FullscreenGuardTarget {
    target.unwrap_or(FullscreenGuardTarget::Restore)
}

fn guard_retry_prepare_target(
    state: &mut GuardRetryState,
    target: FullscreenGuardTarget,
    now: Instant,
) -> bool {
    if state.target != Some(target) {
        state.target = Some(target);
        state.failures = 0;
        state.retry_deadline = None;
        return true;
    }

    match state.retry_deadline {
        Some(deadline) if now < deadline => false,
        _ => true,
    }
}

fn guard_retry_register_failure(state: &mut GuardRetryState, now: Instant) {
    state.failures = state.failures.saturating_add(1);
    state.retry_deadline = Some(now + fullscreen_guard_retry_delay(state.failures));
}

fn guard_retry_reset(state: &mut GuardRetryState) {
    state.target = None;
    state.failures = 0;
    state.retry_deadline = None;
}

fn set_work_area_with_retry_or_warn(rect: RECT, context: &str) -> AppResult<()> {
    match set_work_area_with_retry(rect, context) {
        Ok(()) => Ok(()),
        Err(error) => {
            let message = error.to_string();
            if message.contains("failed to restore") {
                eprintln!("warning: {message}; continuing with observed work area");
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}

fn move_window_to_rect(hwnd: HWND, rect: RECT) -> AppResult<()> {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;

    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            rect.left,
            rect.top,
            width,
            height,
            SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        )?;
    }

    Ok(())
}

fn virtual_desktop_rect() -> RECT {
    unsafe {
        let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
        RECT {
            left,
            top,
            right: left.saturating_add(GetSystemMetrics(SM_CXVIRTUALSCREEN)),
            bottom: top.saturating_add(GetSystemMetrics(SM_CYVIRTUALSCREEN)),
        }
    }
}

fn parked_rect_below_virtual_desktop(surface_rect: RECT, virtual_desktop: RECT) -> RECT {
    let height = (surface_rect.bottom - surface_rect.top).max(1);
    let top = virtual_desktop.bottom.saturating_add(1);

    RECT {
        left: surface_rect.left,
        top,
        right: surface_rect.right,
        bottom: top.saturating_add(height),
    }
}

/// Keeps persistent WebView2 surfaces alive during fullscreen without letting them overlap any monitor.
///
/// Unlike `move_window_to_rect`, this deliberately avoids `SWP_FRAMECHANGED`: frame/visibility churn
/// after a fullscreen app exits can leave a Tauri WebView visually blank despite correct HWND geometry.
fn park_window_below_virtual_desktop(hwnd: HWND, surface_rect: RECT) -> AppResult<()> {
    let parked = parked_rect_below_virtual_desktop(surface_rect, virtual_desktop_rect());
    let width = parked.right - parked.left;
    let height = parked.bottom - parked.top;

    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            parked.left,
            parked.top,
            width,
            height,
            SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        )?;
    }

    Ok(())
}

fn park_shell_surface_windows(layout: ShellSurfaceLayout) -> AppResult<()> {
    park_window_below_virtual_desktop(HWND(layout.top_hwnd as *mut _), layout.top_rect)?;
    park_window_below_virtual_desktop(HWND(layout.bottom_hwnd as *mut _), layout.bottom_rect)
}

fn runtime_window_rect(hwnd: HWND) -> AppResult<WindowRectSnapshot> {
    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut rect)?;
    }

    Ok(WindowRectSnapshot {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    })
}

fn stabilize_runtime_window_rect(hwnd: HWND, label: &str, expected_rect: RECT) -> AppResult<()> {
    stabilize_runtime_window_rect_with(
        label,
        expected_rect,
        || runtime_window_rect(hwnd),
        |rect| move_window_to_rect(hwnd, rect),
    )
}

fn stabilize_runtime_window_rect_with<GetRectFn, MoveFn>(
    label: &str,
    expected_rect: RECT,
    mut get_rect: GetRectFn,
    mut move_to_rect: MoveFn,
) -> AppResult<()>
where
    GetRectFn: FnMut() -> AppResult<WindowRectSnapshot>,
    MoveFn: FnMut(RECT) -> AppResult<()>,
{
    let mut stable_polls = 0;
    let mut last_rect = None;

    for _ in 0..STARTUP_STABILIZATION_POLLS {
        let rect = get_rect()?;
        let matches_expected = snapshot_matches_rect(&rect, expected_rect);
        last_rect = Some(rect.clone());

        if matches_expected {
            stable_polls += 1;

            if stable_polls >= REQUIRED_STABLE_POLLS {
                return Ok(());
            }
        } else {
            stable_polls = 0;
            eprintln!(
                "{label} runtime rect drift detected (observed: {},{},{},{} height={}; expected: {},{},{},{}) - reapplying shell rect",
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
                rect.height,
                expected_rect.left,
                expected_rect.top,
                expected_rect.right,
                expected_rect.bottom
            );
            move_to_rect(expected_rect)?;
        }

        thread::sleep(STARTUP_STABILIZATION_DELAY);
    }

    let rect = last_rect.unwrap_or(WindowRectSnapshot {
        left: expected_rect.left,
        top: expected_rect.top,
        right: expected_rect.right,
        bottom: expected_rect.bottom,
        width: expected_rect.right - expected_rect.left,
        height: expected_rect.bottom - expected_rect.top,
    });

    Err(format!(
        "{label} runtime rect failed to stabilize after show (observed: {},{},{},{} height={}; expected: {},{},{},{})",
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        rect.height,
        expected_rect.left,
        expected_rect.top,
        expected_rect.right,
        expected_rect.bottom
    )
    .into())
}

fn rects_match(left: RECT, right: RECT) -> bool {
    left.left == right.left
        && left.top == right.top
        && left.right == right.right
        && left.bottom == right.bottom
}

fn snapshot_matches_rect(snapshot: &WindowRectSnapshot, expected_rect: RECT) -> bool {
    snapshot.height > 0
        && snapshot.left == expected_rect.left
        && snapshot.top == expected_rect.top
        && snapshot.right == expected_rect.right
        && snapshot.bottom == expected_rect.bottom
}

pub fn capture_shell_surface_runtime_metrics(
    app_handle: &AppHandle,
    frontend: FrontendSurfaceMetrics,
) -> AppResult<ShellSurfaceRuntimeMetrics> {
    let window = app_handle
        .get_webview_window(&frontend.label)
        .ok_or_else(|| format!("Unknown shell surface '{}'", frontend.label))?;
    let native_rect = runtime_window_rect(hwnd_from_tauri_window(&window)?)?;
    let metrics = ShellSurfaceRuntimeMetrics {
        label: frontend.label,
        native_height_ok: native_rect.height > 0,
        webview_height_ok: frontend.outer_height > 0
            && frontend.inner_height > 0
            && frontend.client_height > 0,
        native_rect,
        outer_height: frontend.outer_height,
        inner_height: frontend.inner_height,
        client_height: frontend.client_height,
    };

    eprintln!(
        "runtime shell metrics label={} native_rect={},{},{},{} native_height={} outer_height={} inner_height={} client_height={} native_height_ok={} webview_height_ok={}",
        metrics.label,
        metrics.native_rect.left,
        metrics.native_rect.top,
        metrics.native_rect.right,
        metrics.native_rect.bottom,
        metrics.native_rect.height,
        metrics.outer_height,
        metrics.inner_height,
        metrics.client_height,
        metrics.native_height_ok,
        metrics.webview_height_ok
    );

    Ok(metrics)
}

fn cleanup_runtime_state(state: &mut ShellRuntimeState) -> AppResult<()> {
    stop_fullscreen_guard(state);
    stop_taskbar_guard(state);

    cleanup_runtime_state_with(
        state,
        unregister_appbar,
        set_work_area_with_retry_or_warn,
        |snapshot| explorer::restore_taskbar(snapshot).map_err(Into::into),
    )
}

fn cleanup_runtime_state_with<UnregisterFn, RestoreWorkAreaFn, RestoreTaskbarFn>(
    state: &mut ShellRuntimeState,
    mut unregister: UnregisterFn,
    mut restore_work_area: RestoreWorkAreaFn,
    mut restore_taskbar: RestoreTaskbarFn,
) -> AppResult<()>
where
    UnregisterFn: FnMut(HWND) -> AppResult<()>,
    RestoreWorkAreaFn: FnMut(RECT, &str) -> AppResult<()>,
    RestoreTaskbarFn: FnMut(explorer::ExplorerTaskbarSnapshot) -> AppResult<bool>,
{
    let mut cleanup_errors = Vec::new();

    if let Err(error) =
        unregister_tracked_appbars_with(&mut state.registered_appbars, &mut unregister)
    {
        cleanup_errors.push(error.to_string());
    }

    let hidden_taskbars = if let Some(v2) = state.hidden_explorer_taskbars_v2.take() {
        v2.lock().expect("taskbar v2 state poisoned").clone()
    } else {
        state.hidden_explorer_taskbars.drain(..).collect()
    };
    for taskbar_snapshot in hidden_taskbars {
        match restore_taskbar(taskbar_snapshot) {
            Ok(true) => {}
            Ok(false) => cleanup_errors.push(
                "Explorer taskbar remained hidden after repeated restore attempts".to_string(),
            ),
            Err(error) => cleanup_errors.push(error.to_string()),
        }
    }

    if let Some(baseline_work_area) = state.baseline_work_area.take() {
        if let Err(error) = restore_work_area(
            baseline_work_area,
            "baseline shell work area during cleanup",
        ) {
            cleanup_errors.push(error.to_string());
        }
    }

    state.cleaned_up = true;
    state.fullscreen_state = FullscreenAppBarState::Reserved;
    state.shell_layout = None;

    if cleanup_errors.is_empty() {
        Ok(())
    } else {
        Err(cleanup_errors.join("; ").into())
    }
}

fn start_fullscreen_guard(app_handle: AppHandle, state: &mut ShellRuntimeState) {
    stop_fullscreen_guard(state);
    FULLSCREEN_GUARD_WAKE_COUNT.store(0, Ordering::Relaxed);

    let stop = Arc::new(AtomicBool::new(false));
    let stop_signal = Arc::clone(&stop);
    let guard = thread::spawn(move || {
        let guard_started_at = Instant::now();
        let mut retry_state = GuardRetryState::default();
        while !stop_signal.load(Ordering::Relaxed) {
            FULLSCREEN_GUARD_WAKE_COUNT.fetch_add(1, Ordering::Relaxed);
            let now = Instant::now();
            if let Some(target) = foreground_fullscreen_target_for_layout_with_state(&app_handle) {
                if guard_retry_prepare_target(&mut retry_state, target, now) {
                    let sync_result =
                        sync_fullscreen_shell_surfaces_for_target(&app_handle, target);
                    if let Err(error) = sync_result {
                        if retry_state.failures == 0 {
                            eprintln!("fullscreen shell sync failed: {error}");
                        }
                        guard_retry_register_failure(&mut retry_state, now);
                    } else {
                        guard_retry_reset(&mut retry_state);
                    }
                }
            }
            thread::sleep(FULLSCREEN_GUARD_POLL_DELAY);
        }
        eprintln!(
            "fullscreen guard summary duration_ms={} wake_count={}",
            guard_started_at.elapsed().as_millis(),
            FULLSCREEN_GUARD_WAKE_COUNT.load(Ordering::Relaxed)
        );
    });

    state.fullscreen_guard_stop = Some(stop);
    state.fullscreen_guard = Some(guard);
}

fn stop_fullscreen_guard(state: &mut ShellRuntimeState) {
    if let Some(stop) = state.fullscreen_guard_stop.take() {
        stop.store(true, Ordering::Relaxed);
    }

    if let Some(guard) = state.fullscreen_guard.take() {
        let _ = guard.join();
    }
}

fn sync_fullscreen_shell_surfaces_for_target(
    app_handle: &AppHandle,
    target: FullscreenGuardTarget,
) -> AppResult<()> {
    let (cleaned_up, state_value) = {
        let state = app_handle.state::<Mutex<ShellRuntimeState>>();
        let Ok(state) = state.try_lock() else {
            return Ok(());
        };
        (state.cleaned_up, state.fullscreen_state)
    };

    if cleaned_up {
        return Ok(());
    }

    match fullscreen_sync_action_for_state(state_value, target) {
        FullscreenSyncAction::Hide | FullscreenSyncAction::Park => {
            hide_shell_for_fullscreen(app_handle)
        }
        FullscreenSyncAction::Restore => restore_shell_after_fullscreen(app_handle),
        FullscreenSyncAction::Noop => Ok(()),
    }
}

fn fullscreen_sync_action_for_state(
    state: FullscreenAppBarState,
    target: FullscreenGuardTarget,
) -> FullscreenSyncAction {
    match (state, target) {
        (FullscreenAppBarState::Reserved, FullscreenGuardTarget::Hide) => {
            FullscreenSyncAction::Hide
        }
        (FullscreenAppBarState::Reserved, FullscreenGuardTarget::Restore) => {
            FullscreenSyncAction::Noop
        }
        (FullscreenAppBarState::Released, FullscreenGuardTarget::Hide) => {
            FullscreenSyncAction::Park
        }
        (FullscreenAppBarState::Released, FullscreenGuardTarget::Restore) => {
            FullscreenSyncAction::Restore
        }
        (FullscreenAppBarState::Parked, FullscreenGuardTarget::Hide) => FullscreenSyncAction::Noop,
        (FullscreenAppBarState::Parked, FullscreenGuardTarget::Restore) => {
            FullscreenSyncAction::Restore
        }
    }
}

fn foreground_fullscreen_target_for_layout_with_state(
    app_handle: &AppHandle,
) -> Option<FullscreenGuardTarget> {
    let state = app_handle.state::<Mutex<ShellRuntimeState>>();
    let Ok(state) = state.try_lock() else {
        return None;
    };
    let Some(layout) = state.shell_layout else {
        return None;
    };
    let work_area_rect = state.baseline_work_area.unwrap_or(layout.monitor_rect);
    Some(fullscreen_guard_target_or_restore(
        foreground_fullscreen_target_for_layout(layout, work_area_rect, std::process::id()),
    ))
}

fn foreground_fullscreen_target_for_layout(
    layout: ShellSurfaceLayout,
    work_area_rect: RECT,
    current_process_id: u32,
) -> Option<FullscreenGuardTarget> {
    foreground_fullscreen_candidate(layout.monitor_rect, work_area_rect, current_process_id).map(
        |candidate| {
            if should_hide_shell_for_fullscreen_window(candidate) {
                FullscreenGuardTarget::Hide
            } else {
                FullscreenGuardTarget::Restore
            }
        },
    )
}

fn hide_shell_for_fullscreen(app_handle: &AppHandle) -> AppResult<()> {
    let state = app_handle.state::<Mutex<ShellRuntimeState>>();
    let Ok(mut state) = state.try_lock() else {
        return Ok(());
    };
    if state.cleaned_up {
        return Ok(());
    }

    let Some(layout) = state.shell_layout else {
        return Ok(());
    };

    match state.fullscreen_state {
        FullscreenAppBarState::Reserved => {}
        FullscreenAppBarState::Released => {}
        FullscreenAppBarState::Parked => return Ok(()),
    }

    let mut release_errors = Vec::new();
    if let Err(error) =
        unregister_tracked_appbars_with(&mut state.registered_appbars, unregister_appbar)
    {
        release_errors.push(error.to_string());
    }

    if !release_errors.is_empty() {
        return Err(release_errors.join("; ").into());
    }

    state.fullscreen_state = FullscreenAppBarState::Released;
    let _ = sync_work_area_best_effort_with(layout.monitor_rect, set_work_area, get_work_area);
    // Keep persistent WebView2 surfaces alive. Tauri hide/show can leave a surface blank
    // after fullscreen exits even when its HWND geometry is correct.
    park_shell_surface_windows(layout)?;
    state.fullscreen_state = FullscreenAppBarState::Parked;
    Ok(())
}

fn prepare_fullscreen_restore_retry_with<UnregisterFn>(
    registered_appbars: &mut Vec<isize>,
    unregister: UnregisterFn,
) -> AppResult<()>
where
    UnregisterFn: FnMut(HWND) -> AppResult<()>,
{
    unregister_tracked_appbars_with(registered_appbars, unregister).map_err(|error| {
        format!(
            "cannot retry fullscreen shell restore until prior AppBar registrations are released: {error}"
        )
        .into()
    })
}

fn restore_shell_after_fullscreen(app_handle: &AppHandle) -> AppResult<()> {
    let restored_layout = {
        let state = app_handle.state::<Mutex<ShellRuntimeState>>();
        let Ok(mut state) = state.try_lock() else {
            return Ok(());
        };
        if state.cleaned_up || state.fullscreen_state == FullscreenAppBarState::Reserved {
            return Ok(());
        }

        let Some(layout) = state.shell_layout else {
            return Ok(());
        };

        // A failed restore can leave registrations tracked. They must be removed
        // successfully before a retry can issue a new ABM_NEW for either HWND.
        if !state.registered_appbars.is_empty() {
            prepare_fullscreen_restore_retry_with(
                &mut state.registered_appbars,
                unregister_appbar,
            )?;
            let _ = sync_work_area_best_effort(
                layout.monitor_rect,
                "full-monitor work area before retrying fullscreen shell restore",
            );
        }

        let top_hwnd = HWND(layout.top_hwnd as *mut _);
        let bottom_hwnd = HWND(layout.bottom_hwnd as *mut _);

        let restore_result = (|| -> AppResult<ShellSurfaceLayout> {
            let top_rect =
                register_tracked_appbar(&mut state, top_hwnd, register_appbar, |hwnd| {
                    reserve_appbar(hwnd, AppBarEdge::Top, layout.top_rect)
                })?;
            let bottom_rect =
                register_tracked_appbar(&mut state, bottom_hwnd, register_appbar, |hwnd| {
                    reserve_appbar(hwnd, AppBarEdge::Bottom, layout.bottom_rect)
                })?;
            let _ = sync_work_area_best_effort(
                reserved_work_area(layout.monitor_rect, top_rect, bottom_rect),
                "reserved shell work area after fullscreen foreground app exits",
            );

            Ok(restored_shell_surface_layout(layout, top_rect, bottom_rect))
        })();

        match restore_result {
            Ok(restored_layout) => restored_layout,
            Err(error) => {
                let mut cleanup_errors = Vec::new();
                if let Err(park_error) = park_shell_surface_windows(layout) {
                    cleanup_errors.push(format!(
                        "failed to park released shell surfaces: {park_error}"
                    ));
                }
                if let Err(release_error) = unregister_tracked_appbars_with(
                    &mut state.registered_appbars,
                    unregister_appbar,
                ) {
                    cleanup_errors.push(format!(
                        "cannot retry fullscreen shell restore until partial AppBar registrations are released: {release_error}"
                    ));
                } else {
                    let _ = sync_work_area_best_effort(
                        layout.monitor_rect,
                        "full-monitor work area after failed fullscreen shell restore",
                    );
                }

                if cleanup_errors.is_empty() {
                    return Err(error);
                }
                return Err(format!("{error}; {}", cleanup_errors.join("; ")).into());
            }
        }
    };

    let top_hwnd = HWND(restored_layout.top_hwnd as *mut _);
    let bottom_hwnd = HWND(restored_layout.bottom_hwnd as *mut _);
    let position_result = (|| -> AppResult<()> {
        move_window_to_rect(top_hwnd, restored_layout.top_rect)?;
        move_window_to_rect(bottom_hwnd, restored_layout.bottom_rect)?;

        stabilize_runtime_window_rect(
            top_hwnd,
            super::shell_windows::TOP_BAR_LABEL,
            restored_layout.top_rect,
        )?;
        stabilize_runtime_window_rect(
            bottom_hwnd,
            super::shell_windows::BOTTOM_BAR_LABEL,
            restored_layout.bottom_rect,
        )?;
        Ok(())
    })();

    match position_result {
        Ok(()) => {
            let state = app_handle.state::<Mutex<ShellRuntimeState>>();
            let mut state = state.lock().expect("shell runtime state is poisoned");
            if state.cleaned_up {
                return Ok(());
            }

            state.shell_layout = Some(restored_layout);
            state.fullscreen_state = FullscreenAppBarState::Reserved;
            Ok(())
        }
        Err(error) => {
            let mut cleanup_errors = Vec::new();
            if let Err(park_error) = park_shell_surface_windows(restored_layout) {
                cleanup_errors.push(format!(
                    "failed to park released shell surfaces: {park_error}"
                ));
            }

            let state = app_handle.state::<Mutex<ShellRuntimeState>>();
            let mut state = state.lock().expect("shell runtime state is poisoned");
            if !state.cleaned_up {
                match unregister_tracked_appbars_with(
                    &mut state.registered_appbars,
                    unregister_appbar,
                ) {
                    Ok(()) => {
                        let _ = sync_work_area_best_effort(
                            restored_layout.monitor_rect,
                            "full-monitor work area after failed fullscreen shell surface placement",
                        );
                    }
                    Err(release_error) => cleanup_errors.push(format!(
                        "cannot retry fullscreen shell restore until partial AppBar registrations are released: {release_error}"
                    )),
                }
            }

            if cleanup_errors.is_empty() {
                Err(error)
            } else {
                Err(format!(
                    "{error}; additionally failed to release fullscreen shell restore state: {}",
                    cleanup_errors.join("; ")
                )
                .into())
            }
        }
    }
}

fn foreground_fullscreen_candidate(
    monitor_rect: RECT,
    work_area_rect: RECT,
    current_process_id: u32,
) -> Option<FullscreenWindowCandidate> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }

    let mut process_id = 0;
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    if process_id == 0 {
        return None;
    }

    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 };
    let has_window_frame = (style & WS_CAPTION.0) != 0 || (style & WS_THICKFRAME.0) != 0;
    let window_rect = window_bounds_for_fullscreen(hwnd)?;

    Some(FullscreenWindowCandidate {
        window_rect,
        monitor_rect,
        work_area_rect,
        is_shell_process: process_id == current_process_id,
        is_desktop_shell_window: is_desktop_shell_class(&window_class_name(hwnd)),
        is_visible: unsafe { IsWindowVisible(hwnd).as_bool() },
        is_minimized: unsafe { IsIconic(hwnd).as_bool() },
        is_cloaked: is_window_cloaked(hwnd),
        has_window_frame,
    })
}

fn should_hide_shell_for_fullscreen_window(candidate: FullscreenWindowCandidate) -> bool {
    if candidate.is_shell_process
        || candidate.is_desktop_shell_window
        || !candidate.is_visible
        || candidate.is_minimized
        || candidate.is_cloaked
    {
        return false;
    }

    rect_covers_target(candidate.window_rect, candidate.monitor_rect)
        || (!candidate.has_window_frame
            && rect_covers_target(candidate.window_rect, candidate.work_area_rect))
}

fn rect_covers_target(window_rect: RECT, target_rect: RECT) -> bool {
    rect_has_area(window_rect)
        && rect_has_area(target_rect)
        && window_rect.left <= target_rect.left + FULLSCREEN_RECT_TOLERANCE
        && window_rect.top <= target_rect.top + FULLSCREEN_RECT_TOLERANCE
        && window_rect.right >= target_rect.right - FULLSCREEN_RECT_TOLERANCE
        && window_rect.bottom >= target_rect.bottom - FULLSCREEN_RECT_TOLERANCE
}

fn is_desktop_shell_class(class_name: &str) -> bool {
    matches!(
        class_name.to_ascii_lowercase().as_str(),
        "progman" | "workerw" | "shelldll_defview"
    )
}

fn window_class_name(hwnd: HWND) -> String {
    let mut buffer = [0_u16; 256];
    let length = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if length <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..length as usize])
}

fn window_bounds_for_fullscreen(hwnd: HWND) -> Option<RECT> {
    extended_frame_bounds(hwnd).or_else(|| {
        let mut rect = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut rect).ok()? };
        rect_has_area(rect).then_some(rect)
    })
}

fn extended_frame_bounds(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    let result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut rect as *mut RECT).cast(),
            size_of::<RECT>() as u32,
        )
    };

    result.ok().filter(|_| rect_has_area(rect)).map(|_| rect)
}

fn is_window_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0_u32;
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast(),
            size_of::<u32>() as u32,
        )
    }
    .is_ok()
        && cloaked != 0
}

fn rect_has_area(rect: RECT) -> bool {
    rect.right > rect.left && rect.bottom > rect.top
}

fn start_taskbar_guard(state: &mut ShellRuntimeState) {
    stop_taskbar_guard(state);

    let Some(snapshot) = state.hidden_explorer_taskbars.first().copied() else {
        return;
    };

    let stop = Arc::new(AtomicBool::new(false));
    let stop_signal = Arc::clone(&stop);
    let guard = thread::spawn(move || {
        while !stop_signal.load(Ordering::Relaxed) {
            let _ = explorer::enforce_hidden_taskbars(&[snapshot]);
            thread::sleep(Duration::from_millis(100));
        }
    });

    state.taskbar_guard_stop = Some(stop);
    state.taskbar_guard = Some(guard);
}

fn start_taskbar_guard_v2(
    state: &mut ShellRuntimeState,
    owned: Arc<Mutex<Vec<explorer::ExplorerTaskbarSnapshot>>>,
) {
    stop_taskbar_guard(state);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_signal = Arc::clone(&stop);
    let guard = thread::spawn(move || {
        let mut generation = 0;
        while !stop_signal.load(Ordering::Relaxed) {
            generation = explorer::wait_for_taskbar_reconcile(generation);
            if stop_signal.load(Ordering::Relaxed) {
                break;
            }
            if let Ok(mut snapshots) = owned.lock() {
                let _ = explorer::reconcile_owned_taskbars(&mut snapshots);
            }
        }
    });
    state.taskbar_guard_stop = Some(stop);
    state.taskbar_guard = Some(guard);
}

fn taskbar_suppression_v2_enabled_from_env() -> bool {
    matches!(std::env::var("JASONSHELL_EXPLORER_SUPPRESSION_V2"), Ok(value) if value == "1")
}

fn stop_taskbar_guard(state: &mut ShellRuntimeState) {
    if let Some(stop) = state.taskbar_guard_stop.take() {
        stop.store(true, Ordering::Relaxed);
        explorer::request_taskbar_reconcile();
    }

    if let Some(guard) = state.taskbar_guard.take() {
        let _ = guard.join();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_requested_thickness, cleanup_runtime_state_with, ensure_shell_bar_resize_allowed,
        fullscreen_guard_retry_delay, fullscreen_guard_target_or_restore,
        fullscreen_sync_action_for_state, guard_retry_prepare_target, guard_retry_register_failure,
        guard_retry_reset, normalize_rect_thickness, parked_rect_below_virtual_desktop,
        prepare_fullscreen_restore_retry_with, rect_covers_target, register_tracked_appbar,
        reserved_work_area, resolve_baseline_work_area, restored_shell_surface_layout,
        should_hide_shell_for_fullscreen_window, stabilize_runtime_window_rect_with,
        sync_work_area_best_effort_with, unregister_tracked_appbars_with, FullscreenAppBarState,
        FullscreenGuardTarget, FullscreenSyncAction, FullscreenWindowCandidate, GuardRetryState,
        ShellRuntimeState, ShellSurfaceLayout, WindowRectSnapshot, WorkAreaSyncResult,
    };
    use crate::explorer::ExplorerTaskbarSnapshot;
    use std::time::Duration;
    use windows::Win32::Foundation::{HWND, RECT};

    #[test]
    fn reserved_work_area_uses_actual_reserved_edges() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let top = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 28,
        };
        let bottom = RECT {
            left: 0,
            top: 1032,
            right: 1920,
            bottom: 1080,
        };

        assert_eq!(
            reserved_work_area(monitor, top, bottom),
            RECT {
                left: 0,
                top: 28,
                right: 1920,
                bottom: 1032,
            }
        );
    }

    #[test]
    fn rollback_removes_appbar_registered_before_later_setup_failure() {
        let baseline_work_area = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let hidden_taskbar = ExplorerTaskbarSnapshot {
            identity: crate::explorer::ExplorerTaskbarIdentity {
                hwnd: 44,
                process_id: 12,
                class_name: crate::explorer::ExplorerTaskbarClass::Primary,
            },
            monitor_rect: baseline_work_area,
            taskbar_rect: RECT {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
            edge: crate::explorer::TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: true,
        };
        let mut state = ShellRuntimeState {
            cleaned_up: false,
            hidden_explorer_taskbars: vec![hidden_taskbar],
            hidden_explorer_taskbars_v2: None,
            baseline_work_area: Some(baseline_work_area),
            registered_appbars: Vec::new(),
            shell_layout: None,
            fullscreen_state: FullscreenAppBarState::Reserved,
            fullscreen_guard_stop: None,
            fullscreen_guard: None,
            taskbar_guard_stop: None,
            taskbar_guard: None,
        };
        let hwnd = HWND(77 as *mut _);

        let register_result = register_tracked_appbar(
            &mut state,
            hwnd,
            |_| Ok(()),
            |_| Err("ABM_SETPOS failed after ABM_NEW".into()),
        );

        assert!(register_result.is_err());
        assert_eq!(state.registered_appbars, vec![hwnd.0 as isize]);

        let mut removed_appbars = Vec::new();
        let mut restored_work_areas = Vec::new();
        let mut restored_taskbars = Vec::new();

        cleanup_runtime_state_with(
            &mut state,
            |hwnd| {
                removed_appbars.push(hwnd.0 as isize);
                Ok(())
            },
            |rect, _context| {
                restored_work_areas.push(rect);
                Ok(())
            },
            |snapshot| {
                restored_taskbars.push(snapshot.identity.hwnd);
                Ok(true)
            },
        )
        .expect("rollback should clean tracked partial registrations");

        assert_eq!(removed_appbars, vec![hwnd.0 as isize]);
        assert_eq!(restored_work_areas, vec![baseline_work_area]);
        assert_eq!(restored_taskbars, vec![hidden_taskbar.identity.hwnd]);
        assert!(state.registered_appbars.is_empty());
        assert!(state.baseline_work_area.is_none());
        assert!(state.hidden_explorer_taskbars.is_empty());
        assert!(state.cleaned_up);
    }

    #[test]
    fn failed_appbar_removal_remains_tracked_and_blocks_fresh_restore_registration() {
        let mut tracked_appbars = vec![11, 22];
        let mut removal_attempts = Vec::new();
        let mut fresh_registrations = 0;

        let retry_result = prepare_fullscreen_restore_retry_with(&mut tracked_appbars, |hwnd| {
            removal_attempts.push(hwnd.0 as isize);
            if hwnd.0 as isize == 22 {
                Err("ABM_REMOVE failed for bottom bar".into())
            } else {
                Ok(())
            }
        });

        if retry_result.is_ok() {
            fresh_registrations += 1;
        }

        assert!(retry_result.is_err());
        assert_eq!(removal_attempts, vec![22, 11]);
        assert_eq!(tracked_appbars, vec![22]);
        assert_eq!(fresh_registrations, 0);
    }

    #[test]
    fn successful_appbar_removal_clears_all_tracking() {
        let mut tracked_appbars = vec![11, 22];
        let mut removal_attempts = Vec::new();

        unregister_tracked_appbars_with(&mut tracked_appbars, |hwnd| {
            removal_attempts.push(hwnd.0 as isize);
            Ok(())
        })
        .expect("successful AppBar removals should clear tracking");

        assert_eq!(removal_attempts, vec![22, 11]);
        assert!(tracked_appbars.is_empty());
    }

    #[test]
    fn visible_primary_taskbar_overrides_dirty_work_area_baseline() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let dirty_work_area = RECT {
            left: 0,
            top: 28,
            right: 1920,
            bottom: 1032,
        };
        let taskbar_snapshot = ExplorerTaskbarSnapshot {
            identity: crate::explorer::ExplorerTaskbarIdentity {
                hwnd: 44,
                process_id: 12,
                class_name: crate::explorer::ExplorerTaskbarClass::Primary,
            },
            monitor_rect: monitor,
            taskbar_rect: RECT {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
            edge: crate::explorer::TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: true,
        };

        assert_eq!(
            resolve_baseline_work_area(monitor, dirty_work_area, Some(taskbar_snapshot)),
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1032,
            }
        );
    }

    #[test]
    fn hidden_primary_taskbar_with_dirty_work_area_recovers_taskbar_baseline() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let dirty_work_area = RECT {
            left: 0,
            top: 28,
            right: 1920,
            bottom: 1032,
        };
        let taskbar_snapshot = ExplorerTaskbarSnapshot {
            identity: crate::explorer::ExplorerTaskbarIdentity {
                hwnd: 44,
                process_id: 12,
                class_name: crate::explorer::ExplorerTaskbarClass::Primary,
            },
            monitor_rect: monitor,
            taskbar_rect: RECT {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
            edge: crate::explorer::TaskbarEdge::Bottom,
            originally_visible: false,
            hidden_by_jasonshell: false,
        };

        assert_eq!(
            resolve_baseline_work_area(monitor, dirty_work_area, Some(taskbar_snapshot)),
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1032,
            }
        );
    }

    #[test]
    fn startup_stabilization_reapplies_rect_after_zero_height_poll() {
        let expected = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 28,
        };
        let mut observed_rects = vec![
            WindowRectSnapshot {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 0,
                width: 1920,
                height: 0,
            },
            WindowRectSnapshot {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 28,
                width: 1920,
                height: 28,
            },
            WindowRectSnapshot {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 28,
                width: 1920,
                height: 28,
            },
            WindowRectSnapshot {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 28,
                width: 1920,
                height: 28,
            },
        ]
        .into_iter();
        let mut repositions = Vec::new();

        stabilize_runtime_window_rect_with(
            "top-bar",
            expected,
            || {
                Ok(observed_rects
                    .next()
                    .expect("test must provide enough rect polls"))
            },
            |rect| {
                repositions.push(rect);
                Ok(())
            },
        )
        .expect("runtime stabilization should recover from a zero-height startup rect");

        assert_eq!(repositions, vec![expected]);
    }

    #[test]
    fn parked_rect_stays_below_a_negative_coordinate_virtual_desktop() {
        let virtual_desktop = RECT {
            left: -1920,
            top: -1080,
            right: 2560,
            bottom: 1440,
        };
        let bottom_bar = RECT {
            left: 0,
            top: 1408,
            right: 2560,
            bottom: 1440,
        };

        assert_eq!(
            parked_rect_below_virtual_desktop(bottom_bar, virtual_desktop),
            RECT {
                left: 0,
                top: 1441,
                right: 2560,
                bottom: 1473,
            }
        );
    }

    #[test]
    fn fullscreen_restore_layout_uses_newly_negotiated_bottom_rect() {
        let previous_layout = ShellSurfaceLayout {
            monitor_rect: RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            },
            top_hwnd: 11,
            bottom_hwnd: 22,
            top_rect: RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 28,
            },
            bottom_rect: RECT {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
        };
        let negotiated_top_rect = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 30,
        };
        let negotiated_bottom_rect = RECT {
            left: 0,
            top: 1026,
            right: 1920,
            bottom: 1080,
        };

        let restored_layout = restored_shell_surface_layout(
            previous_layout,
            negotiated_top_rect,
            negotiated_bottom_rect,
        );

        assert_eq!(restored_layout.top_rect, negotiated_top_rect);
        assert_eq!(restored_layout.bottom_rect, negotiated_bottom_rect);
        assert_eq!(
            reserved_work_area(
                restored_layout.monitor_rect,
                restored_layout.top_rect,
                restored_layout.bottom_rect,
            ),
            RECT {
                left: 0,
                top: 30,
                right: 1920,
                bottom: 1026,
            }
        );
        assert_eq!(restored_layout.monitor_rect, previous_layout.monitor_rect);
        assert_eq!(restored_layout.top_hwnd, previous_layout.top_hwnd);
        assert_eq!(restored_layout.bottom_hwnd, previous_layout.bottom_hwnd);
    }

    #[test]
    fn shell_bar_resize_is_rejected_while_fullscreen_releases_appbars() {
        assert!(ensure_shell_bar_resize_allowed(FullscreenAppBarState::Reserved).is_ok());
        assert_eq!(
            ensure_shell_bar_resize_allowed(FullscreenAppBarState::Released)
                .expect_err("fullscreen-released AppBars must reject resize")
                .to_string(),
            "Shell AppBars are temporarily released while a fullscreen foreground app is active"
        );
        assert!(ensure_shell_bar_resize_allowed(FullscreenAppBarState::Parked).is_err());
    }

    #[test]
    fn guard_retry_delay_caps_at_four_seconds() {
        assert_eq!(fullscreen_guard_retry_delay(0), Duration::from_millis(250));
        assert_eq!(fullscreen_guard_retry_delay(1), Duration::from_millis(250));
        assert_eq!(fullscreen_guard_retry_delay(2), Duration::from_millis(500));
        assert_eq!(fullscreen_guard_retry_delay(3), Duration::from_millis(1000));
        assert_eq!(fullscreen_guard_retry_delay(4), Duration::from_millis(2000));
        assert_eq!(
            fullscreen_guard_retry_delay(99),
            Duration::from_millis(4000)
        );
    }

    #[test]
    fn released_state_hides_via_park_action_and_restore_is_eligible_from_released() {
        assert_eq!(
            fullscreen_sync_action_for_state(
                FullscreenAppBarState::Released,
                FullscreenGuardTarget::Hide
            ),
            FullscreenSyncAction::Park
        );
        assert_eq!(
            fullscreen_sync_action_for_state(
                FullscreenAppBarState::Released,
                FullscreenGuardTarget::Restore
            ),
            FullscreenSyncAction::Restore
        );
    }

    #[test]
    fn work_area_sync_reports_diagnostics_without_failing_transition() {
        let target = RECT {
            left: 0,
            top: 10,
            right: 100,
            bottom: 90,
        };
        assert_eq!(
            sync_work_area_best_effort_with(target, |_| Err("set fail".into()), || Ok(target)),
            WorkAreaSyncResult::SetFailed
        );
        assert_eq!(
            sync_work_area_best_effort_with(target, |_| Ok(()), || Err("get fail".into())),
            WorkAreaSyncResult::GetFailed
        );
        assert_eq!(
            sync_work_area_best_effort_with(
                target,
                |_| Ok(()),
                || Ok(RECT {
                    left: 0,
                    top: 0,
                    right: 100,
                    bottom: 100
                })
            ),
            WorkAreaSyncResult::Mismatch
        );
    }

    #[test]
    fn best_effort_work_area_sync_does_not_fail_on_mismatch() {
        let target = RECT {
            left: 0,
            top: 10,
            right: 100,
            bottom: 90,
        };
        let mut set_calls = 0;
        let result = sync_work_area_best_effort_with(
            target,
            |_| {
                set_calls += 1;
                Ok(())
            },
            || {
                Ok(RECT {
                    left: 0,
                    top: 0,
                    right: 100,
                    bottom: 100,
                })
            },
        );
        assert_eq!(result, WorkAreaSyncResult::Mismatch);
        assert_eq!(set_calls, 1);
    }

    #[test]
    fn guard_retry_skips_until_deadline_then_retries_and_target_change_bypasses() {
        let now = std::time::Instant::now();
        let later = now + Duration::from_millis(300);
        let mut retry = GuardRetryState::default();
        assert!(guard_retry_prepare_target(
            &mut retry,
            FullscreenGuardTarget::Hide,
            now
        ));
        retry.failures = 1;
        retry.retry_deadline = Some(later);
        assert!(!guard_retry_prepare_target(
            &mut retry,
            FullscreenGuardTarget::Hide,
            now
        ));
        assert!(guard_retry_prepare_target(
            &mut retry,
            FullscreenGuardTarget::Hide,
            later
        ));
        assert!(guard_retry_prepare_target(
            &mut retry,
            FullscreenGuardTarget::Restore,
            now
        ));
    }

    #[test]
    fn guard_retry_success_and_failure_state_updates_are_deterministic() {
        let now = std::time::Instant::now();
        let mut retry = GuardRetryState::default();
        guard_retry_register_failure(&mut retry, now);
        assert_eq!(retry.target, None);
        assert_eq!(retry.failures, 1);
        assert_eq!(retry.retry_deadline, Some(now + Duration::from_millis(250)));
        guard_retry_reset(&mut retry);
        assert_eq!(retry.target, None);
        assert_eq!(retry.failures, 0);
        assert_eq!(retry.retry_deadline, None);
    }

    #[test]
    fn foreground_candidate_absent_or_non_hide_restores_shell() {
        assert_eq!(
            fullscreen_guard_target_or_restore(None),
            FullscreenGuardTarget::Restore
        );
    }

    #[test]
    fn top_appbar_keeps_requested_height_after_query_offset() {
        let mut rect = RECT {
            left: 0,
            top: 28,
            right: 1920,
            bottom: 28,
        };

        apply_requested_thickness(&mut rect, super::AppBarEdge::Top, 28);

        assert_eq!(
            rect,
            RECT {
                left: 0,
                top: 28,
                right: 1920,
                bottom: 56,
            }
        );
    }

    #[test]
    fn normalized_bottom_appbar_keeps_requested_height() {
        let rect = normalize_rect_thickness(
            RECT {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
            super::AppBarEdge::Bottom,
            48,
        );

        assert_eq!(rect.top, 1032);
        assert_eq!(rect.bottom, 1080);
    }

    #[test]
    fn fullscreen_candidate_covering_monitor_hides_shell() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let work_area = RECT {
            left: 0,
            top: 26,
            right: 1920,
            bottom: 1044,
        };

        assert!(should_hide_shell_for_fullscreen_window(
            FullscreenWindowCandidate {
                window_rect: monitor,
                monitor_rect: monitor,
                work_area_rect: work_area,
                is_shell_process: false,
                is_desktop_shell_window: false,
                is_visible: true,
                is_minimized: false,
                is_cloaked: false,
                has_window_frame: true,
            }
        ));
    }

    #[test]
    fn framed_work_area_candidate_does_not_hide_shell() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let work_area = RECT {
            left: 0,
            top: 26,
            right: 1920,
            bottom: 1044,
        };

        assert!(!should_hide_shell_for_fullscreen_window(
            FullscreenWindowCandidate {
                window_rect: work_area,
                monitor_rect: monitor,
                work_area_rect: work_area,
                is_shell_process: false,
                is_desktop_shell_window: false,
                is_visible: true,
                is_minimized: false,
                is_cloaked: false,
                has_window_frame: true,
            }
        ));
    }

    #[test]
    fn borderless_work_area_candidate_hides_shell() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let work_area = RECT {
            left: 0,
            top: 26,
            right: 1920,
            bottom: 1044,
        };

        assert!(should_hide_shell_for_fullscreen_window(
            FullscreenWindowCandidate {
                window_rect: work_area,
                monitor_rect: monitor,
                work_area_rect: work_area,
                is_shell_process: false,
                is_desktop_shell_window: false,
                is_visible: true,
                is_minimized: false,
                is_cloaked: false,
                has_window_frame: false,
            }
        ));
    }

    #[test]
    fn shell_or_hidden_candidates_do_not_hide_shell() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let base = FullscreenWindowCandidate {
            window_rect: monitor,
            monitor_rect: monitor,
            work_area_rect: monitor,
            is_shell_process: false,
            is_desktop_shell_window: false,
            is_visible: true,
            is_minimized: false,
            is_cloaked: false,
            has_window_frame: false,
        };

        assert!(!should_hide_shell_for_fullscreen_window(
            FullscreenWindowCandidate {
                is_shell_process: true,
                ..base
            }
        ));
        assert!(!should_hide_shell_for_fullscreen_window(
            FullscreenWindowCandidate {
                is_visible: false,
                ..base
            }
        ));
        assert!(!should_hide_shell_for_fullscreen_window(
            FullscreenWindowCandidate {
                is_minimized: true,
                ..base
            }
        ));
        assert!(!should_hide_shell_for_fullscreen_window(
            FullscreenWindowCandidate {
                is_cloaked: true,
                ..base
            }
        ));
        assert!(!should_hide_shell_for_fullscreen_window(
            FullscreenWindowCandidate {
                is_desktop_shell_window: true,
                ..base
            }
        ));
    }

    #[test]
    fn fullscreen_rect_cover_tolerates_small_dwm_offsets() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let almost_monitor = RECT {
            left: 1,
            top: 1,
            right: 1919,
            bottom: 1079,
        };
        let too_small = RECT {
            left: 4,
            top: 0,
            right: 1920,
            bottom: 1080,
        };

        assert!(rect_covers_target(almost_monitor, monitor));
        assert!(!rect_covers_target(too_small, monitor));
    }
}
