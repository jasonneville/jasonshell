# Terminal Blank-Startup Root Cause and Fix Plan

## Scope

User reported recent terminal workspace/tab changes can open the terminal panel without initialized terminal output. Desired behavior: terminal should load immediately when opening a tab/workspace, similar to wmux. This document records the no-code investigation result and concrete fix plan.

No code changes were made during the investigation pass.

## Summary

Primary failure is in `src/components/TerminalPanelSurface.svelte`: the workspace refactor can render pane DOM before a `TerminalPaneRuntime` exists. The Svelte `use:bindPaneHost={pane}` action runs at DOM mount, sees no runtime, returns, and never records the DOM host. Later code creates the runtime after `await tick()`, but the action does not rerun because the action parameter did not change. The runtime therefore keeps `host: null`, `ensureTerminalViewForPane()` returns early, xterm never attaches, and no retained output can render.

Secondary failure: backend terminal sessions do not retain replay output. Reattaching to an existing/prewarmed backend session can produce no prompt/banner because `list_stack_terminals` returns metadata only and `read_stack_terminal` only drains currently queued output.

## Primary Root Cause: Lost Pane Host Binding

### Evidence

`refreshTerminalSessionList()` creates workspaces/panes from backend sessions but does not create pane runtimes:

- `src/components/TerminalPanelSurface.svelte:555-599`

Key lines:

```ts
terminalWorkspaces = pruneWorkspaceSessions(terminalWorkspaces, liveSessionIds);
for (const nextSession of nextSessions) {
  if (nextSession.running === false) continue;
  if (!terminalWorkspaces.some((workspace) => workspaceContainsSession(workspace, nextSession.sessionId))) {
    terminalWorkspaces = [...terminalWorkspaces, createWorkspaceForSession(nextSession)];
  }
}
...
syncActiveWorkspacePanes();
```

Pane output DOM mounts with the Svelte action:

- `src/components/TerminalPanelSurface.svelte:2453-2462`

```svelte
<div
  use:bindPaneHost={pane}
  class="terminal-panel-output"
  role="log"
  aria-label="Terminal output"
  data-pane-id={pane.paneId}
  ...
></div>
```

`attachPaneHost()` returns if the runtime does not already exist:

- `src/components/TerminalPanelSurface.svelte:2184-2186`

```ts
function attachPaneHost(node: HTMLDivElement, pane: TerminalPaneModel) {
  const runtime = paneRuntimes.get(pane.paneId);
  if (!runtime) return;
  runtime.host = node;
  ...
}
```

`showActiveWorkspacePanes()` currently waits for DOM update before ensuring runtimes:

- `src/components/TerminalPanelSurface.svelte:1346-1355`

```ts
async function showActiveWorkspacePanes() {
  syncActiveWorkspacePanes();
  visibleResizeSettled = false;
  await tick();
  for (const pane of terminalPanes) {
    const runtime = runtimeForPaneModel(pane);
    if (!runtime) continue;
    runtime.visibleResizeSettled = false;
    ensureTerminalViewForPane(runtime);
    replayTerminalSessionOutput(runtime);
    startPollingForRuntime(runtime);
  }
  await scheduleFitAfterPanelOpen();
}
```

New runtime starts without a host:

- `src/components/TerminalPanelSurface.svelte:859-863`

```ts
function createPaneRuntime(nextSession: StackTerminalSession, paneId: string): TerminalPaneRuntime {
  return {
    paneId,
    host: null,
    terminal: null,
```

`ensureTerminalViewForPane()` cannot attach xterm without the host:

- `src/components/TerminalPanelSurface.svelte:1004-1005`

```ts
function ensureTerminalViewForPane(runtime: TerminalPaneRuntime) {
  if (!runtime.host) return;
```

### Failure Sequence

1. Terminal panel opens or visible reconciliation runs.
2. `refreshTerminalSessionList()` sees a running backend terminal session, often from idle prewarm or preserved app session.
3. It creates `terminalWorkspaces` and `terminalPanes` but no `paneRuntimes`.
4. Svelte renders current workspace and pane output DOM.
5. `use:bindPaneHost={pane}` runs immediately when node mounts.
6. `attachPaneHost()` calls `paneRuntimes.get(pane.paneId)` and returns because no runtime exists.
7. `showActiveWorkspacePanes()` resumes after `await tick()` and only then calls `runtimeForPaneModel(pane)`.
8. Runtime exists but has `host: null`; the action has already missed its chance to store the DOM node.
9. `ensureTerminalViewForPane()` exits early because `!runtime.host`.
10. xterm never opens in that pane; replay and output writes have no visible target.
11. User sees blank/no initialized terminal output.

### Why Recent Changes Caused It

Earlier code created a runtime/pane before or during the immediate open path. Current workspace code moved to a model where backend sessions create workspace/pane state first, then runtimes are lazily created after `tick()`. That inverted order violates Svelte action timing.

Svelte actions run when an element is mounted. Their `update` callback only runs when the action parameter changes after Svelte updates markup. If the action returns early because required runtime state is missing, it does not automatically rerun when unrelated JS state later gains a runtime.

## Secondary Root Cause: No Backend Replay on Reattach

### Evidence

Backend session snapshot has metadata only, no output replay:

- `src-tauri/src/stack_popup/terminal.rs:70-83`

```rust
pub struct StackTerminalSessionSnapshot {
    pub session_id: String,
    pub profile: TerminalProfile,
    pub cwd: String,
    pub running: bool,
    pub title: String,
    pub created_at: u64,
    pub last_output_at: Option<u64>,
    pub command_count: u32,
    pub cols: u16,
    pub rows: u16,
    pub pixel_width: Option<u16>,
    pub pixel_height: Option<u16>,
}
```

`read_stack_terminal()` only returns chunks drained from the current queue:

- `src-tauri/src/stack_popup/terminal.rs:457-491`
- `src-tauri/src/stack_popup/terminal.rs:844-855`

```rust
let result = poll_stack_terminal_session(app_handle, state, session_id)?;
let output = result
    .chunks
    .iter()
    .map(|chunk| chunk.text.as_str())
    .collect::<String>();
```

```rust
fn drain_terminal_output(session: &mut StackTerminalSession) -> Vec<StackTerminalOutputChunk> {
    let mut chunks = Vec::new();
    while let Ok(message) = session.output_rx.try_recv() {
        ...
        chunks.push(StackTerminalOutputChunk { ... });
    }
    chunks
}
```

### Failure Sequence

1. Backend PTY starts while panel is hidden or before xterm is attached.
2. Shell emits prompt/banner/integration sequences.
3. Frontend listener may miss, not have runtime, or not have a host/xterm yet.
4. Queue may later be drained by poll, or prompt output may have already passed through the frontend memory path.
5. On later workspace reattach, `list_stack_terminals('terminal-panel')` returns only metadata.
6. `read_stack_terminal(sessionId)` returns only currently queued chunks, not retained prompt history.
7. If no fresh output occurs, attached xterm remains visually blank.

## Contributing Fragility: Dedupe Marks Rendered Before Actual xterm Write

### Evidence

`writeTerminalChunkForRuntime()` adds sequence to `runtime.renderedSequences` before calling `writeTerminalOutputForRuntime()`:

- `src/components/TerminalPanelSurface.svelte:2122-2129`

```ts
if (typeof chunk.sequence === 'number') {
  const sequenceKey = `${chunk.sessionId}:${chunk.stream ?? 'stdout'}:${chunk.sequence}`;
  if (runtime.renderedSequences.has(sequenceKey)) return;
  runtime.renderedSequences.add(sequenceKey);
  renderedSequenceKeysBySession = new Map(renderedSequenceKeysBySession).set(chunk.sessionId, new Set(runtime.renderedSequences));
}
writeTerminalOutputForRuntime(runtime, chunk.text);
```

Actual xterm write is optional:

- `src/components/TerminalPanelSurface.svelte:2155`

```ts
runtime.terminal?.write(output);
```

If output arrives before `runtime.terminal` exists, the sequence can be marked rendered even though no visible write occurred. Replay usually compensates through `sessionReplayBuffers`, but sequence/render semantics are conflated and can hide missed output bugs.

## Exact Fix Plan

### Fix 1: Make `attachPaneHost()` create missing runtime

Change `attachPaneHost()` so it does not return just because the runtime is missing. It should call `runtimeForPaneModel(pane)` and then attach the host.

Target behavior:

```ts
function attachPaneHost(node: HTMLDivElement, pane: TerminalPaneModel) {
  const runtime = paneRuntimes.get(pane.paneId) ?? runtimeForPaneModel(pane);
  if (!runtime) return;
  runtime.host = node;
  commitRuntime(runtime);
  ensureTerminalViewForPane(runtime);
  replayTerminalSessionOutput(runtime);
  scheduleFitForRuntime(runtime);
}
```

This is the most direct fix for the permanent lost-host bug.

### Fix 2: Ensure runtimes before `await tick()` in open/workspace paths

Update `showActiveWorkspacePanes()` ordering:

```ts
async function showActiveWorkspacePanes() {
  syncActiveWorkspacePanes();
  for (const pane of terminalPanes) {
    runtimeForPaneModel(pane);
  }
  visibleResizeSettled = false;
  await tick();
  for (const pane of terminalPanes) {
    const runtime = runtimeForPaneModel(pane);
    if (!runtime) continue;
    runtime.visibleResizeSettled = false;
    ensureTerminalViewForPane(runtime);
    replayTerminalSessionOutput(runtime);
    startPollingForRuntime(runtime);
  }
  await scheduleFitAfterPanelOpen();
}
```

Also apply same pre-runtime pattern to `activateTerminalWorkspace()` and any path that creates workspace/pane models before rendering.

### Fix 3: Visible first-open should create workspace/pane immediately

For wmux-like behavior, opening terminal should create visible terminal workspace/pane shell immediately instead of waiting for backend list/start. Current `startTerminalOnce()` waits for `refreshTerminalSessionList()` and `startPersistentTerminal()` before creating the workspace/runtime:

- `src/components/TerminalPanelSurface.svelte:526-546`

Fix approach:

1. On visible `first-open`/user action with no active workspace, create a pending frontend workspace/pane immediately.
2. Render the pane and starting status immediately.
3. Start backend session in parallel.
4. When `startPersistentTerminal()` resolves, replace pending pane session id with real `sessionId` and create/update runtime.
5. Attach xterm, fit, poll, replay.

This avoids black/bootstrap-only visual state while backend shell starts.

### Fix 4: Add backend replay ring for terminal sessions

Add bounded replay retention per backend session, probably 256 KiB to match frontend replay contract.

Implementation options:

- Add `replay: VecDeque` or bounded `String`/bytes field to `StackTerminalSession`.
- Append each reader chunk in `drain_terminal_output()` or reader thread/session state path.
- Return replay through a new attach command, e.g. `attach_stack_terminal(sessionId)`.
- Or extend `read_stack_terminal` with an optional `includeReplay`/`fromSequence` contract.

Expected attach behavior:

1. Frontend asks backend for replay before declaring session visually ready.
2. Backend returns retained chunks/text and latest sequence boundary.
3. Frontend writes replay into xterm, then starts live event/poll handling.

Fallback rule: if attaching to an existing backend session with no frontend buffer and no backend replay, start a fresh CLI rather than showing blank forever.

### Fix 5: Split received vs rendered sequence tracking

Current `renderedSequences` includes chunks that may only have been received, not actually rendered. Fix by splitting state:

- `receivedSequences`: prevents duplicate event/poll processing.
- `renderedSequences`: records only chunks written to an attached xterm or replayed successfully.

At minimum, do not mark a sequence rendered until either:

- `runtime.terminal` exists and `runtime.terminal.write(output)` was called, or
- output was appended to replay and later replayed into an attached xterm.

### Fix 6: Validate stale session reuse

Current logic trusts any `running !== false` session:

- `src/components/TerminalPanelSurface.svelte:533-534`
- `src/components/TerminalPanelSurface.svelte:227-231`

Fix behavior:

- On visible open, verify selected candidate exists in a successful backend `list_stack_terminals('terminal-panel')` result.
- If `readStackTerminal()` returns session-not-found or exited-without-output, evict session/workspace and start replacement CLI once for that visible open.
- Do not let list failure preserve stale sessions as authoritative for visible startup.

## Test Plan

Current tests are mostly source-regex checks and do not simulate runtime ordering. Add behavioral tests or stronger source tests for these flows.

### Must-cover scenarios

1. **Existing backend session before runtime**
   - Given `terminalWorkspaces`/`terminalPanes` exist from `refreshTerminalSessionList()` and `paneRuntimes` is empty.
   - When panel opens and pane DOM mounts.
   - Then `attachPaneHost()` creates runtime, stores host, and calls `ensureTerminalViewForPane()`.

2. **`showActiveWorkspacePanes()` runtime-before-tick ordering**
   - Assert runtimes are ensured before `await tick()`.

3. **Idle prewarm then first visible open**
   - Backend session exists before visible open.
   - Opening panel must attach xterm and render replay/prompt.

4. **Output before host attach**
   - Push output for session before runtime host exists.
   - Later host attach must replay output and not dedupe it away.

5. **Backend reattach replay**
   - Existing backend session with prompt output retained.
   - New frontend/runtime attach receives replay and shows prompt without waiting for new output.

6. **Stale preserved session**
   - Frontend has preserved `running !== false` session not found in backend list/read.
   - Visible open evicts stale session and starts fresh terminal.

### Focused validation commands

Run after implementation:

```bash
npm run check -- --output human
npm run test:node -- --test-name-pattern="terminal"
cargo test --manifest-path src-tauri/Cargo.toml terminal --quiet
```

Also run a manual Tauri smoke if available:

1. Start JasonShell.
2. Open terminal panel immediately before idle prewarm.
3. Confirm workspace/tab appears immediately and terminal starts.
4. Close/hide panel, wait for idle/prewarm, reopen.
5. Confirm existing/prewarmed session attaches with prompt/output visible.
6. Create new workspace with `+`; confirm prompt appears.
7. Split right/down; confirm both panes attach and show prompt.
8. Switch workspaces; confirm xterm/output remains visible.

## Files Most Likely to Change

- `src/components/TerminalPanelSurface.svelte`
  - `attachPaneHost()`
  - `showActiveWorkspacePanes()`
  - `activateTerminalWorkspace()`
  - `startTerminalOnce()` / visible first-open path
  - output dedupe/replay helpers

- `src-tauri/src/stack_popup/terminal.rs`
  - `StackTerminalSession`
  - reader/drain output flow
  - read/attach replay contract

- `src-tauri/src/stack_popup.rs`
  - possible new/extended Tauri command wrapper for terminal attach/replay

- `src/lib/persistentTerminal.ts` / `src/lib/stackPopup.ts`
  - frontend wrapper for new/extended replay attach command

- `tests/persistentTerminalPanel.test.mjs`
  - add source/behavioral regressions for ordering and replay

## Non-goals

- Do not persist terminal workspaces/live sessions across app restart in this fix unless separately approved.
- Do not reintroduce eager hidden xterm/ConPTY startup on app load as the only fix; it hides the race but does not fix host/replay correctness.
- Do not rely solely on polling delay or retry timers; fix ordering and replay semantics.
- Do not remove current split/workspace UX unless necessary.

## Recommended Implementation Order

1. Patch frontend host/runtime ordering:
   - `attachPaneHost()` runtime creation.
   - `showActiveWorkspacePanes()` pre-tick runtime creation.
   - `activateTerminalWorkspace()` same pattern.

2. Add focused tests for lost-host ordering.

3. Patch frontend stale-session/retry behavior.

4. Add backend replay ring + attach/read contract.

5. Patch frontend replay/dedupe semantics.

6. Run focused validation and manual smoke.

## Confidence

High confidence primary bug is the lost-host/runtimes-after-tick issue. It directly explains blank/no initialized output and matches the new workspace changes. Backend replay is a second real correctness gap that will continue to cause blank reattach cases even after host binding is fixed, especially with idle prewarm, renderer reload, missed events, or workspace switching.