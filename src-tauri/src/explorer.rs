#![cfg(target_os = "windows")]

use std::thread;
use std::time::Duration;
use windows::core::{w, Error as WindowsError, HRESULT, PCWSTR};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowRect, IsWindowVisible, ShowWindow, SW_HIDE, SW_SHOW,
};

const TASKBAR_VISIBILITY_RETRY_ATTEMPTS: usize = 20;
const TASKBAR_VISIBILITY_RETRY_DELAY: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug)]
pub struct ExplorerTaskbarSnapshot {
    pub hwnd_value: isize,
    pub originally_visible: bool,
    pub restore_to_visible: bool,
    pub original_rect: RECT,
}

impl ExplorerTaskbarSnapshot {
    pub fn baseline_work_area(self, primary_monitor_rect: RECT) -> Option<RECT> {
        if !occupies_primary_bottom_edge(self.original_rect, primary_monitor_rect) {
            return None;
        }

        Some(RECT {
            left: primary_monitor_rect.left,
            top: primary_monitor_rect.top,
            right: primary_monitor_rect.right,
            bottom: self.original_rect.top,
        })
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

    let taskbar = HWND(snapshot.hwnd_value as *mut _);

    if !set_taskbar_visibility(taskbar, false)? {
        eprintln!(
            "Explorer taskbar remained visible after hide retries; startup will continue with guard enforcement"
        );
    }

    Ok(Some(snapshot))
}

pub fn primary_taskbar_snapshot(
    primary_monitor_rect: RECT,
) -> Result<Option<ExplorerTaskbarSnapshot>, WindowsError> {
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null())? };

    if taskbar.0.is_null() {
        return Ok(None);
    }

    let mut taskbar_rect = RECT::default();
    unsafe { GetWindowRect(taskbar, &mut taskbar_rect)? };
    let taskbar_visible = unsafe { IsWindowVisible(taskbar) }.as_bool();

    if !occupies_primary_bottom_edge(taskbar_rect, primary_monitor_rect) {
        return Ok(None);
    }

    let snapshot = ExplorerTaskbarSnapshot {
        hwnd_value: taskbar.0 as isize,
        originally_visible: taskbar_visible,
        restore_to_visible: taskbar_visible,
        original_rect: taskbar_rect,
    };

    Ok(Some(snapshot))
}

pub fn enforce_taskbar_hidden(snapshot: ExplorerTaskbarSnapshot) -> Result<bool, WindowsError> {
    let taskbar = HWND(snapshot.hwnd_value as *mut _);
    set_taskbar_visibility(taskbar, false)
}

pub fn restore_taskbar(snapshot: ExplorerTaskbarSnapshot) -> Result<bool, WindowsError> {
    if !snapshot.restore_to_visible {
        return Ok(true);
    }

    set_taskbar_visibility(HWND(snapshot.hwnd_value as *mut _), true)
}

fn occupies_primary_bottom_edge(taskbar_rect: RECT, primary_monitor_rect: RECT) -> bool {
    let left_aligned = taskbar_rect.left <= primary_monitor_rect.left + 1;
    let right_aligned = taskbar_rect.right >= primary_monitor_rect.right - 1;
    let bottom_aligned = taskbar_rect.bottom >= primary_monitor_rect.bottom - 1;
    let has_height = taskbar_rect.bottom > taskbar_rect.top;

    left_aligned && right_aligned && bottom_aligned && has_height
}

fn hide_taskbar(taskbar: HWND) -> Result<(), WindowsError> {
    unsafe {
        let _ = ShowWindow(taskbar, SW_HIDE);
    }

    Ok(())
}

fn ensure_taskbar_hidden(taskbar: HWND) -> Result<(), WindowsError> {
    if unsafe { IsWindowVisible(taskbar) }.as_bool() {
        return Err(WindowsError::new(
            HRESULT(0x8000_4005_u32 as i32),
            "Explorer taskbar remained visible after hide request",
        ));
    }

    Ok(())
}

fn set_taskbar_visibility(taskbar: HWND, visible: bool) -> Result<bool, WindowsError> {
    for attempt in 0..TASKBAR_VISIBILITY_RETRY_ATTEMPTS {
        if visible {
            unsafe {
                let _ = ShowWindow(taskbar, SW_SHOW);
            }
        } else {
            hide_taskbar(taskbar)?;
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
        ensure_taskbar_hidden(taskbar).map(|_| true).or(Ok(false))
    }
}

#[cfg(test)]
mod tests {
    use super::occupies_primary_bottom_edge;
    use super::ExplorerTaskbarSnapshot;
    use windows::Win32::Foundation::RECT;

    #[test]
    fn detects_full_width_bottom_taskbar() {
        let primary_monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let taskbar = RECT {
            left: 0,
            top: 1032,
            right: 1920,
            bottom: 1080,
        };

        assert!(occupies_primary_bottom_edge(taskbar, primary_monitor));
    }

    #[test]
    fn rejects_non_bottom_edge_rectangles() {
        let primary_monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let floating_rect = RECT {
            left: 100,
            top: 1032,
            right: 1820,
            bottom: 1080,
        };

        assert!(!occupies_primary_bottom_edge(
            floating_rect,
            primary_monitor
        ));
    }

    #[test]
    fn baseline_work_area_uses_original_taskbar_height() {
        let primary_monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let snapshot = ExplorerTaskbarSnapshot {
            hwnd_value: 1,
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
            snapshot
                .baseline_work_area(primary_monitor)
                .expect("visible bottom taskbar should infer a baseline work area"),
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1032,
            }
        );
    }
}
