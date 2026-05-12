# Task: FN-003 - Add measured terminal-panel prewarm/idle policy

**Created:** 2026-05-12
**Size:** M

## Review Level: 2 (Plan and Code)

**Assessment:** This changes terminal startup timing across the hidden persistent terminal-panel surface and its ConPTY lifecycle, so regressions could affect shell availability and first-open behavior. The implementation is reversible and should stay within existing terminal-panel/session patterns without adding new security-sensitive shell launch inputs.
**Score:** 4/8 — Blast radius: 2, Pattern novelty: 1, Security: 0, Reversibility: 1

## Mission

Reduce JasonShell cold-start/idle overhead from the hidden terminal panel while keeping first terminal open latency bounded and preserving existing terminal tabs, splits, restart, ConPTY, xterm, resize, and event routing semantics. Today `src/components/TerminalPanelSurface.svelte` starts xterm and calls `startPersistentTerminal()` during hidden webview mount; this task replaces that eager hidden startup with a measured delayed/idle/on-first-open policy, captures before/after metrics, and documents the latency/resource tradeoff.

## Frontend UX Criteria

- [ ] **Design tokens only** — no hardcoded `px` values except `0`, no hardcoded hex/rgb colors; use CSS custom properties (`--color-*`, `--spacing-*`, etc.)
- [ ] **Icon sizing** — match the surrounding component's icon size convention (default lucide size unless the local pattern already uses an explicit `size={N}`)
- [ ] **Semantic color tokens for status** — use `--color-error` for stderr/error states, `--color-warning` for starting/pending states; never hardcode status colors
- [ ] **Component reuse** — reach for existing classes (`.btn`, `.btn-icon`, `.card`, `.input`) before writing one-off styles
- [ ] **Responsive scaffolding** — add `@media (max-width: 768px)` overrides for any new layout; verify mobile usability
- [ ] **Single canonical nav destination** — each route must appear in exactly one of: Header primary nav, Header overflow menu, or MobileNavBar More; no duplicates across all three
- [ ] **Status-indicator dot convention** — use the existing `.status-dot` pattern (size, border, animation) rather than custom dot styling
- [ ] **Visual hierarchy preserved** — new elements must not disrupt heading levels, content flow, or information architecture established in the surrounding page

## Dependencies

- **None**

## Context to Read First

- `AGENTS.md` — repository operating rules; read the referenced user-level agents file if available.
- `master_spec.md` — canonical terminal-panel, terminal backend, persistent surface lifecycle, and documentation expectations.
- `CHANGELOG_POLICY.md` — changelog update rules.
- `src/components/TerminalPanelSurface.svelte` — current hidden-mount xterm/session startup, listeners, tab/split/runtime management, resize-before-input behavior.
- `src/components/TerminalPanelSurface.css` — terminal-panel UI/status styling if startup states need presentation changes.
- `src/lib/persistentTerminal.ts` and `src/lib/stackPopup.ts` — frontend terminal IPC wrappers and session types.
- `src-tauri/src/terminal_panel.rs` — terminal panel show/hide/open event behavior.
- `src-tauri/src/stack_popup.rs` and `src-tauri/src/stack_popup/terminal.rs` — `start_persistent_terminal`, terminal session registry, target-label routing, list/stop/resize/read/write behavior.
- `tests/persistentTerminalPanel.test.mjs` and `tests/stackBrowserTerminal.test.mjs` — source-level terminal behavior coverage.
- Task document `plan` for FN-003 — planning notes saved during triage.

## File Scope

- `src/components/TerminalPanelSurface.svelte`
- `src/components/TerminalPanelSurface.css` (only if startup/idle status presentation changes)
- `src/lib/persistentTerminal.ts` (only if extracting typed startup policy helpers or metrics wrappers is necessary)
- `src-tauri/src/terminal_panel.rs` (only if backend open/show timing instrumentation or event payload changes are necessary)
- `src-tauri/src/stack_popup.rs` (only if adding/adjusting persistent-terminal command behavior is necessary)
- `src-tauri/src/stack_popup/terminal.rs` (only if backend metrics/tests require small helper exposure; do not change shell launch semantics unnecessarily)
- `tests/persistentTerminalPanel.test.mjs`
- `tests/stackBrowserTerminal.test.mjs` (only if existing terminal contract expectations need updates)
- `docs/terminal-panel-prewarm-idle-policy.md` (new)
- `master_spec.md`
- `changelog.md`

## Steps

### Step 0: Preflight

- [ ] Required files and paths exist.
- [ ] Dependencies satisfied.
- [ ] Read the FN-003 task document `plan` with `fn_task_document_read` and incorporate any useful details.
- [ ] Confirm the current eager startup path: `onMount` in `TerminalPanelSurface.svelte` calls `void startTerminal()`, `startTerminal()` ensures xterm and calls `startPersistentTerminal()` when no backend terminal-panel session exists, and `tests/persistentTerminalPanel.test.mjs` currently asserts that startup behavior.
- [ ] Preserve unrelated dirty worktree changes.

### Step 1: Capture Baseline Metrics and Existing Behavior

- [ ] Record current eager-prewarm baseline numbers for cold app startup/hidden idle and first terminal open latency before code changes. At minimum capture: approximate startup idle memory delta, relevant process/thread count observations, time from app launch to hidden terminal session creation if observable, and first terminal open-to-ready latency.
- [ ] Use repeatable local commands/manual observations appropriate for this Windows/Tauri app, such as Task Manager/Process Explorer or PowerShell `Get-Process` snapshots plus timestamped terminal open observations; note machine/context limitations.
- [ ] Save the baseline notes in `docs/terminal-panel-prewarm-idle-policy.md` and also as a task document via `fn_task_document_write(key="docs", content=...)`.
- [ ] Run the existing focused terminal tests before implementation to verify the baseline source contracts are currently green or document pre-existing failures.

**Artifacts:**
- `docs/terminal-panel-prewarm-idle-policy.md` (new)

### Step 2: Implement Delayed/Idle/First-Open Startup Policy

- [ ] Replace hidden-mount eager terminal startup in `TerminalPanelSurface.svelte` with a clear policy: do not create xterm or call `startPersistentTerminal()` immediately on hidden `onMount`; schedule a bounded delayed/idle prewarm instead, and start immediately when `terminal-panel:open` or window focus indicates explicit first-open user intent.
- [ ] Use a small, testable policy shape with named constants for the idle/delay timing and explicit states such as not-started, scheduled, starting, waiting, running, failed, or exited; avoid scattering raw timeouts.
- [ ] Ensure timer cleanup follows the project persistent-surface lifecycle pattern: synchronous `onMount` cleanup, disposed guard/listener cleanup, and no delayed callback after component destroy/HMR can start a PTY.
- [ ] Make first-open behavior race-safe: if the user opens while an idle prewarm timer is pending, cancel the pending timer and start exactly one terminal session/runtime; if a backend terminal-panel session already exists, reattach/list it instead of starting another.
- [ ] Preserve visible-open UX: on first open, status text should explain startup/waiting rather than blank output, xterm should be created and focused, `handlePanelOpen()` should still fit/resend PTY geometry, and first PTY input must still await visible resize through the existing `ensureVisibleResizeBeforeInput()` path.
- [ ] Run targeted tests for changed files.

**Artifacts:**
- `src/components/TerminalPanelSurface.svelte` (modified)
- `src/components/TerminalPanelSurface.css` (modified, if needed)

### Step 3: Preserve Terminal Tabs, Splits, Restart, and Backend Semantics

- [ ] Verify and, if necessary, adjust tab/split entry points so explicit user actions (`newSession`, split pane, restart) continue to call `startPersistentTerminal()` immediately after user intent and do not depend on the idle prewarm timer.
- [ ] Preserve backend target-label routing to `terminal-panel`, sequenced output dedupe, retained hidden-tab replay buffers, polling fallback, shell integration, cwd updates, and `stop_terminal_panel_sessions` app-exit cleanup.
- [ ] Do not add arbitrary executable/profile settings. If adding a user-facing toggle is judged necessary, use the existing settings patterns and document why; otherwise keep this as an internal measured policy.
- [ ] Add or update real automated tests covering that `TerminalPanelSurface.svelte` no longer calls `startTerminal()` from hidden mount, starts from `terminal-panel:open`/first-open handling, schedules/cancels idle prewarm safely, and still allows explicit tab/split/restart session creation.
- [ ] Run targeted tests for changed files.

**Artifacts:**
- `src/components/TerminalPanelSurface.svelte` (modified)
- `src/lib/persistentTerminal.ts` (modified, only if needed)
- `src-tauri/src/terminal_panel.rs` (modified, only if needed)
- `src-tauri/src/stack_popup.rs` (modified, only if needed)
- `src-tauri/src/stack_popup/terminal.rs` (modified, only if needed)
- `tests/persistentTerminalPanel.test.mjs` (modified)
- `tests/stackBrowserTerminal.test.mjs` (modified, only if needed)

### Step 4: Measure After Metrics and Document Tradeoff

- [ ] Re-run the same baseline measurement procedure after implementation and record before/after resource and latency numbers in `docs/terminal-panel-prewarm-idle-policy.md`.
- [ ] Document the chosen policy precisely: delay duration, what counts as first-open user intent, how duplicate starts are prevented, what startup costs are avoided at cold hidden idle, and the observed first-open latency tradeoff.
- [ ] Update `master_spec.md` terminal-panel sections so the canonical behavior no longer claims hidden webview mount immediately starts xterm/ConPTY; describe the new delayed/idle/on-first-open policy and preserved non-restoring session behavior.
- [ ] Append concise `changelog.md` entries under `CHANGELOG_POLICY.md` rules for behavior, tests, metrics, and validation evidence.
- [ ] Save the final documentation/metrics content as a task document via `fn_task_document_write(key="docs", content=...)`.
- [ ] Run targeted tests for changed files.

**Artifacts:**
- `docs/terminal-panel-prewarm-idle-policy.md` (modified)
- `master_spec.md` (modified)
- `changelog.md` (modified)

### Step 5: Testing & Verification

> ZERO test failures allowed. Full test suite as quality gate.
> If keeping lint/tests/build/typecheck green requires edits outside the initial File Scope, make those fixes as part of this task.

- [ ] Run Svelte/type check (`npm run check`).
- [ ] Run build/typecheck (`npm run build`).
- [ ] Run full Node test suite (`npm run test:node`).
- [ ] Run Rust tests (`npm run cargo:test`).
- [ ] Run Rust check (`npm run cargo:check`).
- [ ] Run full project validation (`npm run validate`).
- [ ] Fix all failures.
- [ ] Perform manual PTY/xterm smoke on Windows: launch app, observe no immediate hidden terminal ConPTY/shell before policy trigger, open terminal panel and confirm prompt appears within documented latency bound, type commands, resize panel, create tab, split pane, switch/close tabs, restart terminal, run a simple full-screen TUI if available, hide/show panel, and confirm no duplicate sessions or lost output.
- [ ] Build passes.

### Step 6: Documentation & Delivery

- [ ] Ensure `docs/terminal-panel-prewarm-idle-policy.md`, `master_spec.md`, and `changelog.md` are updated and consistent.
- [ ] Save documentation deliverables as task documents via `fn_task_document_write` (key="docs", content=...).
- [ ] Out-of-scope findings created as new tasks via `fn_task_create` tool.

## Documentation Requirements

**Must Update:**
- `docs/terminal-panel-prewarm-idle-policy.md` — add baseline and after metrics, measurement method, chosen policy, first-open latency bound, resource tradeoff, and manual smoke evidence.
- `master_spec.md` — update current terminal-panel behavior to describe delayed/idle/on-first-open startup instead of eager hidden mount startup.
- `changelog.md` — append concise entries under `CHANGELOG_POLICY.md` for behavior, tests, metrics, and validation.

**Check If Affected:**
- `README.md` — update only if it documents terminal-panel startup/performance behavior.
- `src-tauri/capabilities/terminal-panel.json` — update only if commands/events are added or changed.

## Completion Criteria

- [ ] All steps complete.
- [ ] Cold-start/hidden-idle baseline and after measurements are recorded with methodology and limitations.
- [ ] Terminal panel no longer eagerly starts xterm/ConPTY on hidden mount; it starts on first open or bounded idle prewarm without duplicate sessions.
- [ ] Existing terminal tab/split/restart/session semantics are preserved.
- [ ] Manual PTY/xterm smoke passes.
- [ ] Lint/check passing.
- [ ] All tests passing.
- [ ] Typecheck passing (if available).
- [ ] Documentation updated and saved as task document.

## Git Commit Convention

Commits at step boundaries. All commits include the task ID:

- **Step completion:** `feat(FN-003): complete Step N — description`
- **Bug fixes:** `fix(FN-003): description`
- **Tests:** `test(FN-003): description`

## Do NOT

- Expand task scope beyond terminal-panel startup/prewarm/idle policy and its metrics/docs.
- Skip tests.
- Refuse necessary fixes just because they touch files outside the initial File Scope.
- Commit without the task ID prefix.
- Remove, delete, or gut modules, settings, interfaces, exports, or test files outside the File Scope.
- Remove features as "cleanup" — if something seems unused, create a task via `fn_task_create`.
- Reintroduce the removed Stack Browser embedded CLI or move terminal ownership away from `TerminalPanelSurface.svelte`.
- Break ConPTY/xterm transparent I/O, sequenced output dedupe, resize-before-input, retained hidden-tab replay, shell integration markers, or terminal-panel target-label event routing.
- Add arbitrary executable paths, raw shell command settings, or persistence of live terminal sessions across app restart.
- Start multiple persistent terminal sessions from an idle/first-open race.

## Changeset Requirements

If this task REMOVES existing functionality (deleting modules, settings, API endpoints, or exports), a changeset file is REQUIRED:
- Create `.changeset/fn-003-removal.md` explaining what was removed and why
- This is mandatory for any net-negative change (more deletions than additions to existing files)
