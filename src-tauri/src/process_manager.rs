use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

use crate::shell_windows::{
    BOTTOM_BAR_LABEL, PROCESS_MANAGER_HEIGHT_LOGICAL, PROCESS_MANAGER_LABEL,
    PROCESS_MANAGER_WIDTH_LOGICAL,
};

const PROCESS_MANAGER_MARGIN_PHYSICAL: i32 = 8;
const PROCESS_MANAGER_EDGE_PADDING_PHYSICAL: i32 = 8;
const PROCESS_MANAGER_OPEN_EVENT: &str = "process-manager:open";
pub const PROCESS_MANAGER_CLOSED_EVENT: &str = "process-manager:closed";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub executable_path: Option<String>,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub thread_count: Option<u32>,
    pub start_time_ms: Option<u64>,
    pub status: String,
    pub is_killable: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowProcessManagerRequest {
    pub anchor_left: f64,
    pub anchor_width: f64,
}

#[derive(Clone, Debug)]
struct ProcessCpuSnapshot {
    observed_at: Instant,
    cpu_time_ticks: u64,
}

static PROCESS_CPU_SNAPSHOTS: OnceLock<Mutex<HashMap<u32, ProcessCpuSnapshot>>> = OnceLock::new();

#[tauri::command]
pub fn show_process_manager(
    app_handle: AppHandle,
    request: ShowProcessManagerRequest,
) -> Result<(), String> {
    let popup = app_handle
        .get_webview_window(PROCESS_MANAGER_LABEL)
        .ok_or_else(|| "Process manager window is unavailable".to_string())?;
    let bottom_bar = app_handle
        .get_webview_window(BOTTOM_BAR_LABEL)
        .ok_or_else(|| "Bottom bar window is unavailable".to_string())?;
    let monitor = bottom_bar
        .current_monitor()
        .map_err(|error| format!("Failed to inspect bottom-bar monitor: {error}"))?
        .or_else(|| app_handle.primary_monitor().ok().flatten())
        .ok_or_else(|| "Primary monitor is unavailable".to_string())?;
    let scale_factor = monitor.scale_factor();
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let bottom_position = bottom_bar
        .outer_position()
        .map_err(|error| format!("Failed to read bottom-bar position: {error}"))?;
    let bottom_size = bottom_bar
        .outer_size()
        .map_err(|error| format!("Failed to read bottom-bar size: {error}"))?;

    let width = ((PROCESS_MANAGER_WIDTH_LOGICAL * scale_factor).round() as u32).min(
        monitor_size
            .width
            .saturating_sub((PROCESS_MANAGER_EDGE_PADDING_PHYSICAL * 2) as u32),
    );
    let height = ((PROCESS_MANAGER_HEIGHT_LOGICAL * scale_factor).round() as u32).min(
        monitor_size
            .height
            .saturating_sub((PROCESS_MANAGER_EDGE_PADDING_PHYSICAL * 2) as u32),
    );
    let anchor_right = bottom_position.x
        + ((request.anchor_left + request.anchor_width) * scale_factor).round() as i32;
    let min_x = monitor_position.x + PROCESS_MANAGER_EDGE_PADDING_PHYSICAL;
    let max_x = monitor_position.x + monitor_size.width as i32
        - width as i32
        - PROCESS_MANAGER_EDGE_PADDING_PHYSICAL;
    let x = (anchor_right - width as i32).clamp(min_x, max_x.max(min_x));
    let min_y = monitor_position.y + PROCESS_MANAGER_EDGE_PADDING_PHYSICAL;
    let y = (bottom_position.y - height as i32 - PROCESS_MANAGER_MARGIN_PHYSICAL)
        .max(min_y)
        .min(bottom_position.y + bottom_size.height as i32);

    popup
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| format!("Failed to size the process manager: {error}"))?;
    popup
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| format!("Failed to position the process manager: {error}"))?;
    popup
        .show()
        .map_err(|error| format!("Failed to show the process manager: {error}"))?;
    popup
        .set_focus()
        .map_err(|error| format!("Failed to focus the process manager: {error}"))?;
    popup
        .emit(PROCESS_MANAGER_OPEN_EVENT, ())
        .map_err(|error| format!("Failed to publish process manager open event: {error}"))
}

#[tauri::command]
pub fn hide_process_manager(app_handle: AppHandle) -> Result<(), String> {
    let popup = app_handle
        .get_webview_window(PROCESS_MANAGER_LABEL)
        .ok_or_else(|| "Process manager window is unavailable".to_string())?;
    popup
        .emit(PROCESS_MANAGER_CLOSED_EVENT, ())
        .map_err(|error| format!("Failed to publish process manager closed event: {error}"))?;
    popup
        .hide()
        .map_err(|error| format!("Failed to hide the process manager: {error}"))
}

#[tauri::command]
pub fn list_processes() -> Result<Vec<ProcessInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        windows_impl::list_processes()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(Vec::new())
    }
}

#[tauri::command]
pub fn kill_process(pid: u32) -> Result<(), String> {
    if !is_pid_killable(pid, std::process::id()) {
        return Err(format!("Refusing to terminate protected process {pid}"));
    }

    #[cfg(target_os = "windows")]
    {
        windows_impl::kill_process(pid)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = pid;
        Err("Process termination is only supported on Windows".to_string())
    }
}

fn is_pid_killable(pid: u32, current_pid: u32) -> bool {
    pid != 0 && pid != current_pid
}

fn cpu_percent_from_snapshots(
    previous: Option<&ProcessCpuSnapshot>,
    current: &ProcessCpuSnapshot,
    logical_processors: u32,
) -> Option<f64> {
    let previous = previous?;
    let elapsed = current.observed_at.duration_since(previous.observed_at);
    if elapsed.is_zero() || logical_processors == 0 {
        return None;
    }

    let cpu_delta = current
        .cpu_time_ticks
        .saturating_sub(previous.cpu_time_ticks) as f64;
    let wall_ticks = elapsed.as_secs_f64() * 10_000_000.0 * f64::from(logical_processors);
    if wall_ticks <= 0.0 {
        return None;
    }

    Some(((cpu_delta / wall_ticks) * 100.0).clamp(0.0, 100.0))
}

fn retain_process_snapshots(pids: impl Iterator<Item = u32>) {
    let snapshots = PROCESS_CPU_SNAPSHOTS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut snapshots) = snapshots.lock() else {
        return;
    };
    let pids = pids.collect::<std::collections::HashSet<_>>();
    snapshots.retain(|pid, _| pids.contains(pid));
}

fn record_cpu_snapshot(pid: u32, snapshot: ProcessCpuSnapshot) -> Option<f64> {
    let logical_processors = std::thread::available_parallelism()
        .map(|count| count.get() as u32)
        .unwrap_or(1);
    let snapshots = PROCESS_CPU_SNAPSHOTS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut snapshots) = snapshots.lock() else {
        return None;
    };
    let cpu_percent =
        cpu_percent_from_snapshots(snapshots.get(&pid), &snapshot, logical_processors);
    snapshots.insert(pid, snapshot);
    cpu_percent
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::{
        is_pid_killable, record_cpu_snapshot, retain_process_snapshots, ProcessCpuSnapshot,
        ProcessInfo,
    };
    use std::mem::{size_of, zeroed};
    use std::time::Instant;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    };

    pub(super) fn list_processes() -> Result<Vec<ProcessInfo>, String> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
            .map_err(|error| format!("Failed to snapshot processes: {error}"))?;
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut processes = Vec::new();
        let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry).is_ok() };

        while has_entry {
            processes.push(process_info_from_entry(&entry));
            has_entry = unsafe { Process32NextW(snapshot, &mut entry).is_ok() };
        }

        unsafe {
            let _ = CloseHandle(snapshot);
        }

        processes.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
                .then_with(|| left.pid.cmp(&right.pid))
        });
        retain_process_snapshots(processes.iter().map(|process| process.pid));
        Ok(processes)
    }

    pub(super) fn kill_process(pid: u32) -> Result<(), String> {
        let process_handle = unsafe { OpenProcess(PROCESS_TERMINATE, false, pid) }
            .map_err(|error| format!("Failed to open process {pid} for termination: {error}"))?;
        let result = unsafe { TerminateProcess(process_handle, 1) }
            .map_err(|error| format!("Failed to terminate process {pid}: {error}"));
        unsafe {
            let _ = CloseHandle(process_handle);
        }
        result
    }

    fn process_info_from_entry(entry: &PROCESSENTRY32W) -> ProcessInfo {
        let pid = entry.th32ProcessID;
        let process_handle =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok();
        let process_name = wide_c_string(&entry.szExeFile)
            .trim_end_matches(".exe")
            .to_string();
        let cpu_percent = process_cpu_snapshot(process_handle)
            .and_then(|snapshot| record_cpu_snapshot(pid, snapshot));
        let current_pid = std::process::id();
        let process = ProcessInfo {
            pid,
            parent_pid: non_zero_parent_pid(entry.th32ParentProcessID),
            name: if process_name.trim().is_empty() {
                format!("Process {pid}")
            } else {
                process_name
            },
            executable_path: process_handle.and_then(process_image_path),
            cpu_percent,
            memory_bytes: process_handle.and_then(process_memory_bytes),
            thread_count: Some(entry.cntThreads),
            start_time_ms: process_handle.and_then(process_start_time_ms),
            status: "running".to_string(),
            is_killable: is_pid_killable(pid, current_pid),
        };

        if let Some(process_handle) = process_handle {
            unsafe {
                let _ = CloseHandle(process_handle);
            }
        }

        process
    }

    fn non_zero_parent_pid(pid: u32) -> Option<u32> {
        (pid != 0).then_some(pid)
    }

    fn process_cpu_snapshot(process_handle: Option<HANDLE>) -> Option<ProcessCpuSnapshot> {
        let process_handle = process_handle?;
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
        Some(ProcessCpuSnapshot {
            observed_at: Instant::now(),
            cpu_time_ticks: filetime_ticks(kernel_time) + filetime_ticks(user_time),
        })
    }

    fn process_start_time_ms(process_handle: HANDLE) -> Option<u64> {
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
        filetime_to_unix_ms(creation_time)
    }

    fn process_memory_bytes(process_handle: HANDLE) -> Option<u64> {
        let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { zeroed() };
        counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if !unsafe {
            K32GetProcessMemoryInfo(
                process_handle,
                &mut counters,
                size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            )
        }
        .as_bool()
        {
            return None;
        }
        Some(counters.WorkingSetSize as u64)
    }

    fn process_image_path(process_handle: HANDLE) -> Option<String> {
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
        Some(String::from_utf16_lossy(&buffer[..size as usize]))
    }

    fn wide_c_string(buffer: &[u16]) -> String {
        let length = buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..length])
    }

    fn filetime_ticks(filetime: FILETIME) -> u64 {
        ((filetime.dwHighDateTime as u64) << 32) | filetime.dwLowDateTime as u64
    }

    fn filetime_to_unix_ms(filetime: FILETIME) -> Option<u64> {
        const UNIX_EPOCH_FILETIME_TICKS: u64 = 116_444_736_000_000_000;
        let ticks = filetime_ticks(filetime);
        ticks
            .checked_sub(UNIX_EPOCH_FILETIME_TICKS)
            .map(|unix_ticks| unix_ticks / 10_000)
    }
}

#[cfg(test)]
mod tests {
    use super::{cpu_percent_from_snapshots, is_pid_killable, ProcessCpuSnapshot};
    use std::time::{Duration, Instant};

    #[test]
    fn refuses_to_kill_system_or_current_process() {
        assert!(!is_pid_killable(0, 42));
        assert!(!is_pid_killable(42, 42));
        assert!(is_pid_killable(43, 42));
    }

    #[test]
    fn computes_cpu_percent_from_elapsed_process_ticks() {
        let now = Instant::now();
        let previous = ProcessCpuSnapshot {
            observed_at: now,
            cpu_time_ticks: 1_000,
        };
        let current = ProcessCpuSnapshot {
            observed_at: now + Duration::from_secs(1),
            cpu_time_ticks: 2_000_000,
        };

        let percent = cpu_percent_from_snapshots(Some(&previous), &current, 4)
            .expect("cpu percentage should be computed");

        assert!(percent > 0.0);
        assert!(percent < 100.0);
    }
}
