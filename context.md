# Code Context

## Files Retrieved
1. `master_spec.md` (lines 25-30, 40, 77, 991-1004) - current terminal ownership, backend/event contract, Phase 7 metadata/session commands.
2. `src/components/TerminalPanelSurface.svelte` (lines 220-365, 1138-1200, 1540-1604) - active persistent terminal UI, tabs, new session/split, frontend 4-session gate.
3. `src-tauri/src/stack_popup/terminal.rs` (lines 22-24, 160-199, 338-383, 590-633, 649-718) - backend ConPTY registry, hard 4-session cap, target-label session listing, output routing.
4. `src-tauri/src/stack_popup.rs` (lines 530-653) - Tauri command wrappers, `start_persistent_terminal` target label, list/rename/stop panel sessions.
5. `src/lib/stackPopup.ts` (lines 107-137, 592-629) - frontend terminal session/chunk types and IPC wrappers.
6. `src/lib/persistentTerminal.ts` (lines 1-32) - persistent terminal API re-export surface.
7. `tests/persistentTerminalPanel.test.mjs` (lines 21-184) - source-contract tests for terminal-panel wiring, xterm ownership, per-pane runtimes, event routing.

## Key Code

Backend cap:
```rust
// src-tauri/src/stack_popup/terminal.rs:22
pub(crate) const MAX_STACK_TERMINAL_SESSIONS: usize = 4;

// lines 160-162
pub(crate) fn can_start_session(&self) -> bool {
    self.sessions.len() < MAX_STACK_TERMINAL_SESSIONS
}

// lines 352-354 / 370-376
if !runtime.terminal_sessions.can_start_session() {
    return Err(format!("Stack Browser terminal sessions are limited to {MAX_STACK_TERMINAL_SESSIONS}"));
}
```

Frontend cap:
```ts
// src/components/TerminalPanelSurface.svelte:1159
canCreateSession: terminalSessions.length < 4,
```

Tab/session creation:
```ts
// src/components/TerminalPanelSurface.svelte:271-333
async function refreshTerminalSessionList() {
  terminalSessions = await listStackTerminals('terminal-panel').catch(() => session ? [session] : []);
}

async function createTerminalSession() {
  const nextSession = await startPersistentTerminal();
  terminalSessions = [...terminalSessions, nextSession];
  const runtime = ensurePrimaryPaneForSession(nextSession);
  activateTerminalSession(nextSession);
  ...
}
```

Pane eviction/orphan risk:
```ts
// src/components/TerminalPanelSurface.svelte:278-283
if (terminalPanes.length >= 2) {
  const [removedPane] = terminalPanes;
  if (removedPane) {
    removePaneRuntime(removedPane.paneId, true);
  }
}
```
`removePaneRuntime(..., true)` attempts backend stop, but it is async/fire-and-forget elsewhere and can hide failures.

Backend target scoping:
```rust
// src-tauri/src/stack_popup/terminal.rs:195-199
fn list_by_target(&self, target_label: Option<&str>) -> Vec<StackTerminalSessionSnapshot> {
  self.sessions.values()
    .filter(|session| target_label.map(|label| session.target_label == label).unwrap_or(true))
    .map(StackTerminalSession::snapshot)
```

Persistent terminal target:
```rust
// src-tauri/src/stack_popup.rs:530-552
pub async fn start_persistent_terminal(...) -> Result<StackTerminalSessionSnapshot, String> {
  ...
  target_label: Some(crate::shell_windows::TERMINAL_PANEL_LABEL.to_string()),
}
```

Tabs UI:
```svelte
<!-- src/components/TerminalPanelSurface.svelte:1582-1597 -->
<div class="terminal-session-tabs" role="tablist" aria-label="Terminal sessions">
  {#each terminalSessions as terminalSession, index}
    <button role="tab" aria-selected={terminalSession.sessionId === session?.sessionId}
      on:click={() => activateTerminalSession(terminalSession)}>
      {terminalSession.title || `Session ${index + 1}`} <small>{terminalSession.running ? '●' : '○'}</small>
    </button>
  {/each}
</div>
```

## Architecture
- One shared backend registry lives in `StackPopupRuntimeState.terminal_sessions` through `src-tauri/src/stack_popup/terminal.rs`.
- `start_persistent_terminal` is just `start_stack_terminal_session` with `target_label = "terminal-panel"` and default cwd under `%USERPROFILE%`.
- Backend output/cwd/closed events emit to `target_label`; `terminal-panel` listens and routes chunks by `sessionId` to a per-pane runtime.
- Frontend maintains two related lists:
  - `terminalSessions`: tab/header list from `listStackTerminals('terminal-panel')` plus local appends.
  - `terminalPanes` / `paneRuntimes`: visible xterm instances. Current implementation allows only up to 2 panes and may stop/evict a pane when activating another session.
- Tabs are not pure Windows Terminal-style tab shells yet: switching to a session without a visible pane can create a pane and evict/stop another backend session. This couples tabs to panes and can kill hidden sessions.

## Current cap/orphan risks
- Hard backend global cap: `MAX_STACK_TERMINAL_SESSIONS = 4` applies to all stack/persistent terminal sessions, not only visible terminal-panel tabs.
- Frontend duplicate cap: `canCreateSession: terminalSessions.length < 4` disables New Session before backend is consulted.
- Race risk: backend checks cap before spawn and again after spawn; if cap is hit after spawn, it kills the just-created child. Good cleanup, but still global cap behavior.
- Orphan risk A: terminal-panel startup always calls `startPersistentTerminal()` before/while refreshing list. If backend already has terminal-panel sessions, startup can create an extra session rather than attaching to existing one first.
- Orphan risk B: `refreshTerminalSessionList()` only updates `terminalSessions`; it does not reconcile `terminalPanes`/`paneRuntimes` for sessions that exist backend-side but have no visible tab/runtime, nor does it warn on backend sessions not represented in tabs.
- Orphan risk C: `ensurePrimaryPaneForSession()` evicts first pane when pane count >= 2 and calls `removePaneRuntime(..., true)`. If user only clicked another tab, hidden session may be stopped instead of preserved like Windows Terminal.
- Orphan risk D: `removePaneRuntime`/cleanup uses async `stopStackTerminal(...).catch(...)` debug logging in some paths, so failed cleanup can leave backend PTYs running but absent from frontend state.
- Orphan risk E: closed event only updates runtime if `runtimeForSession()` exists; backend closed sessions that have no runtime may remain in `terminalSessions` until refresh/reconcile.

## Suggested test targets
1. `tests/persistentTerminalPanel.test.mjs`
   - Remove source-contract expectation of `terminalSessions.length < 4` and assert no literal 4-session frontend cap remains.
   - Add contract that startup lists/attaches existing `terminal-panel` sessions before creating a new one, or that it reconciles backend sessions into tabs.
   - Add contract that tab switching does not call `stopStackTerminal` / `removePaneRuntime(..., true)` merely because pane count is 2.
2. Rust tests in `src-tauri/src/stack_popup/terminal.rs`
   - Update/remove tests tied to `MAX_STACK_TERMINAL_SESSIONS`.
   - Add registry tests for `list_by_target("terminal-panel")` including sessions that are running but not frontend-visible.
   - Add command-level behavior for closed/stopped sessions and target-scoped stop.
3. IPC/capability source contracts
   - `tests/persistentTerminalPanel.test.mjs` already checks `list_stack_terminals`, `rename_stack_terminal`, `stop_terminal_panel_sessions` capability. Add any new orphan-detection command/event here if introduced.
4. UI behavior tests/source contracts
   - Assert `terminalSessions` is authoritative for tabs and can exceed four.
   - Assert hidden tab sessions remain backend-running when not active/visible.
   - Assert orphan indicator/reconcile path: backend `listStackTerminals('terminal-panel')` sessions not in local state become tabs, or are surfaced for cleanup.

## Start Here
Open `src/components/TerminalPanelSurface.svelte` first. It holds the tab semantics and the frontend 4-session gate. Then open `src-tauri/src/stack_popup/terminal.rs` to remove/reshape the backend global cap and add orphan/session listing behavior.
