//! Pure acceptance tests: no desktop capture, DPI mutation, or clipboard side effects.
#[path = "../src/snipping/geometry.rs"]
mod geometry;

use geometry::{checked_rgba_len, crop_rgba, physical_rect_from_logical, LogicalRect, PhysicalRect};

const LIMIT: usize = 128 * 1024 * 1024;

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PhysicalRect {
    PhysicalRect { left, top, right, bottom }
}

#[test]
fn snipping_negative_origin_crop_has_exact_rows_and_no_scaling() {
    let source = rect(-4, -2, 0, 1);
    let bytes: Vec<u8> = (0..12).flat_map(|pixel| [pixel, 50, 100, 255]).collect();
    let original = bytes.clone();
    let crop = crop_rgba(source, rect(-3, -1, -1, 1), &bytes, LIMIT).unwrap();
    assert_eq!((crop.width, crop.height), (2, 2));
    assert_eq!(crop.pixels, [5, 50, 100, 255, 6, 50, 100, 255, 9, 50, 100, 255, 10, 50, 100, 255]);
    assert_eq!(bytes, original);
}

#[test]
fn snipping_mixed_dpi_is_monitor_local_then_translated_once() {
    let logical = LogicalRect { x: 10.25, y: 20.5, width: 100.5, height: 40.25 };
    assert_eq!(physical_rect_from_logical(rect(-1920, -300, 0, 780), 1.0, logical).unwrap(),
        rect(-1910, -280, -1809, -239));
    assert_eq!(physical_rect_from_logical(rect(0, 0, 2560, 1440), 1.5, logical).unwrap(),
        rect(15, 30, 167, 92));
}

#[test]
fn snipping_allocation_is_checked_before_allocation() {
    assert_eq!(checked_rgba_len(1, 1, 4).unwrap(), 4);
    assert_eq!(checked_rgba_len(2, 3, 24).unwrap(), 24);
    for (width, height, limit) in [(0, 1, LIMIT), (1, 0, LIMIT), (2, 3, 23), (u32::MAX, u32::MAX, usize::MAX)] {
        assert!(checked_rgba_len(width, height, limit).is_err(), "{width}x{height} limit {limit}");
    }
}

#[test]
fn snipping_invalid_overflow_outside_and_truncated_rectangles_fail_closed() {
    let source = rect(-4, -2, 0, 1);
    let bytes = vec![255; 48];
    for selection in [rect(-3, -1, -3, 1), rect(-1, -1, -3, 1), rect(-3, 1, -1, -1),
        rect(-5, -1, -1, 1), rect(-3, -1, 1, 1), rect(i32::MIN, 0, i32::MAX, 1)] {
        assert!(crop_rgba(source, selection, &bytes, LIMIT).is_err());
    }
    assert!(crop_rgba(source, source, &bytes[..47], LIMIT).is_err());
    assert!(crop_rgba(source, source, &bytes, 47).is_err());
    assert!(crop_rgba(rect(i32::MIN, 0, i32::MAX, 1), source, &[], LIMIT).is_err());
    assert!(crop_rgba(rect(0, 0, 0, 1), source, &[], LIMIT).is_err());
}

#[test]
fn snipping_logical_conversion_rejects_nonfinite_empty_outside_and_overflow() {
    let monitor = rect(-1920, -300, 0, 780);
    let valid = LogicalRect { x: 10.0, y: 20.0, width: 100.0, height: 40.0 };
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(physical_rect_from_logical(monitor, scale, valid).is_err());
    }
    for logical in [LogicalRect { x: f64::NAN, ..valid }, LogicalRect { y: f64::INFINITY, ..valid },
        LogicalRect { width: 0.0, ..valid }, LogicalRect { height: -1.0, ..valid },
        LogicalRect { x: -1.0, ..valid }, LogicalRect { width: f64::MAX, ..valid },
        LogicalRect { x: 1900.0, ..valid }] {
        assert!(physical_rect_from_logical(monitor, 1.0, logical).is_err());
    }
    assert!(physical_rect_from_logical(rect(0, 0, 0, 1), 1.0, valid).is_err());
}
