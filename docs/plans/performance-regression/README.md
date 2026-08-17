---
date: 2026-08-15
status: Plan 01 harness implemented; P05 source implementation complete; release acceptance pending manual scenario measurement
scope: Performance regression plan-series status through Plan 05
owner: Future measurement agent
---

# Performance Regression Remediation Plan Series

## Purpose

This directory tracks current JasonShell performance-regression measurement status. Plan 01 now has a durable harness, but release performance acceptance is not complete because required complex/manual scenarios have not been measured in an interactive desktop session. Plan 05 source implementation is complete, but release acceptance is not complete because fullscreen smoke and the final Plan 01 release comparison/residual-risk artifact are blocked or pending.

## Execution status

| Plan | Implementation status | Latest evidence | Release acceptance |
|---|---|---|---|
| [01 Release Baseline and Budgets](01-release-baseline-and-budgets.md) | Harness implemented: `scripts/measure-performance.ps1`; contract tests implemented: `tests/performanceBaselineContract.test.mjs`; release build passed. | `test-results/performance-regression/20260815-195509805-a7231ef9ae274252a4c565b12e0ff8bc/`: cold-idle produced 3 passing release runs; 18 complex/manual scenario runs were blocked by noninteractive execution. | Pending. Do not mark Plan 01 closed until required manual scenario measurements complete or the user explicitly accepts the documented limitation. |
| [02 Task Window Snapshot Pipeline](02-task-window-snapshot-pipeline.md) | Implementation complete; release scenario comparison pending/manual blocked. | Focused Node/Rust validation passed; release scenario comparison not yet measured. | Pending. Do not mark Plan 02 closed until required release comparison completes or the user explicitly accepts the documented limitation. |
| [03 Native Cache and Memory Bounds](03-native-cache-and-memory-bounds.md) | Implemented in source; bounded cache helper plus target cache migrations landed. | Latest evidence: focused Rust cache tests + full repo validation run. Release churn evidence still pending. | Pending; release acceptance still blocked until 10-minute interactive churn evidence is recorded. |
| [04 Quick Commands Output Pipeline](04-quick-commands-output-pipeline.md) | Implementation and source QA complete/clean; release acceptance pending noisy interactive scenario measurement. | Focused source/Rust coverage exists for `VecDeque` bounded transcript ordering, bounded renderer helper/cache behavior including terminal entries, one reused terminal sequence, cache sequence fallback, rAF coalescing, DOM text/redaction, and per-active-run poll watchdog freshness. Final actual validation: `npm run check` passed; `npm run cargo:test` passed with 442 passed and 1 ignored; focused P04 Node/Rust tests passed. `npm run test:node` and `npm run validate` fail only two unrelated pre-existing `tests/stackPopupState.test.mjs` sort assertions: `sorts stack entries deterministically while preserving folders first` and `sorts modified desc with folders/files grouped by timestamp and nulls last`. No full Node/validate pass claimed. No noisy-output release measurement is claimed; user accepted the Plan 01 blocked manual scenario limitation for docs status only, not as Plan 04 measurement. | Pending. Do not mark release performance accepted until noisy-output release evidence is measured or the limitation is explicitly accepted for Plan 04 release acceptance. |
| [05 Global Input and Shell Guard Hardening](05-global-input-and-shell-guard-hardening.md) | Source implementation complete; release gates blocked/pending. Windows low-level callback now only classifies/tiny state then `try_send`s to bounded sync channel (capacity 8); emitter worker owns `AppHandle`/`emit_to`; uninstall stops/joins worker. Ctrl+Space unchanged; Alt+Backquote repeat suppresses duplicate terminal toggles while held. Frontend fallback preserved. AppBar semantics unchanged; guard instrumentation resets per guard start and logs `duration_ms`/`wake_count` on stop. | Focused tests passed: `node --test tests/windowsKeyOverride.test.mjs`; from `src-tauri`, `cargo fmt --all && cargo test windows_key_hook`. Fullscreen smoke interactive/not run. Full Plan 01 release comparison and residual-risk artifact unavailable because 18 manual scenarios remain blocked. | Pending/blocked. P05 is not complete/release accepted; T5-T7/release gates remain blocked or pending until interactive fullscreen smoke, Plan 01 release comparison, and residual-risk reporting are complete or explicitly accepted as limited. |

## Shared invariants

- Preserve product behavior unless a plan explicitly states a behavior change; Plan 01 has no product behavior changes.
- Preserve dirty unrelated files, especially existing dirty `src-tauri/Cargo.toml`.
- Do not edit product source or `Cargo.toml` while executing measurement docs/status updates.
- Implementation agents must use RED-first tests where practical and label proposed test files as proposed until created.
- Use `CHANGELOG_POLICY.md` for ledger entries.
- Update `master_spec.md` only when implementation changes behavior, commands, events, persistence, validation coverage, or known risks.
- Performance conclusions must distinguish release acceptance evidence from dev diagnostic evidence.
- If multi-monitor, fullscreen, or notification prerequisites are unavailable, mark affected scenario `blocked/not measured`; do not pass/fail unrelated repo validation because of unavailable manual scenario. Plan cannot close until required release scenarios are measured or user explicitly accepts limitation.

## Artifact locations

- Harness script: `scripts/measure-performance.ps1`.
- Harness output root: `test-results/performance-regression/<timestamp>/`.
- Per-run JSON: `test-results/performance-regression/<timestamp>/run-<scenario>-<index>.json`.
- Summary Markdown: `test-results/performance-regression/<timestamp>/summary.md`.
- Residual-risk Markdown: `test-results/performance-regression/<timestamp>/residual-risk.md`.
- Latest evidence root: `test-results/performance-regression/20260815-195509805-a7231ef9ae274252a4c565b12e0ff8bc/`.

## Future prompts

- `Complete Plan 01 manual release scenario measurement from docs/plans/performance-regression/01-release-baseline-and-budgets.md. Use an interactive desktop session, preserve unrelated dirty work, update evidence artifacts, and only mark release acceptance complete if required scenarios are measured or the user explicitly accepts documented limitations.`
- `Implement Plan 02 from docs/plans/performance-regression/02-task-window-snapshot-pipeline.md. Preflight: read master_spec.md; inspect git status; preserve unrelated dirty work; confirm Plan 01 stop/go artifact. Follow RED-first tests, stop/go gates, validation, master_spec/changelog rules.`
- `Implement Plan 03 from docs/plans/performance-regression/03-native-cache-and-memory-bounds.md. Preflight: read master_spec.md; inspect git status; preserve unrelated dirty work; confirm Plan 01 stop/go artifact. Follow RED-first tests, stop/go gates, validation, master_spec/changelog rules.`
- `Implement Plan 04 from docs/plans/performance-regression/04-quick-commands-output-pipeline.md. Preflight: read master_spec.md; inspect git status; preserve unrelated dirty work; confirm Plan 01 stop/go artifact and any prior-plan stop/go artifact being consumed. Follow RED-first tests, stop/go gates, validation, master_spec/changelog rules.`
- `Implement Plan 05 from docs/plans/performance-regression/05-global-input-and-shell-guard-hardening.md. Preflight: read master_spec.md; inspect git status; preserve unrelated dirty work; confirm Plans 01-04 stop/go artifacts. Follow RED-first tests, stop/go gates, validation, master_spec/changelog rules.`
