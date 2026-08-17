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
    pub parent_name: Option<String>,
    pub name: String,
    pub icon_data_url: Option<String>,
    pub executable_path: Option<String>,
    pub command_line: Option<String>,
    pub listening_ports: Vec<u16>,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub memory_percent: Option<f64>,
    pub gpu_percent: Option<f64>,
    pub thread_count: Option<u32>,
    pub start_time_ms: Option<u64>,
    pub child_process_count: u32,
    pub descendant_process_count: u32,
    pub workspace_hint: Option<ProcessWorkspaceHint>,
    pub status: String,
    pub is_killable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessWorkspaceHint {
    pub kind: String,
    pub label: String,
    pub path: Option<String>,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(not(test), allow(dead_code))]
pub struct ProcessKillPlan {
    pub target_pid: u32,
    pub mode: String,
    pub affected_pids: Vec<u32>,
    pub descendant_pids: Vec<u32>,
    pub warnings: Vec<String>,
    pub requires_second_confirmation: bool,
    pub can_execute: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessKillConfirmation {
    pub confirmed_target_pid: u32,
    pub mode: String,
    pub affected_pids: Vec<u32>,
    pub descendant_pids: Vec<u32>,
    pub acknowledged_warning_count: usize,
    pub requires_second_confirmation: bool,
    pub can_execute: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowProcessManagerRequest {
    pub anchor_left: f64,
    pub anchor_width: f64,
    pub focus_pid: Option<u32>,
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
        .emit(PROCESS_MANAGER_OPEN_EVENT, request.focus_pid)
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
pub fn kill_process(pid: u32, confirmation: Option<ProcessKillConfirmation>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let processes = windows_impl::list_processes()?;
        validate_kill_guardrail_execution(
            &processes,
            pid,
            confirmation.as_ref(),
            std::process::id(),
        )?;
        windows_impl::kill_process(pid)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (pid, confirmation);
        Err("Process termination is only supported on Windows".to_string())
    }
}

fn is_pid_killable(pid: u32, current_pid: u32) -> bool {
    pid != 0 && pid != current_pid
}

fn enrich_process_tree(processes: &mut [ProcessInfo]) {
    let by_pid = processes
        .iter()
        .map(|process| (process.pid, process.name.clone()))
        .collect::<HashMap<_, _>>();
    let children_by_parent = children_by_parent(processes);

    for process in processes {
        process.parent_name = process
            .parent_pid
            .and_then(|parent_pid| by_pid.get(&parent_pid).cloned());
        let children = children_by_parent
            .get(&process.pid)
            .cloned()
            .unwrap_or_default();
        process.child_process_count = children.len() as u32;
        process.descendant_process_count =
            descendant_pids(process.pid, &children_by_parent).len() as u32;
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn build_kill_guardrail_plan(
    processes: &[ProcessInfo],
    target_pid: u32,
    include_tree_requested: bool,
    current_pid: u32,
) -> Result<ProcessKillPlan, String> {
    let target = processes
        .iter()
        .find(|process| process.pid == target_pid)
        .ok_or_else(|| format!("Process {target_pid} is no longer visible"))?;
    if !is_pid_killable(target.pid, current_pid) || !target.is_killable {
        return Err(format!(
            "Refusing to terminate protected process {target_pid}"
        ));
    }

    let children_by_parent = children_by_parent(processes);
    let descendants = descendant_pids(target_pid, &children_by_parent);
    let mut warnings = Vec::new();

    if let Some(workspace_hint) = &target.workspace_hint {
        warnings.push(format!(
            "Target process is associated with workspace {}",
            workspace_hint.label
        ));
    }

    if descendants.iter().any(|descendant_pid| {
        processes
            .iter()
            .any(|process| process.pid == *descendant_pid && process.workspace_hint.is_some())
    }) {
        warnings
            .push("Process tree includes workspace-associated descendant process(es)".to_string());
    }

    if include_tree_requested {
        warnings.push(
            "Tree termination is plan-only; JasonShell will not kill descendant processes by default"
                .to_string(),
        );
        let mut affected_pids = vec![target_pid];
        affected_pids.extend(descendants.iter().copied());
        return Ok(ProcessKillPlan {
            target_pid,
            mode: "tree-plan".to_string(),
            affected_pids,
            descendant_pids: descendants,
            warnings,
            requires_second_confirmation: true,
            can_execute: false,
        });
    }

    if !descendants.is_empty() {
        warnings.push(format!(
            "Single-process kill leaves {} descendant process(es) running",
            descendants.len()
        ));
    }

    Ok(ProcessKillPlan {
        target_pid,
        mode: "single".to_string(),
        affected_pids: vec![target_pid],
        descendant_pids: descendants,
        warnings,
        requires_second_confirmation: true,
        can_execute: true,
    })
}

#[cfg_attr(not(test), allow(dead_code))]
fn validate_kill_guardrail_execution(
    processes: &[ProcessInfo],
    target_pid: u32,
    confirmation: Option<&ProcessKillConfirmation>,
    current_pid: u32,
) -> Result<ProcessKillPlan, String> {
    let confirmation = confirmation.ok_or_else(|| {
        format!("Refusing to terminate process {target_pid} without guardrail confirmation")
    })?;
    if confirmation.mode == "tree-plan" {
        let _ = build_kill_guardrail_plan(processes, target_pid, true, current_pid)?;
        return Err(
            "Refusing to execute tree termination; tree kill is plan-only in JasonShell"
                .to_string(),
        );
    }
    if confirmation.mode != "single" {
        return Err(format!(
            "Refusing to terminate process {target_pid} with unsupported kill mode {}",
            confirmation.mode
        ));
    }

    let plan = build_kill_guardrail_plan(processes, target_pid, false, current_pid)?;
    if !plan.can_execute || !confirmation.can_execute {
        return Err(format!(
            "Refusing to execute guarded kill plan for {target_pid}"
        ));
    }
    if confirmation.confirmed_target_pid != plan.target_pid
        || confirmation.mode != plan.mode
        || confirmation.affected_pids != plan.affected_pids
        || confirmation.descendant_pids != plan.descendant_pids
        || confirmation.requires_second_confirmation != plan.requires_second_confirmation
        || confirmation.acknowledged_warning_count != plan.warnings.len()
    {
        return Err(format!(
            "Refusing to terminate process {target_pid}; guardrail confirmation is stale or incomplete"
        ));
    }

    Ok(plan)
}

fn children_by_parent(processes: &[ProcessInfo]) -> HashMap<u32, Vec<u32>> {
    let visible_pids = processes
        .iter()
        .map(|process| process.pid)
        .collect::<std::collections::HashSet<_>>();
    let mut children = HashMap::<u32, Vec<u32>>::new();
    for process in processes {
        let Some(parent_pid) = process.parent_pid else {
            continue;
        };
        if visible_pids.contains(&parent_pid) {
            children.entry(parent_pid).or_default().push(process.pid);
        }
    }
    children
}

fn descendant_pids(pid: u32, children_by_parent: &HashMap<u32, Vec<u32>>) -> Vec<u32> {
    let mut descendants = Vec::new();
    let mut stack = children_by_parent.get(&pid).cloned().unwrap_or_default();
    let mut visited = std::collections::HashSet::new();

    while let Some(child_pid) = stack.pop() {
        if !visited.insert(child_pid) {
            continue;
        }
        descendants.push(child_pid);
        if let Some(children) = children_by_parent.get(&child_pid) {
            stack.extend(children.iter().copied());
        }
    }

    descendants.sort_unstable();
    descendants
}

fn workspace_hint_from_metadata(
    executable_path: Option<&str>,
    command_line: Option<&str>,
) -> Option<ProcessWorkspaceHint> {
    let values = [executable_path, command_line];
    if let Ok(current_dir) = std::env::current_dir() {
        let current_path = current_dir.to_string_lossy().to_string();
        let current_normalized = normalize_path_for_match(&current_path);
        if !current_normalized.is_empty()
            && values
                .iter()
                .flatten()
                .any(|value| normalize_path_for_match(value).contains(current_normalized.as_str()))
        {
            let label = current_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("current workspace")
                .to_string();
            return Some(ProcessWorkspaceHint {
                kind: "path-associated".to_string(),
                label,
                path: Some(current_path),
                source: "current-process-directory".to_string(),
            });
        }
    }

    for value in values.iter().flatten() {
        if let Some((label, path)) = dev_workspace_from_text(value) {
            return Some(ProcessWorkspaceHint {
                kind: "path-associated".to_string(),
                label,
                path: Some(path),
                source: "process-path".to_string(),
            });
        }
    }

    let mentions_jasonshell = values
        .iter()
        .flatten()
        .any(|value| value.to_ascii_lowercase().contains("jasonshell"));
    mentions_jasonshell.then(|| ProcessWorkspaceHint {
        kind: "jasonshell-associated".to_string(),
        label: "JasonShell".to_string(),
        path: None,
        source: "process-metadata".to_string(),
    })
}

fn dev_workspace_from_text(value: &str) -> Option<(String, String)> {
    let normalized = normalize_path_for_match(value);
    let marker = "c:\\dev\\";
    let start = normalized.find(marker)?;
    let tail = &normalized[start + marker.len()..];
    let label = tail
        .split(|character: char| character == '\\' || character == '"' || character.is_whitespace())
        .find(|segment| !segment.is_empty())?;
    Some((label.to_string(), format!("C:\\dev\\{label}")))
}

fn normalize_path_for_match(value: &str) -> String {
    value.replace('/', "\\").to_ascii_lowercase()
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

fn memory_percent_from_bytes(
    memory_bytes: Option<u64>,
    total_memory_bytes: Option<u64>,
) -> Option<f64> {
    let memory_bytes = memory_bytes?;
    let total_memory_bytes = total_memory_bytes?;
    if total_memory_bytes == 0 {
        return None;
    }

    Some(((memory_bytes as f64 / total_memory_bytes as f64) * 100.0).clamp(0.0, 100.0))
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
        enrich_process_tree, is_pid_killable, record_cpu_snapshot, retain_process_snapshots,
        workspace_hint_from_metadata, ProcessCpuSnapshot, ProcessInfo,
    };
    use crate::task_windows::bounded_string_cache::BoundedStringCache;
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::mem::{size_of, zeroed};
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, NTSTATUS};
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, MIB_TCP_STATE_LISTEN,
        TCP_TABLE_OWNER_PID_LISTENER,
    };
    use windows::Win32::Networking::WinSock::AF_INET;
    use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
        PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
    };
    use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, TerminateProcess, PEB,
        PROCESS_BASIC_INFORMATION, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_TERMINATE, PROCESS_VM_READ, RTL_USER_PROCESS_PARAMETERS,
    };

    type ProcessIconCache = Mutex<BoundedStringCache<String>>;

    static PROCESS_ICON_DATA_URLS: OnceLock<ProcessIconCache> = OnceLock::new();
    const PROCESS_ICON_CACHE_CAPACITY: usize = 128;
    const PROCESS_ICON_CACHE_TTL: Duration = Duration::from_secs(30 * 60);
    const PROCESS_ICON_CACHE_NEGATIVE_TTL: Duration = Duration::from_secs(30);

    pub(super) fn list_processes() -> Result<Vec<ProcessInfo>, String> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
            .map_err(|error| format!("Failed to snapshot processes: {error}"))?;
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut processes = Vec::new();
        let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry).is_ok() };
        let listening_ports = listening_ports_by_pid();
        let gpu_percent = gpu_percent_by_pid();
        let observed_at = Instant::now();
        let total_memory_bytes = total_physical_memory_bytes();

        while has_entry {
            processes.push(process_info_from_entry(
                &entry,
                &listening_ports,
                &gpu_percent,
                observed_at,
                total_memory_bytes,
            ));
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
        enrich_process_tree(&mut processes);
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

    fn process_info_from_entry(
        entry: &PROCESSENTRY32W,
        listening_ports: &HashMap<u32, Vec<u16>>,
        gpu_percent: &HashMap<u32, f64>,
        observed_at: Instant,
        total_memory_bytes: Option<u64>,
    ) -> ProcessInfo {
        let pid = entry.th32ProcessID;
        let process_handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
                false,
                pid,
            )
        }
        .ok()
        .or_else(|| unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok());
        let process_name = wide_c_string(&entry.szExeFile)
            .trim_end_matches(".exe")
            .to_string();
        let cpu_percent = process_cpu_snapshot(process_handle, observed_at)
            .and_then(|snapshot| record_cpu_snapshot(pid, snapshot));
        let memory_bytes = process_handle.and_then(process_memory_bytes);
        let current_pid = std::process::id();
        let executable_path = process_handle.and_then(process_image_path);
        let icon_data_url = process_icon_data_url(executable_path.as_deref());
        let command_line = process_handle.and_then(process_command_line);
        let workspace_hint =
            workspace_hint_from_metadata(executable_path.as_deref(), command_line.as_deref());
        let process = ProcessInfo {
            pid,
            parent_pid: non_zero_parent_pid(entry.th32ParentProcessID),
            parent_name: None,
            name: if process_name.trim().is_empty() {
                format!("Process {pid}")
            } else {
                process_name
            },
            icon_data_url,
            executable_path,
            command_line,
            listening_ports: listening_ports.get(&pid).cloned().unwrap_or_default(),
            cpu_percent,
            memory_bytes,
            memory_percent: super::memory_percent_from_bytes(memory_bytes, total_memory_bytes),
            gpu_percent: gpu_percent.get(&pid).copied(),
            thread_count: Some(entry.cntThreads),
            start_time_ms: process_handle.and_then(process_start_time_ms),
            child_process_count: 0,
            descendant_process_count: 0,
            workspace_hint,
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

    fn process_icon_data_url(executable_path: Option<&str>) -> Option<String> {
        let cache = PROCESS_ICON_DATA_URLS.get_or_init(|| {
            Mutex::new(BoundedStringCache::new(
                PROCESS_ICON_CACHE_CAPACITY,
                PROCESS_ICON_CACHE_TTL,
                PROCESS_ICON_CACHE_NEGATIVE_TTL,
            ))
        });
        process_icon_data_url_from_cache_with_extractor(
            cache,
            executable_path,
            resolve_process_icon_data_url,
        )
    }

    fn process_icon_data_url_from_cache_with_extractor<F>(
        cache: &ProcessIconCache,
        executable_path: Option<&str>,
        extractor: F,
    ) -> Option<String>
    where
        F: FnOnce(&Path) -> Option<String>,
    {
        let executable_path = executable_path?.trim();
        if executable_path.is_empty() {
            return None;
        }
        let icon_cache_key = executable_path.to_string();

        if let Some(cached) = cached_process_icon_data_url(cache, &icon_cache_key) {
            return cached;
        }

        let icon_data_url = extractor(Path::new(&icon_cache_key));
        store_process_icon_data_url(cache, &icon_cache_key, icon_data_url.clone());
        icon_data_url
    }

    fn cached_process_icon_data_url(
        cache: &ProcessIconCache,
        executable_path: &str,
    ) -> Option<Option<String>> {
        let Ok(mut cache) = cache.lock() else {
            return None;
        };
        cache.get_cloned(&executable_path.to_string())
    }

    fn resolve_process_icon_data_url(executable_path: &Path) -> Option<String> {
        crate::task_windows::shell_file_icon_data_url(executable_path).ok()
    }

    fn store_process_icon_data_url(
        cache: &ProcessIconCache,
        executable_path: &str,
        icon_data_url: Option<String>,
    ) {
        let Ok(mut cache) = cache.lock() else {
            return;
        };
        cache.insert(executable_path.to_string(), icon_data_url)
    }

    fn gpu_percent_by_pid() -> HashMap<u32, f64> {
        let mut query = PDH_HQUERY::default();
        if unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) } != 0 || query.is_invalid() {
            return HashMap::new();
        }

        let result = collect_gpu_percent_by_pid(query);
        unsafe {
            let _ = PdhCloseQuery(query);
        }
        result
    }

    fn collect_gpu_percent_by_pid(query: PDH_HQUERY) -> HashMap<u32, f64> {
        let counter_path = wide_null(r"\GPU Engine(*)\Utilization Percentage");
        let mut counter = PDH_HCOUNTER::default();
        if unsafe { PdhAddEnglishCounterW(query, PCWSTR(counter_path.as_ptr()), 0, &mut counter) }
            != 0
        {
            return HashMap::new();
        }

        if unsafe { PdhCollectQueryData(query) } != 0 {
            return HashMap::new();
        }
        std::thread::sleep(Duration::from_millis(60));
        if unsafe { PdhCollectQueryData(query) } != 0 {
            return HashMap::new();
        }

        formatted_gpu_counter_array(counter)
    }

    fn formatted_gpu_counter_array(counter: PDH_HCOUNTER) -> HashMap<u32, f64> {
        let mut buffer_size = 0_u32;
        let mut item_count = 0_u32;
        let sizing = unsafe {
            PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                None,
            )
        };
        if sizing != PDH_MORE_DATA || buffer_size == 0 || item_count == 0 {
            return HashMap::new();
        }

        let mut buffer = vec![0_u8; buffer_size as usize];
        let result = unsafe {
            PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                Some(buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>()),
            )
        };
        if result != 0 {
            return HashMap::new();
        }

        let items = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                item_count as usize,
            )
        };
        let mut by_pid = HashMap::<u32, f64>::new();
        for item in items {
            if item.FmtValue.CStatus != PDH_CSTATUS_VALID_DATA {
                continue;
            }
            let instance_name = unsafe { wide_ptr_string(item.szName.0) };
            let Some(pid) = pid_from_gpu_engine_instance(&instance_name) else {
                continue;
            };
            let value = unsafe { item.FmtValue.Anonymous.doubleValue };
            if !value.is_finite() || value <= 0.0 {
                continue;
            }
            by_pid
                .entry(pid)
                .and_modify(|current| *current = (*current + value).clamp(0.0, 100.0))
                .or_insert(value.clamp(0.0, 100.0));
        }
        by_pid
    }

    fn pid_from_gpu_engine_instance(instance_name: &str) -> Option<u32> {
        let lower = instance_name.to_ascii_lowercase();
        let tail = lower.split("pid_").nth(1)?;
        let digits = tail
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect::<String>();
        (!digits.is_empty()).then(|| digits.parse::<u32>().ok())?
    }

    fn listening_ports_by_pid() -> HashMap<u32, Vec<u16>> {
        let mut size = 0_u32;
        unsafe {
            let _ = GetExtendedTcpTable(
                None,
                &mut size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            );
        }
        if size == 0 {
            return HashMap::new();
        }

        let mut buffer = vec![0_u8; size as usize];
        let result = unsafe {
            GetExtendedTcpTable(
                Some(buffer.as_mut_ptr().cast::<c_void>()),
                &mut size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            )
        };
        if result != 0 {
            return HashMap::new();
        }

        let table = buffer.as_ptr().cast::<MIB_TCPTABLE_OWNER_PID>();
        let row_count = unsafe { (*table).dwNumEntries as usize };
        let rows = unsafe {
            std::slice::from_raw_parts(
                (*table).table.as_ptr() as *const MIB_TCPROW_OWNER_PID,
                row_count,
            )
        };
        let mut ports_by_pid = HashMap::<u32, Vec<u16>>::new();
        for row in rows {
            if row.dwState != MIB_TCP_STATE_LISTEN.0 as u32 {
                continue;
            }
            let port = u16::from_be((row.dwLocalPort & 0xffff) as u16);
            if port == 0 {
                continue;
            }
            ports_by_pid.entry(row.dwOwningPid).or_default().push(port);
        }
        for ports in ports_by_pid.values_mut() {
            ports.sort_unstable();
            ports.dedup();
        }
        ports_by_pid
    }

    fn process_command_line(process_handle: HANDLE) -> Option<String> {
        let mut basic_info = PROCESS_BASIC_INFORMATION::default();
        let status = unsafe {
            NtQueryInformationProcess(
                process_handle,
                0,
                (&mut basic_info as *mut PROCESS_BASIC_INFORMATION).cast::<c_void>(),
                size_of::<PROCESS_BASIC_INFORMATION>() as u32,
                std::ptr::null_mut(),
            )
        };
        if status.0 < 0 || basic_info.PebBaseAddress.is_null() {
            return None;
        }

        let peb: PEB =
            read_remote_value(process_handle, basic_info.PebBaseAddress.cast::<c_void>())?;
        if peb.ProcessParameters.is_null() {
            return None;
        }
        let parameters = read_remote_value::<RTL_USER_PROCESS_PARAMETERS>(
            process_handle,
            peb.ProcessParameters.cast::<c_void>(),
        )?;
        let command_line = parameters.CommandLine;
        let char_count = usize::from(command_line.Length) / size_of::<u16>();
        if char_count == 0 || char_count > 32_768 || command_line.Buffer.0.is_null() {
            return None;
        }

        let mut buffer = vec![0_u16; char_count];
        let mut bytes_read = 0_usize;
        unsafe {
            ReadProcessMemory(
                process_handle,
                command_line.Buffer.0.cast::<c_void>(),
                buffer.as_mut_ptr().cast::<c_void>(),
                usize::from(command_line.Length),
                Some(&mut bytes_read),
            )
            .ok()?;
        }
        if bytes_read < usize::from(command_line.Length) {
            return None;
        }

        let command_line = String::from_utf16_lossy(&buffer).trim().to_string();
        (!command_line.is_empty()).then_some(command_line)
    }

    fn read_remote_value<T: Default>(process_handle: HANDLE, address: *const c_void) -> Option<T> {
        let mut value = T::default();
        let mut bytes_read = 0_usize;
        unsafe {
            ReadProcessMemory(
                process_handle,
                address,
                (&mut value as *mut T).cast::<c_void>(),
                size_of::<T>(),
                Some(&mut bytes_read),
            )
            .ok()?;
        }
        (bytes_read == size_of::<T>()).then_some(value)
    }

    extern "system" {
        fn NtQueryInformationProcess(
            process_handle: HANDLE,
            process_information_class: u32,
            process_information: *mut c_void,
            process_information_length: u32,
            return_length: *mut u32,
        ) -> NTSTATUS;
    }

    fn process_cpu_snapshot(
        process_handle: Option<HANDLE>,
        observed_at: Instant,
    ) -> Option<ProcessCpuSnapshot> {
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
            observed_at,
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

    fn total_physical_memory_bytes() -> Option<u64> {
        let mut status = MEMORYSTATUSEX {
            dwLength: size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        unsafe { GlobalMemoryStatusEx(&mut status).ok()? };
        (status.ullTotalPhys > 0).then_some(status.ullTotalPhys)
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

    fn wide_null(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn wide_ptr_string(value: *const u16) -> String {
        if value.is_null() {
            return String::new();
        }
        let mut length = 0_usize;
        while unsafe { *value.add(length) } != 0 {
            length += 1;
        }
        String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(value, length) }.as_ref())
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

    #[cfg(test)]
    mod icon_cache_tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{mpsc, Arc, Mutex};
        use std::thread;

        #[test]
        fn process_icon_cache_hit_is_reused_without_extraction() {
            let cache = Mutex::new(BoundedStringCache::new(
                PROCESS_ICON_CACHE_CAPACITY,
                PROCESS_ICON_CACHE_TTL,
                PROCESS_ICON_CACHE_NEGATIVE_TTL,
            ));
            let calls = AtomicUsize::new(0);

            let first = process_icon_data_url_from_cache_with_extractor(
                &cache,
                Some("C:\\Tools\\app.exe"),
                |_| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Some("data:image/png;base64,first".to_string())
                },
            );
            let second = process_icon_data_url_from_cache_with_extractor(
                &cache,
                Some("C:\\Tools\\app.exe"),
                |_| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Some("data:image/png;base64,second".to_string())
                },
            );

            assert_eq!(first.as_deref(), Some("data:image/png;base64,first"));
            assert_eq!(second.as_deref(), Some("data:image/png;base64,first"));
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn process_icon_cache_hit_is_available_while_miss_extracts() {
            let cache = Arc::new(Mutex::new(BoundedStringCache::new(
                PROCESS_ICON_CACHE_CAPACITY,
                PROCESS_ICON_CACHE_TTL,
                PROCESS_ICON_CACHE_NEGATIVE_TTL,
            )));
            cache.lock().unwrap().insert(
                "C:\\Tools\\cached.exe".to_string(),
                Some("data:image/png;base64,cached".to_string()),
            );
            let (started_tx, started_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let slow_cache = Arc::clone(&cache);

            let slow_miss = thread::spawn(move || {
                process_icon_data_url_from_cache_with_extractor(
                    &slow_cache,
                    Some("C:\\Tools\\slow.exe"),
                    |_| {
                        started_tx.send(()).unwrap();
                        release_rx
                            .recv_timeout(Duration::from_secs(2))
                            .expect("test should release slow icon extraction");
                        Some("data:image/png;base64,slow".to_string())
                    },
                )
            });

            started_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("slow icon extraction should start");
            let started_at = Instant::now();
            let cached = process_icon_data_url_from_cache_with_extractor(
                &cache,
                Some("C:\\Tools\\cached.exe"),
                |_| panic!("cache hit must not invoke shell extraction"),
            );

            assert_eq!(cached.as_deref(), Some("data:image/png;base64,cached"));
            assert!(
                started_at.elapsed() < Duration::from_millis(200),
                "cache hit waited behind slow miss for {:?}",
                started_at.elapsed()
            );
            release_tx.send(()).unwrap();
            assert_eq!(
                slow_miss.join().unwrap().as_deref(),
                Some("data:image/png;base64,slow")
            );
        }

        #[test]
        fn process_icon_cache_source_splits_lookup_resolve_and_store() {
            let source = include_str!("process_manager.rs");
            let helper = source
                .split("fn process_icon_data_url_from_cache_with_extractor")
                .nth(1)
                .expect("process icon cache helper should exist");
            let lookup = helper
                .find("cached_process_icon_data_url")
                .expect("helper should look up cached icon first");
            let resolve = helper
                .find("extractor(Path::new(&icon_cache_key))")
                .expect("helper should resolve misses outside cache helpers");
            let store = helper
                .find("store_process_icon_data_url")
                .expect("helper should store resolved miss after extraction");

            assert!(lookup < resolve);
            assert!(resolve < store);

            let lookup_helper = source
                .split("fn cached_process_icon_data_url")
                .nth(1)
                .and_then(|tail| tail.split("fn resolve_process_icon_data_url").next())
                .expect("lookup helper should be isolated");
            let store_helper = source
                .split("fn store_process_icon_data_url")
                .nth(1)
                .and_then(|tail| tail.split("fn gpu_percent_by_pid").next())
                .expect("store helper should be isolated");

            assert!(!lookup_helper.contains("shell_file_icon_data_url"));
            assert!(!store_helper.contains("shell_file_icon_data_url"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_kill_guardrail_plan, cpu_percent_from_snapshots, dev_workspace_from_text,
        enrich_process_tree, is_pid_killable, memory_percent_from_bytes,
        validate_kill_guardrail_execution, workspace_hint_from_metadata, ProcessCpuSnapshot,
        ProcessInfo, ProcessKillConfirmation, ProcessWorkspaceHint,
    };
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
            cpu_time_ticks: 1_000_000,
        };
        let current = ProcessCpuSnapshot {
            observed_at: now + Duration::from_secs(1),
            cpu_time_ticks: 6_000_000,
        };

        let percent = cpu_percent_from_snapshots(Some(&previous), &current, 4)
            .expect("cpu percentage should be computed");

        assert_eq!(percent, 12.5);
    }

    #[test]
    fn computes_memory_percent_from_total_memory() {
        let percent = memory_percent_from_bytes(Some(512), Some(2048))
            .expect("memory percentage should be computed");
        assert_eq!(percent, 25.0);
        assert_eq!(memory_percent_from_bytes(Some(512), Some(0)), None);
        assert_eq!(memory_percent_from_bytes(None, Some(2048)), None);
    }

    #[test]
    fn enriches_parent_and_descendant_process_context() {
        let mut processes = vec![
            process(100, None, "Terminal", true),
            process(110, Some(100), "pwsh", true),
            process(120, Some(110), "node", true),
            process(200, Some(999), "orphan", true),
        ];

        enrich_process_tree(&mut processes);

        assert_eq!(processes[0].child_process_count, 1);
        assert_eq!(processes[0].descendant_process_count, 2);
        assert_eq!(processes[1].parent_name.as_deref(), Some("Terminal"));
        assert_eq!(processes[1].descendant_process_count, 1);
        assert_eq!(processes[3].parent_name, None);
    }

    #[test]
    fn kill_guardrail_plan_keeps_tree_termination_non_executing() {
        let mut processes = vec![
            process(100, None, "Terminal", true),
            process(110, Some(100), "pwsh", true),
            process(120, Some(110), "node", true),
        ];
        enrich_process_tree(&mut processes);

        let single =
            build_kill_guardrail_plan(&processes, 100, false, 999).expect("single plan is valid");
        assert_eq!(single.mode, "single");
        assert_eq!(single.affected_pids, vec![100]);
        assert_eq!(single.descendant_pids, vec![110, 120]);
        assert!(single.can_execute);
        assert!(single.requires_second_confirmation);

        let tree =
            build_kill_guardrail_plan(&processes, 100, true, 999).expect("tree plan is valid");
        assert_eq!(tree.mode, "tree-plan");
        assert_eq!(tree.affected_pids, vec![100, 110, 120]);
        assert!(!tree.can_execute);
        assert!(tree
            .warnings
            .iter()
            .any(|warning| warning.contains("will not kill descendant processes by default")));
    }

    #[test]
    fn kill_guardrail_plan_refuses_protected_processes() {
        let processes = vec![process(100, None, "JasonShell", false)];

        let error = build_kill_guardrail_plan(&processes, 100, false, 999)
            .expect_err("protected process should be rejected");

        assert!(error.contains("protected process 100"));
    }

    #[test]
    fn kill_guardrail_execution_requires_explicit_confirmation() {
        let processes = vec![process(100, None, "node", true)];

        let error = validate_kill_guardrail_execution(&processes, 100, None, 999)
            .expect_err("direct kill without confirmation should be rejected");

        assert!(error.contains("without guardrail confirmation"));
    }

    #[test]
    fn kill_guardrail_execution_refuses_current_pid_even_with_confirmation() {
        let processes = vec![process(100, None, "JasonShell", true)];
        let confirmation = single_confirmation(100, &[], 0);

        let error = validate_kill_guardrail_execution(&processes, 100, Some(&confirmation), 100)
            .expect_err("current process kill should be rejected");

        assert!(error.contains("protected process 100"));
    }

    #[test]
    fn kill_guardrail_execution_rejects_stale_descendant_confirmation() {
        let mut processes = vec![
            process(100, None, "Terminal", true),
            process(110, Some(100), "pwsh", true),
            process(120, Some(110), "node", true),
        ];
        enrich_process_tree(&mut processes);
        let confirmation = single_confirmation(100, &[], 1);

        let error = validate_kill_guardrail_execution(&processes, 100, Some(&confirmation), 999)
            .expect_err("stale descendant plan should be rejected");

        assert!(error.contains("stale or incomplete"));
    }

    #[test]
    fn kill_guardrail_execution_rejects_tree_plan_execution() {
        let mut processes = vec![
            process(100, None, "Terminal", true),
            process(110, Some(100), "pwsh", true),
        ];
        enrich_process_tree(&mut processes);
        let mut confirmation = single_confirmation(100, &[110], 2);
        confirmation.mode = "tree-plan".to_string();
        confirmation.affected_pids = vec![100, 110];
        confirmation.can_execute = false;

        let error = validate_kill_guardrail_execution(&processes, 100, Some(&confirmation), 999)
            .expect_err("tree kill should remain plan-only");

        assert!(error.contains("tree kill is plan-only"));
    }

    #[test]
    fn kill_guardrail_execution_accepts_fresh_single_confirmation() {
        let mut processes = vec![
            process(100, None, "Terminal", true),
            process(110, Some(100), "pwsh", true),
        ];
        enrich_process_tree(&mut processes);
        let confirmation = single_confirmation(100, &[110], 1);

        let plan = validate_kill_guardrail_execution(&processes, 100, Some(&confirmation), 999)
            .expect("fresh single-process confirmation should be executable");

        assert_eq!(plan.mode, "single");
        assert_eq!(plan.descendant_pids, vec![110]);
    }

    #[test]
    fn kill_guardrail_execution_requires_workspace_warning_acknowledgement() {
        let mut processes = vec![process(100, None, "node", true)];
        processes[0].workspace_hint = Some(ProcessWorkspaceHint {
            kind: "path-associated".to_string(),
            label: "jasonshell".to_string(),
            path: Some("C:\\dev\\jasonshell".to_string()),
            source: "process-path".to_string(),
        });
        let confirmation = single_confirmation(100, &[], 0);

        let error = validate_kill_guardrail_execution(&processes, 100, Some(&confirmation), 999)
            .expect_err("workspace-associated process requires warning acknowledgement");

        assert!(error.contains("stale or incomplete"));
    }

    #[test]
    fn workspace_hints_use_current_or_dev_paths_without_workspace_crud() {
        let hint = workspace_hint_from_metadata(
            Some("C:\\tools\\node.exe"),
            Some("node C:\\dev\\jasonshell\\server.js"),
        )
        .expect("dev path should produce a hint");

        assert_eq!(hint.kind, "path-associated");
        assert_eq!(hint.label, "jasonshell");
        assert_eq!(hint.path.as_deref(), Some("C:\\dev\\jasonshell"));
        assert_eq!(
            dev_workspace_from_text("\"C:/dev/example-app/package.json\"")
                .unwrap()
                .0,
            "example-app"
        );
    }

    fn process(pid: u32, parent_pid: Option<u32>, name: &str, is_killable: bool) -> ProcessInfo {
        ProcessInfo {
            pid,
            parent_pid,
            parent_name: None,
            name: name.to_string(),
            icon_data_url: None,
            executable_path: None,
            command_line: None,
            listening_ports: Vec::new(),
            cpu_percent: None,
            memory_bytes: None,
            memory_percent: None,
            gpu_percent: None,
            thread_count: None,
            start_time_ms: None,
            child_process_count: 0,
            descendant_process_count: 0,
            workspace_hint: None,
            status: "running".to_string(),
            is_killable,
        }
    }

    fn single_confirmation(
        target_pid: u32,
        descendant_pids: &[u32],
        acknowledged_warning_count: usize,
    ) -> ProcessKillConfirmation {
        ProcessKillConfirmation {
            confirmed_target_pid: target_pid,
            mode: "single".to_string(),
            affected_pids: vec![target_pid],
            descendant_pids: descendant_pids.to_vec(),
            acknowledged_warning_count,
            requires_second_confirmation: true,
            can_execute: true,
        }
    }
}
