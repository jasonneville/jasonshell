# Task: FN-005 - Build JasonShell Performance Measurement Harness

**Created:** 2026-05-12
**Size:** L

## Review Level: 2 (Plan and Code)

**Assessment:** This adds an opt-in diagnostics/performance measurement path spanning scripts, renderer instrumentation, and Rust runtime metrics, but it should not change normal shell behavior. Risk is moderate because startup/window instrumentation touches app bootstrap and IPC registration, while the harness remains reversible and dev-only.
**Score:** 5/8 — Blast radius: 2, Pattern novelty: 1, Security: 1, Reversibility: 1

## Mission

Build a dev-only JasonShell performance measurement harness that makes future optimization work evidence-backed. The harness should collect cold-start-to-AppBar/first-paint milestones, per-window/process memory snapshots, dist/release asset and binary sizes, JS parse/evaluate timing proxies per surface, window counts, and selected IPC latency timings without changing release behavior or requiring manual spreadsheet work.

## Dependencies

- **None**

## Context to Read First

- `master_spec.md` — read the current system snapshot, primary surface labels, diagnostics, shell window/AppBar, terminal-panel, Stack Browser, and Process Manager sections.
- `CHANGELOG_POLICY.md` — follow repository changelog rules.
- `package.json` — existing validation scripts and Tauri/Vite commands.
- `src/main.ts` — renderer bootstrap point where opt-in startup/performance marks can be initialized.
- `src/App.svelte` — surface routing by Tauri webview window label.
- `src/lib/runtimeMetrics.ts` — existing frontend runtime metrics pattern for shell surfaces.
- `src/ipc/commands.ts` and `src/ipc/diagnostics.ts` — IPC command registry and diagnostics redaction/ring-buffer helpers.
- `src-tauri/src/main.rs` — Tauri command registration and managed state setup.
- `src-tauri/src/diagnostics.rs` — bounded diagnostics export pattern and redaction expectations.
- `src-tauri/src/shell_windows.rs` — canonical window labels and webview creation.
- `src-tauri/src/appbar.rs` — startup/AppBar reservation and shell runtime metrics command context.
- `tests/devTools.test.mjs`, `tests/sourceContractIntentRegistry.test.mjs`, and `tests/persistentSurfaceLifecycle.test.mjs` — source/contract test style for non-UI wiring and lifecycle invariants.
- Task document `plan` for FN-005, if available — planning notes saved during specification.

## File Scope

- `scripts/measure-performance.mjs` (new)
- `scripts/clean-dist-tests.mjs` (check/update only if generated perf report cleanup is needed)
- `src/lib/performanceHarness.ts` (new)
- `src/lib/runtimeMetrics.ts`
- `src/main.ts`
- `src/App.svelte`
- `src/ipc/commands.ts`
- `src/ipc/diagnostics.ts`
- `src-tauri/src/performance_metrics.rs` (new)
- `src-tauri/src/main.rs`
- `src-tauri/src/contracts.rs`
- `src-tauri/src/shell_windows.rs` (check/update only if shared label constants are needed by metrics)
- `src-tauri/src/diagnostics.rs` (check/update only for shared redaction/export patterns)
- `tests/performanceHarness.test.mjs` (new)
- `tests/sourceContractIntentRegistry.test.mjs` (check/update only for command registry expectations)
- `tests/*.test.mjs` related to changed source contracts, if required by existing policy tests
- `master_spec.md`
- `changelog.md`
- `.gitignore` (check/update only if generated perf output paths are not already ignored)

## Steps

### Step 0: Preflight

- [ ] Required files and paths exist
- [ ] Dependencies satisfied
- [ ] Confirm there is no existing performance harness or duplicate measurement script beyond `src/lib/runtimeMetrics.ts` and diagnostics helpers.
- [ ] Confirm generated measurement output paths will be ignored or written under an already ignored directory such as `.local/`.

### Step 1: Add opt-in frontend performance instrumentation

- [ ] Create `src/lib/performanceHarness.ts` with pure, testable helpers for detecting opt-in measurement mode, recording startup milestones, collecting `performance.getEntriesByType('navigation')` / resource timing summaries, and summarizing script/resource timing without exporting raw sensitive URLs.
- [ ] Instrument `src/main.ts` and `src/App.svelte` so each webview can record bootstrap, Svelte mount, surface-label resolution, and first-render/first-paint-adjacent milestones only when the harness is explicitly enabled by environment/query/local flag chosen by the implementation.
- [ ] Record the current surface label from the existing App routing path and include all canonical JasonShell labels from `src-tauri/src/shell_windows.rs`/frontend surface routing where practical.
- [ ] Ensure normal release/runtime behavior is unchanged when measurement mode is not enabled: no extra persistent timers, no user-visible UI, no storage writes, and no failing IPC calls.
- [ ] Write Node tests with assertions for opt-in detection, metric redaction/summarization, milestone ordering, and disabled-mode no-op behavior.
- [ ] Run targeted tests for changed frontend harness files (`npm run test:node -- performanceHarness` or the closest supported filter).

**Artifacts:**
- `src/lib/performanceHarness.ts` (new)
- `src/lib/runtimeMetrics.ts` (modified)
- `src/main.ts` (modified)
- `src/App.svelte` (modified)
- `tests/performanceHarness.test.mjs` (new)

### Step 2: Add backend runtime metrics and IPC timing commands

- [ ] Add `src-tauri/src/performance_metrics.rs` with Tauri commands that return a redacted performance snapshot containing app/window labels, visible/focused state where available, window count, per-process memory/process snapshots available on Windows, and binary/dist path size metadata when files exist.
- [ ] Include a simple selected IPC latency command, such as a ping/echo command returning backend receive/respond timestamps, suitable for repeated measurement from the script and/or renderer.
- [ ] Register new commands in `src-tauri/src/main.rs` and `src/ipc/commands.ts`; update `src-tauri/src/contracts.rs` if this repository's command/event contract tests require registry coverage.
- [ ] Keep all measurement commands read-only and dev-safe: no process mutation, no shell setting writes, no cache clearing, no launching external programs except the harness runner itself.
- [ ] Add Rust unit tests for size aggregation, snapshot shaping, missing-artifact handling, redaction/path sanitization, and latency timestamp monotonicity using deterministic test helpers.
- [ ] Run targeted Rust tests for the new module.

**Artifacts:**
- `src-tauri/src/performance_metrics.rs` (new)
- `src-tauri/src/main.rs` (modified)
- `src/ipc/commands.ts` (modified)
- `src-tauri/src/contracts.rs` (modified if needed)

### Step 3: Build the dev-only measurement runner

- [ ] Create `scripts/measure-performance.mjs` as a Node CLI that records repository metadata, measures `dist/` asset sizes, detects Tauri release/debug executable/bundle artifacts when available, and writes a timestamped JSON report under an ignored output directory.
- [ ] Implement harness modes that work without a packaged release: size-only/report generation must succeed after `npm run build`; runtime measurements should be collected when a runnable dev or release binary is available and otherwise be marked `skipped` with explicit reasons.
- [ ] When runtime mode is available, collect cold-start-to-first-milestone timing, startup milestone payloads from frontend instrumentation, window count, process/memory snapshots, per-window WebView2-related process data where Windows exposes it, and repeated selected IPC timings with min/median/p95/max summaries.
- [ ] Ensure the runner has bounded timeouts, exits non-zero only for harness/validation failures (not for clearly reported unavailable optional artifacts), and never changes release configuration or normal app behavior.
- [ ] Add Node tests with assertions for CLI argument parsing, asset-size traversal, binary discovery, percentile/summary math, skipped-runtime report shape, and JSON report schema.
- [ ] Run targeted Node tests for the script and shared harness helpers.

**Artifacts:**
- `scripts/measure-performance.mjs` (new)
- `.gitignore` (modified only if needed)
- `tests/performanceHarness.test.mjs` (modified)

### Step 4: Integrate reports with diagnostics/runtime metrics

- [ ] Provide a typed frontend wrapper in `src/lib/runtimeMetrics.ts` for the new backend performance snapshot and IPC ping commands, following the existing `invoke(IPC_COMMANDS...)` pattern.
- [ ] If frontend diagnostic export is reused, ensure `src/ipc/diagnostics.ts` redaction helpers cover any new performance fields that may include paths, command lines, usernames, or tokens.
- [ ] Ensure reports include these sections at minimum: `assetSizes`, `binarySizes`, `startupMilestones`, `windows`, `processMemory`, `ipcLatency`, `environment`, `skipped`, and `errors`.
- [ ] Add contract/source tests that prove new IPC command names are registered consistently between frontend and Rust and that reports do not include raw secret-like fields.
- [ ] Run focused source contract and diagnostics tests.

**Artifacts:**
- `src/lib/runtimeMetrics.ts` (modified)
- `src/ipc/diagnostics.ts` (modified if needed)
- `tests/performanceHarness.test.mjs` (modified)
- `tests/sourceContractIntentRegistry.test.mjs` (modified if needed)

### Step 5: Testing & Verification

> ZERO test failures allowed. Full test suite as quality gate.
> If keeping lint/tests/build/typecheck green requires edits outside the initial File Scope, make those fixes as part of this task.

- [ ] Run focused tests for the new harness and contracts (`npm run test:node -- performanceHarness` plus any relevant source-contract filters, or the closest exact commands supported by Node's test runner).
- [ ] Run focused Rust tests for `performance_metrics` (`npm run cargo:test -- performance_metrics` or the closest exact Cargo filter).
- [ ] Run lint/check (`npm run check`).
- [ ] Run full automated test suite (`npm run test:node` and `npm run cargo:test`).
- [ ] Run project typecheck/build (`npm run build` and `npm run cargo:check`).
- [ ] Run `node scripts/measure-performance.mjs --size-only` after build and verify it writes a valid JSON report without changing release behavior.
- [ ] Fix all failures.
- [ ] Build passes.

### Step 6: Documentation & Delivery

- [ ] Update `master_spec.md` with the current performance harness contract: opt-in enablement, collected metrics, report location/schema, runtime limitations, and read-only/no-release-behavior guarantee.
- [ ] Update `changelog.md` according to `CHANGELOG_POLICY.md` with concise implementation and validation notes.
- [ ] Add or update user/developer docs in `README.md` or `docs/` only if there is an existing appropriate diagnostics/performance section; otherwise keep the durable contract in `master_spec.md` and the script self-help output.
- [ ] Ensure `scripts/measure-performance.mjs --help` explains prerequisites, size-only mode, runtime mode, output path, and skipped-measurement semantics.
- [ ] Save documentation deliverables as task documents via `fn_task_document_write` (key="docs", content=...) including how to run the harness and where reports are written.
- [ ] Out-of-scope findings created as new tasks via `fn_task_create` tool.

## Documentation Requirements

**Must Update:**
- `master_spec.md` — document the opt-in performance measurement harness, collected metrics, IPC/runtime metric commands, report schema/location, and no-release-behavior-change guarantee.
- `changelog.md` — add policy-compliant FN-005 entries for implementation and validation.

**Check If Affected:**
- `README.md` — update if it already contains developer diagnostics/performance commands or if adding the harness command there is the clearest runbook location.
- `docs/` — update or add a focused diagnostics/performance runbook only if repository docs already have a suitable place or README would become too large.
- `.gitignore` — update only if generated performance report output is not already ignored.

## Completion Criteria

- [ ] All steps complete
- [ ] Harness runs in size-only mode without requiring a packaged release binary
- [ ] Runtime mode records or explicitly skips startup milestones, window count, per-window/process memory snapshots, IPC latency summaries, and available binary/bundle sizes
- [ ] Frontend startup/first-paint-adjacent instrumentation is opt-in and no-op when disabled
- [ ] Backend metrics commands are read-only, registered, and covered by automated tests
- [ ] JSON reports include `assetSizes`, `binarySizes`, `startupMilestones`, `windows`, `processMemory`, `ipcLatency`, `environment`, `skipped`, and `errors`
- [ ] Docs explain how to run the harness and interpret unavailable/skipped metrics
- [ ] Lint/check passing
- [ ] All tests passing
- [ ] Typecheck/build passing
- [ ] `node scripts/measure-performance.mjs --size-only` succeeds after build

## Git Commit Convention

Commits at step boundaries. All commits include the task ID:

- **Step completion:** `feat(FN-005): complete Step N — description`
- **Bug fixes:** `fix(FN-005): description`
- **Tests:** `test(FN-005): description`

## Do NOT

- Expand task scope into optimizing startup, memory, bundles, WebView2 behavior, icon caches, terminal prewarm, or window creation policy
- Skip tests
- Refuse necessary fixes just because they touch files outside the initial File Scope
- Commit without the task ID prefix
- Remove, delete, or gut modules, settings, interfaces, exports, or test files outside the File Scope
- Remove features as "cleanup" — if something seems unused, create a task via `fn_task_create`
- Change release defaults, app window visibility, AppBar reservation behavior, terminal startup policy, or user settings as part of measurement
- Persist performance measurements to normal user settings or localStorage unless explicitly gated as harness-only output
- Export secrets, bearer tokens, raw command lines with credentials, or unnecessary user-private paths in reports
- Make runtime measurements depend on a specific developer machine having a release installer; optional missing artifacts must be reported as skipped

## Changeset Requirements

If this task REMOVES existing functionality (deleting modules, settings, API endpoints, or exports), a changeset file is REQUIRED:
- Create `.changeset/fn-005-removal.md` explaining what was removed and why
- This is mandatory for any net-negative change (more deletions than additions to existing files)
