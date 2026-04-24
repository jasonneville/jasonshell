use super::{parse_hwnd, TaskWindowPreviewImage};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use png::{BitDepth, ColorType, Encoder};
use std::mem::size_of;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS,
    HBITMAP, HDC, ROP_CODE, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetWindowRect, IsHungAppWindow, IsIconic};

const MAX_PREVIEW_WIDTH: u32 = 320;
const MAX_PREVIEW_HEIGHT: u32 = 180;

pub(crate) fn capture_task_window_preview(hwnd: String) -> Result<TaskWindowPreviewImage, String> {
    let hwnd = parse_hwnd(&hwnd)?;

    if unsafe { IsHungAppWindow(hwnd).as_bool() } {
        return Err("Preview unavailable because the target window is not responding".to_string());
    }

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

fn capture_window_rgba(
    hwnd: windows::Win32::Foundation::HWND,
) -> Result<(u32, u32, Vec<u8>), String> {
    let rect = window_bounds(hwnd)?;
    let width = rect_width(&rect)?;
    let height = rect_height(&rect)?;
    let screen_dc = unsafe { GetDC(None) };
    if screen_dc.0.is_null() {
        return Err("Failed to acquire a screen device context for the task preview".to_string());
    }

    let memory_dc = unsafe { CreateCompatibleDC(Some(screen_dc)) };
    if memory_dc.0.is_null() {
        unsafe {
            let _ = ReleaseDC(None, screen_dc);
        }
        return Err("Failed to create a memory device context for the task preview".to_string());
    }

    let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width as i32, height as i32) };
    if bitmap.0.is_null() {
        unsafe {
            let _ = DeleteDC(memory_dc);
            let _ = ReleaseDC(None, screen_dc);
        }
        return Err("Failed to allocate a bitmap for the task preview".to_string());
    }

    let previous = unsafe { SelectObject(memory_dc, bitmap.into()) };
    let pixels = match unsafe {
        BitBlt(
            memory_dc,
            0,
            0,
            width as i32,
            height as i32,
            Some(screen_dc),
            rect.left,
            rect.top,
            capture_raster_operation(),
        )
    } {
        Ok(()) => read_bitmap_rgba(memory_dc, bitmap, width, height),
        Err(error) => Err(format!(
            "Failed to capture the visible task preview: {error}"
        )),
    };

    unsafe {
        if !previous.0.is_null() {
            let _ = SelectObject(memory_dc, previous);
        }
        delete_bitmap(bitmap);
        let _ = DeleteDC(memory_dc);
        let _ = ReleaseDC(None, screen_dc);
    }

    pixels
}

fn window_bounds(hwnd: windows::Win32::Foundation::HWND) -> Result<RECT, String> {
    if let Some(rect) = extended_frame_bounds(hwnd) {
        return Ok(rect);
    }

    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut rect)
            .map_err(|error| format!("Failed to read the task preview bounds: {error}"))?;
    }

    if rect_has_area(&rect) {
        return Ok(rect);
    }

    Err("Task preview bounds were empty".to_string())
}

fn extended_frame_bounds(hwnd: windows::Win32::Foundation::HWND) -> Option<RECT> {
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

fn capture_raster_operation() -> ROP_CODE {
    ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0)
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
        rect_width,
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
}
