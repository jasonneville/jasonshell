#![cfg(target_os = "windows")]

use std::sync::{Condvar, Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use windows::core::{Error as WindowsError, BOOL};
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowRect, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
    ShowWindow, SW_HIDE, SW_SHOW,
};

const TASKBAR_VISIBILITY_RETRY_ATTEMPTS: usize = 20;
const TASKBAR_VISIBILITY_RETRY_DELAY: Duration = Duration::from_millis(50);
const TASKBAR_RECONCILE_WATCHDOG: Duration = Duration::from_millis(1500);

static RECONCILE_WAKE: OnceLock<(Mutex<u64>, Condvar)> = OnceLock::new();

pub fn request_taskbar_reconcile() {
    let (generation, wake) = RECONCILE_WAKE.get_or_init(|| (Mutex::new(0), Condvar::new()));
    if let Ok(mut generation) = generation.lock() {
        *generation = generation.wrapping_add(1);
        wake.notify_all();
    }
}

pub fn wait_for_taskbar_reconcile(last_generation: u64) -> u64 {
    let (generation, wake) = RECONCILE_WAKE.get_or_init(|| (Mutex::new(0), Condvar::new()));
    let Ok(generation) = generation.lock() else {
        return last_generation;
    };
    let Ok((generation, _)) =
        wake.wait_timeout_while(generation, TASKBAR_RECONCILE_WATCHDOG, |generation| {
            *generation == last_generation
        })
    else {
        return last_generation;
    };
    *generation
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorerTaskbarClass {
    Primary,
    Secondary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskbarEdge {
    Left,
    Top,
    Right,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExplorerTaskbarIdentity {
    pub hwnd: isize,
    pub process_id: u32,
    pub class_name: ExplorerTaskbarClass,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExplorerTaskbarSnapshot {
    pub identity: ExplorerTaskbarIdentity,
    pub monitor_rect: RECT,
    pub taskbar_rect: RECT,
    pub edge: TaskbarEdge,
    pub originally_visible: bool,
    pub hidden_by_jasonshell: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExplorerTaskbarDiagnosticsCounters {
    pub tracked: u64,
    pub hidden: u64,
    pub recreation: u64,
    pub hide_failure: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExplorerTaskbarDiagnosticsState {
    pub counters: ExplorerTaskbarDiagnosticsCounters,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ExplorerTaskbarModel {
    pub tracked: Vec<ExplorerTaskbarSnapshot>,
    pub hidden: Vec<ExplorerTaskbarSnapshot>,
    pub recreation_failures: u64,
    pub hide_failures: u64,
    pub last_error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorerTaskbarFakeOp {
    Hide(isize),
    Restore(isize),
}

impl ExplorerTaskbarSnapshot {
    pub fn baseline_work_area(self, primary_monitor_rect: RECT) -> Option<RECT> {
        if self.edge != TaskbarEdge::Bottom || self.monitor_rect != primary_monitor_rect {
            return None;
        }
        Some(RECT {
            left: primary_monitor_rect.left,
            top: primary_monitor_rect.top,
            right: primary_monitor_rect.right,
            bottom: self.taskbar_rect.top,
        })
    }
}

pub fn primary_taskbar_snapshot(
    primary_monitor_rect: RECT,
) -> Result<Option<ExplorerTaskbarSnapshot>, WindowsError> {
    Ok(primary_taskbar_snapshots(primary_monitor_rect)?
        .into_iter()
        .find(|s| s.identity.class_name == ExplorerTaskbarClass::Primary))
}

pub fn primary_taskbar_snapshots(
    primary_monitor_rect: RECT,
) -> Result<Vec<ExplorerTaskbarSnapshot>, WindowsError> {
    let mut snapshots = Vec::new();
    unsafe {
        EnumWindows(
            Some(enum_taskbars),
            LPARAM((&mut snapshots as *mut Vec<_>) as isize),
        )?;
    }
    snapshots.retain(|s: &ExplorerTaskbarSnapshot| s.monitor_rect == primary_monitor_rect);
    Ok(snapshots)
}

pub fn all_taskbar_snapshots() -> Result<Vec<ExplorerTaskbarSnapshot>, WindowsError> {
    let mut snapshots = Vec::new();
    unsafe {
        EnumWindows(
            Some(enum_taskbars),
            LPARAM((&mut snapshots as *mut Vec<_>) as isize),
        )?;
    }
    Ok(snapshots)
}

pub fn revalidate_snapshot_identity(snapshot: &ExplorerTaskbarSnapshot) -> bool {
    if !validate_identity(snapshot) {
        return false;
    }
    let hwnd = HWND(snapshot.identity.hwnd as *mut _);
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        return false;
    }
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    if pid != snapshot.identity.process_id {
        return false;
    }
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return false;
    }
    if current_taskbar_class(hwnd) != Some(snapshot.identity.class_name) {
        return false;
    }
    if current_taskbar_monitor_rect(hwnd) != Some(snapshot.monitor_rect) {
        return false;
    }
    let current_edge = classify_taskbar_edge(snapshot.monitor_rect, rect);
    current_edge == snapshot.edge
}

pub fn classify_taskbar_edge(monitor_rect: RECT, taskbar_rect: RECT) -> TaskbarEdge {
    let horizontal_span =
        taskbar_rect.left <= monitor_rect.left + 1 && taskbar_rect.right >= monitor_rect.right - 1;
    let vertical_span =
        taskbar_rect.top <= monitor_rect.top + 1 && taskbar_rect.bottom >= monitor_rect.bottom - 1;
    if horizontal_span {
        if taskbar_rect.top <= monitor_rect.top + 1 {
            TaskbarEdge::Top
        } else {
            TaskbarEdge::Bottom
        }
    } else if vertical_span {
        if taskbar_rect.left <= monitor_rect.left + 1 {
            TaskbarEdge::Left
        } else {
            TaskbarEdge::Right
        }
    } else if taskbar_rect.top <= monitor_rect.top + 1 {
        TaskbarEdge::Top
    } else if taskbar_rect.bottom >= monitor_rect.bottom - 1 {
        TaskbarEdge::Bottom
    } else if taskbar_rect.left <= monitor_rect.left + 1 {
        TaskbarEdge::Left
    } else {
        TaskbarEdge::Right
    }
}

pub fn hide_primary_taskbar_if_needed(
    primary_monitor_rect: RECT,
) -> Result<Option<ExplorerTaskbarSnapshot>, WindowsError> {
    let Some(snapshot) = primary_taskbar_snapshot(primary_monitor_rect)? else {
        return Ok(None);
    };
    if !snapshot.originally_visible {
        return Ok(Some(snapshot));
    }
    let taskbar = HWND(snapshot.identity.hwnd as *mut _);
    let hidden_by_jasonshell = set_taskbar_visibility(taskbar, false)?;
    if !hidden_by_jasonshell {
        eprintln!("Explorer taskbar remained visible after hide retries; startup will continue with guard enforcement");
    }
    Ok(Some(ExplorerTaskbarSnapshot {
        hidden_by_jasonshell,
        ..snapshot
    }))
}

pub fn hide_taskbars_if_needed(
    snapshots: Vec<ExplorerTaskbarSnapshot>,
) -> Result<Vec<ExplorerTaskbarSnapshot>, WindowsError> {
    let mut hide = |snapshot: ExplorerTaskbarSnapshot| -> Result<bool, WindowsError> {
        set_taskbar_visibility(HWND(snapshot.identity.hwnd as *mut _), false)
    };
    Ok(hide_taskbars_core(snapshots, &mut hide)?.0)
}

pub fn enforce_taskbar_hidden(snapshot: ExplorerTaskbarSnapshot) -> Result<bool, WindowsError> {
    set_taskbar_visibility(HWND(snapshot.identity.hwnd as *mut _), false)
}

pub fn enforce_hidden_taskbars(snaps: &[ExplorerTaskbarSnapshot]) -> Result<(), WindowsError> {
    for snapshot in snaps.iter().copied() {
        if snapshot.hidden_by_jasonshell {
            let _ = enforce_taskbar_hidden(snapshot)?;
        }
    }
    Ok(())
}

pub fn reconcile_owned_taskbars(
    owned: &mut Vec<ExplorerTaskbarSnapshot>,
) -> Result<(), WindowsError> {
    let current = all_taskbar_snapshots()?;
    let mut hide = |snapshot: ExplorerTaskbarSnapshot| -> Result<bool, WindowsError> {
        set_taskbar_visibility(HWND(snapshot.identity.hwnd as *mut _), false)
    };
    let (next, recreated, failures) =
        reconcile_owned_taskbars_core(owned.clone(), current.clone(), &mut hide)?;
    *owned = next;
    let recreations = recreated;
    let hide_failures = failures;
    let hidden = owned
        .iter()
        .filter(|snapshot| snapshot.hidden_by_jasonshell && revalidate_snapshot_identity(snapshot))
        .count() as u64;
    crate::task_windows::note_explorer_taskbar_reconcile(
        current.len() as u64,
        hidden,
        recreations,
        hide_failures,
        (hide_failures > 0).then_some("Explorer taskbar hide verification failed"),
    );
    Ok(())
}

pub fn reconcile_primary_taskbar_ownership(
    owned: &mut Vec<ExplorerTaskbarSnapshot>,
    primary_monitor_rect: RECT,
) -> Result<(), WindowsError> {
    let current = primary_taskbar_snapshots(primary_monitor_rect)?;
    let mut hide = |snapshot: ExplorerTaskbarSnapshot| -> Result<bool, WindowsError> {
        set_taskbar_visibility(HWND(snapshot.identity.hwnd as *mut _), false)
    };
    let (next, _, _) = reconcile_primary_taskbar_ownership_core(
        owned.clone(),
        current,
        primary_monitor_rect,
        &mut hide,
    )?;
    *owned = next;
    Ok(())
}

fn reconcile_primary_taskbar_ownership_core(
    mut owned: Vec<ExplorerTaskbarSnapshot>,
    current: Vec<ExplorerTaskbarSnapshot>,
    primary_monitor_rect: RECT,
    hide_taskbar: &mut impl FnMut(ExplorerTaskbarSnapshot) -> Result<bool, WindowsError>,
) -> Result<(Vec<ExplorerTaskbarSnapshot>, u64, u64), WindowsError> {
    let mut hide_failures = 0_u64;
    let mut recreations = 0_u64;
    for snapshot in current {
        if snapshot.monitor_rect != primary_monitor_rect
            || snapshot.identity.class_name != ExplorerTaskbarClass::Primary
        {
            continue;
        }
        if let Some(existing_index) = owned
            .iter()
            .position(|existing| existing.identity.hwnd == snapshot.identity.hwnd)
        {
            let existing = owned[existing_index];
            if existing.identity == snapshot.identity {
                if should_retry_owned_taskbar_hide(&existing) && snapshot.originally_visible {
                    if hide_taskbar(snapshot)? {
                        owned[existing_index] = ExplorerTaskbarSnapshot {
                            hidden_by_jasonshell: true,
                            ..snapshot
                        };
                        recreations = recreations.saturating_add(1);
                    } else {
                        hide_failures = hide_failures.saturating_add(1);
                    }
                }
            } else if should_retry_owned_taskbar_hide(&existing)
                && snapshot.originally_visible
                && existing.identity.class_name == snapshot.identity.class_name
                && existing.monitor_rect == snapshot.monitor_rect
                && existing.edge == snapshot.edge
            {
                if hide_taskbar(snapshot)? {
                    owned[existing_index] = ExplorerTaskbarSnapshot {
                        hidden_by_jasonshell: true,
                        ..snapshot
                    };
                    recreations = recreations.saturating_add(1);
                } else {
                    hide_failures = hide_failures.saturating_add(1);
                }
            }
            continue;
        }
        let replaces_owned = owned.iter().any(|existing| {
            should_retry_owned_taskbar_hide(existing)
                && existing.identity.class_name == snapshot.identity.class_name
                && existing.monitor_rect == snapshot.monitor_rect
                && existing.edge == snapshot.edge
        });
        if !replaces_owned || !snapshot.originally_visible {
            if owned.is_empty() {
                if snapshot.originally_visible && hide_taskbar(snapshot)? {
                    owned.push(ExplorerTaskbarSnapshot {
                        hidden_by_jasonshell: true,
                        ..snapshot
                    });
                    recreations = recreations.saturating_add(1);
                } else {
                    hide_failures = hide_failures.saturating_add(1);
                }
            }
            continue;
        }
        if hide_taskbar(snapshot)? {
            if let Some(existing_index) = owned.iter().position(|existing| {
                should_retry_owned_taskbar_hide(existing)
                    && existing.identity.class_name == snapshot.identity.class_name
                    && existing.monitor_rect == snapshot.monitor_rect
                    && existing.edge == snapshot.edge
            }) {
                owned[existing_index] = ExplorerTaskbarSnapshot {
                    hidden_by_jasonshell: true,
                    ..snapshot
                };
            } else {
                owned.push(ExplorerTaskbarSnapshot {
                    hidden_by_jasonshell: true,
                    ..snapshot
                });
            }
            recreations = recreations.saturating_add(1);
        } else {
            hide_failures = hide_failures.saturating_add(1);
        }
    }
    Ok((owned, recreations, hide_failures))
}

fn reconcile_owned_taskbars_core(
    mut owned: Vec<ExplorerTaskbarSnapshot>,
    current: Vec<ExplorerTaskbarSnapshot>,
    hide_taskbar: &mut impl FnMut(ExplorerTaskbarSnapshot) -> Result<bool, WindowsError>,
) -> Result<(Vec<ExplorerTaskbarSnapshot>, u64, u64), WindowsError> {
    let mut hide_failures = 0_u64;
    let mut recreations = 0_u64;
    for snapshot in current {
        if let Some(existing_index) = owned
            .iter()
            .position(|existing| existing.identity.hwnd == snapshot.identity.hwnd)
        {
            let existing = owned[existing_index];
            if existing.identity == snapshot.identity {
                if should_retry_owned_taskbar_hide(&existing) && snapshot.originally_visible {
                    if hide_taskbar(snapshot)? {
                        owned[existing_index] = ExplorerTaskbarSnapshot {
                            hidden_by_jasonshell: true,
                            ..snapshot
                        };
                        recreations = recreations.saturating_add(1);
                    } else {
                        hide_failures = hide_failures.saturating_add(1);
                    }
                }
            } else if should_retry_owned_taskbar_hide(&existing)
                && snapshot.originally_visible
                && existing.identity.class_name == snapshot.identity.class_name
                && existing.monitor_rect == snapshot.monitor_rect
                && existing.edge == snapshot.edge
            {
                if hide_taskbar(snapshot)? {
                    owned[existing_index] = ExplorerTaskbarSnapshot {
                        hidden_by_jasonshell: true,
                        ..snapshot
                    };
                    recreations = recreations.saturating_add(1);
                } else {
                    hide_failures = hide_failures.saturating_add(1);
                }
            }
            continue;
        }
        let replaces_owned = owned.iter().any(|existing| {
            should_retry_owned_taskbar_hide(existing)
                && existing.identity.class_name == snapshot.identity.class_name
                && existing.monitor_rect == snapshot.monitor_rect
                && existing.edge == snapshot.edge
        });
        if !replaces_owned || !snapshot.originally_visible {
            continue;
        }
        if hide_taskbar(snapshot)? {
            if let Some(existing_index) = owned.iter().position(|existing| {
                should_retry_owned_taskbar_hide(existing)
                    && existing.identity.class_name == snapshot.identity.class_name
                    && existing.monitor_rect == snapshot.monitor_rect
                    && existing.edge == snapshot.edge
            }) {
                owned[existing_index] = ExplorerTaskbarSnapshot {
                    hidden_by_jasonshell: true,
                    ..snapshot
                };
            } else {
                owned.push(ExplorerTaskbarSnapshot {
                    hidden_by_jasonshell: true,
                    ..snapshot
                });
            }
            recreations = recreations.saturating_add(1);
        } else {
            hide_failures = hide_failures.saturating_add(1);
        }
    }
    Ok((owned, recreations, hide_failures))
}

fn should_retry_owned_taskbar_hide(snapshot: &ExplorerTaskbarSnapshot) -> bool {
    snapshot.originally_visible
}

fn hide_taskbars_core(
    snapshots: Vec<ExplorerTaskbarSnapshot>,
    hide_taskbar: &mut impl FnMut(ExplorerTaskbarSnapshot) -> Result<bool, WindowsError>,
) -> Result<(Vec<ExplorerTaskbarSnapshot>, u64), WindowsError> {
    let mut hidden = Vec::new();
    let mut hide_failures = 0_u64;
    for snapshot in snapshots {
        if !snapshot.originally_visible {
            continue;
        }
        if hide_taskbar(snapshot)? {
            hidden.push(ExplorerTaskbarSnapshot {
                hidden_by_jasonshell: true,
                ..snapshot
            });
        } else {
            hide_failures = hide_failures.saturating_add(1);
        }
    }
    Ok((hidden, hide_failures))
}

pub fn enforce_primary_taskbar_hidden(primary_monitor_rect: RECT) -> Result<bool, WindowsError> {
    let Some(snapshot) = primary_taskbar_snapshot(primary_monitor_rect)? else {
        return Ok(false);
    };
    if !snapshot.originally_visible {
        return Ok(true);
    }
    set_taskbar_visibility(HWND(snapshot.identity.hwnd as *mut _), false)
}

pub fn restore_taskbar(snapshot: ExplorerTaskbarSnapshot) -> Result<bool, WindowsError> {
    if !snapshot.hidden_by_jasonshell || !snapshot.originally_visible {
        return Ok(true);
    }
    if !revalidate_snapshot_identity(&snapshot) {
        return Ok(false);
    }
    set_taskbar_visibility(HWND(snapshot.identity.hwnd as *mut _), true)
}

pub fn reconcile_taskbars(
    model: &ExplorerTaskbarModel,
    current: Vec<ExplorerTaskbarSnapshot>,
) -> (ExplorerTaskbarModel, Vec<ExplorerTaskbarFakeOp>) {
    let mut next = model.clone();
    next.tracked = current.clone();
    let mut ops = Vec::new();
    for snapshot in current {
        let hidden = next
            .hidden
            .iter()
            .any(|hidden| hidden.identity.hwnd == snapshot.identity.hwnd);
        if snapshot.originally_visible && !hidden {
            ops.push(ExplorerTaskbarFakeOp::Hide(snapshot.identity.hwnd));
        }
    }
    (next, ops)
}

pub fn safe_restore_taskbars(
    snapshots: &[ExplorerTaskbarSnapshot],
) -> (Vec<ExplorerTaskbarSnapshot>, Vec<ExplorerTaskbarFakeOp>) {
    safe_restore_plan(snapshots, |snapshot| revalidate_snapshot_identity(snapshot))
}

pub fn safe_restore_plan(
    snapshots: &[ExplorerTaskbarSnapshot],
    mut is_live: impl FnMut(&ExplorerTaskbarSnapshot) -> bool,
) -> (Vec<ExplorerTaskbarSnapshot>, Vec<ExplorerTaskbarFakeOp>) {
    let mut restored = Vec::new();
    let mut ops = Vec::new();
    for snapshot in snapshots.iter().copied() {
        if snapshot.originally_visible && snapshot.hidden_by_jasonshell && is_live(&snapshot) {
            ops.push(ExplorerTaskbarFakeOp::Restore(snapshot.identity.hwnd));
            restored.push(snapshot);
        }
    }
    (restored, ops)
}

fn set_taskbar_visibility(taskbar: HWND, visible: bool) -> Result<bool, WindowsError> {
    for attempt in 0..TASKBAR_VISIBILITY_RETRY_ATTEMPTS {
        unsafe {
            let _ = ShowWindow(taskbar, if visible { SW_SHOW } else { SW_HIDE });
        }
        if unsafe { IsWindowVisible(taskbar) }.as_bool() == visible {
            return Ok(true);
        }
        if attempt + 1 < TASKBAR_VISIBILITY_RETRY_ATTEMPTS {
            thread::sleep(TASKBAR_VISIBILITY_RETRY_DELAY);
        }
    }
    if visible {
        Ok(false)
    } else {
        Ok(!unsafe { IsWindowVisible(taskbar) }.as_bool())
    }
}

unsafe extern "system" fn enum_taskbars(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let snapshots = &mut *(lparam.0 as *mut Vec<ExplorerTaskbarSnapshot>);
    let mut class_buf = [0u16; 64];
    let len = GetClassNameW(hwnd, &mut class_buf) as usize;
    let class_name = match &class_buf[..len] {
        name if name
            == "Shell_TrayWnd"
                .encode_utf16()
                .collect::<Vec<_>>()
                .as_slice() =>
        {
            Some(ExplorerTaskbarClass::Primary)
        }
        name if name
            == "Shell_SecondaryTrayWnd"
                .encode_utf16()
                .collect::<Vec<_>>()
                .as_slice() =>
        {
            Some(ExplorerTaskbarClass::Secondary)
        }
        _ => None,
    };
    let Some(class_name) = class_name else {
        return BOOL(1);
    };
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(monitor, &mut mi).as_bool() {
        return BOOL(1);
    }
    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect).is_err() {
        return BOOL(1);
    }
    let edge = classify_taskbar_edge(mi.rcMonitor, rect);
    snapshots.push(ExplorerTaskbarSnapshot {
        identity: ExplorerTaskbarIdentity {
            hwnd: hwnd.0 as isize,
            process_id: pid,
            class_name,
        },
        monitor_rect: mi.rcMonitor,
        taskbar_rect: rect,
        edge,
        originally_visible: IsWindowVisible(hwnd).as_bool(),
        hidden_by_jasonshell: false,
    });
    BOOL(1)
}

fn validate_identity(snapshot: &ExplorerTaskbarSnapshot) -> bool {
    if snapshot.identity.hwnd == 0 || snapshot.identity.process_id == 0 {
        return false;
    }
    if snapshot.taskbar_rect.right <= snapshot.taskbar_rect.left
        || snapshot.taskbar_rect.bottom <= snapshot.taskbar_rect.top
    {
        return false;
    }
    true
}

fn current_taskbar_class(hwnd: HWND) -> Option<ExplorerTaskbarClass> {
    let mut class_buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut class_buf) } as usize;
    match &class_buf[..len] {
        name if name
            == "Shell_TrayWnd"
                .encode_utf16()
                .collect::<Vec<_>>()
                .as_slice() =>
        {
            Some(ExplorerTaskbarClass::Primary)
        }
        name if name
            == "Shell_SecondaryTrayWnd"
                .encode_utf16()
                .collect::<Vec<_>>()
                .as_slice() =>
        {
            Some(ExplorerTaskbarClass::Secondary)
        }
        _ => None,
    }
}

fn current_taskbar_monitor_rect(hwnd: HWND) -> Option<RECT> {
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut mi) }.as_bool() {
        return None;
    }
    Some(mi.rcMonitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_work_area_uses_original_taskbar_height() {
        let primary = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let snapshot = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 1,
                process_id: 2,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: primary,
            taskbar_rect: RECT {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: true,
        };
        assert_eq!(snapshot.baseline_work_area(primary).unwrap().bottom, 1032);
    }

    #[test]
    fn validate_identity_rejects_stale_handles() {
        assert!(!validate_identity(&ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 0,
                process_id: 0,
                class_name: ExplorerTaskbarClass::Primary
            },
            monitor_rect: RECT::default(),
            taskbar_rect: RECT::default(),
            edge: TaskbarEdge::Bottom,
            originally_visible: false,
            hidden_by_jasonshell: false
        }));
    }

    #[test]
    fn reconcile_taskbars_returns_fake_hide_ops_for_visible_snapshots() {
        let primary = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 11,
                process_id: 22,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: RECT::default(),
            taskbar_rect: RECT {
                left: 0,
                top: 0,
                right: 10,
                bottom: 1,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: false,
        };
        let (next, ops) = reconcile_taskbars(&ExplorerTaskbarModel::default(), vec![primary]);
        assert_eq!(next.tracked.len(), 1);
        assert_eq!(ops, vec![ExplorerTaskbarFakeOp::Hide(11)]);
    }

    #[test]
    fn safe_restore_taskbars_only_keeps_hidden_visible_matches() {
        let snapshot = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 7,
                process_id: 8,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: RECT::default(),
            taskbar_rect: RECT {
                left: 0,
                top: 0,
                right: 10,
                bottom: 1,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: true,
        };
        let (restored, ops) = safe_restore_plan(&[snapshot], |_| true);
        assert_eq!(restored.len(), ops.len());
    }

    #[test]
    fn classify_taskbar_edge_handles_all_edges_with_nonzero_origin() {
        let monitor = RECT {
            left: 100,
            top: 200,
            right: 2100,
            bottom: 1400,
        };
        assert_eq!(
            classify_taskbar_edge(
                monitor,
                RECT {
                    left: 100,
                    top: 200,
                    right: 2100,
                    bottom: 250
                }
            ),
            TaskbarEdge::Top
        );
        assert_eq!(
            classify_taskbar_edge(
                monitor,
                RECT {
                    left: 100,
                    top: 1350,
                    right: 2100,
                    bottom: 1400
                }
            ),
            TaskbarEdge::Bottom
        );
        assert_eq!(
            classify_taskbar_edge(
                monitor,
                RECT {
                    left: 100,
                    top: 200,
                    right: 160,
                    bottom: 1400
                }
            ),
            TaskbarEdge::Left
        );
        assert_eq!(
            classify_taskbar_edge(
                monitor,
                RECT {
                    left: 2040,
                    top: 200,
                    right: 2100,
                    bottom: 1400
                }
            ),
            TaskbarEdge::Right
        );
    }

    #[test]
    fn hide_taskbars_if_needed_excludes_originally_hidden_snapshots() {
        let visible = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 1,
                process_id: 2,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: RECT::default(),
            taskbar_rect: RECT {
                left: 0,
                top: 0,
                right: 10,
                bottom: 1,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: false,
        };
        let hidden = ExplorerTaskbarSnapshot {
            originally_visible: false,
            hidden_by_jasonshell: false,
            ..visible
        };
        let plan = snapshots_to_hide_for_test(&[visible, hidden]).unwrap();
        assert_eq!(
            plan,
            vec![ExplorerTaskbarSnapshot {
                hidden_by_jasonshell: true,
                ..visible
            }]
        );
    }

    #[test]
    fn restore_safety_only_allows_exact_live_hidden_owned_snapshots() {
        let live = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 9,
                process_id: 10,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: RECT::default(),
            taskbar_rect: RECT {
                left: 0,
                top: 0,
                right: 10,
                bottom: 1,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: true,
        };
        let stale = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 9,
                process_id: 11,
                class_name: ExplorerTaskbarClass::Primary,
            },
            ..live
        };
        let not_owned = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 99,
                process_id: 10,
                class_name: ExplorerTaskbarClass::Primary,
            },
            ..live
        };
        let originally_hidden = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 100,
                process_id: 10,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: live.monitor_rect,
            taskbar_rect: live.taskbar_rect,
            edge: live.edge,
            originally_visible: false,
            hidden_by_jasonshell: true,
        };
        let (restored, _) =
            safe_restore_plan(&[live, stale, not_owned, originally_hidden], |snapshot| {
                snapshot.identity == live.identity
            });
        assert_eq!(restored, vec![live]);
    }

    #[test]
    fn reconcile_owned_taskbars_skips_prehidden_replacement_and_keeps_exact_ownership() {
        let monitor = RECT {
            left: 100,
            top: 200,
            right: 2100,
            bottom: 1400,
        };
        let stale = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 41,
                process_id: 1,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: monitor,
            taskbar_rect: RECT {
                left: 100,
                top: 1350,
                right: 2100,
                bottom: 1400,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: true,
        };
        let nonmatching = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 77,
                process_id: 2,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: monitor,
            taskbar_rect: RECT {
                left: 100,
                top: 1350,
                right: 2100,
                bottom: 1400,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: false,
            hidden_by_jasonshell: false,
        };
        let (next, ops) = reconcile_owned_taskbars_for_test(vec![stale], vec![nonmatching], true);
        assert_eq!(ops, vec![]);
        assert_eq!(next, vec![stale]);
    }

    #[test]
    fn reconcile_owned_taskbars_retries_initial_hide_failure_for_originally_visible_owned_taskbars()
    {
        let monitor = RECT {
            left: 100,
            top: 200,
            right: 2100,
            bottom: 1400,
        };
        let owned = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 42,
                process_id: 1,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: monitor,
            taskbar_rect: RECT {
                left: 100,
                top: 1350,
                right: 2100,
                bottom: 1400,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: false,
        };
        let replacement = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 42,
                process_id: 2,
                class_name: ExplorerTaskbarClass::Primary,
            },
            ..owned
        };
        let (next, ops) = reconcile_owned_taskbars_for_test(vec![owned], vec![replacement], true);

        assert_eq!(ops, vec![ExplorerTaskbarFakeOp::Hide(42)]);
        assert_eq!(next[0].identity.process_id, 2);
        assert!(next[0].hidden_by_jasonshell);
    }

    #[test]
    fn reconcile_owned_taskbars_updates_same_identity_retry_after_successful_hide() {
        let monitor = RECT {
            left: 100,
            top: 200,
            right: 2100,
            bottom: 1400,
        };
        let owned = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 42,
                process_id: 1,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: monitor,
            taskbar_rect: RECT {
                left: 100,
                top: 1350,
                right: 2100,
                bottom: 1400,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: false,
        };
        let replacement = ExplorerTaskbarSnapshot {
            identity: owned.identity,
            ..owned
        };
        let (next, ops) = reconcile_owned_taskbars_for_test(vec![owned], vec![replacement], true);

        assert_eq!(ops, vec![ExplorerTaskbarFakeOp::Hide(42)]);
        assert_eq!(next[0].identity.process_id, 1);
        assert!(next[0].hidden_by_jasonshell);
    }

    #[test]
    fn reconcile_owned_taskbars_replaces_same_hwnd_with_new_pid_identity() {
        let monitor = RECT {
            left: 100,
            top: 200,
            right: 2100,
            bottom: 1400,
        };
        let owned = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 42,
                process_id: 1,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: monitor,
            taskbar_rect: RECT {
                left: 100,
                top: 1350,
                right: 2100,
                bottom: 1400,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: true,
        };
        let replacement = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 42,
                process_id: 2,
                class_name: ExplorerTaskbarClass::Primary,
            },
            ..owned
        };
        let (next, ops) = reconcile_owned_taskbars_for_test(vec![owned], vec![replacement], true);
        assert_eq!(ops, vec![ExplorerTaskbarFakeOp::Hide(42)]);
        assert_eq!(next[0].identity.process_id, 2);
        assert!(next[0].hidden_by_jasonshell);
    }

    #[test]
    fn reconcile_owned_taskbars_replaces_stale_owned_snapshot_for_new_hwnd() {
        let monitor = RECT {
            left: 100,
            top: 200,
            right: 2100,
            bottom: 1400,
        };
        let owned = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 42,
                process_id: 1,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: monitor,
            taskbar_rect: RECT {
                left: 100,
                top: 1350,
                right: 2100,
                bottom: 1400,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: true,
        };
        let replacement = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 43,
                process_id: 2,
                class_name: ExplorerTaskbarClass::Primary,
            },
            ..owned
        };
        let (next, ops) = reconcile_owned_taskbars_for_test(vec![owned], vec![replacement], true);
        assert_eq!(ops, vec![ExplorerTaskbarFakeOp::Hide(43)]);
        assert_eq!(next[0].identity.hwnd, 43);
        assert_eq!(next[0].identity.process_id, 2);
        assert!(next[0].hidden_by_jasonshell);
    }

    #[test]
    fn primary_ownership_reconcile_hides_late_visible_primary_and_records_ownership() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let visible = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 61,
                process_id: 21,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: monitor,
            taskbar_rect: RECT {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: true,
            hidden_by_jasonshell: false,
        };
        let mut hides = Vec::new();
        let (next, _, _) = reconcile_primary_taskbar_ownership_core(
            Vec::new(),
            vec![visible],
            monitor,
            &mut |snapshot| {
                hides.push(snapshot.identity.hwnd);
                Ok(true)
            },
        )
        .unwrap();
        assert_eq!(hides, vec![61]);
        assert_eq!(
            next,
            vec![ExplorerTaskbarSnapshot {
                hidden_by_jasonshell: true,
                ..visible
            }]
        );
    }

    #[test]
    fn primary_ownership_reconcile_ignores_prehidden_and_other_monitor_primary() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let other_monitor = RECT {
            left: 1920,
            top: 0,
            right: 3840,
            bottom: 1080,
        };
        let hidden_primary = ExplorerTaskbarSnapshot {
            identity: ExplorerTaskbarIdentity {
                hwnd: 62,
                process_id: 22,
                class_name: ExplorerTaskbarClass::Primary,
            },
            monitor_rect: monitor,
            taskbar_rect: RECT {
                left: 0,
                top: 1032,
                right: 1920,
                bottom: 1080,
            },
            edge: TaskbarEdge::Bottom,
            originally_visible: false,
            hidden_by_jasonshell: false,
        };
        let other = ExplorerTaskbarSnapshot {
            monitor_rect: other_monitor,
            ..hidden_primary
        };
        let mut hides = Vec::new();
        let (next, _, _) = reconcile_primary_taskbar_ownership_core(
            Vec::new(),
            vec![hidden_primary, other],
            monitor,
            &mut |snapshot| {
                hides.push(snapshot.identity.hwnd);
                Ok(true)
            },
        )
        .unwrap();
        assert!(hides.is_empty());
        assert!(next.is_empty());
    }

    fn snapshots_to_hide_for_test(
        snapshots: &[ExplorerTaskbarSnapshot],
    ) -> Result<Vec<ExplorerTaskbarSnapshot>, WindowsError> {
        let mut hide_ok = |snapshot: ExplorerTaskbarSnapshot| -> Result<bool, WindowsError> {
            Ok(snapshot.originally_visible)
        };
        Ok(hide_taskbars_core(snapshots.to_vec(), &mut hide_ok)?.0)
    }

    fn reconcile_owned_taskbars_for_test(
        owned: Vec<ExplorerTaskbarSnapshot>,
        current: Vec<ExplorerTaskbarSnapshot>,
        hide_ok: bool,
    ) -> (Vec<ExplorerTaskbarSnapshot>, Vec<ExplorerTaskbarFakeOp>) {
        let mut ops = Vec::new();
        let mut hide = |snapshot: ExplorerTaskbarSnapshot| -> Result<bool, WindowsError> {
            if hide_ok {
                ops.push(ExplorerTaskbarFakeOp::Hide(snapshot.identity.hwnd));
            }
            Ok(hide_ok)
        };
        let (next, _, _) = reconcile_owned_taskbars_core(owned, current, &mut hide).unwrap();
        (next, ops)
    }
}
