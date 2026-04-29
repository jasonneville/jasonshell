use super::{
    icons::{window_icon_data_url, EMPTY_ICON_DATA_URL},
    TaskbarProcessWindow, TaskbarWindow, TaskbarWindowActivityState,
};
use std::collections::HashMap;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, FILETIME, HWND, LPARAM, POINT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    MonitorFromPoint, MonitorFromWindow, HMONITOR, MONITOR_DEFAULTTONULL, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    GWL_EXSTYLE, GW_OWNER, WINDOW_EX_STYLE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

const EXCLUDED_CLASSES: &[&str] = &[
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "Progman",
    "WorkerW",
    "Dwm",
];
const EXCLUDED_PROCESS_NAMES: &[&str] = &["dwm"];
const EXCLUDED_TITLES: &[&str] = &["DWM Notification Window"];
const CPU_TIME_BUSY_DELTA_TICKS: u64 = 200_000;

#[derive(Clone, Debug)]
pub(super) struct ActivitySnapshot {
    pub(super) process_id: u32,
    pub(super) title: String,
    pub(super) cpu_time_ticks: Option<u64>,
}

static ACTIVITY_SNAPSHOTS: OnceLock<Mutex<HashMap<String, ActivitySnapshot>>> = OnceLock::new();

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
        let hwnd = candidate.hwnd_string();
        let activity_state = task_window_activity_state(
            &hwnd,
            candidate.process_id,
            &candidate.title,
            &candidate.process_name,
            process_cpu_time_ticks(candidate.process_id),
        );

        windows.push(TaskbarWindow {
            hwnd,
            title: candidate.title,
            process_id: candidate.process_id,
            process_name: candidate.process_name,
            icon_data_url,
            is_active: candidate.is_active,
            is_minimized: candidate.is_minimized,
            activity_state,
        });
    }

    sort_windows_stably(&mut windows);
    retain_activity_snapshots(windows.iter().map(|window| window.hwnd.as_str()));
    Ok(windows)
}

pub(super) fn list_taskbar_process_windows() -> Result<Vec<TaskbarProcessWindow>, String> {
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

        windows.push(TaskbarProcessWindow {
            hwnd: candidate.hwnd_string(),
            title: candidate.title,
            process_id: candidate.process_id,
            is_active: candidate.is_active,
        });
    }

    windows.sort_by(|left, right| compare_window_handles(&left.hwnd, &right.hwnd));
    Ok(windows)
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    let handles = &mut *(lparam.0 as *mut Vec<HWND>);
    handles.push(hwnd);
    true.into()
}

impl WindowCandidate {
    fn hwnd_string(&self) -> String {
        (self.hwnd.0 as isize).to_string()
    }
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
    let process_name = resolve_process_name(process_path.as_deref());
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
    let is_no_activate_window = (candidate.ex_style.0 & WS_EX_NOACTIVATE.0) != 0;
    let forces_taskbar = (candidate.ex_style.0 & WS_EX_APPWINDOW.0) != 0;
    let has_taskbar_identity = !candidate.title.trim().is_empty()
        || (forces_taskbar && !candidate.process_name.is_empty());

    (candidate.is_visible || candidate.is_minimized)
        && candidate.is_primary_monitor
        && !candidate.is_shell_process
        && candidate.process_id != current_process_id
        && !candidate.has_owner
        && !candidate.is_cloaked
        && !is_tool_window
        && !is_no_activate_window
        && has_taskbar_identity
        && !is_internal_notification_window(candidate)
        && !EXCLUDED_PROCESS_NAMES
            .iter()
            .any(|process_name| candidate.process_name.eq_ignore_ascii_case(process_name))
        && !EXCLUDED_CLASSES
            .iter()
            .any(|class_name| candidate.class_name.eq_ignore_ascii_case(class_name))
}

pub(super) fn is_internal_notification_window(candidate: &WindowCandidate) -> bool {
    let title = candidate.title.trim();
    if EXCLUDED_TITLES
        .iter()
        .any(|excluded| title.eq_ignore_ascii_case(excluded))
    {
        return true;
    }

    title.eq_ignore_ascii_case("Notification")
        && (candidate.process_name.is_empty()
            || candidate.process_name.eq_ignore_ascii_case("dwm")
            || candidate.class_name.to_ascii_lowercase().contains("dwm"))
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

fn process_cpu_time_ticks(process_id: u32) -> Option<u64> {
    let process_handle =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };

    let result = (|| {
        let mut creation_time = FILETIME::default();
        let mut exit_time = FILETIME::default();
        let mut kernel_time = FILETIME::default();
        let mut user_time = FILETIME::default();
        unsafe {
            GetProcessTimes(
                process_handle,
                &mut creation_time,
                &mut exit_time,
                &mut kernel_time,
                &mut user_time,
            )
            .ok()?;
        }

        Some(filetime_ticks(kernel_time) + filetime_ticks(user_time))
    })();

    unsafe {
        let _ = CloseHandle(process_handle);
    }

    result
}

fn filetime_ticks(filetime: FILETIME) -> u64 {
    ((filetime.dwHighDateTime as u64) << 32) | filetime.dwLowDateTime as u64
}

pub(super) fn infer_activity_state(
    previous: Option<&ActivitySnapshot>,
    process_id: u32,
    title: &str,
    process_name: &str,
    cpu_time_ticks: Option<u64>,
) -> TaskbarWindowActivityState {
    if !is_activity_indicator_eligible(process_name, title) {
        return TaskbarWindowActivityState::Idle;
    }

    let Some(previous) = previous else {
        return TaskbarWindowActivityState::Idle;
    };

    if previous.title != title {
        return TaskbarWindowActivityState::Busy;
    }

    if previous.process_id == process_id {
        if let (Some(previous_ticks), Some(current_ticks)) =
            (previous.cpu_time_ticks, cpu_time_ticks)
        {
            if current_ticks.saturating_sub(previous_ticks) >= CPU_TIME_BUSY_DELTA_TICKS {
                return TaskbarWindowActivityState::Busy;
            }
        }
    }

    TaskbarWindowActivityState::Idle
}

fn task_window_activity_state(
    hwnd: &str,
    process_id: u32,
    title: &str,
    process_name: &str,
    cpu_time_ticks: Option<u64>,
) -> TaskbarWindowActivityState {
    let snapshots = ACTIVITY_SNAPSHOTS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut snapshots) = snapshots.lock() else {
        return TaskbarWindowActivityState::Idle;
    };
    let activity_state = infer_activity_state(
        snapshots.get(hwnd),
        process_id,
        title,
        process_name,
        cpu_time_ticks,
    );

    snapshots.insert(
        hwnd.to_string(),
        ActivitySnapshot {
            process_id,
            title: title.to_string(),
            cpu_time_ticks,
        },
    );

    activity_state
}

pub(super) fn is_activity_indicator_eligible(process_name: &str, title: &str) -> bool {
    let metadata = format!("{} {}", process_name, title).to_ascii_lowercase();
    let metadata = metadata.trim();
    let process_metadata = process_name.trim().to_ascii_lowercase();
    let title_metadata = title.trim().to_ascii_lowercase();
    if metadata.is_empty() && process_metadata.is_empty() {
        return false;
    }

    let is_terminal_process = [
        "terminal",
        "windowsterminal",
        "windows terminal",
        "wt",
        "cmd",
        "command prompt",
        "powershell",
        "pwsh",
        "conhost",
        "console",
    ]
    .iter()
    .any(|pattern| process_metadata.contains(pattern));
    let is_llm_process = [
        "opencode",
        "open code",
        "claude",
        "copilot",
        "cursor",
        "aider",
        "continue",
        "llm",
        "chatgpt",
        "codex",
    ]
    .iter()
    .any(|pattern| process_metadata.contains(pattern));
    let has_terminal_title_llm_signal = is_terminal_process
        && [
            "opencode",
            "open code",
            "claude",
            "copilot",
            "cursor",
            "aider",
            "continue",
            "llm",
            "chatgpt",
            "codex",
        ]
        .iter()
        .any(|pattern| title_metadata.contains(pattern));
    if is_terminal_process || is_llm_process || has_terminal_title_llm_signal {
        return true;
    }

    let is_browser = [
        "firefox", "chrome", "msedge", "edge", "brave", "opera", "vivaldi",
    ]
    .iter()
    .any(|pattern| process_metadata.contains(pattern));
    let has_download_signal = ["download", "downloading", "downloads"]
        .iter()
        .any(|pattern| metadata.contains(pattern));

    is_browser && has_download_signal
}

fn retain_activity_snapshots<'a>(visible_hwnds: impl Iterator<Item = &'a str>) {
    let snapshots = ACTIVITY_SNAPSHOTS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut snapshots) = snapshots.lock() else {
        return;
    };
    let visible_hwnds = visible_hwnds.collect::<std::collections::HashSet<_>>();
    snapshots.retain(|hwnd, _| visible_hwnds.contains(hwnd.as_str()));
}

fn resolve_process_name(process_path: Option<&Path>) -> String {
    process_path
        .and_then(Path::file_stem)
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(|name| name.to_string())
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
