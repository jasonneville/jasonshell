use super::{
    icons::{window_icon_data_url, EMPTY_ICON_DATA_URL},
    TaskbarWindow,
};
use std::ffi::OsStr;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    MonitorFromPoint, MonitorFromWindow, HMONITOR, MONITOR_DEFAULTTONULL, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    GWL_EXSTYLE, GW_OWNER, WINDOW_EX_STYLE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

const EXCLUDED_CLASSES: &[&str] = &[
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "Progman",
    "WorkerW",
];

#[derive(Clone, Debug)]
pub(super) struct WindowCandidate {
    pub(super) class_name: String,
    pub(super) title: String,
    pub(super) process_name: String,
    pub(super) process_path: Option<PathBuf>,
    pub(super) hwnd: HWND,
    pub(super) is_active: bool,
    pub(super) is_minimized: bool,
    pub(super) has_owner: bool,
    pub(super) is_cloaked: bool,
    pub(super) is_primary_monitor: bool,
    pub(super) is_shell_process: bool,
    pub(super) is_visible: bool,
    pub(super) ex_style: WINDOW_EX_STYLE,
    pub(super) process_id: u32,
}

pub(super) fn list_open_task_windows() -> Result<Vec<TaskbarWindow>, String> {
    let current_process_id = std::process::id();
    let foreground = unsafe { GetForegroundWindow() };
    let primary_monitor =
        unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
    let mut handles = Vec::new();

    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            LPARAM((&mut handles as *mut Vec<HWND>) as isize),
        )
        .map_err(|error| format!("Failed to enumerate top-level windows: {error}"))?;
    }

    let mut windows = Vec::new();
    for hwnd in handles {
        let Some(candidate) =
            build_window_candidate(hwnd, foreground, primary_monitor, current_process_id)?
        else {
            continue;
        };

        if !is_taskbar_candidate(&candidate, current_process_id) {
            continue;
        }

        let icon_data_url = window_icon_data_url(candidate.hwnd, candidate.process_path.as_deref())
            .unwrap_or_else(|_| EMPTY_ICON_DATA_URL.to_string());

        windows.push(TaskbarWindow {
            hwnd: (candidate.hwnd.0 as isize).to_string(),
            title: candidate.title,
            process_name: candidate.process_name,
            icon_data_url,
            is_active: candidate.is_active,
            is_minimized: candidate.is_minimized,
        });
    }

    sort_windows_stably(&mut windows);
    Ok(windows)
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    let handles = &mut *(lparam.0 as *mut Vec<HWND>);
    handles.push(hwnd);
    true.into()
}

fn build_window_candidate(
    hwnd: HWND,
    foreground: HWND,
    primary_monitor: HMONITOR,
    current_process_id: u32,
) -> Result<Option<WindowCandidate>, String> {
    let mut process_id = 0;
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }

    if process_id == 0 {
        return Ok(None);
    }

    let title = window_text(hwnd);
    let class_name = class_name(hwnd);
    let process_path = process_image_path(process_id);
    let process_name = resolve_process_name(process_path.as_deref(), &title);
    let owner = unsafe { GetWindow(hwnd, GW_OWNER).unwrap_or_default() };
    let ex_style = WINDOW_EX_STYLE(unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 });
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL) };

    Ok(Some(WindowCandidate {
        class_name,
        title,
        process_name,
        process_path,
        hwnd,
        is_active: hwnd == foreground,
        is_minimized: unsafe { IsIconic(hwnd).as_bool() },
        has_owner: !owner.0.is_null(),
        is_cloaked: is_window_cloaked(hwnd),
        is_primary_monitor: monitor == primary_monitor,
        is_shell_process: process_id == current_process_id,
        is_visible: unsafe { IsWindowVisible(hwnd).as_bool() },
        ex_style,
        process_id,
    }))
}

pub(super) fn is_taskbar_candidate(candidate: &WindowCandidate, current_process_id: u32) -> bool {
    let is_tool_window = (candidate.ex_style.0 & WS_EX_TOOLWINDOW.0) != 0
        && (candidate.ex_style.0 & WS_EX_APPWINDOW.0) == 0;
    let has_identity = !candidate.title.trim().is_empty() || !candidate.process_name.is_empty();

    (candidate.is_visible || candidate.is_minimized)
        && candidate.is_primary_monitor
        && !candidate.is_shell_process
        && candidate.process_id != current_process_id
        && !candidate.has_owner
        && !candidate.is_cloaked
        && !is_tool_window
        && has_identity
        && !EXCLUDED_CLASSES
            .iter()
            .any(|class_name| candidate.class_name.eq_ignore_ascii_case(class_name))
}

pub(super) fn sort_windows_stably(windows: &mut [TaskbarWindow]) {
    windows.sort_by(|left, right| compare_window_handles(&left.hwnd, &right.hwnd));
}

fn window_text(hwnd: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length <= 0 {
        return String::new();
    }

    let mut buffer = vec![0_u16; length as usize + 1];
    let actual = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    from_wide_buffer(&buffer[..actual.max(0) as usize])
}

fn class_name(hwnd: HWND) -> String {
    let mut buffer = vec![0_u16; 256];
    let actual = unsafe { GetClassNameW(hwnd, &mut buffer) };
    from_wide_buffer(&buffer[..actual.max(0) as usize])
}

fn process_image_path(process_id: u32) -> Option<PathBuf> {
    let process_handle =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };

    let result = (|| {
        let mut buffer = vec![0_u16; 1024];
        let mut size = buffer.len() as u32;
        unsafe {
            QueryFullProcessImageNameW(
                process_handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            )
            .ok()?;
        }

        Some(PathBuf::from(from_wide_buffer(&buffer[..size as usize])))
    })();

    unsafe {
        let _ = CloseHandle(process_handle);
    }

    result
}

fn resolve_process_name(process_path: Option<&Path>, title: &str) -> String {
    process_path
        .and_then(Path::file_stem)
        .and_then(OsStr::to_str)
        .filter(|name| !name.trim().is_empty())
        .map(|name| name.to_string())
        .or_else(|| {
            let trimmed = title.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .unwrap_or_default()
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

pub(super) fn compare_window_handles(left: &str, right: &str) -> std::cmp::Ordering {
    match (left.parse::<i128>(), right.parse::<i128>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        (Err(_), Err(_)) => left.cmp(right),
    }
}

fn from_wide_buffer(buffer: &[u16]) -> String {
    String::from_utf16_lossy(buffer)
        .trim_matches('\0')
        .trim()
        .to_string()
}
