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
    let source = fs::read_to_string(manifest_dir().join("src/speech_runtime.rs"))
        .expect("speech_runtime.rs should be readable");
    let emit_body = function_body(&source, "emit");
    assert!(
        emit_body.contains("app.emit(contracts::events::SPEECH_STATUS_CHANGED, event)"),
        "shared speech emit helper must publish the canonical status event"
    );
    source
}

fn main_source() -> String {
    fs::read_to_string(manifest_dir().join("src/main.rs")).expect("main.rs should be readable")
}

fn const_usize(source: &str, name: &str) -> usize {
    let needle = format!("const {name}: usize =");
    let start = source
        .find(&needle)
        .unwrap_or_else(|| panic!("constant `{name}` should exist"));
    let value_start = start + needle.len();
    let value_end = source[value_start..]
        .find(';')
        .map(|index| value_start + index)
        .unwrap_or_else(|| panic!("constant `{name}` must end with semicolon"));
    source[value_start..value_end]
        .trim()
        .replace('_', "")
        .parse::<usize>()
        .unwrap_or_else(|error| panic!("constant `{name}` should be usize literal: {error}"))
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

fn terminal_status_emit(section: &str) -> Option<usize> {
    [
        "app.emit(contracts::events::SPEECH_STATUS_CHANGED",
        "emit(contracts::events::SPEECH_STATUS_CHANGED",
        "emit(&app, &event)",
        "emit(app, &event)",
    ]
    .iter()
    .filter_map(|needle| section.find(needle))
    .min()
}

fn block_starting_at<'a>(source: &'a str, marker: &str) -> &'a str {
    let marker_start = source
        .find(marker)
        .unwrap_or_else(|| panic!("expected source to contain block marker `{marker}`"));
    let open = source[marker_start..]
        .find('{')
        .map(|index| marker_start + index)
        .unwrap_or_else(|| panic!("block marker `{marker}` should open a scope"));
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
    panic!("block marker `{marker}` scope should close")
}

fn assert_deferred_terminal_emit_has_lifecycle_gate(section: &str, label: &str) {
    let commit_guard = section.find("state.commit.lock").unwrap_or_else(|| {
        panic!(
            "{label} must acquire state.commit in the same deferred terminal emit scope; computing an event under the gate and emitting after scope release lets shutdown publish stale terminal events"
        )
    });
    let shutdown_recheck = section[commit_guard..]
        .find("state.shutting_down.load")
        .map(|index| commit_guard + index)
        .unwrap_or_else(|| {
            panic!(
                "{label} must recheck shutting_down after acquiring state.commit and before terminal event emit"
            )
        });
    let generation_recheck = section[commit_guard..]
        .find("state.generation.load")
        .map(|index| commit_guard + index)
        .unwrap_or_else(|| {
            panic!(
                "{label} must recheck generation after acquiring state.commit and before terminal event emit"
            )
        });
    let terminal_emit = terminal_status_emit(section)
        .unwrap_or_else(|| panic!("{label} must emit a terminal SPEECH_STATUS_CHANGED event"));
    let drop_commit_before_emit = section[commit_guard..terminal_emit]
        .find("drop(_commit)")
        .or_else(|| section[commit_guard..terminal_emit].find("drop(commit)"))
        .map(|index| commit_guard + index);

    assert!(
        commit_guard < shutdown_recheck
            && commit_guard < generation_recheck
            && shutdown_recheck < terminal_emit
            && generation_recheck < terminal_emit,
        "{label} must hold state.commit, then recheck shutdown/generation, then emit terminal event before the lifecycle gate can release; commit_guard={commit_guard} shutdown_recheck={shutdown_recheck} generation_recheck={generation_recheck} terminal_emit={terminal_emit}"
    );
    assert!(
        drop_commit_before_emit.is_none(),
        "{label} must not drop state.commit before terminal event emit; drop_commit_before_emit={drop_commit_before_emit:?} terminal_emit={terminal_emit}"
    );
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

    let transcript_scope_marker = "let transcript =";
    let transcript_scope = block_starting_at(copy, transcript_scope_marker);
    let history_lock = index_of(transcript_scope, "state.history.lock");
    let publish_call = index_of(copy, "publish(");
    let transcript_clone = transcript_scope.find("transcript.clone()").unwrap_or_else(|| {
        panic!(
            "copy_retained_history_transcript must clone transcript while holding history.lock(), then publish the owned transcript after the guard scope ends; publishing a borrowed entry transcript keeps the mutex held"
        )
    });
    let marker_start = index_of(copy, transcript_scope_marker);
    let scope_open = marker_start + index_of(&copy[marker_start..], "{");
    let guard_scope_end = scope_open + 1 + transcript_scope.len();

    assert!(
        history_lock < transcript_clone
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
fn tdt_rolling_context_window_is_at_least_eight_seconds_with_explicit_overlap() {
    let source = streaming_source();
    let segment_samples = const_usize(&source, "SEGMENT_SAMPLES");
    let window_segments = const_usize(&source, "WINDOW_SEGMENTS");
    let overlap_segments = const_usize(&source, "OVERLAP_SEGMENTS");
    let window_samples = segment_samples * window_segments;
    let minimum_samples = 16_000 * 8;

    assert!(
        window_samples >= minimum_samples,
        "TDT fallback must use finite rolling context of at least 8 seconds at mono 16k/2,560-sample chunks; current WINDOW_SEGMENTS={window_segments} gives {window_samples} samples"
    );
    assert!(
        overlap_segments > 0 && overlap_segments < window_segments,
        "TDT fallback must keep explicit nonzero overlap smaller than the rolling context window"
    );
}

#[test]
fn start_speech_capture_does_not_load_model_synchronously_before_mic_activation() {
    let source = runtime_source();
    let start = function_body(&source, "start_speech_capture");

    assert!(
        !start.contains("preload_parakeet_tdt") && !start.contains("load_parakeet_tdt"),
        "start_speech_capture must not call the TDT model loader directly; model warmup must be process-lifetime async setup work so click-to-mic-start is not blocked by model load"
    );
}

#[test]
fn tdt_model_warmup_begins_nonblocking_during_tauri_setup() {
    let main = main_source();
    let setup = main
        .find(".setup(|app|")
        .map(|index| &main[index..])
        .unwrap_or_else(|| panic!("Tauri setup closure should exist"));

    assert!(
        setup.contains("speech")
            && (setup.contains("spawn") || setup.contains("spawn_blocking"))
            && (setup.contains("preload_parakeet_tdt")
                || setup.contains("load_parakeet_tdt")
                || setup.contains("warm")),
        "Tauri setup must begin nonblocking process-lifetime Parakeet TDT warmup; start_speech_capture must not pay first-load latency"
    );
}

#[test]
fn tdt_model_is_reused_across_sessions_and_returned_after_worker_finishes() {
    let source = runtime_source();
    let worker = function_body(&source, "run_streaming_worker");

    assert!(
        source.contains("OnceLock")
            && (source.contains("model_cache")
                || source.contains("warm_model")
                || source.contains("model_pool")
                || source.contains("available_model")
                || source.contains("take_model")
                || source.contains("return_model")),
        "speech runtime must own a process-lifetime warmed/reusable TDT model slot or pool instead of recreating the model per press"
    );
    assert!(
        !worker.contains("mut model: parakeet_rs::ParakeetTDT")
            || worker.contains("return_model")
            || worker.contains("put_model")
            || worker.contains("available_model"),
        "run_streaming_worker must return/reuse the loaded TDT model after a session instead of dropping the per-press instance"
    );
}

#[test]
fn model_checkout_cannot_escape_when_controller_start_fails() {
    let source = runtime_source();
    let start = function_body(&source, "start_speech_capture");

    let take_model = index_of(start, "take_model()");
    let controller_start = index_of(start, "controller.start");

    if controller_start < take_model {
        return;
    }

    let controller_start_statement = &start[controller_start
        ..start[controller_start..]
            .find(';')
            .map(|index| controller_start + index + 1)
            .unwrap_or(start.len())];

    assert!(
        !controller_start_statement.contains('?') && controller_start_statement.contains("return_model"),
        "model checkout must not escape after failed controller.start(); start_source position take_model={take_model} controller.start={controller_start}. Move controller.start before take_model, or handle controller.start failure after checkout with visible return_model before returning"
    );
}

#[test]
fn start_speech_capture_rejects_shutdown_before_model_checkout_and_capture_publish() {
    let source = runtime_source();
    let start = function_body(&source, "start_speech_capture");

    let shutdown_guard = start
        .find("state.shutting_down.load")
        .unwrap_or_else(|| {
            panic!(
                "start_speech_capture must visibly reject when shutting_down is already true before model checkout/capture setup; missing state.shutting_down.load guard"
            )
        });
    let take_model = index_of(start, "take_model()");
    let intake_install = index_of(start, "intake_slot.set");
    let capture_publish = index_of(start, "inner.capture = Some");

    assert!(
        shutdown_guard < take_model,
        "shutdown gate must run before process-warmed model checkout so post-shutdown start cannot steal a model; shutdown_guard={shutdown_guard} take_model={take_model}"
    );
    assert!(
        shutdown_guard < intake_install && shutdown_guard < capture_publish,
        "shutdown gate must run before CPAL callback intake install and Capture publication; shutdown_guard={shutdown_guard} intake_install={intake_install} capture_publish={capture_publish}"
    );
}

#[test]
fn start_speech_capture_rechecks_shutdown_under_commit_guard_before_publishing_capture() {
    let source = runtime_source();
    let start = function_body(&source, "start_speech_capture");

    let commit_guard = start.find("state.commit.lock").unwrap_or_else(|| {
        panic!(
            "start_speech_capture must coordinate with shutdown/commit lifecycle guard before publishing Capture; missing state.commit.lock guard"
        )
    });
    let shutdown_after_commit = start[commit_guard..]
        .find("state.shutting_down.load")
        .map(|index| commit_guard + index)
        .unwrap_or_else(|| {
            panic!(
                "start_speech_capture must recheck shutting_down while holding commit/lifecycle guard before Capture publication"
            )
        });
    let capture_publish = index_of(start, "inner.capture = Some");
    let worker_spawn = index_of(start, "spawn_blocking");

    assert!(
        commit_guard < shutdown_after_commit && shutdown_after_commit < capture_publish,
        "shutdown recheck must be serialized under commit/lifecycle guard before capture publish; commit_guard={commit_guard} shutdown_after_commit={shutdown_after_commit} capture_publish={capture_publish}"
    );
    assert!(
        shutdown_after_commit < worker_spawn,
        "shutdown recheck must happen before worker spawn so post-shutdown start cannot leave live worker/capture; shutdown_after_commit={shutdown_after_commit} worker_spawn={worker_spawn}"
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
fn intake_close_state_publishes_reason_and_closed_as_one_observable_state() {
    let source = streaming_source();
    let first_close = function_body(&source, "first_close");

    let uses_packed_close_state = source.contains("close_state")
        || source.contains("ClosedState")
        || source.contains("AtomicUsize")
        || source.contains("AtomicU8")
        || source.contains("AtomicU32");
    let reason_store = first_close.find("close_reason");
    let input_closed_store = first_close.find("input_closed");

    assert!(
        uses_packed_close_state || (reason_store.is_some() && input_closed_store.is_some()),
        "BoundedIntake close must expose winner reason with closed bit: use atomic packed close state, or publish reason before closed"
    );

    if !uses_packed_close_state {
        let reason_store = reason_store.unwrap();
        let input_closed_store = input_closed_store.unwrap();
        assert!(
            reason_store < input_closed_store,
            "first_close currently lets readers see input_closed=true before close_reason is published; publish reason before closed or pack both atomically. reason_store={reason_store} input_closed_store={input_closed_store}"
        );
    }
}

#[test]
fn closed_intake_readers_never_fallback_safe_failure_from_missing_reason() {
    let source = streaming_source();

    assert!(
        !source.contains("reason().unwrap_or(InputCloseReason::SafeFailure)"),
        "readers must not map input_closed=true plus missing close_reason to SafeFailure; Finish/RecordingCap winner must be observable with closed state before rejecting tail/commit"
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

#[test]
fn start_emit_failure_rolls_back_live_capture_and_wakes_worker_for_model_return() {
    let source = runtime_source();
    let start = function_body(&source, "start_speech_capture");

    let emit = index_of(start, "emit(&app, &event)");
    let schedule_limit = index_of(start, "schedule_recording_limit");
    let after_emit = &start[emit..schedule_limit];

    assert!(
        !after_emit.contains("emit(&app, &event)?"),
        "start_speech_capture must not use bare `emit(&app, &event)?` after capture install/worker spawn; emit failure must run rollback so capture/controller/worker do not stay live and model can return"
    );
    assert!(
        after_emit.contains("if let Err(error) = emit(&app, &event)")
            || after_emit.contains("match emit(&app, &event)")
            || after_emit.contains("rollback")
            || after_emit.contains("recover_failed_start"),
        "start_speech_capture emit failure branch must be visible after worker spawn; handle error explicitly instead of propagating with `?`"
    );
    assert!(
        after_emit.contains("capture.take") || after_emit.contains("inner.capture.take"),
        "start emit failure rollback must remove live Capture from runtime state before returning error"
    );
    assert!(
        after_emit.contains("first_close(InputCloseReason::Shutdown)")
            || after_emit.contains("first_close(InputCloseReason::SafeFailure)")
            || after_emit.contains("first_close(InputCloseReason::Cancel)"),
        "start emit failure rollback must seal BoundedIntake so recv_until_closed wakes and worker can return model"
    );
    assert!(
        after_emit.contains("drop(capture)") || after_emit.contains("drop(capture.intake)"),
        "start emit failure rollback must drop/remove capture resources after sealing intake"
    );
    assert!(
        after_emit.contains("recover_failed_stop")
            || after_emit.contains("complete_error")
            || after_emit.contains("state.invalidate"),
        "start emit failure rollback must invalidate/complete controller before returning error"
    );
}

#[test]
fn start_recording_emit_is_inside_commit_guard_after_shutdown_recheck_and_capture_publish() {
    let source = runtime_source();
    let start = function_body(&source, "start_speech_capture");

    let commit_guard = start.find("state.commit.lock").unwrap_or_else(|| {
        panic!("start_speech_capture must acquire commit/lifecycle guard before start publication")
    });
    let shutdown_recheck = start[commit_guard..]
        .find("state.shutting_down.load")
        .map(|index| commit_guard + index)
        .unwrap_or_else(|| {
            panic!(
                "start_speech_capture must recheck shutdown while holding commit/lifecycle guard before publishing Recording"
            )
        });
    let capture_publish = index_of(start, "inner.capture = Some");
    let recording_emit = index_of(start, "emit(&app, &event)");
    let drop_commit_before_emit = start[commit_guard..recording_emit]
        .find("drop(commit)")
        .map(|index| commit_guard + index);

    assert!(
        commit_guard < shutdown_recheck
            && shutdown_recheck < capture_publish
            && capture_publish < recording_emit,
        "Recording emit must be serialized under the same commit/lifecycle guard after shutdown recheck and Capture publication; commit_guard={commit_guard} shutdown_recheck={shutdown_recheck} capture_publish={capture_publish} recording_emit={recording_emit}"
    );
    assert!(
        drop_commit_before_emit.is_none(),
        "start_speech_capture must not release commit/lifecycle guard before Recording emit; otherwise shutdown can remove Capture and stale start can still emit success. drop_commit_before_emit={drop_commit_before_emit:?} recording_emit={recording_emit}"
    );
}

#[test]
fn worker_returns_model_to_available_pool_before_terminal_event_emit() {
    let source = runtime_source();
    let worker = function_body(&source, "run_streaming_worker");

    let terminal_emit = terminal_status_emit(worker).unwrap_or_else(|| {
        panic!("run_streaming_worker must emit terminal speech event through SPEECH_STATUS_CHANGED")
    });
    let explicit_return_before_emit = worker[..terminal_emit].contains("return_model")
        || worker[..terminal_emit].contains("put_model")
        || worker[..terminal_emit].contains("available_model")
        || worker[..terminal_emit].contains("drop(model)");

    assert!(
        explicit_return_before_emit,
        "run_streaming_worker must return/drop WorkerModel before terminal event emit so immediate restart observes model Ready instead of InUse; terminal_emit={terminal_emit}"
    );
}

#[test]
fn worker_completion_terminal_emit_rechecks_shutdown_generation_under_lifecycle_gate() {
    let source = runtime_source();
    let worker = function_body(&source, "run_streaming_worker");
    let terminal_emit_scope = block_starting_at(worker, "if let Some(event) = event");

    assert_deferred_terminal_emit_has_lifecycle_gate(
        terminal_emit_scope,
        "worker completion deferred terminal emit",
    );
}

#[test]
fn stop_emit_failure_closes_extracted_capture_intake_before_recovery_return() {
    let source = runtime_source();
    let stop = function_body(&source, "stop_speech_capture");

    let capture_take = index_of(stop, "inner.capture.take");
    let emit_failure = index_of(stop, "if let Err(error) = emit(&app, &event)");
    let recover = index_of(stop, "recover_failed_stop(&app, &state, request.nonce)");
    let return_error = index_of(stop, "return Err(error)");
    let success_destructure = index_of(stop, "let Capture");
    let failure_branch = &stop[emit_failure..success_destructure];
    let first_close = failure_branch
        .find("capture.intake.first_close(InputCloseReason::SafeFailure)")
        .map(|index| emit_failure + index)
        .unwrap_or_else(|| {
            panic!(
                "stop emitter leak: stop_speech_capture must first-close the already extracted Capture intake with InputCloseReason::SafeFailure before recover_failed_stop/return; dropping CPAL stream alone can leave worker-owned sender and recv_until_closed alive, preventing warmed model return"
            )
        });

    assert!(
        capture_take < emit_failure,
        "stop emitter leak test requires emit-error branch after Capture extraction; capture.take={capture_take} emit_failure={emit_failure}"
    );
    assert!(
        emit_failure < first_close && first_close < recover && recover < return_error,
        "stop emitter leak: order must be emit failure -> capture.intake.first_close(InputCloseReason::SafeFailure) -> recover_failed_stop -> return Err; order emit_failure={emit_failure} first_close={first_close} recover={recover} return_error={return_error}"
    );
}

#[test]
fn shutdown_after_commit_closes_capture_intake_before_dropping_capture() {
    let source = runtime_source();
    let shutdown = function_body(&source, "shutdown_after_commit");

    let capture_take = index_of(shutdown, "capture.take");
    let first_close = shutdown
        .find("first_close(InputCloseReason::Shutdown)")
        .unwrap_or_else(|| {
            panic!(
                "shutdown_after_commit must call capture.intake.first_close(InputCloseReason::Shutdown) before drop(capture); otherwise worker-retained sender can keep recv_until_closed alive and prevent model return"
            )
        });
    let drop_capture = index_of(shutdown, "drop(capture)");

    assert!(
        capture_take < first_close && first_close < drop_capture,
        "shutdown_after_commit must take Capture, close captured BoundedIntake with InputCloseReason::Shutdown, then drop capture; order capture.take={capture_take} first_close={first_close} drop(capture)={drop_capture}"
    );
}

#[test]
fn terminal_reset_emit_rechecks_shutdown_generation_under_lifecycle_gate() {
    let source = runtime_source();
    let terminal_reset = function_body(&source, "schedule_terminal_reset");
    let terminal_emit_scope = block_starting_at(
        terminal_reset,
        "if let Some(event) = reset_terminal_if_current",
    );

    assert_deferred_terminal_emit_has_lifecycle_gate(
        terminal_emit_scope,
        "terminal reset deferred terminal emit",
    );
}

#[test]
fn transcription_timeout_emit_rechecks_shutdown_generation_under_lifecycle_gate() {
    let source = runtime_source();
    let transcription_limit = function_body(&source, "schedule_transcription_limit");
    let terminal_emit_scope = block_starting_at(transcription_limit, "if let Some(event) = event");

    assert_deferred_terminal_emit_has_lifecycle_gate(
        terminal_emit_scope,
        "transcription timeout deferred terminal emit",
    );
}

#[test]
fn stop_speech_capture_holds_lifecycle_gate_through_transcribing_handoff() {
    let source = runtime_source();
    let stop = function_body(&source, "stop_speech_capture");

    let commit_guard = stop.find("state.commit.lock").unwrap_or_else(|| {
        panic!(
            "stop_speech_capture must acquire state.commit before state.inner so start/stop cannot race and publish a mic activation after stop"
        )
    });
    let inner_lock = index_of(stop, "state.inner.lock");
    let capture_take = index_of(stop, "inner.capture.take");
    let transcribing_emit = index_of(stop, "emit(&app, &event)");
    let intake_close = index_of(stop, "intake.first_close(InputCloseReason::Finish)");
    let drop_commit_before_close = stop[commit_guard..intake_close]
        .find("drop(commit)")
        .map(|index| commit_guard + index);

    assert!(
        commit_guard < inner_lock,
        "stop_speech_capture must acquire state.commit before state.inner; commit_guard={commit_guard} inner_lock={inner_lock}"
    );
    assert!(
        commit_guard < capture_take && capture_take < transcribing_emit && transcribing_emit < intake_close,
        "stop_speech_capture lifecycle gate must cover Capture removal, Transcribing event, and intake close; commit_guard={commit_guard} capture_take={capture_take} transcribing_emit={transcribing_emit} intake_close={intake_close}"
    );
    assert!(
        drop_commit_before_close.is_none(),
        "stop_speech_capture must retain state.commit through intake close; dropping it earlier allows start/stop race to activate mic after stop. drop_commit_before_close={drop_commit_before_close:?} intake_close={intake_close}"
    );
}

#[test]
fn recording_cap_terminal_transition_holds_lifecycle_gate_through_handoff_and_emit() {
    let source = runtime_source();
    let schedule_cap = function_body(&source, "schedule_recording_limit");

    let commit_guard = schedule_cap.find("state.commit.lock").unwrap_or_else(|| {
        panic!(
            "recording-cap timer must acquire state.commit before state.inner so shutdown cannot interleave and later receive stale Transcribing; missing state.commit.lock in schedule_recording_limit"
        )
    });
    let shutdown_recheck = schedule_cap[commit_guard..]
        .find("state.shutting_down.load")
        .map(|index| commit_guard + index)
        .unwrap_or_else(|| {
            panic!(
                "recording-cap timer must recheck shutting_down while holding state.commit before mutating inner state"
            )
        });
    let generation_recheck = schedule_cap[commit_guard..]
        .find("state.generation.load")
        .map(|index| commit_guard + index)
        .unwrap_or_else(|| {
            panic!(
                "recording-cap timer must recheck generation while holding state.commit before mutating inner state"
            )
        });
    let inner_lock = index_of(schedule_cap, "state.inner.lock");
    let stop_for_cap = index_of(schedule_cap, "stop_for_recording_cap");
    let capture_take = index_of(schedule_cap, "inner.capture.take");
    let drop_stream = index_of(schedule_cap, "drop(capture.stream)");
    let handoff_tail = index_of(schedule_cap, "handoff_tail");
    let intake_close = index_of(
        schedule_cap,
        "capture.intake.first_close(InputCloseReason::RecordingCap)",
    );
    let drop_intake = index_of(schedule_cap, "drop(capture.intake)");
    let terminal_emit =
        terminal_status_emit(schedule_cap).expect("recording cap terminal status emit");
    let transcription_limit = index_of(schedule_cap, "schedule_transcription_limit");
    let drop_commit_before_emit = schedule_cap[commit_guard..terminal_emit]
        .find("drop(_commit)")
        .or_else(|| schedule_cap[commit_guard..terminal_emit].find("drop(commit)"))
        .map(|index| commit_guard + index);

    assert!(
        commit_guard < inner_lock,
        "recording-cap timer must acquire state.commit before state.inner; commit_guard={commit_guard} inner_lock={inner_lock}"
    );
    assert!(
        commit_guard < shutdown_recheck
            && commit_guard < generation_recheck
            && shutdown_recheck < inner_lock
            && generation_recheck < inner_lock,
        "recording-cap shutdown/generation guards must run under state.commit before state.inner mutation; commit_guard={commit_guard} shutdown_recheck={shutdown_recheck} generation_recheck={generation_recheck} inner_lock={inner_lock}"
    );
    assert!(
        inner_lock < stop_for_cap && stop_for_cap < capture_take,
        "recording-cap terminal transition must stop controller and take Capture while lifecycle gate is held; inner_lock={inner_lock} stop_for_cap={stop_for_cap} capture_take={capture_take}"
    );
    assert!(
        capture_take < drop_stream
            && drop_stream < handoff_tail
            && handoff_tail < intake_close
            && intake_close < drop_intake
            && drop_intake < terminal_emit
            && terminal_emit < transcription_limit,
        "recording-cap lifecycle gate must cover Capture take, stream close, deadline-aware tail handoff, RecordingCap intake close, terminal emit, then transcription-limit scheduling; capture_take={capture_take} drop_stream={drop_stream} handoff_tail={handoff_tail} intake_close={intake_close} drop_intake={drop_intake} terminal_emit={terminal_emit} transcription_limit={transcription_limit}"
    );
    assert!(
        drop_commit_before_emit.is_none(),
        "recording-cap timer must not release state.commit before terminal Transcribing emit; drop_commit_before_emit={drop_commit_before_emit:?} terminal_emit={terminal_emit}"
    );
}

#[test]
fn capture_health_terminal_error_holds_lifecycle_gate_through_recovery_and_emit() {
    let source = runtime_source();
    let schedule_health = function_body(&source, "schedule_capture_health");

    let commit_guard = schedule_health.find("state.commit.lock").unwrap_or_else(|| {
        panic!(
            "capture-health failure path must acquire state.commit before state.inner so shutdown cannot interleave and later receive stale Error; missing state.commit.lock in schedule_capture_health"
        )
    });
    let shutdown_recheck = schedule_health[commit_guard..]
        .find("state.shutting_down.load")
        .map(|index| commit_guard + index)
        .unwrap_or_else(|| {
            panic!(
                "capture-health failure path must recheck shutting_down while holding state.commit before terminal recovery"
            )
        });
    let generation_recheck = schedule_health[commit_guard..]
        .find("state.generation.load")
        .map(|index| commit_guard + index)
        .unwrap_or_else(|| {
            panic!(
                "capture-health failure path must recheck generation while holding state.commit before terminal recovery"
            )
        });
    let inner_lock = index_of(schedule_health, "state.inner.lock");
    let capture_failure = index_of(schedule_health, "capture_failure");
    let controller_stop = index_of(schedule_health, "controller.stop");
    let complete_error = index_of(schedule_health, "complete_error_code");
    let capture_take = index_of(schedule_health, "inner.capture.take");
    let invalidate = index_of(schedule_health, "state.invalidate");
    let safe_failure_close = index_of(
        schedule_health,
        "capture.intake.first_close(InputCloseReason::SafeFailure)",
    );
    let drop_capture = index_of(schedule_health, "drop(capture)");
    let terminal_emit =
        terminal_status_emit(schedule_health).expect("capture health terminal status emit");
    let terminal_reset = index_of(schedule_health, "schedule_terminal_reset");
    let drop_commit_before_emit = schedule_health[commit_guard..terminal_emit]
        .find("drop(_commit)")
        .or_else(|| schedule_health[commit_guard..terminal_emit].find("drop(commit)"))
        .map(|index| commit_guard + index);

    assert!(
        commit_guard < inner_lock,
        "capture-health failure path must acquire state.commit before state.inner; commit_guard={commit_guard} inner_lock={inner_lock}"
    );
    assert!(
        commit_guard < shutdown_recheck
            && commit_guard < generation_recheck
            && shutdown_recheck < inner_lock
            && generation_recheck < inner_lock,
        "capture-health shutdown/generation guards must run under state.commit before state.inner mutation; commit_guard={commit_guard} shutdown_recheck={shutdown_recheck} generation_recheck={generation_recheck} inner_lock={inner_lock}"
    );
    assert!(
        inner_lock < capture_failure
            && capture_failure < controller_stop
            && controller_stop < complete_error
            && complete_error < capture_take,
        "capture-health failure path must detect Capture health failure, stop controller, complete terminal Error, then take Capture under lifecycle gate; inner_lock={inner_lock} capture_failure={capture_failure} controller_stop={controller_stop} complete_error={complete_error} capture_take={capture_take}"
    );
    assert!(
        capture_take < invalidate
            && invalidate < safe_failure_close
            && safe_failure_close < drop_capture
            && drop_capture < terminal_emit
            && terminal_emit < terminal_reset,
        "capture-health terminal recovery must invalidate session, SafeFailure-close intake, drop Capture, emit Error, then schedule terminal reset while lifecycle gate is retained; capture_take={capture_take} invalidate={invalidate} safe_failure_close={safe_failure_close} drop_capture={drop_capture} terminal_emit={terminal_emit} terminal_reset={terminal_reset}"
    );
    assert!(
        drop_commit_before_emit.is_none(),
        "capture-health failure path must not release state.commit before terminal Error emit; drop_commit_before_emit={drop_commit_before_emit:?} terminal_emit={terminal_emit}"
    );
}
