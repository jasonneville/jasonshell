//! Authoritative logical geometry for the approved Ribbon/Matrix gallery.
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGalleryLayout {
    pub mode: &'static str,
    pub columns: usize,
    pub card_width: f64,
    pub card_height: f64,
    pub thumbnail_width: f64,
    pub thumbnail_height: f64,
    pub footer_height: f64,
    pub viewport_width: f64,
    pub viewport_height: f64,
    pub panel_width: f64,
    pub panel_height: f64,
}

/// Dimensions are CSS/logical pixels; callers scale and round physical edges once.
/// A short viewport scrolls full-size cards rather than shrinking footer controls.
pub fn task_gallery_layout(window_count: usize, available_width: f64, available_height: f64) -> TaskGalleryLayout {
    let width = if available_width.is_finite() { available_width.max(0.0) } else { 0.0 };
    let height = if available_height.is_finite() { available_height.max(0.0) } else { 0.0 };
    let count = window_count.max(1);
    let ribbon_width = 16.0 + count.min(3) as f64 * 180.0 + count.min(3).saturating_sub(1) as f64 * 8.0;
    let ribbon = (2..=3).contains(&count) && ribbon_width <= width && height >= 184.0;
    let gutter = if ribbon { 0.0 } else { 12.0 };
    let columns = if ribbon { count } else { (((width - 20.0).max(0.0) / 188.0).floor() as usize).clamp(1, 3).min(count) };
    let card_width = if columns == 1 { (width - 16.0 - gutter).clamp(32.0, 180.0) } else { 180.0 };
    let viewport_width = columns as f64 * card_width + columns.saturating_sub(1) as f64 * 8.0 + gutter;
    let rows = count.div_ceil(columns).min(2);
    let desired_viewport_height = rows as f64 * 136.0 + rows.saturating_sub(1) as f64 * 8.0;
    let panel_height = (48.0 + desired_viewport_height).min(height);
    TaskGalleryLayout {
        mode: if ribbon { "ribbon" } else { "matrix" }, columns,
        card_width, card_height: 136.0, thumbnail_width: card_width - 8.0,
        thumbnail_height: 96.0, footer_height: 32.0,
        viewport_width: viewport_width.min((width - 16.0).max(0.0)),
        viewport_height: (panel_height - 48.0).max(0.0),
        panel_width: (viewport_width + 16.0).min(width), panel_height,
    }
}
