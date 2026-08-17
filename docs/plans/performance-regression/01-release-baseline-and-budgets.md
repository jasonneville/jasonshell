---
date: 2026-08-15
status: Harness implemented; release acceptance pending manual scenario measurement
requirements: [P01-FR1, P01-FR2, P01-FR3, P01-FR4, P01-FR5, P01-NFR1]
depends_on: []
scope: Measurement harness only; no product behavior changes
---

# Plan 01: Release Baseline and Budgets

## Context and evidence

Performance remediation needs a trusted baseline before product behavior changes. Debug/dev runs can expose hotspots, but release acceptance must be based on packaged or release-mode measurement. The harness compares scenarios across repeated runs and produces machine-readable JSON plus human-readable Markdown.

Latest evidence: `test-results/performance-regression/20260815-195509805-a7231ef9ae274252a4c565b12e0ff8bc/` contains 3 passing cold-idle release runs. The same execution blocked 18 complex/manual scenario runs because it was noninteractive. Release accepted remains false; this plan is not closed.

## Preflight

- Read `master_spec.md` before changing files.
- Inspect current `git status` and preserve unrelated dirty work.
- Confirm no prior-plan stop/go artifact is required; this is first plan.
- Confirm `test-results/performance-regression/` remains ignored before writing run artifacts.

## Requirements

- P01-FR1: The harness MUST measure release and dev diagnostic modes separately.
- P01-FR2: The harness MUST run each scenario three times and report median CPU, private bytes, working set, thread count, handle count, and control I/O where available.
- P01-FR3: The harness MUST cover cold/idle, 20+ windows, notifications, noisy Quick Commands output, terminal hidden/prewarm, fullscreen, and multi-monitor scenarios.
- P01-FR4: Budgets MUST initially use measured baseline non-regression, not fabricated numeric thresholds.
- P01-FR5: A keyboard-hook p99 latency target MAY be proposed later only after harness validity is established.
- P01-NFR1: Release acceptance evidence MUST be clearly separated from dev diagnostic evidence.

## Invariants and non-goals

- No product behavior changes.
- No source behavior optimization in this plan.
- No fabricated numeric thresholds beyond measured baseline non-regression.
- Do not edit `Cargo.toml` except if future implementation explicitly needs approved tooling changes.

## Exact file/symbol impact

- Harness script: `scripts/measure-performance.ps1`.
- Harness output root: `test-results/performance-regression/<timestamp>/`.
- Per-run JSON: `test-results/performance-regression/<timestamp>/run-<scenario>-<index>.json`.
- Summary Markdown: `test-results/performance-regression/<timestamp>/summary.md`.
- Residual-risk Markdown: `test-results/performance-regression/<timestamp>/residual-risk.md`.
- Release build commands: `npm run build`, then `npm run tauri -- build`.
- Harness must discover and launch produced release binary; do not hardcode package path without detection.
- Contract tests: `tests/performanceBaselineContract.test.mjs`.
- Repo validation command: `npm run validate`; this is not performance acceptance.

## Phased tasks

| ID | Task | Depends on | Definition of done |
|---|---|---|---|
| P01-T1 | Define JSON schema for process metrics, scenario metadata, run mode, and timestamps. | None | Complete. Schema is covered by `tests/performanceBaselineContract.test.mjs`. |
| P01-T2 | Add RED-first contract tests for schema and scenario matrix. | P01-T1 | Complete. Tests exist at `tests/performanceBaselineContract.test.mjs`. |
| P01-T3 | Implement `scripts/measure-performance.ps1` scenario runner for release and dev diagnostic modes. | P01-T2 | Complete. Runner records repeated scenario artifacts under `test-results/performance-regression/<timestamp>/`. |
| P01-T4 | Implement Markdown median summary and non-regression budget derivation. | P01-T3 | Complete. Summary separates release acceptance from dev diagnostics and keeps blocked/manual scenarios explicit. |
| P01-T5 | Run initial baseline and mark budgets as measured-baseline-relative. | P01-T4 | Partially complete. Cold-idle release baseline has 3 passing runs; 18 complex/manual release scenario runs are blocked pending interactive measurement. |

## RED-first tests

- Implemented: `tests/performanceBaselineContract.test.mjs` covers scenario matrix and artifact schema.
- No Rust/product unit coverage is required because Plan 01 adds harness/tooling only and no product behavior changes.

## Acceptance criteria

- Given the harness runs in release mode, when all scenarios complete, then each scenario has three JSON run artifacts and a median summary.
- Given dev diagnostic measurement runs, when summary is generated, then it cannot be used as release acceptance.
- Given no valid baseline exists, when budgets are computed, then budgets use measured current baseline only after successful release runs.

## Performance evidence path

Claim: future remediation can be judged by repeatable release metrics. Evidence target: 3-run median JSON+Markdown across full scenario matrix. Current evidence: cold-idle has 3 passing release runs in `test-results/performance-regression/20260815-195509805-a7231ef9ae274252a4c565b12e0ff8bc/`; 18 complex/manual scenario runs are blocked by noninteractive execution. Limit: local machine variability remains; compare same machine/config unless CI perf host exists.

## Stop/go gate

- Go only if release harness produces repeatable 3-run medians plus per-run JSON, `summary.md`, and `residual-risk.md` under `test-results/performance-regression/<timestamp>/`.
- Repo validation (`npm run check`, `npm run build`, `npm run validate`) is separate from performance acceptance.
- If notifications, fullscreen, or multi-monitor prerequisites are unavailable, mark affected release scenario `blocked/not measured`; do not silently pass/fail unrelated repo validation.
- Plan cannot close until required release scenarios are measured or user explicitly accepts documented limitation. Current status: not closed; release accepted false.

## Rollback

Remove harness scripts/artifacts if the harness proves invalid before adoption. Changelog is append-only: do not remove an entry; append a corrective or superseding entry if earlier text becomes wrong. No product rollback required.

## Risks

- Scenario automation may be flaky on multi-monitor/fullscreen setups.
- Control I/O availability may vary by Windows API/tooling.
- Dev Vite/WebView diagnostics can be misleading if confused with release evidence.
- Noninteractive execution blocks complex/manual scenarios; these must be rerun in an interactive desktop session before release acceptance.

## Required implementation docs updates

- `master_spec.md`: document durable measurement harness contract, validation coverage, and release-acceptance risk.
- `changelog.md`: add `[CODE]` docs/tooling and `[TOOL]` validation bullets per policy.

## Validation commands

- Focused: `node --test tests/performanceBaselineContract.test.mjs`.
- Build release: `npm run build`; `npm run tauri -- build`.
- Repo validation: `npm run check`; `npm run build`; `npm run validate`.
- Performance acceptance: run `scripts/measure-performance.ps1`, launch discovered release binary, verify 3-run scenario artifacts under `test-results/performance-regression/<timestamp>/`.

## Invocation

`Implement Plan 01 from docs/plans/performance-regression/01-release-baseline-and-budgets.md. Preflight: read master_spec.md; inspect git status; preserve unrelated dirty work; confirm no prior-plan stop/go artifact is required. Follow RED-first tests, stop/go gates, validation, master_spec/changelog rules.`
