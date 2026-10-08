use crate::shell_windows::{
    BOTTOM_BAR_LABEL, TASK_GALLERY_LABEL, TASK_GALLERY_WIDTH_LOGICAL,
};
use crate::{task_preview, task_windows, taskbar_menu};
pub mod layout;
pub mod thumbnails;
pub mod resources;
use resources::{ResourceOwner, ResourceSet};
use layout::{task_gallery_layout, TaskGalleryLayout};
use thumbnails::{Resource, SyncArgs, SyncResponse, Thumbnail, LiveThumbnail};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, Window};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE};

const TASK_GALLERY_EDGE_PADDING_LOGICAL: f64 = 0.0;
const TASK_GALLERY_TAB_GAP_LOGICAL: f64 = 0.0;
const MAX_CANCELLED_GALLERY_NONCES: usize = 256;

pub const TASK_GALLERY_CLOSED_EVENT: &str = "task-gallery:closed";
pub const TASK_GALLERY_OPEN_EVENT: &str = "task-gallery:open";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryShowArgs {
    pub anchor_left: f64,
    pub anchor_width: f64,
    pub nonce: String,
    pub group_key: String,
    pub label: String,
    pub windows: Vec<TaskGalleryWindowRow>,
    #[serde(default)]
    pub focus_gallery: bool,
    #[serde(default)]
    pub refresh_existing: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryActivateArgs {
    pub nonce: String,
    pub hwnd: String,
    pub minimize_if_active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryClosePreviewedWindowArgs {
    pub nonce: String,
    pub hwnd: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryCloseWindowArgs {
    pub nonce: String,
    pub hwnd: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryContextMenuArgs {
    pub nonce: String,
    pub hwnd: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryPreviewArgs {
    pub nonce: String,
    #[serde(rename = "requestId")]
    pub request_id: u64,
    pub hwnd: String,
    pub title: String,
    pub process_name: String,
    pub icon_data_url: String,
    pub is_minimized: bool,
    pub anchor_left: f64,
    pub anchor_width: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryOpenPayload {
    pub nonce: String,
    pub group_key: String,
    pub label: String,
    pub focus_gallery: bool,
    pub windows: Vec<TaskGalleryWindowRow>,
    pub revision: u64,
    pub layout: TaskGalleryLayout,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryWindowRow {
    pub hwnd: String,
    pub title: String,
    pub process_id: Option<u32>,
    pub process_name: String,
    pub icon_data_url: String,
    pub is_active: bool,
    pub is_minimized: bool,
}

#[derive(Default)]
struct TaskGalleryRuntimeState {
    nonce: Option<String>,
    cancelled_nonces: VecDeque<String>,
    windows_by_hwnd: BTreeMap<String, TaskGalleryAuthorizedWindow>,
    focus_loss_hold_count: u32,
    revision: u64,
    latest_thumbnail_request_id: u64,
    layout: Option<TaskGalleryLayout>,
    thumbnail_scale: Option<f64>,
    thumbnail_order: Vec<String>,
    thumbnails: ResourceOwner<ThumbnailGeneration, Resource>,
}

#[derive(Clone, PartialEq, Eq)]
struct ThumbnailGeneration {
    nonce: String,
    revision: u64,
    request_id: u64,
}

fn thumbnail_generation(args: &SyncArgs) -> ThumbnailGeneration {
    ThumbnailGeneration { nonce: args.nonce.clone(), revision: args.revision, request_id: args.request_id }
}

#[derive(Clone)]
struct TaskGalleryAuthorizedWindow {
    row: TaskGalleryWindowRow,
    #[cfg(target_os = "windows")]
    identity: task_windows::TaskWindowIdentity,
}

fn state() -> &'static Mutex<TaskGalleryRuntimeState> {
    static STATE: OnceLock<Mutex<TaskGalleryRuntimeState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(TaskGalleryRuntimeState::default()))
}

// Serializes native side effects, NOT runtime state access. Capture, DWM and
// Tauri operations never hold the runtime mutex. Commands are async so waiting
// for this lane cannot block the Tauri UI thread's window-operation dispatch.
fn native_lane() -> &'static Mutex<()> {
    static LANE: Mutex<()> = Mutex::new(());
    &LANE
}

fn reset_state() {
    let resources = {
        let mut runtime = state().lock().unwrap();
        runtime.nonce = None;
        runtime.windows_by_hwnd.clear();
        runtime.focus_loss_hold_count = 0;
        runtime.layout = None;
        runtime.thumbnail_scale = None;
        runtime.thumbnail_order.clear();
        runtime.latest_thumbnail_request_id = 0;
        runtime.thumbnails.clear()
    };
    drop(resources);
}

fn record_cancelled_nonce(runtime: &mut TaskGalleryRuntimeState, nonce: String) {
    if runtime.cancelled_nonces.contains(&nonce) {
        return;
    }
    if runtime.cancelled_nonces.len() == MAX_CANCELLED_GALLERY_NONCES {
        runtime.cancelled_nonces.pop_front();
    }
    runtime.cancelled_nonces.push_back(nonce);
}

fn take_cancelled_nonce(runtime: &mut TaskGalleryRuntimeState, nonce: &str) -> bool {
    let Some(index) = runtime
        .cancelled_nonces
        .iter()
        .position(|cancelled| cancelled == nonce)
    else {
        return false;
    };
    let _ = index;
    // Keep bounded tombstones: delayed repeated refreshes cannot resurrect it.
    true
}

fn emit_closed_event(app_handle: &AppHandle, nonce: Option<String>) {
    let payload = json!({"nonce": nonce});
    let _ = app_handle.emit_to(BOTTOM_BAR_LABEL, TASK_GALLERY_CLOSED_EVENT, payload.clone());
    let _ = app_handle.emit_to(TASK_GALLERY_LABEL, TASK_GALLERY_CLOSED_EVENT, payload);
}

fn begin_focus_hold() {
    state().lock().unwrap().focus_loss_hold_count += 1;
}
fn end_focus_hold() {
    let mut state = state().lock().unwrap();
    state.focus_loss_hold_count = state.focus_loss_hold_count.saturating_sub(1);
}
fn focus_hold_active() -> bool {
    state().lock().unwrap().focus_loss_hold_count > 0
}

fn validate(args_nonce: &str, hwnd: &str) -> Result<(), String> {
    snapshot_window(args_nonce, hwnd).map(|_| ())
}

fn validate_nonce(args_nonce: &str) -> Result<(), String> {
    let runtime = state().lock().unwrap();
    if runtime.nonce.as_deref() != Some(args_nonce) || runtime.cancelled_nonces.iter().any(|n| n == args_nonce) {
        return Err("Stale task gallery nonce".to_string());
    }
    Ok(())
}

fn snapshot_window(args_nonce: &str, hwnd: &str) -> Result<TaskGalleryAuthorizedWindow, String> {
    let authorized = {
        let state = state().lock().unwrap();
        snapshot_window_from_runtime(&state, args_nonce, hwnd)?
    };
    #[cfg(target_os = "windows")]
    {
        let current = task_windows::task_window_identity(hwnd)?;
        if current != authorized.identity {
            return Err("Task gallery window identity changed".to_string());
        }
    }
    Ok(authorized)
}

fn snapshot_window_from_runtime(
    runtime: &TaskGalleryRuntimeState,
    nonce: &str,
    hwnd: &str,
) -> Result<TaskGalleryAuthorizedWindow, String> {
    if runtime.nonce.as_deref() != Some(nonce) || runtime.cancelled_nonces.iter().any(|n| n == nonce) {
        return Err("Stale task gallery nonce".to_string());
    }
    runtime.windows_by_hwnd.get(hwnd).cloned()
        .ok_or_else(|| "Task gallery hwnd not allowed".to_string())
}

fn request_gallery_close_with(
    caller: &str,
    snapshot: impl FnOnce() -> Result<TaskGalleryAuthorizedWindow, String>,
    dispatch: impl FnOnce(TaskGalleryAuthorizedWindow) -> Result<(), String>,
) -> Result<(), String> {
    if caller != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command close_task_gallery_window".to_string());
    }
    dispatch(snapshot()?)
}

fn hide_task_gallery_window(gallery: &WebviewWindow) {
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = gallery.hwnd() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        return;
    }
    let _ = gallery.hide();
}

fn hide_gallery_and_reset(app_handle: &AppHandle) {
    if let Some(gallery) = app_handle.get_webview_window(TASK_GALLERY_LABEL) {
        hide_task_gallery_window(&gallery);
    }
    let nonce = {
        let mut state = state().lock().unwrap();
        let nonce = state.nonce.take();
        if let Some(ref nonce) = nonce {
            record_cancelled_nonce(&mut state, nonce.clone());
        }
        nonce
    };
    reset_state();
    if nonce.is_some() {
        let preview_state = app_handle.state::<Mutex<task_preview::TaskPreviewRuntimeState>>();
        if let Ok(request_id) = task_preview::next_task_preview_request_id(&preview_state) {
            let _ = task_preview::hide_task_window_preview(
                app_handle.clone(),
                preview_state,
                request_id,
            );
        }
        emit_closed_event(app_handle, nonce);
    }
}

fn task_gallery_width_logical(window_count: usize, monitor_width_logical: f64) -> f64 {
    let tab_count = window_count.max(1) as f64;
    let derived_width = tab_count * TASK_GALLERY_WIDTH_LOGICAL
        + ((tab_count - 1.0).max(0.0) * TASK_GALLERY_TAB_GAP_LOGICAL)
        + (TASK_GALLERY_EDGE_PADDING_LOGICAL * 2.0);
    let min_width = TASK_GALLERY_WIDTH_LOGICAL + (TASK_GALLERY_EDGE_PADDING_LOGICAL * 2.0);
    let max_width = monitor_width_logical.max(min_width);
    derived_width.clamp(min_width, max_width)
}

fn show_task_gallery_window(gallery: &WebviewWindow, activate: bool) -> Result<(), String> {
    if activate {
        return gallery
            .show()
            .map_err(|error| format!("Failed to show task gallery: {error}"));
    }
    #[cfg(target_os = "windows")]
    unsafe {
        let hwnd = gallery
            .hwnd()
            .map_err(|error| format!("Failed to read task gallery HWND: {error}"))?;
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    gallery
        .show()
        .map_err(|error| format!("Failed to show task gallery: {error}"))
}

#[tauri::command]
pub async fn show_task_gallery(
    window: WebviewWindow,
    app_handle: AppHandle,
    args: TaskGalleryShowArgs,
) -> Result<Option<TaskGalleryLayout>, String> {
    if window.label() != BOTTOM_BAR_LABEL {
        return Err("Unauthorized caller for command show_task_gallery".into());
    }
    // Pin the generation at receipt, not after waiting for native work.
    let expected_revision = state().lock().unwrap().revision;
    tauri::async_runtime::spawn_blocking(move || {
        let _lane = native_lane().lock().map_err(|_| "Gallery native lane poisoned")?;
        let nonce = args.nonce.clone();
        let result = show_gallery(&app_handle, args, expected_revision);
        if result.is_err() && state().lock().unwrap().nonce.as_deref() == Some(nonce.as_str()) {
            hide_gallery_and_reset(&app_handle);
        }
        result
    }).await.map_err(|e| e.to_string())?
}

fn show_gallery(app_handle: &AppHandle, args: TaskGalleryShowArgs, expected_revision: u64) -> Result<Option<TaskGalleryLayout>, String> {
    let same_session = {
        let mut runtime = state().lock().unwrap();
        if take_cancelled_nonce(&mut runtime, &args.nonce)
            || (args.refresh_existing && (runtime.nonce.as_deref() != Some(args.nonce.as_str()) || runtime.revision != expected_revision)) {
            return Ok(None);
        }
        let same = runtime.nonce.as_deref() == Some(args.nonce.as_str());
        if !same && runtime.nonce.is_some() { return Err("Another task gallery session is active".into()); }
        same
    };
    if args.nonce.is_empty() || !args.anchor_left.is_finite() || !args.anchor_width.is_finite() || args.anchor_width < 0.0 {
        return Err("Invalid task gallery show arguments".into());
    }
    let gallery = app_handle.get_webview_window(TASK_GALLERY_LABEL).ok_or("Task gallery window is unavailable")?;
    let bottom_bar = app_handle.get_webview_window(BOTTOM_BAR_LABEL).ok_or("Bottom bar window is unavailable")?;
    let scale_factor = bottom_bar.scale_factor().map_err(|e| e.to_string())?;
    if !scale_factor.is_finite() || scale_factor <= 0.0 { return Err("Invalid gallery scale".into()); }
    let bottom_position = bottom_bar.outer_position().map_err(|e| e.to_string())?;
    let monitor = bottom_bar.current_monitor().map_err(|e| e.to_string())?.ok_or("Gallery monitor unavailable")?;
    let padding = (8.0 * scale_factor).round() as i32;
    let min_x = monitor.position().x + padding;
    let min_y = monitor.position().y + padding;
    let available_width = (monitor.size().width as f64 / scale_factor - 16.0).max(0.0);
    let available_height = ((bottom_position.y - padding - min_y) as f64 / scale_factor).max(0.0);
    if available_width < 60.0 || available_height < 80.0 { return Err("Insufficient safe space for task gallery".into()); }
    let layout = task_gallery_layout(args.windows.len(), available_width, available_height);
    let mut allowed_hwnds = BTreeMap::new();
    for row in &args.windows {
        #[cfg(target_os = "windows")]
        let identity = task_windows::task_window_identity(&row.hwnd)?;
        if allowed_hwnds.insert(row.hwnd.clone(), TaskGalleryAuthorizedWindow {
            row: row.clone(),
            #[cfg(target_os = "windows")]
            identity,
        }).is_some() { return Err("Duplicate gallery HWND".into()); }
    }
    // Revoke stale work without dropping unchanged same-session destinations.
    // Changed layout/order/identity must retire overlays before host mutation.
    let (revision, old) = {
        let mut runtime = state().lock().unwrap();
        if take_cancelled_nonce(&mut runtime, &args.nonce) { return Ok(None); }
        let next_revision = runtime.revision.checked_add(1).filter(|n| *n <= thumbnails::MAX_SAFE_INTEGER).ok_or("Gallery revision exhausted")?;
        let reusable: Vec<String> = if same_session && runtime.layout.as_ref() == Some(&layout)
            && runtime.thumbnail_scale == Some(scale_factor) {
            args.windows.iter().enumerate().filter_map(|(index, row)| {
                if runtime.thumbnail_order.get(index) != Some(&row.hwnd) { return None; }
                let previous = runtime.windows_by_hwnd.get(&row.hwnd)?;
                let current = allowed_hwnds.get(&row.hwnd)?;
                #[cfg(target_os = "windows")]
                if previous.identity != current.identity { return None; }
                if previous.row.is_minimized != current.row.is_minimized { return None; }
                Some(row.hwnd.clone())
            }).collect()
        } else { Vec::new() };
        let old = runtime.thumbnails.retain_visible(reusable.iter().map(String::as_str));
        runtime.thumbnails.cancel();
        runtime.revision = next_revision;
        runtime.nonce = Some(args.nonce.clone());
        runtime.windows_by_hwnd = allowed_hwnds;
        runtime.layout = Some(layout.clone());
        runtime.thumbnail_scale = Some(scale_factor);
        runtime.thumbnail_order = args.windows.iter().map(|row| row.hwnd.clone()).collect();
        if !same_session { runtime.latest_thumbnail_request_id = 0; }
        (runtime.revision, old)
    };
    drop(old);
    let width = (layout.panel_width * scale_factor).round() as u32;
    let height = (layout.panel_height * scale_factor).round() as u32;
    let max_x = monitor.position().x + monitor.size().width as i32 - padding - width as i32;
    let anchor = bottom_position.x + ((args.anchor_left + args.anchor_width / 2.0) * scale_factor).round() as i32;
    let x = (anchor - width as i32 / 2).clamp(min_x, max_x.max(min_x));
    let y = (bottom_position.y - padding - height as i32).max(min_y);
    let result = (|| {
        validate_nonce(&args.nonce)?;
        gallery.set_size(PhysicalSize::new(width, height)).map_err(|e| e.to_string())?;
        validate_nonce(&args.nonce)?;
        gallery.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
        validate_nonce(&args.nonce)?;
        if !same_session { show_task_gallery_window(&gallery, args.focus_gallery)?; }
        validate_nonce(&args.nonce)?;
        if args.focus_gallery && !same_session { gallery.set_focus().map_err(|e| e.to_string())?; }
        validate_nonce(&args.nonce)?;
        app_handle.emit_to(TASK_GALLERY_LABEL, TASK_GALLERY_OPEN_EVENT, TaskGalleryOpenPayload {
            nonce: args.nonce.clone(), group_key: args.group_key, label: args.label,
            focus_gallery: args.focus_gallery, windows: args.windows, revision, layout: layout.clone(),
        }).map_err(|e| e.to_string())
    })();
    if let Err(error) = result {
        let cancelled = state().lock().unwrap().cancelled_nonces.contains(&args.nonce);
        record_cancelled_nonce(&mut state().lock().unwrap(), args.nonce.clone());
        hide_task_gallery_window(&gallery);
        reset_state();
        emit_closed_event(app_handle, Some(args.nonce));
        return if cancelled { Ok(None) } else { Err(error) };
    }
    Ok(Some(layout))
}

#[tauri::command]
pub async fn sync_task_gallery_thumbnails(window: WebviewWindow, args: SyncArgs) -> Result<SyncResponse, String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command sync_task_gallery_thumbnails".into());
    }
    {
        let mut runtime = state().lock().unwrap();
        thumbnails::admit_request(runtime.nonce.as_deref(), runtime.revision, runtime.latest_thumbnail_request_id, &args)?;
        if runtime.cancelled_nonces.contains(&args.nonce) { return Err("Cancelled gallery session".into()); }
        let layout = runtime.layout.as_ref().ok_or("Gallery layout unavailable")?;
        thumbnails::validate_frames(&args, layout, layout.panel_width, layout.panel_height)?;
        for frame in &args.frames { snapshot_window_from_runtime(&runtime, &args.nonce, &frame.hwnd)?; }
        runtime.latest_thumbnail_request_id = args.request_id;
        runtime.thumbnails.advance(thumbnail_generation(&args));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let result = sync_thumbnails(&window, &args);
        if result.is_err() {
            let _lane = native_lane().lock().map_err(|_| "Gallery native lane poisoned")?;
            let old = {
                let mut runtime = state().lock().unwrap();
                runtime.thumbnails.take_if_current(&thumbnail_generation(&args)).unwrap_or_default()
            };
            drop(old);
        }
        result
    }).await.map_err(|e| e.to_string())?
}

fn thumbnail_work_is_current(args: &SyncArgs) -> Result<(), String> {
    let runtime = state().lock().unwrap();
    if runtime.nonce.as_deref() != Some(args.nonce.as_str()) || runtime.revision != args.revision
        || runtime.latest_thumbnail_request_id != args.request_id || runtime.cancelled_nonces.contains(&args.nonce)
        || !runtime.thumbnails.is_current(&thumbnail_generation(args)) {
        return Err("Stale task gallery thumbnail work".into());
    }
    Ok(())
}

fn sync_thumbnails(window: &WebviewWindow, args: &SyncArgs) -> Result<SyncResponse, String> {
    let _lane = native_lane().lock().map_err(|_| "Gallery native lane poisoned")?;
    thumbnail_work_is_current(&args)?;
    let layout = {
        let runtime = state().lock().unwrap();
        for frame in &args.frames { snapshot_window_from_runtime(&runtime, &args.nonce, &frame.hwnd)?; }
        runtime.layout.clone().ok_or("Gallery layout unavailable")?
    };
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    if !scale.is_finite() || scale <= 0.0 { return Err("Invalid gallery scale".into()); }
    let client = window.inner_size().map_err(|e| e.to_string())?;
    thumbnails::validate_frames(&args, &layout, client.width as f64 / scale, client.height as f64 / scale)?;
    if !args.frames.is_empty() && !window.is_visible().map_err(|e| e.to_string())? {
        return Err("Gallery is hidden".into());
    }
    let mut old = {
        let mut runtime = state().lock().unwrap();
        runtime.thumbnails.take_if_current(&thumbnail_generation(args))
            .ok_or("Stale task gallery thumbnail work")?
    };
    // Drop omitted overlays before processing replacements. Native lane prevents
    // hide/refresh/new sync from racing a checked native update or publication.
    drop(old.retain_visible(args.frames.iter().map(|frame| frame.hwnd.as_str())));
    let mut staged = ResourceSet::default();
    let mut results = Vec::with_capacity(args.frames.len());
    for frame in &args.frames {
        let mut resource = old.remove(&frame.hwnd);
        let work = (|| -> Result<Resource, String> {
            thumbnail_work_is_current(&args)?;
            snapshot_window(&args.nonce, &frame.hwnd)?;
            if let Some(existing) = resource.take() {
                if let Some(ref live) = existing.live {
                    if live.update(frame.bounds, scale).is_ok() {
                        snapshot_window(&args.nonce, &frame.hwnd)?;
                        return Ok(existing);
                    }
                    // Failed update unregisters before capture, never leaves an
                    // old destination painting across frontend chrome.
                    drop(existing);
                } else {
                    snapshot_window(&args.nonce, &frame.hwnd)?;
                    return Ok(existing);
                }
            }
            let live = LiveThumbnail::register(window, &frame.hwnd)
                .and_then(|live| {
                    thumbnail_work_is_current(&args)?;
                    snapshot_window(&args.nonce, &frame.hwnd)?;
                    live.update(frame.bounds, scale)?;
                    snapshot_window(&args.nonce, &frame.hwnd)?;
                    Ok(live)
                });
            snapshot_window(&args.nonce, &frame.hwnd)?;
            thumbnail_work_is_current(&args)?;
            let captured = task_windows::capture_task_window_preview(frame.hwnd.clone());
            thumbnail_work_is_current(&args)?;
            snapshot_window(&args.nonce, &frame.hwnd)?;
            match live {
                Ok(live) => Ok(Resource { live: Some(live), result: Thumbnail {
                    hwnd: frame.hwnd.clone(), source: "live",
                    image_data_url: captured.ok().map(|image| image.image_data_url), error: None,
                }}),
                Err(_) => captured.map(|image| Resource { live: None, result: Thumbnail {
                    hwnd: frame.hwnd.clone(), source: "captured", image_data_url: Some(image.image_data_url), error: None,
                }}),
            }
        })();
        match work {
            Ok(resource) => { results.push(resource.result.clone()); drop(staged.replace(frame.hwnd.clone(), resource)); }
            Err(error) => {
                drop(resource);
                // Do not cache identity failures or transient unavailable sources.
                results.push(Thumbnail { hwnd: frame.hwnd.clone(), source: "unavailable", image_data_url: None, error: Some(error) });
            }
        }
    }
    let completion = {
        let mut runtime = state().lock().unwrap();
        runtime.thumbnails.finish(&thumbnail_generation(args), staged)
    };
    let accepted = completion.accepted;
    drop(completion.retired);
    if !accepted { return Err("Stale task gallery thumbnail work".into()); }
    Ok(SyncResponse { nonce: args.nonce.clone(), revision: args.revision, request_id: args.request_id, thumbnails: results })
}

#[tauri::command]
pub async fn hide_task_gallery(
    window: Window,
    app_handle: AppHandle,
    nonce: Option<String>,
) -> Result<(), String> {
    if window.label() != BOTTOM_BAR_LABEL && window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command hide_task_gallery".to_string());
    }
    let target_nonce = {
        let mut runtime = state().lock().unwrap();
        let target = nonce.or_else(|| runtime.nonce.clone());
        if let Some(ref nonce) = target {
            record_cancelled_nonce(&mut runtime, nonce.clone());
            if runtime.nonce.as_deref() == Some(nonce.as_str()) { runtime.thumbnails.cancel(); }
        }
        target
    };
    let _lane = native_lane().lock().map_err(|_| "Gallery native lane poisoned")?;
    let current_nonce = state().lock().unwrap().nonce.clone();
    if current_nonce.is_some() && current_nonce != target_nonce { return Ok(()); }
    hide_gallery_and_reset(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn hide_task_gallery_on_focus_loss(
    window: Window,
    app_handle: AppHandle,
) -> Result<(), String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command hide_task_gallery_on_focus_loss".to_string());
    }
    if focus_hold_active() {
        return Ok(());
    }
    // Native event callback executes on UI thread; never wait for the native
    // worker lane there. Pin this session so delayed focus loss cannot hide next.
    let nonce = state().lock().unwrap().nonce.clone();
    if nonce.is_some() {
        tauri::async_runtime::spawn(async move {
            let _ = hide_task_gallery(window, app_handle, nonce).await;
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn activate_task_gallery_window(
    window: WebviewWindow,
    app_handle: AppHandle,
    args: TaskGalleryActivateArgs,
) -> Result<(), String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command activate_task_gallery_window".to_string());
    }
    validate(&args.nonce, &args.hwnd)?;
    let result = task_windows::activate_task_window(args.hwnd, args.minimize_if_active);
    let _lane = native_lane().lock().map_err(|_| "Gallery native lane poisoned")?;
    if validate_nonce(&args.nonce).is_ok() { hide_gallery_and_reset(&app_handle); }
    result
}

#[tauri::command]
pub async fn show_task_gallery_window_context_menu(
    window: WebviewWindow,
    app_handle: AppHandle,
    args: TaskGalleryContextMenuArgs,
) -> Result<(), String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err(
            "Unauthorized caller for command show_task_gallery_window_context_menu".to_string(),
        );
    }
    let authorized = snapshot_window(&args.nonce, &args.hwnd)?;
    begin_focus_hold();
    let result = taskbar_menu::show_task_window_context_menu_for_owner(
        &app_handle,
        TASK_GALLERY_LABEL,
        taskbar_menu::ShowTaskWindowContextMenuRequest {
            hwnd: authorized.row.hwnd,
            process_id: Some(authorized.identity.process_id),
            is_minimized: authorized.row.is_minimized,
            x: args.x,
            y: args.y,
        },
    );
    end_focus_hold();
    let gallery_focused = app_handle
        .get_webview_window(TASK_GALLERY_LABEL)
        .and_then(|gallery| gallery.is_focused().ok())
        .unwrap_or(false);
    if !gallery_focused {
        let _lane = native_lane().lock().map_err(|_| "Gallery native lane poisoned")?;
        if validate_nonce(&args.nonce).is_ok() { hide_gallery_and_reset(&app_handle); }
    }
    result
}

#[tauri::command]
pub fn show_task_gallery_window_preview(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: tauri::State<'_, Mutex<task_preview::TaskPreviewRuntimeState>>,
    args: TaskGalleryPreviewArgs,
) -> Result<(), String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command show_task_gallery_window_preview".to_string());
    }
    validate(&args.nonce, &args.hwnd)?;
    let gallery = app_handle
        .get_webview_window(TASK_GALLERY_LABEL)
        .ok_or_else(|| "Task gallery window is unavailable".to_string())?;
    task_preview::show_task_window_preview_with_host(
        &app_handle,
        &gallery,
        state,
        task_preview::ShowTaskPreviewRequest {
            request_id: args.request_id,
            hwnd: args.hwnd,
            title: args.title,
            process_name: args.process_name,
            icon_data_url: args.icon_data_url,
            is_minimized: args.is_minimized,
            anchor_left: args.anchor_left,
            anchor_width: args.anchor_width,
            gallery_nonce: Some(args.nonce),
        },
    )
}

#[tauri::command]
pub fn close_task_gallery_window(
    window: WebviewWindow,
    args: TaskGalleryCloseWindowArgs,
) -> Result<(), String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command close_task_gallery_window".to_string());
    }
    request_gallery_close_with(
        window.label(),
        || snapshot_window(&args.nonce, &args.hwnd),
        |authorized| task_windows::request_close_task_window_with_identity(authorized.row.hwnd, authorized.identity),
    )
}

#[tauri::command]
pub async fn close_task_gallery_previewed_window(
    window: WebviewWindow,
    args: TaskGalleryClosePreviewedWindowArgs,
) -> Result<(), String> {
    if window.label() != crate::shell_windows::TASK_PREVIEW_LABEL {
        return Err(
            "Unauthorized caller for command close_task_gallery_previewed_window".to_string(),
        );
    }
    let authorized = snapshot_window(&args.nonce, &args.hwnd)?;
    task_windows::close_task_window_with_identity(authorized.row.hwnd, authorized.identity)?;
    let _lane = native_lane().lock().map_err(|_| "Gallery native lane poisoned")?;
    let removed = {
        let mut runtime = state().lock().unwrap();
        if runtime.nonce.as_deref() != Some(args.nonce.as_str()) { return Ok(()); }
        runtime.windows_by_hwnd.remove(&args.hwnd);
        runtime.thumbnails.remove(&args.hwnd)
    };
    drop(removed);
    Ok(())
}

#[tauri::command]
pub fn hide_task_gallery_window_preview(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: tauri::State<'_, Mutex<task_preview::TaskPreviewRuntimeState>>,
    nonce: String,
    hwnd: String,
    request_id: u64,
) -> Result<(), String> {
    if window.label() != TASK_GALLERY_LABEL {
        return Err("Unauthorized caller for command hide_task_gallery_window_preview".to_string());
    }
    let _ = hwnd;
    validate_nonce(&nonce)?;
    task_preview::hide_task_window_preview(app_handle, state, request_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn close_fixture() -> TaskGalleryRuntimeState {
        let row = TaskGalleryWindowRow {
            hwnd: "101".into(), title: "Fixture".into(), process_id: Some(42),
            process_name: "Fixture".into(), icon_data_url: String::new(),
            is_active: false, is_minimized: false,
        };
        let authorized = TaskGalleryAuthorizedWindow {
            row,
            #[cfg(target_os = "windows")]
            identity: task_windows::TaskWindowIdentity {
                process_id: 42, creation_time: 99,
                canonical_image_path: std::path::PathBuf::from(r"C:\fixture.exe"),
            },
        };
        TaskGalleryRuntimeState {
            nonce: Some("fixture-session".into()),
            windows_by_hwnd: BTreeMap::from([("101".into(), authorized)]),
            ..Default::default()
        }
    }

    #[test]
    fn gallery_close_wrong_caller_never_snapshots_or_dispatches() {
        for caller in ["", BOTTOM_BAR_LABEL, crate::shell_windows::TASK_PREVIEW_LABEL] {
            let result = request_gallery_close_with(caller,
                || panic!("unauthorized caller inspected target"),
                |_| panic!("unauthorized caller dispatched close"));
            assert!(result.unwrap_err().contains("Unauthorized caller"));
        }
    }

    #[test]
    fn gallery_close_stale_missing_nonce_or_membership_never_dispatches() {
        let mut runtime = close_fixture();
        for (nonce, hwnd) in [("old-session", "101"), ("", "101"), ("fixture-session", "999")] {
            let result = request_gallery_close_with(TASK_GALLERY_LABEL,
                || snapshot_window_from_runtime(&runtime, nonce, hwnd),
                |_| panic!("invalid session target dispatched"));
            assert!(result.is_err());
        }
        runtime.nonce = None;
        assert!(request_gallery_close_with(TASK_GALLERY_LABEL,
            || snapshot_window_from_runtime(&runtime, "fixture-session", "101"),
            |_| panic!("hidden session dispatched")).is_err());
    }

    #[test]
    fn gallery_close_success_and_failure_retain_authorization_for_retry() {
        let runtime = close_fixture();
        let calls = Cell::new(0);
        for outcome in [Ok(()), Err("dispatch access denied".to_string()), Ok(())] {
            let expected = outcome.clone();
            let result = request_gallery_close_with(TASK_GALLERY_LABEL,
                || snapshot_window_from_runtime(&runtime, "fixture-session", "101"),
                |authorized| {
                    assert_eq!(authorized.row.hwnd, "101");
                    #[cfg(target_os = "windows")]
                    assert_eq!(authorized.identity.process_id, 42);
                    calls.set(calls.get() + 1);
                    outcome
                });
            assert_eq!(result, expected);
            assert_eq!(runtime.nonce.as_deref(), Some("fixture-session"));
            assert_eq!(runtime.windows_by_hwnd.len(), 1);
            assert!(snapshot_window_from_runtime(&runtime, "fixture-session", "101").is_ok());
        }
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn task_gallery_width_scales_from_tab_count_and_clamps_to_monitor() {
        assert_eq!(task_gallery_width_logical(1, 1920.0), 160.0);
        assert_eq!(task_gallery_width_logical(4, 1920.0), 640.0);
        assert_eq!(task_gallery_width_logical(20, 900.0), 900.0);
    }
}
