//! Full-resolution desktop capture with scoped GDI ownership, never disk-backed.
use super::geometry::{checked_monitor_len, PhysicalRect, MAX_FROZEN_BYTES, MAX_STAGING_BYTES};
use std::{mem::size_of, ptr};
use windows::Win32::Graphics::Gdi::*;

#[derive(Debug)]
pub struct FrozenMonitor { pub rect: PhysicalRect, pub pixels: Vec<u8> }

struct ScreenDc(HDC);
impl Drop for ScreenDc { fn drop(&mut self) { unsafe { let _ = ReleaseDC(None, self.0); } } }
struct MemoryDc(HDC);
impl Drop for MemoryDc { fn drop(&mut self) { unsafe { let _ = DeleteDC(self.0); } } }
struct Bitmap(HBITMAP);
impl Drop for Bitmap { fn drop(&mut self) { unsafe { let _ = DeleteObject(self.0.into()); } } }
struct Selection { dc: HDC, previous: HGDIOBJ }
impl Drop for Selection { fn drop(&mut self) { unsafe { let _ = SelectObject(self.dc, self.previous); } } }

/// Capture every supplied native monitor or fail; never substitute primary-only output.
/// Caller must hide/exclude all prior snip windows and flush composition first.
pub fn freeze_monitors(rects: &[PhysicalRect]) -> Result<Vec<FrozenMonitor>, &'static str> {
    if rects.is_empty() || rects.len() > 32 { return Err("invalid monitor count"); }
    let mut total = 0usize;
    let mut largest = 0usize;
    for rect in rects {
        let len = checked_monitor_len(*rect, MAX_FROZEN_BYTES).map_err(|_| "monitor capture budget")?;
        total = total.checked_add(len).ok_or("capture overflow")?;
        largest = largest.max(len);
    }
    // Frozen Rust buffers plus one live DIB. No PNG/base64 staging is allocated here.
    if total > MAX_FROZEN_BYTES || total.checked_add(largest).ok_or("capture overflow")? > MAX_STAGING_BYTES {
        return Err("aggregate capture budget");
    }
    let mut output = Vec::new();
    output.try_reserve_exact(rects.len()).map_err(|_| "capture allocation")?;
    for rect in rects { output.push(capture(*rect)?); }
    Ok(output)
}

fn capture(rect: PhysicalRect) -> Result<FrozenMonitor, &'static str> {
    let len = checked_monitor_len(rect, MAX_FROZEN_BYTES).map_err(|_| "monitor capture budget")?;
    let (width, height) = rect.dimensions().map_err(|_| "invalid monitor bounds")?;
    // SAFETY: every handle is checked before use and dropped in reverse creation order;
    // selection is restored before deleting the bitmap or memory DC.
    unsafe {
        let screen = ScreenDc(GetDC(None));
        if screen.0.0.is_null() { return Err("screen dc unavailable"); }
        let memory = MemoryDc(CreateCompatibleDC(Some(screen.0)));
        if memory.0.0.is_null() { return Err("memory dc unavailable"); }
        let info = BITMAPINFO { bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: width as i32,
            biHeight: -(height as i32), biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0,
            ..Default::default()
        }, ..Default::default() };
        let mut bits = ptr::null_mut();
        let bitmap = Bitmap(CreateDIBSection(Some(screen.0), &info, DIB_RGB_COLORS, &mut bits, None, 0)
            .map_err(|_| "dib allocation failed")?);
        if bitmap.0.0.is_null() || bits.is_null() { return Err("dib allocation failed"); }
        let previous = SelectObject(memory.0, bitmap.0.into());
        if previous.0.is_null() || previous.0 as isize == -1 { return Err("bitmap selection failed"); }
        let _selection = Selection { dc: memory.0, previous };
        BitBlt(memory.0, 0, 0, width as i32, height as i32, Some(screen.0), rect.left, rect.top,
            ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0)).map_err(|_| "desktop capture failed")?;
        if !GdiFlush().as_bool() { return Err("gdi flush failed"); }
        let bgra = std::slice::from_raw_parts(bits.cast::<u8>(), len);
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(len).map_err(|_| "capture allocation failed")?;
        for pixel in bgra.chunks_exact(4) { pixels.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]); }
        Ok(FrozenMonitor { rect, pixels })
    }
}
