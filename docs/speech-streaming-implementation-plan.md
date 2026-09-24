# Speech Streaming and Five-Minute Dictation Plan

## 1. Metadata

- **Document ID:** SPEECH-STREAMING-PLAN-2026-09-23
- **Status:** Draft — implementation unauthorized pending Phase 0 evidence and model/provenance gate.
- **Author role:** documentation-only planner
- **Owner:** repository owner
- **Required reviewers:** Rust/native owner, frontend owner, QA owner, privacy reviewer, model/provenance reviewer
- **Source authority:** inspected runtime source/tests take precedence over this plan; `README.md` is user-facing truth; `master_spec.md` is architecture reference.
- **Product code impact from this document:** none. This document MUST NOT be read as shipped behavior.
- **Scope:** local microphone dictation only. No live shell smoke is authorized by this plan.

## Context

Current dictation is capture-then-transcribe. `speech_runtime.rs` retains whole CPAL recording in an interleaved `Vec<f32>` for up to 90 seconds; stopping capture resamples whole recording, loads `ParakeetTDT`, then transcribes whole audio on a blocking worker. This causes long recordings to wait roughly 10–15 seconds after **Finish** before clipboard publication. `MAX_SPEECH_OPERATION_DURATION` is measured from capture start, so a long capture can also consume its finalization budget before ASR starts.

Target behavior: transcribe bounded audio work while microphone capture continues, then stop/flush only remaining audio when user presses **Finish**. Finish should publish completed transcript with no full-recording ASR backlog. Recording is limited to five minutes, but UI has no short countdown/cancellation behavior; reaching cap finalizes already captured speech rather than discarding it.

## 3. Definition of done and non-regression invariants

### Definition of done

- **DOD-1:** Approved streaming-model spike proves artifact validity, provenance, local-only operation, correctness, and latency budget; or owner explicitly approves documented TDT fallback.
- **DOD-2:** Five-minute capture cap finalizes captured audio exactly once; no speech before cap is silently discarded solely because cap was reached.
- **DOD-3:** Normal Finish performs only bounded tail flush/finalization, not initial model loading or whole-session ASR.
- **DOD-4:** Required RED-first tests, benchmark evidence, review, and consented Windows microphone E2E pass.
- **DOD-5:** `master_spec.md`, relevant smoke guidance, and changelog are updated only with implemented durable behavior/evidence.

### Existing invariants that MUST remain true

- **INV-1:** Speech/audio/transcripts stay local. No cloud ASR, network inference, disk audio persistence, transcript persistence, telemetry, or content logging.
- **INV-2:** Error, stale, cancel, blank, model, timeout, and clipboard failure MUST preserve existing clipboard contents.
- **INV-3:** Clipboard remains native persistent STA/OLE Unicode ownership; browser and PowerShell fallbacks remain forbidden.
- **INV-4:** Generation/nonce stale guard, commit serialization, shutdown invalidation, terminal reset, and RAM-only history (newest five, each <=16 KiB) remain effective.
- **INV-5:** Existing public Tauri commands/events and frontend nonce rejection remain compatible unless separately approved and contract-tested.
- **INV-6:** Audio callback MUST NOT block on ASR, await, model loading, or clipboard work; locks MUST NOT span awaits.

## Scope

### In scope

- Preload one local ASR model instance before first usable recording, with explicit readiness/failure state.
- Move resampling, segmentation, and ASR work off audio callback and run incrementally during capture.
- Five-minute hard capture cap and deterministic forced-finalization path.
- No-countdown recording UX and accessible status language.
- Streaming-model spike; EOU route when gates pass; bounded TDT fallback when they do not.
- Deterministic tests, fixture benchmarks, manual Windows microphone/clipboard E2E after consent.

## Out of Scope

- OS-1: Cloud transcription, API keys, remote model download, telemetry, or disk persistence.
- OS-2: Automatic source/model replacement beyond approved Parakeet EOU artifact; any unrelated ASR engine needs a new plan.
- OS-3: Speaker diarization, word timestamps, partial transcript display, editing UI, language selection, VAD product UI, or multi-session concurrency.
- OS-4: Changing clipboard fallback policy, history limits, or shell permissions.
- OS-5: Live Tauri/shell smoke without explicit human consent.
- OS-6: Claiming model license/redistribution approval before provenance review completes.

## 5. External research and decision record

### Evidence

| Source | Finding | Design consequence |
|---|---|---|
| [parakeet-rs 0.3.7 `ParakeetTDT`](https://docs.rs/parakeet-rs/0.3.7/parakeet_rs/struct.ParakeetTDT.html), accessed 2026-09-23 | Public API exposes `transcribe_samples(Vec<f32>, sample_rate, channels, Option<TimestampMode>)`; no stateful incremental API. Calls need mutable model access. | Current TDT bundle cannot provide true streaming merely by slicing audio. It can only provide independent rolling chunks with JShell merge risk. |
| [parakeet-rs 0.3.7 `ParakeetEOU`](https://docs.rs/parakeet-rs/0.3.7/parakeet_rs/struct.ParakeetEOU.html), accessed 2026-09-23 | Explicit cache-aware streaming API: `transcribe(&[f32], is_final)`; docs show 16 kHz mono, 2,560-sample/160 ms chunks and zero-chunk flush. | EOU is preferred route if compatible artifact/provenance/quality gates pass. |
| [parakeet-rs 0.3.7 crate docs](https://docs.rs/parakeet-rs/0.3.7/parakeet_rs/), accessed 2026-09-23 | `ParakeetEOU` supports shared model handles; crate uses ONNX Runtime and 16 kHz mono models. | Keep inference local; model loading/ownership must be explicit and bounded. |

### Decision matrix

| ID | Choice | Recommendation | Gate |
|---|---|---|---|
| DEC-1 | True streaming engine | Prefer `ParakeetEOU`; it owns streaming context and end-of-utterance state. | Phase 0 artifact + measurement gates. |
| DEC-2 | Existing TDT | Retain as fallback only: preload one model, serialize it on one worker, run overlapping finite segments, merge conservatively, process tail on finish. | Owner approves quality/latency tradeoff if EOU fails. |
| DEC-3 | Model load lifecycle | Preload before recording becomes available; never first-load after Finish. | Startup/ready-state tests and bounded worker proof. |
| DEC-4 | Audio work cadence | Start EOU spike at 2,560 mono-16k samples (160 ms) per upstream example; make cadence an internal measured setting, not user setting. | Phase 0 throughput/quality measurements. |
| DEC-5 | Recording cap | 300 seconds hard cap; cap stops CPAL input then runs same final tail flush as Finish. | Invariant; tests required. |
| DEC-6 | Timeout policy | Replace start-to-finish 120 s operation deadline with finalization deadline beginning only after input closes. Initial candidate: 45 s, finalized only after profiling. | Phase 0 evidence + owner approval. |

### Phase 0 blocking gates

Implementation MUST stop and obtain owner decision if any applies:

1. EOU artifact cannot be sourced with exact revision, source URL, license, redistribution status, checksum, and verified expected files.
2. Artifact fails offline model load, 16 kHz mono stream processing, or approved fixture quality test.
3. Measured sustained inference cannot keep pace with input on supported Windows target, causing unbounded queued audio.
4. P95 Finish-to-clipboard time cannot meet approved budget after streaming; no fabricated latency threshold is accepted.
5. EOU output/finalization semantics cannot prove no duplicated/dropped tail on fixtures.

Recommended outcome: select EOU only when every gate passes. Otherwise return an evidence report with TDT fallback quality/latency measurements and ask owner to choose fallback or defer.

## Functional Requirements

- FR-1: System MUST start local model readiness/preload before accepting a recording that claims streaming capability. Model-ready failure MUST produce existing safe error behavior and retain clipboard.
- FR-2: System MUST convert source audio to bounded 16 kHz mono segments while capture is active and enqueue them with bounded backpressure.
- FR-3: Audio callback MUST only copy/convert minimal bounded data or send nonblocking work; it MUST NOT invoke ASR, load models, await, or allocate unbounded session audio.
- FR-4: EOU route MUST send ordered segments to one session stream and keep output state tied to nonce/generation. Its Finish path MUST mark input final and perform documented flush sequence before commit.
- FR-5: System MUST publish to clipboard only one nonblank finalized transcript after authorized finalization. Interim text MUST NOT write clipboard or history.
- FR-6: Explicit Finish MUST close capture immediately, drain only accepted pre-close audio, run final tail flush, then enter copied/error terminal state.
- FR-7: At 300 seconds measured from successful CPAL stream activation (`stream.play()` returning `Ok`), system MUST prevent additional input, transition through normal finalization, and retain/process accepted pre-cap audio. Input handoff MUST compare an activation-derived deadline before accepting each callback block/segment, so scheduler delay cannot admit post-deadline audio; timer task is a wake-up/close trigger only. It MUST NOT use old 90-second cancellation/timeout behavior. Failed activation MUST NOT arm cap timer or create a streaming session.
- FR-8: Recording UI MUST NOT present a user-facing short countdown or tell user capture was cancelled before 300 seconds. It MAY show neutral recording status and accessible cap state after cap is reached.
- FR-9: Finalization timeout MUST begin when capture closes (explicit Finish or cap), not when capture starts. On timeout it MUST reject final clipboard write, preserve clipboard, invalidate stream, and emit whitelisted error.
- FR-10: Newer nonce, cancel, shutdown, input failure, worker failure, stale worker message, or terminal state MUST prevent older session commit and release RAM-only buffers/model stream state.
- FR-11: System MUST cap buffered audio and queued segments. On sustained overload, it MUST fail safely with opaque error before silently dropping speech; callback-overrun/drop metrics are test-only/aggregate and MUST NOT contain content.
- FR-12: Existing commands/events (`start_speech_capture`, `stop_speech_capture`, existing speech event name/status shape) SHOULD remain. Add public fields/statuses only if required to represent cap/readiness safely and contract tests document compatibility.

## Non-Functional Requirements

- **NFR-PERF-1:** Phase 0 MUST record baseline and selected-route p50/p95: model cold/warm load, capture-to-segment queue delay, per-segment inference, backlog high-water, Finish-to-clipboard, and cap-to-clipboard. Fixture data only.
- **NFR-PERF-2:** During supported-target five-minute fixture replay, queue/backlog MUST remain bounded and no accepted segment may be silently dropped.
- **NFR-PERF-3:** Final latency budget MUST be based on measurements. Candidate acceptance target: p95 Finish-to-clipboard <=2 s for normal fixture utterances after warm model, excluding deliberately injected clipboard failure; owner must approve final threshold.
- **NFR-REL-1:** Exactly one terminal outcome occurs per nonce. Late messages cannot alter UI, history, or clipboard.
- **NFR-REL-2:** Memory use is bounded by finite queue/segment capacities plus only model-required context; no full five-minute raw `Vec<f32>` remains on EOU path.
- **NFR-PRIV-1:** No transcript/audio/model-path content enters logs, benchmark artifacts, events beyond existing user-visible final transcript, or analytics.
- **NFR-PRIV-2:** Model provenance record contains artifact metadata only, never user speech.
- **NFR-A11Y-1:** Mic control retains keyboard operation, current busy semantics, coherent accessible status, and an announcement when cap automatically finalizes.
- **NFR-MAINT-1:** Streaming coordinator is deterministic/testable without CPAL hardware or Windows clipboard.

## API Contracts

### Preserve public contract unless Phase 3 approves extension

```ts
type SpeechStatus = 'idle' | 'recording' | 'transcribing' | 'copied' | 'error';

interface SpeechEvent {
  nonce: number | null;
  status: SpeechStatus;
  error?: SpeechErrorCode; // existing opaque whitelist only
  // Proposed optional compatibility field; required only to distinguish cap finalization.
  finalizationReason?: 'user_finish' | 'recording_cap';
}
```

`transcribing` means **input is closed and bounded finalization is in progress**; it MUST NOT mean full recording ASR has only just begun. The frontend keeps nonce filtering, including current nullable nonce behavior. `start_speech_capture` and `stop_speech_capture(nonce)` names/parameters remain unchanged unless an approved compatibility migration says otherwise. Existing events omit `finalizationReason`; Phase 3 MUST add this optional field on the cap-driven `transcribing` event only, with Rust/TypeScript exact-shape tests. This is required because present `transcribing` does not distinguish manual Finish from automatic cap.

No HTTP endpoint exists: native Tauri IPC replaces `POST /` API transport. No HTTP method/path is planned.

### Proposed internal Rust concepts

```rust
struct StreamingSession {
    nonce: u64,
    input_closed: bool,
    queued_samples: usize,
    finalization_started_at: Option<Instant>,
    // bounded segment sender; EOU/TDT worker owns ASR state and transcript accumulator
}

enum InputCloseReason { UserFinish, RecordingCap, DeviceFailure, Cancel, Shutdown }

enum WorkerMessage { Segment(Mono16kSegment), Close(InputCloseReason), Cancel(u64) }

struct FinalizationResult {
    nonce: u64,
    transcript: String,
    close_reason: InputCloseReason,
}
```

These are internal design names, not required final APIs. Use typed nonce/session identifiers where nearby code supports them. `WorkerMessage` channel MUST be bounded. The owner of the stateful `ParakeetEOU` session is exactly one worker for the active recording; no mutex is held across awaits or inference.

## Data Models

No persistent data model or migration is planned. Speech audio and transcript data remain transient, RAM-only, and subject to existing history limits.

### Transient data limits

| Item | Limit/policy |
|---|---|
| Capture duration | 300 seconds wall-clock hard cap, anchored at successful CPAL stream activation (`stream.play()` returns `Ok`), not controller transition/request receipt. |
| Input format | normalized 16 kHz mono before ASR. |
| EOU segment candidate | 2,560 samples / 160 ms; tune only from Phase 0 data. |
| Queue | fixed count derived from segment cadence and approved maximum queue delay; fail safe before overwrite/drop. |
| Raw capture | EOU route: bounded conversion/queue buffers only. TDT fallback: finite rolling overlap/tail buffer only, never full uncapped session vector. |
| Transcript/history | existing RAM-only newest five, <=16 KiB each; trim/blank rules unchanged. |
| Finalization | candidate 45 s from input close; owner approves value after measurements. |

## 9. Architecture target

```text
CPAL callback
  -> bounded source-frame handoff / mono16k conversion
  -> bounded ordered segment channel
  -> one owned ASR worker (preloaded EOU session)
  -> RAM-only finalized transcript accumulator
  -> nonce + commit-lock authorization
  -> native OLE clipboard + history

Finish or 300 s cap
  -> stop/take CPAL stream
  -> seal segment input
  -> drain accepted segments in order
  -> `transcribe(..., true)` + documented zero-tail flush
  -> validate/trim, authorize current nonce
  -> one clipboard commit or safe error
```

### Ownership, concurrency, and backpressure rules

1. CPAL callback must not wait. Use a bounded nonblocking handoff; account for rejected send without content logging.
2. Conversion segmentation happens in a bounded worker or a proven safe short callback path. It preserves sample ordering and source format boundaries.
3. Stateful model worker owns `&mut` model/session use. For EOU shared-handle API, sharing must not imply concurrent calls for one logical session.
4. Session state changes and commit authorization occur under existing short critical sections. Clone/move needed state before awaits.
5. Stop/cap seals intake atomically before draining. Capture handoff compares `Instant::now()` to activation deadline before every accepted callback block/segment; at/after deadline it seals/rejects intake even if cap scheduler is delayed. First close reason wins. Later callback/worker data with nonce mismatch is ignored and freed.
6. Backpressure is explicit: queue full never causes indefinite callback blocking. Chosen policy must either absorb within approved fixed buffer or produce safe terminal error; silent sample eviction is forbidden.

### EOU finalization semantics to prove in spike

1. Create/load approved EOU model locally and establish a fresh per-recording stream state.
2. Feed each 16 kHz mono segment with `is_final = false`; append only model-emitted committed text according to crate semantics.
3. On input close, send remaining partial segment, then `is_final = true` exactly as crate docs require.
4. Send documented three zero chunks if artifact/API requires them to flush cached tail.
5. Normalize whitespace only; do not invent word de-duplication that can remove valid speech. Verify fixture exact/normalized output and no duplicate tail.
6. Reject blank output by current safe behavior. Do not write partial output on error.

### TDT contingency boundary

If EOU gate fails and owner approves fallback, implement a separately tested route:

- Preload one `ParakeetTDT` and own it on one serialized worker.
- Cut rolling finite segments with measured overlap and a final tail segment.
- Merge only at tested stable textual boundaries; surface uncertain merge/quality as a documented limitation.
- Never promise true streaming, seamless context, or same accuracy as EOU.
- Keep EOU-specific artifacts/files out of fallback unless provenance gate passes.

## 10. File-level implementation work graph

### Phase 0 — spike, characterization, and approval evidence

**Allowed files:** new isolated Rust spike/fixture test module, `src-tauri/Cargo.toml` only if needed for approved artifact wiring, model provenance record, test fixtures/manifests, plan evidence directory ignored/redacted as policy requires. Do not replace production capture path.

1. Characterize current TDT: first-load time, full-recording transcription time, finish latency, memory allocation, and 90/120-second boundaries with deterministic fixture audio.
2. Obtain candidate EOU artifact only through owner-authorized source. Record origin URL, publisher, exact revision, license, redistribution decision, hashes, expected layout, and validation owner. No artifact is committed/downloaded automatically by runtime.
3. Build offline EOU smoke test: load artifact, feed fixed 16 kHz mono fixture in 160 ms chunks, final flag/zero flush, assert nonblank expected normalized transcript and deterministic no-duplicate tail.
4. Replay >=5-minute synthetic and representative licensed fixture audio. Capture worker throughput, max queue, RAM high-water, and final tail time. Fixtures must never contain private dictation.
5. Produce decision record: EOU selected / TDT fallback proposed / defer. Owner approves route, finalization deadline, queue capacity, latency threshold, and model artifact before Phase 1.

**RED tests:** future EOU load, chunk ordering, final flush, cap, and timeout semantics are first introduced failing; characterization tests may pass against current behavior but do not count as target evidence.

### Phase 1 — isolated streaming coordinator and bounded audio pipeline

**Primary files:** `src-tauri/src/speech_runtime.rs`; possibly new `speech_streaming.rs`; `src-tauri/src/speech.rs`; unit tests alongside module; `tests/speechContracts.test.mjs` only if public contract changes.

1. Extract pure/session coordinator from CPAL/Tauri dependencies: nonce lifecycle, input seal, bounded queue accounting, close reason, finalization start, cancellation, and terminal authorization.
2. Replace capture-wide raw-audio storage with bounded handoff and ordered mono16k segmentation. Preserve device health/dropped-frame handling, but turn sustained overload into explicit safe failure.
3. Raise `MAX_RECORDING_DURATION` to `Duration::from_secs(300)`; update capacity logic so it does not allocate five minutes of raw audio for EOU route.
4. Store activation-derived cap deadline immediately after successful `stream.play()`; input handoff must reject/seal at/after deadline. Schedule cap task exactly 300 seconds after activation only as wake-up/close path; delayed scheduling must not extend accepted capture. Failed activation never stores deadline or schedules task. Cap calls same idempotent close/finalize entry as Finish, but emits optional `finalizationReason: 'recording_cap'` with `transcribing` so UI can announce cap deterministically; it must not emit old `timeout` before draining.
5. Split timers: remove start-based whole-operation expiry; create cancelable finalization deadline when input closes. Preserve stale/commit restrictions when it expires.
6. Add deterministic clock/timer seam or pure time transition tests. Avoid tests that sleep five minutes.

**Commit slice:** coordinator tests -> bounded capture/segmentation -> cap/timer migration. Each slice remains revertable.

### Phase 2 — approved ASR worker integration and preload

**Primary files:** `src-tauri/src/speech_runtime.rs`, `src-tauri/src/speech_model.rs`, new focused model/worker module if needed, `src-tauri/tests/speech_model_load.rs`, `src-tauri/Cargo.toml` only for approved model route.

1. Add process-lifetime lazy preload manager with explicit `Loading`, `Ready`, and safe `Failed` states. Model load occurs on bounded blocking worker, never under callback/state lock.
2. Start preload at controlled app initialization or first mic affordance readiness, based on current Tauri lifecycle. Do not block shell startup; define disabled/error user state if not ready.
3. EOU route: construct owned per-nonce stream state from approved loaded handle/model; run one ordered blocking inference worker; retain only bounded local transient state.
4. Finish/cap sends close after accepted segment order, performs final flush, validates text, then calls existing authorized history+clipboard commit path.
5. TDT fallback, if selected, implements only Phase-0-approved segmentation/overlap/merge policy and explicit quality limitations.
6. On model/device/worker failures, cancel queued work, clear transient buffers, emit existing opaque error code, preserve clipboard.

### Phase 3 — user surface and IPC compatibility

**Primary files:** `src/lib/speech.ts`, current top-bar mic component(s), `tests/topBarMicControl.test.mjs`, `tests/speechContracts.test.mjs`, `tests/speechHistoryPanelUi.test.mjs`, `src/ipc/commands.ts`, `src/ipc/events.ts`, `src-tauri/src/contracts.rs` only if approved fields change.

1. Keep start/finish ownership and nonce filtering. UI should show recording/processing status, not a short countdown or user-visible cancellation timer.
2. On automatic cap, use `finalizationReason: 'recording_cap'` to present neutral accessible status such as “Recording limit reached; finishing dictation.” Manual Finish has no cap announcement. Neither path may falsely report cancellation or loss.
3. Retain existing transcribing spinner until native terminal event. Do not optimistically claim copied.
4. Add optional `finalizationReason` field only for cap-driven `transcribing`, after exact cross-language contract tests prove `nonce: number | null`, legacy omitted field behavior, user-Finish event, and cap event. Do not derive cap state from existing `transcribing`; it is ambiguous.
5. Validate keyboard access, accessible name, busy status, focus behavior, stale events, failure state, and history rendering unchanged.

### Phase 4 — docs, release evidence, and rollout

**Primary files:** `master_spec.md`, `docs/smoke-test-windows.md`, `README.md` only if user-facing workflow changes, `changelog.md` per `CHANGELOG_POLICY.md`, model provenance record, benchmark evidence summary.

1. Update durable docs after code/tests demonstrate actual behavior; distinguish EOU selected vs TDT fallback limitations.
2. Add consent-gated manual Windows E2E steps for warm/cold start, normal Finish, cap, device failure, stale/cancel, clipboard preservation, and five-minute speech retention.
3. Roll out via isolated commits; default rollback is revert. Runtime flag requires separate owner approval, both-path tests, and removal plan.

## Acceptance Criteria

### AC-1: (FR-1, NFR-PERF-1)
Given approved model artifact is ready, when recording starts, then no model load begins after Finish and evidence records warm readiness.

### AC-2: (FR-2, FR-3, NFR-REL-2)
Given continuous source input, when capture runs, then ordered 16 kHz mono segments reach bounded worker queue without callback waiting or whole-session raw accumulation.

### AC-3: (FR-4, FR-5)
Given known chunk fixture, when EOU stream is finalized, then output equals approved normalized expected transcript once, with no duplicated/dropped final tail, and only final output can commit.

### AC-4: (FR-6, NFR-PERF-3)
Given a warmed long-session fixture, when Finish closes input, then only outstanding tail is processed and Finish-to-clipboard meets owner-approved p95 budget.

### AC-5: (FR-7)
Given `stream.play()` succeeds and input handoff sees its activation deadline, when time is at or after 300 seconds—even if cap task is delayed—then later input is refused/sealed, all accepted pre-cap audio enters finalization, cap `transcribing` emits `finalizationReason: 'recording_cap'`, and exactly one terminal copied/error event results.

### AC-6: (FR-8, NFR-A11Y-1)
Given recording is active, when it is below cap, then UI has no short countdown/cancellation timer; when cap closes input, it announces neutral finishing state and preserves keyboard semantics.

### AC-7: (FR-9)
Given finalization exceeds approved post-close deadline, when deadline expires, then no clipboard write occurs, existing clipboard is preserved, state is terminal error, and late worker result is ignored.

### AC-8: (FR-10, NFR-REL-1)
Given Finish then new nonce/cancel/shutdown, when old worker emits result, then old transcript cannot update history, UI, or clipboard.

### AC-9: (FR-11)
Given worker is intentionally slower than source beyond fixed queue capacity, when queue fills, then system reaches safe explicit error with no silent segment overwrite and callback remains nonblocking.

### AC-10: (FR-12)
Given existing frontend command/event consumers, when implementation lands without unapproved API migration, then source contract tests show unchanged command/event names, `nonce: number | null`, legacy omitted optional field, and nonce filtering; separate tests distinguish manual Finish from cap `finalizationReason` event.

### AC-11: (FR-5, FR-9, FR-10)
Given success/failure/cancel/cap paths, when outcome occurs, then no disk/network/content logs occur; native clipboard/history policies remain unchanged.

### AC-12: (FR-6, FR-7)
Given release candidate on consented Windows hardware, when physical microphone and clipboard smoke is run, then normal Finish and cap paths meet approved observed outcome and evidence is recorded without transcript content.

## Edge Cases

| ID | Scenario | Required behavior |
|---|---|---|
| EC-1 | Finish immediately after start / no samples | Close safely; blank rule preserves clipboard; no worker panic. |
| EC-2 | Partial final segment | Pad/flush exactly per approved model semantics; prove via fixture. |
| EC-3 | Cap and Finish race | First idempotent input-close wins; one finalization and terminal event. |
| EC-4 | Cap scheduler delayed or worker backlogged | Intake boundary rejects at/after activation deadline independently of delayed timer, then seals input, drains accepted bounded queue, and uses post-close deadline; never old start-based 120 s cancellation. |
| EC-5 | Device callback error/disconnect | Stop intake, invalidate or safe-finalize only if data integrity policy permits; preserve clipboard on failure. |
| EC-6 | Queue full | Do not block callback or overwrite old samples; explicit safe error and cleanup. |
| EC-7 | Model preload fails or model files malformed | Existing safe error, no recording that pretends streaming works, no clipboard change. |
| EC-8 | Worker/model panic or ASR error | Convert to opaque error; cleanup state/buffers; no stale commit. |
| EC-9 | Clipboard unavailable/fails | No fallback; history/terminal behavior follows current tested policy; clipboard remains unchanged. |
| EC-10 | Nonce replacement/cancel/shutdown during flush | Generation/commit check rejects late output; clear session RAM/history as current shutdown contract requires. |
| EC-11 | Five-minute silence | Final blank behavior; no stale clipboard overwrite. |
| EC-12 | Long continuous speech across chunks | EOU preserves model context; fallback must demonstrate merge behavior or be rejected. |
| EC-13 | Source sample rate/channels change/unsupported | Normalize or reject through existing capture validation; no invalid data to model. |
| EC-14 | Transcript exceeds history cap | Existing per-entry trim/limit applies only to RAM history. Clipboard receives validated full final transcript under current contract; no new clipboard truncation is implied. |

Traceability index: EC-1 empty; EC-2 partial tail; EC-3 Finish/cap race; EC-4 cap backlog; EC-5 device; EC-6 queue; EC-7 model; EC-8 worker; EC-9 clipboard; EC-10 stale/cancel/shutdown; EC-11 silence; EC-12 continuous cross-chunk speech; EC-13 source format; EC-14 history/clipboard limits.

- EC-1: Empty input preserves blank/clipboard safety.
- EC-2: Partial tail flush follows approved model semantics.
- EC-3: First close wins Finish/cap race.
- EC-4: Cap deadline blocks post-cap intake despite delayed scheduler; backlog drains accepted input under post-close deadline.
- EC-5: Device errors preserve integrity policy and clipboard safety.
- EC-6: Queue overflow is explicit and nonblocking.
- EC-7: Model errors do not begin deceptive capture.
- EC-8: Worker error cannot commit late data.
- EC-9: Clipboard failure has no fallback.
- EC-10: Stale/cancel/shutdown invalidates old result.
- EC-11: Silence follows blank behavior.
- EC-12: Continuous speech preserves EOU context or rejects fallback quality.
- EC-13: Format changes normalize or fail safe.
- EC-14: History cap never silently changes full clipboard transcript.

## 13. Verification plan and required evidence

### Automated tests (RED first)

| Layer | Tests/evidence |
|---|---|
| Rust coordinator | state transitions, first-close-wins, cap deadline enforced at intake despite delayed scheduler, finalization timer starts on close, nonce cancellation, stale results, exactly-once commit authorization. |
| Audio pipeline | sample ordering, rate/channel conversion, partial segment, fixed capacity, queue-full safe failure, callback never waits. Use synthetic fixtures/fakes. |
| EOU/TDT worker | warm preload, sequential worker ownership, chunk output accumulation, final flag/zero flush, blank/error behavior, duplicate-tail regression fixture. |
| Existing Rust speech | update 90/120-second assertions to 300-second cap and post-close deadline; retain clipboard/history/shutdown/stale assertions. |
| Node contracts/UI | unchanged command/event names unless approved, nonce reduction, Stop/transcribing/copied sequence, no timer copy/countdown, accessible cap processing status. |
| Benchmark harness | fixed licensed/synthetic corpus, cold/warm, short/long/five-minute replay; p50/p95 and max queue/RAM; redacted metadata only. |

### Manual Windows E2E — explicit consent required

Do not run during implementation unless owner grants consent. After consent, use updated `docs/smoke-test-windows.md` and verify:

1. Cold start readiness/error behavior without blocking shell.
2. Warm short dictation: Finish copies final text without visible whole-recording delay.
3. Warm long dictation: speaking continues while worker processes; Finish tail is prompt.
4. Five-minute cap: spoken content before cap appears after finalization; UI says finishing, not cancellation.
5. Cancel/new recording/shutdown during tail: older text never changes clipboard.
6. Mic disconnect/model failure/clipboard failure: prior clipboard remains intact.

### Quality and privacy evidence

- Store only artifact metadata, checksum, timings, counts, and pass/fail in benchmark records.
- Fixture transcripts/audio must be synthetic or license-approved. Do not capture developer/user dictation.
- Compare selected route to current TDT baseline on approved representative fixture corpus; owner defines error measure and acceptance threshold in Phase 0.

## 14. Rollback and operations

- Default rollback: revert isolated phase commit(s); no persistent runtime switch by default.
- Runtime flag is permitted only if owner approves operational need, both paths are tested, docs state lifespan, and removal/revert owner is assigned.
- Model artifact failure after release: disable availability safely, keep old clipboard intact, report existing opaque model error. Do not silently switch to cloud or unapproved engine.
- Any post-release perf/quality regression: retain non-content metrics, reproduce on fixed fixtures, revert selected route or disable affordance via approved rollback plan.

## 15. Implementation checklist

- [ ] Owner approves Phase 0 spike scope and artifact acquisition source.
- [ ] Provenance/license/revision/checksum record complete and reviewed.
- [ ] EOU versus TDT decision approved with quality/performance evidence.
- [ ] Queue capacity, post-close deadline, and latency budget approved from measurements.
- [ ] RED tests added and observed failing before corresponding production change.
- [ ] Rust/Node focused tests pass; `npm run test:node`, `npm run cargo:test`, checks/build required by changed scope pass.
- [ ] Privacy/security review confirms no content persistence/logging/network path.
- [ ] Accessibility/source UI tests pass.
- [ ] Independent code review passes.
- [ ] Human consents to, then passes, Windows microphone/clipboard E2E.
- [ ] Durable docs/changelog updated from actual implementation evidence.

## 16. Open decisions requiring owner approval

1. Approve Phase 0 sourcing and evaluation of a specific EOU model artifact, subject to provenance/license review.
2. Select EOU route if gates pass; otherwise explicitly approve TDT fallback or defer.
3. Approve final p95 Finish-to-clipboard target, queue capacity, and post-close finalization deadline from target-device measurements.
4. Approve any public IPC event shape/status change if current statuses cannot express readiness/cap safely.
5. Approve consented live Windows shell/microphone smoke when implementation reaches E2E gate.

## 17. Final implementation boundary

No phase may claim “streaming” solely because it asynchronously transcribes independent TDT chunks. Only EOU (or another separately approved cache-aware stream API with equivalent evidence) may claim true stateful streaming. Until Phase 0 gates and owner approvals close, this document is a plan, not authority to alter product behavior.
