use super::{parse_hwnd, TaskWindowPreviewImage};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use png::{BitDepth, ColorType, Encoder};
use std::mem::size_of;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    GetWindowDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT,
    DIB_RGB_COLORS, HBITMAP, HDC, ROP_CODE, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, IsHungAppWindow, IsIconic, IsWindow, IsWindowVisible,
};

const MAX_PREVIEW_WIDTH: u32 = 320;
const MAX_PREVIEW_HEIGHT: u32 = 180;
const MAX_CAPTURE_DIMENSION: u32 = 8192;
const MAX_CAPTURE_PIXELS: u64 = 33_554_432;

pub(crate) fn capture_task_window_preview(hwnd: String) -> Result<TaskWindowPreviewImage, String> {
    let hwnd = validate_task_window_preview_source(&hwnd)?;

    if unsafe { IsIconic(hwnd).as_bool() } {
        return Err("Preview unavailable for minimized windows".to_string());
    }

    let (width, height, pixels) = capture_window_rgba(hwnd)?;
    let (scaled_width, scaled_height, scaled_pixels) = scale_rgba(width, height, pixels);

    Ok(TaskWindowPreviewImage {
        image_data_url: format!(
            "data:image/png;base64,{}",
            BASE64.encode(encode_png(scaled_width, scaled_height, &scaled_pixels)?)
        ),
        width: scaled_width,
        height: scaled_height,
    })
}

pub(crate) fn validate_task_window_preview_source(hwnd: &str) -> Result<HWND, String> {
    let hwnd = parse_hwnd(hwnd)?;

    if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        return Err("Preview unavailable because the target window no longer exists".to_string());
    }

    if unsafe { IsHungAppWindow(hwnd).as_bool() } {
        return Err("Preview unavailable because the target window is not responding".to_string());
    }

    if !unsafe { IsWindowVisible(hwnd).as_bool() } {
        return Err("Preview unavailable because the target window is hidden".to_string());
    }

    if is_window_cloaked(hwnd) {
        return Err("Preview unavailable because the target window is cloaked".to_string());
    }

    Ok(hwnd)
}

fn capture_window_rgba(hwnd: HWND) -> Result<(u32, u32, Vec<u8>), String> {
    let rect = window_bounds(hwnd)?;
    let width = rect_width(&rect)?;
    let height = rect_height(&rect)?;

    if let Ok(pixels) = capture_window_dc(hwnd, width, height) {
        return Ok((width, height, pixels));
    }

    capture_screen_region(rect, width, height)
}

fn capture_window_dc(hwnd: HWND, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let window_dc = unsafe { GetWindowDC(Some(hwnd)) };
    if window_dc.0.is_null() {
        return Err("Failed to acquire the target window device context".to_string());
    }

    let pixels = capture_into_bitmap(width, height, |memory_dc| {
        unsafe {
            BitBlt(
                memory_dc,
                0,
                0,
                width as i32,
                height as i32,
                Some(window_dc),
                0,
                0,
                capture_raster_operation(),
            )
        }
        .map_err(|error| format!("Failed to capture the target window device context: {error}"))
    });

    unsafe {
        let _ = ReleaseDC(Some(hwnd), window_dc);
    }

    pixels
}

fn capture_screen_region(
    rect: RECT,
    width: u32,
    height: u32,
) -> Result<(u32, u32, Vec<u8>), String> {
    let pixels = capture_screen_region_pixels(rect, width, height)?;

    Ok((width, height, pixels))
}

fn capture_screen_region_pixels(rect: RECT, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let screen_dc = unsafe { GetDC(None) };
    if screen_dc.0.is_null() {
        return Err("Failed to acquire a screen device context for the task preview".to_string());
    }

    let pixels = capture_into_bitmap_with_source(width, height, screen_dc, rect.left, rect.top)
        .map_err(|error| format!("Failed to capture the visible task preview: {error}"));

    unsafe {
        let _ = ReleaseDC(None, screen_dc);
    }

    pixels
}

fn capture_into_bitmap<DrawFn>(width: u32, height: u32, draw: DrawFn) -> Result<Vec<u8>, String>
where
    DrawFn: FnOnce(HDC) -> Result<(), String>,
{
    let screen_dc = unsafe { GetDC(None) };
    if screen_dc.0.is_null() {
        return Err("Failed to acquire a screen device context for the task preview".to_string());
    }

    let result = capture_into_bitmap_with_draw(width, height, screen_dc, draw);

    unsafe {
        let _ = ReleaseDC(None, screen_dc);
    }

    result
}

fn capture_into_bitmap_with_source(
    width: u32,
    height: u32,
    source_dc: HDC,
    source_x: i32,
    source_y: i32,
) -> Result<Vec<u8>, String> {
    capture_into_bitmap_with_draw(width, height, source_dc, |memory_dc| {
        unsafe {
            BitBlt(
                memory_dc,
                0,
                0,
                width as i32,
                height as i32,
                Some(source_dc),
                source_x,
                source_y,
                capture_raster_operation(),
            )
        }
        .map_err(|error| error.to_string())
    })
}

fn capture_into_bitmap_with_draw<DrawFn>(
    width: u32,
    height: u32,
    compatible_dc: HDC,
    draw: DrawFn,
) -> Result<Vec<u8>, String>
where
    DrawFn: FnOnce(HDC) -> Result<(), String>,
{
    let memory_dc = unsafe { CreateCompatibleDC(Some(compatible_dc)) };
    if memory_dc.0.is_null() {
        return Err("Failed to create a memory device context for the task preview".to_string());
    }

    let bitmap = unsafe { CreateCompatibleBitmap(compatible_dc, width as i32, height as i32) };
    if bitmap.0.is_null() {
        unsafe {
            let _ = DeleteDC(memory_dc);
        }
        return Err("Failed to allocate a bitmap for the task preview".to_string());
    }

    let previous = unsafe { SelectObject(memory_dc, bitmap.into()) };
    let pixels = draw(memory_dc).and_then(|()| {
        read_bitmap_rgba(memory_dc, bitmap, width, height).map(|(_, _, pixels)| pixels)
    });

    unsafe {
        if !previous.0.is_null() {
            let _ = SelectObject(memory_dc, previous);
        }
        delete_bitmap(bitmap);
        let _ = DeleteDC(memory_dc);
    }

    pixels
}

fn window_bounds(hwnd: HWND) -> Result<RECT, String> {
    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut rect)
            .map_err(|error| format!("Failed to read the task preview bounds: {error}"))?;
    }

    select_preview_bounds(extended_frame_bounds(hwnd), Some(rect))
}

fn extended_frame_bounds(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    let result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut rect as *mut RECT).cast(),
            size_of::<RECT>() as u32,
        )
    };

    result.ok().filter(|_| rect_has_area(&rect)).map(|_| rect)
}

fn select_preview_bounds(
    extended_frame: Option<RECT>,
    window_rect: Option<RECT>,
) -> Result<RECT, String> {
    for rect in [extended_frame, window_rect].into_iter().flatten() {
        if !rect_has_area(&rect) {
            continue;
        }

        let width = rect_width(&rect)?;
        let height = rect_height(&rect)?;
        if preview_dimensions_are_sane(width, height) {
            return Ok(rect);
        }
    }

    Err("Task preview bounds were empty or too large".to_string())
}

fn rect_has_area(rect: &RECT) -> bool {
    rect.right > rect.left && rect.bottom > rect.top
}

fn rect_width(rect: &RECT) -> Result<u32, String> {
    let width = rect.right - rect.left;
    if width <= 0 {
        return Err("Task preview width was empty".to_string());
    }

    Ok(width as u32)
}

fn rect_height(rect: &RECT) -> Result<u32, String> {
    let height = rect.bottom - rect.top;
    if height <= 0 {
        return Err("Task preview height was empty".to_string());
    }

    Ok(height as u32)
}

fn preview_dimensions_are_sane(width: u32, height: u32) -> bool {
    width > 0
        && height > 0
        && width <= MAX_CAPTURE_DIMENSION
        && height <= MAX_CAPTURE_DIMENSION
        && u64::from(width) * u64::from(height) <= MAX_CAPTURE_PIXELS
}

fn capture_raster_operation() -> ROP_CODE {
    ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0)
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

fn read_bitmap_rgba(
    dc: HDC,
    bitmap: HBITMAP,
    width: u32,
    height: u32,
) -> Result<(u32, u32, Vec<u8>), String> {
    let mut pixels = vec![0_u8; (width * height * 4) as usize];
    let mut bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let scanlines = unsafe {
        GetDIBits(
            dc,
            bitmap,
            0,
            height,
            Some(pixels.as_mut_ptr().cast()),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        )
    };

    if scanlines == 0 {
        return Err("Failed to read the captured task preview pixels".to_string());
    }

    normalize_rgba_pixels_from_gdi(&mut pixels);

    Ok((width, height, pixels))
}

fn normalize_rgba_pixels_from_gdi(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        pixel[3] = u8::MAX;
    }
}

fn scale_rgba(width: u32, height: u32, pixels: Vec<u8>) -> (u32, u32, Vec<u8>) {
    if width <= MAX_PREVIEW_WIDTH && height <= MAX_PREVIEW_HEIGHT {
        return (width, height, pixels);
    }

    let scale = f32::min(
        MAX_PREVIEW_WIDTH as f32 / width as f32,
        MAX_PREVIEW_HEIGHT as f32 / height as f32,
    );
    let scaled_width = ((width as f32 * scale).round() as u32).max(1);
    let scaled_height = ((height as f32 * scale).round() as u32).max(1);
    let mut scaled_pixels = vec![0_u8; (scaled_width * scaled_height * 4) as usize];

    for y in 0..scaled_height {
        let source_y = ((y as f32 / scale).floor() as u32).min(height - 1);
        for x in 0..scaled_width {
            let source_x = ((x as f32 / scale).floor() as u32).min(width - 1);
            let source_index = ((source_y * width + source_x) * 4) as usize;
            let target_index = ((y * scaled_width + x) * 4) as usize;
            scaled_pixels[target_index..target_index + 4]
                .copy_from_slice(&pixels[source_index..source_index + 4]);
        }
    }

    (scaled_width, scaled_height, scaled_pixels)
}

fn encode_png(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>, String> {
    let mut png_bytes = Vec::new();
    let mut encoder = Encoder::new(&mut png_bytes, width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("Failed to start preview PNG encoding: {error}"))?;
    writer
        .write_image_data(pixels)
        .map_err(|error| format!("Failed to encode task preview: {error}"))?;

    drop(writer);
    Ok(png_bytes)
}

unsafe fn delete_bitmap(bitmap: HBITMAP) {
    if !bitmap.0.is_null() {
        let _ = DeleteObject(bitmap.into());
    }
}

#[cfg(test)]
mod preview_tests {
    use super::{
        capture_raster_operation, normalize_rgba_pixels_from_gdi, rect_has_area, rect_height,
        rect_width, select_preview_bounds,
    };
    use windows::Win32::Foundation::RECT;

    #[test]
    fn normalizes_gdi_bgra_pixels_for_png_output() {
        let mut pixels = vec![12, 34, 56, 0, 78, 90, 123, 0];

        normalize_rgba_pixels_from_gdi(&mut pixels);

        assert_eq!(pixels, vec![56, 34, 12, 255, 123, 90, 78, 255]);
    }

    #[test]
    fn validates_non_empty_preview_bounds() {
        let rect = RECT {
            left: 10,
            top: 20,
            right: 210,
            bottom: 140,
        };

        assert!(rect_has_area(&rect));
        assert_eq!(rect_width(&rect).unwrap(), 200);
        assert_eq!(rect_height(&rect).unwrap(), 120);
    }

    #[test]
    fn capture_rop_includes_layered_windows() {
        let rop = capture_raster_operation();

        assert_eq!(rop.0, 13369376 | 1073741824);
    }

    #[test]
    fn preview_bounds_prefers_sane_extended_frame() {
        let extended = RECT {
            left: 10,
            top: 20,
            right: 210,
            bottom: 140,
        };
        let window = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };

        assert_eq!(
            select_preview_bounds(Some(extended), Some(window)).unwrap(),
            extended
        );
    }

    #[test]
    fn preview_bounds_falls_back_when_extended_frame_is_empty() {
        let extended = RECT {
            left: 10,
            top: 20,
            right: 10,
            bottom: 140,
        };
        let window = RECT {
            left: 30,
            top: 40,
            right: 430,
            bottom: 340,
        };

        assert_eq!(
            select_preview_bounds(Some(extended), Some(window)).unwrap(),
            window
        );
    }

    #[test]
    fn preview_bounds_rejects_unsane_whole_virtual_desktop_bounds() {
        let too_large = RECT {
            left: -20000,
            top: -20000,
            right: 20000,
            bottom: 20000,
        };

        assert!(select_preview_bounds(Some(too_large), None).is_err());
    }
}
