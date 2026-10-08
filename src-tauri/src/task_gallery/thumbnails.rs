//! Bounded thumbnail admission and owned native resources. No OS work in runtime locks.
use super::layout::TaskGalleryLayout;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_VISIBLE_FRAMES: usize = 12;
pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bounds { pub left: f64, pub top: f64, pub width: f64, pub height: f64 }
impl Bounds {
    fn valid(self) -> bool {
        [self.left, self.top, self.width, self.height].iter().all(|n| n.is_finite() && *n >= 0.0)
            && self.width > 0.0 && self.height > 0.0
            && (self.left + self.width).is_finite() && (self.top + self.height).is_finite()
    }
    fn contains(self, other: Self) -> bool {
        other.left >= self.left && other.top >= self.top
            && other.left + other.width <= self.left + self.width
            && other.top + other.height <= self.top + self.height
    }
    fn overlaps(self, other: Self) -> bool {
        self.left < other.left + other.width && other.left < self.left + self.width
            && self.top < other.top + other.height && other.top < self.top + self.height
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Frame { pub hwnd: String, pub bounds: Bounds }
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncArgs {
    pub nonce: String, pub revision: u64, pub request_id: u64,
    pub viewport: Bounds, pub frames: Vec<Frame>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnail {
    pub hwnd: String, pub source: &'static str,
    pub image_data_url: Option<String>, pub error: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    pub nonce: String, pub revision: u64, pub request_id: u64, pub thumbnails: Vec<Thumbnail>,
}

/// Pure admission seam: rejects stale generation before any HWND/OS inspection.
pub fn admit_request(nonce: Option<&str>, revision: u64, latest_request_id: u64, args: &SyncArgs) -> Result<(), String> {
    if nonce != Some(args.nonce.as_str()) || revision == 0 || revision != args.revision {
        return Err("Stale task gallery revision or nonce".into());
    }
    if args.request_id == 0 || args.request_id > MAX_SAFE_INTEGER || args.request_id <= latest_request_id {
        return Err("Stale or invalid task gallery thumbnail requestId".into());
    }
    Ok(())
}

/// Only fully contained thumbnails may become native overlays. Partial cards use
/// renderer fallback; overlays/menus must submit the empty set before painting.
pub fn validate_frames(args: &SyncArgs, layout: &TaskGalleryLayout, client_width: f64, client_height: f64) -> Result<(), String> {
    let client = Bounds { left: 0.0, top: 0.0, width: client_width, height: client_height };
    let safe = Bounds { left: 8.0, top: 40.0, width: layout.viewport_width,
        height: layout.viewport_height };
    if args.frames.len() > MAX_VISIBLE_FRAMES { return Err("Too many visible gallery frames".into()); }
    // Cleanup remains valid after unmount/hidden content collapses to zero.
    if args.frames.is_empty() && [args.viewport.left, args.viewport.top, args.viewport.width, args.viewport.height]
        .iter().all(|n| n.is_finite() && *n >= 0.0) && client.contains(args.viewport) {
        return Ok(());
    }
    if !args.viewport.valid() || !client.contains(args.viewport) || !safe.contains(args.viewport) {
        return Err("Gallery viewport is outside the safe content region".into());
    }
    let frame_region = Bounds { width: (safe.width - if layout.mode == "matrix" { 12.0 } else { 0.0 }).max(0.0), ..safe };
    let mut hwnds = BTreeSet::new();
    for (index, frame) in args.frames.iter().enumerate() {
        if !frame.bounds.valid() || !client.contains(frame.bounds) || !args.viewport.contains(frame.bounds) || !frame_region.contains(frame.bounds)
            || frame.bounds.width > layout.thumbnail_width || frame.bounds.height > layout.thumbnail_height
            || !hwnds.insert(&frame.hwnd)
            || args.frames[..index].iter().any(|other| frame.bounds.overlaps(other.bounds)) {
            return Err("Unsafe gallery thumbnail bounds or duplicate HWND".into());
        }
    }
    Ok(())
}

pub struct Resource { pub live: Option<LiveThumbnail>, pub result: Thumbnail }

pub struct LiveThumbnail {
    #[cfg(target_os = "windows")]
    handle: isize,
}
#[cfg(target_os = "windows")]
impl Drop for LiveThumbnail {
    fn drop(&mut self) { unsafe { let _ = windows::Win32::Graphics::Dwm::DwmUnregisterThumbnail(self.handle); } }
}

impl LiveThumbnail {
    #[cfg(target_os = "windows")]
    pub fn register(window: &tauri::WebviewWindow, hwnd: &str) -> Result<Self, String> {
        let source = crate::task_windows::validate_task_window_preview_source(hwnd)?;
        let destination = window.hwnd().map_err(|e| e.to_string())?;
        let handle = unsafe { windows::Win32::Graphics::Dwm::DwmRegisterThumbnail(destination, source) }.map_err(|e| e.to_string())?;
        // Registration starts hidden. Query/update failures drop this exact handle.
        Ok(Self { handle })
    }
    #[cfg(not(target_os = "windows"))]
    pub fn register(_: &tauri::WebviewWindow, _: &str) -> Result<Self, String> { Err("DWM is unavailable".into()) }

    #[cfg(target_os = "windows")]
    pub fn update(&self, bounds: Bounds, scale: f64) -> Result<(), String> {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::Graphics::Dwm::*;
        let size = unsafe { DwmQueryThumbnailSourceSize(self.handle) }.map_err(|e| e.to_string())?;
        if size.cx <= 0 || size.cy <= 0 { return Err("Invalid DWM source size".into()); }
        let left = (bounds.left * scale).round() as i32;
        let top = (bounds.top * scale).round() as i32;
        let right = ((bounds.left + bounds.width) * scale).round() as i32;
        let bottom = ((bounds.top + bounds.height) * scale).round() as i32;
        let ratio = ((right - left) as f64 / size.cx as f64).min((bottom - top) as f64 / size.cy as f64);
        let width = (size.cx as f64 * ratio).round() as i32;
        let height = (size.cy as f64 * ratio).round() as i32;
        if width <= 0 || height <= 0 { return Err("Empty DWM destination".into()); }
        let x = left + (right - left - width) / 2;
        let y = top + (bottom - top - height) / 2;
        let properties = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_VISIBLE | DWM_TNP_OPACITY | DWM_TNP_SOURCECLIENTAREAONLY,
            rcDestination: RECT { left: x, top: y, right: x + width, bottom: y + height },
            opacity: 255, fVisible: true.into(), fSourceClientAreaOnly: false.into(), ..Default::default()
        };
        unsafe { DwmUpdateThumbnailProperties(self.handle, &properties) }.map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    pub fn update(&self, _: Bounds, _: f64) -> Result<(), String> { Err("DWM is unavailable".into()) }
}
