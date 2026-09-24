//! RED-ready acceptance contract for the approved local TDT fallback coordinator.
//!
//! The crate is currently binary-only, so this harness uses deterministic
//! source/contract checks instead of importing private module symbols. These
//! tests are intentionally test-only acceptance coverage for safety regressions
//! found by independent review.

use std::fs;
use std::path::PathBuf;

const EXPECTED_SEGMENT_SAMPLES: &str = "2_560";
const EXPECTED_QUEUE_CAPACITY: &str = "32";
const EXPECTED_RECORDING_CAP_SECS: &str = "300";
const EXPECTED_FINALIZATION_TIMEOUT_SECS: &str = "45";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn streaming_source() -> String {
    let path = manifest_dir().join("src/speech_streaming.rs");
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "planned local TDT fallback module is missing or unreadable at {}: {error}",
            path.display()
        )
    })
}

fn speech_source() -> String {
    fs::read_to_string(manifest_dir().join("src/speech.rs")).expect("speech.rs should be readable")
}

fn runtime_source() -> String {
    fs::read_to_string(manifest_dir().join("src/speech_runtime.rs"))
        .expect("speech_runtime.rs should be readable")
}

fn function_body<'a>(source: &'a str, name: &str) -> &'a str {
    let needle = format!("fn {name}");
    let fn_start = source
        .find(&needle)
        .unwrap_or_else(|| panic!("function `{name}` should exist"));
    let open = source[fn_start..]
        .find('{')
        .map(|index| fn_start + index)
        .unwrap_or_else(|| panic!("function `{name}` should have a body"));
    let mut depth = 0usize;
    for (offset, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[open + 1..open + offset];
                }
            }
            _ => {}
        }
    }
    panic!("function `{name}` body should close")
}

fn index_of(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("expected source to contain `{needle}`"))
}

#[test]
fn tdt_fallback_constants_are_bounded_and_not_true_streaming_claims() {
    let source = streaming_source();

    assert!(
        source.contains(EXPECTED_SEGMENT_SAMPLES),
        "segment size must be 2,560 mono-16k samples"
    );
    assert!(
        source.contains(EXPECTED_QUEUE_CAPACITY),
        "queue capacity must be fixed at 32 segments"
    );
    assert!(
        source.contains(EXPECTED_RECORDING_CAP_SECS),
        "recording cap must be 300 seconds"
    );
    assert!(
        source.contains(EXPECTED_FINALIZATION_TIMEOUT_SECS),
        "post-close finalization timeout must be 45 seconds"
    );
    assert!(!source
        .to_ascii_lowercase()
        .contains("true stateful streaming"));
}

#[test]
fn intake_deadline_is_activation_anchored_and_rejects_at_or_after_cap() {
    let source = streaming_source();

    assert!(
        source.contains("activation_deadline"),
        "cap deadline must derive from successful stream.play activation"
    );
    assert!(
        source.contains("Instant::now") || source.contains("now()"),
        "intake must compare current time before accepting input"
    );
    assert!(
        source.contains(">=") && source.contains("RecordingCap"),
        "input at or after 300s must seal/reject as cap-driven close"
    );
    assert!(
        source.contains("Rejected")
            || source.contains("QueueFull")
            || source.contains("SafeFailure"),
        "post-deadline input must be rejected explicitly"
    );
}

#[test]
fn first_close_wins_and_late_worker_result_cannot_commit() {
    let source = streaming_source();

    assert!(
        source.contains("first_close")
            || source.contains("compare_exchange")
            || source.contains("input_closed"),
        "Finish/cap race must be idempotent with first close winning"
    );
    assert!(
        source.contains("generation") || source.contains("nonce"),
        "late worker results must be authorized by nonce/generation"
    );
    assert!(
        source.contains("can_commit")
            || source.contains("authorize")
            || source.contains("StateRace"),
        "old worker completion must be rejected before clipboard/history commit"
    );
}

#[test]
fn queue_full_is_safe_failure_not_silent_drop_or_callback_block() {
    let source = streaming_source();

    assert!(
        source.contains("try_send") || source.contains("try_lock"),
        "audio callback path must be nonblocking"
    );
    assert!(
        source.contains("QueueFull")
            || source.contains("queue-full")
            || source.contains("backpressure"),
        "bounded queue overflow must surface explicit safe failure"
    );
    assert!(
        !source.contains("blocking_send"),
        "callback must not block on bounded queue"
    );
    assert!(
        !source.contains("send_timeout"),
        "callback must not wait for worker capacity"
    );
}

#[test]
fn public_speech_event_supports_cap_only_finalization_reason() {
    let speech = speech_source();

    assert!(
        speech.contains("FinalizationReason"),
        "speech event contract must define finalization reason enum"
    );
    assert!(
        speech.contains("RecordingCap"),
        "recording cap must be representable on transcribing event"
    );
    assert!(
        speech.contains("skip_serializing_if = \"Option::is_none\""),
        "legacy events must omit optional field"
    );
    assert!(
        !speech.contains("UserFinish"),
        "public optional field is only required for recording_cap, not manual Finish"
    );
}

#[test]
fn queue_full_close_uses_out_of_band_worker_wake_not_full_work_queue() {
    let source = streaming_source();
    let first_close = function_body(&source, "first_close");

    assert!(
        source.contains("close_notify")
            || source.contains("close_notifier")
            || source.contains("close_signal")
            || source.contains("wake_close")
            || source.contains("abort_handle")
            || source.contains("drop_sender")
            || source.contains("Receiver::recv_timeout"),
        "queue-full safe failure must wake/terminate the worker through a dedicated out-of-band close/shutdown signal, not only the bounded segment queue"
    );
    assert!(
        !first_close.contains("try_send(WorkerMessage::Close"),
        "first_close must not rely solely on try_send(Close): a full queue can drop Close while the worker-retained sender keeps recv() alive"
    );
}

#[test]
fn publish_path_authorizes_before_any_history_insert_or_clipboard_attempt() {
    let source = runtime_source();
    let publish = function_body(&source, "publish_recorded_if_current");

    let insert = index_of(publish, "insert_history_attempt");
    let commit_lock = index_of(publish, "state.commit.lock");
    let shutdown_guard = index_of(publish, "state.shutting_down.load");
    let generation_guard = index_of(publish, "state.generation.load");
    let can_complete_guard = index_of(publish, ".can_complete");
    let publish_call = index_of(publish, "publish()");

    assert!(
        commit_lock < insert
            && shutdown_guard < insert
            && generation_guard < insert
            && can_complete_guard < insert,
        "stale/timeout/shutdown authorization must occur before insert_history_attempt so rejected completions leave no history"
    );
    assert!(
        insert < publish_call,
        "authorized history attempt may be recorded before clipboard publish, but only after stale/timeout/shutdown guards"
    );
}

#[test]
fn record_clipboard_attempt_is_not_used_to_bypass_publish_authorization() {
    let source = runtime_source();
    let publish = function_body(&source, "publish_recorded_if_current");

    assert!(
        !publish.contains("record_clipboard_attempt"),
        "publication path must not use eager record_clipboard_attempt before nonce/generation/shutdown/deadline authorization"
    );
}

#[test]
fn retained_history_copy_releases_history_lock_before_clipboard_publish() {
    let source = runtime_source();
    let copy = function_body(&source, "copy_retained_history_transcript");

    let history_lock = index_of(copy, "state.history.lock");
    let publish_call = index_of(copy, "publish(");
    let transcript_clone = copy.find("transcript.clone()").unwrap_or_else(|| {
        panic!(
            "copy_retained_history_transcript must clone transcript while holding history.lock(), then publish the owned transcript after the guard scope ends; publishing a borrowed entry transcript keeps the mutex held"
        )
    });
    let guard_scope_end = copy
        .find("};\n    publish")
        .or_else(|| copy.find("}\n    publish"))
        .unwrap_or_else(|| {
            panic!(
                "copy_retained_history_transcript must close the history.lock() guard scope before clipboard publish; current lock-order can stall finalization/shutdown"
            )
        });

    assert!(
        history_lock < transcript_clone
            && transcript_clone < guard_scope_end
            && guard_scope_end < publish_call,
        "copy_retained_history_transcript must clone transcript inside the history lock, drop the guard, then call publish outside the mutex"
    );
}

#[test]
fn commit_authorization_requires_terminal_close_reason_and_matching_generation() {
    let source = streaming_source();
    let can_commit = function_body(&source, "can_commit");

    assert!(
        can_commit.contains("nonce") && can_commit.contains("generation"),
        "commit authorization must keep immutable nonce/current-generation checks"
    );
    assert!(
        can_commit.contains("reason()")
            || can_commit.contains("close_reason")
            || can_commit.contains("InputCloseReason"),
        "commit authorization must inspect the winning input close reason; nonce/generation alone lets queue-full SafeFailure drain and publish"
    );
    assert!(
        can_commit.contains("InputCloseReason::Finish")
            && can_commit.contains("InputCloseReason::RecordingCap"),
        "only user Finish or RecordingCap may authorize history/clipboard commit"
    );
    assert!(
        can_commit.contains("matches!") || can_commit.contains("match"),
        "SafeFailure/Cancel/Shutdown must fail closed even when queued audio later drains"
    );
    assert!(
        !can_commit.contains("InputCloseReason::SafeFailure => true")
            && !can_commit.contains("InputCloseReason::Cancel => true")
            && !can_commit.contains("InputCloseReason::Shutdown => true"),
        "non-terminal/safety close reasons must never authorize commit"
    );
}

#[test]
fn cap_tail_handoff_trims_to_activation_deadline_before_queue_send() {
    let source = runtime_source();
    let handoff_tail = function_body(&source, "handoff_tail");

    let trim = handoff_tail
        .find("trim_to_deadline")
        .or_else(|| handoff_tail.find("samples_before_deadline"))
        .or_else(|| handoff_tail.find("activation_deadline"))
        .unwrap_or_else(|| {
            panic!(
                "cap tail handoff must compute deadline-aware accepted prefix before queueing tail"
            )
        });
    let queue_send = handoff_tail
        .find("try_send_tail")
        .or_else(|| handoff_tail.find("try_send"))
        .unwrap_or_else(|| {
            panic!("tail handoff should queue through bounded intake after trimming")
        });

    assert!(
        trim < queue_send,
        "cap-tail path must trim post-cap suffix before any queue send admits tail audio"
    );
    assert!(
        handoff_tail.contains("RecordingCap")
            || handoff_tail.contains("activation_deadline")
            || handoff_tail.contains("deadline"),
        "tail handoff must be cap/deadline aware, not a blind full-partial enqueue after 300s"
    );
}

#[test]
fn model_is_loaded_or_preloaded_before_recording_is_accepted() {
    let source = runtime_source();
    let start = function_body(&source, "start_speech_capture");

    let recording_accept = index_of(start, "StartSpeechCaptureResponse");
    let controller_start = index_of(start, "controller.start");
    let stream_play = index_of(start, "stream.play");
    let model_ready = start
        .find("preload_parakeet_tdt")
        .or_else(|| start.find("load_parakeet_tdt"))
        .or_else(|| start.find("SpeechModelReady"))
        .unwrap_or_else(|| {
            panic!("start_speech_capture must prove model readiness before recording acceptance")
        });

    assert!(
        model_ready < controller_start && model_ready < stream_play && model_ready < recording_accept,
        "model readiness/preload must complete before controller enters Recording, stream starts, or StartSpeechCaptureResponse is returned"
    );
}

#[test]
fn intake_is_installed_before_stream_play_can_fire_callbacks() {
    let source = runtime_source();
    let start = function_body(&source, "start_speech_capture");

    let stream_play = index_of(start, "stream.play");
    let intake_install = index_of(start, "intake_slot.set");

    assert!(
        intake_install < stream_play,
        "capture callback can run immediately after stream.play(); intake_slot must be installed first so early callbacks are not counted as loss"
    );
}

#[test]
fn deadline_crossing_callback_keeps_pre_cap_prefix_and_refuses_post_cap_suffix() {
    let source = streaming_source();
    let intake = function_body(&source, "try_send");

    assert!(
        source.contains("split_at")
            || source.contains("pre_cap")
            || source.contains("samples_before_deadline")
            || source.contains("accepted_prefix")
            || source.contains("trim_to_deadline"),
        "deadline-crossing audio block must split/trim: keep samples before activation deadline and refuse only post-cap samples"
    );
    assert!(
        !intake.contains("if now >= self.activation_deadline")
            || intake.contains("pre_cap")
            || intake.contains("samples_before_deadline")
            || intake.contains("accepted_prefix"),
        "whole-block rejection at/after deadline is insufficient; tests require bounded pre-cap prefix preservation"
    );
}
