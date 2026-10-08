//! Pure request/frame admission; no window creation, DWM registration, or shell effects.
#[path = "../src/task_gallery/layout.rs"]
mod layout;
#[path = "../src/task_gallery/thumbnails.rs"]
mod thumbnails;

// The production module contains native methods, but acceptance here calls only
// its pure admission functions. Fail-closed linker seam cannot supply a real HWND.
#[cfg(target_os = "windows")]
mod task_windows {
    pub fn validate_task_window_preview_source(_: &str) -> Result<windows::Win32::Foundation::HWND, String> {
        Err("pure acceptance never authorizes a native source".into())
    }
}
use thumbnails::{admit_request, validate_frames, Bounds, Frame, SyncArgs, MAX_SAFE_INTEGER};

fn request() -> SyncArgs {
    SyncArgs { nonce: "session-A".into(), revision: 7, request_id: 11,
        viewport: Bounds { left: 8.0, top: 40.0, width: 568.0, height: 280.0 },
        frames: vec![Frame { hwnd: "101".into(), bounds: Bounds { left: 12.0, top: 44.0, width: 172.0, height: 96.0 } }] }
}
fn valid(args: &SyncArgs) -> Result<(), String> {
    validate_frames(args, &layout::task_gallery_layout(30, 1920.0, 1000.0), 584.0, 328.0)
}

#[test]
fn exact_session_revision_and_latest_positive_safe_request_are_required() {
    let args = request();
    assert!(admit_request(Some("session-A"), 7, 10, &args).is_ok());
    for (nonce, revision, latest) in [(None, 7, 10), (Some("session-B"), 7, 10),
        (Some("session-A"), 0, 10), (Some("session-A"), 6, 10),
        (Some("session-A"), 8, 10), (Some("session-A"), 7, 11), (Some("session-A"), 7, 12)] {
        assert!(admit_request(nonce, revision, latest, &args).is_err());
    }
    for id in [0, MAX_SAFE_INTEGER + 1, u64::MAX] {
        let mut args = request(); args.request_id = id;
        assert!(admit_request(Some("session-A"), 7, 0, &args).is_err());
    }
    let mut boundary = request(); boundary.request_id = MAX_SAFE_INTEGER;
    assert!(admit_request(Some("session-A"), 7, MAX_SAFE_INTEGER - 1, &boundary).is_ok());
}

#[test]
fn empty_clear_and_exact_fully_contained_frame_are_admitted() {
    let mut args = request(); assert!(valid(&args).is_ok());
    args.frames.clear(); assert!(valid(&args).is_ok());
    args.frames.push(Frame { hwnd: "101".into(), bounds: Bounds { left: 392.0, top: 224.0, width: 172.0, height: 96.0 } });
    assert!(valid(&args).is_ok());
}

#[test]
fn partial_scroll_fragments_header_footer_overflow_and_oversize_frames_are_rejected() {
    for bounds in [
        Bounds { top: 39.0, ..request().frames[0].bounds },
        Bounds { left: 7.0, ..request().frames[0].bounds },
        Bounds { left: 405.0, ..request().frames[0].bounds },
        Bounds { top: 225.0, ..request().frames[0].bounds },
        Bounds { width: 173.0, ..request().frames[0].bounds },
        Bounds { height: 97.0, ..request().frames[0].bounds },
        Bounds { width: 0.0, ..request().frames[0].bounds },
        Bounds { height: -1.0, ..request().frames[0].bounds },
        Bounds { left: f64::NAN, ..request().frames[0].bounds },
        Bounds { top: f64::INFINITY, ..request().frames[0].bounds },
        Bounds { width: f64::MAX, ..request().frames[0].bounds },
    ] {
        let mut args = request(); args.frames[0].bounds = bounds;
        assert!(valid(&args).is_err(), "unsafe bounds accepted: {bounds:?}");
    }
}

#[test]
fn invalid_or_outside_content_viewports_cannot_admit_live_frames() {
    for viewport in [
        Bounds { top: 0.0, ..request().viewport },
        Bounds { left: 0.0, ..request().viewport },
        Bounds { width: 569.0, ..request().viewport },
        Bounds { height: 281.0, ..request().viewport },
        Bounds { width: 0.0, ..request().viewport },
        Bounds { top: f64::NAN, ..request().viewport },
        Bounds { height: f64::INFINITY, ..request().viewport },
    ] {
        let mut args = request(); args.viewport = viewport;
        assert!(valid(&args).is_err());
    }
    assert!(validate_frames(&request(), &layout::task_gallery_layout(30, 1920.0, 1000.0), 580.0, 300.0).is_err());
}

#[test]
fn empty_clear_can_cleanup_collapsed_content_but_never_nonfinite_or_outside_client_bounds() {
    let mut args = request(); args.frames.clear();
    args.viewport = Bounds { left: 0.0, top: 0.0, width: 0.0, height: 0.0 };
    assert!(valid(&args).is_ok());
    for viewport in [Bounds { left: -1.0, ..args.viewport }, Bounds { left: f64::NAN, ..args.viewport },
        Bounds { width: f64::INFINITY, ..args.viewport }, Bounds { width: 585.0, ..args.viewport },
        Bounds { top: 329.0, ..args.viewport }] {
        args.viewport = viewport; assert!(valid(&args).is_err());
    }
}

#[test]
fn scrollbar_gutter_is_never_a_native_thumbnail_destination() {
    let mut args = request();
    args.frames[0].bounds = Bounds { left: 393.0, top: 224.0, width: 172.0, height: 96.0 };
    assert!(valid(&args).is_err());
}

#[test]
fn duplicate_overlapping_and_over_budget_sets_fail_closed() {
    let mut args = request(); args.frames.push(args.frames[0].clone());
    assert!(valid(&args).is_err());
    args.frames[1].hwnd = "102".into(); assert!(valid(&args).is_err());
    args.frames[1].bounds.left = 184.0; assert!(valid(&args).is_ok(), "edge-touch is not overlap");
    // Tiny non-overlapping rectangles ensure the count limit, not overlap, rejects.
    args.frames = (0..13).map(|index| Frame { hwnd: (101 + index).to_string(),
        bounds: Bounds { left: 8.0 + index as f64 * 2.0, top: 40.0, width: 1.0, height: 1.0 } }).collect();
    assert!(valid(&args).is_err()); args.frames.pop(); assert!(valid(&args).is_ok());
}

#[test]
fn ipc_rejects_unknown_fields_in_frame_bounds_and_request() {
    for json in [
        r#"{"nonce":"A","revision":1,"requestId":1,"viewport":{"left":8,"top":40,"width":1,"height":1},"frames":[],"destinationHwnd":"forged"}"#,
        r#"{"nonce":"A","revision":1,"requestId":1,"viewport":{"left":8,"top":40,"width":1,"height":1,"scale":2},"frames":[]}"#,
        r#"{"nonce":"A","revision":1,"requestId":1,"viewport":{"left":8,"top":40,"width":1,"height":1},"frames":[{"hwnd":"101","bounds":{"left":8,"top":40,"width":1,"height":1},"pid":99}]}"#,
    ] { assert!(serde_json::from_str::<SyncArgs>(json).is_err()); }
}
