# JasonShell Bug Scourge Findings

Date: 2026-05-06  
Scope: read-only audit of current JasonShell functionality across Svelte UI, TypeScript IPC/state, Tauri/Rust command/event wiring, Windows shell integration, tests, capabilities, and runtime layout.  
Constraint: no production fixes in this pass. This file is the handoff plan and evidence artifact.

## Audit Method

- Read `master_spec.md` first and logged the audit request there.
- Used Context7 MCP for current Tauri 2 webview event/capability guidance and Svelte 5 reactivity/lifecycle guidance.
- Loaded relevant skills: `caveman`, `adversarial-reviewer`, `senior-frontend`, `rust-skills`, `senior-backend`, `tdd-guide`, `spec-driven-workflow`, and `agent-browser`.
- Delegated audit work to function-named subagents:
  - `frontend-ui-audit-subagent`: Svelte UI/state/event audit.
  - `rust-backend-wiring-audit-subagent`: Rust/Tauri/Win32 wiring audit.
  - `validation-contract-audit-subagent`: tests/contracts/capabilities audit.
  - `runtime-visual-audit-subagent`: runtime/layout/static visual audit.
- Parent validation run passed:
  - `npm run check`
  - `npm run build`
  - `npm run test:node`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
  - `cargo test --manifest-path src-tauri/Cargo.toml`
  - `npm run validate`
- Runtime visual limitation: `npm run tauri dev` was not launched because it installs the Windows-key hook, creates shell AppBars, and can alter Explorer/taskbar focus/work area. Vite was reachable at `http://localhost:1420`, but outside Tauri `App.svelte` falls back to `bottom-bar`, so browser-only rendering is weak signal.

## Priority Summary

P0: none found.

P1:
- Search focus-loss close event is emitted to the wrong webview, so TopBar can keep stale `searchOpen` state.
- Audio panel is a real window but lacks Tauri capability and is omitted from central surface registries.
- Audio panel starts polling while hidden and never stops on native focus-loss close.
- Windows-key hook likely breaks Win-key chords such as `Win+R` and `Win+D`.
- Rust `contracts.rs` command ledger is stale relative to shipped IPC commands.
- Stack context menu and tray panel can clip content because fixed-height popup bodies lack scroll paths.
- Process Manager header/body horizontal scroll can desync, making right columns hard to reach.

P2:
- `open_shell_path` is a broad native ShellExecute surface.
- Stack archive/file operations block Tauri command threads for large operations.
- Process Manager holds icon-cache mutex during shell icon extraction.
- Task preview holds runtime mutex while performing Tauri window ops.
- Tray panel snapshots can go stale because hidden persistent webview only loads icons on mount.
- Control Plane tab semantics conflict with always-visible content panels.
- Async Tauri listener registration can leak if components destroy before `listen(...).then(...)` resolves.
- Event constants are split between central IPC events and feature wrappers.
- Current master spec has contradictory search scheduling text.
- Search result list height likely clips bottom rows at small panel sizes.

P3:
- `audio:refresh` appears frontend-only, with no Rust emitter found.
- TopBar rail scroll timeout is untracked.
- Source-text tests are too brittle in several areas.
- Ignored `dist-tests` root artifacts can become stale and be imported accidentally.
- `src/ipc/surfaces.ts` omits `audio-panel`, causing tooling to classify it as unknown even if capability is fixed.

## Findings

### P1: Search Focus-Loss Close Event Targets Search Panel, Not TopBar

Evidence:
- `src-tauri/src/main.rs:203` handles search-panel `WindowEvent::Focused(false)`.
- `src-tauri/src/main.rs:206` calls `window.emit(search_panel::SEARCH_PANEL_CLOSED_EVENT, ())`, which emits on the search-panel webview.
- `src/components/TopBar.svelte:1265` listens for `SEARCH_PANEL_CLOSED_EVENT` and resets `searchOpen`, query state, rows, timers, and panel anchor.
- `src-tauri/src/search_panel.rs:148` hides the search panel without publishing close state to TopBar.

Likely trigger:
- Open centered search.
- Click outside so native focus-loss hides the `search-panel` window.
- TopBar never receives close event, keeps `searchOpen = true`, and the next Windows-key/top-bar open path can skip or stale-route native show/publish behavior.

Fix plan:
- Emit `SEARCH_PANEL_CLOSED_EVENT` to `TOP_BAR_LABEL` on native focus loss and explicit hide, or use a shared close helper that always notifies TopBar.
- Keep same-surface `search-panel:closed` only if the search panel also needs local cleanup.

Tests to write:
- Source contract: `main.rs` must use `emit_to(shell_windows::TOP_BAR_LABEL, search_panel::SEARCH_PANEL_CLOSED_EVENT, ...)` on search-panel focus loss.
- Node behavior test: native close event must drive TopBar `resetActiveSearchState()` path.
- Live smoke: open centered search, click desktop, press Windows key again, verify panel opens with blank fresh state.

### P1: `audio-panel` Missing Tauri Capability and Central Surface Registries

Evidence:
- `src-tauri/src/shell_windows.rs:25` defines `AUDIO_PANEL_LABEL = "audio-panel"`.
- `src-tauri/src/shell_windows.rs:96` creates the audio panel window.
- `src-tauri/src/shell_windows.rs:270` builds the audio-panel webview.
- `src/App.svelte:74` routes `surface === 'audio-panel'` to `AudioPanelSurface`.
- `src/lib/shellSurface.ts:12`, `src/lib/shellSurface.ts:61`, and `src/lib/shellSurface.ts:83` recognize `audio-panel`.
- `src/ipc/surfaces.ts:1-12` omits `audio-panel`.
- `src-tauri/src/contracts.rs:3-26` omits `audio-panel` from `surfaces::ALL`.
- There is no `src-tauri/capabilities/audio-panel.json`.
- `tests/contractsSettings.test.mjs:186` capability parity expectation omits `audio-panel`, so current validation passes despite drift.

Likely trigger:
- Tauri v2 capability enforcement denies `audio-panel` use of allowed core/window/event/invoke permissions, or dev tooling that depends on `src/ipc/surfaces.ts` treats the valid audio webview as unknown.

Fix plan:
- Add `src-tauri/capabilities/audio-panel.json` with the same scoped baseline as other auxiliary surfaces.
- Add `audioPanel: 'audio-panel'` and title to `src/ipc/surfaces.ts`.
- Add `AUDIO_PANEL` to `contracts::surfaces::ALL`.
- Update tests so expected window labels are derived from one source of truth, not manually copied arrays.

Tests to write:
- Node parity test across `shell_windows.rs`, `src/lib/shellSurface.ts`, `src/ipc/surfaces.ts`, `App.svelte`, and `src-tauri/capabilities/*.json`.
- Rust contract test requiring `contracts::surfaces::ALL` to include all `shell_windows` labels.
- Tauri smoke: open sound control, verify audio panel can listen, invoke, and close without permission errors.

### P1: Audio Panel Polls While Hidden and Does Not Stop on Native Focus-Loss Close

Evidence:
- `src/components/AudioPanelSurface.svelte:180-185` sets `audioPanelVisible = true`, starts 2s polling, and calls `refreshAudioState()` on mount.
- Hidden persistent webviews are created at startup, so the panel can begin polling before the user opens it.
- `src/components/AudioPanelSurface.svelte:186-197` listens for `audio-panel:open` and `audio:refresh`, but not `audio-panel:closed`.
- `src-tauri/src/main.rs:226-234` emits `audio-panel:closed` to TopBar on focus loss and hides the panel.
- `src-tauri/src/audio_panel.rs:65-72` emits `audio-panel:closed` only to TopBar on explicit hide.

Likely trigger:
- Start JasonShell. Hidden audio webview mounts and polls every 2 seconds.
- Open sound, click away. TopBar marks `audioOpen = false`, but audio panel webview does not hear close and can keep polling.

Fix plan:
- Initialize `audioPanelVisible = false`.
- Start polling only on `audio-panel:open`.
- Emit a close event to the audio-panel window as well as TopBar, or make audio panel listen to its own focus/visibility state.
- Stop timers on close and cleanup.

Tests to write:
- Node source test proving `AudioPanelSurface` does not set visible/start polling on mount.
- Node source test proving `AudioPanelSurface` listens for `AUDIO_PANEL_CLOSED_EVENT` and calls `stopAudioRefreshPolling()`.
- Rust/Node event test proving both TopBar and audio panel receive close or that one shared event target covers both.
- Live smoke: open sound, close by blur, wait > 2s, verify no `get_audio_state` polling.

### P1: Windows-Key Hook Likely Breaks `Win+R`, `Win+D`, and Other Chords

Evidence:
- `src-tauri/src/windows_key_hook.rs:82-88` suppresses Windows-key down events.
- `src-tauri/src/windows_key_hook.rs:112` passes non-Windows keys through while marking chord state.
- `src-tauri/src/windows_key_hook.rs:239-241` returns `LRESULT(1)` for `OpenSearch` and `Suppress`.
- Existing tests assert keydown suppression for chords, e.g. `Win+R`/`Win+D` flows around `src-tauri/src/windows_key_hook.rs:323-351`.

Likely trigger:
- Press `Win+R` or `Win+D` while JasonShell runs. The non-Win key passes through, but Windows may never see the Win modifier because Win keydown was swallowed.

Fix plan:
- Re-evaluate product contract: if bare Windows key should open JasonShell search while chords remain native, suppress only the bare-key release/default Start activation path, not the modifier down needed by OS chords.
- If low-level hook cannot both allow OS chords and suppress bare Start reliably, document that limitation explicitly and choose the lesser product break.

Tests to write:
- Classifier test expecting Windows-key down for potential chords to pass through or otherwise prove OS receives modifier state.
- Live manual smoke: `Win+R`, `Win+D`, `Win+E`, `Win+L` behavior while JasonShell runs.
- Regression: bare Win tap still opens centered JasonShell search and does not open Start Menu.

### P1: Rust Contract Ledger Is Stale for Shipped IPC Commands

Evidence:
- `src/ipc/commands.ts` exposes commands that are registered in `src-tauri/src/main.rs` but omitted from `src-tauri/src/contracts.rs:29-180`.
- Missing examples include `list_quick_icons`, `pin_task_window_quick_icon`, `unpin_quick_icon`, `launch_quick_icon`, `show_centered_search_panel`, `resize_search_panel`, `show_quick_icon_context_menu`, `trigger_system_power_action`, and `open_stack_folder_in_vscode`.

Likely trigger:
- Future codegen, docs, control-plane introspection, or contract tests use `contracts.rs` as a source of truth and silently miss active commands.

Fix plan:
- Add all shipped command constants to `contracts.rs`.
- Make `contracts.rs` either authoritative and generated/checked from `main.rs` + `src/ipc/commands.ts`, or demote it to a non-authoritative historical helper.

Tests to write:
- Node parity test: every `IPC_COMMANDS` value must appear in Rust command registration and contract ledger.
- Rust test: `commands::ALL` contains all registered command constants and no duplicates.

### P1: Stack Context Menu Can Clip Actions Without Scroll

Evidence:
- `src/components/StackPopupSurface.svelte:2014` renders stack context menus inside fixed popup bounds.
- `src/components/StackPopupSurface.css:564` styles menu surfaces as fixed overlays.
- Stack Browser popup default height is 430 logical px and can be persisted smaller.

Likely trigger:
- Right-click a file/folder with full context menu, Open With submenu, archive actions, VS Code actions, and properties near bottom of popup.
- Lower actions become clipped by the webview because the menu has no internal scroll or viewport-aware flip/size path.

Fix plan:
- Reuse `contextMenuPosition` style clamping and add menu max-height with internal vertical scroll.
- Ensure submenus also clamp/scroll and remain pointer-reachable.

Tests to write:
- Unit test for menu placement with menu taller than available viewport.
- Browser smoke: open Stack Browser at 980x430, right-click bottom row, verify last menu item is visible/clickable.

### P1: Tray Panel Can Clip Large Icon Sets

Evidence:
- `src/components/TrayPanelSurface.svelte:59` renders tray icon grid directly in the dialog.
- `src/components/TrayPanelSurface.css:1` defines fixed panel styling; runtime subagent found no scroll container for overflow.
- The Tauri window is 252x220 logical px.

Likely trigger:
- Explorer exposes many visible/overflow tray icons, or mocked list has 36+ icons.
- Lower icons are unreachable because the root app disables page scrolling and the tray panel lacks an internal scroll area.

Fix plan:
- Add an internal scroll container for the grid with `min-height: 0` and `overflow: auto`.
- Keep error/loading states visible at top.

Tests to write:
- Node source/layout test requiring `.tray-grid` or parent scroll container.
- UI smoke with mocked 45 icons: scroll to last icon and invoke it.

### P1: Process Manager Header and Body Can Horizontally Desync

Evidence:
- `src/components/ProcessManagerSurface.svelte:291` renders process table header/body structure.
- `src/components/ProcessManagerSurface.css:109` defines body scroll behavior; runtime subagent found header outside the horizontally scrollable body.
- Default process-manager window width is 720 logical px while columns can exceed that.

Likely trigger:
- Open Process Manager at default width with long process names/paths and all metric/action columns.
- Body scroll reaches right columns, but header remains fixed/misaligned or right columns are clipped.

Fix plan:
- Put header and body under one horizontal scroll container, or use CSS grid with shared column template and sticky header inside the same scroller.
- Verify action/status columns remain reachable at default width.

Tests to write:
- Layout/source test requiring shared scroll wrapper around header and body.
- Live smoke: open Process Manager, scroll horizontally, verify headers align with cells and Kill/Action column is reachable.

### P2: `open_shell_path` Is Too Broad a Native Execution Boundary

Evidence:
- `src-tauri/src/shell_paths.rs:4` accepts arbitrary path-like input.
- `src-tauri/src/shell_paths.rs:120` sends it to `ShellExecuteW`.

Likely trigger:
- Compromised renderer or malformed result payload calls `open_shell_path` with executable, script, URL, or custom protocol input beyond intended safe file/folder opening.

Fix plan:
- Split by intent: open existing file/folder, open allowed `ms-settings:` URI, run vetted control panel applet, reveal path, open URL if explicitly allowed.
- Reject `.exe`, `.cmd`, `.bat`, `.ps1`, remote URLs, and unrecognized protocols unless routed through a specific audited command.

Tests to write:
- Rust negative tests for `cmd.exe`, `.ps1`, `http://`, `file://` surprises, and unknown protocols.
- Rust positive tests for existing local folder/file and allowed settings/control-panel intents.

### P2: Stack Archive and File Operations Can Block Command Threads

Evidence:
- `src-tauri/src/stack_popup.rs:442-457` runs archive extraction and waits synchronously with `.status()`.
- `src-tauri/src/stack_popup/file_ops.rs:161`, `:226`, and `:245` contain recursive copy/move/delete style operations.

Likely trigger:
- Extract a large archive, paste a large folder, or copy a deep tree from Stack Browser. Tauri command thread can remain blocked and other UI IPC responsiveness can degrade.

Fix plan:
- Move long operations behind `spawn_blocking` at minimum.
- Better: introduce job model with progress, cancel, partial success, and UI status.

Tests to write:
- Rust source/behavior tests proving extraction and recursive copy are off the command thread.
- Integration smoke with large fake tree while search/top-bar IPC remains responsive.

### P2: Process Manager Icon Cache Holds Mutex During Shell Icon Extraction

Evidence:
- `src-tauri/src/process_manager.rs:669` locks process icon cache.
- `src-tauri/src/process_manager.rs:677` performs shell icon extraction while still under the cache lock.
- Recent Stack Browser icon code fixed the same class of issue by resolving cache misses outside mutex.

Likely trigger:
- First Process Manager refresh with many uncached process icons or slow shell extensions. One slow extraction serializes all cache access and can stall refresh.

Fix plan:
- Use short lock for lookup, drop lock for shell extraction, reacquire only to store.
- Consider bounded concurrent icon hydration if process list grows.

Tests to write:
- Rust test or source contract proving cache miss extraction happens outside locked section.
- Concurrency test for overlapping `list_processes` calls.

### P2: Task Preview Holds Runtime Mutex Across Tauri Window Ops

Evidence:
- `src-tauri/src/task_preview.rs:257` locks preview runtime state.
- `src-tauri/src/task_preview.rs:272-277` emits/positions/shows while still in the same flow.

Likely trigger:
- Rapid hover/leave across taskbar tiles. Slow window operations can block hide/show freshness and retain stale previews.

Fix plan:
- Copy request freshness decision and thumbnail handle while locked, drop lock before `emit`, `set_position`, `show`, `set_focus`, then reacquire only if state must be updated on failure.

Tests to write:
- Rust unit around stale request A, hide, request B freshness.
- Source test requiring no `MutexGuard` live across Tauri window calls.

### P2: Tray Panel Snapshot Can Go Stale

Evidence:
- `src/components/TrayPanelSurface.svelte:54-55` loads tray icons only on mount.
- `src-tauri/src/tray_panel.rs:17` shows/focuses tray panel but emits no open/refresh event to the tray panel webview.

Likely trigger:
- Tray icons change after hidden persistent webview mounts. First visible tray open can show stale or empty icons until an action reloads.

Fix plan:
- Add `tray-panel:open` event targeted to tray-panel on every show.
- Reload icons on that event, not only on mount.

Tests to write:
- Node source test requiring open event listener in `TrayPanelSurface`.
- Rust source test requiring `show_tray_panel` emits open event to `TRAY_PANEL_LABEL`.

### P2: Control Plane Tab Semantics Are Misleading

Evidence:
- `src/components/ControlPlaneSurface.svelte:130` uses Melt tablist/trigger builders.
- `src/components/ControlPlaneSurface.svelte:153` forces content with `hidden={false}`.

Likely trigger:
- Screen reader users hear tab semantics, but selecting a tab does not control panel visibility as expected.

Fix plan:
- Either hide inactive panels properly and let tabs behave like tabs, or replace tab semantics with ordinary filter/navigation buttons if all sections must remain visible.

Tests to write:
- Accessibility source test: inactive tab panels must be hidden when using tab semantics.
- Keyboard smoke: arrow keys and tab selection produce expected visible panel state.

### P2: Async Tauri Listener Registration Can Leak on Early Destroy

Evidence:
- `src/components/TopBar.svelte:1205`, `BottomBar.svelte:556`, `TaskPreviewSurface.svelte:86`, `SearchPanelSurface.svelte:100`, and `ProcessManagerSurface.svelte:240` use `listen(...).then((unlisten) => unlisteners.push(unlisten))`.
- `src/components/StackPopupSurface.svelte:197` already uses a safer disposed-guard pattern.

Likely trigger:
- Component is destroyed during HMR, window teardown, or route mismatch before `listen()` promise resolves. The unlisten callback arrives after cleanup and remains active.

Fix plan:
- Copy StackPopup disposed-guard pattern to all persistent surfaces.

Tests to write:
- Source contract test requiring disposed guard around async listener registration in long-lived surfaces.
- HMR/manual smoke: reload frontend repeatedly and verify no duplicate event reactions.

### P2: Event Constants Are Split and Not Clearly Authoritative

Evidence:
- `src/ipc/events.ts:1` contains partial central event constants.
- `src/lib/audio.ts:55` hardcodes `audio-panel:closed`; `src/lib/audio.ts:56` uses `IPC_EVENTS.audioRefresh`.
- `src/lib/searchPanel.ts` hardcodes several search panel event strings outside `src/ipc/events.ts`.
- Rust emits matching literals from feature modules.

Likely trigger:
- Future rename updates one wrapper or test but not another; source tests still pass because no exhaustive event parity exists.

Fix plan:
- Decide whether `src/ipc/events.ts` is exhaustive.
- If yes, move all cross-window event strings there and test no raw event literals outside central IPC/Rust contracts.
- If no, document it as a convenience subset and avoid treating it as a registry.

Tests to write:
- Event parity test between `src/ipc/events.ts`, Rust `contracts::events::ALL`, and feature wrappers.

### P2: `master_spec.md` Has Contradictory Current Search Scheduling Text

Evidence:
- Current spec says typed search publishes and starts work for every input event with no debounce/coalescing.
- Later current snapshot text still describes `applySearchQuery` scheduling provider work through a zero-delay latest-only sequence.
- Tests such as `tests/searchTypingFreezePhase1.test.mjs` now assert immediate execution and ban older queue/coalescing behavior.

Likely trigger:
- Future agent follows stale spec paragraph and reintroduces query scheduling/coalescing that previously caused one-character-behind search bugs.

Fix plan:
- Mark the zero-delay scheduling paragraph superseded or move it to historical ledger only.
- Keep one authoritative "visible typed search" contract in the Search section.

Tests to write:
- Source/spec consistency test banning `queueSearchQueryProcessing` and stale phrase from current spec sections.

### P2: Search Result List Height Can Clip Bottom Rows

Evidence:
- `src/components/SearchPanelSurface.css:1-8` root uses fixed-height/hidden overflow.
- `src/components/SearchPanelSurface.css:93-96` gives result list `max-height: calc(100% - 2rem)`, which does not appear to account for all panel padding/header/input geometry.

Likely trigger:
- Small 420x320 top-right search panel or minimum centered search size with many rows. Final row or "show more" row can be partly hidden.

Fix plan:
- Use flex column with `min-height: 0` and `overflow: auto` on the results region instead of approximate max-height.

Tests to write:
- Browser smoke with minimum search panel size and broad query; scroll to bottom and verify last row fully visible.
- CSS source test requiring flex `min-height: 0` results scroller.

### P3: `audio:refresh` Event Has No Backend Emitter

Evidence:
- `src/ipc/events.ts:6` defines `audio:refresh`.
- `src/components/AudioPanelSurface.svelte:193` listens for `AUDIO_REFRESH_EVENT`.
- No Rust emitter was found for `audio:refresh`.

Likely trigger:
- Device/session changes rely only on polling, despite event contract implying live updates.

Fix plan:
- Either implement native audio change emitter, or remove/de-emphasize the dead event contract and rely on bounded polling.

Tests to write:
- If kept: Rust source test requiring `emit_to(AUDIO_PANEL_LABEL, AUDIO_REFRESH_EVENT, payload)`.
- If removed: Node test ensuring no listener references a dead event.

### P3: TopBar Rail Scroll Timeout Is Untracked

Evidence:
- `src/components/TopBar.svelte:690` schedules `setTimeout(updateRailScrollButtons, 160)` without storing/clearing it.

Likely trigger:
- HMR/window teardown can let the timeout mutate stale component state after destroy.

Fix plan:
- Store timer id and clear on cleanup.

Tests to write:
- Source test requiring tracked timeout cleanup.

### P3: Source-Text Tests Are Overused and Brittle

Evidence:
- Many tests read source files and regex implementation details, e.g. `tests/frontendUiPolicy.test.mjs`, `tests/stackPopupPagingPhase6Responsiveness.test.mjs`, and `tests/searchCloseReset.test.mjs`.
- These caught important architecture regressions, but also historically blocked validation on stale literal expectations.

Likely trigger:
- Refactor with same behavior changes function names/order and fails tests; or tests assert presence of code shape while behavior remains broken.

Fix plan:
- Keep source tests only for architecture bans and command/contract parity.
- Move behavior checks into pure helper tests, Rust unit tests, or browser/live smoke tests.

Tests to write:
- Meta-test tagging source-contract tests by intent, or separate `tests/source-contracts` from behavior tests.

### P3: Ignored `dist-tests` Root Artifacts Can Become Stale

Evidence:
- `.gitignore:3` ignores generated output.
- `tsconfig.test.json:7` emits to `dist-tests`.
- Existing ignored `dist-tests` root files can remain after compile shape changes.
- Master spec already warns about stale root-level `dist-tests/*.js` imports.

Likely trigger:
- A future test accidentally imports `../dist-tests/searchPanel.js` instead of `../dist-tests/lib/searchPanel.js`, passing against stale output.

Fix plan:
- Clean `dist-tests` before test compilation, or ban root-level imports/artifacts.

Tests to write:
- Node test scanning test imports for `../dist-tests/*.js`.
- Script update: `test:node` cleans `dist-tests` before `tsc -p tsconfig.test.json`.

## Checked Clean / Lower-Risk Areas

- Full validation passes now: `npm run validate`.
- `src/ipc/commands.ts` command strings appear registered in `main.rs`; the drift is specifically stale Rust contract ledger, not missing registration.
- Search engine and Stack icon hydration use async/blocking boundaries for known expensive provider/icon paths.
- Stack Browser recent icon hydration fixes align with spec: filtered `visibleEntries`, visible-window telemetry, row-count progress, stale guards, and short icon-cache locks.
- BottomBar core timers/listeners generally clean up; main issue is the async listener-registration race.
- Process Manager open/close event flow stops refresh on close; main issues are layout/horizontal scroll and backend icon extraction lock.
- Command Panel internal scroll path appears sound (`min-height: 0`, internal overflow).
- Settings Panel basic close/focus path appears stable.
- Quick command runner uses argv-style spawning rather than shell string concatenation.
- AppBar Alt-Tab exclusion style is reapplied after show/restore in current Rust code.

## Suggested Fix Order

1. Fix surface/capability/contract parity first: `audio-panel` capability, `src/ipc/surfaces.ts`, `contracts.rs`, and parity tests.
2. Fix close/open event correctness: search close to TopBar, audio close to audio panel, tray open refresh.
3. Fix Windows-key chord behavior with live smoke because this is user-visible and OS-level.
4. Fix hidden polling/listener lifecycle leaks across persistent surfaces.
5. Fix clipping/layout issues: Stack context menu, tray grid scroll, Process Manager horizontal scroll, search result scroller.
6. Fix backend blocking/lock issues: stack file jobs, process icon cache, task preview mutex.
7. Clean contract/test hygiene: event registry authority, stale spec text, brittle source-test split, `dist-tests` cleanup.
