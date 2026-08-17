---
date: 2026-08-15
status: Implementation/QA complete in source; release acceptance pending noisy-output measurement
requirements: [P04-FR1, P04-FR2, P04-FR3, P04-FR4, P04-FR5, P04-FR6, P04-FR7, P04-FR8, P04-NFR1]
depends_on: [Plan 01]
recommended_after: [Plan 02, Plan 03]
scope: Reduce Quick Commands transcript/render/backend queue overhead without semantic drift
---

# Plan 04: Quick Commands Output Pipeline

## Context and evidence

Quick Commands currently has live output, retained history, and a 1.1 s poll fallback. The plan must remove duplicate live-event plus poll work while preserving poll as watchdog/recovery, preserve serialized payload/history shape/order, and keep DOM text only with no raw HTML.

Current implementation evidence inspected for this docs update:

- `src/components/CommandPanelSurface.svelte` keeps transcript rendering DOM-text-only and memoizes `transcriptBodySegments(...)` in a bounded renderer cache (`TRANSCRIPT_SEGMENT_CACHE_LIMIT = 256`). The bounded helper includes terminal entries, reuses one terminal sequence for terminal-state rows, and falls back to cache sequence keys when entry sequence is unavailable. Live quick-command events merge into `allHistory`; visible `history` and pending-input derivation are coalesced through `window.requestAnimationFrame(...)`. History polling remains a 1100 ms watchdog/recovery path for per-active-run freshness only; merely showing Previous runs is not sufficient reason to poll.
- `src-tauri/src/quick_commands.rs` uses `VecDeque<QuickCommandTranscriptEntry>` for running transcript retention, bounds retained transcript rows with `TRANSCRIPT_LIMIT = 256`, serializes history back to `Vec` in existing order, and preserves stdout/stderr byte caps, input request validation, CRLF stdin write semantics, and secret submitted-input redaction (`[redacted]`).
- Focused coverage currently includes `tests/quickCommands.test.mjs` assertions for `VecDeque<QuickCommandTranscriptEntry>`, stable history/transcript payload ordering, pending input derivation, secret semantics, bounded terminal-entry helper behavior, and merge-by-runId/sequence; `tests/commandPanelWiring.test.mjs` assertions cover the merged transcript surface, `transcriptBodySegments(...)`, bounded cache/fallback sequence behavior, DOM text/redaction, and `requestAnimationFrame` coalescing; Rust `quick_commands` tests cover marker parsing, bounded transcript retention, ordered history serialization, run id uniqueness, redaction helper behavior, and state-to-history fields.
- Final actual validation evidence: `npm run check` passed; `npm run cargo:test` passed with 442 passed and 1 ignored; focused P04 Node/Rust tests passed. `npm run test:node` and `npm run validate` fail only two unrelated pre-existing `tests/stackPopupState.test.mjs` sort assertions: `sorts stack entries deterministically while preserving folders first` and `sorts modified desc with folders/files grouped by timestamp and nulls last`. No full Node/validate pass is claimed.

Completion status: implementation and source QA requirements are satisfied by current source. Release performance acceptance is explicitly pending because the noisy interactive Quick Commands scenario remains unmeasured. The user accepted the Plan 01 blocked manual scenario limitation for docs status purposes only; that acceptance is not a noisy-output measurement and does not close Plan 04 release acceptance.

## Preflight

- Read `master_spec.md` before changing files.
- Inspect current `git status` and preserve unrelated dirty work.
- Confirm Plan 01 stop/go artifact exists: `test-results/performance-regression/<timestamp>/summary.md` with valid release baseline, or stop.
- Confirm Plans 02-03 stop/go artifacts if this plan is run after them; otherwise document why this plan runs earlier.
- Inspect listed symbols before editing; if moved, document plan amendment before implementation.

## Requirements

- P04-FR1: Frontend MUST extract/memoize or precompute transcript segments keyed by stable entry sequence.
- P04-FR2: Renderer cache MUST be bounded.
- P04-FR3: Frontend update application MUST be batched/coalesced with `requestAnimationFrame` or equivalent.
- P04-FR4: Live-event plus 1.1 s poll duplicate work MUST be eliminated while retaining poll as per-active-run freshness watchdog/recovery. Previous Runs visibility alone MUST NOT trigger polling.
- P04-FR5: Backend MUST replace `Vec/remove(0)` output/history paths with `VecDeque` or bounded ring.
- P04-FR6: Serialized payload/history shape, order, input semantics, and secret semantics MUST be preserved.
- P04-FR7: Backend output batching MAY be considered only after tests prove event semantics.
- P04-FR8: The implementation MUST NOT use arbitrary thresholds such as `1MB <= 100 events` unless justified by evidence.
- P04-NFR1: Transcript rendering MUST remain DOM text only with no raw HTML.

## Exact file/symbol impact

- `src/components/CommandPanelSurface.svelte::{refreshHistory,shouldPollHistory,transcriptBodySegments,handleQuickCommandRunUpdated}`.
- Proposed helper path: `src/features/quick-commands/transcriptSegments.ts` unless existing structure forces another path; alternate path requires documented plan amendment.
- `src-tauri/src/quick_commands.rs::{capture_stream,append_running_output,emit_run_snapshot,push_transcript}`.
- Existing tests: `tests/quickCommands.test.mjs`, `tests/commandPanelWiring.test.mjs`, `tests/commandPanelTheme.test.mjs`.
- Focused Rust: `cargo test --manifest-path src-tauri/Cargo.toml quick_commands`.
- Inspect-for-moved-symbol rule: search declarations/usages first; if symbol/path moved or renamed, update this plan via documented amendment before implementation.

## Phased tasks

| ID | Task | Depends on | Definition of done |
|---|---|---|---|
| P04-T1 | Document current Quick Commands event, poll, history, and secret contracts. | Plan 01 | Contract map complete before code changes. |
| P04-T2 | Add RED-first tests for memoized transcript segments and poll-as-watchdog semantics. | P04-T1 | Tests fail against duplicate-work behavior. |
| P04-T3 | Implement stable-entry-sequence transcript memoization with bounded renderer cache. | P04-T2 | Repeated updates reuse unchanged segments. |
| P04-T4 | Batch frontend output application via rAF/coalescing. | P04-T3 | No per-chunk synchronous render cascade under noisy output. |
| P04-T5 | Replace backend `Vec/remove(0)` with `VecDeque`/bounded ring. | P04-T1 | Serialization shape/order unchanged. |
| P04-T6 | Remove duplicate live+poll work while keeping per-active-run poll watchdog/recovery. | P04-T4, P04-T5 | Poll recovers missed live event without steady duplicate processing; Previous Runs visibility alone does not poll. |

## RED-first tests

- Existing: `node --test tests/quickCommands.test.mjs tests/commandPanelWiring.test.mjs tests/commandPanelTheme.test.mjs`.
- Proposed: source-contract test for no `remove(0)` in quick command hot histories if not covered by Rust tests.
- Proposed: test for stable sequence-keyed transcript memoization and bounded cache eviction.

## Acceptance criteria

- Given noisy command output, when live events arrive, then frontend applies updates in coalesced batches and unchanged transcript segments are reused.
- Given live event freshness stalls for an active run, when watchdog poll runs, then missing output is recovered without permanent duplicate processing; merely showing Previous Runs does not cause polling.
- Given retained history is serialized, when backend storage uses `VecDeque` or ring internally, then payload shape and order remain unchanged.
- Given secret input is submitted, when history/transcript updates are stored or rendered, then existing redaction/input semantics remain unchanged.

## Performance evidence path

Use Plan 01 noisy Quick Commands release scenario. Compare renderer CPU, private bytes, working set, event count, poll count, and transcript update timing before/after.

## Stop/go gate

- Go only if Quick Commands serialized history/order/secret semantics pass and noisy-output release metrics are non-regressive against Plan 01.
- Repo validation (`npm run check`, focused Node/Rust tests, `npm run validate`) is separate from scenario/perf acceptance.
- If noisy-output scenario prerequisites are unavailable, mark scenario `blocked/not measured`; do not silently pass/fail unrelated repo validation.
- Plan cannot close until required release scenarios are measured or user explicitly accepts documented Plan 04 release limitation.
- Current release gate: pending. No noisy-output release measurement is documented here; implementation/source QA is complete only.
- Current repo-validation gate: P04 focused validation, `npm run check`, and `npm run cargo:test` passed; full Node/validate remain red only for unrelated pre-existing Stack Browser sort assertions listed above.

## Rollback

Revert frontend memoization/coalescing or backend container swap independently if output order, recovery, or secret semantics regress.

## Risks

- Sequence-key bugs can drop or reorder transcript segments.
- Poll watchdog removal could hide missed output if event delivery assumptions are wrong.
- Backend batching can change live output cadence; defer until tests justify.

## Required implementation docs updates

- `master_spec.md`: document output pipeline, watchdog semantics, bounded ring, and validation coverage.
- `changelog.md`: add implementation and validation bullets.

## Validation commands

- Focused: `node --test tests/quickCommands.test.mjs tests/commandPanelWiring.test.mjs tests/commandPanelTheme.test.mjs`; `cargo test --manifest-path src-tauri/Cargo.toml quick_commands`.
- Full: `npm run check`; `npm run test:node`; `npm run cargo:test`; `npm run validate`.

## Invocation

`Implement Plan 04 from docs/plans/performance-regression/04-quick-commands-output-pipeline.md. Preflight: read master_spec.md; inspect git status; preserve unrelated dirty work; confirm Plan 01 stop/go artifact and any prior-plan stop/go artifact being consumed. Follow RED-first tests, stop/go gates, validation, master_spec/changelog rules.`
