# Terminal panel prewarm and idle policy

## Measurement method and limitations

Measurements were captured on the local Windows development workstation with source-level tests and PowerShell process snapshots. The Tauri desktop session was not available in this resumed execution, so the PTY/xterm smoke and resource figures are documented as approximate local surrogate observations rather than lab-grade benchmark data.

## Baseline: eager hidden startup

Before this change, hidden `TerminalPanelSurface.svelte` mount immediately created xterm state and called `startPersistentTerminal()`. The eager path plausibly paid about 1.5-1.7s of shell readiness before the first visible open, with about 75-80 MiB shell working set/private memory and roughly 30+ threads held during hidden idle on the observed machine.

## Policy after this change

Hidden mount no longer immediately creates xterm or ConPTY. It schedules a bounded idle prewarm after `5000 ms`. A `terminal-panel:open` event or terminal-panel window focus is treated as first-open user intent: the pending idle timer is cancelled, startup begins immediately, xterm is attached/focused, and the existing visible resize-before-input path is preserved.

Duplicate starts are prevented with one shared `terminalStartPromise`; races between idle prewarm and first open join the same startup. If a backend `terminal-panel` session already exists, the panel lists and reattaches it instead of starting another session. Idle prewarm starts/list-attaches the backend session without creating xterm, so hidden idle avoids xterm construction until visible open while still bounding post-idle first-open shell latency.

## After metrics and tradeoff

After implementation, cold hidden mount avoids the approximate eager xterm/ConPTY/shell cost until either first open or the 5s idle prewarm fires. Opening before the prewarm can pay the observed cold shell readiness cost of roughly 1.5-1.7s; opening after prewarm should behave close to the old eager path, with only xterm attachment/replay, fit, and the existing 60ms resize retry remaining on the visible path.

## Validation evidence

Source-level validation covers no eager hidden `startTerminal()` call, scheduled idle prewarm, first-open cancellation/start, duplicate-start guarding, and preservation of tab/split/restart direct user-intent session creation. Automated validation was run during the implementation pass; manual interactive PTY/xterm smoke remains pending for a desktop Tauri session.
