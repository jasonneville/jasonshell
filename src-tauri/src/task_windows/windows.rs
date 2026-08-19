use super::{
    actions, attention,
    icons::{window_icon_data_url, EMPTY_ICON_DATA_URL},
    notifications, TaskbarProcessWindow, TaskbarWindow, TaskbarWindowActivityState,
    TaskbarWindowsSnapshot, TASKBAR_WINDOWS_SNAPSHOT_EVENT,
};
use std::collections::HashMap;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use windows::core::PWSTR;
use windows::Win32::Foundation::{
    CloseHandle, FILETIME, HANDLE, HWND, LPARAM, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, GetProcessTimes, OpenProcess, QueryFullProcessImageNameW,
    WaitForSingleObject, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    GA_ROOTOWNER, GWL_EXSTYLE, GW_OWNER, WINDOW_EX_STYLE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
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
const TASKBAR_SNAPSHOT_REFRESH_CADENCE: Duration = Duration::from_secs(1);
const TASKBAR_REFRESH_NATIVE_COALESCE: Duration = Duration::from_millis(30);
const TASKBAR_REFRESH_MANUAL_COALESCE: Duration = Duration::from_millis(120);

static TASKBAR_SNAPSHOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static TASKBAR_SNAPSHOT_WORKER_STARTED: AtomicBool = AtomicBool::new(false);
static TASKBAR_REFRESH_TX: OnceLock<mpsc::SyncSender<TaskbarRefreshRequest>> = OnceLock::new();
static LAST_TASKBAR_SNAPSHOT: OnceLock<Mutex<Option<TaskbarWindowsSnapshot>>> = OnceLock::new();
static LAST_TASKBAR_SNAPSHOT_AT: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
const TASKBAR_SNAPSHOT_MAX_AGE: Duration = Duration::from_millis(1_120);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskbarRefreshReason {
    Native,
    Manual,
}

impl TaskbarRefreshReason {
    fn as_str(self) -> &'static str {
        match self {
            TaskbarRefreshReason::Native => "native",
            TaskbarRefreshReason::Manual => "manual",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TaskbarRefreshRequest {
    reason: TaskbarRefreshReason,
    requested_at: Instant,
}

#[derive(Clone, Debug, Default)]
struct TaskbarRefreshDiagnostics {
    last_reason: Option<TaskbarRefreshReason>,
    last_signal_at: Option<Instant>,
    last_snapshot_at: Option<Instant>,
    last_latency_ms: u64,
    coalesced_count: u64,
}

static TASKBAR_REFRESH_DIAGNOSTICS: OnceLock<Mutex<TaskbarRefreshDiagnostics>> = OnceLock::new();

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
    pub(super) is_shell_process: bool,
    pub(super) is_visible: bool,
    pub(super) ex_style: WINDOW_EX_STYLE,
    pub(super) process_id: u32,
}

pub(super) fn list_open_task_windows() -> Result<Vec<TaskbarWindow>, String> {
    if let Some(snapshot) = last_taskbar_snapshot_if_fresh() {
        return Ok(snapshot.windows);
    }
    refresh_taskbar_snapshot_now(None)?;
    Ok(last_taskbar_snapshot_if_fresh()
        .map(|snapshot| snapshot.windows)
        .unwrap_or_default())
}

pub(super) fn ensure_taskbar_snapshot_worker_started(app: AppHandle) {
    if TASKBAR_SNAPSHOT_WORKER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    // One pending request is enough: the next scan publishes complete state.
    let (tx, rx) = mpsc::sync_channel::<TaskbarRefreshRequest>(8);
    let _ = TASKBAR_REFRESH_TX.set(tx);
    thread::spawn(move || {
        let mut last_refresh_at = Instant::now();
        loop {
            let next_refresh_at = last_refresh_at + TASKBAR_SNAPSHOT_REFRESH_CADENCE;
            match rx.recv_timeout(next_refresh_at.saturating_duration_since(Instant::now())) {
                Ok(mut request) => {
                    let mut due = request_due_at(request);
                    loop {
                        let now = Instant::now();
                        if now >= due {
                            break;
                        }
                        match rx.recv_timeout(due.saturating_duration_since(now)) {
                            Ok(next) => {
                                request = coalesce_request(request, next);
                                due = request_due_at(request);
                                if let Ok(mut diag) = TASKBAR_REFRESH_DIAGNOSTICS
                                    .get_or_init(
                                        || Mutex::new(TaskbarRefreshDiagnostics::default()),
                                    )
                                    .lock()
                                {
                                    diag.coalesced_count += 1;
                                }
                            }
                            Err(mpsc::RecvTimeoutError::Timeout) => break,
                            Err(mpsc::RecvTimeoutError::Disconnected) => return,
                        }
                    }
                    if let Ok(mut diag) = TASKBAR_REFRESH_DIAGNOSTICS
                        .get_or_init(|| Mutex::new(TaskbarRefreshDiagnostics::default()))
                        .lock()
                    {
                        diag.last_reason = Some(request.reason);
                        diag.last_signal_at = Some(request.requested_at);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
            let _ = refresh_taskbar_snapshot_now(Some(&app));
            if let Ok(mut diag) = TASKBAR_REFRESH_DIAGNOSTICS
                .get_or_init(|| Mutex::new(TaskbarRefreshDiagnostics::default()))
                .lock()
            {
                if let Some(signal_at) = diag.last_signal_at {
                    diag.last_latency_ms = signal_at.elapsed().as_millis() as u64;
                }
                diag.last_snapshot_at = Some(Instant::now());
            }
            last_refresh_at = Instant::now();
        }
    });
}

pub(super) fn request_taskbar_snapshot_refresh() {
    if let Some(tx) = TASKBAR_REFRESH_TX.get() {
        if let Ok(mut diag) = TASKBAR_REFRESH_DIAGNOSTICS
            .get_or_init(|| Mutex::new(TaskbarRefreshDiagnostics::default()))
            .lock()
        {
            diag.last_reason = Some(TaskbarRefreshReason::Manual);
            diag.last_signal_at = Some(Instant::now());
        }
        let _ = tx.try_send(TaskbarRefreshRequest {
            reason: TaskbarRefreshReason::Manual,
            requested_at: Instant::now(),
        });
    }
}

pub(super) fn request_taskbar_snapshot_refresh_native(signal_timestamp: Instant) {
    if let Some(tx) = TASKBAR_REFRESH_TX.get() {
        if let Ok(mut diag) = TASKBAR_REFRESH_DIAGNOSTICS
            .get_or_init(|| Mutex::new(TaskbarRefreshDiagnostics::default()))
            .lock()
        {
            diag.last_reason = Some(TaskbarRefreshReason::Native);
            diag.last_signal_at = Some(signal_timestamp);
        }
        let _ = tx.try_send(TaskbarRefreshRequest {
            reason: TaskbarRefreshReason::Native,
            requested_at: signal_timestamp,
        });
    }
}

pub(super) fn taskbar_snapshot_diagnostics_snapshot(
) -> super::diagnostics::SnapshotPipelineDiagnosticsSnapshot {
    let diag = TASKBAR_REFRESH_DIAGNOSTICS
        .get_or_init(|| Mutex::new(TaskbarRefreshDiagnostics::default()))
        .lock()
        .ok()
        .map(|diag| diag.clone())
        .unwrap_or_default();
    let (sequence, refreshed_at_ms) = LAST_TASKBAR_SNAPSHOT_AT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|at| at.as_ref().copied())
        .map(|at| {
            let age_ms = at.elapsed().as_millis() as u64;
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0);
            (
                TASKBAR_SNAPSHOT_SEQUENCE.load(Ordering::SeqCst),
                now_ms.saturating_sub(age_ms),
            )
        })
        .unwrap_or((TASKBAR_SNAPSHOT_SEQUENCE.load(Ordering::SeqCst), 0));
    super::diagnostics::SnapshotPipelineDiagnosticsSnapshot {
        sequence,
        refresh_reason: diag
            .last_reason
            .map(|r| r.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        refreshed_at_ms: refreshed_at_ms,
        latency_ms: diag.last_latency_ms,
    }
}

fn request_due_at(request: TaskbarRefreshRequest) -> Instant {
    request.requested_at
        + match request.reason {
            TaskbarRefreshReason::Native => TASKBAR_REFRESH_NATIVE_COALESCE,
            TaskbarRefreshReason::Manual => TASKBAR_REFRESH_MANUAL_COALESCE,
        }
}

fn coalesce_request(
    old: TaskbarRefreshRequest,
    new: TaskbarRefreshRequest,
) -> TaskbarRefreshRequest {
    if request_due_at(new) <= request_due_at(old) {
        new
    } else {
        old
    }
}

pub(super) fn refresh_taskbar_snapshot_now(app: Option<&AppHandle>) -> Result<(), String> {
    let current_process_id = std::process::id();
    let foreground = unsafe { GetForegroundWindow() };
    let mut handles = Vec::new();

    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            LPARAM((&mut handles as *mut Vec<HWND>) as isize),
        )
        .map_err(|error| format!("Failed to enumerate top-level windows: {error}"))?;
    }

    let mut windows = Vec::new();
    let mut visible_attention_identities = Vec::new();
    let mut cpu_time_by_process_id: HashMap<u32, Option<u64>> = HashMap::new();
    for hwnd in handles {
        let Some(candidate) = build_window_candidate(hwnd, foreground, current_process_id)? else {
            continue;
        };

        if !is_taskbar_candidate(&candidate, current_process_id) {
            continue;
        }

        let native_hwnd = candidate.hwnd;
        let hwnd = candidate.hwnd_string();
        let title = candidate.title;
        let process_name = candidate.process_name;
        let process_path = candidate.process_path.clone();
        let is_active = candidate.is_active;
        let is_minimized = candidate.is_minimized;
        let process_id = candidate.process_id;
        if is_active {
            if let Some(process_path) = process_path.as_deref() {
                notifications::clear_notifications_for_process_path(process_path);
            }
        }
        let notification_count =
            notifications::notification_count_for_process_path(process_path.as_deref(), None);
        let attention_identity = attention_identity_for_hwnd(native_hwnd, process_id)?;
        visible_attention_identities.push(attention_identity.clone());
        if is_active {
            attention::clear_taskbar_attention_if_matches(&attention_identity);
        }
        let icon_data_url = window_icon_data_url(native_hwnd, process_path.as_deref())
            .unwrap_or_else(|_| EMPTY_ICON_DATA_URL.to_string());
        let cpu_time_ticks = *cpu_time_by_process_id
            .entry(process_id)
            .or_insert_with(|| process_cpu_time_ticks(process_id));
        let activity_state =
            task_window_activity_state(&hwnd, process_id, &title, &process_name, cpu_time_ticks);

        windows.push(TaskbarWindow {
            hwnd,
            title,
            process_id,
            process_name,
            icon_data_url,
            is_active,
            is_minimized,
            activity_state,
            notification_count,
            attention_state: attention::attention_state_for(&attention_identity),
            toast_count: notification_count,
        });
    }

    attention::reconcile_taskbar_attention(&visible_attention_identities);
    sort_windows_stably(&mut windows);
    retain_activity_snapshots(windows.iter().map(|window| window.hwnd.as_str()));
    let sequence = TASKBAR_SNAPSHOT_SEQUENCE.fetch_add(1, Ordering::SeqCst) + 1;
    if let Ok(mut snapshot) = LAST_TASKBAR_SNAPSHOT
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *snapshot = Some(TaskbarWindowsSnapshot {
            sequence,
            windows: windows.clone(),
        });
    }
    if let Ok(mut snapshot_at) = LAST_TASKBAR_SNAPSHOT_AT
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *snapshot_at = Some(Instant::now());
    }
    if let Some(app) = app {
        let _ = app.emit(
            TASKBAR_WINDOWS_SNAPSHOT_EVENT,
            TaskbarWindowsSnapshot {
                sequence,
                windows: windows.clone(),
            },
        );
    }
    Ok(())
}

pub(super) fn list_taskbar_process_windows() -> Result<Vec<TaskbarProcessWindow>, String> {
    let current_process_id = std::process::id();
    let foreground = unsafe { GetForegroundWindow() };
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
        let Some(candidate) = build_window_candidate(hwnd, foreground, current_process_id)? else {
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

pub(super) fn process_image_path_for_hwnd(hwnd: HWND) -> Result<PathBuf, String> {
    let mut process_id = 0;
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    if process_id == 0 {
        return Err("Task window process id is unavailable".to_string());
    }

    process_image_path(process_id)
        .ok_or_else(|| "Task window executable path is unavailable".to_string())
}

pub(super) fn spawn_task_window_helper(
    hwnd: String,
    pid: u32,
    creation_time: u64,
    canonical_image_path: PathBuf,
) -> Result<(), String> {
    launch_task_window_helper(hwnd, pid, creation_time, canonical_image_path)
}

fn launch_task_window_helper(
    hwnd: String,
    pid: u32,
    creation_time: u64,
    canonical_image_path: PathBuf,
) -> Result<(), String> {
    let helper_exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let args = format!(
        "--task-window-helper {} {} {} {}",
        hwnd,
        pid,
        creation_time,
        super::helper::encode_canonical_path(&canonical_image_path)
    );
    let process_handle = shell_execute_runas_wait(&helper_exe, &args, "task window helper")?;
    let exit_code = wait_for_process_exit(process_handle, std::time::Duration::from_secs(8))?;
    super::helper::helper_exit_code_for_shell_execute_result(exit_code)?;
    Ok(())
}

fn shell_execute_runas_wait(
    executable: &Path,
    arguments: &str,
    operation: &str,
) -> Result<HANDLE, String> {
    let mut execute_info = SHELLEXECUTEINFOW::default();
    execute_info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    execute_info.fMask = SEE_MASK_NOCLOSEPROCESS;
    let verb = wide_null(std::ffi::OsStr::new("runas"));
    let executable_wide = wide_null(executable.as_os_str());
    let arguments_wide = wide_null(std::ffi::OsStr::new(arguments));
    execute_info.lpVerb = windows::core::PCWSTR(verb.as_ptr());
    execute_info.lpFile = windows::core::PCWSTR(executable_wide.as_ptr());
    execute_info.lpParameters = windows::core::PCWSTR(arguments_wide.as_ptr());
    execute_info.nShow = windows::Win32::UI::WindowsAndMessaging::SW_HIDE.0 as i32;
    unsafe { ShellExecuteExW(&mut execute_info) }
        .map_err(|error| shell_execute_error_for_operation(operation, error))?;
    if execute_info.hProcess.0.is_null() {
        return Err(format!(
            "Failed to launch {operation}: process handle unavailable"
        ));
    }
    Ok(execute_info.hProcess)
}

fn wait_for_process_exit(handle: HANDLE, timeout: std::time::Duration) -> Result<u32, String> {
    struct ProcessHandleGuard(HANDLE);
    impl Drop for ProcessHandleGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let _guard = ProcessHandleGuard(handle);
    let wait =
        unsafe { WaitForSingleObject(handle, timeout.as_millis().min(u32::MAX as u128) as u32) };
    match wait {
        WAIT_OBJECT_0 => {
            let mut exit_code = 0u32;
            unsafe { GetExitCodeProcess(handle, &mut exit_code) }
                .map_err(|error| format!("Failed to read helper exit code: {error}"))?;
            Ok(exit_code)
        }
        WAIT_TIMEOUT => Err("Task window helper timed out".to_string()),
        _ => Err("Task window helper wait failed".to_string()),
    }
}

fn shell_execute_error_for_operation(operation: &str, error: windows::core::Error) -> String {
    if shell_execute_status_for_error(error.code()) == ShellExecuteStatus::UacCanceled {
        return "UAC canceled".to_string();
    }
    format!("Failed to launch {operation}: {error}")
}

#[derive(PartialEq, Eq)]
enum ShellExecuteStatus {
    UacCanceled,
    Other,
}

fn shell_execute_status_for_error(hr: windows::core::HRESULT) -> ShellExecuteStatus {
    if super::helper::is_uac_canceled_shell_execute_code(hr.0 as u32) {
        ShellExecuteStatus::UacCanceled
    } else {
        ShellExecuteStatus::Other
    }
}

fn wide_null(value: &std::ffi::OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_window_helper_arg_uses_plain_decimal_hwnd_value() {
        let hwnd = 123456789isize;
        assert_eq!(hwnd.to_string(), "123456789");
    }

    #[test]
    fn shell_execute_error_maps_uac_cancel_before_context() {
        let error = windows::core::Error::from(windows::core::HRESULT(1223));
        assert_eq!(
            shell_execute_error_for_operation("task window helper", error),
            "UAC canceled"
        );
    }

    #[test]
    fn shell_execute_error_maps_hresult_from_win32_uac_cancel() {
        let error = windows::core::Error::from(windows::core::HRESULT(0x8007_04C7u32 as i32));
        assert_eq!(
            shell_execute_error_for_operation("task window helper", error),
            "UAC canceled"
        );
    }

    #[test]
    fn shell_execute_error_does_not_map_arbitrary_hresult() {
        let error = windows::core::Error::from(windows::core::HRESULT(0x8007_0005u32 as i32));
        assert_ne!(
            shell_execute_error_for_operation("task window helper", error),
            "UAC canceled"
        );
    }

    #[test]
    fn native_and_manual_refresh_due_rules_hold() {
        let base = Instant::now();
        let native = TaskbarRefreshRequest {
            reason: TaskbarRefreshReason::Native,
            requested_at: base,
        };
        let manual = TaskbarRefreshRequest {
            reason: TaskbarRefreshReason::Manual,
            requested_at: base,
        };
        assert_eq!(
            request_due_at(native),
            base + TASKBAR_REFRESH_NATIVE_COALESCE
        );
        assert_eq!(
            request_due_at(manual),
            base + TASKBAR_REFRESH_MANUAL_COALESCE
        );
        assert_eq!(
            request_due_at(coalesce_request(manual, native)),
            base + TASKBAR_REFRESH_NATIVE_COALESCE
        );
    }
}

fn build_window_candidate(
    hwnd: HWND,
    foreground: HWND,
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

    Ok(Some(WindowCandidate {
        class_name,
        title,
        process_name,
        process_path,
        hwnd,
        is_active: hwnd == foreground
            || unsafe { GetAncestor(hwnd, GA_ROOTOWNER) }
                == unsafe { GetAncestor(foreground, GA_ROOTOWNER) },
        is_minimized: unsafe { IsIconic(hwnd).as_bool() },
        has_owner: !owner.0.is_null(),
        is_cloaked: is_window_cloaked(hwnd),
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

fn last_taskbar_snapshot_if_fresh() -> Option<TaskbarWindowsSnapshot> {
    let snapshot_at = LAST_TASKBAR_SNAPSHOT_AT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()?
        .as_ref()
        .copied()?;
    if snapshot_at.elapsed() > TASKBAR_SNAPSHOT_MAX_AGE {
        return None;
    }
    LAST_TASKBAR_SNAPSHOT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()?
        .clone()
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

pub(super) fn attention_identity_for_hwnd(
    hwnd: HWND,
    process_id: u32,
) -> Result<attention::TaskbarAttentionIdentity, String> {
    let resolved_root = unsafe { GetAncestor(hwnd, GA_ROOTOWNER) };
    let root_owner = if resolved_root.0.is_null() {
        hwnd
    } else {
        resolved_root
    };
    let root_identity = actions::current_task_window_identity(root_owner).ok();
    Ok(attention::TaskbarAttentionIdentity {
        root_owner_hwnd: root_owner.0 as isize,
        process_id: root_identity
            .as_ref()
            .map(|identity| identity.process_id)
            .unwrap_or(process_id),
        creation_time: root_identity.map(|identity| identity.creation_time),
    })
}

pub(super) fn remove_root_owner_taskbar_attention(root_owner_hwnd: isize) {
    attention::remove_root_owner_taskbar_attention(root_owner_hwnd);
}
