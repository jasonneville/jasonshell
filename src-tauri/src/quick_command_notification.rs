//! Completion admission, notification-window lifecycle, and scoped IPC.

use crate::{
    contracts::{commands, events},
    graceful_exit,
    quick_command_notification_queue::{Banner, BannerAction, BannerStatus, Queue, RunTarget},
    shell_windows::{COMMAND_PANEL_LABEL, QUICK_COMMAND_NOTIFICATION_LABEL, TOP_BAR_LABEL},
    stack_popup::{authorize_stack_command, CallerAuthError, StackCommandAuth},
};
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{
    ipc::{Channel, JavaScriptChannelId},
    AppHandle, Emitter, EventTarget, Manager, Webview, WebviewWindow,
};
use windows::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::{
        GetWindowRect, IsIconic, IsWindow, IsWindowVisible, SetWindowPos, ShowWindow, HWND_TOPMOST,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE,
    },
};

const DISPLAY_RECHECK: Duration = Duration::from_millis(250);

#[derive(Default)]
pub(crate) struct Runtime {
    queue: Mutex<Queue>,
    target: Mutex<Option<RunTarget>>,
    renderer: Mutex<Option<Renderer>>,
    timer: Mutex<Option<WakeTimer>>,
    audio: Mutex<Option<AudioWorker>>,
    drive_pending: AtomicBool,
    overlay_available: AtomicBool,
    panel_hwnd: AtomicIsize,
    panel_epoch: AtomicU64,
}

struct Renderer {
    channel: Channel<()>,
    last_pull: Option<String>,
    wake_pending: bool,
}

struct AudioWorker {
    send: mpsc::SyncSender<()>,
    cancelled: Arc<AtomicBool>,
}

impl Drop for AudioWorker {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

struct WakeTimer {
    deadline: Instant,
    _cancel_on_drop: mpsc::Sender<()>,
}

#[derive(Serialize, Clone)]
struct Wake<'a> {
    id: &'a str,
}

pub(crate) struct CompletionSample {
    status: BannerStatus,
    panel_epoch: u64,
}

pub(crate) fn install(app: &AppHandle) {
    let runtime = app.state::<Runtime>();
    let Some(hwnd) = app
        .get_webview_window(COMMAND_PANEL_LABEL)
        .and_then(|panel| panel.hwnd().ok())
    else {
        return;
    };
    runtime.panel_hwnd.store(hwnd.0 as isize, Ordering::Release);
    if crate::shell_windows::build_quick_command_notification_window(app).is_ok() {
        runtime.overlay_available.store(true, Ordering::Release);
        if let Ok(mut audio) = runtime.audio.lock() {
            *audio = start_audio_worker(app);
        }
    }
}

pub(crate) fn retire_window(app: &AppHandle, label: &str) {
    let runtime = app.state::<Runtime>();
    if label == QUICK_COMMAND_NOTIFICATION_LABEL {
        runtime.overlay_available.store(false, Ordering::Release);
    } else {
        runtime.panel_hwnd.store(0, Ordering::Release);
    }
    shutdown(app);
}

fn panel_hidden(runtime: &Runtime) -> bool {
    let hwnd = HWND(runtime.panel_hwnd.load(Ordering::Acquire) as *mut _);
    // SAFETY: these are read-only Win32 queries; the panel is a startup-owned HWND.
    !hwnd.0.is_null()
        && unsafe {
            IsWindow(Some(hwnd)).as_bool()
                && (!IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool())
        }
}

pub(crate) fn sample_completion(
    app: &AppHandle,
    exit_code: Option<i32>,
    stopped: bool,
) -> Option<CompletionSample> {
    let runtime = app.try_state::<Runtime>()?;
    if !runtime.overlay_available.load(Ordering::Acquire) {
        return None;
    }
    let panel_epoch = runtime.panel_epoch.load(Ordering::Acquire);
    let status = BannerStatus::eligible(
        exit_code,
        stopped,
        panel_hidden(&runtime),
        graceful_exit::is_exit_accepted(app),
    )?;
    Some(CompletionSample {
        status,
        panel_epoch,
    })
}

pub(crate) fn complete(
    app: &AppHandle,
    sample: CompletionSample,
    target: RunTarget,
    label: String,
    saved: bool,
) {
    let runtime = app.state::<Runtime>();
    let accepted = graceful_exit::while_running(app, || {
        runtime.queue.lock().is_ok_and(|mut queue| {
            runtime.panel_epoch.load(Ordering::Acquire) == sample.panel_epoch
                && runtime.overlay_available.load(Ordering::Acquire)
                && queue.enqueue(target, label, sample.status, saved)
        })
    })
    .unwrap_or(false);
    if accepted {
        schedule_drive(app);
    }
}

fn schedule_drive(app: &AppHandle) {
    let runtime = app.state::<Runtime>();
    if runtime.drive_pending.swap(true, Ordering::AcqRel) {
        return;
    }
    let dispatch = app.clone();
    if app
        .run_on_main_thread(move || {
            dispatch
                .state::<Runtime>()
                .drive_pending
                .store(false, Ordering::Release);
            drive(&dispatch);
        })
        .is_err()
    {
        runtime.drive_pending.store(false, Ordering::Release);
        clear_queue(&runtime);
    }
}

fn sync_timer(app: &AppHandle, runtime: &Runtime, deadline: Option<Instant>) -> bool {
    let Ok(mut timer) = runtime.timer.lock() else {
        return false;
    };
    if timer.as_ref().map(|timer| timer.deadline) == deadline {
        return true;
    }
    *timer = None;
    let Some(deadline) = deadline else {
        return true;
    };
    let (tx, rx) = mpsc::channel();
    let app = app.clone();
    if std::thread::Builder::new()
        .name("command-banner-timer".into())
        .spawn(move || loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                schedule_drive(&app);
                return;
            }
            if !matches!(
                rx.recv_timeout(remaining.min(DISPLAY_RECHECK)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                return;
            }
            schedule_drive(&app);
        })
        .is_err()
    {
        return false;
    }
    *timer = Some(WakeTimer {
        deadline,
        _cancel_on_drop: tx,
    });
    true
}

fn clear_queue(runtime: &Runtime) {
    if let Ok(mut queue) = runtime.queue.lock() {
        queue.clear();
    }
    if let Ok(mut timer) = runtime.timer.lock() {
        *timer = None;
    }
}

fn notify_projection(runtime: &Runtime, id: Option<&str>) -> bool {
    let channel = runtime.renderer.lock().ok().and_then(|mut renderer| {
        let renderer = renderer.as_mut()?;
        if renderer.wake_pending || renderer.last_pull.as_deref() == id {
            return None;
        }
        renderer.wake_pending = true;
        Some(renderer.channel.clone())
    });
    if let Some(channel) = channel {
        // No payload, no app-wide listener: the Channel is bound to the invoking webview.
        if channel.send(()).is_err() {
            forget_renderer(runtime);
            return false;
        }
    }
    true
}

fn forget_renderer(runtime: &Runtime) {
    // Dropping a Channel evaluates its end marker in the webview. Release the
    // state lock first, and never wait for the renderer's callback.
    let previous = runtime
        .renderer
        .lock()
        .ok()
        .and_then(|mut renderer| renderer.take());
    clear_queue(runtime);
    drop(previous);
}

pub(crate) fn renderer_navigating(app: &AppHandle) {
    forget_renderer(&app.state::<Runtime>());
    hide(app);
}

fn drive(app: &AppHandle) {
    let runtime = app.state::<Runtime>();
    if graceful_exit::is_exit_accepted(app) || !panel_hidden(&runtime) {
        clear_queue(&runtime);
        hide(app);
        let _ = notify_projection(&runtime, None);
        return;
    }
    let renderer_ready = runtime
        .renderer
        .lock()
        .is_ok_and(|renderer| renderer.is_some());
    // At most one current plus three waiting. Bound one UI turn even if new
    // completions arrive while failed presentations are being discarded.
    for _ in 0..4 {
        let Ok(mut queue) = runtime.queue.lock() else {
            return;
        };
        let now = Instant::now();
        let projection = queue
            .offer(now, renderer_ready)
            .map(|banner| banner.id.clone());
        let candidate = queue.ready_to_show(now).map(|banner| banner.id.clone());
        let visible = queue.banner().is_some();
        let deadline = queue.wake_at();
        drop(queue);
        if let Some(id) = candidate {
            if show_candidate(app, &runtime, &id) {
                return;
            }
            continue;
        }
        if visible && !position_window(app, false) {
            clear_queue(&runtime);
            hide(app);
            let _ = notify_projection(&runtime, None);
            return;
        }
        if !visible {
            hide(app);
        }
        if !sync_timer(app, &runtime, deadline) {
            clear_queue(&runtime);
            hide(app);
            let _ = notify_projection(&runtime, None);
            return;
        }
        if !notify_projection(&runtime, projection.as_deref()) {
            hide(app);
        }
        return;
    }
    // A newer admission during the loop already requested another drive.
    let _ = sync_timer(app, &runtime, None);
    let _ = notify_projection(&runtime, None);
}

fn show_candidate(app: &AppHandle, runtime: &Runtime, id: &str) -> bool {
    if !panel_hidden(runtime) || graceful_exit::is_exit_accepted(app) {
        clear_queue(runtime);
        hide(app);
        return false;
    }
    if !runtime.queue.lock().is_ok_and(|queue| {
        queue
            .ready_to_show(Instant::now())
            .is_some_and(|banner| banner.id == id)
    }) {
        return false;
    }
    let shown = position_window(app, true);
    let Ok(mut queue) = runtime.queue.lock() else {
        return false;
    };
    let confirmed = queue.settle_show(id, shown, Instant::now());
    let deadline = queue.wake_at();
    drop(queue);
    if !confirmed || !sync_timer(app, runtime, deadline) || graceful_exit::is_exit_accepted(app) {
        if confirmed {
            clear_queue(runtime);
        }
        hide(app);
        return false;
    }
    if let Ok(audio) = runtime.audio.lock() {
        if let Some(audio) = audio.as_ref() {
            let _ = audio.send.try_send(());
        }
    }
    true
}

fn hide(app: &AppHandle) {
    if let Some(hwnd) = app
        .get_webview_window(QUICK_COMMAND_NOTIFICATION_LABEL)
        .and_then(|window| window.hwnd().ok())
    {
        // SAFETY: HWND comes from the managed overlay; hiding never activates it.
        let _ = unsafe { ShowWindow(HWND(hwnd.0), SW_HIDE) };
    }
}

fn position_window(app: &AppHandle, initial: bool) -> bool {
    let placement = (|| {
        let window = app.get_webview_window(QUICK_COMMAND_NOTIFICATION_LABEL)?;
        let hwnd = HWND(window.hwnd().ok()?.0);
        let monitor = app.primary_monitor().ok()??;
        let work = monitor.work_area();
        let top = app.get_webview_window(TOP_BAR_LABEL)?;
        let top_position = top.outer_position().ok()?;
        let top_size = top.outer_size().ok()?;
        let top_bottom = top_position
            .y
            .checked_add(i32::try_from(top_size.height).ok()?)?;
        let scale = monitor.scale_factor();
        let width = (crate::shell_windows::QUICK_COMMAND_NOTIFICATION_WIDTH * scale).round() as i32;
        let height =
            (crate::shell_windows::QUICK_COMMAND_NOTIFICATION_HEIGHT * scale).round() as i32;
        let gap = (12.0 * scale).round() as i32;
        let top_inset =
            (crate::shell_windows::QUICK_COMMAND_NOTIFICATION_INSET * scale).round() as i32;
        let right = work
            .position
            .x
            .checked_add(i32::try_from(work.size.width).ok()?)?;
        let bottom = work
            .position
            .y
            .checked_add(i32::try_from(work.size.height).ok()?)?;
        let x = right
            .checked_sub(width)?
            .checked_sub(gap)?
            .max(work.position.x);
        // Keep the painted card at its original gap while the badge sits above it.
        let y = top_bottom
            .max(work.position.y)
            .checked_add(gap.checked_sub(top_inset)?.max(0))?;
        if width <= 0
            || height <= 0
            || x.checked_add(width)? > right
            || y.checked_add(height)? > bottom
        {
            return None;
        }
        Some((hwnd, x, y, width, height))
    })();
    let Some((hwnd, x, y, width, height)) = placement else {
        return false;
    };
    // SAFETY: local RECT and live managed overlay; only nonactivating window moves.
    unsafe {
        if initial {
            // Move the hidden HWND onto the target monitor before final sizing,
            // so synchronous DPI handling cannot override the authoritative rect.
            let _ = ShowWindow(hwnd, SW_HIDE);
            if SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, 1, 1, SWP_NOACTIVATE).is_err() {
                return false;
            }
        }
        let mut rect = RECT::default();
        let matches = |rect: &RECT| {
            (
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            ) == (x, y, width, height)
        };
        if (GetWindowRect(hwnd, &mut rect).is_err() || !matches(&rect))
            && (SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE,
            )
            .is_err()
                || GetWindowRect(hwnd, &mut rect).is_err()
                || !matches(&rect))
        {
            return false;
        }
        if initial {
            // Unlike ShowWindow, this does not inherit startup activation flags.
            if SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            )
            .is_err()
            {
                return false;
            }
        }
        IsWindowVisible(hwnd).as_bool()
    }
}

fn start_audio_worker(app: &AppHandle) -> Option<AudioWorker> {
    // One worker serializes accepted cues; slow/missing audio never blocks UI.
    let (send, receive) = mpsc::sync_channel(4);
    let cancelled = Arc::new(AtomicBool::new(false));
    let stop = cancelled.clone();
    let app = app.clone();
    std::thread::Builder::new()
        .name("command-banner-audio".into())
        .spawn(move || {
            use std::os::windows::ffi::OsStrExt;
            use windows::{
                core::PCWSTR,
                Win32::Media::Audio::{PlaySoundW, SND_FILENAME, SND_NODEFAULT, SND_SYNC},
            };
            if graceful_exit::is_exit_accepted(&app) {
                return;
            }
            let Ok(path) = app.path().resolve(
                "resources/quick-command-complete.wav",
                tauri::path::BaseDirectory::Resource,
            ) else {
                return;
            };
            drop(app);
            let path: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
            while receive.recv().is_ok() && !stop.load(Ordering::Acquire) {
                // SAFETY: the terminated filename buffer stays valid for synchronous playback.
                let _ = unsafe {
                    PlaySoundW(
                        PCWSTR(path.as_ptr()),
                        None,
                        SND_SYNC | SND_FILENAME | SND_NODEFAULT,
                    )
                };
            }
        })
        .ok()?;
    Some(AudioWorker { send, cancelled })
}

fn authorize(
    window: &WebviewWindow,
    command: &'static str,
    callers: &'static [&'static str],
) -> Result<(), String> {
    authorize_stack_command(
        window,
        StackCommandAuth::AllowedCallers { command, callers },
    )
    .map_err(CallerAuthError::into_string)
}

#[tauri::command]
pub(crate) fn get_quick_command_notification(
    window: WebviewWindow,
    webview: Webview,
    channel: Option<JavaScriptChannelId>,
    rendered_id: Option<String>,
) -> Result<Option<Banner>, String> {
    authorize(
        &window,
        commands::GET_QUICK_COMMAND_NOTIFICATION,
        &[QUICK_COMMAND_NOTIFICATION_LABEL],
    )?;
    if webview.label() != QUICK_COMMAND_NOTIFICATION_LABEL {
        return Err("notification unavailable".into());
    }
    let app = window.app_handle();
    if graceful_exit::is_exit_accepted(app) {
        return Ok(None);
    }
    let runtime = app.state::<Runtime>();
    let registering = channel.is_some();
    if let Some(channel) = channel {
        let next = Renderer {
            channel: channel.channel_on(webview),
            last_pull: None,
            wake_pending: false,
        };
        let previous = runtime
            .renderer
            .lock()
            .map_err(|_| "notification unavailable")?
            .replace(next);
        if previous.is_some() {
            // Any previous document's offer/ACK is fenced by a new registration.
            clear_queue(&runtime);
            hide(app);
        }
        drop(previous);
    }
    let mut renderer_guard = runtime
        .renderer
        .lock()
        .map_err(|_| "notification unavailable")?;
    let Some(renderer) = renderer_guard.as_mut() else {
        return Ok(None);
    };
    let mut queue = runtime
        .queue
        .lock()
        .map_err(|_| "notification unavailable")?;
    let now = Instant::now();
    if !registering {
        if let Some(id) = rendered_id.filter(|id| renderer.last_pull.as_ref() == Some(id)) {
            let _ = queue.acknowledge(&id, now);
        }
    }
    let projection = queue.offer(now, true).cloned();
    renderer.wake_pending = false;
    renderer.last_pull = projection.as_ref().map(|banner| banner.id.clone());
    drop(queue);
    drop(renderer_guard);
    schedule_drive(app);
    Ok(projection)
}

#[tauri::command]
pub(crate) fn act_on_quick_command_notification(
    window: WebviewWindow,
    id: String,
    action: BannerAction,
) -> Result<(), String> {
    authorize(
        &window,
        commands::ACT_ON_QUICK_COMMAND_NOTIFICATION,
        &[QUICK_COMMAND_NOTIFICATION_LABEL],
    )?;
    let app = window.app_handle();
    if graceful_exit::is_exit_accepted(app) {
        return Ok(());
    }
    let runtime = app.state::<Runtime>();
    if !runtime
        .queue
        .lock()
        .map_err(|_| "notification unavailable")?
        .act(&id, action, Instant::now())
    {
        return Ok(());
    }
    if matches!(action, BannerAction::Open) {
        let _ = app.emit_to(
            EventTarget::webview_window(TOP_BAR_LABEL),
            events::QUICK_COMMAND_NOTIFICATION_OPEN_PANEL,
            Wake { id: &id },
        );
    }
    // Pause/Resume only re-sync the native timer; they never retire or open.
    schedule_drive(app);
    Ok(())
}

pub(crate) fn claim_open(window: &WebviewWindow, id: &str) -> Result<RunTarget, String> {
    authorize(window, commands::SHOW_COMMAND_PANEL, &[TOP_BAR_LABEL])?;
    let app = window.app_handle();
    let runtime = app.state::<Runtime>();
    graceful_exit::while_running(app, || {
        runtime.queue.lock().ok()?.claim_open(id, Instant::now())
    })?
    .ok_or_else(|| "notification no longer available".into())
}

pub(crate) fn panel_shown(app: &AppHandle, panel: &WebviewWindow, target: Option<RunTarget>) {
    let runtime = app.state::<Runtime>();
    runtime.panel_epoch.fetch_add(1, Ordering::AcqRel);
    clear_queue(&runtime);
    hide(app);
    let _ = notify_projection(&runtime, None);
    if let Ok(mut pending) = runtime.target.lock() {
        *pending = target;
        if pending.is_none() {
            return;
        }
    } else {
        return;
    }
    let _ = panel.emit_to(
        EventTarget::webview_window(COMMAND_PANEL_LABEL),
        events::COMMAND_PANEL_NOTIFICATION_TARGET,
        (),
    );
}

pub(crate) fn panel_show_failed(app: &AppHandle) {
    schedule_drive(app);
}

pub(crate) fn panel_hidden_clear_target(app: &AppHandle) {
    let runtime = app.state::<Runtime>();
    if let Ok(mut pending) = runtime.target.lock() {
        *pending = None;
    };
}

#[tauri::command]
pub(crate) fn take_quick_command_notification_target(
    window: WebviewWindow,
) -> Result<Option<RunTarget>, String> {
    authorize(
        &window,
        commands::TAKE_QUICK_COMMAND_NOTIFICATION_TARGET,
        &[COMMAND_PANEL_LABEL],
    )?;
    let app = window.app_handle();
    let runtime = app.state::<Runtime>();
    if graceful_exit::is_exit_accepted(app) || !window.is_visible().unwrap_or(false) {
        return Ok(None);
    }
    let mut pending = runtime
        .target
        .lock()
        .map_err(|_| "notification unavailable")?;
    Ok(pending.take())
}

pub(crate) fn shutdown(app: &AppHandle) {
    let runtime = app.state::<Runtime>();
    if let Ok(mut audio) = runtime.audio.lock() {
        *audio = None;
    }
    panel_hidden_clear_target(app);
    let dispatch = app.clone();
    let _ = app.run_on_main_thread(move || renderer_navigating(&dispatch));
}
