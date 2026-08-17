---
date: 2026-08-15
status: Source implementation complete; release gates blocked/pending
requirements: [P05-FR1, P05-FR2, P05-FR3, P05-FR4, P05-FR5, P05-FR6, P05-FR7, P05-FR8, P05-NFR1]
depends_on: [Plan 01, Plan 02, Plan 03, Plan 04]
scope: Harden low-level input hook, evidence-gate AppBar changes, and produce final release comparison
---

# Plan 05: Global Input and Shell Guard Hardening

## Context and evidence

The low-level `WH_KEYBOARD_LL` hook now remains extremely small and nonblocking: the Windows callback only classifies keys, updates tiny state, and `try_send`s to a bounded `sync_channel` with capacity 8. A worker thread owns `AppHandle` and `emit_to`; uninstall stops and joins the worker. Ctrl+Space behavior is unchanged. Alt+Backquote now suppresses duplicate terminal toggles on repeated keydown while the key is held. Frontend fallback handlers are untouched/preserved.

AppBar/fullscreen guard behavior remains unchanged by default. Guard instrumentation resets per guard start and logs `duration_ms` plus `wake_count` on stop. AppBar/fullscreen guard changes remain evidence-gated: instrument first, then change only if release traces show significant cost. Fullscreen release/park/restore behavior must not change by default.

## Preflight

- Read `master_spec.md` before changing files.
- Inspect current `git status` and preserve unrelated dirty work.
- Confirm Plans 01-04 stop/go artifacts exist, including final accepted or user-accepted limitations.
- Inspect listed symbols before editing; if moved, document plan amendment before implementation.

## Requirements

- P05-FR1: `WH_KEYBOARD_LL` callback MUST perform only classification, tiny state updates, and nonblocking bounded queue send.
- P05-FR2: An emitter worker MUST own `AppHandle` and `emit_to` work.
- P05-FR3: Ctrl+Space and Alt+Backquote suppress/repeat/pass-through behavior MUST be preserved.
- P05-FR4: Frontend fallback handlers MUST be preserved.
- P05-FR5: AppBar guard changes MUST be a separate evidence-gated phase inside this plan.
- P05-FR6: AppBar instrumentation MUST capture guard durations and wake counts before behavioral changes.
- P05-FR7: Release/park/restore/fullscreen behavior MUST NOT change by default.
- P05-FR8: Final release comparison MUST use Plan 01 schema and include residual-risk report.
- P05-NFR1: Fullscreen smoke MUST pass before completion.

## Exact file/symbol impact

- `src-tauri/src/windows_key_hook.rs::windows_key_hook_proc`.
- `src-tauri/src/appbar.rs::{FullscreenAppBarState,start_fullscreen_guard,sync_fullscreen_shell_surfaces_for_target,foreground_fullscreen_candidate,start_taskbar_guard}`.
- `src/components/TopBar.svelte` fallback handler contracts if touched.
- `src/components/BottomBar.svelte` fallback handler contracts if touched.
- Existing tests: `tests/windowsKeyOverride.test.mjs`; fullscreen-related tests if present.
- Existing smoke command: `npm run smoke:fullscreen`.
- Inspect-for-moved-symbol rule: search declarations/usages first; if symbol/path moved or renamed, update this plan via documented amendment before implementation.

## Phased tasks

| ID | Task | Depends on | Definition of done |
|---|---|---|---|
| P05-T1 | Capture current hotkey and fullscreen/AppBar behavior contract. | Plans 01-04 | Done for source contract: Ctrl+Space unchanged; Alt+Backquote repeat now suppresses duplicate terminal toggles; frontend fallback and AppBar semantics preserved. |
| P05-T2 | Add RED-first hook tests/source contracts for tiny callback and worker-owned emit. | P05-T1 | Done for focused source coverage. |
| P05-T3 | Refactor low-level hook to bounded queue plus emitter worker. | P05-T2 | Done: callback has no Tauri emit/AppHandle ownership, uses bounded capacity-8 nonblocking `try_send`; worker owns emit; uninstall stops/joins worker. |
| P05-T4 | Add AppBar guard instrumentation for durations/wake counts. | P05-T1 | Done: instrumentation resets per guard start and logs `duration_ms`/`wake_count` on stop; no AppBar behavior change. |
| P05-T5 | Decide AppBar go/no-go from release trace. | P05-T4 | Blocked/pending: release trace unavailable because manual Plan 01 scenarios remain blocked. No AppBar behavior change accepted. |
| P05-T6 | If go, implement minimal guard hardening preserving release/park/restore/fullscreen semantics. | P05-T5 | Blocked/pending: no go decision; fullscreen smoke is interactive/not run. |
| P05-T7 | Run final release comparison and residual-risk report. | P05-T3, P05-T5; also P05-T6 only if go decision | Blocked/pending: full Plan 01 release comparison plus residual-risk artifact unavailable because 18 manual scenarios remain blocked. |

## RED-first tests

- Existing/focused passed: `node --test tests/windowsKeyOverride.test.mjs`.
- Focused Rust passed from `src-tauri`: `cargo fmt --all && cargo test windows_key_hook`.
- Rust/source-contract coverage now proves no emit in low-level callback and bounded nonblocking queue handoff.
- Fullscreen smoke remains interactive/not run.

## Acceptance criteria

- Given Ctrl+Space is pressed, when the hook handles keydown/up/repeat, then centered search toggle, suppression, repeat suppression, and pass-through behavior match current contract.
- Given Alt+Backquote is pressed, when terminal toggle path runs, then current suppress/pass-through behavior and frontend fallback remain intact.
- Given fullscreen app enters/exits, when AppBar guard instrumentation is enabled, then release/park/restore behavior remains unchanged and smoke passes.
- Given final release comparison runs, when metrics are summarized, then residual risks are documented against Plan 01 baseline.

## Performance evidence path

For hook: prove callback work is bounded by source tests and event latency metrics; propose p99 target only after Plan 01/hook measurement validity. For AppBar: compare release guard durations/wake counts before and after any guarded change. Final report compares all scenario medians against Plan 01.

## Stop/go gate

- Go only if hook behavior is preserved, fullscreen smoke passes, and final release comparison/residual-risk report uses Plan 01 schema.
- Current status: source implementation complete, but P05 is not complete/release accepted. Fullscreen smoke is interactive/not run; final release comparison/residual-risk report is unavailable because 18 Plan 01 manual scenarios remain blocked.
- Repo validation (`npm run check`, focused Node/Rust tests, `npm run validate`) is separate from scenario/perf acceptance.
- If fullscreen or multi-monitor prerequisites are unavailable, mark affected scenario `blocked/not measured`; do not silently pass/fail unrelated repo validation.
- Plan cannot close until required release scenarios are measured or user explicitly accepts documented limitation.

## Rollback

Revert hook worker split if hotkey behavior regresses. Disable or remove AppBar behavior changes if fullscreen smoke or release trace regresses; keep instrumentation only if low-risk and documented.

## Risks

- Hook queue overflow policy could drop toggles if capacity is too small.
- Moving emits to worker can reorder events if sequence metadata is absent.
- AppBar changes can regress fullscreen/work-area recovery; default is no behavior change without evidence.

## Required implementation docs updates

- `master_spec.md`: document hook worker architecture, AppBar instrumentation/behavior changes, fullscreen smoke coverage, and residual risks.
- `changelog.md`: add implementation, validation, and release comparison bullets.

## Validation commands

- Focused: `node --test tests/windowsKeyOverride.test.mjs`; `npm run smoke:fullscreen`.
- Full: `npm run check`; `npm run test:node`; `npm run cargo:test`; `npm run validate`.

## Invocation

`Implement Plan 05 from docs/plans/performance-regression/05-global-input-and-shell-guard-hardening.md. Preflight: read master_spec.md; inspect git status; preserve unrelated dirty work; confirm Plans 01-04 stop/go artifacts. Follow RED-first tests, stop/go gates, validation, master_spec/changelog rules.`
