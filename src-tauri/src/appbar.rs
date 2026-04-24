#![cfg(target_os = "windows")]

use crate::explorer;
use crate::shell_windows::{
    AppResult, CreatedShellWindows, BOTTOM_BAR_HEIGHT_LOGICAL, TOP_BAR_HEIGHT_LOGICAL,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::c_void;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{App, AppHandle, Manager};
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, RegisterWindowMessageW, SetWindowPos, SystemParametersInfoW, HWND_TOPMOST,
    SPIF_SENDCHANGE, SPI_GETWORKAREA, SPI_SETWORKAREA, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOOWNERZORDER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
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

#[derive(Default)]
pub struct ShellRuntimeState {
    pub cleaned_up: bool,
    pub hidden_explorer_taskbar: Option<explorer::ExplorerTaskbarSnapshot>,
    pub baseline_work_area: Option<RECT>,
    pub registered_appbars: Vec<isize>,
    pub taskbar_guard_stop: Option<Arc<AtomicBool>>,
    pub taskbar_guard: Option<JoinHandle<()>>,
}

const WORK_AREA_RETRY_ATTEMPTS: usize = 20;
const WORK_AREA_RETRY_DELAY: Duration = Duration::from_millis(50);
const STARTUP_STABILIZATION_POLLS: usize = 15;
const STARTUP_STABILIZATION_DELAY: Duration = Duration::from_millis(100);
const REQUIRED_STABLE_POLLS: usize = 3;

#[derive(Clone, Copy)]
enum AppBarEdge {
    Top,
    Bottom,
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
    state.hidden_explorer_taskbar = None;
    state.registered_appbars.clear();

    let activation_result = (|| -> AppResult<()> {
        let current_work_area = get_work_area()?;
        let explorer_taskbar = explorer::primary_taskbar_snapshot(monitor_rect)?;
        let should_restore_hidden_taskbar = explorer_taskbar
            .and_then(|snapshot| snapshot.baseline_work_area(monitor_rect))
            .is_some()
            && work_area_looks_dirty(current_work_area, monitor_rect);
        state.baseline_work_area = Some(resolve_baseline_work_area(
            monitor_rect,
            current_work_area,
            explorer_taskbar,
        ));
        state.hidden_explorer_taskbar = explorer_taskbar.and_then(|mut snapshot| {
            if should_restore_hidden_taskbar {
                snapshot.restore_to_visible = true;
            }

            if snapshot.originally_visible || snapshot.restore_to_visible {
                Some(snapshot)
            } else {
                None
            }
        });

        if state.hidden_explorer_taskbar.is_some() {
            if state
                .hidden_explorer_taskbar
                .is_some_and(|snapshot| snapshot.originally_visible)
            {
                state.hidden_explorer_taskbar =
                    explorer::hide_primary_taskbar_if_needed(monitor_rect)?;
            }
            start_taskbar_guard(&mut state);
            set_work_area_with_retry(
                monitor_rect,
                "monitor work area while Explorer taskbar is hidden",
            )?;
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

        set_work_area_with_retry(
            reserved_work_area(monitor_rect, resolved_top_rect, resolved_bottom_rect),
            "reserved shell work area",
        )?;

        if let Some(snapshot) = state.hidden_explorer_taskbar {
            if !explorer::enforce_taskbar_hidden(snapshot)? {
                eprintln!(
                    "Explorer taskbar remained visible after startup retries; guard thread will continue enforcement"
                );
            }
        }

        move_window_to_rect(top_hwnd, resolved_top_rect)?;
        move_window_to_rect(bottom_hwnd, resolved_bottom_rect)?;

        windows.top.show()?;
        windows.bottom.show()?;

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

        if let Some(snapshot) = state.hidden_explorer_taskbar {
            if !explorer::enforce_taskbar_hidden(snapshot)? {
                eprintln!(
                    "Explorer taskbar was still visible after shell surfaces finished showing"
                );
            }
        }

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
    stop_taskbar_guard(state);

    cleanup_runtime_state_with(
        state,
        unregister_appbar,
        set_work_area_with_retry,
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

    for hwnd_value in state.registered_appbars.iter().rev().copied() {
        if let Err(error) = unregister(HWND(hwnd_value as *mut _)) {
            cleanup_errors.push(error.to_string());
        }
    }
    state.registered_appbars.clear();

    if let Some(taskbar_snapshot) = state.hidden_explorer_taskbar.take() {
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

    if cleanup_errors.is_empty() {
        Ok(())
    } else {
        Err(cleanup_errors.join("; ").into())
    }
}

fn start_taskbar_guard(state: &mut ShellRuntimeState) {
    stop_taskbar_guard(state);

    let Some(snapshot) = state.hidden_explorer_taskbar else {
        return;
    };

    let stop = Arc::new(AtomicBool::new(false));
    let stop_signal = Arc::clone(&stop);
    let guard = thread::spawn(move || {
        while !stop_signal.load(Ordering::Relaxed) {
            let _ = explorer::enforce_taskbar_hidden(snapshot);
            thread::sleep(Duration::from_millis(100));
        }
    });

    state.taskbar_guard_stop = Some(stop);
    state.taskbar_guard = Some(guard);
}

fn stop_taskbar_guard(state: &mut ShellRuntimeState) {
    if let Some(stop) = state.taskbar_guard_stop.take() {
        stop.store(true, Ordering::Relaxed);
    }

    if let Some(guard) = state.taskbar_guard.take() {
        let _ = guard.join();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_requested_thickness, cleanup_runtime_state_with, normalize_rect_thickness,
        register_tracked_appbar, reserved_work_area, resolve_baseline_work_area,
        stabilize_runtime_window_rect_with, ShellRuntimeState, WindowRectSnapshot,
    };
    use crate::explorer::ExplorerTaskbarSnapshot;
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
            hwnd_value: 44,
            originally_visible: true,
            restore_to_visible: true,
            original_rect: RECT {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
        };
        let mut state = ShellRuntimeState {
            cleaned_up: false,
            hidden_explorer_taskbar: Some(hidden_taskbar),
            baseline_work_area: Some(baseline_work_area),
            registered_appbars: Vec::new(),
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
                restored_taskbars.push(snapshot.hwnd_value);
                Ok(true)
            },
        )
        .expect("rollback should clean tracked partial registrations");

        assert_eq!(removed_appbars, vec![hwnd.0 as isize]);
        assert_eq!(restored_work_areas, vec![baseline_work_area]);
        assert_eq!(restored_taskbars, vec![hidden_taskbar.hwnd_value]);
        assert!(state.registered_appbars.is_empty());
        assert!(state.baseline_work_area.is_none());
        assert!(state.hidden_explorer_taskbar.is_none());
        assert!(state.cleaned_up);
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
            hwnd_value: 44,
            originally_visible: true,
            restore_to_visible: true,
            original_rect: RECT {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
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
            hwnd_value: 44,
            originally_visible: false,
            restore_to_visible: false,
            original_rect: RECT {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
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
}
