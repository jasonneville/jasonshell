use serde::Deserialize;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::settings;
use crate::shell_windows::{COMMAND_PANEL_LABEL, TOP_BAR_LABEL};
use crate::stack_popup::{authorize_stack_command, CallerAuthError, StackCommandAuth};

pub const COMMAND_PANEL_CLOSED_EVENT: &str = "command-panel:closed";
const COMMAND_PANEL_MARGIN_PHYSICAL: i32 = 0;
static COMMAND_PANEL_FOCUS_LOSS_NONCE: AtomicU64 = AtomicU64::new(1);
static COMMAND_PANEL_SUPPRESS_NEXT_RESIZE_SAVE: AtomicBool = AtomicBool::new(false);
static COMMAND_PANEL_FOCUS_LOSS_HOLD_COUNT: AtomicU32 = AtomicU32::new(0);
const QUICK_COMMAND_ARTIFACT_PICKER_AUTH: StackCommandAuth = StackCommandAuth::AllowedCallers {
    command: crate::contracts::commands::PICK_QUICK_COMMAND_ARTIFACT_LOCATION,
    callers: &[COMMAND_PANEL_LABEL],
};

struct FocusHoldGuard;

impl Drop for FocusHoldGuard {
    fn drop(&mut self) {
        COMMAND_PANEL_FOCUS_LOSS_HOLD_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowCommandPanelRequest {
    pub anchor_left: f64,
    pub anchor_width: f64,
    pub notification_id: Option<String>,
    /// Shows this saved command without opening or starting a run.
    pub command_id: Option<String>,
}

/// An empty run ID means "open the command"; a notification always names a run.
#[cfg(windows)]
fn saved_command_target(
    shell_settings: &settings::ShellSettings,
    command_id: &str,
) -> Result<crate::quick_command_notification_queue::RunTarget, String> {
    shell_settings
        .quick_commands
        .entries
        .iter()
        .find(|entry| entry.id == command_id)
        .map(|entry| crate::quick_command_notification_queue::RunTarget {
            command_id: entry.id.clone(),
            run_id: String::new(),
        })
        .ok_or_else(|| format!("quick command '{command_id}' is not configured"))
}

#[tauri::command]
pub fn show_command_panel(
    window: WebviewWindow,
    app_handle: AppHandle,
    request: ShowCommandPanelRequest,
) -> Result<(), String> {
    #[cfg(windows)]
    let target = match (request.notification_id.as_deref(), request.command_id.as_deref()) {
        (Some(id), _) => Some(crate::quick_command_notification::claim_open(&window, id)?),
        (None, Some(id)) => {
            authorize_stack_command(
                &window,
                StackCommandAuth::AllowedCallers {
                    command: crate::contracts::commands::SHOW_COMMAND_PANEL,
                    callers: &[TOP_BAR_LABEL],
                },
            )
            .map_err(CallerAuthError::into_string)?;
            Some(saved_command_target(&settings::load_shell_settings_for_app(&app_handle)?, id)?)
        }
        (None, None) => None,
    };
    #[cfg(not(windows))]
    let _ = window;
    let panel = match show_command_panel_window(&app_handle, &request) {
        Ok(panel) => panel,
        Err(error) => {
            #[cfg(windows)]
            if request.notification_id.is_some() {
                crate::quick_command_notification::panel_show_failed(&app_handle);
            }
            return Err(error);
        }
    };
    let focus = panel.set_focus();
    #[cfg(windows)]
    crate::quick_command_notification::panel_shown(
        &app_handle,
        &panel,
        if focus.is_ok() { target } else { None },
    );
    focus.map_err(|error| format!("Failed to focus the command panel: {error}"))
}

fn show_command_panel_window(app_handle: &AppHandle, request: &ShowCommandPanelRequest) -> Result<WebviewWindow, String> {
    invalidate_command_panel_focus_loss_nonce();
    let panel = app_handle
        .get_webview_window(COMMAND_PANEL_LABEL)
        .ok_or_else(|| "Command panel window is unavailable".to_string())?;
    let top_bar = app_handle
        .get_webview_window(TOP_BAR_LABEL)
        .ok_or_else(|| "Top bar window is unavailable".to_string())?;

    let top_position = top_bar
        .outer_position()
        .map_err(|error| format!("Failed to read the top bar position: {error}"))?;
    let top_size = top_bar
        .outer_size()
        .map_err(|error| format!("Failed to read the top bar size: {error}"))?;
    let monitor = top_bar
        .current_monitor()
        .map_err(|error| format!("Failed to inspect current monitor: {error}"))?
        .or_else(|| app_handle.primary_monitor().ok().flatten())
        .ok_or_else(|| "Primary monitor is unavailable".to_string())?;
    let work_area = monitor.work_area();
    let scale_factor = monitor.scale_factor();
    let shell_settings = settings::load_shell_settings_for_app(app_handle)?;
    ensure_size_saver(app_handle, &shell_settings, scale_factor);
    let (saved_width, saved_height) = pending_logical_size().unwrap_or((
        shell_settings.ui.command_panel_width_logical,
        shell_settings.ui.command_panel_height_logical,
    ));
    let desired_width = ((saved_width * scale_factor).round() as i32).max(1);
    let desired_height = ((saved_height * scale_factor).round() as i32).max(1);
    let desired_x = top_position.x
        + ((request.anchor_left + request.anchor_width) * scale_factor).round() as i32
        - desired_width;
    let (panel_x, panel_y, panel_width, panel_height) = command_panel_work_area(
        work_area.position,
        work_area.size,
        desired_width,
        desired_height,
        desired_x,
        top_position.y + top_size.height as i32 + COMMAND_PANEL_MARGIN_PHYSICAL,
    );

    let target_size = PhysicalSize::new(panel_width as u32, panel_height as u32);
    let current_size = panel
        .outer_size()
        .map_err(|error| format!("Failed to read the command panel size: {error}"))?;
    if current_size != target_size {
        suppress_next_command_panel_resize_save();
    }
    panel
        .set_size(target_size)
        .map_err(|error| format!("Failed to size the command panel: {error}"))?;
    panel
        .set_position(PhysicalPosition::new(panel_x, panel_y))
        .map_err(|error| format!("Failed to position the command panel: {error}"))?;
    panel
        .show()
        .map_err(|error| format!("Failed to show the command panel: {error}"))?;
    Ok(panel)
}

/// Physical (width, height) of the command panel.
pub(crate) type PanelSizePhysical = (u32, u32);

/// Pure trailing-debounce state machine for the persisted panel size.
/// Time is injected; there is no I/O and no sleeping.
pub(crate) struct SizeDebouncer {
    quiet: Duration,
    persisted: Option<PanelSizePhysical>,
    pending: Option<PanelSizePhysical>,
    due: Option<Instant>,
    inflight: bool,
    /// Consecutive failed persists of the current pending size.
    failures: u32,
}

/// After this many consecutive failures retries stop until a new record or an
/// explicit flush; delays double from `quiet` up to `MAX_RETRY_DELAY`.
const MAX_PERSIST_FAILURES: u32 = 5;
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

impl SizeDebouncer {
    pub(crate) fn new(quiet: Duration, persisted: Option<PanelSizePhysical>) -> Self {
        Self { quiet, persisted, pending: None, due: None, inflight: false, failures: 0 }
    }

    pub(crate) fn record(&mut self, size: PanelSizePhysical, now: Instant) {
        self.failures = 0;
        if !self.inflight && Some(size) == self.persisted {
            self.pending = None;
            self.due = None;
        } else {
            self.pending = Some(size);
            self.due = Some(now + self.quiet);
        }
    }

    pub(crate) fn pending(&self) -> Option<PanelSizePhysical> {
        self.pending
    }

    pub(crate) fn due_at(&self) -> Option<Instant> {
        self.pending.and(self.due)
    }

    fn take(&mut self) -> Option<PanelSizePhysical> {
        if self.inflight {
            return None;
        }
        match self.pending {
            Some(size) if Some(size) != self.persisted => {
                self.inflight = true;
                Some(size)
            }
            Some(_) => {
                self.pending = None;
                self.due = None;
                None
            }
            None => None,
        }
    }

    pub(crate) fn poll(&mut self, now: Instant) -> Option<PanelSizePhysical> {
        match self.due {
            Some(due) if now >= due => self.take(),
            _ => None,
        }
    }

    pub(crate) fn flush(&mut self) -> Option<PanelSizePhysical> {
        self.take()
    }

    pub(crate) fn report_ok(&mut self, size: PanelSizePhysical) {
        self.inflight = false;
        self.failures = 0;
        self.persisted = Some(size);
        if self.pending == Some(size) {
            self.pending = None;
            self.due = None;
        }
    }

    pub(crate) fn report_err(&mut self, size: PanelSizePhysical, now: Instant) {
        self.inflight = false;
        if self.pending == Some(size) {
            self.failures = self.failures.saturating_add(1);
            self.due = if self.failures >= MAX_PERSIST_FAILURES {
                None
            } else {
                let factor = 1u32 << (self.failures - 1).min(16);
                Some(now + self.quiet.saturating_mul(factor).min(MAX_RETRY_DELAY))
            };
        }
    }
}

struct SaverState {
    debouncer: SizeDebouncer,
    flush_requested: bool,
    stop: bool,
    finished: bool,
    error_count: u64,
}

struct SaverShared {
    state: Mutex<SaverState>,
    wake: Condvar,
}

/// Worker-backed saver. `record` and `request_flush` never persist on the
/// caller; only `flush` / `shutdown` wait, and both are bounded.
pub(crate) struct DebouncedSizeSaver {
    shared: Arc<SaverShared>,
}

impl DebouncedSizeSaver {
    pub(crate) fn spawn(
        quiet: Duration,
        persisted: Option<PanelSizePhysical>,
        persist: impl Fn(PanelSizePhysical) -> Result<(), String> + Send + 'static,
    ) -> Self {
        let shared = Arc::new(SaverShared {
            state: Mutex::new(SaverState {
                debouncer: SizeDebouncer::new(quiet, persisted),
                flush_requested: false,
                stop: false,
                finished: false,
                error_count: 0,
            }),
            wake: Condvar::new(),
        });
        let worker = Arc::clone(&shared);
        let spawned = std::thread::Builder::new()
            .name("command-panel-size-saver".into())
            .spawn(move || Self::run(&worker, persist));
        if spawned.is_err() {
            if let Ok(mut state) = shared.state.lock() {
                state.finished = true;
            }
        }
        Self { shared }
    }

    fn run(shared: &SaverShared, persist: impl Fn(PanelSizePhysical) -> Result<(), String>) {
        let mut stop_attempted = false;
        let mut state = match shared.state.lock() {
            Ok(state) => state,
            Err(_) => return,
        };
        loop {
            let take = loop {
                let now = Instant::now();
                if state.stop && stop_attempted {
                    state.finished = true;
                    shared.wake.notify_all();
                    return;
                }
                let flushing = state.flush_requested || state.stop;
                state.flush_requested = false;
                let taken = if flushing {
                    state.debouncer.flush()
                } else {
                    state.debouncer.poll(now)
                };
                if let Some(size) = taken {
                    if state.stop {
                        stop_attempted = true;
                    }
                    break size;
                }
                if state.stop {
                    state.finished = true;
                    shared.wake.notify_all();
                    return;
                }
                state = match state.debouncer.due_at() {
                    Some(due) => match shared
                        .wake
                        .wait_timeout(state, due.saturating_duration_since(now))
                    {
                        Ok((guard, _)) => guard,
                        Err(_) => return,
                    },
                    None => match shared.wake.wait(state) {
                        Ok(guard) => guard,
                        Err(_) => return,
                    },
                };
            };
            drop(state);
            let result = persist(take);
            state = match shared.state.lock() {
                Ok(state) => state,
                Err(_) => return,
            };
            match result {
                Ok(()) => state.debouncer.report_ok(take),
                Err(_) => {
                    state.error_count += 1;
                    state.debouncer.report_err(take, Instant::now());
                }
            }
            shared.wake.notify_all();
        }
    }

    pub(crate) fn record(&self, size: PanelSizePhysical) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.debouncer.record(size, Instant::now());
            self.shared.wake.notify_all();
        }
    }

    pub(crate) fn pending(&self) -> Option<PanelSizePhysical> {
        self.shared.state.lock().ok().and_then(|state| state.debouncer.pending())
    }

    /// Signal the worker to persist now. Never blocks on I/O.
    pub(crate) fn request_flush(&self) {
        if let Ok(mut state) = self.shared.state.lock() {
            if state.debouncer.pending().is_some() {
                state.flush_requested = true;
                self.shared.wake.notify_all();
            }
        }
    }

    /// Waits (bounded) until nothing is pending. Production hide/blur/close use the
    /// non-blocking `request_flush`; only exit waits, through `shutdown`.
    #[cfg(test)]
    pub(crate) fn flush(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let Ok(mut state) = self.shared.state.lock() else { return false };
        if state.debouncer.pending().is_none() {
            return true;
        }
        let errors_at_request = state.error_count;
        state.flush_requested = true;
        self.shared.wake.notify_all();
        loop {
            if state.debouncer.pending().is_none() {
                return true;
            }
            if state.error_count != errors_at_request || state.finished {
                return false;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return false;
            }
            state = match self.shared.wake.wait_timeout(state, remaining) {
                Ok((guard, _)) => guard,
                Err(_) => return false,
            };
        }
    }

    pub(crate) fn shutdown(&self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        let Ok(mut state) = self.shared.state.lock() else { return };
        state.stop = true;
        state.flush_requested = true;
        self.shared.wake.notify_all();
        while !state.finished {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return;
            }
            state = match self.shared.wake.wait_timeout(state, remaining) {
                Ok((guard, _)) => guard,
                Err(_) => return,
            };
        }
    }
}

const SIZE_SAVE_QUIET: Duration = Duration::from_millis(500);
const SIZE_SAVE_EXIT_BOUND: Duration = Duration::from_millis(1_000);
static SIZE_SAVER: OnceLock<DebouncedSizeSaver> = OnceLock::new();
/// Scale factor passed with the most recent `record` (f64 bits), written just before
/// the size. The persist closure converts physical to logical with it; a DPI change
/// always arrives with a new `Resized`, which updates both together. A pending size
/// older than the latest record is superseded by it, so it is never converted with a
/// mismatched scale in practice.
static RECORD_SCALE_BITS: AtomicU64 = AtomicU64::new(0x3FF0_0000_0000_0000);

fn sanitize_scale(scale: f64) -> f64 {
    if scale.is_finite() && scale > 0.0 { scale } else { 1.0 }
}

fn record_scale() -> f64 {
    sanitize_scale(f64::from_bits(RECORD_SCALE_BITS.load(Ordering::SeqCst)))
}

fn logical_to_physical(logical: f64, scale: f64) -> u32 {
    ((logical * scale).round().max(1.0)) as u32
}

fn ensure_size_saver(
    app_handle: &AppHandle,
    shell_settings: &settings::ShellSettings,
    scale: f64,
) -> &'static DebouncedSizeSaver {
    SIZE_SAVER.get_or_init(|| {
        let scale = sanitize_scale(scale);
        RECORD_SCALE_BITS.store(scale.to_bits(), Ordering::SeqCst);
        let seed = (
            logical_to_physical(shell_settings.ui.command_panel_width_logical, scale),
            logical_to_physical(shell_settings.ui.command_panel_height_logical, scale),
        );
        let app = app_handle.clone();
        DebouncedSizeSaver::spawn(SIZE_SAVE_QUIET, Some(seed), move |(width, height)| {
            let scale = record_scale();
            let width_logical = (width as f64 / scale).max(1.0);
            let height_logical = (height as f64 / scale).max(1.0);
            settings::update_shell_settings_for_app(&app, |settings| {
                settings.ui.command_panel_width_logical = width_logical;
                settings.ui.command_panel_height_logical = height_logical;
            })
            .map(|_| ())
        })
    })
}

fn pending_logical_size() -> Option<(f64, f64)> {
    let (width, height) = SIZE_SAVER.get()?.pending()?;
    let scale = record_scale();
    Some(((width as f64 / scale).max(1.0), (height as f64 / scale).max(1.0)))
}

/// `WindowEvent::Resized` entry point: memory only, no settings I/O.
pub fn record_command_panel_resize(width_physical: u32, height_physical: u32, scale_factor: f64) {
    if take_command_panel_resize_save_suppression() {
        return;
    }
    let Some(saver) = SIZE_SAVER.get() else { return };
    if width_physical == 0 || height_physical == 0 {
        return; // minimised
    }
    RECORD_SCALE_BITS.store(sanitize_scale(scale_factor).to_bits(), Ordering::SeqCst);
    saver.record((width_physical, height_physical));
}

/// Hide / focus-loss / close: ask the worker to write now. Never blocks on I/O.
pub fn request_command_panel_size_flush() {
    if let Some(saver) = SIZE_SAVER.get() {
        saver.request_flush();
    }
}

/// Graceful exit: flush pending and stop the worker, bounded.
pub fn shutdown_command_panel_size_saver() {
    if let Some(saver) = SIZE_SAVER.get() {
        saver.shutdown(SIZE_SAVE_EXIT_BOUND);
    }
}

pub fn save_command_panel_size_for_app(
    app_handle: &AppHandle,
    width_physical: u32,
    height_physical: u32,
) -> Result<settings::ShellSettings, String> {
    let mut current = settings::load_shell_settings_for_app(app_handle)?;
    if take_command_panel_resize_save_suppression() {
        return Ok(current);
    }
    let panel = app_handle
        .get_webview_window(COMMAND_PANEL_LABEL)
        .ok_or_else(|| "Command panel window is unavailable".to_string())?;
    let scale_factor = sanitize_scale(panel.scale_factor().unwrap_or(1.0));
    let saver = ensure_size_saver(app_handle, &current, scale_factor);
    if width_physical > 0 && height_physical > 0 {
        RECORD_SCALE_BITS.store(scale_factor.to_bits(), Ordering::SeqCst);
        saver.record((width_physical, height_physical));
    }
    if let Some((width, height)) = pending_logical_size() {
        current.ui.command_panel_width_logical = width;
        current.ui.command_panel_height_logical = height;
    }
    Ok(current)
}

#[tauri::command]
pub fn save_command_panel_size(
    window: WebviewWindow,
    app_handle: AppHandle,
    width_physical: u32,
    height_physical: u32,
) -> Result<settings::ShellSettings, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::SAVE_COMMAND_PANEL_SIZE,
            callers: &[COMMAND_PANEL_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    save_command_panel_size_for_app(&app_handle, width_physical, height_physical)
}

#[tauri::command]
pub fn hide_command_panel(app_handle: AppHandle) -> Result<(), String> {
    invalidate_command_panel_focus_loss_nonce();
    let panel = app_handle
        .get_webview_window(COMMAND_PANEL_LABEL)
        .ok_or_else(|| "Command panel window is unavailable".to_string())?;
    request_command_panel_size_flush();
    panel
        .hide()
        .map_err(|error| format!("Failed to hide the command panel: {error}"))?;
    #[cfg(windows)]
    crate::quick_command_notification::panel_hidden_clear_target(&app_handle);
    app_handle
        .emit_to(TOP_BAR_LABEL, COMMAND_PANEL_CLOSED_EVENT, ())
        .map_err(|error| format!("Failed to publish command panel closed event: {error}"))
}

fn begin_command_panel_focus_loss_hold() -> (FocusHoldGuard, u64) {
    let lifecycle_nonce = invalidate_command_panel_focus_loss_nonce();
    COMMAND_PANEL_FOCUS_LOSS_HOLD_COUNT.fetch_add(1, Ordering::SeqCst);
    (FocusHoldGuard, lifecycle_nonce)
}

pub fn command_panel_focus_loss_held() -> bool {
    COMMAND_PANEL_FOCUS_LOSS_HOLD_COUNT.load(Ordering::SeqCst) > 0
}

#[tauri::command]
pub async fn pick_quick_command_artifact_location(
    window: WebviewWindow,
) -> Result<Option<String>, String> {
    authorize_stack_command(&window, QUICK_COMMAND_ARTIFACT_PICKER_AUTH)
        .map_err(CallerAuthError::into_string)?;
    let (hold, picker_lifecycle_nonce) = begin_command_panel_focus_loss_hold();
    #[cfg(target_os = "windows")]
    let result: Result<Option<String>, String> = {
        let window_for_picker = window.clone();
        async move {
            let owner = window_for_picker
                .hwnd()
                .map_err(|error| format!("Failed to read command panel HWND: {error}"))?
                .0 as isize;
            tauri::async_runtime::spawn_blocking(move || pick_artifact_folder_native(owner))
                .await
                .map_err(|error| format!("Artifact folder picker failed: {error}"))?
        }
        .await
    };
    #[cfg(not(target_os = "windows"))]
    let result = Err("Artifact folder picker is only supported on Windows".to_string());
    drop(hold);
    if artifact_picker_restore_requires_current_open_lifecycle(picker_lifecycle_nonce) {
        let _ = window.show();
        if command_panel_focus_loss_nonce_is_current(picker_lifecycle_nonce) {
            let _ = window.set_focus();
        }
    }
    result
}

#[cfg(target_os = "windows")]
fn pick_artifact_folder_native(owner: isize) -> Result<Option<String>, String> {
    use windows::core::{HRESULT, PWSTR};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR, FOS_PATHMUSTEXIST,
        FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
    };

    struct ComGuard(bool);
    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() }
            }
        }
    }
    unsafe fn take_pwstr(value: PWSTR) -> String {
        if value.is_null() {
            return String::new();
        }
        let mut len = 0;
        while unsafe { *value.0.add(len) } != 0 {
            len += 1
        }
        let text = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(value.0, len) });
        unsafe { CoTaskMemFree(Some(value.0.cast())) };
        text
    }

    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .map_err(|error| format!("Failed to initialize artifact folder picker: {error}"))?;
    let _com = ComGuard(true);
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }
            .map_err(|error| format!("Failed to create artifact folder picker: {error}"))?;
    let options = unsafe { dialog.GetOptions() }
        .map_err(|error| format!("Failed to read artifact folder picker options: {error}"))?;
    unsafe {
        dialog.SetOptions(
            options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST | FOS_NOCHANGEDIR,
        )
    }
    .map_err(|error| format!("Failed to configure artifact folder picker: {error}"))?;
    if let Err(error) = unsafe { dialog.Show(Some(HWND(owner as *mut _))) } {
        const CANCELLED: HRESULT = HRESULT(0x800704C7_u32 as i32);
        return if error.code() == CANCELLED {
            Ok(None)
        } else {
            Err(format!("Artifact folder picker failed: {error}"))
        };
    }
    let item = unsafe { dialog.GetResult() }
        .map_err(|error| format!("Failed to read artifact folder selection: {error}"))?;
    let path = unsafe {
        take_pwstr(
            item.GetDisplayName(SIGDN_FILESYSPATH)
                .map_err(|error| format!("Failed to read artifact folder path: {error}"))?,
        )
    };
    validate_artifact_picker_selection(&path)
}

fn validate_artifact_picker_selection(path: &str) -> Result<Option<String>, String> {
    let path = crate::settings::normalize_optional_absolute_windows_path(Some(path), "picker")?
        .ok_or_else(|| "Artifact folder picker returned an empty path".to_string())?;
    if !std::path::Path::new(&path).is_dir() {
        return Err("Selected artifact location is not an available folder".into());
    }
    Ok(Some(path))
}

pub fn invalidate_command_panel_focus_loss_nonce() -> u64 {
    COMMAND_PANEL_FOCUS_LOSS_NONCE.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn current_command_panel_focus_loss_nonce() -> u64 {
    COMMAND_PANEL_FOCUS_LOSS_NONCE.load(Ordering::SeqCst)
}

pub fn suppress_next_command_panel_resize_save() {
    COMMAND_PANEL_SUPPRESS_NEXT_RESIZE_SAVE.store(true, Ordering::SeqCst);
}

fn take_command_panel_resize_save_suppression() -> bool {
    COMMAND_PANEL_SUPPRESS_NEXT_RESIZE_SAVE.swap(false, Ordering::SeqCst)
}

pub fn command_panel_focus_loss_nonce_is_current(nonce: u64) -> bool {
    current_command_panel_focus_loss_nonce() == nonce
}

fn artifact_picker_restore_requires_current_open_lifecycle(picker_lifecycle_nonce: u64) -> bool {
    command_panel_focus_loss_nonce_is_current(picker_lifecycle_nonce)
}

pub fn command_panel_work_area(
    work_area_position: PhysicalPosition<i32>,
    work_area_size: PhysicalSize<u32>,
    panel_width: i32,
    panel_height: i32,
    x: i32,
    below_y: i32,
) -> (i32, i32, i32, i32) {
    let work_area_left = work_area_position.x;
    let work_area_top = work_area_position.y;
    let work_area_right = work_area_left + work_area_size.width as i32;
    let work_area_bottom = work_area_top + work_area_size.height as i32;
    let available_width = (work_area_right - work_area_left).max(1);
    let available_height = (work_area_bottom - work_area_top).max(1);
    let panel_width = panel_width.max(1).min(available_width);
    let panel_height = panel_height.max(1).min(available_height);
    let min_x = work_area_left;
    let max_x = work_area_right - panel_width;
    let panel_x = x.clamp(min_x, max_x);

    let below_fits = below_y + panel_height <= work_area_bottom;
    let above_y = (below_y - panel_height).max(work_area_top);
    let panel_y = if below_fits {
        below_y.clamp(work_area_top, work_area_bottom - panel_height)
    } else if above_y >= work_area_top {
        above_y
    } else {
        work_area_bottom - panel_height
    }
    .clamp(work_area_top, work_area_bottom - panel_height);

    (panel_x, panel_y, panel_width, panel_height)
}

#[cfg(test)]
#[path = "command_panel_tier1_tests.rs"]
mod tier1_tests;

#[cfg(test)]
mod tests {
    #[test]
    fn registered_size_save_authorizes_command_panel_before_settings_io() {
        let source = include_str!("command_panel.rs").replace("\r\n", "\n");
        let handler = source.split("#[tauri::command]\npub fn save_command_panel_size(").nth(1).expect("registered save entry point").split("#[tauri::command]").next().unwrap();
        let signature = handler.split("-> Result").next().unwrap();
        assert!(signature.contains("window: WebviewWindow"), "handler must inject trusted invoking window");
        assert!(handler.contains("command: crate::contracts::commands::SAVE_COMMAND_PANEL_SIZE"));
        assert!(handler.contains("callers: &[COMMAND_PANEL_LABEL]"), "only command panel may receive full settings");
        let guard = handler.find("authorize_stack_command(").expect("must authorize caller");
        let storage = handler.find("save_command_panel_size_for_app(").expect("settings helper");
        assert!(guard < storage, "denied caller must not read or write settings or receive a full response");
        assert!(handler[guard..storage].contains(".map_err(CallerAuthError::into_string)?"), "caller denial must short-circuit before storage");
    }

    #[test]
    fn size_save_policy_accepts_owner_and_redacts_nonowner_denial() {
        use crate::stack_popup::StackCommandAuth;
        let policy = StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::SAVE_COMMAND_PANEL_SIZE,
            callers: &[crate::shell_windows::COMMAND_PANEL_LABEL],
        };
        assert!(authorize_stack_command_caller(crate::shell_windows::COMMAND_PANEL_LABEL, policy).is_ok());
        for denied in [crate::contracts::surfaces::SETTINGS_PANEL, crate::contracts::surfaces::TOP_BAR,
            crate::contracts::surfaces::PROCESS_MANAGER] {
            let error = authorize_stack_command_caller(denied, policy).unwrap_err().into_string();
            assert_eq!(error, format!("Unauthorized caller for command {}", crate::contracts::commands::SAVE_COMMAND_PANEL_SIZE));
            assert!(!error.contains(denied), "caller label must not leak");
        }
    }
    use super::{
        artifact_picker_restore_requires_current_open_lifecycle,
        begin_command_panel_focus_loss_hold, command_panel_focus_loss_held,
        command_panel_focus_loss_nonce_is_current, command_panel_work_area,
        current_command_panel_focus_loss_nonce, invalidate_command_panel_focus_loss_nonce,
        suppress_next_command_panel_resize_save, take_command_panel_resize_save_suppression,
        validate_artifact_picker_selection, QUICK_COMMAND_ARTIFACT_PICKER_AUTH,
    };
    use crate::stack_popup::authorize_stack_command_caller;
    use tauri::{PhysicalPosition, PhysicalSize};

    #[test]
    fn places_command_panel_within_work_area_and_keeps_width_when_it_fits() {
        assert_eq!(
            command_panel_work_area(
                PhysicalPosition::new(100, 50),
                PhysicalSize::new(1_920, 1_080),
                460,
                320,
                1_650,
                80,
            ),
            (1_560, 80, 460, 320)
        );
    }

    #[test]
    fn shrinks_command_panel_before_positioning_when_work_area_is_too_narrow() {
        assert_eq!(
            command_panel_work_area(
                PhysicalPosition::new(-320, 0),
                PhysicalSize::new(280, 800),
                460,
                900,
                -200,
                20,
            ),
            (-320, 0, 280, 800)
        );
    }

    #[test]
    fn clamps_command_panel_height_to_work_area() {
        assert_eq!(
            command_panel_work_area(
                PhysicalPosition::new(10, -40),
                PhysicalSize::new(400, 180),
                220,
                260,
                180,
                110,
            ),
            (180, -40, 220, 180)
        );
    }

    #[test]
    fn clamps_saved_command_panel_size_to_monitor_work_area() {
        assert_eq!(
            command_panel_work_area(
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(320, 240),
                900,
                800,
                10,
                20,
            ),
            (0, 0, 320, 240)
        );
    }

    #[test]
    fn invalidate_command_panel_focus_loss_nonce_returns_current_nonce_until_next_invalidation() {
        let first = invalidate_command_panel_focus_loss_nonce();
        assert_eq!(current_command_panel_focus_loss_nonce(), first);
        assert!(command_panel_focus_loss_nonce_is_current(first));

        let second = invalidate_command_panel_focus_loss_nonce();
        assert_eq!(current_command_panel_focus_loss_nonce(), second);
        assert_ne!(first, second);
        assert!(!command_panel_focus_loss_nonce_is_current(first));
        assert!(command_panel_focus_loss_nonce_is_current(second));
    }

    #[test]
    fn artifact_picker_completion_does_not_restore_after_explicit_hide() {
        let picker_lifecycle_nonce = invalidate_command_panel_focus_loss_nonce();
        assert!(artifact_picker_restore_requires_current_open_lifecycle(
            picker_lifecycle_nonce
        ));

        invalidate_command_panel_focus_loss_nonce();

        assert!(!artifact_picker_restore_requires_current_open_lifecycle(
            picker_lifecycle_nonce
        ));
    }

    #[test]
    fn suppress_next_resize_save_consumes_single_resize_event() {
        suppress_next_command_panel_resize_save();
        assert!(take_command_panel_resize_save_suppression());
        assert!(!take_command_panel_resize_save_suppression());
    }

    #[test]
    fn command_panel_focus_hold_covers_full_guard_lifetime() {
        assert!(!command_panel_focus_loss_held());
        let (hold, _) = begin_command_panel_focus_loss_hold();
        assert!(command_panel_focus_loss_held());
        drop(hold);
        assert!(!command_panel_focus_loss_held());
    }

    #[test]
    fn artifact_picker_authorization_is_command_panel_only() {
        assert!(authorize_stack_command_caller(
            crate::shell_windows::COMMAND_PANEL_LABEL,
            QUICK_COMMAND_ARTIFACT_PICKER_AUTH,
        )
        .is_ok());
        assert_eq!(
            authorize_stack_command_caller(
                crate::shell_windows::TOP_BAR_LABEL,
                QUICK_COMMAND_ARTIFACT_PICKER_AUTH,
            )
            .unwrap_err()
            .into_string(),
            "Unauthorized caller for command pick_quick_command_artifact_location"
        );
    }

    #[test]
    fn artifact_picker_rejects_empty_relative_and_unavailable_selections() {
        assert!(validate_artifact_picker_selection("").is_err());
        assert!(validate_artifact_picker_selection("relative\\artifacts").is_err());
        let executable = std::env::current_exe().unwrap();
        assert!(validate_artifact_picker_selection(executable.to_str().unwrap()).is_err());
        assert!(
            validate_artifact_picker_selection("C:\\definitely-missing-jasonshell-artifacts")
                .is_err()
        );
    }
}
