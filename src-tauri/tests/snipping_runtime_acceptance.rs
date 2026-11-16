//! Bounded ACTUAL binary-module acceptance runner; never executes shell main.
use std::{process::Command, thread, time::{Duration, Instant}};
const CASES: &[&str] = &[
    "snip_runtime_escape_events_concurrent_cancel_arm_boundary_never_loses_escape",
    "snip_runtime_escape_mailbox_arming_busy_cannot_lose_selection_cancellation",
    "snip_runtime_escape_mailbox_completion_pin_still_wins",
    "snip_runtime_escape_mailbox_full_wake_queue_preserves_latest_generation",
    "snip_runtime_escape_mailbox_delayed_old_generation_cannot_cancel_replacement",
    "snip_runtime_escape_mailbox_shutdown_and_foreign_router_cannot_deliver",
    "snip_runtime_escape_events_held_coordinator_mutex_does_not_block_or_drop_observation",
    "snip_runtime_escape_events_short_press_release_delivers_without_maintenance_tick",
    "snip_runtime_escape_events_delayed_delivery_cannot_cancel_replacement",
    "snip_runtime_escape_events_repeat_keyup_and_other_keys_are_ignored",
    "snip_runtime_escape_events_shutdown_drop_and_runtime_retirement_release_listener",
    "snip_runtime_escape_before_and_during_drag_ignores_webview_focus",
    "snip_runtime_escape_stale_generation_and_capture_id_cannot_cancel_retry",
    "snip_runtime_escape_preparing_and_shutdown_are_unowned",
    "snip_runtime_escape_does_not_dismiss_preview_or_republish",
    "snip_runtime_escape_cannot_interrupt_pinned_completion",
    "snip_runtime_forged_ack_has_zero_composition_and_capture",
    "snip_runtime_exact_ack_once_orders_composition_capture_release",
    "snip_runtime_deadline_rejects_late_ack_and_releases_matching_token",
    "snip_runtime_native_ticket_snapshots_before_worker_and_rejects_late_foreground",
    "snip_runtime_ticket_drop_cross_runtime_and_expiry_release_reservation",
    "snip_runtime_composition_continuity_loss_prevents_freeze",
    "snip_runtime_postcomposition_bar_loss_and_deadline_prevent_freeze",
    "snip_runtime_prepare_emit_failure_and_shutdown_unwind",
    "snip_runtime_restore_requires_generation_capture_focus_and_continuity",
    "snip_runtime_foreground_tracker_reuse_and_identity_fail_closed",
    "snip_runtime_capture_failure_and_drop_leave_no_owned_resources",
    "snip_runtime_own_origin_without_external_candidate_can_reserve",
    "snip_runtime_own_origin_must_not_inherit_previous_external_target",
    "snip_runtime_failed_capture_retry_rejects_old_ack_and_preserves_new_generation",
    "snip_runtime_registered_instance_replacement_and_user_switch_invalidate_origin",
    "snip_runtime_registered_origin_restores_only_without_user_switch_or_replacement",
    "snip_runtime_hotkey_feedback_reports_bounded_failure_and_retry_without_repeat_noise",
    "snip_runtime_renderer_start_does_not_emit_hotkey_feedback",
    "snip_runtime_unregistered_source_never_holds_focus_loss",
    "snip_runtime_expiry_during_create_closes_late_result_without_context_or_arming_then_retries",
    "snip_runtime_native_prepare_topbar_switch_revokes_external_capture_and_restore",
    "snip_runtime_renderer_topbar_admission_preserves_preclick_external_origin",
    "snip_runtime_registered_panel_hold_tracks_helper_generation_phase_and_user_switch",
];
fn cargo(args: &[&str]) -> std::process::Output {
    let path=std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
        .join("Temp/opencode").join(format!("snip-runtime-child-{}.log",std::process::id()));
    let file=std::fs::File::create(&path).unwrap();
    let mut child=Command::new(env!("CARGO")).current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(args).stdout(file.try_clone().unwrap()).stderr(file).spawn().unwrap();
    let deadline=Instant::now()+Duration::from_secs(90);
    let status=loop {
        if let Some(status)=child.try_wait().unwrap() { break status; }
        if Instant::now()>=deadline {
            let _=child.kill(); let _=child.wait(); let _=std::fs::remove_file(&path);
            panic!("bounded runtime acceptance compilation/run timeout");
        }
        thread::sleep(Duration::from_millis(25));
    };
    let bytes=std::fs::read(&path).unwrap(); std::fs::remove_file(path).unwrap();
    std::process::Output { status,stdout:bytes,stderr:Vec::new() }
}
#[test]
fn actual_binary_runtime_acceptance_requires_every_body() {
    let listed=cargo(&["test","--bin","jason-shell","snip_runtime_","--","--list"]);
    assert!(listed.status.success(),"actual module compile failed: {}",String::from_utf8_lossy(&listed.stdout));
    let listing=String::from_utf8_lossy(&listed.stdout);
    for case in CASES {
        assert!(listing.lines().any(|line|line.ends_with(&format!("{case}: test"))),
            "missing actual Runtime acceptance include bridge/case {case}; zero-test GREEN forbidden");
    }
    let run=cargo(&["test","--bin","jason-shell","snip_runtime_","--","--test-threads=1"]);
    assert!(run.status.success(),"actual Runtime acceptance failed: {}",String::from_utf8_lossy(&run.stdout));
    let output=String::from_utf8_lossy(&run.stdout);
    assert!(output.contains(&format!("test result: ok. {} passed; 0 failed; 0 ignored;",CASES.len())),
        "every listed body must execute, none ignored: {output}");
}
