use crate::shell_windows::{
    BOTTOM_BAR_LABEL, TASK_PREVIEW_HEIGHT_LOGICAL, TASK_PREVIEW_LABEL, TASK_PREVIEW_WIDTH_LOGICAL,
};
use crate::task_windows;
#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
#[cfg(target_os = "windows")]
use tauri::WebviewWindow;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, RECT, SIZE};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dwm::{
    DwmQueryThumbnailSourceSize, DwmRegisterThumbnail, DwmUnregisterThumbnail,
    DwmUpdateThumbnailProperties, DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY,
    DWM_TNP_RECTDESTINATION, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
};

const TASK_PREVIEW_UPDATE_EVENT: &str = "task-preview:update";
const TASK_PREVIEW_HIDE_EVENT: &str = "task-preview:hide";
const TASK_PREVIEW_MARGIN_PHYSICAL: i32 = 10;
const TASK_PREVIEW_EDGE_PADDING_PHYSICAL: i32 = 8;
const LIVE_PREVIEW_FRAME_TOP_LOGICAL: f64 = 48.0;
const LIVE_PREVIEW_FRAME_SIDE_LOGICAL: f64 = 4.0;
const LIVE_PREVIEW_FRAME_BOTTOM_LOGICAL: f64 = 4.0;
const LIVE_THUMBNAIL_PLACEHOLDER_DATA_URL: &str =
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNgYAAAAAMAASsJTYQAAAAASUVORK5CYII=";

#[derive(Default)]
pub struct TaskPreviewRuntimeState {
    pub latest_request_id: u64,
    live_thumbnail: Option<LiveTaskThumbnail>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowTaskPreviewRequest {
    pub request_id: u64,
    pub hwnd: String,
    pub title: String,
    pub process_name: String,
    pub icon_data_url: String,
    pub is_minimized: bool,
    pub anchor_left: f64,
    pub anchor_width: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskPreviewPayload {
    hwnd: String,
    title: String,
    process_name: String,
    icon_data_url: String,
    is_minimized: bool,
    preview_source: TaskPreviewSource,
    native_live_thumbnail_active: bool,
    image_data_url: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum TaskPreviewSource {
    NativeDwmThumbnail,
    CapturedImage,
    Unavailable,
}

struct LiveTaskThumbnail {
    #[cfg(target_os = "windows")]
    handle: isize,
}

#[cfg(target_os = "windows")]
impl Drop for LiveTaskThumbnail {
    fn drop(&mut self) {
        if self.handle == 0 {
            return;
        }

        unsafe {
            let _ = DwmUnregisterThumbnail(self.handle);
        }
    }
}

#[tauri::command]
pub fn show_task_window_preview(
    app_handle: AppHandle,
    state: tauri::State<'_, Mutex<TaskPreviewRuntimeState>>,
    request: ShowTaskPreviewRequest,
) -> Result<(), String> {
    let preview_window = app_handle
        .get_webview_window(TASK_PREVIEW_LABEL)
        .ok_or_else(|| "Task preview window is unavailable".to_string())?;
    let _ = preview_window.hide();

    {
        let mut state = state
            .lock()
            .map_err(|_| "task preview runtime state is poisoned".to_string())?;
        begin_task_preview_request(&mut state, request.request_id);
    }

    let bottom_bar = app_handle
        .get_webview_window(BOTTOM_BAR_LABEL)
        .ok_or_else(|| "Bottom bar window is unavailable".to_string())?;
    let scale_factor = bottom_bar
        .scale_factor()
        .map_err(|error| format!("Failed to read the taskbar scale factor: {error}"))?;
    let bottom_position = bottom_bar
        .outer_position()
        .map_err(|error| format!("Failed to read the taskbar position: {error}"))?;
    let bottom_size = bottom_bar
        .outer_size()
        .map_err(|error| format!("Failed to read the taskbar size: {error}"))?;
    let preview_width = (TASK_PREVIEW_WIDTH_LOGICAL * scale_factor).round() as i32;
    let preview_height = (TASK_PREVIEW_HEIGHT_LOGICAL * scale_factor).round() as i32;
    let anchor_midpoint = request.anchor_left + (request.anchor_width / 2.0);
    let anchor_midpoint_physical =
        bottom_position.x + (anchor_midpoint * scale_factor).round() as i32;
    let min_x = bottom_position.x + TASK_PREVIEW_EDGE_PADDING_PHYSICAL;
    let max_x = bottom_position.x + bottom_size.width as i32
        - preview_width
        - TASK_PREVIEW_EDGE_PADDING_PHYSICAL;
    let preview_x = (anchor_midpoint_physical - (preview_width / 2)).clamp(min_x, max_x.max(min_x));
    let preview_y = bottom_position.y - preview_height - TASK_PREVIEW_MARGIN_PHYSICAL;
    let request_id = request.request_id;
    let live_thumbnail = register_live_task_thumbnail(&preview_window, &request.hwnd, scale_factor);
    let payload = match live_thumbnail {
        Ok(live_thumbnail) => {
            let mut state = state
                .lock()
                .map_err(|_| "task preview runtime state is poisoned".to_string())?;
            if !preview_request_is_current(&state, request.request_id) {
                return Ok(());
            }
            state.live_thumbnail = Some(live_thumbnail);

            TaskPreviewPayload {
                hwnd: request.hwnd,
                title: request.title,
                process_name: request.process_name,
                icon_data_url: request.icon_data_url,
                is_minimized: request.is_minimized,
                preview_source: TaskPreviewSource::NativeDwmThumbnail,
                native_live_thumbnail_active: true,
                image_data_url: Some(LIVE_THUMBNAIL_PLACEHOLDER_DATA_URL.to_string()),
                width: Some((preview_width.max(0)) as u32),
                height: Some((preview_height.max(0)) as u32),
                error: None,
            }
        }
        Err(live_error) => match fallback_preview_payload(request, live_error, &state)? {
            Some(payload) => payload,
            None => return Ok(()),
        },
    };

    publish_and_show_preview(
        &preview_window,
        payload,
        preview_x,
        preview_y,
        &state,
        request_id,
    )
}

fn fallback_preview_payload(
    request: ShowTaskPreviewRequest,
    live_error: String,
    state: &tauri::State<'_, Mutex<TaskPreviewRuntimeState>>,
) -> Result<Option<TaskPreviewPayload>, String> {
    let preview_image = task_windows::capture_task_window_preview(request.hwnd.clone());

    {
        let state = state
            .lock()
            .map_err(|_| "task preview runtime state is poisoned".to_string())?;
        if !preview_request_is_current(&state, request.request_id) {
            return Ok(None);
        }
    }

    let payload = match preview_image {
        Ok(preview_image) => TaskPreviewPayload {
            hwnd: request.hwnd,
            title: request.title,
            process_name: request.process_name,
            icon_data_url: request.icon_data_url,
            is_minimized: request.is_minimized,
            preview_source: TaskPreviewSource::CapturedImage,
            native_live_thumbnail_active: false,
            image_data_url: Some(preview_image.image_data_url),
            width: Some(preview_image.width),
            height: Some(preview_image.height),
            error: None,
        },
        Err(error) => TaskPreviewPayload {
            hwnd: request.hwnd,
            title: request.title,
            process_name: request.process_name,
            icon_data_url: request.icon_data_url,
            is_minimized: request.is_minimized,
            preview_source: TaskPreviewSource::Unavailable,
            native_live_thumbnail_active: false,
            image_data_url: None,
            width: None,
            height: None,
            error: Some(format!("{live_error}; GDI fallback failed: {error}")),
        },
    };

    Ok(Some(payload))
}

#[tauri::command]
pub fn hide_task_window_preview(
    app_handle: AppHandle,
    state: tauri::State<'_, Mutex<TaskPreviewRuntimeState>>,
    request_id: u64,
) -> Result<(), String> {
    {
        let mut state = state
            .lock()
            .map_err(|_| "task preview runtime state is poisoned".to_string())?;
        begin_task_preview_hide(&mut state, request_id);
    }

    let preview_window = app_handle
        .get_webview_window(TASK_PREVIEW_LABEL)
        .ok_or_else(|| "Task preview window is unavailable".to_string())?;
    preview_window
        .emit(TASK_PREVIEW_HIDE_EVENT, ())
        .map_err(|error| format!("Failed to clear task preview data: {error}"))?;
    preview_window
        .hide()
        .map_err(|error| format!("Failed to hide the task preview window: {error}"))
}

fn publish_and_show_preview(
    preview_window: &tauri::WebviewWindow,
    payload: TaskPreviewPayload,
    preview_x: i32,
    preview_y: i32,
    state: &tauri::State<'_, Mutex<TaskPreviewRuntimeState>>,
    request_id: u64,
) -> Result<(), String> {
    if !ensure_preview_request_is_current(state, request_id)? {
        return Ok(());
    }
    preview_window
        .emit(TASK_PREVIEW_UPDATE_EVENT, payload)
        .map_err(|error| {
            let _ = clear_active_live_thumbnail_if_current(state, request_id);
            format!("Failed to publish task preview data: {error}")
        })?;
    if !ensure_preview_request_is_current(state, request_id)? {
        return Ok(());
    }
    preview_window
        .set_position(PhysicalPosition::new(preview_x, preview_y))
        .map_err(|error| {
            let _ = clear_active_live_thumbnail_if_current(state, request_id);
            format!("Failed to position the task preview window: {error}")
        })?;
    if !ensure_preview_request_is_current(state, request_id)? {
        return Ok(());
    }
    preview_window.show().map_err(|error| {
        let _ = clear_active_live_thumbnail_if_current(state, request_id);
        format!("Failed to show the task preview window: {error}")
    })
}

fn ensure_preview_request_is_current(
    state: &tauri::State<'_, Mutex<TaskPreviewRuntimeState>>,
    request_id: u64,
) -> Result<bool, String> {
    let state = state
        .lock()
        .map_err(|_| "task preview runtime state is poisoned".to_string())?;
    Ok(preview_request_is_current(&state, request_id))
}

fn clear_active_live_thumbnail_if_current(
    state: &tauri::State<'_, Mutex<TaskPreviewRuntimeState>>,
    request_id: u64,
) -> Result<(), String> {
    let mut state = state
        .lock()
        .map_err(|_| "task preview runtime state is poisoned".to_string())?;
    if preview_request_is_current(&state, request_id) {
        clear_active_live_thumbnail(&mut state);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn register_live_task_thumbnail(
    preview_window: &WebviewWindow,
    source_hwnd: &str,
    scale_factor: f64,
) -> Result<LiveTaskThumbnail, String> {
    let source = task_windows::validate_task_window_preview_source(source_hwnd)?;
    let destination = hwnd_from_tauri_window(preview_window)?;
    let thumbnail = LiveTaskThumbnail {
        handle: unsafe { DwmRegisterThumbnail(destination, source) }
            .map_err(|error| format!("DWM thumbnail registration failed: {error}"))?,
    };
    let source_size = unsafe { DwmQueryThumbnailSourceSize(thumbnail.handle) }
        .map_err(|error| format!("DWM thumbnail source size query failed: {error}"))?;
    let frame = live_thumbnail_frame_rect(
        TASK_PREVIEW_WIDTH_LOGICAL,
        TASK_PREVIEW_HEIGHT_LOGICAL,
        scale_factor,
    );
    let destination = fit_source_in_destination_frame(frame, source_size);
    let properties = live_thumbnail_properties(destination);

    unsafe { DwmUpdateThumbnailProperties(thumbnail.handle, &properties) }
        .map_err(|error| format!("DWM thumbnail update failed: {error}"))?;

    Ok(thumbnail)
}

#[cfg(not(target_os = "windows"))]
fn register_live_task_thumbnail(
    _preview_window: &tauri::WebviewWindow,
    _source_hwnd: &str,
    _scale_factor: f64,
) -> Result<LiveTaskThumbnail, String> {
    Err("DWM live task previews are only supported on Windows".to_string())
}

#[cfg(target_os = "windows")]
fn hwnd_from_tauri_window(window: &WebviewWindow) -> Result<HWND, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("Failed to read task preview HWND: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut _)),
        other => Err(format!("Unsupported task preview window handle: {other:?}")),
    }
}

#[cfg(target_os = "windows")]
fn live_thumbnail_properties(destination: RECT) -> DWM_THUMBNAIL_PROPERTIES {
    DWM_THUMBNAIL_PROPERTIES {
        dwFlags: DWM_TNP_RECTDESTINATION
            | DWM_TNP_VISIBLE
            | DWM_TNP_OPACITY
            | DWM_TNP_SOURCECLIENTAREAONLY,
        rcDestination: destination,
        opacity: u8::MAX,
        fVisible: true.into(),
        fSourceClientAreaOnly: false.into(),
        ..Default::default()
    }
}

#[cfg(target_os = "windows")]
fn live_thumbnail_frame_rect(width_logical: f64, height_logical: f64, scale_factor: f64) -> RECT {
    let width = (width_logical * scale_factor).round() as i32;
    let height = (height_logical * scale_factor).round() as i32;
    let side = (LIVE_PREVIEW_FRAME_SIDE_LOGICAL * scale_factor).round() as i32;
    let top = (LIVE_PREVIEW_FRAME_TOP_LOGICAL * scale_factor).round() as i32;
    let bottom = (LIVE_PREVIEW_FRAME_BOTTOM_LOGICAL * scale_factor).round() as i32;
    let frame = RECT {
        left: side,
        top,
        right: width - side,
        bottom: height - bottom,
    };

    if frame.right > frame.left && frame.bottom > frame.top {
        frame
    } else {
        RECT {
            left: 0,
            top: 0,
            right: width.max(0),
            bottom: height.max(0),
        }
    }
}

#[cfg(target_os = "windows")]
fn fit_source_in_destination_frame(frame: RECT, source_size: SIZE) -> RECT {
    let frame_width = (frame.right - frame.left).max(1);
    let frame_height = (frame.bottom - frame.top).max(1);
    let source_width = source_size.cx.max(1);
    let source_height = source_size.cy.max(1);
    let scale = f64::min(
        frame_width as f64 / source_width as f64,
        frame_height as f64 / source_height as f64,
    );
    let width = ((source_width as f64 * scale).round() as i32).clamp(1, frame_width);
    let height = ((source_height as f64 * scale).round() as i32).clamp(1, frame_height);
    let left = frame.left + ((frame_width - width) / 2);
    let top = frame.top + ((frame_height - height) / 2);

    RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}

fn clear_active_live_thumbnail(state: &mut TaskPreviewRuntimeState) {
    state.live_thumbnail = None;
}

fn begin_task_preview_request(state: &mut TaskPreviewRuntimeState, request_id: u64) {
    state.latest_request_id = request_id;
    clear_active_live_thumbnail(state);
}

fn begin_task_preview_hide(state: &mut TaskPreviewRuntimeState, request_id: u64) {
    state.latest_request_id = request_id;
    clear_active_live_thumbnail(state);
}

fn preview_request_is_current(state: &TaskPreviewRuntimeState, request_id: u64) -> bool {
    state.latest_request_id == request_id
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::{
        fit_source_in_destination_frame, live_thumbnail_frame_rect, live_thumbnail_properties,
        DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
    };
    use windows::Win32::Foundation::{RECT, SIZE};

    #[test]
    fn live_thumbnail_frame_scales_with_preview_window() {
        let frame = live_thumbnail_frame_rect(332.0, 228.0, 1.5);

        assert_eq!(
            frame,
            RECT {
                left: 6,
                top: 72,
                right: 492,
                bottom: 336,
            }
        );
    }

    #[test]
    fn live_thumbnail_destination_preserves_source_aspect_ratio() {
        let frame = RECT {
            left: 4,
            top: 48,
            right: 324,
            bottom: 224,
        };
        let destination = fit_source_in_destination_frame(frame, SIZE { cx: 1920, cy: 1080 });

        assert_eq!(
            destination,
            RECT {
                left: 7,
                top: 48,
                right: 320,
                bottom: 224,
            }
        );
    }

    #[test]
    fn live_thumbnail_properties_make_destination_visible() {
        let rect = RECT {
            left: 1,
            top: 2,
            right: 3,
            bottom: 4,
        };
        let properties = live_thumbnail_properties(rect);
        let flags = properties.dwFlags;
        let opacity = properties.opacity;
        // SAFETY: `DWM_THUMBNAIL_PROPERTIES` is packed by the Windows binding, so
        // multi-byte fields must be copied with unaligned reads in tests too.
        let destination = unsafe { std::ptr::addr_of!(properties.rcDestination).read_unaligned() };
        let visible = unsafe { std::ptr::addr_of!(properties.fVisible).read_unaligned() };
        let source_client_only =
            unsafe { std::ptr::addr_of!(properties.fSourceClientAreaOnly).read_unaligned() };

        assert_eq!(
            flags,
            DWM_TNP_RECTDESTINATION
                | DWM_TNP_VISIBLE
                | DWM_TNP_OPACITY
                | DWM_TNP_SOURCECLIENTAREAONLY
        );
        assert_eq!(destination.left, 1);
        assert_eq!(destination.top, 2);
        assert_eq!(destination.right, 3);
        assert_eq!(destination.bottom, 4);
        assert_eq!(opacity, u8::MAX);
        assert!(visible.as_bool());
        assert!(!source_client_only.as_bool());
    }

    #[test]
    fn stale_preview_request_does_not_match_latest_request_id() {
        let state = super::TaskPreviewRuntimeState {
            latest_request_id: 42,
            live_thumbnail: None,
        };

        assert!(super::preview_request_is_current(&state, 42));
        assert!(!super::preview_request_is_current(&state, 41));
    }

    #[test]
    fn request_hide_request_sequence_rejects_stale_hover_and_hide_generations() {
        let mut state = super::TaskPreviewRuntimeState::default();

        super::begin_task_preview_request(&mut state, 10);
        assert!(super::preview_request_is_current(&state, 10));

        super::begin_task_preview_hide(&mut state, 11);
        assert!(!super::preview_request_is_current(&state, 10));
        assert!(super::preview_request_is_current(&state, 11));

        super::begin_task_preview_request(&mut state, 12);
        assert!(!super::preview_request_is_current(&state, 10));
        assert!(!super::preview_request_is_current(&state, 11));
        assert!(super::preview_request_is_current(&state, 12));
    }

    #[test]
    fn clear_active_live_thumbnail_drops_stale_handle_from_state() {
        let mut state = super::TaskPreviewRuntimeState {
            latest_request_id: 42,
            live_thumbnail: Some(super::LiveTaskThumbnail { handle: 0 }),
        };

        super::clear_active_live_thumbnail(&mut state);

        assert!(state.live_thumbnail.is_none());
    }
}
