//! Checked half-open physical geometry; no display discovery or native side effects.

pub const MAX_DIMENSION: u32 = 16_384;
pub const MAX_MONITOR_PIXELS: u64 = 33_600_000;
pub const MAX_FROZEN_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_STAGING_BYTES: usize = 512 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryError { InvalidRectangle, InvalidScale, Overflow, Budget, BufferLength, Allocation }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalRect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }

#[derive(Clone, Copy, Debug)]
pub struct LogicalRect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }

#[derive(Debug)]
pub struct CroppedImage { pub width: u32, pub height: u32, pub pixels: Vec<u8> }

impl PhysicalRect {
    pub fn dimensions(self) -> Result<(u32, u32), GeometryError> {
        let width = self.right.checked_sub(self.left).ok_or(GeometryError::Overflow)?;
        let height = self.bottom.checked_sub(self.top).ok_or(GeometryError::Overflow)?;
        if width <= 0 || height <= 0 { return Err(GeometryError::InvalidRectangle); }
        Ok((width as u32, height as u32))
    }
    fn contains(self, other: Self) -> bool {
        other.left >= self.left && other.top >= self.top && other.right <= self.right && other.bottom <= self.bottom
    }
}

pub fn checked_rgba_len(width: u32, height: u32, max_bytes: usize) -> Result<usize, GeometryError> {
    if width == 0 || height == 0 { return Err(GeometryError::InvalidRectangle); }
    let bytes = (width as usize).checked_mul(height as usize).and_then(|n| n.checked_mul(4)).ok_or(GeometryError::Overflow)?;
    if bytes > max_bytes { return Err(GeometryError::Budget); }
    Ok(bytes)
}

/// Product limits supplement the caller's stage budget, before any native allocation.
pub fn checked_monitor_len(rect: PhysicalRect, max_bytes: usize) -> Result<usize, GeometryError> {
    let (width, height) = rect.dimensions()?;
    if width > MAX_DIMENSION || height > MAX_DIMENSION || u64::from(width) * u64::from(height) > MAX_MONITOR_PIXELS {
        return Err(GeometryError::Budget);
    }
    checked_rgba_len(width, height, max_bytes.min(MAX_FROZEN_BYTES))
}

pub fn physical_rect_from_logical(monitor: PhysicalRect, scale: f64, local: LogicalRect) -> Result<PhysicalRect, GeometryError> {
    let (width, height) = monitor.dimensions()?;
    if !scale.is_finite() || scale <= 0.0 { return Err(GeometryError::InvalidScale); }
    if ![local.x, local.y, local.width, local.height].iter().all(|v| v.is_finite())
        || local.x < 0.0 || local.y < 0.0 || local.width <= 0.0 || local.height <= 0.0 {
        return Err(GeometryError::InvalidRectangle);
    }
    let edges = [(local.x * scale).floor(), (local.y * scale).floor(),
        ((local.x + local.width) * scale).ceil(), ((local.y + local.height) * scale).ceil()];
    if !edges.iter().all(|v| v.is_finite() && *v >= 0.0 && *v <= i32::MAX as f64)
        || edges[2] > f64::from(width) || edges[3] > f64::from(height) { return Err(GeometryError::InvalidRectangle); }
    let result = PhysicalRect {
        left: monitor.left.checked_add(edges[0] as i32).ok_or(GeometryError::Overflow)?,
        top: monitor.top.checked_add(edges[1] as i32).ok_or(GeometryError::Overflow)?,
        right: monitor.left.checked_add(edges[2] as i32).ok_or(GeometryError::Overflow)?,
        bottom: monitor.top.checked_add(edges[3] as i32).ok_or(GeometryError::Overflow)?,
    };
    result.dimensions()?;
    Ok(result)
}

pub fn crop_rgba(source: PhysicalRect, crop: PhysicalRect, pixels: &[u8], max_bytes: usize) -> Result<CroppedImage, GeometryError> {
    let (source_width, source_height) = source.dimensions()?;
    let source_len = checked_rgba_len(source_width, source_height, max_bytes)?;
    let (width, height) = crop.dimensions()?;
    if !source.contains(crop) { return Err(GeometryError::InvalidRectangle); }
    if pixels.len() != source_len { return Err(GeometryError::BufferLength); }
    let output_len = checked_rgba_len(width, height, max_bytes)?;
    // This accounts for borrowed frozen input plus owned crop, not merely the output.
    if source_len.checked_add(output_len).ok_or(GeometryError::Overflow)? > MAX_STAGING_BYTES { return Err(GeometryError::Budget); }
    let stride = source_width as usize * 4;
    let row_len = width as usize * 4;
    let x = (i64::from(crop.left) - i64::from(source.left)) as usize;
    let y = (i64::from(crop.top) - i64::from(source.top)) as usize;
    let mut output = Vec::new();
    output.try_reserve_exact(output_len).map_err(|_| GeometryError::Allocation)?;
    for row in y..y + height as usize {
        let offset = row * stride + x * 4;
        output.extend_from_slice(pixels.get(offset..offset + row_len).ok_or(GeometryError::BufferLength)?);
    }
    Ok(CroppedImage { width, height, pixels: output })
}
