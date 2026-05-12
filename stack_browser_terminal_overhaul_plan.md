# Stack Browser Terminal Overhaul Plan

Status: Draft, research/planning only
Source: 2026-05-09 Stack Browser terminal research pass
Priority: P0 for startup correctness and terminal parity; later phases ordered by developer impact

## Goal

Turn the Stack Browser CLI view into a serious developer terminal that can replace jumping out to Windows Terminal or PowerShell for common project work, while preserving JasonShell's advantage: the terminal lives beside file navigation, git state, pinned folders, and shell surfaces.

This is not an implementation pass. This document records the current design, external terminal research, gaps, black-screen hypotheses, tradeoffs, and a phase plan for a future worker.

## Current JasonShell Terminal Snapshot

Current good foundation:

- `src-tauri/src/stack_popup/terminal.rs` uses `portable-pty` with the native Windows ConPTY backend instead of plain pipes. That is the correct base for PSReadLine, arrow keys, tab completion, TTY-sensitive tools, and tools such as Codex that check terminal state.
- `src/components/StackPopupSurface.svelte` uses `@xterm/xterm` plus `@xterm/addon-fit`, so the frontend no longer owns a fake prompt or separate input row.
- Terminal settings are enum-bounded through `stackBrowser.terminalProfile` with `windowsTerminal`, `gitBash`, and `powershell`; no arbitrary executable path is persisted.
- Backend sessions have bounded ids, a max active-session cap, input-size guard, lifecycle cleanup, stop-request tombstones, and mutex boundaries around spawn/write/poll/kill.
- The frontend handles xterm selection copy, Escape close, output replay after view disposal, cwd sync into Stack Browser breadcrumbs, and bottom scroll after output.

Current high-risk gaps:

- PTY dimensions are fixed at `120x30` on spawn. `FitAddon` resizes xterm visually, but there is no backend resize command to call `MasterPty::resize`, so shells and full-screen TUIs can disagree with the visible grid.
- Output is still polled through `readStackTerminal` every 700ms. Backend emits `stack-terminal:output`, but the surface does not listen to that event today, so output latency and burst handling are worse than modern terminals.
- The `startStackTerminal(...)` response is typed as if a session may include output, but the Rust `StackTerminalSessionSnapshot` has no `output` field. First paint therefore resets xterm with an empty buffer and waits for a later poll/event path.
- xterm only loads `FitAddon`. Missing first-class addons/features: WebGL renderer, WebLinks, Search, Unicode/grapheme support tuning, Serialize, clipboard behavior policy, link providers, decorations/markers, and command-aware scroll navigation.
- Current render theme uses very small type and `fontFamily: var(--js-font-sans)` inside xterm. A terminal should default to a real monospace stack and expose profile-level font size, font family, cursor, scrollback, and theme controls.
- Startup can still appear as a black terminal because the visible surface opens before prompt output arrives, because `Clear-Host` intentionally clears early output, because xterm fit can run before the container is measurable, or because PowerShell launch still fails silently enough to leave only an empty black renderer.
- Cwd tracking is command-string inference from input, not shell integration. It misses `pushd`, aliases/functions, scripts that change location, subshell transitions, failed `cd`, and remote shell cases.
- There is no tabs/splits model, no session restore, no task/process awareness, no command block model, no recent command/directory UI, no quick fixes, and no terminal-specific QA benchmark suite.
- Tests are mostly regex/source-shape checks. They do not prove Svelte section nesting, real xterm DOM paint, startup prompt timing, overlapping poll/write behavior, or live ConPTY first prompt rendering.

## External Research Summary

Windows Terminal:

- Baseline modern Windows terminal features are tabs, panes, Unicode/UTF-8, GPU accelerated text rendering, themes, backgrounds, custom actions, and profile launch arguments.
- Shell integration uses OSC 133 prompt/command/output markers to support command marks, scrollbar marks, command navigation, output selection, and recent command suggestions.
- Rendering settings default toward differential screen repaint instead of full repaint, with software rendering as a fallback.
- ConPTY is the Windows-native pseudoconsole boundary. The terminal host owns presentation and input, while the pseudoconsole streams UTF-8 plus virtual terminal sequences.

VS Code terminal:

- VS Code uses shell integration quality levels, command decorations, command navigation, sticky scroll, recent directory/command features, IntelliSense, quick fixes, and accessibility support.
- It supports FinalTerm `OSC 133` and VS Code `OSC 633` sequences, plus current directory OSC sequences.
- Important design lesson: when command structure is known, the terminal becomes navigable and actionable instead of a plain character buffer.

xterm.js:

- xterm.js is the right web-terminal core. It powers VS Code, Hyper, and other web/electron terminals.
- Useful official addon surface includes Fit, WebGL, WebLinks, Search, Serialize, Attach, Clipboard, Unicode, and parser/link/decorator APIs.
- The WebGL addon matters for high-output streams because GPU rendering avoids DOM/canvas bottlenecks.
- Buffer APIs, markers, decorations, custom key handlers, link providers, and parser hooks give us the primitives for command navigation, links, status marks, and copy/export flows.

WezTerm:

- Important features: tabs, panes, multiplexing, local/remote sessions, searchable scrollback, hyperlinks, bracketed paste, SGR mouse reporting, ligatures, color emoji, font fallback, true color, dynamic color schemes, SSH client, serial ports, Kitty/iTerm2/Sixel image protocols, hot-reloaded config.
- Quick Select highlights URLs, paths, git hashes, IPs, and other patterns, then lets the user copy or paste by typing a short prefix. This is high leverage for developer ergonomics.

Ghostty:

- Focuses on native feel, GPU rendering, tabs/splits, large theme library, ligatures, grapheme clustering, Kitty graphics protocol, and shell integration.
- Shell integration features include prompt-aware close confirmation, new terminal in previous cwd, prompt resizing/redraw, command-output selection, jump-to-prompt, prompt cursor styling, alt-click cursor movement, and ssh/sudo wrappers for compatibility.

Kitty:

- Shell integration supports previous/next prompt jumps, viewing last command output in a pager, mouse cursor movement at prompt, cwd/title updates, and shell-aware cursor styling.
- Kitty also pushes terminal application capability through graphics protocols, kittens, remote control, and scriptability.

Warp:

- Warp's key idea is command blocks: commands and outputs become atomic units that can be copied, re-run, shared, bookmarked, navigated, and rendered with status.
- Warp's implementation notes emphasize 60fps output, Rust/native rendering, shell hooks, and a block model instead of one unstructured scroll grid.
- Tradeoff: a block model creates powerful UX, but requires shell integration and careful fallback for background process output, alternate-screen apps, and traditional shell behavior.

Alacritty:

- Alacritty is a useful constraint model: fast OpenGL terminal with sensible defaults, vi mode, search, regex hints, multi-window, and a bias toward integration instead of reimplementing everything.
- Lesson: do not overbuild UX if the core emulator is slow or incompatible.

Tabby:

- Tabby's plugin API exposes profiles, tabs, frontends, session middleware, stream processors, OSC processors, color schemes, and context-menu/decorator extension points.
- Lesson: terminal architecture should have extension seams, not one giant Svelte component owning every feature.

## Competitive Feature Matrix

| Capability | Leaders | JasonShell Now | Gap / Opportunity |
| --- | --- | --- | --- |
| PTY/ConPTY | Windows Terminal, VS Code, WezTerm, Alacritty | Yes, through `portable-pty` | Add resize, richer lifecycle, event streaming |
| Emulator renderer | xterm.js, native GPU engines | xterm.js core only | Add WebGL, Unicode tuning, renderer health fallback |
| Tabs/panes | Windows Terminal, WezTerm, Ghostty, Tabby | No | Add sessions panel, split panes, detach/external open |
| Shell integration | Windows Terminal, VS Code, Ghostty, Kitty, Warp | No real markers | Add OSC 133/633/1337 parser, profile injection |
| Command blocks | Warp, VS Code-ish command decorations | No | Add optional command model over xterm markers |
| Search scrollback | Windows Terminal, VS Code, WezTerm, Alacritty, xterm addon | No | Add SearchAddon UI and keyboard route |
| Links / paths / quick select | VS Code, WezTerm, Kitty, xterm WebLinks | No | Add WebLinks + path/git hash link provider + Quick Select |
| Performance architecture | GPU/diff renderers, async event streams | Polling + xterm DOM/canvas default | Add event-driven output, WebGL, backpressure, benchmarks |
| Cwd tracking | VS Code, Windows Terminal, Kitty, Ghostty | Input inference | Add shell integration cwd sequences; keep inference fallback |
| Modern command help | VS Code quick fixes, Warp AI/workflows | No | Add command result affordances, recent commands, quick fixes |
| Accessibility | VS Code, xterm helper textarea | Basic xterm | Add accessible command buffer, announcements, focus model |
| Customization | Windows Terminal, WezTerm, Ghostty, Alacritty | Profile enum only | Add theme/font/cursor/scrollback/keybinding settings |

## Black Screen Analysis

The black screen should be treated as a launch-state bug until proven otherwise. The current black background is intentional styling, but a useful terminal must show either a prompt, a startup progress state, or a clear error.

Likely causes, ranked:

1. Empty-but-alive startup window. The CLI view creates a black xterm surface before the PTY has emitted prompt output. The start response currently has no output payload, so `restartStackTerminal(...)` resets xterm with `''` and then waits for the 700ms polling path. If PowerShell startup is slow, hidden warmup did not finish, or output polling has not fired, the user sees only black.
   - Benefit of fixing: immediate confidence that the shell is loading.
   - Ramification: need a state overlay that disappears on first byte or confirmed prompt, without covering real terminal output.

2. Startup script clears the screen. `powershell_startup_script()` sends aliases and `Clear-Host`. That can erase banner/prompt transitions and leave the renderer empty until the next prompt redraw lands.
   - Benefit of removing/altering: preserves visible shell startup and error text.
   - Ramification: old startup clutter can return unless we replace `Clear-Host` with better prompt readiness handling.

3. PowerShell launch resolution drift. The backend resolves a trusted PowerShell path, prepends the trusted PowerShell directory to `PATH`, then launches `pwsh.exe` by name through `cmd.exe /K`. If `pwsh.exe` is absent, path setup is wrong, or the shell resolves unexpectedly, the visible result can be no prompt or an early exit.
   - Benefit of fixing: fewer machine-specific black launches.
   - Ramification: quoted absolute PowerShell paths previously caused `cmd` quoting problems, so the fix must preserve the known ConPTY/cmd launch constraints.

4. xterm fit before layout is measurable. `fitStackTerminal()` catches failure, and the `ResizeObserver` later retries, but first open can still produce bad geometry if the hidden/persistent webview reports zero or stale dimensions.
   - Benefit of fixing: fewer blank or mis-sized first paints.
   - Ramification: requires deterministic first-visible fit scheduling and screenshot smoke coverage.

5. Poll/write overlap. Backend temporarily removes a session from the registry during write/poll. Frontend interval polling and write-triggered polling can overlap, which can surface a transient "Terminal session not found", stop polling, and leave a dead-looking black terminal.
   - Benefit of fixing: removes a race that source-shape tests may miss.
   - Ramification: likely needs an in-flight poll/write queue, per-session lock, or command-level serialization.

6. Child process launch failure is not visible enough. Startup errors route to `errorMessage` and reset xterm with the message, but failures after spawn can appear only as sparse output or a closed event.
   - Benefit of fixing: black screen becomes actionable.
   - Ramification: need to distinguish "starting", "running but no prompt yet", "exited", and "spawn failed".

7. PTY/grid mismatch. Fixed backend `120x30` while xterm uses fitted rows/cols can confuse prompt redraw or full-screen apps.
   - Benefit of fixing: better shell prompt rendering and TUI correctness.
   - Ramification: adds a backend command/event and resize throttling.

Black-screen diagnostics to add before fixes:

- Log and surface phase states: `creating-pty`, `spawned`, `first-byte`, `first-prompt-marker`, `first-fit`, `first-poll`, `exited`.
- Add first-byte timeout: if no output within 1200ms, show "Starting PowerShell..." overlay with profile/cwd and spinner.
- Add source test requiring a user-visible startup/empty state while `stackTerminalBusy` or no first output exists.
- Add live smoke: open Stack Browser, click CLI, screenshot at 100ms, 800ms, 1500ms; assert either prompt text, startup overlay, or error text is visible.
- Add overlap test: trigger write and interval poll concurrently; assert no false not-found terminal state and polling continues.
- Add Settings Panel structure check: current audit found suspicious nested section structure around JSON shell settings vs shell bars. Verify `SettingsPanelSurface.svelte` compiles and the terminal profile selector remains visible before implementation.

## Design Direction

Recommended terminal identity:

- Keep it embedded inside Stack Browser, but stop treating it as a tiny utility panel. It should be a developer workbench mode with terminal sessions as first-class objects.
- Keep Files/CLI toggle, but make CLI mode able to expand vertically, split, and persist sessions while the popup remains open.
- Keep xterm.js as the renderer core. Do not hand-roll a terminal emulator.
- Build shell integration as optional enhancement. The terminal must remain useful when markers are unavailable.
- Prefer command-aware features that improve developer flow: command blocks/marks, output copy, quick select, links, recent commands, quick fixes, cwd sync, and git-aware affordances.

Visual principles:

- Terminal type should use a real monospace font stack by default: `Cascadia Mono`, `Cascadia Code`, `Consolas`, `ui-monospace`, `monospace`.
- Default font size should be readable in a developer tool, likely 12.5-14px, with settings.
- The black background is acceptable, but empty black is not. Startup and empty states must be explicit.
- Toolbar should stay compact. Terminal actions should use icon buttons or command palette actions, not explanatory text.
- Avoid card nesting. CLI content should be a full-height work surface inside the Stack Browser.

## Architecture Direction

Backend:

- Add explicit `resize_stack_terminal(sessionId, cols, rows, pixelWidth, pixelHeight)` with debounce/coalescing from frontend fit.
- Convert output from polling to push-first streaming. Backend reader thread should enqueue chunks and emit events to `stack-popup`; frontend can poll only as recovery.
- Keep blocking PTY work outside the Stack Popup runtime mutex.
- Add terminal session metadata: createdAt, firstByteAt, lastOutputAt, exitCode, exitReason, profile, cwd, cols, rows.
- Avoid temporary registry removal races for normal poll/write. Use per-session synchronization or a short registry lock to obtain stable handles/state, then serialize IO without making other commands see the session as missing.
- Add shell integration injector per profile, gated by setting and env var. Support PowerShell, Git Bash/bash, and later zsh/fish if WSL support arrives.
- Add OSC parser-side metadata handling either in frontend xterm parser hooks or backend if needed for security/ordering.

Frontend:

- Add xterm addons in phases: WebGL with context-loss fallback, WebLinks, Search, Unicode, Serialize.
- Use xterm markers/decorations for command markers, exit status, cwd changes, and scroll navigation.
- Add command palette/quick action surface for terminal commands: search, clear, copy output, copy cwd, split, new session, restart, kill, open external terminal.
- Add first-paint/startup state overlay and terminal health status.
- Add event listeners for `stack-terminal:output`, `stack-terminal:cwd`, and `stack-terminal:closed`; keep polling as watchdog.
- Split terminal state out of `StackPopupSurface.svelte` into a terminal-specific module/component. The current surface is too large to carry all terminal work safely.

Settings:

- Keep executable profiles enum-bounded for safety.
- Add terminal appearance settings: font family, size, line height, cursor shape/blink, scrollback, theme, default shell profile.
- Add behavior settings: shell integration on/off, startup overlay on/off, copy on select, right-click paste/context menu, prompt marks, preserve sessions on Files switch, auto-scroll mode.
- Add keybinding settings only after command registry exists.

Security:

- Do not persist arbitrary shell executable paths until there is a trust UI and validation model.
- Treat OSC 52 clipboard writes as a policy decision. Allow copy-only by default or prompt for remote sessions.
- Be careful with shell integration injection: scripts must be static, local, reviewable, profile-specific, and opt-out.
- Do not send secrets from terminal output into logs, diagnostics, AI, or command suggestions.

## Performance Algorithms And Needs

Modern terminals win through a pipeline, not one trick:

- PTY read loop: non-blocking or dedicated thread, large enough buffers, preserve byte order, avoid per-byte IPC.
- Decode/parser: terminal escape parser streams into a grid/buffer model, not string concatenation.
- Render invalidation: repaint changed rows/cells, not the whole screen every chunk.
- GPU acceleration: atlas/glyph caching or WebGL renderer for high-output workloads.
- Backpressure: when output arrives faster than the UI can render, batch chunks per animation frame and preserve final state.
- Scrollback storage: ring buffer with bounded memory and optional serialization, not unbounded JS strings.
- Resize: fit frontend, coalesce resize commands, resize PTY, then let shell redraw.
- Alternate screen: full-screen apps must not pollute normal scrollback and should receive keys/mouse exactly.

JasonShell needs:

- Replace `stackTerminalOutput` as the source of truth for large history. xterm buffer should own terminal content; use SerializeAddon for restore/export only.
- Batch event writes to xterm inside `requestAnimationFrame` or a small microtask queue.
- Add high-output benchmark fixtures: `yes`, large `git log`, `npm install`, `cargo test`, `Get-ChildItem -Recurse`, and full-screen apps like `vim`, `less`, `fzf`, `top` equivalents where available.
- Track time-to-first-byte, time-to-first-prompt, output throughput, frame time, memory after 100k lines, and input latency under output load.

## Feature Backlog Prioritized

### P0: Make Launch Trustworthy

Outcome: CLI never opens as unexplained black.

Tasks:

- Add startup/empty/error state overlay until first PTY byte or prompt marker.
- Remove or gate `Clear-Host` from startup script.
- Add first-byte timeout and visible error/diagnostic path.
- Make start response/session state honest: either return initial drained output, or remove the frontend expectation that `session.output` exists and rely on event-driven first-byte state.
- Fix PowerShell launch resolution without reintroducing quoted-path `cmd.exe` failures.
- Serialize frontend/backend poll/write paths so a live session cannot transiently disappear during overlapping operations.
- Add live screenshot smoke for first launch and warm launch.
- Add terminal lifecycle telemetry in dev mode.

Acceptance:

- CLI first open shows prompt, startup state, or error within 150ms.
- If no PTY output arrives in 1200ms, user sees profile/cwd/status.
- Spawn failure never leaves blank black content.

### P1: Terminal Geometry And Event Streaming

Outcome: terminal behaves like real terminal under resize and output load.

Tasks:

- Add backend resize command and source tests proving xterm fit rows/cols are sent to PTY.
- Add frontend resize coalescing after `FitAddon.fit()`.
- Listen to `stack-terminal:output`, `stack-terminal:cwd`, and `stack-terminal:closed`.
- Keep 700ms poll only as recovery/watchdog.
- Batch output writes per frame.

Acceptance:

- Full-screen terminal apps see current rows/cols.
- Output appears event-driven, not delayed by polling interval.
- Closing while write/poll/resize is in flight does not restore killed session.

### P2: xterm Addon Modernization

Outcome: renderer matches baseline modern web terminal expectations.

Tasks:

- Add `@xterm/addon-webgl` with context-loss fallback.
- Add `@xterm/addon-web-links` plus custom path/git hash link provider.
- Add `@xterm/addon-search` and compact find UI.
- Add Unicode/grapheme support tuning.
- Add `@xterm/addon-serialize` for session snapshot/export and safe restore.

Acceptance:

- Links are clickable and keyboard reachable.
- Scrollback search works without leaving CLI.
- WebGL failure falls back cleanly.
- Unicode/emoji/powerline prompt smoke passes.

### P3: Shell Integration And Command Marks

Outcome: terminal understands prompts, commands, outputs, exit codes, and cwd.

Tasks:

- Implement OSC 133 support for prompt start/end, command start, command finished.
- Support VS Code/Windows Terminal-compatible cwd sequences where practical.
- Inject shell integration for PowerShell and Git Bash through static scripts.
- Add command markers/decorations in xterm gutter/overview.
- Replace input-inferred cwd with shell marker cwd when available; keep inference fallback.

Acceptance:

- Commands get success/failure marks.
- Jump previous/next command works.
- Copy command output works.
- `cd`, `pushd`, profile functions, and prompt-level cwd updates sync Stack Browser path.

### P4: Developer Ergonomics

Outcome: the terminal becomes faster for daily work than external Windows Terminal for project tasks.

Tasks:

- Add recent command and recent directory UI.
- Add Quick Select for URLs, paths, ports, git hashes, branch names, and error file refs.
- Add right-click menu: copy, paste, copy output, rerun, open file, open cwd in Files, open external terminal.
- Add quick fixes for common output: port in use, git upstream missing, command not found, failed tests with file paths.
- Add command palette commands scoped to terminal.

Acceptance:

- User can copy one command's output without drag-selecting.
- User can jump to failed file path from terminal output.
- User can rerun recent command from UI.

### P5: Sessions, Tabs, Splits, And Restore

Outcome: embedded terminal can host real workflows.

Tasks:

- Add terminal session list with names, cwd, profile, running state.
- Add split panes inside CLI mode.
- Add optional session preservation when switching back to Files.
- Add restart/kill session controls.
- Add open external Windows Terminal in same cwd.

Acceptance:

- Two commands can run side by side.
- User can switch Files/CLI without losing session if setting enabled.
- Session state remains understandable and killable.

### P6: Appearance, Accessibility, And Settings

Outcome: terminal is readable, configurable, and accessible.

Tasks:

- Add terminal settings in Settings Panel: font, size, line height, cursor, theme, scrollback, shell integration, copy/paste policy.
- Use monospace defaults.
- Add accessible buffer/navigation announcements.
- Add keyboard-only coverage for toolbar/search/session controls.

Acceptance:

- Terminal text is readable at default Stack Browser size.
- Keyboard-only user can open CLI, search, copy, paste, split, switch sessions, and return to Files.
- Screen reader path has useful labels and status changes.

### P7: Benchmark And QA Harness

Outcome: future terminal changes cannot regress performance or compatibility silently.

Tasks:

- Add automated source tests for renderer/addon wiring, resize IPC, event listener cleanup, and settings.
- Add Rust tests for resize/session metadata/stop races.
- Add live smoke scripts for first launch, warm launch, resize, high output, Unicode, full-screen app, Ctrl+C, paste, and close.
- Add performance thresholds for output throughput and input latency.

Acceptance:

- Focused terminal validation command exists and is documented.
- QA can prove no black launch, no delayed output, no stuck PTY, and no session resurrection.

## Implementation Phase Plan

Use strict phase order. Do not start P2 before P0/P1 are validated.

### Phase 1: RED Startup And Geometry Tests

Tests to write:

- Source test requiring visible startup/empty/error state in `StackPopupSurface.svelte`.
- Source test requiring backend resize command contract in `src/ipc/commands.ts`, `src/lib/stackPopup.ts`, `src-tauri/src/contracts.rs`, and `src-tauri/src/stack_popup/terminal.rs`.
- Rust test for `resize_stack_terminal` validating session id and dimensions.
- Source test requiring frontend to listen for `stack-terminal:output`.

Validation gate:

- `node --test tests/stackBrowserTerminal.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml terminal`

### Phase 2: P0/P1 Implementation

Tasks:

- Add startup overlay and first-byte state.
- Add backend resize and frontend fit-to-PTY.
- Convert output to event-driven primary path.
- Keep poll watchdog.
- Add dev diagnostics for startup phases.

Validation gate:

- Focused terminal Node + Rust tests.
- `npm run check`
- Live screenshot smoke.

### Phase 3: Addon Upgrade

Tasks:

- Add WebGL, WebLinks, Search, Unicode, Serialize.
- Add UI affordances for search and links.
- Add fallback and context-loss handling.

Validation gate:

- Focused Node source tests.
- Browser/WebView smoke for search/link/copy/high-output.

### Phase 4: Shell Integration

Tasks:

- Add static shell integration scripts/resources.
- Add OSC marker parser path.
- Add command decorations and cwd authority.
- Add command navigation/copy-output.

Validation gate:

- Shell-marker fixture tests.
- PowerShell and Git Bash live smoke.

### Phase 5: Developer Workbench Features

Tasks:

- Quick Select.
- Recent commands/directories.
- Quick fixes.
- Session list, tabs/splits, preserve-on-switch option.

Validation gate:

- Feature source tests.
- Live workflow smoke using repo tasks: `npm run check`, `npm run test:node`, `cargo test`.

### Phase 6: QA And Documentation

Tasks:

- Update `master_spec.md` only after behavior ships.
- Update `changelog.md` per `CHANGELOG_POLICY.md`.
- Add terminal smoke checklist to `docs/smoke-test-windows.md`.
- Run adversarial QA.

Validation gate:

- Focused tests.
- `npm run validate` if touched scope crosses frontend/backend/contracts/settings.

## Subagent Routing For Future Implementation

- `terminal-startup-geometry-test-worker`: `tdd-guide`, `senior-frontend`, `senior-backend`, `rust-skills`, `svelte5-best-practices`.
- `terminal-backend-pty-worker`: `rust-skills`, `senior-backend`, `tdd-guide`.
- `terminal-xterm-ui-worker`: `senior-frontend`, `svelte5-best-practices`, `frontend-design`, `tdd-guide`.
- `terminal-shell-integration-worker`: `senior-backend`, `senior-frontend`, `rust-skills`, `tdd-guide`.
- `terminal-adversarial-qa-reviewer`: `adversarial-reviewer`, `agent-browser` for live smoke if available.

## Source Links Used

- Windows Terminal overview: https://learn.microsoft.com/en-us/windows/terminal/
- Windows Terminal shell integration: https://learn.microsoft.com/en-us/windows/terminal/tutorials/shell-integration
- Windows Terminal rendering settings: https://learn.microsoft.com/en-us/windows/terminal/customize-settings/rendering
- Windows ConPTY pseudoconsoles: https://learn.microsoft.com/en-us/windows/console/pseudoconsoles
- Windows CreatePseudoConsole: https://learn.microsoft.com/en-us/windows/console/createpseudoconsole
- VS Code terminal shell integration: https://code.visualstudio.com/docs/terminal/shell-integration
- xterm.js docs: https://xtermjs.org/docs
- WezTerm features: https://wezterm.org/features.html
- WezTerm Quick Select: https://wezterm.org/quickselect.html
- Ghostty features: https://ghostty.org/docs/features
- Ghostty shell integration: https://ghostty.org/docs/features/shell-integration
- Kitty shell integration: https://sw.kovidgoyal.net/kitty/shell-integration/
- Warp blocks: https://docs.warp.dev/terminal/blocks
- Warp implementation article: https://www.warp.dev/blog/how-warp-works
- Warp block model article: https://www.warp.dev/blog/block-model-behind-warps-agentic-development-environment
- Alacritty: https://alacritty.org/
- Tabby terminal API docs: https://docs.tabby.sh/terminal/

Context7 docs were also consulted for `xterm.js` and Tauri 2 command/event/process guidance.
