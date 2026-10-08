//! Pure native geometry acceptance. No shell launch, AppBar, HWND, or DWM calls.
#[path = "../src/task_gallery/layout.rs"]
mod layout;

use layout::task_gallery_layout;
use serde_json::{to_value, Value};

fn geometry(count: usize, width: f64, height: f64) -> Value {
    to_value(task_gallery_layout(count, width, height)).expect("native layout must serialize to IPC")
}
fn number(value: &Value, name: &str) -> f64 {
    value[name].as_f64().unwrap_or_else(|| panic!("missing numeric IPC field {name}: {value}"))
}

#[test]
fn ribbon_thresholds_have_image_first_geometry_independent_of_bar_height() {
    for (count, width) in [(2, 384.0), (3, 572.0)] {
        let result = geometry(count, 1920.0, 1000.0);
        assert_eq!(result["mode"], "ribbon");
        assert_eq!(number(&result, "columns"), count as f64);
        assert_eq!(number(&result, "panelWidth"), width);
        assert_eq!(number(&result, "panelHeight"), 184.0);
        assert_eq!(number(&result, "cardWidth"), 180.0);
        assert_eq!(number(&result, "cardHeight"), 136.0);
        assert_eq!(number(&result, "thumbnailWidth"), 172.0);
        assert_eq!(number(&result, "thumbnailHeight"), 96.0);
        assert_eq!(number(&result, "footerHeight"), 32.0);
    }
}

#[test]
fn matrix_thresholds_bound_columns_and_visible_rows_not_member_count() {
    for count in [4, 6, 30, 1000] {
        let result = geometry(count, 1920.0, 1000.0);
        assert_eq!(result["mode"], "matrix");
        assert_eq!(number(&result, "columns"), 3.0);
        assert_eq!(number(&result, "panelWidth"), 584.0);
        assert_eq!(number(&result, "panelHeight"), 328.0);
        assert_eq!(number(&result, "viewportHeight"), 280.0);
        assert_eq!(number(&result, "viewportWidth"), 568.0);
    }
}

#[test]
fn small_groups_wrap_when_ribbon_cannot_fit_and_reserve_scrollbar() {
    for (count, available, columns, width, height) in [
        (2, 383.0, 1.0, 208.0, 328.0),
        (3, 571.0, 2.0, 396.0, 328.0),
        (6, 396.0, 2.0, 396.0, 328.0),
        (6, 395.0, 1.0, 208.0, 328.0),
        (30, 208.0, 1.0, 208.0, 328.0),
    ] {
        let result = geometry(count, available, 1000.0);
        assert_eq!(result["mode"], "matrix");
        assert_eq!(number(&result, "columns"), columns);
        assert_eq!(number(&result, "panelWidth"), width);
        assert_eq!(number(&result, "panelHeight"), height);
    }
}

#[test]
fn undersized_monitor_reduces_one_card_and_clamps_viewport_height() {
    let result = geometry(30, 144.0, 220.0);
    assert_eq!(result["mode"], "matrix");
    assert_eq!(number(&result, "columns"), 1.0);
    assert_eq!(number(&result, "panelWidth"), 144.0);
    assert_eq!(number(&result, "cardWidth"), 116.0);
    assert_eq!(number(&result, "thumbnailWidth"), 108.0);
    assert_eq!(number(&result, "panelHeight"), 220.0);
    assert_eq!(number(&result, "viewportHeight"), 172.0);
}

#[test]
fn logical_geometry_does_not_depend_on_dpi_and_all_contract_dimensions_are_finite() {
    for count in [2, 3, 4, 6, 30] {
        // Native caller converts physical monitor space to logical before this seam.
        let logical = geometry(count, 900.0, 600.0);
        for scale in [1.0, 1.25, 1.5, 2.0] {
            assert_eq!(logical, geometry(count, 900.0 * scale / scale, 600.0 * scale / scale));
        }
        for field in ["columns", "cardWidth", "cardHeight", "thumbnailWidth", "thumbnailHeight",
            "footerHeight", "viewportWidth", "viewportHeight", "panelWidth", "panelHeight"] {
            assert!(number(&logical, field).is_finite());
            assert!(number(&logical, field) > 0.0);
        }
        assert!(number(&logical, "panelWidth") <= 900.0);
        assert!(number(&logical, "panelHeight") <= 600.0);
    }
}
