# Task: FN-002 - Lazy-create auxiliary Tauri windows

**Created:** 2026-05-12
**Size:** L

## Review Level: 2 (Plan and Code)

**Assessment:** This is a startup-lifecycle refactor that changes native Tauri window creation for many shell surfaces, but it should preserve existing window labels, IPC names, and UI behavior. The main risk is first-open regressions or missing focus-loss events for lazily-created panels; the change is reversible by restoring eager creation if needed.
**Score:** 5/8 — Blast radius: 2, Pattern novelty: 1, Security: 0, Reversibility: 2

## Mission

Reduce JasonShell cold-start WebView2/native memory pressure and time to visible top/bottom AppBars by creating only essential shell windows at startup and lazy-creating auxiliary Tauri windows the first time they are needed. Preserve all existing panel labels, positioning, open/hide/focus-loss behavior, event contracts, and terminal/session semantics so users see identical behavior after a panel's first open, with documented before/after startup window-count and timing measurements.

## Dependencies

- **None**

## Context to Read First

- `AGENTS.md` — repository operating rules; also read the user-level agents file it references before engineering work.
- `master_spec.md` — canonical shell/window/panel behavior, especially Primary surfaces, Native shell reservation, Top bar, Bottom bar, Search, Stack Browser, Process manager, and Persistent terminal panel sections.
- `CHANGELOG_POLICY.md` — changelog format and rules.
- `package.json` — validation commands; use the existing `npm run validate` chain plus focused commands.
- `src-tauri/src/shell_windows.rs` — current eager `create_shell_windows` implementation and all window builders/labels.
- `src-tauri/src/main.rs` — startup setup, focus-loss handlers, and `report_shell_surface_runtime_metrics` command.
- `src-tauri/src/search_panel.rs`, `src-tauri/src/stack_popup/popup_window.rs`, `src-tauri/src/process_manager.rs`, `src-tauri/src/terminal_panel.rs`, `src-tauri/src/settings_panel.rs`, `src-tauri/src/tray_panel.rs`, `src-tauri/src/command_panel.rs`, `src-tauri/src/audio_panel.rs`, `src-tauri/src/calendar_panel.rs`, `src-tauri/src/control_plane.rs`, `src-tauri/src/task_preview.rs` — show/hide/publish/positioning code that currently assumes windows already exist.
- `tests/shellOpenCloseEvents.test.mjs`, `tests/persistentSurfaceLifecycle.test.mjs`, `tests/persistentTerminalPanel.test.mjs`, `tests/centeredSearchSurface.test.mjs`, `tests/processManagerWiring.test.mjs`, `tests/trayPanelWiring.test.mjs`, `tests/commandPanelWiring.test.mjs`, `tests/topBarCalendar.test.mjs` — existing source-contract tests around panel lifecycle.

## File Scope

- `src-tauri/src/shell_windows.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/search_panel.rs`
- `src-tauri/src/stack_popup/popup_window.rs`
- `src-tauri/src/process_manager.rs`
- `src-tauri/src/terminal_panel.rs`
- `src-tauri/src/settings_panel.rs`
- `src-tauri/src/tray_panel.rs`
- `src-tauri/src/command_panel.rs`
- `src-tauri/src/audio_panel.rs`
- `src-tauri/src/calendar_panel.rs`
- `src-tauri/src/control_plane.rs`
- `src-tauri/src/task_preview.rs`
- `src-tauri/capabilities/*`
- `tests/*shell*Window*.test.mjs`
- `tests/shellOpenCloseEvents.test.mjs`
- Relevant existing panel wiring tests under `tests/*.test.mjs`
- `master_spec.md`
- `changelog.md`

## Steps

### Step 0: Preflight

- [ ] Required files and paths exist.
- [ ] Dependencies satisfied.
- [ ] Read the saved task plan document with `fn_task_document_read(key="plan")` and incorporate any still-relevant notes.
- [ ] Confirm the working tree status and preserve unrelated dirty changes.

### Step 1: Add a central lazy shell-window creation contract

- [ ] Refactor `src-tauri/src/shell_windows.rs` so `create_shell_windows(app: &mut App)` creates only `top-bar` and `bottom-bar` during startup and no longer calls auxiliary builders for hidden panels.
- [ ] Add a central, label-based lazy creation API for auxiliary surfaces, such as `ensure_shell_window(app_handle: &AppHandle, label: &str) -> AppResult<WebviewWindow>`, that returns an existing window if present or builds the correct window with the same label, URL, title, size, theme, visibility, shadow, resizability, and context-menu script as before.
- [ ] Keep top/bottom AppBar windows non-lazy and reject or no-op appropriately if callers try to lazy-create those labels.
- [ ] Preserve `ALL_LABELS` as the full surface registry for tests/contracts, and add a testable auxiliary-label list or helper so startup-created labels and lazy labels are explicitly distinct.
- [ ] Write Rust unit tests for label classification and builder dispatch behavior that can run through `npm run cargo:test` without launching the full app.
- [ ] Run targeted tests for changed files.

**Artifacts:**
- `src-tauri/src/shell_windows.rs` (modified)

### Step 2: Migrate panel and preview callers to lazy ensure before first use

- [ ] Update every auxiliary show/publish/resize/preview path that currently does `get_webview_window(AUX_LABEL).ok_or_else(...)` to call the central lazy creation helper before sizing, positioning, showing, focusing, emitting, or hiding where creation is required.
- [ ] Preserve anchor/monitor calculations that depend on existing top/bottom host windows; only auxiliary windows are lazy-created.
- [ ] Ensure hide/closed paths remain idempotent and do not create a window just to hide it unless that behavior is necessary for an existing close/reset contract. If a hide target has not been created yet, emit any required owner close/reset event and return success.
- [ ] Preserve panel open/closed event targets exactly: search close resets top-bar, audio close reaches both top-bar and audio-panel owners when the audio panel exists, terminal open emits `terminal-panel:open`, tray open emits `tray-panel:open`, process manager open emits `process-manager:open`, and existing focus-loss handlers keep their current user-visible behavior.
- [ ] Write or update Node source-contract tests proving these modules call the lazy helper before first show/publish and that hide/focus-loss handlers are safe when the auxiliary window does not yet exist.
- [ ] Run targeted tests for changed files.

**Artifacts:**
- `src-tauri/src/search_panel.rs` (modified)
- `src-tauri/src/stack_popup/popup_window.rs` (modified)
- `src-tauri/src/process_manager.rs` (modified)
- `src-tauri/src/terminal_panel.rs` (modified)
- `src-tauri/src/settings_panel.rs` (modified)
- `src-tauri/src/tray_panel.rs` (modified)
- `src-tauri/src/command_panel.rs` (modified)
- `src-tauri/src/audio_panel.rs` (modified)
- `src-tauri/src/calendar_panel.rs` (modified)
- `src-tauri/src/control_plane.rs` (modified)
- `src-tauri/src/task_preview.rs` (modified)
- `src-tauri/src/main.rs` (modified if focus-loss handling needs non-created-window guards)
- `tests/shellOpenCloseEvents.test.mjs` (modified)
- Additional `tests/*.test.mjs` lifecycle/wiring tests as needed (modified or new)

### Step 3: Verify contracts, capabilities, and startup behavior

- [ ] Add or update tests that assert startup code does not eagerly build auxiliary labels such as `search-panel`, `stack-popup`, `process-manager`, `terminal-panel`, `command-panel`, `audio-panel`, `calendar-panel`, `settings-panel`, `control-plane`, and `task-preview`.
- [ ] Confirm capability files under `src-tauri/capabilities/*` still authorize the same windows/commands and do not require removing existing labels merely because windows are lazy-created.
- [ ] Confirm frontend routing by window label still works through shared `index.html`; do not rename labels or routes.
- [ ] Manually collect Windows smoke metrics before and after the change: startup-created WebView2/native window count, approximate native memory, and time until top/bottom AppBars are visible/reserving work area.
- [ ] Save the measurement method and before/after results as a task document via `fn_task_document_write(key="startup-metrics", content=...)`.
- [ ] Run targeted tests for changed files.

**Artifacts:**
- `tests/*shell*Window*.test.mjs` (new or modified)
- `src-tauri/capabilities/*` (modified only if required to keep existing capability tests green)
- Task document `startup-metrics` (new revision)

### Step 4: Windows smoke coverage for all lazy panels

- [ ] On Windows, smoke-test that the top and bottom bars still start, reserve AppBars, remain topmost, and are excluded from Alt+Tab/task switching as before.
- [ ] Smoke-test first open, second open, hide, and focus-loss cycles for `search-panel`, `stack-popup`, `process-manager`, `settings-panel`, `tray-panel`, `command-panel`, `audio-panel`, `calendar-panel`, `terminal-panel`, `control-plane`, and `task-preview`.
- [ ] Include terminal-specific checks that first opening `terminal-panel` creates/focuses the panel, starts or attaches terminal-panel sessions according to current `TerminalPanelSurface.svelte` behavior, and subsequent hide/show cycles do not lose existing live backend sessions unexpectedly.
- [ ] Record the full smoke checklist and results via `fn_task_document_write(key="windows-smoke", content=...)`.
- [ ] Run targeted tests for changed files.

**Artifacts:**
- Task document `windows-smoke` (new revision)

### Step 5: Testing & Verification

> ZERO test failures allowed. Full test suite as quality gate.
> If keeping lint/tests/build/typecheck green requires edits outside the initial File Scope, make those fixes as part of this task.

- [ ] Run lint/check command (`npm run check`).
- [ ] Run full automated test suite (`npm run test:node` and `npm run cargo:test`).
- [ ] Run project typecheck/build (`npm run build` and `npm run cargo:check`).
- [ ] Run full validation (`npm run validate`).
- [ ] Fix all failures.
- [ ] Build passes.

### Step 6: Documentation & Delivery

- [ ] Update `master_spec.md` to describe that only top/bottom AppBar windows are created at startup and auxiliary surfaces are lazy-created on first use while preserving the same labels/events.
- [ ] Update `changelog.md` according to `CHANGELOG_POLICY.md` with the behavior change, validation performed, and Windows smoke/metrics summary.
- [ ] Save documentation deliverables as task documents via `fn_task_document_write(key="docs", content=...)`, including links or excerpts for the updated docs plus the metrics and smoke document keys.
- [ ] Out-of-scope findings created as new tasks via `fn_task_create` tool.

## Documentation Requirements

**Must Update:**
- `master_spec.md` — add/update shell-window lifecycle text covering startup-created top/bottom AppBars and lazy-created auxiliary surfaces.
- `changelog.md` — record the lazy-window change, validation commands, and before/after metric summary per policy.

**Check If Affected:**
- `README.md` — update only if it currently claims all shell windows/panels are created at startup.
- `docs/*` — update only if a doc describes startup window creation, panel lifecycle, or smoke/measurement procedures.

## Completion Criteria

- [ ] `create_shell_windows` creates only `top-bar` and `bottom-bar` at startup.
- [ ] Every auxiliary panel/preview is created on first open/use and works on subsequent open/hide/focus-loss cycles.
- [ ] Existing window labels, IPC command names, event names, capability intent, and frontend label routing are unchanged.
- [ ] Startup WebView2/native window count, memory, and time-to-visible AppBar measurements are documented before/after.
- [ ] Windows smoke checklist covers all panels and is saved as a task document.
- [ ] Lint/check passing.
- [ ] All tests passing.
- [ ] Typecheck/build passing.
- [ ] Documentation updated.

## Git Commit Convention

Commits at step boundaries. All commits include the task ID:

- **Step completion:** `feat(FN-002): complete Step N — description`
- **Bug fixes:** `fix(FN-002): description`
- **Tests:** `test(FN-002): description`

## Do NOT

- Expand task scope beyond lazy creation and required lifecycle/test/doc fixes.
- Rename any Tauri window label, IPC command, event, route, capability identifier, or frontend surface component.
- Make top/bottom AppBars lazy-created; they must still start immediately and reserve work area.
- Create windows in hide-only paths unless required to preserve an existing event/reset contract.
- Stop terminal sessions merely because the `terminal-panel` is hidden; preserve current terminal-panel session lifecycle.
- Remove existing capability entries, tests, panel modules, exports, or settings as cleanup.
- Skip tests or Windows smoke validation.
- Refuse necessary fixes just because they touch files outside the initial File Scope.
- Commit without the task ID prefix.
- Remove features as "cleanup" — if something seems unused, create a task via `fn_task_create`.

## Changeset Requirements

If this task REMOVES existing functionality (deleting modules, settings, API endpoints, or exports), a changeset file is REQUIRED:
- Create `.changeset/{task-id}-removal.md` explaining what was removed and why
- This is mandatory for any net-negative change (more deletions than additions to existing files)
