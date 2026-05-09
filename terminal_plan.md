# Stack Browser Terminal Implementation Plan

Status: Draft, implementation not started
Source: `stack_browser_terminal_overhaul_plan.md`, Context7 xterm.js docs, Context7 Tauri 2 docs, current `master_spec.md`
Priority: Execute phases in order. Do not start polish phases before launch/geometry/event-stream phases are green.

## Goal

Overhaul the Stack Browser embedded CLI into a reliable developer terminal that can replace routine trips to Windows Terminal or PowerShell for project work.

This plan is written for implementation workers. Each phase includes scope, exact work items, acceptance criteria, tests, and validation gates. Workers should treat acceptance criteria as blockers, not suggestions.

## Worker Rules

- Read `master_spec.md`, `terminal_plan.md`, and `stack_browser_terminal_overhaul_plan.md` before coding.
- Preserve unrelated dirty tree changes.
- Use RED-first tests where practical. Each phase starts with tests unless it explicitly says discovery-only.
- Do not skip ahead. Finish phase validation before the next phase begins.
- Keep terminal profiles enum-bounded. Do not add arbitrary executable-path persistence.
- Keep `stackBrowser.terminalProfile` in Settings, not Stack Browser toolbar chrome.
- Keep Stack Browser cwd/breadcrumb sync as a product invariant.
- Keep PTY/process operations outside long-held shared runtime locks.
- Update `changelog.md` for every implementation slice. Update `master_spec.md` only when behavior, IPC, events, persistence, tests, or known risks actually change.

## Current Relevant Surfaces

Frontend:

- `src/components/StackPopupSurface.svelte`
- `src/components/StackPopupSurface.css`
- `src/lib/stackPopup.ts`
- `src/ipc/commands.ts`
- `src/ipc/events.ts`
- `src/components/SettingsPanelSurface.svelte`
- `src/components/SettingsPanelSurface.css`
- `tests/stackBrowserTerminal.test.mjs`
- `tests/contractsSettings.test.mjs`
- `tests/settingsPanelWiring.test.mjs`
- `tests/persistentSurfaceLifecycle.test.mjs`

Backend:

- `src-tauri/src/stack_popup/terminal.rs`
- `src-tauri/src/stack_popup.rs`
- `src-tauri/src/contracts.rs`
- `src-tauri/src/main.rs`
- `src-tauri/capabilities/stack-popup.json`
- `src-tauri/src/settings.rs`
- `src-tauri/Cargo.toml`

Docs:

- `master_spec.md`
- `changelog.md`
- `docs/smoke-test-windows.md`
- `stack_browser_terminal_overhaul_plan.md`
- `terminal_plan.md`

## Context7 Implementation Notes

xterm.js notes:

- Use `FitAddon.fit()` to compute visible rows/cols, then propagate those dimensions to the backend PTY.
- xterm `onData` should remain the user-input path to the backend PTY.
- xterm `onResize` or fit result should drive terminal resize events.
- Addons should be loaded deliberately: WebGL with context-loss fallback, WebLinks for URL detection, Search for scrollback search, Serialize for controlled snapshot/export, Unicode support where available.
- WebGL can lose context after memory pressure or system resume; fallback must dispose the addon without breaking the terminal.
- Buffer APIs expose active buffer, rows, cells, cursor, viewport, and normal vs alternate buffer. Use this for tests, copy-output, quick select, and command-aware behavior.

Tauri 2 notes:

- Use async commands or `tauri::async_runtime::spawn_blocking` for blocking PTY/process work.
- Use targeted `emit_to` for surface-scoped events when using events.
- For high-volume streaming, Tauri `Channel` is a candidate because it is designed for optimized Rust-to-JS data streaming. If Channel adoption is too broad for Phase 2, use targeted events first and keep Channel as Phase 7 performance work.
- Frontend listeners must have cleanup and disposed guards for persistent webviews.
- Capability files must be updated when new commands are exposed.

## Phase 0: Readiness And Baseline Audit

Status: Completed 2026-05-09

### Objective

Confirm the current terminal state and document exact baseline failures before writing new implementation code.

### Work Items

1. Inspect current terminal code paths.
   - `StackPopupSurface.svelte`: startup state, xterm creation, polling, event listeners, view switching, cleanup.
   - `terminal.rs`: session registry, PTY spawn, reader thread, write path, poll path, stop path, PowerShell launch plan, cwd parser.
   - `stackPopup.ts` and `commands.ts`: command types and wrappers.
   - `contracts.rs` and capability JSON: registered command/event authority.

2. Reproduce the black-launch symptom if possible.
   - Open Stack Browser.
   - Switch to CLI.
   - Capture what is visible at 100ms, 800ms, and 1500ms.
   - Classify symptom as one of:
     - blank xterm but process alive
     - no xterm DOM/canvas
     - prompt output delayed
     - PowerShell exited
     - error hidden outside terminal
     - fit/zero-size renderer

3. Record current validation baseline.
   - Run focused tests if environment permits:
     - `node --test tests/stackBrowserTerminal.test.mjs`
     - `cargo test --manifest-path src-tauri/Cargo.toml terminal`
     - `npm run check`
   - If not run, record why in the phase notes.

4. Confirm Settings Panel terminal profile visibility.
   - Check that `SettingsPanelSurface.svelte` compiles.
   - Confirm `stackBrowser.terminalProfile` selector is visible in Settings JSON shell section.
   - Check for broken section nesting around JSON shell settings and shell-bar settings.

### Acceptance Criteria

- Baseline symptom is classified in `changelog.md` or a short phase note in the PR/worker summary.
- Current command/event/settings surfaces are identified before changes.
- No implementation changes are made in this phase except harmless notes if needed.

### Validation

- At minimum: source inspection plus `git status --short`.
- Preferred:
  - `node --test tests/stackBrowserTerminal.test.mjs`
  - `cargo test --manifest-path src-tauri/Cargo.toml terminal`
  - `npm run check`

### Phase 0 Baseline Result

- Source audit classified the likely black-launch symptom as blank xterm / prompt-delayed / fit-zero risk, not a proven PowerShell process exit. Live WebView2 screenshot capture at 100ms, 800ms, and 1500ms was not run in this pass.
- Current frontend creates xterm with `FitAddon`, but startup has no visible overlay and still reads optional `session.output` even though the Rust start snapshot has no output field.
- Current backend starts ConPTY at fixed `120x30`; there is no `resize_stack_terminal` command, wrapper, contract constant, or capability entry.
- `stack-terminal:output` exists in contracts, but frontend does not listen for it; backend emits it only while polling drains output, so polling remains the normal output path.
- Poll/write overlap remains a baseline risk because frontend interval polling and write-triggered polling can overlap while backend temporarily removes live sessions from the registry during write/poll.
- Settings Panel terminal profile selector is present in the JSON shell settings area, but `npm run check` reports one current Svelte warning: `src/components/SettingsPanelSurface.svelte:300:3` implicitly closes the JSON shell settings `<section>` before `</main>`.
- Baseline validation on 2026-05-09: `git status --short` showed a pre-existing dirty terminal worktree; `node --test tests/stackBrowserTerminal.test.mjs` passed 11/11; `cargo test --manifest-path src-tauri/Cargo.toml terminal` passed 11/11; `npm run check` passed with 0 errors and the one SettingsPanel section nesting warning above.

## Phase 1: RED Tests For Launch, Event Flow, Geometry, And Races

Status: Completed 2026-05-09

### Objective

Write failing tests that prove current weaknesses before fixing them.

### Scope

Add or extend tests only. Do not implement production fixes in this phase.

### Work Items

1. Add terminal startup state tests in `tests/stackBrowserTerminal.test.mjs`.
   - Test that CLI mode has a visible startup/empty/error state while a session is starting or no first PTY byte has arrived.
   - Test that the user-visible state includes profile and cwd or a compact status label, not only an empty black region.
   - Test that frontend does not rely on a nonexistent `session.output` field unless backend contract adds one.

2. Add event-driven output contract tests.
   - Assert frontend listens for `stack-terminal:output`.
   - Assert listener cleanup uses the persistent-surface pattern: synchronous cleanup list plus disposed guard.
   - Assert output chunks are ignored if their `sessionId` does not match the active terminal session.
   - Assert polling remains as watchdog/fallback, not the only normal path.

3. Add resize command contract tests.
   - Assert `src/ipc/commands.ts` includes `resizeStackTerminal: 'resize_stack_terminal'`.
   - Assert `src/lib/stackPopup.ts` exposes typed `resizeStackTerminal(sessionId, cols, rows, pixelWidth, pixelHeight)`.
   - Assert `src-tauri/src/contracts.rs` registers `RESIZE_STACK_TERMINAL`.
   - Assert `src-tauri/capabilities/stack-popup.json` allows the command.
   - Assert `StackPopupSurface.svelte` sends fitted rows/cols after `FitAddon.fit()`.

4. Add backend resize tests in `terminal.rs`.
   - Invalid session id rejects.
   - Rows/cols must be bounded and nonzero.
   - Pixel width/height may be optional or bounded but must not panic.
   - A test-session path can record requested size without requiring a live OS PTY, or a Windows-only ConPTY smoke can run when available.

5. Add poll/write race tests.
   - Source-level test: frontend has an in-flight `pollStackTerminal` guard or queue so interval poll and write-triggered poll cannot overlap.
   - Rust test: session registry should not report not found for a live session merely because a poll/write is in progress, or the command should return a transient-busy result that frontend treats as retryable.

6. Add PowerShell launch-plan tests.
   - Assert launch plan does not silently ignore the trusted PowerShell path.
   - Assert fallback behavior is explicit if PowerShell 7 is unavailable.
   - Preserve previous guard against quoted full path being passed as literal `"C:\Program Files\..."` to `cmd.exe`.

7. Add Settings Panel structure test.
   - Source test that the terminal profile selector exists in Settings Panel.
   - Source or compile test that JSON shell settings section and shell bar section are properly nested/closed.

### Acceptance Criteria

- New tests fail before production changes for the currently missing behavior.
- Test names clearly map to Phase 2 implementation tasks.
- Tests do not rely only on fragile exact text where structural checks are possible.

### Validation

- `node --test tests/stackBrowserTerminal.test.mjs tests/contractsSettings.test.mjs tests/settingsPanelWiring.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml terminal`
- `npm run check` only if Svelte source tests were added or touched.

## Phase 2: P0 Launch Trustworthiness And Black-Screen Fixes

Status: Completed 2026-05-09

### Objective

Make CLI launch never appear as unexplained black. It must show a prompt, startup state, or actionable error.

### Work Items

1. Add terminal lifecycle state in `StackPopupSurface.svelte` or a new terminal component.
   - Suggested states:
     - `idle`
     - `starting`
     - `runningWaitingForFirstByte`
     - `running`
     - `exited`
     - `failed`
   - Track:
     - active `sessionId`
     - profile
     - cwd
     - first byte received
     - last output timestamp
     - startup timeout timer id

2. Add visible startup/empty/error overlay.
   - Show overlay only when no first PTY output has rendered.
   - Overlay content should be compact:
     - "Starting PowerShell..."
     - cwd path
     - profile label
   - If no output after 1200ms, change to "Still starting..." and expose a small diagnostic action or status text.
   - Hide overlay immediately on first output chunk.
   - Do not cover terminal content after first output.

3. Remove or gate startup `Clear-Host`.
   - In `powershell_startup_script()`, stop clearing the screen by default.
   - If aliases are still needed, install aliases without clearing prompt/banner.
   - If clean startup is desired later, add a setting after shell readiness exists.

4. Make start response/session contract honest.
   - Preferred: do not expect `session.output` from `startStackTerminal(...)`; rely on first output event/poll.
   - Alternative: backend may drain and return initial output, but this must be explicit in `StackTerminalSessionSnapshot` type and tests.
   - Update TypeScript types so frontend and Rust agree.

5. Fix PowerShell launch resolution.
   - Use the trusted PowerShell path deterministically.
   - Preserve the known workaround that avoids literal quoted-path failure through `cmd.exe /K`.
   - If `pwsh.exe` is unavailable, fall back to Windows PowerShell path or return an explicit visible error.
   - Do not add arbitrary shell path persistence.

6. Fix first-fit timing.
   - After `terminal.open(...)`, call fit after `tick()`.
   - Retry fit on next animation frame if measured cols/rows are zero.
   - Keep `ResizeObserver`, but do not rely on it as the only first-open correction.
   - Do not swallow fit failures silently in dev diagnostics; record state for startup debugging.

7. Fix frontend poll/write overlap.
   - Add a per-session poll queue or `pollInFlight` guard.
   - A write-triggered poll should await the current poll or schedule one after current poll finishes.
   - Interval poll must skip if another poll is in flight.
   - If backend returns a transient busy state, retry without surfacing an error.

8. Improve post-spawn failure display.
   - If `stack-terminal:closed` or poll returns exited before first byte, show visible error state inside CLI.
   - Include exit status if available.
   - Keep "Restart" action for later Phase 6 if button surface is ready; otherwise describe next action in status text.

9. Update docs.
   - `changelog.md`: implementation summary and validation.
   - `master_spec.md`: update terminal startup behavior, launch state, PowerShell launch contract, and validation map if tests changed.

### Acceptance Criteria

- CLI first open shows either prompt output, startup overlay, or error state within 150ms.
- No-output startup reaches "still starting" state by 1200ms.
- Spawn failure and early process exit never leave blank black content.
- PowerShell 7 missing or launch failure produces visible error.
- Poll/write overlap does not stop terminal polling or leave a false not-found state.
- `stackBrowser.terminalProfile` remains in Settings Panel.
- No arbitrary executable path is persisted.

### Tests

- `node --test tests/stackBrowserTerminal.test.mjs tests/settingsPanelWiring.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml terminal`
- `npm run check`

### Live Smoke

- Open Stack Browser, click CLI, capture visual state at 100ms, 800ms, 1500ms.
- Confirm black screen alone never persists.
- Run `echo hi`, `clear`, `cd ..`, and `pwd`/`Get-Location`.
- Close Stack Browser during startup; confirm no session resurrection.

### Phase 2 Result

- `StackPopupSurface.svelte` now tracks terminal lifecycle, first-output state, startup timeout, profile/cwd status, serialized polling, persistent output/close listeners, and resize-after-fit IPC.
- `StackPopupSurface.css` adds compact startup/error overlay styling that only appears before first PTY output.
- `stackPopup.ts`, `commands.ts`, Rust command registration, contracts, and backend terminal code now include `resize_stack_terminal` / `resizeStackTerminal` with bounded rows/cols and ConPTY resize.
- PowerShell startup no longer runs a final `Clear-Host`, and the `cmd.exe /K` launch path stays tied to trusted PowerShell discovery without passing a quoted full path literal as the command token.
- `SettingsPanelSurface.svelte` now closes the JSON shell settings section before the shell bar section so the terminal profile setting cannot corrupt following controls.

Validation performed:

- `node --test tests/stackBrowserTerminal.test.mjs tests/settingsPanelWiring.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml terminal`
- `npm run check`

Residual risk:

- Live WebView/ConPTY smoke is still pending on a desktop run; automated coverage verifies source contracts and Rust terminal logic but does not prove first-paint timing in the real Tauri webview.

## Phase 3: P1 PTY Geometry, Event Streaming, And Backpressure

Status: Implementation not started

### Objective

Make terminal output and sizing behave like a real terminal under resize and high-output load.

### Work Items

1. Harden backend resize command already introduced in Phase 2.
   - Command name: `resize_stack_terminal`.
   - Request fields:
     - `sessionId: String`
     - `cols: u16` or bounded `u32`
     - `rows: u16` or bounded `u32`
     - optional `pixelWidth`
     - optional `pixelHeight`
   - Validate:
     - session id format
     - rows/cols > 0
     - upper bound such as rows <= 300, cols <= 600 unless xterm fit can exceed this reasonably
   - Phase 2 already calls `MasterPty::resize(PtySize { rows, cols, pixel_width, pixel_height })`.
   - Store current rows/cols in session snapshot metadata.

2. Harden frontend resize wiring already introduced in Phase 2.
   - Phase 2 already sends rows/cols after `FitAddon.fit()`.
   - Phase 2 already coalesces resize fits with `requestAnimationFrame`.
   - Phase 2 already avoids duplicate same-size resize IPC by session/cols/rows/pixel-size key.
   - On terminal open, send initial resize before or immediately after first input.
   - On Stack Browser resize grip drag, ensure resize eventually reaches PTY.

3. Convert output to push-first.
   - Option A: keep reader thread sending chunks into registry and emit with `app_handle.emit_to('stack-popup', 'stack-terminal:output', chunk)`.
   - Option B: use Tauri `Channel` for streaming chunks if command lifecycle supports it cleanly.
   - Phase 3 should choose the smaller safe route. If event volume proves too high, document Channel as Phase 10 follow-up.
   - Frontend listens to `stack-terminal:output` and writes chunks to xterm.
   - Polling becomes watchdog every 1500-3000ms or recovery after missed events.

4. Add output batching.
   - Queue incoming chunks per session.
   - Flush to xterm inside `requestAnimationFrame` or microtask batch.
   - Preserve chunk order by sequence.
   - Drop chunks from old session ids.
   - Keep xterm as source of visual buffer; do not grow unbounded `stackTerminalOutput`.

5. Add first-byte timing.
   - Backend or frontend records first chunk time.
   - Use this to dismiss startup overlay.
   - Use diagnostics only; do not log raw terminal output.

6. Fix backend session access race.
   - Avoid temporary registry removal making live sessions appear missing.
   - Possible designs:
     - session has internal mutex for writer/child/output receiver
     - registry keeps placeholder while operation owns mutable internals
     - frontend serializes all operations and backend returns retryable busy
   - Preferred: backend API should not expose live session as not found due to internal in-progress work.

7. Add process-tree stop hardening.
   - Current `cmd.exe /K pwsh.exe` can create nested process behavior.
   - Evaluate whether `PtyChild::kill()` kills the ConPTY child tree on Windows.
   - If not, add a safe Windows process-tree termination helper for terminal sessions only.
   - Keep guardrails: never kill JasonShell process, never kill arbitrary user PID outside owned terminal session.

8. Update docs.
   - `master_spec.md`: new `resize_stack_terminal` command, event/polling contract, PTY geometry behavior.
   - `changelog.md`: tests and validation.

### Acceptance Criteria

- PTY rows/cols match xterm visible rows/cols after open and after Stack Browser resize.
- Full-screen terminal apps and prompt wrapping use current size.
- Output appears without waiting for 700ms interval.
- High-output commands stay responsive enough for input and close.
- No unbounded JS string stores all output forever.
- Closing during write/poll/resize does not restore a killed session.

### Tests

- `node --test tests/stackBrowserTerminal.test.mjs tests/contractsSettings.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml terminal`
- `npm run check`
- `npm run cargo:check`

### Live Smoke

- Resize Stack Browser while CLI open; run a command that prints long lines and confirm wrapping changes.
- Run `Get-ChildItem -Recurse .` in a medium repo and confirm output streams continuously.
- Press Ctrl+C during output and confirm shell remains usable.
- Close popup during output and confirm child process exits.

## Phase 4: Terminal Component Extraction And xterm Addon Modernization

Status: Implementation not started

### Objective

Move terminal complexity out of the huge Stack Browser component and add modern xterm features safely.

### Work Items

1. Extract terminal-specific frontend into smaller files.
   - Suggested files:
     - `src/components/StackTerminalPane.svelte`
     - `src/components/StackTerminalPane.css`
     - `src/features/stack-browser/terminalState.ts`
     - `src/features/stack-browser/terminalViewModel.ts`
   - Keep parent `StackPopupSurface.svelte` responsible for view mode and folder integration.
   - Terminal component owns xterm instance, addons, event listener cleanup, output queue, resize dispatch, and terminal-specific actions.

2. Preserve integration contracts.
   - Parent passes:
     - current path
     - active terminal profile
     - callbacks for cwd change / open folder
     - close request
   - Child emits/calls:
     - cwd changed
     - terminal started/stopped
     - error state
   - Do not reintroduce separate HTML command input.

3. Add WebGL addon.
   - Add dependency `@xterm/addon-webgl`.
   - Load after xterm opens.
   - On context loss, dispose addon and continue on fallback renderer.
   - Add dev diagnostic state `renderer: 'webgl' | 'default' | 'fallback'`.
   - Do not crash if WebGL unavailable in WebView2.

4. Add WebLinks addon and custom link provider.
   - Add `@xterm/addon-web-links`.
   - Enable safe URL opening behavior through existing shell/open wrapper if needed.
   - Add custom provider for:
     - absolute Windows paths
     - relative file paths with line/column if rooted in current Stack Browser cwd
     - git hashes
     - localhost URLs and ports
   - All actions must validate paths before opening.

5. Add Search addon.
   - Add `@xterm/addon-search`.
   - Add compact search UI inside CLI mode.
   - Keyboard route: Ctrl+F opens terminal search when CLI has focus.
   - Escape closes search first, then popup on second Escape or when no search is open.
   - Search UI must not steal normal terminal input after close.

6. Add Serialize addon if available for installed xterm version.
   - Use for explicit "copy visible terminal", export, or controlled restore only.
   - Do not use Serialize as an unbounded replacement for xterm scrollback.

7. Add Unicode/grapheme smoke.
   - Configure xterm Unicode support where supported by current xterm package.
   - Smoke with emoji, powerline symbols, box drawing, and CJK.

8. Update CSS.
   - Use terminal monospace font stack:
     - `Cascadia Mono`, `Cascadia Code`, `Consolas`, `ui-monospace`, `SFMono-Regular`, `monospace`
   - Use readable default size, target 12.5-14px unless constrained by existing surface size.
   - Keep full-height black work surface.
   - Avoid nested cards.

9. Update docs.
   - `master_spec.md`: component split and addon set.
   - `changelog.md`: added dependencies and validation.

### Acceptance Criteria

- `StackPopupSurface.svelte` is smaller and no longer owns all xterm internals.
- WebGL loads when available and falls back on context loss.
- Links are clickable and safe.
- Scrollback search works entirely inside CLI mode.
- Terminal remains usable with WebGL disabled/unavailable.
- Fonts are readable and monospace by default.

### Tests

- Source tests for new component ownership and imports.
- Source tests for WebGL context-loss handler.
- Source tests for WebLinks/Search addon wiring.
- Link provider unit tests for URL/path/hash detection and path validation.
- `node --test tests/stackBrowserTerminal.test.mjs`
- `npm run check`
- `npm run build`

### Live Smoke

- Open CLI, search scrollback with Ctrl+F.
- Click a localhost URL and a safe local path.
- Suspend/resume or force WebGL context loss if feasible; terminal should survive.
- Print Unicode sample:
  - `Write-Output "emoji 😀 box ┌─┐ powerline  CJK 日本語"`

## Phase 5: Shell Integration, Cwd Truth, And Command Marks

Status: Implementation not started

### Objective

Make the terminal understand prompts, commands, outputs, exit codes, and cwd using shell integration instead of input guessing.

### Work Items

1. Define supported shell integration protocols.
   - Support OSC 133:
     - prompt start
     - command start
     - command executed/output start
     - command finished with exit code
   - Support cwd sequences:
     - OSC 1337 current dir where practical
     - VS Code-style cwd sequence only if parser can safely detect it and it does not break other terminals
   - Keep unknown escape sequences ignored by xterm as normal.

2. Decide parser location.
   - Preferred frontend path: xterm parser hooks observe OSC sequences and update command model.
   - Backend path only if ordering or security requires it.
   - Do not strip sequences needed by terminal apps unless xterm parser consumes them safely.

3. Add PowerShell integration script.
   - Static script content generated by backend or bundled resource.
   - Emits prompt/command markers and cwd.
   - Captures exit code.
   - Must not leak secrets.
   - Must be opt-out through future settings; default can be on only after smoke is green.

4. Add Git Bash/bash integration script.
   - Uses prompt command/preexec-style hooks where available.
   - Emits same marker model.
   - Handles login interactive shell.
   - Must not permanently mutate user shell files.

5. Add command model.
   - Store command records:
     - id
     - session id
     - command text if known
     - start marker
     - output start marker
     - end marker
     - exit code
     - cwd
     - timestamps
   - Use xterm markers/decorations where possible.
   - Keep bounded command history per session.

6. Replace cwd authority.
   - Shell integration cwd is authoritative when present.
   - Input-inferred cwd remains fallback.
   - Failed `cd` must not update Stack Browser cwd.
   - `pushd`, `popd`, aliases/functions, and prompt-level cwd changes should sync if shell integration reports them.

7. Add command navigation.
   - Jump previous/next command.
   - Select/copy command output.
   - Show exit success/failure decoration.
   - Keep behavior graceful when no command markers exist.

8. Update docs.
   - `master_spec.md`: shell integration protocol, cwd source priority, command marker behavior.
   - `changelog.md`: implementation and validation.

### Acceptance Criteria

- PowerShell commands produce command records with exit status.
- Git Bash commands produce command records where supported.
- Failed `cd` does not move Stack Browser path.
- Successful prompt-reported cwd updates Stack Browser path/breadcrumbs.
- User can jump previous/next command.
- User can copy output for a command without drag-select.
- Terminal remains useful when shell integration is disabled or unsupported.

### Tests

- Unit tests for OSC parser fixtures.
- Source tests for shell integration injection and opt-out setting.
- Tests for command record reducer/model.
- Cwd priority tests:
  - shell marker wins
  - failed cd ignored
  - input parser fallback works when no marker
- `node --test tests/stackBrowserTerminal.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml terminal`
- `npm run check`

### Live Smoke

- PowerShell:
  - `pwd`
  - `cd ..`
  - failed `cd does-not-exist`
  - `exit 7` in a subshell or command returning nonzero
- Git Bash:
  - `pwd`
  - `cd ..`
  - failed cd
- Confirm command marks, cwd sync, and output copy.

## Phase 6: Developer Ergonomics And Terminal Actions

Status: Implementation not started

### Objective

Add the features that make the embedded CLI faster for daily developer work than leaving JasonShell.

### Work Items

1. Add terminal action registry.
   - Define typed actions:
     - copy selection
     - copy command
     - copy command output
     - rerun command
     - search
     - clear
     - open cwd in Files
     - open external terminal here
     - restart terminal
     - stop terminal
   - Actions must be state-gated.
   - Destructive actions require clear user intent.

2. Add compact terminal command surface.
   - Use icon buttons where possible.
   - Keep toolbar dense and not explanatory.
   - Suggested visible controls:
     - search
     - split/session menu placeholder if Phase 7 not done
     - open external
     - restart/stop menu
   - Avoid visible text that explains how to use terminal.

3. Add right-click context menu.
   - Context-aware options:
     - copy
     - paste
     - copy command output
     - rerun command
     - open file/path
     - open cwd in Files
     - open external terminal
   - Keep native/Svelte menu placement inside Stack Browser viewport.
   - Validate any path before opening.

4. Add Quick Select.
   - Detect:
     - URLs
     - Windows absolute paths
     - relative paths from cwd
     - file:line and file:line:column
     - localhost ports
     - git hashes
     - branch names where context is known
   - Overlay short labels.
   - Typing label copies or opens based on mode.
   - Escape cancels.

5. Add recent command/directory UI.
   - Source from command records, not raw unbounded output.
   - Keep bounded per-session history.
   - Do not persist command history until privacy policy is decided.
   - Provide rerun and cd/open actions.

6. Add quick fixes.
   - Pattern examples:
     - port already in use -> open Process Manager focused to process or suggest kill only if process id known and guardrails apply
     - git upstream missing -> suggest `git push --set-upstream origin <branch>`
     - command not found -> suggest search or install path only if reliable
     - test failure file path -> open file / reveal in Stack Browser
   - Quick fixes must be suggestions, not automatic execution.

7. Add terminal-to-Stack Browser bridge actions.
   - "Reveal cwd in Files"
   - "Open selected path in Stack Browser"
   - "Open cwd in VS Code"
   - "Run Git status" or open Git workbench for current repo.

8. Update docs.
   - `master_spec.md`: action registry, quick select, recent command policy.
   - `changelog.md`.

### Acceptance Criteria

- User can copy a command output block without mouse drag-selection.
- User can search and open/copy URLs/paths from output.
- User can rerun recent commands.
- Right-click menu is reachable at popup edges.
- Quick fixes never execute destructive commands automatically.
- No command history is persisted unless explicitly approved in a later phase.

### Tests

- Unit tests for Quick Select pattern matching.
- Source tests for terminal action registry and menu wiring.
- Tests for path validation before open actions.
- Tests for quick-fix parsing and non-execution default.
- `node --test tests/stackBrowserTerminal.test.mjs tests/contextMenuPosition.test.mjs`
- `npm run check`

### Live Smoke

- Run command that prints URL and path; Quick Select can copy/open.
- Run failing `git push` without upstream in test repo; quick fix appears but does not auto-run.
- Right-click near bottom of Stack Browser; menu remains reachable.

## Phase 7: Sessions, Splits, Persistence Policy, And Workbench Layout

Status: Implementation not started

### Objective

Make CLI mode a real terminal workbench with multiple terminal sessions and optional split panes.

### Work Items

1. Design session model.
   - Session fields:
     - session id
     - title
     - profile
     - cwd
     - running
     - createdAt
     - lastOutputAt
     - rows/cols
     - command count
   - Keep backend session cap or make cap configurable with safe upper bound.

2. Add session list UI.
   - Compact tabs or sidebar inside CLI mode.
   - Show profile/cwd/running state.
   - Allow new session in current folder.
   - Allow rename title.
   - Allow stop/restart.

3. Add split panes.
   - Support at least two panes:
     - vertical split
     - horizontal split
   - Each pane owns one xterm instance and one backend session.
   - Resize panes sends PTY resize to each visible terminal.
   - Focus indicators must be clear.

4. Add session preservation behavior.
   - Setting: preserve terminal sessions when switching to Files.
   - Default should be conservative. If uncertain, default preserve only while Stack Browser stays open, stop on popup close.
   - Do not persist live sessions across app restart until there is a real restore model.

5. Add external terminal handoff.
   - Open current session cwd in Windows Terminal / external terminal using existing safe launch paths.
   - Do not transfer raw shell state; only cwd/profile where safe.

6. Add backend process tree cleanup.
   - Ensure all owned sessions stop on Stack Browser close when not preserved.
   - Ensure all owned sessions stop on app exit.
   - Confirm child process cleanup for nested `cmd.exe /K pwsh.exe`.

7. Update docs.
   - `master_spec.md`: session model, preservation policy, split behavior, cleanup.
   - `changelog.md`.

### Acceptance Criteria

- User can run two terminal sessions side by side.
- Switching Files/CLI follows configured preservation behavior.
- Closing Stack Browser cleans up sessions according to policy.
- Each split has correct PTY size.
- Session controls are keyboard reachable and do not obscure terminal output.

### Tests

- Frontend state tests for session list, active pane, split layout.
- Source tests for cleanup on close/destroy.
- Rust tests for multi-session cap and cleanup.
- `node --test tests/stackBrowserTerminal.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml terminal`
- `npm run check`

### Live Smoke

- Create two sessions.
- Run long command in one, interactive command in another.
- Resize split and Stack Browser.
- Close popup; confirm owned processes exit when policy says stop.

## Phase 8: Appearance, Accessibility, And Settings

Status: Implementation not started

### Objective

Make terminal readable, configurable, accessible, and consistent with JasonShell design rules.

### Work Items

1. Add terminal settings schema.
   - Extend `stackBrowser` settings or add `terminal` subsection.
   - Suggested settings:
     - default profile
     - font family
     - font size
     - line height
     - cursor style
     - cursor blink
     - scrollback
     - theme id
     - copy on selection
     - right click behavior
     - shell integration enabled
     - preserve sessions while popup open
     - startup diagnostics overlay enabled
   - Version or default the settings safely.

2. Add Settings Panel controls.
   - Keep in Settings Panel JSON shell area.
   - Use existing Melt wrappers where they fit.
   - Do not add terminal profile controls back to Stack Browser toolbar.
   - Validate numeric bounds in frontend and backend.

3. Add themes.
   - Start with a few built-in themes:
     - JasonShell dark
     - Windows Terminal dark-like
     - high contrast
   - Keep color values readable.
   - Avoid one-note palettes.

4. Improve font defaults.
   - Use monospace stack by default.
   - Default font size should be readable at default Stack Browser size.
   - Ensure line height does not clip powerline/emoji.

5. Add accessibility path.
   - Labels for terminal toolbar actions.
   - Keyboard-only access for search, quick select, session switch, split switch, close/restart.
   - Announce terminal state changes where practical:
     - starting
     - first output ready
     - command failed
     - process exited
   - Keep xterm helper textarea usable; do not hide focus in a way that breaks input.

6. Add reduced-motion and high-contrast checks.
   - Avoid animated overlays that ignore reduced motion.
   - High-contrast theme should pass readable contrast.

7. Update docs.
   - `master_spec.md`: settings schema and persistence.
   - `changelog.md`.

### Acceptance Criteria

- Terminal uses readable monospace default.
- Settings Panel owns terminal customization.
- Invalid settings are clamped or rejected safely.
- Keyboard-only user can operate terminal workflows.
- High contrast theme is usable.
- No text overlaps or clipped controls at default and minimum Stack Browser sizes.

### Tests

- `tests/contractsSettings.test.mjs`
- `tests/settingsPanelWiring.test.mjs`
- `tests/stackBrowserTerminal.test.mjs`
- `tests/frontendUiPolicy.test.mjs` if layout/design policy touched.
- `npm run check`
- Rust settings tests:
  - `cargo test --manifest-path src-tauri/Cargo.toml settings`

### Live Smoke

- Change terminal font size, profile, and theme; reopen CLI.
- Keyboard-only path through Settings and CLI.
- Narrow/minimum Stack Browser size visual check.

## Phase 9: Performance Benchmarks And Stress Harness

Status: Implementation not started

### Objective

Prevent future terminal regressions in throughput, memory, first paint, and input latency.

### Work Items

1. Add benchmark definitions.
   - Time to first terminal visual state.
   - Time to first PTY byte.
   - Output throughput chunks/sec and bytes/sec.
   - Input latency while output is streaming.
   - Memory after large scrollback.
   - Resize-to-PTY propagation latency.

2. Add deterministic output fixtures.
   - PowerShell:
     - loop output 10k lines
     - long-line wrapping
     - Unicode output
     - clear screen
   - Node script fixture if easier for deterministic output.
   - Rust test helper for backend chunk ordering.

3. Add live smoke script.
   - New doc section or script under `docs/smoke-test-windows.md`.
   - Include:
     - first launch
     - warm launch
     - high output
     - resize
     - Ctrl+C
     - close during output
     - Unicode
     - search
     - link click

4. Add non-live source/perf tests where possible.
   - Output queue batches writes.
   - No unbounded output string accumulation.
   - Sequence ordering preserved.
   - Old session chunks ignored.

5. Add manual thresholds.
   - Suggested initial thresholds:
     - visible startup state <= 150ms
     - first output or startup timeout transition <= 1200ms
     - no UI freeze > 250ms under 10k line output
     - input remains accepted during streaming output
   - Treat thresholds as baseline; tune after first measurements.

6. Update docs.
   - `docs/smoke-test-windows.md`
   - `master_spec.md` validation coverage map.
   - `changelog.md`.

### Acceptance Criteria

- There is a focused terminal smoke checklist.
- Source tests guard against unbounded output accumulation.
- A worker can measure startup, output, resize, close, and input behavior.
- Performance findings are recorded without logging raw terminal output or secrets.

### Tests

- `node --test tests/stackBrowserTerminal.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml terminal`
- `npm run check`
- Live smoke per checklist.

## Phase 10: Documentation, Spec Closure, And Adversarial QA

Status: Final gate after any substantial implementation bundle

### Objective

Close implementation with durable docs and independent QA.

### Work Items

1. Update `master_spec.md`.
   - Current terminal behavior.
   - New commands/events.
   - Settings schema.
   - Session lifecycle.
   - Shell integration protocol.
   - Validation coverage.
   - Known residual risks.

2. Update `changelog.md`.
   - User request.
   - Code changes.
   - QA follow-ups.
   - Tool validation.

3. Update `terminal_plan.md`.
   - Mark completed phases.
   - Add discovered follow-up risks.
   - Do not erase unresolved work.

4. Run adversarial QA.
   - QA should try to break:
     - startup black screen
     - PowerShell missing/failing path
     - overlapping poll/write
     - resize drift
     - WebGL fallback
     - shell integration disabled
     - failed cd cwd sync
     - close during output
     - large output memory
     - keyboard-only operation

5. Fix blocking QA findings.
   - Do not declare complete while in-scope QA blockers remain.

6. Final validation.
   - Focused tests for touched areas.
   - `npm run check`
   - `npm run build` if dependencies/frontend modules changed.
   - `npm run test:node`
   - `cargo test --manifest-path src-tauri/Cargo.toml terminal`
   - `npm run cargo:check`
   - `npm run validate` if scope crosses frontend/backend/contracts/settings.

### Acceptance Criteria

- Durable docs match behavior.
- Changelog records implementation and validation.
- QA blockers are fixed or explicitly documented as out-of-scope with user approval.
- Validation commands pass or failures are explained with exact reason.

## Suggested Worker Assignments

Use these names for lower-level implementation workers:

- `terminal-red-test-worker`: Phase 1 tests.
- `terminal-startup-reliability-worker`: Phase 2 black-screen/startup work.
- `terminal-pty-streaming-worker`: Phase 3 backend resize/event streaming/race fixes.
- `terminal-xterm-component-worker`: Phase 4 component extraction and xterm addons.
- `terminal-shell-integration-worker`: Phase 5 shell markers/cwd/command records.
- `terminal-actions-worker`: Phase 6 quick select/actions/recent commands.
- `terminal-sessions-worker`: Phase 7 sessions/splits/preservation.
- `terminal-settings-accessibility-worker`: Phase 8 settings/appearance/a11y.
- `terminal-performance-qa-worker`: Phase 9 benchmarks/smoke.
- `terminal-adversarial-reviewer`: Phase 10 QA.

## Phase Dependency Summary

| Phase | Blocks | Why |
| --- | --- | --- |
| Phase 0 | all | workers need baseline and exact current failure mode |
| Phase 1 | Phase 2+ | prevents fixing without regression coverage |
| Phase 2 | Phase 3+ | black launch and false dead sessions poison all later work |
| Phase 3 | Phase 4+ | addons/features need correct streaming and PTY size |
| Phase 4 | Phase 5+ | shell integration and actions should sit on stable xterm component |
| Phase 5 | Phase 6 | command-aware ergonomics need command/cwd model |
| Phase 6 | Phase 7 optional | actions can ship before multi-session work |
| Phase 7 | Phase 8 partial | settings may need session preservation controls |
| Phase 8 | Phase 9 | benchmarks should use final-ish UI/settings |
| Phase 10 | release/merge | docs and QA closure |

## Validation Command Reference

Focused:

```powershell
node --test tests/stackBrowserTerminal.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml terminal
npm run check
```

Contract/settings:

```powershell
node --test tests/contractsSettings.test.mjs tests/settingsPanelWiring.test.mjs tests/stackBrowserTerminal.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml settings
cargo test --manifest-path src-tauri/Cargo.toml contracts
```

Broader:

```powershell
npm run test:node
npm run build
npm run cargo:check
npm run validate
```

Use broader validation when a phase changes dependencies, settings schema, IPC contracts, Tauri capability files, or cross-surface behavior.

## Do Not Do In Early Phases

- Do not add arbitrary shell executable paths.
- Do not persist command history before privacy/product approval.
- Do not remove cwd sync.
- Do not move terminal profile selector into Stack Browser toolbar.
- Do not hand-roll a terminal emulator.
- Do not make WebGL mandatory.
- Do not replace xterm with a custom `<pre>`/`input` terminal.
- Do not log raw terminal output in diagnostics.
- Do not allow quick fixes to run destructive commands automatically.
- Do not claim done from source tests alone when behavior requires live WebView2/ConPTY smoke.
