use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use png::{BitDepth, ColorType, Encoder};
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetClassLongPtrW, GetIconInfo, SendMessageTimeoutW, GCLP_HICON, GCLP_HICONSM,
    HICON, ICONINFO, ICON_BIG, ICON_SMALL, ICON_SMALL2, SMTO_ABORTIFHUNG, SMTO_ERRORONEXIT,
    WM_GETICON,
};

pub(super) const EMPTY_ICON_DATA_URL: &str = "data:image/gif;base64,R0lGODlhAQABAAAAACw=";

const WM_GETICON_TIMEOUT_MS: u32 = 25;
const WINDOW_ICON_CACHE_CAPACITY: usize = 256;
const WINDOW_ICON_CACHE_TTL: Duration = Duration::from_secs(30 * 60);
const WINDOW_ICON_CACHE_NEGATIVE_TTL: Duration = Duration::from_secs(20);

static WINDOW_ICON_CACHE: OnceLock<
    Mutex<crate::task_windows::bounded_string_cache::BoundedStringCache<String>>,
> = OnceLock::new();

const HUNG_ICON_BACKOFF_TTL: Duration = Duration::from_secs(5);
const HUNG_ICON_BACKOFF_CAPACITY: usize = 256;

/// Short backoff for windows whose message loop did not answer `WM_GETICON`, keyed by
/// (hwnd, pid) so a reused HWND is never penalised. Entries expire (never permanent).
pub(super) struct HungIconBackoff {
    ttl: Duration,
    entries: HashMap<(isize, u32), Instant>,
}

impl HungIconBackoff {
    pub(super) fn new(ttl: Duration) -> Self {
        Self { ttl, entries: HashMap::new() }
    }

    pub(super) fn should_skip(&self, hwnd: isize, pid: u32, now: Instant) -> bool {
        self.entries
            .get(&(hwnd, pid))
            .is_some_and(|recorded_at| now < *recorded_at + self.ttl)
    }

    pub(super) fn record_unresponsive(&mut self, hwnd: isize, pid: u32, now: Instant) {
        let ttl = self.ttl;
        self.entries.retain(|_, recorded_at| now < *recorded_at + ttl);
        if self.entries.len() >= HUNG_ICON_BACKOFF_CAPACITY {
            self.entries.clear();
        }
        self.entries.insert((hwnd, pid), now);
    }

    pub(super) fn record_responsive(&mut self, hwnd: isize, pid: u32) {
        self.entries.remove(&(hwnd, pid));
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
}

static HUNG_ICON_BACKOFF: OnceLock<Mutex<HungIconBackoff>> = OnceLock::new();
/// Last icon returned per (hwnd, pid); served while a hung window is in backoff.
static LAST_WINDOW_ICON: OnceLock<Mutex<HashMap<(isize, u32), String>>> = OnceLock::new();

fn hung_icon_backoff() -> &'static Mutex<HungIconBackoff> {
    HUNG_ICON_BACKOFF.get_or_init(|| Mutex::new(HungIconBackoff::new(HUNG_ICON_BACKOFF_TTL)))
}

fn last_window_icons() -> &'static Mutex<HashMap<(isize, u32), String>> {
    LAST_WINDOW_ICON.get_or_init(|| Mutex::new(HashMap::new()))
}

fn remember_window_icon(key: (isize, u32), icon: &str) {
    if let Ok(mut icons) = last_window_icons().lock() {
        if icons.len() >= WINDOW_ICON_CACHE_CAPACITY && !icons.contains_key(&key) {
            icons.clear();
        }
        icons.insert(key, icon.to_string());
    }
}

pub(super) fn window_icon_data_url(
    hwnd: HWND,
    process_id: u32,
    process_path: Option<&Path>,
) -> Result<String, String> {
    let key = (hwnd.0 as isize, process_id);
    let skip = hung_icon_backoff()
        .lock()
        .map(|backoff| backoff.should_skip(key.0, key.1, Instant::now()))
        .unwrap_or(false);
    if skip {
        // No WM_GETICON (up to 3 x 25 ms) and no icon-cache write: the retry after the
        // backoff resolves the real icon.
        let known = last_window_icons().lock().ok().and_then(|icons| icons.get(&key).cloned());
        if let Some(known) = known {
            return Ok(known);
        }
        let fallback = match process_path {
            Some(path) => file_icon_data_url(path).unwrap_or_else(|_| empty_icon_data_url().to_string()),
            None => empty_icon_data_url().to_string(),
        };
        remember_window_icon(key, &fallback);
        return Ok(fallback);
    }

    let (icon_identity, unresponsive) = taskbar_icon_identity(hwnd);
    if let Ok(mut backoff) = hung_icon_backoff().lock() {
        if unresponsive {
            backoff.record_unresponsive(key.0, key.1, Instant::now());
        } else {
            backoff.record_responsive(key.0, key.1);
        }
    }
    let result = window_icon_data_url_with_resolver(
        hwnd,
        process_path,
        icon_identity,
        |_hwnd, process_path, icon| {
            if let Some(icon) = icon {
                let png = icon_to_png_bytes(icon)?;
                Ok(format!("data:image/png;base64,{}", BASE64.encode(png)))
            } else if let Some(process_path) = process_path {
                file_icon_data_url(process_path)
            } else {
                Ok(empty_icon_data_url().to_string())
            }
        },
    );
    if let Ok(icon) = &result {
        remember_window_icon(key, icon);
    }
    result
}

fn window_icon_data_url_with_resolver<F>(
    hwnd: HWND,
    process_path: Option<&Path>,
    icon_identity: WindowIconIdentity,
    resolver: F,
) -> Result<String, String>
where
    F: FnOnce(HWND, Option<&Path>, Option<HICON>) -> Result<String, String>,
{
    let key = window_icon_cache_key(hwnd, process_path, &icon_identity);
    let cache = WINDOW_ICON_CACHE.get_or_init(|| {
        Mutex::new(
            crate::task_windows::bounded_string_cache::BoundedStringCache::new(
                WINDOW_ICON_CACHE_CAPACITY,
                WINDOW_ICON_CACHE_TTL,
                WINDOW_ICON_CACHE_NEGATIVE_TTL,
            ),
        )
    });

    if let Ok(mut cache) = cache.lock() {
        if let Some(cached) = cache.get_cloned(&key) {
            return Ok(cached.unwrap_or_else(|| empty_icon_data_url().to_string()));
        }
    }

    let resolved = resolver(hwnd, process_path, icon_identity.hicon());

    if let Ok(mut cache) = cache.lock() {
        cache.insert(key, resolved.clone().ok());
    }

    resolved
}

fn window_icon_cache_key(
    hwnd: HWND,
    process_path: Option<&Path>,
    icon_identity: &WindowIconIdentity,
) -> String {
    let path = process_path
        .map(|path| path.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let mut key = String::with_capacity(path.len() + 64);
    let _ = write!(
        &mut key,
        "{}|{}|{}",
        hwnd.0 as usize,
        path,
        icon_identity.cache_token()
    );
    key
}

#[derive(Clone, Copy)]
enum WindowIconIdentity {
    Icon(HICON),
    Missing,
}

impl WindowIconIdentity {
    fn hicon(self) -> Option<HICON> {
        match self {
            Self::Icon(icon) => Some(icon),
            Self::Missing => None,
        }
    }

    fn cache_token(self) -> String {
        match self {
            Self::Icon(icon) => format!("icon:{:p}", icon.0),
            Self::Missing => "icon:none".to_string(),
        }
    }
}

/// Returns the identity and whether the window failed to answer `WM_GETICON` in time.
fn taskbar_icon_identity(hwnd: HWND) -> (WindowIconIdentity, bool) {
    let (handle, unresponsive) = taskbar_icon_handle(hwnd);
    (
        handle
            .map(WindowIconIdentity::Icon)
            .unwrap_or(WindowIconIdentity::Missing),
        unresponsive,
    )
}

fn empty_icon_data_url() -> &'static str {
    EMPTY_ICON_DATA_URL
}

fn taskbar_icon_handle(hwnd: HWND) -> (Option<HICON>, bool) {
    let mut unresponsive = false;
    for icon_kind in [ICON_SMALL2, ICON_SMALL, ICON_BIG] {
        match taskbar_icon_handle_with_timeout(hwnd, icon_kind as usize) {
            IconLookup::Found(icon) => return (Some(icon), false),
            IconLookup::Unavailable => continue,
            IconLookup::Unresponsive => {
                unresponsive = true;
                break;
            }
        }
    }

    for class_index in [GCLP_HICONSM, GCLP_HICON] {
        let icon_value = unsafe { GetClassLongPtrW(hwnd, class_index) };
        let icon = HICON(icon_value as *mut _);
        if !icon.0.is_null() {
            return (Some(icon), unresponsive);
        }
    }

    (None, unresponsive)
}

enum IconLookup {
    Found(HICON),
    Unavailable,
    Unresponsive,
}

fn taskbar_icon_handle_with_timeout(hwnd: HWND, icon_kind: usize) -> IconLookup {
    let mut result = 0usize;
    let status = unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_GETICON,
            WPARAM(icon_kind),
            LPARAM(0),
            SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT,
            WM_GETICON_TIMEOUT_MS,
            Some(&mut result),
        )
    };

    if status.0 == 0 {
        return IconLookup::Unresponsive;
    }

    let icon = HICON(result as *mut _);
    if icon.0.is_null() {
        IconLookup::Unavailable
    } else {
        IconLookup::Found(icon)
    }
}

pub(super) fn file_icon_data_url(path: &Path) -> Result<String, String> {
    let path_wide = to_wide(path);
    let mut icon_info = SHFILEINFOW::default();
    let icon_result = unsafe {
        SHGetFileInfoW(
            PCWSTR(path_wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut icon_info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        )
    };

    if icon_result == 0 || icon_info.hIcon.0.is_null() {
        return Err(format!("Failed to extract icon for {}", path.display()));
    }

    let png_result = icon_to_png_bytes(icon_info.hIcon);

    unsafe {
        let _ = DestroyIcon(icon_info.hIcon);
    }

    Ok(format!(
        "data:image/png;base64,{}",
        BASE64.encode(png_result?)
    ))
}

fn icon_to_png_bytes(icon_handle: HICON) -> Result<Vec<u8>, String> {
    let (width, height, pixels) = icon_to_rgba(icon_handle)?;
    let mut png_bytes = Vec::new();
    let mut encoder = Encoder::new(&mut png_bytes, width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("Failed to start PNG encoding: {error}"))?;
    writer
        .write_image_data(&pixels)
        .map_err(|error| format!("Failed to encode task window icon: {error}"))?;

    drop(writer);

    Ok(png_bytes)
}

fn icon_to_rgba(icon_handle: HICON) -> Result<(u32, u32, Vec<u8>), String> {
    let mut icon = ICONINFO::default();

    unsafe {
        GetIconInfo(icon_handle, &mut icon)
            .map_err(|error| format!("Failed to read icon metadata: {error}"))?;
    }

    let conversion_result = (|| {
        if icon.hbmColor.0.is_null() {
            return Err("Task window icon does not expose a color bitmap".to_string());
        }

        let mut bitmap = BITMAP::default();
        let object_size = unsafe {
            GetObjectW(
                icon.hbmColor.into(),
                size_of::<BITMAP>() as i32,
                Some((&mut bitmap as *mut BITMAP).cast()),
            )
        };

        if object_size == 0 {
            return Err("Failed to inspect task window icon bitmap".to_string());
        }

        let width = bitmap.bmWidth as i32;
        let height = bitmap.bmHeight as i32;
        if width <= 0 || height <= 0 {
            return Err("Task window icon bitmap dimensions are invalid".to_string());
        }

        let mut pixels = vec![0_u8; (width * height * 4) as usize];
        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let dc = unsafe { CreateCompatibleDC(Some(HDC::default())) };

        if dc.0.is_null() {
            return Err("Failed to create task window icon device context".to_string());
        }

        let scanlines = unsafe {
            GetDIBits(
                dc,
                icon.hbmColor,
                0,
                height as u32,
                Some(pixels.as_mut_ptr().cast()),
                &mut bitmap_info,
                DIB_RGB_COLORS,
            )
        };

        unsafe {
            let _ = DeleteDC(dc);
        }

        if scanlines == 0 {
            return Err("Failed to read task window icon pixels".to_string());
        }

        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        Ok((width as u32, height as u32, pixels))
    })();

    unsafe {
        delete_bitmap(icon.hbmColor);
        delete_bitmap(icon.hbmMask);
    }

    conversion_result
}

unsafe fn delete_bitmap(bitmap: HBITMAP) {
    if !bitmap.0.is_null() {
        let _ = DeleteObject(bitmap.into());
    }
}

fn to_wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn source_helper_keeps_resolution_outside_lock() {
        let calls = AtomicUsize::new(0);
        let result = window_icon_data_url_with_resolver(
            HWND(1 as *mut _),
            None,
            WindowIconIdentity::Missing,
            |_, _, _| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok("data:image/png;base64,test".to_string())
            },
        );

        assert_eq!(result.as_deref(), Ok("data:image/png;base64,test"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cache_key_changes_with_icon_identity() {
        let hwnd = HWND(7 as *mut _);
        let path = Some(Path::new("C:/Windows/explorer.exe"));
        let missing = window_icon_cache_key(hwnd, path, &WindowIconIdentity::Missing);
        let icon_a =
            window_icon_cache_key(hwnd, path, &WindowIconIdentity::Icon(HICON(1 as *mut _)));
        let icon_b =
            window_icon_cache_key(hwnd, path, &WindowIconIdentity::Icon(HICON(2 as *mut _)));

        assert_ne!(missing, icon_a);
        assert_ne!(icon_a, icon_b);
    }

    #[test]
    fn cache_hit_skips_resolution_even_with_icon_identity() {
        let cache = WINDOW_ICON_CACHE.get_or_init(|| {
            Mutex::new(
                crate::task_windows::bounded_string_cache::BoundedStringCache::new(
                    WINDOW_ICON_CACHE_CAPACITY,
                    WINDOW_ICON_CACHE_TTL,
                    WINDOW_ICON_CACHE_NEGATIVE_TTL,
                ),
            )
        });

        let hwnd = HWND(11 as *mut _);
        let key = window_icon_cache_key(hwnd, None, &WindowIconIdentity::Missing);
        cache
            .lock()
            .unwrap()
            .insert(key, Some("data:image/png;base64,cached".to_string()));

        let calls = AtomicUsize::new(0);
        let result = window_icon_data_url_with_resolver(
            hwnd,
            None,
            WindowIconIdentity::Missing,
            |_, _, _| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok("data:image/png;base64,miss".to_string())
            },
        );

        assert_eq!(result.as_deref(), Ok("data:image/png;base64,cached"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
