---
goal: Standalone JasonTerm Svelte Tauri Rust Terminal Application Plan
version: 1.0
date_created: 2026-06-08
last_updated: 2026-06-08
owner: JasonTerm implementation agent
status: 'Planned'
tags: [architecture, feature, terminal, svelte, tauri, rust]
---

# Introduction

![Status: Planned](https://img.shields.io/badge/status-Planned-blue)

This plan defines deterministic implementation steps for creating `C:/dev/jasonterm` as a standalone Svelte 5.55.2 + Tauri 2.10.x + Rust terminal application that preserves JasonShell's xterm/ConPTY quality while making tabs into workspaces and each workspace into an unlimited tmux-like pane tree.

## 1. Requirements & Constraints

- **REQ-001**: Create a new standalone repository at `C:/dev/jasonterm`; do not implement inside `C:/dev/jasonshell`.
- **REQ-002**: Use the same framework family and UI libraries as JasonShell: Svelte 5.55.2, TypeScript, Vite, Tauri 2.10.x, Rust edition 2021, xterm 6.0.0 with Fit/Search/Serialize/Unicode11/WebLinks/WebGL addons, and `melt` 0.44.0.
- **REQ-003**: Use Rust `portable-pty` 0.8.1 on Windows ConPTY for shell sessions.
- **REQ-004**: JasonTerm must run as its own OS process and must not depend on JasonShell being open.
- **REQ-005**: Initial UI must be a minimal rectangular developer terminal window with a top tab/workspace strip, split controls, and pane chrome that does not overlay xterm viewports.
- **REQ-006**: Tabs are workspaces. Each workspace owns a persistent pane tree plus one active pane id.
- **REQ-007**: A new workspace starts with exactly one full-window terminal pane.
- **REQ-008**: Split right creates a recursive row split beside the active pane. Split down creates a recursive column split below the active pane.
- **REQ-009**: Pane splitting must be unlimited by application policy; no two-pane or four-session cap may be introduced.
- **REQ-010**: New panes must become active immediately after backend session creation, must display cwd quickly, and must accept typed input without waiting for poll/read roundtrips.
- **REQ-011**: Workspace switching must preserve hidden workspace pane state, backend sessions, cwd, output sequence dedupe state, and active pane id.
- **REQ-012**: Each workspace must have exactly one active pane at a time.
- **REQ-013**: Preserve JasonShell terminal behavior: transparent PTY input, no frontend echo, `convertEol: false`, xterm `windowsPty: { backend: 'conpty' }`, bounded replay buffers, requestAnimationFrame output flushing, push-first output events, poll fallback, and resize dedupe.
- **REQ-014**: Preserve PowerShell/Git Bash/Windows Terminal profile behavior where practical, including trusted executable plans and shell integration markers.
- **REQ-015**: Future JasonShell integration must be prepared but not implemented as an in-process webview. Define a stable external launch/focus IPC boundary for future pin/embed/drag-out support.
- **CON-001**: Do not port JasonShell top bar, bottom bar, AppBar reservation, Stack Browser, process manager, audio, search, calendar, or shell replacement behavior.
- **CON-002**: Do not persist live terminal sessions across app restart in the first implementation.
- **CON-003**: Do not store arbitrary shell executable paths in settings; use safe enum profiles.
- **CON-004**: Do not block Tauri async command threads with PTY reads, shell process waits, or filesystem-heavy work.
- **CON-005**: Keep JasonTerm source paths under `C:/dev/jasonterm` and use JasonShell files only as source references to port or adapt.
- **SEC-001**: IPC commands must validate session ids, workspace ids, pane ids, resize bounds, write byte limits, cwd paths, and target labels.
- **SEC-002**: External integration commands must not execute arbitrary command lines; they may accept cwd/profile/workspace hints only after validation.
- **GUD-001**: Prefer pure state reducers for workspace/pane-tree transitions before Svelte wiring.
- **GUD-002**: Preserve xterm ownership per visible pane; dispose only frontend xterm instances for hidden workspaces, never backend PTY sessions.
- **PAT-001**: Port terminal behavior from `C:/dev/jasonshell/src/components/TerminalPanelSurface.svelte` into smaller JasonTerm modules instead of copying one monolithic component unchanged.

## 2. Implementation Steps

### Implementation Phase 1

- GOAL-001: Bootstrap the standalone repository and pin framework versions.

| Task | Description | Completed | Date |
| -------- | --------------------- | --------- | ---- |
| TASK-001 | Create `C:/dev/jasonterm/package.json` with scripts `dev`, `build`, `check`, `test:node`, `cargo:check`, `cargo:test`, and `validate`; pin dependencies `svelte` `5.55.2`, `@tauri-apps/api` `2.10.x`, `@tauri-apps/cli` `2.10.x`, `@xterm/xterm` `6.0.0`, xterm addons matching JasonShell major-compatible versions, `melt` `0.44.0`, TypeScript, Vite, and Svelte plugin versions compatible with Svelte 5.55.2. |  |  |
| TASK-002 | Create `C:/dev/jasonterm/src-tauri/Cargo.toml` with package name `jasonterm`, Rust edition `2021`, Tauri `2.10.x`, `tauri-build` `2.10.x`, `portable-pty` `0.8.1`, `serde`, `serde_json`, `base64`, and Windows dependencies needed for ConPTY launch helpers. |  |  |
| TASK-003 | Create base frontend files `C:/dev/jasonterm/index.html`, `C:/dev/jasonterm/src/main.ts`, `C:/dev/jasonterm/src/App.svelte`, `C:/dev/jasonterm/src/app.css`, `C:/dev/jasonterm/tsconfig.json`, `C:/dev/jasonterm/tsconfig.node.json`, `C:/dev/jasonterm/tsconfig.test.json`, `C:/dev/jasonterm/vite.config.ts`, and `C:/dev/jasonterm/svelte.config.js`. |  |  |
| TASK-004 | Create Tauri files `C:/dev/jasonterm/src-tauri/tauri.conf.json`, `C:/dev/jasonterm/src-tauri/build.rs`, `C:/dev/jasonterm/src-tauri/src/main.rs`, and `C:/dev/jasonterm/src-tauri/capabilities/default.json`; set product name `JasonTerm`, identifier `com.jnev1.jasonterm`, one main window label `main`, CSP equivalent to JasonShell without shell-surface asset allowances not needed by JasonTerm. |  |  |
| TASK-005 | Create `C:/dev/jasonterm/README.md`, `C:/dev/jasonterm/master_spec.md`, `C:/dev/jasonterm/changelog.md`, and `C:/dev/jasonterm/CHANGELOG_POLICY.md` describing standalone scope, non-goals, and validation commands. |  |  |

### Implementation Phase 2

- GOAL-002: Port and isolate the Rust PTY backend from JasonShell into JasonTerm-specific contracts.

| Task | Description | Completed | Date |
| -------- | --------------------- | --------- | ---- |
| TASK-006 | Port terminal backend logic from `C:/dev/jasonshell/src-tauri/src/stack_popup/terminal.rs` into `C:/dev/jasonterm/src-tauri/src/terminal/pty.rs`; rename public models from `StackTerminal*` to `JasonTerm*`; keep `portable-pty`, ConPTY master lifetime retention, UTF-8 split-safe reads, bounded output queue, session snapshots, sequence numbers, cwd updates, and close events. |  |  |
| TASK-007 | Create `C:/dev/jasonterm/src-tauri/src/terminal/profiles.rs` by adapting JasonShell `TerminalProfile` and launch-plan code from `C:/dev/jasonshell/src-tauri/src/settings.rs` and `terminal.rs`; support enum values `windowsTerminal`, `gitBash`, and `powershell`; reject arbitrary executable paths. |  |  |
| TASK-008 | Create `C:/dev/jasonterm/src-tauri/src/contracts.rs` with command constants `start_terminal_session`, `list_terminal_sessions`, `read_terminal_session`, `write_terminal_session`, `resize_terminal_session`, `stop_terminal_session`, `rename_terminal_session`, `stop_all_terminal_sessions`, and event constants `terminal:output`, `terminal:cwd`, `terminal:closed`, `terminal:integration-request`. |  |  |
| TASK-009 | Create `C:/dev/jasonterm/src-tauri/src/terminal/commands.rs` exposing Tauri commands that wrap `pty.rs`; enforce `MAX_SESSION_ID_LEN = 48`, `MAX_WRITE_BYTES = 16 * 1024`, cols range `2..=500`, rows range `1..=300`, and optional pixel dimensions `1..=32767`. |  |  |
| TASK-010 | Register managed state and commands in `C:/dev/jasonterm/src-tauri/src/main.rs`; on app exit call `stop_all_terminal_sessions` so no PTY child is intentionally orphaned. |  |  |
| TASK-011 | Add Rust unit tests in `C:/dev/jasonterm/src-tauri/src/terminal/pty.rs` and `profiles.rs` for session id validation, resize validation, cwd normalization, trusted PowerShell path plan, Git Bash candidate handling, and bounded write rejection. |  |  |

### Implementation Phase 3

- GOAL-003: Implement pure frontend workspace and pane-tree state before xterm rendering.

| Task | Description | Completed | Date |
| -------- | --------------------- | --------- | ---- |
| TASK-012 | Port and rename `C:/dev/jasonshell/src/features/terminal/terminalWorkbenchState.ts` to `C:/dev/jasonterm/src/features/workspace/workspaceState.ts`; model `JasonTermWorkspace`, recursive `PaneTreeNode`, `PaneModel`, `activePaneId`, and owned session ids. |  |  |
| TASK-013 | Extend `workspaceState.ts` with pure functions `createWorkspacePlan`, `activateWorkspacePlan`, `closeWorkspacePlan`, `splitActivePanePlan`, `closePanePlan`, `focusNextPanePlan`, and `focusPreviousPanePlan`; ensure each plan returns next workspace list, active workspace id, active pane id, sessions to start, sessions to stop, and sessions to keep. |  |  |
| TASK-014 | Add `C:/dev/jasonterm/tests/workspaceState.test.mjs` covering one-pane workspace creation, split right/down recursion, unlimited repeated splits, workspace switch preservation, active-pane uniqueness, closing active pane, closing active workspace, stale split rejection inputs, and flattening owned session ids. |  |  |
| TASK-015 | Create `C:/dev/jasonterm/src/features/workspace/paneIds.ts` with deterministic frontend id helpers using prefixes `workspace-`, `pane-`, `split-`, and a monotonic counter scoped to the renderer runtime. |  |  |

### Implementation Phase 4

- GOAL-004: Implement xterm pane runtime modules preserving JasonShell terminal quality.

| Task | Description | Completed | Date |
| -------- | --------------------- | --------- | ---- |
| TASK-016 | Create `C:/dev/jasonterm/src/lib/ipc/commands.ts`, `events.ts`, and `terminal.ts`; adapt wrappers from `C:/dev/jasonshell/src/ipc/commands.ts`, `src/ipc/events.ts`, `src/lib/persistentTerminal.ts`, and `src/lib/stackPopup.ts` using JasonTerm command/event names only. |  |  |
| TASK-017 | Port `C:/dev/jasonshell/src/features/stack-browser/terminalShellIntegration.ts` to `C:/dev/jasonterm/src/features/terminal/shellIntegration.ts` without Stack Browser dependencies; keep OSC 133, 1337, and 633 parser/reducer behavior and command records. |  |  |
| TASK-018 | Port terminal helpers from `C:/dev/jasonshell/src/features/terminal/terminalActions.ts`, `terminalHistory.ts`, `terminalQuickSelect.ts`, `terminalTabTitle.ts`, and selected safe path helpers from `C:/dev/jasonshell/src/features/stack-browser/terminalViewModel.ts` into `C:/dev/jasonterm/src/features/terminal/`. |  |  |
| TASK-019 | Create `C:/dev/jasonterm/src/features/terminal/outputBuffer.ts` to own bounded per-session replay buffers with 256 KiB retained output, stream-aware sequence dedupe keys, and replay-after-attach behavior. |  |  |
| TASK-020 | Create `C:/dev/jasonterm/src/features/terminal/paneRuntime.ts` to own xterm `Terminal`, `FitAddon`, `SearchAddon`, optional `WebglAddon`, `SerializeAddon`, `Unicode11Addon`, and `WebLinksAddon` per visible pane; include requestAnimationFrame output flushing, write queue, resize observer, resize dedupe, poll fallback, listener cleanup, and disposed/runtime-id guards based on `C:/dev/jasonshell/src/components/TerminalPanelSurface.svelte`. |  |  |
| TASK-021 | Add `C:/dev/jasonterm/tests/terminalOutputBuffer.test.mjs`, `terminalShellIntegration.test.mjs`, `terminalQuickSelect.test.mjs`, and `paneRuntimeState.test.mjs` for pure behavior and runtime guard functions. |  |  |

### Implementation Phase 5

- GOAL-005: Build the minimal rectangular Svelte UI and wire workspace operations.

| Task | Description | Completed | Date |
| -------- | --------------------- | --------- | ---- |
| TASK-022 | Create `C:/dev/jasonterm/src/components/JasonTermApp.svelte` that loads the first workspace on mount, renders `WorkspaceTabs.svelte`, `TerminalToolbar.svelte`, and `WorkspacePaneTree.svelte`, and owns global keyboard routing. |  |  |
| TASK-023 | Create `C:/dev/jasonterm/src/components/WorkspaceTabs.svelte` with rectangular tabs, close buttons, new workspace button, active workspace indication, and no wrapping that obscures controls. |  |  |
| TASK-024 | Create `C:/dev/jasonterm/src/components/TerminalToolbar.svelte` with Melt-backed action buttons copied from the pattern in `C:/dev/jasonshell/src/components/melt/MeltActionButton.svelte`; include `Split right`, `Split down`, `New workspace`, `Rename workspace`, `Close pane`, and `Restart pane`. |  |  |
| TASK-025 | Create `C:/dev/jasonterm/src/components/WorkspacePaneTree.svelte` that recursively renders split containers with `min-width: 0`, `min-height: 0`, visible gutters, and no viewport overlays. |  |  |
| TASK-026 | Create `C:/dev/jasonterm/src/components/TerminalPane.svelte` that binds a pane host to `paneRuntime.ts`, focuses active pane on pointerdown, shows a thin title/cwd chrome row above xterm, and sends all xterm `onData` directly to the active backend session write queue. |  |  |
| TASK-027 | Create `C:/dev/jasonterm/src/components/JasonTermApp.css` and `TerminalPane.css`; port xterm typography from `C:/dev/jasonshell/src/components/TerminalPanelSurface.css`: Cascadia/Consolas stack, `letterSpacing: 0`, default 13px, line-height 1.25, ligatures disabled, full-size xterm screen rows, hidden assistive mirror layout, and no forced shell CSS height. |  |  |
| TASK-028 | Wire split operations so clicking `Split right` or `Split down` starts a backend session for the active pane cwd, rejects stale completions if workspace/pane generation changed, inserts the pane immediately when the session returns, focuses it, fits it, and accepts input without requiring `read_terminal_session`. |  |  |
| TASK-029 | Wire workspace switching so hidden workspace xterm instances may be disposed to save DOM resources, but backend sessions, output buffers, rendered sequence keys, pane tree, and active pane id remain in memory and reattach on return. |  |  |

### Implementation Phase 6

- GOAL-006: Add standalone app behavior and future JasonShell integration boundary.

| Task | Description | Completed | Date |
| -------- | --------------------- | --------- | ---- |
| TASK-030 | Create `C:/dev/jasonterm/src-tauri/src/integration.rs` with safe commands `focus_or_create_workspace`, `open_workspace_at_cwd`, and `get_integration_contract`; inputs may include cwd, profile enum, and optional title only. |  |  |
| TASK-031 | Add CLI/deep-link planning documentation in `C:/dev/jasonterm/master_spec.md` for future JasonShell top-bar pin/embed/drag-out: JasonShell launches or focuses the JasonTerm process and sends cwd/profile/title hints; no JasonShell webview embeds JasonTerm internals in this phase. |  |  |
| TASK-032 | Configure Tauri single-instance behavior if available in Tauri 2.10.x for JasonTerm; otherwise document that duplicate-process coordination is deferred and keep all commands safe when multiple JasonTerm processes exist. |  |  |
| TASK-033 | Create `C:/dev/jasonterm/src/features/integration/integrationContract.ts` containing a typed frontend mirror of the integration command payloads and version string `jasonterm.integration.v1`. |  |  |

### Implementation Phase 7

- GOAL-007: Validate behavior, document implementation, and prepare release-quality checks.

| Task | Description | Completed | Date |
| -------- | --------------------- | --------- | ---- |
| TASK-034 | Add Node tests under `C:/dev/jasonterm/tests/*.test.mjs` for workspace reducers, tab title generation, shell integration parsing, quick select detection, output buffer dedupe, command/action gating, and integration contract shape. |  |  |
| TASK-035 | Add Rust tests under `C:/dev/jasonterm/src-tauri/src/terminal/` for profile plans, command validation, session registry behavior, output sequence monotonicity, and stop cleanup. |  |  |
| TASK-036 | Run `npm run check` in `C:/dev/jasonterm` and fix all Svelte/TypeScript diagnostics. |  |  |
| TASK-037 | Run `npm run test:node` in `C:/dev/jasonterm` and fix all failures. |  |  |
| TASK-038 | Run `cargo test --manifest-path C:/dev/jasonterm/src-tauri/Cargo.toml` and `cargo check --manifest-path C:/dev/jasonterm/src-tauri/Cargo.toml`; fix all failures. |  |  |
| TASK-039 | Run `npm run build` and `npm run tauri build` in `C:/dev/jasonterm`; verify the app starts, creates one workspace, splits right/down repeatedly, switches workspaces without state loss, and accepts immediate typing in new panes. |  |  |
| TASK-040 | Update `C:/dev/jasonterm/master_spec.md` with final command names, event names, pane-tree invariants, validation commands, known risks, and non-goals; update `C:/dev/jasonterm/changelog.md` according to `CHANGELOG_POLICY.md`. |  |  |

## 3. Alternatives

- **ALT-001**: Implement JasonTerm as another JasonShell Tauri webview. Rejected because the requirement states JasonTerm must be standalone and run in its own process.
- **ALT-002**: Fork `TerminalPanelSurface.svelte` unchanged into JasonTerm. Rejected because the source is monolithic and includes JasonShell top-bar, Stack Browser, and panel-specific concerns that must be removed for maintainability.
- **ALT-003**: Use a browser-only pseudo terminal without ConPTY. Rejected because Codex, PowerShell PSReadLine, full-screen TUIs, shell history, and tab completion require a real PTY.
- **ALT-004**: Persist live sessions across restart. Rejected for initial implementation because JasonShell currently uses a conservative non-restoring policy and live shell restoration has high correctness risk.
- **ALT-005**: Cap panes per workspace. Rejected because the requirement explicitly says split right/down unlimited.

## 4. Dependencies

- **DEP-001**: `C:/dev/jasonterm/package.json` must pin Svelte `5.55.2`, Tauri API/CLI `2.10.x`, xterm `6.0.0`, xterm addons, `melt` `0.44.0`, TypeScript, Vite, and Svelte tooling.
- **DEP-002**: `C:/dev/jasonterm/src-tauri/Cargo.toml` must include Tauri `2.10.x`, `tauri-build` `2.10.x`, `portable-pty` `0.8.1`, `serde`, `serde_json`, and Windows support crates.
- **DEP-003**: Phase 2 depends on JasonShell source references `C:/dev/jasonshell/src-tauri/src/stack_popup/terminal.rs`, `terminal_panel.rs`, `contracts.rs`, and `settings.rs`.
- **DEP-004**: Phase 4 depends on JasonShell source references `C:/dev/jasonshell/src/components/TerminalPanelSurface.svelte`, `TerminalPanelSurface.css`, `src/lib/persistentTerminal.ts`, `src/features/stack-browser/terminalShellIntegration.ts`, and `src/features/terminal/*.ts`.
- **DEP-005**: Phase 5 depends on Phase 3 pure workspace reducers and Phase 4 pane runtime modules.
- **DEP-006**: Phase 6 integration boundary depends on final command names from Phase 2 and workspace model names from Phase 3.

## 5. Files

- **FILE-001**: `C:/dev/jasonterm/package.json` - frontend dependencies and validation scripts.
- **FILE-002**: `C:/dev/jasonterm/src-tauri/Cargo.toml` - Rust dependencies and package metadata.
- **FILE-003**: `C:/dev/jasonterm/src-tauri/tauri.conf.json` - standalone JasonTerm Tauri window and security config.
- **FILE-004**: `C:/dev/jasonterm/src-tauri/src/main.rs` - Tauri bootstrap, state registration, command registration, shutdown cleanup.
- **FILE-005**: `C:/dev/jasonterm/src-tauri/src/contracts.rs` - command/event constants.
- **FILE-006**: `C:/dev/jasonterm/src-tauri/src/terminal/pty.rs` - ConPTY session registry and PTY I/O implementation.
- **FILE-007**: `C:/dev/jasonterm/src-tauri/src/terminal/profiles.rs` - safe profile enum and executable launch plans.
- **FILE-008**: `C:/dev/jasonterm/src-tauri/src/terminal/commands.rs` - Tauri terminal commands and validation.
- **FILE-009**: `C:/dev/jasonterm/src-tauri/src/integration.rs` - future JasonShell external integration commands.
- **FILE-010**: `C:/dev/jasonterm/src/main.ts` - Svelte app mount.
- **FILE-011**: `C:/dev/jasonterm/src/App.svelte` - shell for `JasonTermApp.svelte`.
- **FILE-012**: `C:/dev/jasonterm/src/components/JasonTermApp.svelte` - top-level workspace UI orchestration.
- **FILE-013**: `C:/dev/jasonterm/src/components/WorkspaceTabs.svelte` - rectangular workspace tabs.
- **FILE-014**: `C:/dev/jasonterm/src/components/TerminalToolbar.svelte` - minimal split/workspace toolbar.
- **FILE-015**: `C:/dev/jasonterm/src/components/WorkspacePaneTree.svelte` - recursive split layout renderer.
- **FILE-016**: `C:/dev/jasonterm/src/components/TerminalPane.svelte` - xterm pane host and active-pane focus.
- **FILE-017**: `C:/dev/jasonterm/src/features/workspace/workspaceState.ts` - pure workspace and pane-tree reducers.
- **FILE-018**: `C:/dev/jasonterm/src/features/workspace/paneIds.ts` - deterministic renderer id helpers.
- **FILE-019**: `C:/dev/jasonterm/src/features/terminal/paneRuntime.ts` - per-pane xterm runtime ownership.
- **FILE-020**: `C:/dev/jasonterm/src/features/terminal/outputBuffer.ts` - bounded replay and dedupe.
- **FILE-021**: `C:/dev/jasonterm/src/features/terminal/shellIntegration.ts` - shell marker parser/reducer.
- **FILE-022**: `C:/dev/jasonterm/src/lib/ipc/commands.ts` - frontend command constants.
- **FILE-023**: `C:/dev/jasonterm/src/lib/ipc/events.ts` - frontend event constants.
- **FILE-024**: `C:/dev/jasonterm/src/lib/ipc/terminal.ts` - frontend terminal IPC wrappers.
- **FILE-025**: `C:/dev/jasonterm/master_spec.md` - durable JasonTerm behavior specification.
- **FILE-026**: `C:/dev/jasonterm/changelog.md` - per-change history.
- **FILE-027**: `C:/dev/jasonshell/src-tauri/src/stack_popup/terminal.rs` - source file to port from; do not modify for JasonTerm.
- **FILE-028**: `C:/dev/jasonshell/src/components/TerminalPanelSurface.svelte` - source file to port from; do not modify for JasonTerm.

## 6. Testing

- **TEST-001**: `C:/dev/jasonterm/tests/workspaceState.test.mjs` verifies workspace creation, activation, close, split right/down, unlimited recursive splits, active-pane uniqueness, and hidden workspace preservation.
- **TEST-002**: `C:/dev/jasonterm/tests/terminalOutputBuffer.test.mjs` verifies 256 KiB retention, sequence dedupe, stream-aware keys, and replay-after-reattach ordering.
- **TEST-003**: `C:/dev/jasonterm/tests/terminalShellIntegration.test.mjs` verifies OSC 133, 1337, and 633 parsing plus cwd marker authority.
- **TEST-004**: `C:/dev/jasonterm/tests/terminalQuickSelect.test.mjs` verifies safe URL, localhost, Windows path, relative path, file:line, git hash, and branch detection.
- **TEST-005**: `C:/dev/jasonterm/tests/terminalTabTitle.test.mjs` verifies cwd, AI harness, Maven, package tool, and renamed workspace title generation.
- **TEST-006**: `C:/dev/jasonterm/tests/integrationContract.test.mjs` verifies `jasonterm.integration.v1` payloads accept only cwd/profile/title hints.
- **TEST-007**: `cargo test --manifest-path C:/dev/jasonterm/src-tauri/Cargo.toml terminal` verifies PTY validation, trusted profile plans, session registry, resize bounds, write limits, and cleanup.
- **TEST-008**: Manual validation: launch JasonTerm, confirm first workspace has one pane, split right five times, split down five times, type immediately in the newest pane, switch workspaces, return, and confirm all panes retain state and output.
- **TEST-009**: Manual validation: run PowerShell PSReadLine history, tab completion, `codex` or equivalent full-screen TUI, and `npm test` output to confirm xterm/PTTY behavior remains transparent.
- **TEST-010**: Full validation command: run `npm run validate` in `C:/dev/jasonterm` after all implementation tasks are complete.

## 7. Risks & Assumptions

- **RISK-001**: Tauri 2.10.x exact crate/package versions may differ from JasonShell's current broad `2.0.0` ranges; implementation must resolve compatible exact versions during bootstrap.
- **RISK-002**: Copying JasonShell's monolithic terminal component unchanged will import shell-surface assumptions and cause hidden state bugs; implementation must split runtime, state, and UI modules.
- **RISK-003**: Disposing hidden workspace xterm instances can lose visual scrollback if replay buffers or sequence keys are incorrect.
- **RISK-004**: New pane responsiveness can regress if frontend writes wait for polling, fitting, or session reads.
- **RISK-005**: Windows ConPTY startup can fail if the master handle is dropped or PowerShell launch plan changes from the trusted JasonShell pattern.
- **RISK-006**: Unlimited splits can create very small panes; CSS must keep layout stable and resizing bounded even when terminal cells become too small.
- **RISK-007**: Future JasonShell embed/pin/drag-out semantics are not fully specified; this plan implements only a safe external integration contract.
- **ASSUMPTION-001**: The first release targets Windows as the primary platform because the required PTY behavior names Windows ConPTY.
- **ASSUMPTION-002**: Live terminal sessions are preserved only during the current JasonTerm process lifetime.
- **ASSUMPTION-003**: JasonTerm can use one main Tauri window initially; multi-window embedding is deferred.
- **ASSUMPTION-004**: The implementation agent may read and port source from `C:/dev/jasonshell` but must not modify JasonShell except for any separately requested documentation plan files.

## 8. Related Specifications / Further Reading

- `C:/dev/jasonshell/master_spec.md`
- `C:/dev/jasonshell/src/components/TerminalPanelSurface.svelte`
- `C:/dev/jasonshell/src/components/TerminalPanelSurface.css`
- `C:/dev/jasonshell/src-tauri/src/stack_popup/terminal.rs`
- `C:/dev/jasonshell/src-tauri/src/terminal_panel.rs`
- `C:/dev/jasonshell/src/features/terminal/terminalWorkbenchState.ts`
- `C:/dev/jasonshell/src/features/stack-browser/terminalShellIntegration.ts`
- `C:/dev/jasonshell/src/features/terminal/terminalActions.ts`
- `C:/dev/jasonshell/src/features/terminal/terminalQuickSelect.ts`
- `C:/dev/jasonshell/src/features/terminal/terminalTabTitle.ts`
- `C:/dev/jasonshell/tests/persistentTerminalPanel.test.mjs`
- `C:/dev/jasonshell/tests/terminalWorkbenchState.test.mjs`
- `C:/dev/jasonshell/tests/stackBrowserTerminal.test.mjs`
- Tauri 2 documentation for windows, commands, events, capabilities, and single-instance support.
- xterm.js 6 documentation for Terminal options and Fit/Search/Serialize/Unicode11/WebLinks/WebGL addons.
- portable-pty 0.8.1 documentation for ConPTY session lifetime and resizing.
