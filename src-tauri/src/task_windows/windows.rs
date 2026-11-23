use super::{
    actions, attention,
    icons::{window_icon_data_url, EMPTY_ICON_DATA_URL},
    notifications, TaskbarProcessWindow, TaskbarWindow, TaskbarWindowActivityState,
    TaskbarWindowsSnapshot, TASKBAR_WINDOWS_SNAPSHOT_EVENT,
};
use std::collections::HashMap;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use windows::core::PWSTR;
use windows::Win32::Foundation::{
    CloseHandle, FILETIME, HANDLE, HWND, LPARAM,
};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
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
static SNAPSHOTS_STOPPED: AtomicBool = AtomicBool::new(false);
static SNAPSHOT_WORKERS: OnceLock<crate::shutdown_workers::WorkerGroup> = OnceLock::new();
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

// Cheap-flag/detail fields are read by the reference predicate (tests); production
// filters through `resolve_taskbar_candidacy` before building the candidate.
#[cfg_attr(not(test), allow(dead_code))]
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
    if SNAPSHOTS_STOPPED.load(Ordering::Acquire) { return; }
    if TASKBAR_SNAPSHOT_WORKER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    // One pending request is enough: the next scan publishes complete state.
    let (tx, rx) = mpsc::sync_channel::<TaskbarRefreshRequest>(8);
    let _ = TASKBAR_REFRESH_TX.set(tx);
    let _ = SNAPSHOT_WORKERS.get_or_init(Default::default).spawn(move || {
        let mut last_refresh_at = Instant::now();
        loop {
            if SNAPSHOTS_STOPPED.load(Ordering::Acquire) { return; }
            let next_refresh_at = last_refresh_at + TASKBAR_SNAPSHOT_REFRESH_CADENCE;
            let mut request_driven = false;
            match rx.recv_timeout(next_refresh_at.saturating_duration_since(Instant::now())) {
                Ok(mut request) => {
                    request_driven = true;
                    let mut due = request_due_at(request);
                    loop {
                        if SNAPSHOTS_STOPPED.load(Ordering::Acquire) { return; }
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
            // Watchdog ticks skip the broadcast when nothing changed; refreshes that
            // were requested (native hook / renderer) always broadcast.
            let _ = if request_driven {
                refresh_taskbar_snapshot_now(Some(&app))
            } else {
                refresh_taskbar_snapshot_heartbeat(Some(&app))
            };
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

/// Scan and publish. Request-driven (native/manual/initial) refreshes always emit,
/// so optimistic renderer state is reconciled even when nothing changed.
pub(super) fn refresh_taskbar_snapshot_now(app: Option<&AppHandle>) -> Result<(), String> {
    refresh_taskbar_snapshot_with(app, true)
}

/// Watchdog tick: identical to a forced refresh except that an unchanged snapshot is
/// stored (freshness, diagnostics) but not broadcast.
fn refresh_taskbar_snapshot_heartbeat(app: Option<&AppHandle>) -> Result<(), String> {
    refresh_taskbar_snapshot_with(app, false)
}

/// Fields compared to decide whether a snapshot must be emitted.
fn should_emit_taskbar_snapshot(prev: Option<&[TaskbarWindow]>, next: &[TaskbarWindow]) -> bool {
    prev != Some(next)
}

#[derive(Default)]
struct SnapshotPublisher {
    sequence: u64,
    last_emitted: Option<Vec<TaskbarWindow>>,
}

struct PublishOutcome {
    snapshot: TaskbarWindowsSnapshot,
    emit: bool,
}

impl SnapshotPublisher {
    fn new(start_sequence: u64) -> Self {
        Self { sequence: start_sequence, last_emitted: None }
    }

    /// Changed (or first) => sequence advances and the snapshot is to be emitted.
    /// Unchanged => sequence stays at the last emitted one; the caller still stores it.
    #[cfg(test)]
    fn publish(&mut self, windows: Vec<TaskbarWindow>) -> PublishOutcome {
        self.publish_with(windows, false)
    }

    fn publish_with(&mut self, windows: Vec<TaskbarWindow>, force_emit: bool) -> PublishOutcome {
        let emit = force_emit || should_emit_taskbar_snapshot(self.last_emitted.as_deref(), &windows);
        if emit {
            self.sequence += 1;
            self.last_emitted = Some(windows.clone());
        }
        PublishOutcome {
            snapshot: TaskbarWindowsSnapshot { sequence: self.sequence, windows },
            emit,
        }
    }

    /// Pull-path snapshot (no renderer receives it): does not move the emitted baseline.
    fn observe(&self, windows: Vec<TaskbarWindow>) -> TaskbarWindowsSnapshot {
        TaskbarWindowsSnapshot { sequence: self.sequence, windows }
    }
}

struct PublishState {
    publisher: SnapshotPublisher,
    last_ticket: u64,
    last_emit_at: Option<Instant>,
}

/// Unchanged watchdog ticks skip the broadcast, but a renderer that missed or lost an
/// update (listener attached after its mount pull, overwritten state) must heal: a
/// heartbeat emits anyway once the last emit is older than this.
const HEARTBEAT_FORCE_EMIT_AFTER: Duration = Duration::from_secs(5);

fn heartbeat_must_emit(last_emit: Option<Instant>, now: Instant) -> bool {
    last_emit.map_or(true, |at| now.saturating_duration_since(at) >= HEARTBEAT_FORCE_EMIT_AFTER)
}

static PUBLISH_STATE: OnceLock<Mutex<PublishState>> = OnceLock::new();
static SCAN_TICKET: AtomicU64 = AtomicU64::new(0);

/// Process identity cache keyed by (pid, creation FILETIME ticks): a reused pid has a
/// different creation time and therefore misses. Bounded, FIFO eviction.
struct ProcessInfoCache {
    capacity: usize,
    entries: HashMap<(u32, u64), (Option<PathBuf>, String)>,
    order: std::collections::VecDeque<(u32, u64)>,
}

impl ProcessInfoCache {
    fn new(capacity: usize) -> Self {
        Self { capacity: capacity.max(1), entries: HashMap::new(), order: Default::default() }
    }

    fn get(&self, pid: u32, creation_time: u64) -> Option<(Option<PathBuf>, String)> {
        self.entries.get(&(pid, creation_time)).cloned()
    }

    fn insert(&mut self, pid: u32, creation_time: u64, path: Option<PathBuf>, name: String) {
        let key = (pid, creation_time);
        if self.entries.insert(key, (path, name)).is_none() {
            self.order.push_back(key);
        }
        while self.entries.len() > self.capacity {
            match self.order.pop_front() {
                Some(oldest) => {
                    self.entries.remove(&oldest);
                }
                None => break,
            }
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

const PROCESS_INFO_CACHE_CAPACITY: usize = 256;
static PROCESS_INFO_CACHE: OnceLock<Mutex<ProcessInfoCache>> = OnceLock::new();

fn refresh_taskbar_snapshot_with(app: Option<&AppHandle>, force_emit: bool) -> Result<(), String> {
    let _work = SNAPSHOT_WORKERS.get_or_init(Default::default).enter()?;
    // Ordered at scan start: a scan that started earlier never overwrites a later one.
    let ticket = SCAN_TICKET.fetch_add(1, Ordering::SeqCst) + 1;
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
        let Some(candidate) = build_window_candidate(
            hwnd,
            foreground,
            current_process_id,
            &mut cpu_time_by_process_id,
        )?
        else {
            continue;
        };

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
        let icon_data_url = window_icon_data_url(native_hwnd, process_id, process_path.as_deref())
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
    publish_taskbar_snapshot(app, ticket, force_emit, windows)
}

/// Serialises sequence assignment, snapshot storage and broadcast order across
/// concurrent refreshes (worker vs command). The enumeration above runs with no lock held.
fn publish_taskbar_snapshot(
    app: Option<&AppHandle>,
    ticket: u64,
    force_emit: bool,
    windows: Vec<TaskbarWindow>,
) -> Result<(), String> {
    let state = PUBLISH_STATE.get_or_init(|| {
        Mutex::new(PublishState { publisher: SnapshotPublisher::new(0), last_ticket: 0, last_emit_at: None })
    });
    let mut state = state
        .lock()
        .map_err(|_| "taskbar snapshot publisher is poisoned".to_string())?;
    let mut windows = windows;
    if ticket < state.last_ticket {
        // A scan that started later already published. A request-driven refresh still
        // owes its renderers an emit, so republish the newest stored windows.
        if !(force_emit && app.is_some()) {
            return Ok(());
        }
        let newest = LAST_TASKBAR_SNAPSHOT
            .get_or_init(|| Mutex::new(None))
            .lock()
            .ok()
            .and_then(|snapshot| snapshot.as_ref().map(|snapshot| snapshot.windows.clone()));
        match newest {
            Some(newest) => windows = newest,
            None => return Ok(()),
        }
    } else {
        state.last_ticket = ticket;
    }
    let (snapshot, emit) = if app.is_some() {
        let force = force_emit || heartbeat_must_emit(state.last_emit_at, Instant::now());
        let outcome = state.publisher.publish_with(windows, force);
        if outcome.emit {
            state.last_emit_at = Some(Instant::now());
        }
        (outcome.snapshot, outcome.emit)
    } else {
        (state.publisher.observe(windows), false)
    };
    TASKBAR_SNAPSHOT_SEQUENCE.store(snapshot.sequence, Ordering::SeqCst);
    let payload = emit.then(|| snapshot.clone());
    if let Ok(mut last) = LAST_TASKBAR_SNAPSHOT.get_or_init(|| Mutex::new(None)).lock() {
        *last = Some(snapshot);
    }
    if let Ok(mut snapshot_at) = LAST_TASKBAR_SNAPSHOT_AT
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *snapshot_at = Some(Instant::now());
    }
    if let (Some(app), Some(payload)) = (app, payload) {
        // Broadcast kept: TopBar and BottomBar both consume it. Emitting while the
        // publisher lock is held keeps emit order equal to sequence order.
        let _ = app.emit(TASKBAR_WINDOWS_SNAPSHOT_EVENT, payload);
    }
    Ok(())
}

pub(super) fn stop_and_drain_snapshots() -> Result<(), String> {
    SNAPSHOTS_STOPPED.store(true, Ordering::Release);
    SNAPSHOT_WORKERS.get_or_init(Default::default).drain()
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
    let mut cpu_time_by_process_id: HashMap<u32, Option<u64>> = HashMap::new();
    for hwnd in handles {
        let Some(candidate) = build_window_candidate(
            hwnd,
            foreground,
            current_process_id,
            &mut cpu_time_by_process_id,
        )?
        else {
            continue;
        };

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
#[path = "windows_tier1_tests.rs"]
mod tier1_tests;

#[cfg(test)]
mod tests {
    use super::*;

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
    cpu_times: &mut HashMap<u32, Option<u64>>,
) -> Result<Option<WindowCandidate>, String> {
    let mut process_id = 0;
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }

    if process_id == 0 {
        return Ok(None);
    }

    // Cheap, handle-free checks first. Most of the several hundred top-level HWNDs
    // are invisible, owned or tool windows and never reach title/class/process work.
    let is_visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    let is_minimized = unsafe { IsIconic(hwnd).as_bool() };
    let is_shell_process = process_id == current_process_id;
    if !(is_visible || is_minimized) || is_shell_process {
        return Ok(None);
    }
    let owner = unsafe { GetWindow(hwnd, GW_OWNER).unwrap_or_default() };
    let has_owner = !owner.0.is_null();
    if has_owner {
        return Ok(None);
    }
    let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 };
    let mut flags = CheapWindowFlags {
        is_visible,
        is_minimized,
        has_owner,
        is_cloaked: false,
        is_shell_process,
        ex_style,
    };
    if !cheap_flags_allow(&flags) {
        return Ok(None);
    }
    flags.is_cloaked = is_window_cloaked(hwnd);

    let Some(detail) = resolve_taskbar_candidacy(&flags, || {
        resolve_window_detail(hwnd, process_id, cpu_times)
    }) else {
        return Ok(None);
    };

    Ok(Some(WindowCandidate {
        class_name: detail.class_name,
        title: detail.title,
        process_name: detail.process_name,
        process_path: detail.process_path,
        hwnd,
        is_active: hwnd == foreground
            || unsafe { GetAncestor(hwnd, GA_ROOTOWNER) }
                == unsafe { GetAncestor(foreground, GA_ROOTOWNER) },
        is_minimized,
        has_owner,
        is_cloaked: flags.is_cloaked,
        is_shell_process,
        is_visible,
        ex_style: WINDOW_EX_STYLE(ex_style),
        process_id,
    }))
}

/// Flags obtainable without title / class / process-path / icon work.
#[derive(Clone, Copy, Debug)]
struct CheapWindowFlags {
    is_visible: bool,
    is_minimized: bool,
    has_owner: bool,
    is_cloaked: bool,
    is_shell_process: bool,
    ex_style: u32,
}

/// Expensive details: title, class, process path and name.
#[derive(Clone, Debug)]
struct WindowDetail {
    title: String,
    class_name: String,
    process_name: String,
    process_path: Option<PathBuf>,
}

fn cheap_flags_allow(flags: &CheapWindowFlags) -> bool {
    let is_tool_window =
        (flags.ex_style & WS_EX_TOOLWINDOW.0) != 0 && (flags.ex_style & WS_EX_APPWINDOW.0) == 0;
    let is_no_activate_window = (flags.ex_style & WS_EX_NOACTIVATE.0) != 0;
    (flags.is_visible || flags.is_minimized)
        && !flags.is_shell_process
        && !flags.has_owner
        && !flags.is_cloaked
        && !is_tool_window
        && !is_no_activate_window
}

fn detail_allows(ex_style: u32, title: &str, class_name: &str, process_name: &str) -> bool {
    let forces_taskbar = (ex_style & WS_EX_APPWINDOW.0) != 0;
    let has_taskbar_identity =
        !title.trim().is_empty() || (forces_taskbar && !process_name.is_empty());
    has_taskbar_identity
        && !is_internal_notification(title, class_name, process_name)
        && !EXCLUDED_PROCESS_NAMES
            .iter()
            .any(|excluded| process_name.eq_ignore_ascii_case(excluded))
        && !EXCLUDED_CLASSES
            .iter()
            .any(|excluded| class_name.eq_ignore_ascii_case(excluded))
}

/// Same verdict as `is_taskbar_candidate`, but `detail` is only evaluated when every
/// cheap flag already allows the window, and at most once. Accepted details are
/// returned so the caller never resolves them twice.
fn resolve_taskbar_candidacy(
    flags: &CheapWindowFlags,
    detail: impl FnOnce() -> WindowDetail,
) -> Option<WindowDetail> {
    if !cheap_flags_allow(flags) {
        return None;
    }
    let detail = detail();
    detail_allows(
        flags.ex_style,
        &detail.title,
        &detail.class_name,
        &detail.process_name,
    )
    .then_some(detail)
}

fn resolve_window_detail(
    hwnd: HWND,
    process_id: u32,
    cpu_times: &mut HashMap<u32, Option<u64>>,
) -> WindowDetail {
    let title = window_text(hwnd);
    let class_name = class_name(hwnd);
    let probe = probe_process(process_id);
    cpu_times.entry(process_id).or_insert(probe.cpu_time_ticks);
    let process_name = probe.process_name;
    WindowDetail {
        title,
        class_name,
        process_name,
        process_path: probe.process_path,
    }
}

/// Reference predicate (oracle for `resolve_taskbar_candidacy`); production filters
/// through `resolve_taskbar_candidacy`.
#[cfg(test)]
pub(super) fn is_taskbar_candidate(candidate: &WindowCandidate, current_process_id: u32) -> bool {
    let flags = CheapWindowFlags {
        is_visible: candidate.is_visible,
        is_minimized: candidate.is_minimized,
        has_owner: candidate.has_owner,
        is_cloaked: candidate.is_cloaked,
        is_shell_process: candidate.is_shell_process || candidate.process_id == current_process_id,
        ex_style: candidate.ex_style.0,
    };
    cheap_flags_allow(&flags)
        && detail_allows(
            flags.ex_style,
            &candidate.title,
            &candidate.class_name,
            &candidate.process_name,
        )
}

#[cfg(test)]
pub(super) fn is_internal_notification_window(candidate: &WindowCandidate) -> bool {
    is_internal_notification(&candidate.title, &candidate.class_name, &candidate.process_name)
}

fn is_internal_notification(title: &str, class_name: &str, process_name: &str) -> bool {
    let title = title.trim();
    if EXCLUDED_TITLES
        .iter()
        .any(|excluded| title.eq_ignore_ascii_case(excluded))
    {
        return true;
    }

    title.eq_ignore_ascii_case("Notification")
        && (process_name.is_empty()
            || process_name.eq_ignore_ascii_case("dwm")
            || class_name.to_ascii_lowercase().contains("dwm"))
}

struct ProcessProbe {
    process_path: Option<PathBuf>,
    process_name: String,
    cpu_time_ticks: Option<u64>,
}

/// One process handle per window (not per pid): creation time (cache key) and CPU ticks
/// come from the same `GetProcessTimes` call; the image path and name are only
/// resolved on a cache miss, and never while the cache lock is held.
fn probe_process(process_id: u32) -> ProcessProbe {
    let Some(process_handle) =
        (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok() })
    else {
        return ProcessProbe { process_path: None, process_name: String::new(), cpu_time_ticks: None };
    };
    let times = process_times(process_handle);
    let cache = PROCESS_INFO_CACHE
        .get_or_init(|| Mutex::new(ProcessInfoCache::new(PROCESS_INFO_CACHE_CAPACITY)));
    let cached = times.and_then(|(creation, _)| {
        cache.lock().ok().and_then(|cache| cache.get(process_id, creation))
    });
    let (process_path, process_name) = match cached {
        Some(hit) => hit,
        None => {
            let path = query_process_image_path(process_handle);
            let name = resolve_process_name(path.as_deref());
            if let (Some((creation, _)), Some(path)) = (times, path.as_ref()) {
                if let Ok(mut cache) = cache.lock() {
                    cache.insert(process_id, creation, Some(path.clone()), name.clone());
                }
            }
            (path, name)
        }
    };
    unsafe {
        let _ = CloseHandle(process_handle);
    }
    ProcessProbe { process_path, process_name, cpu_time_ticks: times.map(|(_, cpu)| cpu) }
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
    let result = query_process_image_path(process_handle);
    unsafe {
        let _ = CloseHandle(process_handle);
    }
    result
}

fn query_process_image_path(process_handle: HANDLE) -> Option<PathBuf> {
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
}

/// (creation FILETIME ticks, kernel + user CPU ticks).
fn process_times(process_handle: HANDLE) -> Option<(u64, u64)> {
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

    Some((
        filetime_ticks(creation_time),
        filetime_ticks(kernel_time) + filetime_ticks(user_time),
    ))
}

fn process_cpu_time_ticks(process_id: u32) -> Option<u64> {
    let process_handle =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };
    let result = process_times(process_handle).map(|(_, cpu)| cpu);
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
