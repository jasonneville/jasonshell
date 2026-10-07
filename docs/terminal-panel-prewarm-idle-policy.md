# Terminal panel prewarm, idle, and status policy

## Measurement method and limitations

Measurements were captured on the local Windows development workstation with source-level tests and PowerShell process snapshots. The Tauri desktop session was not available in this resumed execution, so the PTY/xterm smoke and resource figures are documented as approximate local surrogate observations rather than lab-grade benchmark data.

## Baseline: eager hidden startup

Before this change, hidden `TerminalPanelSurface.svelte` mount immediately created xterm state and called `startPersistentTerminal()`. The eager path plausibly paid about 1.5-1.7s of shell readiness before the first visible open, with about 75-80 MiB shell working set/private memory and roughly 30+ threads held during hidden idle on the observed machine.

## Policy after this change

Hidden mount no longer immediately creates xterm or ConPTY. It schedules a bounded idle prewarm after `5000 ms`. A `terminal-panel:open` event or terminal-panel window focus is treated as first-open user intent: the pending idle timer is cancelled, startup begins immediately, xterm is attached/focused, and the existing visible resize-before-input path is preserved.

Duplicate starts are prevented with one shared `terminalStartPromise`; races between idle prewarm and first open join the same startup. If a backend `terminal-panel` session already exists, the panel lists and reattaches it instead of starting another session. Idle prewarm starts/list-attaches the backend session without creating xterm, so hidden idle avoids xterm construction until visible open while still bounding post-idle first-open shell latency.

## Visible status policy

Normal startup and output waiting never cover the terminal with a status notice. This includes hidden mount, the five-second idle prewarm, slow first-open startup, an empty or whitespace-only initial output, and the existing waiting timeout. The panel may remain visually quiet while it is starting or waiting; it must not invent a running pane before a session exists.

Genuine diagnostics remain visible and accessible regardless of earlier output: a startup rejection before a pane exists, a PTY/read failure, an exit (including exit before output), and an explicit stop of the final tab produce one truthful alert/status notice. Existing output remains visible when a later exit or read failure is reported. Retry clears the prior failure before a new attempt and does not duplicate notices. This policy removes normal persistent startup/waiting UI only; it does not suppress real failed, exited, or stopped diagnostics.

## After metrics and tradeoff

After implementation, cold hidden mount avoids the approximate eager xterm/ConPTY/shell cost until either first open or the 5s idle prewarm fires. Opening before the prewarm can pay the observed cold shell readiness cost of roughly 1.5-1.7s; opening after prewarm should behave close to the old eager path, with only xterm attachment/replay, fit, and the existing 60ms resize retry remaining on the visible path.

## Validation evidence

Source-level validation covers no eager hidden `startTerminal()` call, scheduled idle prewarm, first-open cancellation/start, duplicate-start guarding, and preservation of tab/split/restart direct user-intent session creation. Rendered component coverage in `tests/components/terminal-panel-status.test.ts` verifies quiet normal startup/waiting, empty/whitespace/escape/prompt output, session reuse, and truthful accessible failure/exit/stop notices before and after output. These tests use mocked IPC/xterm and do not certify native ConPTY, PTY, or desktop Tauri behavior. Manual interactive PTY/xterm smoke remains pending for a desktop Tauri session.
