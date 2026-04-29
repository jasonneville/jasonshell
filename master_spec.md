# JasonShell Master Specification

## Purpose and usage rules for future agents

`master_spec.md` is the durable, compaction-safe master briefing for `C:\dev\jasonShell`. Future AI agents should use it as the first repository-specific source of truth for system concepts, architecture, behavior contracts, and current risks.

This document is deliberately more granular than a continuity ledger. Keep implementation details here when they prevent future rediscovery: exact module paths, Tauri window labels, IPC command names, event names, state files, geometry constants, test names, behavioral invariants, and known gaps.

### Agent operating rules

- Read this file before making feature, bug-fix, refactor, test, documentation, or workflow changes in this workspace.
- Treat this file as a living specification, not a changelog-only artifact.
- Update the relevant functional section when behavior, APIs, commands, events, persistence, tests, or known risks change.
- Add a change-ledger entry for every user request that changes the repository or project workflow.
- Keep entries factual and repository-specific. Use provenance tags: `[USER]`, `[CODE]`, `[TOOL]`, `[ASSUMPTION]`.
- Do not store secrets, credentials, private tokens, or sensitive machine-specific content beyond already-known repo/config paths necessary for operation.
- Preserve unrelated worktree changes; this repository commonly has in-progress files and untracked validation artifacts.

## Mandatory first-step ledger protocol for new user requests

For every new user request that asks for a feature, bug fix, refactor, validation pass, documentation update, workflow change, or similar engineering work:

1. Read `master_spec.md` first.
2. Immediately append a new `Change Ledger` entry with:
   - date or ISO timestamp,
   - provenance tag `[USER]`,
   - concise objective,
   - expected affected surfaces/modules if known,
   - initial status such as `REQUESTED`, `IN_PROGRESS`, `VALIDATED`, or `BLOCKED`.
3. Proceed with discovery/implementation/validation. Test-Driven development should be used.
4. Update the affected master_spec.md spec sections before returning: behavior, implementation details, commands/events, persistence, tests, and known risks.
5. Add final `[CODE]` and `[TOOL]` ledger entries for what changed and what validation ran.

This first-step logging is mandatory even if the implementation is small. If a request is purely conversational and does not ask for workspace changes, do not add noise to the ledger.

## Current system snapshot

- Product: JasonShell, a Windows shell-foundation prototype using Tauri 2, Svelte 5, TypeScript, Rust, and Win32 APIs.
- UI primitive baseline: Svelte 5 uses the current `melt` package, not legacy `@melt-ui/svelte`. Shared local wrappers live under `src/components/melt/` and should be used where Melt has a direct headless primitive fit without collapsing Tauri webview boundaries or replacing native shell state machines. Renderer action buttons that are direct command controls should use `MeltActionButton` when their existing real `<button>` classes, ARIA attributes, and events can be preserved.
- Primary surfaces: `top-bar`, `bottom-bar`, `task-preview`, `search-panel`, `stack-popup`, `process-manager`, `control-plane`, `settings-panel`, and `audio-panel`, all routed through a shared `index.html` and selected by Tauri webview window label.
- Native shell reservation: the top and bottom bars reserve primary-monitor edge space via Windows AppBar APIs and adjust/restabilize the work area. A Win32 fullscreen guard hides/releases the JasonShell AppBars while a non-JasonShell foreground window covers the primary monitor or a borderless foreground window covers the baseline work area, then restores the reservation and surfaces when fullscreen exits. Desktop shell foreground windows such as `Progman`, `WorkerW`, and `SHELLDLL_DefView` are explicitly ignored so clicking the desktop does not hide JasonShell.
- Top bar: 26 logical pixels high; hosts the JasonShell settings button, pinned folder stack rail, sound control, date/time pill, and search input; opens settings, search, stack browser, and audio dropdown auxiliary controls. The sound control sits left of the time and opens the hidden persistent `audio-panel` webview because the top-bar webview is too short to render usable dropdown content itself. The audio panel shows master volume plus per-application session volume, lists input/output devices, and calls native audio commands immediately on slider/select changes. The shell-home/settings button, pin rail scroll buttons, pinned folder buttons, and sound button use the local Melt-backed action button wrapper while preserving real `<button>` semantics and `button[data-path]` queryability for pins.
- Bottom bar: 36 logical pixels high; hosts Explorer taskbar `.lnk` launchers, grouped open-window task tiles with previews/menus/activation/minimization/reorder gestures, content-sized active window tiles with min/max truncation bounds, and a right-edge process-manager button. Launcher right-click menus expose launch, Run as administrator, Properties, Open shortcut location, Open target location, and Copy shortcut path. The launcher, task window, and process-manager command buttons use the local Melt-backed action button wrapper while preserving real `<button>` semantics, CSS selectors, native menus, preview timing, drag/click suppression, and keyboard focus behavior. Task-window enumeration filters internal/system/no-activate/helper windows to match Windows taskbar expectations, including suppressing DWM notification windows.
- Stack Browser: hidden persistent `stack-popup` webview opened from top-bar pinned folders; supports paged folder reads, navigation history, an editable current-path textbox with clickable path segment navigation, sorting, selection, file operations, in-webview delete confirmation, delete-time focus-loss suppression with refresh-in-place, persisted in-webview resizing, inline rename/new folder, drag/drop, row and background/margin context menus, and pin updates. Safe command controls such as path segment buttons, toolbar actions, details sort headers, inline editor actions, context-menu items, and delete-confirm Delete use `MeltActionButton`; complex grid/file rows, the delete-cancel focus target, and the resize grip remain raw buttons because they own row semantics, drag/drop, focus refs, or pointer capture. File rows expose native Open With candidates and Windows Explorer copy drag through a Shell OLE drag source.
- Process manager: hidden persistent `process-manager` webview opened from bottom-bar; lists live processes grouped as Applications, Background processes, and Windows processes, with cached native process icons at the far-left of names, at-a-glance CPU, memory percentage, memory size, GPU percentage from Windows GPU Engine counters when available, Task Manager-style aggregate CPU/memory/GPU percentages and thread totals in column headers, centered metric cells, start time, thread count, status, executable-path title, sortable columns, guarded kill action, and focus-loss close behavior. Refresh/close, sortable column headers, group toggles, and kill actions use `MeltActionButton` while preserving the process grid roles and guarded kill state.
- Control plane: hidden persistent `control-plane` webview opened through explicit Tauri commands; renders an authority-light settings/developer dashboard model over existing settings, workspace, git, task, process, and provider contracts without exposing secrets or unbounded source lists. Its section navigation uses Melt `Tabs` and remains raw builder trigger buttons; section action buttons use `MeltActionButton` while keeping the dashboard authority-light and renderer-only.
- Search: top-bar input drives a dedicated `search-panel` webview because the top bar is too short for rich results. Search merges pinned apps, open windows, static commands, and indexed system app/file/folder results; the panel closes on top-bar outside clicks, search-input blur when the user is not interacting with results, and search-panel focus loss. The pin-folder result action uses `MeltActionButton` while result rows stay raw role-option rows for selection, double-click, keydown, and drag behavior.
- Settings panel: hidden persistent `settings-panel` webview opened from the top-left `jasonshell` button; renders renderer-only app preferences including live theme, font, date-format, clock, density, focus, transparency, and search-hint controls in a vertically scrollable panel so lower controls remain reachable. Theme/font choices use a local Melt-backed select wrapper; preference booleans use a local Melt-backed toggle wrapper; close/reset/done command buttons use `MeltActionButton`.
- Backend: Rust commands are registered in `src-tauri/src/main.rs`; Windows-specific native logic is under `src-tauri/src/*` and `src-tauri/src/task_windows/*`.
- Validation bundle: `npm run validate` runs Svelte check, TypeScript/Vite build, `npm run test:node`, Rust tests, and Cargo check. `npm run test:search` remains a compatibility alias for the broader Node helper/source suite.
- System tray: parked/experimental as of 2026-04-27 Phase 0. The Rust module is compiled for Windows tests only and Tauri commands are intentionally not registered until a shipped surface, capabilities, and live smoke coverage exist.
- Current important residual risk: live Windows/Tauri smoke coverage is still useful for multi-webview event delivery, native popup placement, native Explorer drag cursor behavior, mouse XButton behavior, exact WebView geometry, real multi-monitor hardware behavior, and control-plane show/focus placement despite strong unit/build validation.

## Deep technical orientation for future implementation agents

JasonShell should be reasoned about as a small native shell with nine long-lived Tauri webview windows, not as a single-page web app. Most defects in this repository come from crossing one of these boundaries incorrectly:

- **Surface boundary:** `src/App.svelte` routes one shared `index.html` bundle to a surface by `getCurrentWindow().label`. A component must assume it only owns its own labeled window and must use explicit Tauri events/commands to affect another surface.
- **IPC boundary:** TypeScript wrappers in `src/lib/*.ts` are the canonical frontend boundary to Rust commands and app events. Components should not invent command strings or event names inline unless the wrapper, event table, tests, and this spec are updated together.
- **Native shell boundary:** `src-tauri/src/appbar.rs`, `explorer.rs`, `layout.rs`, `shell_windows.rs`, `task_windows/*`, `launchers.rs`, `taskbar_menu.rs`, `task_preview.rs`, `search_panel.rs`, `stack_popup.rs`, `process_manager.rs`, `control_plane.rs`, `workspaces.rs`, `automation.rs`, `providers.rs`, `dev_tools/*`, and `shell_paths.rs` are allowed to touch native window, process, ShellExecute, AppBar, DWM, OLE DB, COM, GDI, clipboard, process spawning, and filesystem APIs. Treat these modules as failure domains with explicit rollback, stale-response, and pointer-lifetime invariants.
- **Source of truth:** Rust is authoritative for OS state, persisted stack pins, normalized filesystem paths, latest auxiliary-window payloads, task-window/process enumeration, process killability, launcher validation, and native menu selection. Svelte state is authoritative only for view state such as current selection, scroll/reveal intent, transient drag state, search query, history cursor, retained rows, process sort column/direction, refresh/killing status, and local result ranking usage.
- **Persistence ownership:** `stack_popup.rs` owns stack pin storage. `search_sources/index.rs` owns search index cache storage. `settings.rs` owns versioned shell settings and workspace profile storage. `dev_tools/task_runner.rs` owns bounded task-history storage. `searchRanking.ts` owns only browser `localStorage` usage boosts. Do not persist secrets, HWNDs, active/minimized state, preview payloads, latest popup/search payloads, drag state, process snapshots, or native clipboard mirrors.
- **Staleness model:** Search, preview, stack popup, and process manager use request identifiers, latest-request fallbacks, or sequence gates. New async work must either be idempotent or explicitly rejected when stale. The correct default is to keep the previous visible state until the next authoritative payload arrives rather than clearing UI early.
- **Validation model:** Use pure JS/TS unit tests for reducers, event-target construction, drag/drop parsing, context-menu math, taskbar reorder/click state machines, and process-manager sort/format/source wiring. Use Rust tests for path validation, pagination, filesystem semantics, AppBar geometry/fullscreen helpers, task-window filtering, task-preview bounds/ROP behavior, process-manager helper behavior, search scoring/cache/provider mapping, and Win32 wrapper invariants. Use live `npm run tauri dev` smoke only for WebView2 delivery, exact native geometry, real fullscreen appbar release/restore, DWM/window capture, Explorer taskbar/AppBar interaction, native menus, native process-manager placement/focus-loss behavior, native file drops, Open With picker, and mouse XButton delivery.

Implementation mental model by surface:

1. `top-bar` is the orchestration surface for search, settings, and stack pins. It owns query text, selected search result, pin rail view state, date/time rendering preferences, and reacts to events from `search-panel`, `settings-panel`, Stack Browser pin mutations, and native top-bar pin menus.
2. `bottom-bar` is the orchestration surface for launchers and open windows. It owns preferred task group order, drag/click disambiguation, preview request sequence, content-sized task tile presentation bounds, and refresh reactions after native menu actions.
3. `search-panel` is a render-only auxiliary surface for the latest `SearchPanelPayload`. It emits selection/activation/pin intents plus `search-panel:interaction` while the user is working in results; top bar executes intents and suppresses blur dismissal during the interaction grace window. Rust emits `search-panel:closed` when native focus loss hides the panel so top-bar `aria-expanded` state stays correct.
4. `stack-popup` is a stateful auxiliary folder browser. It owns navigation history, editable path draft/reset/submit state, selection, sorting, retained rows, virtualized visible-row calculation, inline editor state, in-webview delete confirmation state, custom resize drag state, HTML drop suppression, row and background/margin context menus, and background page merging; Rust owns canonical filesystem mutations, pin persistence, persisted popup geometry, and focus-loss suppression while delete execution is in flight. The path textbox displays the actual current path, Enter validates/lists the typed path before committing current path/history and listing state, invalid typed paths keep the previous current path and entries visible without retained rows under the bad path, Escape resets the draft to the current path, and adjacent path segment buttons navigate to that segment through the retained-row folder-open flow. Background/margin right-clicks preserve row-specific menus, ignore toolbar/editor/dialog/resize controls, and expose directory actions plus selection-aware actions when selection exists. Stack Browser destructive confirmation must stay inside the `stack-popup` webview, not `window.confirm`, because the native browser dialog can cause Tauri focus loss and hide the popup before the user answers.
5. `task-preview` is a render-only auxiliary preview surface. Rust owns preview positioning and image capture; bottom bar owns request ordering and hide/show timers. Preview capture validates the requested HWND, rejects unavailable/minimized/hidden/cloaked/hung targets, prefers DWM extended-frame bounds when sane, falls back to `GetWindowRect` bounds, captures through the target window DC first, and only then falls back to a bounded screen-region `BitBlt` using `SRCCOPY | CAPTUREBLT`.
6. `process-manager` is a stateful auxiliary process table. Rust owns process enumeration, CPU-time snapshots, executable-path icon lookup/cache, metadata, positioning, focus-loss hide, and kill guardrails; Svelte owns open/closed refresh cadence, in-flight request gating, filtering, tree-aware display rows, aggregate metric header formatting, metric mini bars, two-step kill confirmation state, sorting, formatting, and status text.
7. `control-plane` is an authority-light auxiliary dashboard. Rust owns window creation, show/hide positioning, and automation/provider contract validation; Svelte owns bounded rendering, section filtering, keyboard tab navigation, and secret-redacted summaries.
8. `settings-panel` is a renderer-only auxiliary preferences dropdown. Rust owns top-bar anchored window placement and focus-loss hide; Svelte owns the preference controls and writes only localStorage-backed renderer preferences plus existing theme selection.

Cross-boundary do nots:

- Do not mutate AppBar/work area from frontend code.
- Do not make search keystrokes recursively scan broad filesystem roots; use the warmed index and Windows Search provider cache model.
- Do not treat window labels as optional strings; every IPC/event target must name the intended surface when delivery to a hidden auxiliary window matters.
- Do not assume native menu events carry arbitrary JSON; `taskbar_menu.rs` encodes menu IDs and decodes payloads itself.
- Do not rely on the auxiliary webview being visible before it receives payload; `search_panel` and `stack_popup` keep latest payload/request fallbacks specifically because direct event delivery can race window startup/focus/visibility.

## Architecture overview

### Frontend routing and surface selection

- `src/main.ts` mounts the Svelte application.
- `src/App.svelte` calls `getCurrentWindow().label` and maps it through `src/lib/shellSurface.ts`.
- `src/lib/shellSurface.ts` defines `ShellSurface = 'top-bar' | 'bottom-bar' | 'task-preview' | 'search-panel' | 'stack-popup' | 'process-manager' | 'control-plane' | 'settings-panel' | 'unknown'` and surface metadata.
- `src/App.svelte` renders:
  - `src/components/TopBar.svelte` for `top-bar`,
  - `src/components/BottomBar.svelte` for `bottom-bar`,
  - `src/components/TaskPreviewSurface.svelte` for `task-preview`,
  - `src/components/SearchPanelSurface.svelte` for `search-panel`,
  - `src/components/StackPopupSurface.svelte` for `stack-popup`,
  - `src/components/ProcessManagerSurface.svelte` for `process-manager`,
  - `src/components/ControlPlaneSurface.svelte` for `control-plane`.
  - `src/components/SettingsPanelSurface.svelte` for `settings-panel`.
- `src/App.svelte` also suppresses the browser-native context menu globally; `src-tauri/src/shell_windows.rs` injects a native initialization script with the same context-menu suppression for the Tauri webviews.

### Backend startup and state

- `src-tauri/src/main.rs` is the command and lifecycle hub.
- Managed state:
  - `ShellRuntimeState` from `appbar.rs` on Windows, `Mutex<()>` on non-Windows,
  - `TaskPreviewRuntimeState`,
  - `SearchPanelRuntimeState`,
  - `StackPopupRuntimeState`,
  - `search_sources::SearchIndexRuntimeState`.
- Startup sequence:
  1. Register all Tauri commands via `tauri::generate_handler!`.
  2. Create shell webview windows with `shell_windows::create_shell_windows`.
  3. Warm the search index asynchronously via `search_sources::warm_search_index`.
  4. On Windows, call `appbar::activate_shell_surfaces` to register, position, show, and stabilize top/bottom appbars.
  5. On non-Windows, show top/bottom windows without native AppBar integration.
- Shutdown/close behavior:
  - Closing or destroying `top-bar` or `bottom-bar` triggers `appbar::cleanup_shell_surfaces` and exits the app.
  - `stack-popup` hides itself when it loses focus; destructive delete confirmation is rendered inside the webview, and delete execution uses a narrow Rust focus-loss hold so Recycle Bin/delete focus changes do not hide the popup before the refreshed folder is visible.
  - `search-panel` emits `search-panel:closed` and hides itself when it loses focus.
  - `process-manager` emits `process-manager:closed`, hides itself, and stops frontend polling when it loses focus.
  - `control-plane` is shown/hidden by explicit commands and is not part of AppBar cleanup ownership beyond normal app exit.
  - `settings-panel` hides itself on native focus loss and can also be hidden by its Close/Done/Escape controls.
  - App exit also attempts appbar cleanup.

Detailed lifecycle contract from `src-tauri/src/main.rs`:

- `tauri::Builder::default()` installs managed `Mutex` state before any command can run: Windows `ShellRuntimeState` or non-Windows `Mutex<()>`, `TaskPreviewRuntimeState`, `SearchPanelRuntimeState`, `StackPopupRuntimeState`, and `search_sources::SearchIndexRuntimeState`.
- `invoke_handler` registers all commands up front. If a command is added to Rust but not this list, frontend `invoke()` fails at runtime even when Rust compiles.
- `on_menu_event` is centralized in `taskbar_menu::handle_taskbar_menu_event`. Native menu IDs encode action type and payload; do not attach per-menu closures.
- `on_window_event` has special cases: `stack-popup` hides on `WindowEvent::Focused(false)` unless `StackPopupRuntimeState` is holding focus loss during an internal delete command, `search-panel` emits `search-panel:closed` then hides on `WindowEvent::Focused(false)`, `process-manager` emits `process-manager:closed` then hides on `WindowEvent::Focused(false)`, `settings-panel` hides on `WindowEvent::Focused(false)`, and primary shell surfaces (`top-bar`, `bottom-bar`) clean AppBars and exit on close/destroy. `control-plane` currently has explicit show/hide commands and no focus-loss lifecycle event. Avoid adding generic close handlers that would make auxiliary windows terminate the shell.
- `.setup()` creates windows first, starts index warming second, then activates AppBars on Windows. This order matters: AppBar activation needs native HWNDs; search warming must not block surface creation; non-Windows fallback only shows top/bottom windows and leaves AppBar/runtime metric commands unsupported.
- `app.run()` repeats AppBar cleanup on `RunEvent::Exit` and `ExitRequested`. Cleanup is intentionally idempotent through `ShellRuntimeState.cleaned_up`.
- Frontend `reportShellSurfaceRuntimeMetrics(label)` calls the Windows-only `report_shell_surface_runtime_metrics` command about 250ms after `TopBar`/`BottomBar` mount. The returned/logged `ShellSurfaceRuntimeMetrics` contains native rect and frontend `outerHeight`, `innerHeight`, and `clientHeight` to catch zero-height WebView/native-window regressions early.

### Native window and geometry model

- `src-tauri/src/shell_windows.rs` creates all nine webview windows as borderless, native dark-themed, always-on-top, skip-taskbar windows; the renderer applies the user-selected JasonShell CSS theme and shell preferences across webviews after load.
- Window labels and dimensions:
  - `top-bar`: `TOP_BAR_HEIGHT_LOGICAL = 26.0`.
  - `bottom-bar`: `BOTTOM_BAR_HEIGHT_LOGICAL = 36.0`.
  - `task-preview`: `332x228` logical.
  - `search-panel`: `420x320` logical.
  - `stack-popup`: initial `980x430` logical; runtime stack popup size uses the persisted `stack-popup-geometry-v1.json` logical size when present, otherwise defaults to `980` wide and a monitor-height ratio, then clamps to the current monitor.
  - `process-manager`: `720x520` logical, clamped to the current monitor in `process_manager.rs` when shown.
  - `control-plane`: `860x620` logical, clamped to the current or primary monitor in `control_plane.rs` when shown.
  - `settings-panel`: `440x520` logical, anchored below the top-left JasonShell button and clamped to the top-bar host width in `settings_panel.rs` when shown.
- `src-tauri/src/layout.rs` computes preview rects for the top and bottom shell bars.
- `src-tauri/src/layout.rs` also exposes pure Phase 9 multi-monitor planning contracts: `MonitorDescriptor`, `MonitorShellPlan`, `MonitorShellOwnership`, `plan_monitor_shell_layout`, `plan_popup_anchor`, and `assign_task_strip_monitor`. These helpers model mixed-DPI primary-shell vs secondary-task-strip ownership, popup anchoring against the source monitor, and stable task-strip monitor assignment before live multi-monitor AppBar activation is implemented.
- `src-tauri/src/appbar.rs` handles AppBar registration (`ABM_NEW`, `ABM_QUERYPOS`, `ABM_SETPOS`, `ABM_REMOVE`), work-area mutation, Explorer taskbar hiding/restoring, topmost `SetWindowPos`, startup stabilization polling, and runtime surface metrics.
- Do not reintroduce redundant `SetWindowPos`/AppBar positioning that races WebView2 startup or double-reserves work area; prior continuity recorded regressions around hidden appbar reservations and WebView blank/zero-height startup.
- `shell_windows::create_shell_windows` reads the primary monitor once, converts physical monitor size to logical width using monitor scale factor, then calls `layout::build_shell_preview_rects` with physical top/bottom heights to size the top/bottom shell windows before AppBar activation.
- Window creation order is top bar, bottom bar, task preview, search panel, stack popup, process manager, control plane, settings panel, audio panel. Only top and bottom are returned to AppBar activation; auxiliary windows are hidden persistent webviews reachable by label.
- All nine windows are created with `always_on_top(true)`, `decorations(false)`, `focused(false)`, `resizable(false)`, `maximizable(false)`, `minimizable(false)`, `skip_taskbar(true)`, native dark theme, and a context-menu suppression initialization script. The renderer theme is CSS-token driven and may be dark or light independent of the native window hint. `task-preview`, `search-panel`, `stack-popup`, `process-manager`, `control-plane`, `settings-panel`, and `audio-panel` keep `shadow(true)`; top/bottom bars use `shadow(false)`. Stack Browser user resizing is implemented by an in-webview bottom-right grip that calls Rust sizing commands because the native popup is borderless.
- AppBar activation registers top and bottom HWNDs with `ABM_NEW`, negotiates edge rectangles with `ABM_QUERYPOS`, forces requested thickness back into the returned rect, commits with `ABM_SETPOS`, mutates work area with `SPI_SETWORKAREA`, positions windows topmost with `SetWindowPos(..., SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOOWNERZORDER)`, shows windows, and then polls native rects until three stable matches or failure.
- `appbar.rs` snapshots Explorer primary taskbar state, may hide it, starts a guard thread that repeatedly enforces hidden state every 100ms while active, and restores both taskbar and baseline work area during cleanup. Partial activation failure hides shell windows and rolls back registered AppBars/taskbar/work area.
- `appbar.rs` stores the activated primary-monitor shell layout and starts a fullscreen guard thread. The guard polls the foreground window every 250ms, ignores JasonShell/current-process foreground windows, desktop shell classes (`Progman`, `WorkerW`, `SHELLDLL_DefView`), and hidden/minimized/cloaked targets, treats monitor-covering windows as fullscreen, and treats work-area-covering windows as fullscreen only when borderless. On fullscreen entry it unregisters tracked AppBars, sets the work area back to the full primary monitor, hides top/bottom webviews, and keeps HWND rects warm. On exit it re-registers/reserves both AppBars, restores the reserved work area, repositions, and shows the surfaces. Cleanup stops the guard and remains idempotent even if the shell is hidden for fullscreen.
- `STARTUP_STABILIZATION_POLLS = 15`, `STARTUP_STABILIZATION_DELAY = 100ms`, and `REQUIRED_STABLE_POLLS = 3`; changing these alters live startup tolerance and should be justified with live metrics.
- Non-Windows builds compile via fallbacks: launchers/window enumeration return empty or unsupported errors, AppBar state is `Mutex<()>`, top/bottom are merely shown, and runtime metrics command returns an unsupported error. Keep this compile path intact even though product target is Windows.

## Frontend surfaces and windows/webviews

| Label | Component | Rust owner | Purpose |
| --- | --- | --- | --- |
| `top-bar` | `src/components/TopBar.svelte` + `TopBar.css` | `shell_windows.rs`, `appbar.rs`, `search_panel.rs`, `stack_popup.rs`, `taskbar_menu.rs` | Primary upper rail: pinned folder stacks, time/date, search input. |
| `bottom-bar` | `src/components/BottomBar.svelte` + `BottomBar.css` | `shell_windows.rs`, `appbar.rs`, `launchers.rs`, `task_windows/*`, `taskbar_menu.rs`, `task_preview.rs` | Taskbar-like lower rail: launchers, grouped windows, previews, menus. |
| `task-preview` | `src/components/TaskPreviewSurface.svelte` + `TaskPreviewSurface.css` | `task_preview.rs`, `task_windows/previews.rs` | Hover preview for open task windows. |
| `search-panel` | `src/components/SearchPanelSurface.svelte` + `SearchPanelSurface.css` | `search_panel.rs`, `search_sources/*`, `shell_paths.rs` | Search result list anchored under the top-bar search input. |
| `stack-popup` | `src/components/StackPopupSurface.svelte` + `StackPopupSurface.css` | `stack_popup.rs`, `shell_paths.rs`, `task_windows/icons.rs` | Persistent folder browser anchored under top-bar pinned folders. |
| `process-manager` | `src/components/ProcessManagerSurface.svelte` + `ProcessManagerSurface.css` | `process_manager.rs`, `shell_windows.rs` | Task Manager-like process list popup anchored above the bottom-bar process button. |
| `control-plane` | `src/components/ControlPlaneSurface.svelte` + `ControlPlaneSurface.css` | `control_plane.rs`, `shell_windows.rs`, `automation.rs`, `providers.rs` | Settings and developer dashboard surface with bounded, secret-redacted summaries over existing contracts. |
| `settings-panel` | `src/components/SettingsPanelSurface.svelte` + `SettingsPanelSurface.css` | `settings_panel.rs`, `shell_windows.rs` | Top-left JasonShell dropdown for renderer-only app preferences: theme, font, date/time, density, focus, transparency, and search hint. |
| `audio-panel` | `src/components/AudioPanelSurface.svelte` | `audio_panel.rs`, `audio.rs`, `shell_windows.rs` | Top-bar anchored sound dropdown with master volume, per-app session volume, input device, and output device controls. |

## Top bar spec

### User behavior

- Stays docked on the primary monitor top edge at roughly half taskbar height.
- The leftmost `jasonshell` button opens the `settings-panel` dropdown anchored below that button. It no longer opens search; search remains available from the search input and Ctrl/Cmd+K.
- Shows a horizontally scrollable pinned folder rail on the left/middle.
- Shows current time and date in a compact `time-pill` updated every second. The time/date display reads renderer preferences from `src/lib/shellPreferences.ts`, including 24-hour time, seconds visibility, and custom date format strings.
- Shows a search control on the right with placeholder `Search` and an optional shortcut hint `Ctrl K`.
- Shows a sound icon immediately left of the time pill. Clicking it opens `audio-panel` anchored under the button; clicking it again or moving focus away hides the panel.
- Focusing or typing in search opens `search-panel`; Ctrl+K focuses the input and opens the panel.
- Pinned folder click opens `stack-popup` anchored below the pin.
- Right-clicking a pin opens a native Tauri popup menu with `Open` and `Unpin`; native menu is required because a Svelte menu would be clipped by the 26px top-bar webview.
- Dragging folders from the search panel, native Explorer/Tauri file drops, or compatible HTML drag payloads can pin folders.
- Pinned folder rail supports drag reorder with persisted order.
- Wheel over rail translates vertical wheel movement to horizontal scroll.
- Keyboard in pin rail supports left/right navigation and Enter/Space activation.
- Newly added pins should be revealed immediately after pinning, but startup hydration must not auto-scroll to the last persisted pin.

### Svelte/TypeScript implementation details

- Main component: `src/components/TopBar.svelte`.
- Styling: `src/components/TopBar.css`.
- Pin hydration invariant: initial `loadStackPins()` applies backend pins without recursively reloading pins. `applyStackPins()` updates state, reveals only explicitly/newly added pins after initial hydration, then ticks and updates rail scroll affordances. This prevents `applyStackPins -> loadStackPins -> applyStackPins` recursion and keeps startup from auto-scrolling to the last persisted pin.
- Search helpers:
  - `src/lib/searchPanel.ts` wraps `show_search_panel`, `hide_search_panel`, `publish_search_panel`, `get_search_panel_payload`, and `open_shell_path`.
  - `src/lib/settingsPanel.ts` wraps `show_settings_panel` and `hide_settings_panel` for the top-left JasonShell settings dropdown.
  - `src/lib/audio.ts` wraps `show_audio_panel`, `hide_audio_panel`, audio panel events, audio state reads, master volume, per-app volume, and default input/output device commands.
  - `src/lib/searchCatalog.ts` merges pinned launcher/open-window/system results into a search catalog.
  - `src/lib/searchRanking.ts` ranks results and records usage.
  - `src/lib/systemSearch.ts` wraps backend `search_system` and declares `search-index:refreshed`.
  - `src/lib/systemSearchState.ts` gates stale responses and retry-on-index-refresh behavior.
- Pin helpers:
  - `src/lib/stackPopup.ts` wraps pin/list/reorder/show stack commands and emits pin update events.
  - `src/lib/topBarPins.ts` builds the explicit `{ kind: 'WebviewWindow', label: 'top-bar' }` event target and computes added-pin reveal paths.
  - `src/lib/folderDrag.ts` normalizes folder drag/drop payloads.
  - `src/lib/taskbarMenus.ts` wraps `show_top_bar_pin_context_menu` and exports `top-bar:pin-menu-action`.
- Important state variables in `TopBar.svelte`:
  - `launchers`, `openWindows`, `systemResults` for search catalog composition.
  - `searchQuery`, `searchOpen`, `selectedIndex`, `searchStatus` for search UX.
  - `shellPreferences` for date/time formatting and search shortcut hint visibility; it updates through the local preference-change event emitted by `src/lib/shellPreferences.ts`.
  - `stackPins`, `stackPinsLoaded`, `pendingVisiblePinPath`, `focusedPinIndex` for pin rail.
  - `audioOpen` and `soundControl` for top-bar sound button `aria-expanded` state and outside-click closure; actual controls live in `AudioPanelSurface.svelte`.
  - `draggingPinPath`, `pinDropStatus`, `pinRailHover`, scroll affordance flags for drag/drop and rail UI.
- TopBar polls `listOpenTaskWindows()` every second so search window results stay fresh.
- TopBar sends runtime metrics after 250ms via `reportShellSurfaceRuntimeMetrics('top-bar')`.
- Top-bar shell-home/settings, rail scroll, pinned folder buttons, bottom-bar launcher/task/process-manager command buttons, search-panel pin-folder actions, task-preview root activation, settings-panel close/reset/done, process-manager refresh/close/sort/kill controls, control-plane section actions, and safe Stack Browser command controls are rendered through `src/components/melt/MeltActionButton.svelte`, a local wrapper around `melt/builders` `Tooltip`. The wrapper still emits real DOM `<button>` elements, forwards original click/contextmenu/dblclick/keydown/mousedown/pointer/hover/drag/drop/lostpointercapture event objects, preserves role/ARIA/data/style/value/name attributes needed by existing controls, omits `aria-label` when no explicit label is provided so text buttons keep their visible text accessible name, only attaches Melt Tooltip trigger/`aria-describedby` attributes when actual tooltip content exists, preserves `data-path={pin.path}` and `draggable="true"` for pin buttons, supports native `disabled` state, and does not replace native pin/task menus, Stack Browser/search/settings/taskbar state machines, preview IPC, process-manager anchoring, or popup IPC.
- Search data flow:
  - `loadSearchCatalog()` fetches Explorer launchers and current task windows in parallel, then `buildSearchCatalog(launchers, openWindows, systemResults)` merges local app/window results, backend system results, static shell folders, and command results.
  - `rankSearchResults()` applies query token scoring, priority, and `localStorage` usage boosts (`jasonshell.search.usage`, +8 per activation capped at 80, max 14 visible results).
  - A reactive statement publishes `{ query, results, selectedIndex, statusMessage }` whenever `searchOpen` is true. `search-panel` is not the source of result truth; it renders the latest top-bar payload.
  - `scheduleSystemSearch()` debounces backend search by 140ms, increments `systemSearchSequence`, clears stale refresh timers, and ignores queries under 2 trimmed characters. `loadSystemSearchResults()` uses `shouldApplySystemSearchResponse()` to reject stale responses and `shouldRetryIndexedSearch()` to retry empty indexed results up to two delayed refresh attempts.
  - `search-index:refreshed` restarts an active query only when the panel is open and the trimmed query has length >= 2.
- Pin data flow:
  - `listStackPins()` hydrates persisted pins without reveal side effects (`applyStackPins(nextPins, false)` on first load).
  - `pinStackFolder()`, `unpinStackFolder()`, and `reorderStackPins()` return authoritative full arrays from Rust, then `src/lib/stackPopup.ts` emits `stack-pins:updated` both globally and explicitly to `{ kind: 'WebviewWindow', label: 'top-bar' }`.
  - `pendingVisiblePinPath` is used for user-initiated add/reveal. `stackPinRevealPath(currentPins, nextPins, pendingVisiblePinPath, allowDetectedAdd)` intentionally prevents startup hydration from scrolling the rail to a previously persisted pin.
  - `applyStackPins()` must not call `loadStackPins()` after a successful reveal; doing so can reload and overwrite immediate authoritative mutation arrays.
- Drag/drop normalization:
  - `folderDrag.ts` accepts the custom `application/x-jasonshell-folder` payload, `text/uri-list`, `text/plain`, native `Files`, and Tauri `onDragDropEvent` paths. It converts `file://` URLs, UNC hosts, localhost URLs, slash-separated Windows paths, and quoted paths into normalized path strings.
  - Top bar does not validate filesystem type in Svelte; Rust `pin_stack_folder` canonicalizes and rejects non-directories. UX errors should reflect this asynchronous validation.
- Native menu interaction:
  - Right-click pin coordinates are client coordinates relative to the top-bar webview and passed to `show_top_bar_pin_context_menu` as `{ path, x, y }`.
  - `taskbar_menu.rs` base64url-encodes the path in the menu ID and emits `top-bar:pin-menu-action` to `top-bar`. TopBar handles only `open` and `unpin`; adding menu actions requires updating Rust parsing, `TopBarPinMenuActionPayload`, this spec, and tests.
- Stack popup anchoring:
  - `openStackPath()` closes search first, reads the clicked pin/search-control `getBoundingClientRect()`, and calls `showStackPopup({ folderPath, anchorLeft, anchorWidth })`. The TS wrapper translates `folderPath` to Rust `path` and adds a unique `requestId`.
  - Rust stores the normalized latest request, positions/focuses the popup, and emits `stack-popup:open`; the TS wrapper additionally emits directly to `stack-popup` as a delivery hardening duplicate.
- Settings panel anchoring:
  - `openSettingsPanel()` closes search first, reads the JasonShell button `getBoundingClientRect()`, and calls `showSettingsPanel({ anchorLeft, anchorWidth })`.
  - `src-tauri/src/settings_panel.rs` positions `settings-panel` below the top bar, aligning to the button's left edge and clamping inside the top-bar host width. The settings panel hides on focus loss, Escape, Done, or Close.
- Keyboard/wheel behavior:
  - Ctrl/Cmd+K focuses search and opens the panel. Search input ArrowUp/ArrowDown moves top-bar-owned selection; Enter activates; Escape hides.
  - Pin rail ArrowLeft/ArrowRight moves focus among pin buttons, Enter/Space opens the focused pin, and vertical wheel deltas become horizontal rail scroll unless horizontal wheel movement dominates.
- Known top-bar failure modes:
  - Stale `systemSearchSequence` acceptance can show results for an old query; always use the helper gate when changing search async paths.
  - Emitting `stack-pins:updated` only globally can fail to update a hidden/isolated top-bar listener; keep explicit `WebviewWindow` targeting.
  - Treating `folderPath` and Rust `path` interchangeably across the TS/Rust boundary can break stack-popup open requests; wrapper owns that conversion.
  - Reloading pins after every event can erase reveal intent and cause startup auto-scroll regressions.

### Rust integration

- `src-tauri/src/shell_windows.rs` creates `top-bar` and gives it title `JasonShell Top Bar`.
- `src-tauri/src/appbar.rs` positions/registers it on the top AppBar edge and captures runtime metrics.
- `src-tauri/src/search_panel.rs` positions `search-panel` based on top-bar position and search-control anchor.
- `src-tauri/src/stack_popup.rs` positions `stack-popup` based on top-bar position and clicked pin anchor.
- `src-tauri/src/audio_panel.rs` positions `audio-panel` below the top bar based on the sound button rect, clamps it within the top-bar host width, emits `audio-panel:open`, and emits `audio-panel:closed` to `top-bar` before hiding on focus loss or explicit hide.
- `src-tauri/src/taskbar_menu.rs` shows native top-bar pin context menus and emits `top-bar:pin-menu-action` back to `top-bar`.
- `src-tauri/src/stack_popup.rs` persists pinned stack folders and returns full next pin arrays from `pin_stack_folder`, `unpin_stack_folder`, and `reorder_pinned_stack_folders`.
- Static shell folder search aliases (`shell:Profile`, `shell:Desktop`, `shell:Personal`/`shell:Documents`, `shell:Downloads`) are accepted by `stack_popup.rs` path normalization and should remain usable for search folder results.
- `open_shell_path` can open search path results directly through `ShellExecuteW`; do not route folders through Stack Browser unless the user action is explicitly pin/open stack.

### Events and IPC

- `search-panel:activate`: emitted by `SearchPanelSurface.svelte`, listened by `TopBar.svelte`, activates selected result.
- `search-panel:select`: emitted by search panel row click/keyboard, listened by top bar to update selection.
- `search-panel:pin-folder`: emitted by search panel pin button, listened by top bar to pin folder and reveal it.
- `search-panel:interaction`: emitted by `SearchPanelSurface.svelte` on result-pane mouse interaction so `TopBar.svelte` can suppress the delayed input-blur close while a result click/select/pin/activate is in flight.
- `search-panel:closed`: emitted by Rust `main.rs` when `search-panel` receives `WindowEvent::Focused(false)` and is hidden; `TopBar.svelte` listens to clear `searchOpen`/`aria-expanded` state.
- `search-index:refreshed`: emitted by backend search index warmer, listened by top bar to refresh active indexed search.
- `stack-pins:updated`: emitted by `src/lib/stackPopup.ts` both globally and explicitly to `top-bar` as a `WebviewWindow` target; `TopBar.svelte` listens with the same target shape.
- `top-bar:pin-menu-action`: emitted by Rust native menu handling to `top-bar` with `{ action: 'open' | 'unpin', path }`.
- `audio-panel:open`: emitted by Rust `audio_panel.rs` after showing the hidden audio panel so `AudioPanelSurface.svelte` refreshes device/session state.
- `audio-panel:closed`: emitted to `top-bar` by Rust `audio_panel.rs` when the audio panel hides, so the sound button clears `aria-expanded`.
- `getCurrentWindow().onDragDropEvent`: top bar consumes Tauri native file drops and pins dropped folders.

### Top bar tests

- `tests/topBarPins.test.mjs`: event target shape and pin reveal helper behavior.
- `tests/folderDrag.test.mjs`: native/HTML folder drag recognition and path normalization.
- `tests/stackBrowserTopBarPinFlow.test.mjs`: regression for Stack Browser pinning requiring immediate top-bar update with authoritative backend mutation arrays.
- `tests/meltMigrationWiring.test.mjs`: source-wiring regression for local Melt wrappers and migrated `MeltActionButton` usage, including preservation of `button[data-path]`, draggable pin buttons, native top-bar pin context menu flow, `showStackPopup` flow, bottom-bar launcher/task/process-manager command handlers, task group wrapper attributes, `.task-button` focus selector behavior, process-manager anchor rect flow, search-panel pin action wiring, task-preview activation/key/hover behavior, settings/control/process command controls, Stack Browser safe command controls, and explicit raw exclusions for Stack Browser row buttons, delete-cancel focus target, and resize grip.
- `tests/frontendUiPolicy.test.mjs`: source policy regressions for no active gradients in app/component UI and Stack Browser path/sort wiring. `tests/audioControls.test.mjs` covers top-bar sound button source wiring, the dedicated `audio-panel` webview route, dialog control wiring, immediate audio command calls, and stable audio wrapper command constants. `tests/stackPopupState.test.mjs` covers typed-path commit-after-validation success/failure behavior and sort-header helper state.
- Search-related top-bar state is partly covered by `tests/searchPanelState.test.mjs` and `tests/systemSearch*` helpers when present in the test bundle.
- `tests/overlayDismissalWiring.test.mjs`: source-wiring regression for Stack Browser in-webview delete confirmation and search-panel outside-dismiss/interaction/closed event handshake.

### Known risks

- Live Tauri multi-webview delivery should still be smoke-tested after pin-event changes; unit tests verify helper shape and source wiring but not all WebView2 runtime delivery edge cases.
- Native Explorer drag cursor feedback depends on Tauri/WebView2 file drop behavior and should be validated manually on Windows.
- Native top-bar popup placement must be checked on scaled displays and near screen edges because menu placement is OS-owned.
- Search panel/top-bar selection is split across two webviews: the panel emits selected IDs, but top bar maps IDs back into its current `searchResults`. If result identities change mid-flight, selection may no-op safely rather than activating an unintended row.

## Bottom bar/taskbar spec

### User behavior

- Stays docked on the primary monitor bottom edge at roughly normal taskbar height.
- Shows Explorer-pinned taskbar `.lnk` launchers from `%APPDATA%\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar`.
- Launches pinned apps by executing the shortcut itself through Windows Shell APIs.
- Shows currently open primary-monitor task windows.
- Groups windows by process name, preserving a local preferred order while visible keys change.
- Click behavior toggles focus/minimize through `activate_task_window(hwnd, was_active)`.
- Hover behavior opens `task-preview` after `TASK_PREVIEW_DELAY_MS`; moving away schedules hide after `TASK_PREVIEW_HIDE_DELAY_MS`.
- Right-click task window opens native menu with focus/restore, minimize, and close.
- Right-click launcher opens native menu with launch and reveal shortcut location.
- Pointer drag reorders task groups after a 6px threshold; click-vs-drag suppression prevents accidental activation after a drag release.
- Activity/busy indicator is gated to terminal/LLM/developer/browser-download-like workloads to avoid noisy generic CPU flicker.
- A rightmost process-manager button opens the `process-manager` popup anchored above the button; the popup closes on Escape, Close, or focus loss.

### Svelte/TypeScript implementation details

- Main component: `src/components/BottomBar.svelte`.
- Styling: `src/components/BottomBar.css`.
- Launcher wrapper: `src/lib/taskbarLaunchers.ts` wraps `list_pinned_taskbar_apps` and `launch_pinned_taskbar_app`.
- Window wrapper: `src/lib/taskbarWindows.ts` wraps `list_open_task_windows` and `activate_task_window`.
- Menu wrapper: `src/lib/taskbarMenus.ts` wraps native menu commands.
- Preview wrapper: `src/lib/taskbarPreview.ts` wraps `show_task_window_preview` and `hide_task_window_preview`.
- Process-manager wrapper: `src/lib/processManager.ts` wraps `show_process_manager`, `hide_process_manager`, `list_processes`, `kill_process`, and exports `process-manager:open` / `process-manager:closed` event constants.
- UI constants and labels: `src/lib/taskbarUi.ts` defines refresh events and preview timing constants.
- Grouping/reorder math: `src/lib/taskbarGroups.ts` defines group keys, activity eligibility, drag thresholds, reorder offsets, and target computation.
- Pointer click suppression: `src/lib/taskbarTilePointer.ts` distinguishes click activation from drag/release suppression.
- `BottomBar.svelte` polls `refreshTaskbarWindows()` every second and listens for refresh events from native menu actions.
- `BottomBar.svelte` sends runtime metrics after 250ms via `reportShellSurfaceRuntimeMetrics('bottom-bar')`.
- `BottomBar.svelte` opens the process manager from the rightmost `.process-manager-button` by reading that button's `getBoundingClientRect()` and calling `showProcessManager({ anchorLeft, anchorWidth })`.
- Bottom-bar launcher `.launcher-button`, task window `.task-button`, and `.process-manager-button` controls are `MeltActionButton` instances, so they are Melt-backed tooltipped controls while remaining real DOM buttons for CSS selectors, disabled state, `event.currentTarget` geometry, `.task-button` keyboard focus queries, native context menus, preview hover timing, pointer down state, and click suppression. Task group wrappers remain plain `div.task-group` elements with their class flags, `data-task-group-key`, `data-window-count`, `role="group"`, inline order/transform style, and pointer capture/reorder handlers.
- Launcher flow:
  - `loadPinnedLaunchers()` calls `list_pinned_taskbar_apps`; backend skips unsupported/unresolvable shortcuts. Clicks call `launch_pinned_taskbar_app(shortcutPath)` and set `launchingShortcutPath` to suppress duplicate launches.
  - Launch failure removes the failing launcher from the visible array for the current session and refreshes windows, but does not mutate Explorer's pin folder.
- Open-window refresh/filter flow:
  - `refreshTaskbarWindows()` calls `list_open_task_windows`, then `buildTaskWindowGroups(nextWindows, taskGroupOrder)` and replaces `taskGroupOrder` with currently visible group keys. Lost windows are removed from preferred order; new visible keys append in backend order.
  - `BottomBar.svelte` listens for `taskbar:refresh-windows` and `taskbar:refresh-launchers` emitted after native menu actions, and still polls every second for external changes.
- Grouping/reorder algorithm:
  - `taskWindowGroupKey()` lowercases trimmed process name; empty process names fall back to `window:${hwnd}`.
  - `buildTaskWindowGroups()` folds windows into process groups, deriving group activity/minimized/active flags. Busy state is only surfaced if backend says `activityState === 'busy'` and frontend eligibility heuristics agree.
  - Pointer drag begins only on left button and only when not activating a window. `TASKBAR_GROUP_DRAG_THRESHOLD_PX = 6`; below threshold, release can still activate the originally pressed tile. Above threshold, release suppresses click activation and commits `taskbarGroupOrderFromDisplacement()`.
  - Drag calculations use captured group rects and moved center-point displacement, not live DOM index mutation alone. This prevents rightward neighbor crossing from being dropped when pointer capture is available.
- Click-vs-drag activation state machine:
  - `pendingTaskbarTilePointer(event.button, hwnd)` records only left-button candidates.
  - `resolveTaskbarTilePointerRelease(pendingHwnd, dragStarted)` returns either an activation HWND or a suppress-click HWND; `shouldSuppressTaskbarTileClick()` consumes suppressions on the subsequent click event.
  - `toggleWindow()` hides preview first, calls `activate_task_window(hwnd, isActive)`, then refreshes windows. Backend minimizes only when the window was active/foreground and not already minimized.
- Preview lifecycle:
  - Hover schedules `show_task_window_preview` after `TASK_PREVIEW_DELAY_MS`; leaving schedules hide after `TASK_PREVIEW_HIDE_DELAY_MS`.
  - `previewRequestId` increments for every show/hide. Rust stores `latest_request_id` and drops stale capture results if a newer request arrived while DWM/GDI capture was running.
  - `task-preview:hover-enter` cancels hide when pointer moves into the preview webview. Preview click/Enter/Space calls `maximize_task_window`, emits `taskbar:refresh-windows`, and hides itself.

### Rust integration

- `src-tauri/src/launchers.rs` enumerates `.lnk` pins, validates shortcut paths, resolves shell links in an STA thread, extracts icons, and launches via `ShellExecuteW`.
- `src-tauri/src/task_windows/windows.rs` enumerates top-level HWNDs with `EnumWindows`, filters candidates, extracts title/class/process, captures icon data, derives minimized/active state, computes activity state, and stable-sorts by HWND.
- `src-tauri/src/task_windows/actions.rs` owns focus/minimize/maximize/close operations.
- `src-tauri/src/task_windows/previews.rs` captures preview images and DWM extended frame bounds for task previews.
- `src-tauri/src/task_preview.rs` positions and publishes task preview data to the `task-preview` window.
- `src-tauri/src/taskbar_menu.rs` builds Tauri native popup menus for task windows and launchers, handles selected actions, then emits refresh events to `bottom-bar`.
- `src-tauri/src/process_manager.rs` sizes/positions the process-manager popup relative to the bottom-bar monitor and process-manager button anchor, enumerates processes, derives process CPU from cached process-time deltas, and owns kill guardrails.
- `src-tauri/src/appbar.rs` positions/registers the bottom bar on the bottom AppBar edge and may hide/restore Explorer's own primary taskbar.
- `launchers.rs` runs shortcut enumeration/launch in a dedicated STA thread (`CoInitializeEx(COINIT_APARTMENTTHREADED)`), validates that requested `.lnk` files canonicalize under Explorer's pinned taskbar directory, resolves shell links with `SLR_NO_UI | SLR_NOSEARCH | SLR_NOTRACK`, extracts explicit icon locations first, then target/file icons, and launches the shortcut itself through `ShellExecuteW`.
- `task_windows/windows.rs` enumerates HWNDs with `EnumWindows`, builds candidates with title/class/process path/process name/owner/ex-style/monitor/cloak/active/minimized metadata, filters down to primary-monitor app-like windows, then sorts by numeric HWND for stable backend order.
- `task_windows/icons.rs` reads task icons using `WM_GETICON` (`ICON_SMALL2`, `ICON_SMALL`, `ICON_BIG`) with a 25ms `SendMessageTimeoutW`, then class icons, then process file icons; unresponsive windows fall back safely.
- `task_windows/previews.rs` rejects hung or minimized windows, prefers `DWMWA_EXTENDED_FRAME_BOUNDS`, falls back to `GetWindowRect`, captures with `BitBlt(SRCCOPY | CAPTUREBLT)`, normalizes BGRA to RGBA, and scales to max 320x180 before PNG encoding.
- `task_windows/actions.rs` uses parsed HWND strings only. Close sends `WM_CLOSE` with `SendMessageTimeoutW(SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT)`, polls `IsWindow` for up to roughly 500ms, falls back to `PostMessageW(WM_CLOSE)` when the HWND remains, then returns an explicit error if the window still exists. This makes elevated/protected/vetoing windows visible to the UI as close failures instead of silent success. Focus/restore/maximize/minimize use `ShowWindowAsync` and `SetForegroundWindow` best-effort semantics.

### DWM/no-activate filtering

- `src-tauri/src/task_windows/windows.rs` rejects windows that are not visible/minimized, are not on the primary monitor, belong to JasonShell/current process, have owners, are DWM cloaked (`DWMWA_CLOAKED`), are tool windows without `WS_EX_APPWINDOW`, have `WS_EX_NOACTIVATE`, lack a visible title unless explicitly forced onto the taskbar by `WS_EX_APPWINDOW`, are known Explorer/shell/internal classes (`Shell_TrayWnd`, `Shell_SecondaryTrayWnd`, `Progman`, `WorkerW`, `Dwm`), have title `DWM Notification Window`, or process name `dwm`.
- `src-tauri/src/task_windows/tests.rs` contains tests for the filtering rules, including no-activate windows, DWM notification windows, empty-title helper suppression, explicit `WS_EX_APPWINDOW` inclusion, and close retry/failure helper behavior.
- Filtering deliberately rejects `WS_EX_NOACTIVATE` windows so no-activate overlays/tooltips/widgets do not become taskbar tiles. `WS_EX_TOOLWINDOW` is rejected unless paired with `WS_EX_APPWINDOW`.
- DWM cloaking is checked with `DWMWA_CLOAKED`; cloaked UWP/virtual desktop/window-manager artifacts must stay out of the task strip.
- Primary monitor restriction is explicit via `MonitorFromPoint(0,0, MONITOR_DEFAULTTOPRIMARY)` and `MonitorFromWindow(..., MONITOR_DEFAULTTONULL)`. Multi-monitor parity requires a new ownership model and should not be implied by small filter edits.
- Activity detection stores process CPU-time/title snapshots by HWND in a static `OnceLock<Mutex<HashMap<...>>>`, prunes invisible HWNDs after each enumeration, and only reports `Busy` for terminal/LLM/browser-download-like workloads to avoid constant CPU flicker.

### Events and menus

- `taskbar:refresh-windows`: emitted to `bottom-bar` after task-window menu actions; listened by `BottomBar.svelte`.
- `taskbar:refresh-launchers`: emitted to `bottom-bar` after launcher menu actions; listened by `BottomBar.svelte`.
- `task-preview:hover-enter`: emitted by the preview surface to keep hover preview open when the pointer enters preview.
- `process-manager:open`: emitted by Rust directly on the `process-manager` webview after `show_process_manager` sizes, positions, shows, and focuses it; `ProcessManagerSurface.svelte` starts polling and refreshes immediately.
- `process-manager:closed`: emitted by Rust on explicit hide or focus loss; `ProcessManagerSurface.svelte` stops its refresh interval and marks the popup closed.
- Native menu IDs are encoded with prefixes in `taskbar_menu.rs`: `task-window`, `launcher`, and `top-bar-pin`.
- Launcher menu payloads use base64url to safely encode paths inside menu item IDs; reveal validates `.lnk` extension and canonical parent directory before invoking Explorer `/select`.

### Bottom bar tests

- `tests/taskbarGroups.test.mjs`: group construction, order reconciliation, drag math, activity gating.
- `tests/taskbarTilePointer.test.mjs`: click-vs-drag activation suppression behavior.
- `tests/processManagerWiring.test.mjs`: source-level routing/command/process-manager button wiring, including the visible Start Time column wired to `startTimeMs` metadata, process icon payload exposure, and aggregate metric header wiring.
- `tests/processManagerState.test.mjs`: process sort toggling, Applications/Background/Windows grouping, metric/start-time sorting, CPU/memory/GPU/start-time formatter behavior, and aggregate CPU/memory/GPU/thread metric calculation.
- Rust task-window tests in `src-tauri/src/task_windows/tests.rs`: candidate filtering and activity state helpers.
- Rust process-manager tests in `src-tauri/src/process_manager.rs`: kill guardrails and CPU-percent snapshot math.
- Additional validation through `npm run cargo:test`, `npm run check`, and `npm run validate`.

### Known risks

- Live taskbar behavior still depends on Win32 window enumeration quirks, multi-process UWP/modern app behavior, monitor placement, and DWM states.
- This milestone targets the primary monitor only; multi-monitor taskbar parity is not implemented despite feature-document references to broader Cairo behavior.
- Launcher parity with Explorer is conservative: unsupported/unresolvable pins are skipped.

## Visual system and accessibility baseline

- Global visual tokens live in `src/app.css` with `--js-*` variables for text, surfaces, borders, accent, semantic status colors, radii, spacing, shadows, focus ring, scrollbar colors, and motion durations.
- Open Sans is the default application font. Local WOFF2 assets downloaded from Google Fonts live under `src/assets/fonts/open-sans/`; `src/app.css` declares `@font-face` entries for Latin and Latin Extended subsets and initializes `--js-font-sans` to the Open Sans stack.
- `src/app.css` owns shared focus-visible treatment, form/button transition defaults, scrollbar styling, `.surface-state` status classes (`loading`, `info`, `warning`, `error`), reduced-motion overrides, and high-contrast/forced-colors token substitutions.
- Surface CSS should prefer tokens from `src/app.css` over hard-coded colors, radii, shadows, spacing, or scrollbar colors. Active app/component UI styling uses uniform token colors such as `--js-bg-surface`, `--js-bg-bar`, `--js-bg-control`, and `--js-bg-active`; gradients and `background-image` are intentionally blocked by source tests. When extracting Svelte-scoped styles into a global CSS file, scope selectors to the surface root class to avoid leaking generic class names such as `.surface` into other webviews.
- Shell-wide themes are `data-theme` token sets on `document.documentElement`, implemented by `src/lib/themes.ts` and `src/app.css`. The registry includes `base-dark`, `base-light`, `monokai`, `atom-one-dark`, `atom-one-light`, `nord`, `dracula`, `solarized-dark`, `solarized-light`, `github-dark`, `github-light`, `gruvbox-dark`, `gruvbox-light`, `tokyo-night`, `catppuccin-mocha`, and `ayu-dark`.
- Theme application is renderer-only and must not move icons, tabs, bars, popup anchors, or component ordering. The selected theme is stored in `localStorage` under `jasonshell.theme`, applied before Svelte mount in `src/main.ts`, then kept synchronized across all Tauri webviews by `installShellThemeSync()` using `storage` events plus `BroadcastChannel` name `jasonshell.theme.changed` when available. BroadcastChannel failure is tolerated because theme persistence is cosmetic.
- `ControlPlaneSurface.svelte` hosts the theme picker in the existing toolbar so the top bar, bottom bar, task tiles, and stack/search/process layouts retain their existing positions. The picker is `src/components/melt/MeltSelect.svelte`, calls `setShellTheme()` only, does not invoke privileged settings IPC, and does not extend the authority-light Control Plane model. Control-plane section navigation uses `melt/builders` `Tabs` for roving tab semantics while the summary cards remain bounded renderer data from `controlPlaneState.ts`.
- Shell-wide renderer preferences are implemented by `src/lib/shellPreferences.ts` and surfaced in `settings-panel`. Preferences are stored in `localStorage["jasonshell.uiPreferences"]`, synchronized by `BroadcastChannel("jasonshell.uiPreferences.changed")` plus `storage` events, and applied before Svelte mount in `src/main.ts`. Current preferences include `fontId`, `dateFormat`, `use24HourTime`, `showSeconds`, `compactDensity`, `strongFocusRing`, `reducedTransparency`, and `showSearchShortcutHint`.
- `SettingsPanelSurface.svelte` uses shared Melt-backed `MeltSelect` controls for theme/font, shared Melt-backed `MeltToggle` controls for boolean preferences, and shared Melt-backed `MeltRadioGroup` controls for date-format presets. Those wrappers are intentionally renderer-only, styled by JasonShell CSS tokens, and do not introduce new Tauri commands, persistence files, or cross-webview event contracts.
- Date format strings are renderer-only and support tokens `yyyy`, `yy`, `MMMM`, `MMM`, `MM`, `M`, `dd`, `d`, `EEEE`, and `EEE`. Unknown characters are preserved as literals after control-character stripping and length bounding. The default date format is `EEE, MMM d`.
- `ProcessManagerSurface.svelte` uses shared Melt-backed `MeltProgress` meters for CPU and memory bars while preserving the existing process grid, row order, guarded kill actions, taskbar-active badges, and Rust-owned process data contracts.
- Search panel, Stack Browser, and Task Preview styles are split into `SearchPanelSurface.css`, `StackPopupSurface.css`, and `TaskPreviewSurface.css`; their Svelte files import the CSS files explicitly.
- Accessibility baseline:
  - Top-bar rail scroll buttons are real named buttons and the pinned-folder toolbar exposes horizontal orientation.
  - Top-bar search input exposes popup state via `aria-haspopup="listbox"` and `aria-expanded`.
  - Search panel result list owns `aria-activedescendant`; folder pin buttons have explicit labels.
  - Stack Browser grid and status regions expose busy/status state; breadcrumbs mark the current crumb; delete confirmation uses an in-webview `role="dialog"` with focusable controls instead of native `window.confirm`.
  - Process Manager uses a grid with sortable column headers and matching row gridcells, including the action cell.
  - Task Preview is a real button, not a generic div with button role.

## Process manager popup spec

### User behavior

- `process-manager` is a hidden persistent Tauri webview opened by the rightmost button in `bottom-bar`.
- It displays a compact Task Manager-like table with sortable `Name`, `PID`, `CPU`, `Memory`, `GPU`, `Start Time`, `Threads`, `Status`, and `Action` columns. `CPU`, `Memory`, and `GPU` headers show aggregate visible-row percentages above the label; metric values are centered under their headers.
- Default sort is CPU descending; metric columns, including `startTimeMs`, default to descending while `name` defaults to ascending. Unknown numeric/start-time values sort last under the default descending direction.
- Taskbar-active processes are promoted ahead of background-only processes for every sort. They show a compact `taskbar` badge, with foreground taskbar windows highlighted through the same badge.
- Automatic refreshes while sorted by volatile metrics (`CPU`, `Memory`, or `Threads`) update row values in place without continuously reordering existing rows; user sort changes, explicit refreshes, popup open, and post-kill refreshes can reapply the selected order.
- Rows show process executable path in the row title when available. Rows can also expose best-effort command line, parent process name, child/descendant counts, listening TCP ports, and workspace/path hints. Protected/elevated processes may omit path, command line, ports, memory, CPU, or start-time metadata and display `—` through frontend formatters.
- `Kill` is enabled only when Rust marks a process killable. Kill attempts are two-step confirmed in the frontend and must include backend guardrail confirmation data when invoking `kill_process`; bare direct-IPC PID kills are rejected.
- Tree termination is plan-only and non-executing. JasonShell may warn about descendants/workspace-owned processes, but the shipped `kill_process` command terminates only the confirmed target process.
- The popup refreshes at `REFRESH_INTERVAL_MS = 1_000` only while open; Escape, Close, explicit hide, or native focus loss closes the popup and stops polling.

### Svelte/TypeScript implementation details

- UI: `src/components/ProcessManagerSurface.svelte` and `src/components/ProcessManagerSurface.css`.
- IPC wrapper and event constants: `src/lib/processManager.ts`.
- Sort/format helpers: `src/lib/processManagerState.ts`.
- UX/tree/kill-plan helpers: `src/features/process-manager/processManagerUxState.ts`.
- `ProcessInfo` payload fields are camelCase in TS: `{ pid, parentPid?, parentName?, name, iconDataUrl?, executablePath?, commandLine?, listeningPorts?, cpuPercent?, memoryBytes?, memoryPercent?, gpuPercent?, threadCount?, startTimeMs?, childProcessCount?, descendantProcessCount?, workspaceHint?, taskbarWindowCount?, taskbarActive?, taskbarForeground?, taskbarTitles?, status, isKillable }`. The `taskbar*` fields are frontend enrichment from task-window metadata, not persisted backend process payload fields. `iconDataUrl` is backend process payload metadata when executable-path icon lookup succeeds and stays nullable for protected/system processes. `memoryPercent` is computed by Rust against total physical memory; `gpuPercent` is sampled from Windows PDH `\GPU Engine(*)\Utilization Percentage` counters, parsed by `pid_...` instance names, summed by PID, and rendered as unavailable only when the counter family or process sample is unavailable.
- `ProcessManagerSurface.svelte` listens for `process-manager:open` and `process-manager:closed`, tracks `isOpen`, `isLoading`, `inFlightRequest`, `killingPid`, local `sortState`, and `statusMessage`. It computes visible-row aggregate metrics through `aggregateProcessMetrics()` and renders process icons before the tree indent/name text.
- Refresh is concurrency-gated: if `isLoading` is true, a new refresh returns early; `inFlightRequest` rejects stale responses before replacing rows or status.
- Each refresh loads `list_processes` plus best-effort lightweight `list_taskbar_process_windows`; task-window metadata includes `processId` so `enrichProcessesWithTaskbarWindows()` can mark taskbar-active process rows without invoking the full icon/activity taskbar enumeration used by the bottom bar.
- `orderProcessRefresh()` keeps taskbar-active rows ahead of inactive rows. For automatic volatile-metric refreshes it preserves existing relative row order inside active/inactive buckets while replacing the metric values, reducing visual jumpiness under changing CPU/memory/thread samples. `buildProcessTreeRows(..., { promotedRootPids })` is used so taskbar-active child processes remain readable as top-level rows instead of being buried under inactive parents.
- Table formatting uses `formatProcessCpu`, `formatProcessMemoryPercent`, `formatProcessMemory`, `formatProcessGpu`, and `formatProcessStartTime`. CPU percentages come from Rust process CPU-time deltas against a shared observation timestamp and logical processor count; memory percentage comes from Rust total-memory normalization; GPU percentages come from PDH GPU Engine utilization counters; start time is visible in the UI and sortable through the same `startTimeMs` column already supplied by Rust.
- `killProcess(pid, confirmation?)` wraps `kill_process`; the UI builds confirmation from the current backend-enriched row/plan so direct IPC cannot bypass descendant/workspace/tree guardrails.

### Rust implementation details

- Backend: `src-tauri/src/process_manager.rs`.
- Commands registered in `src-tauri/src/main.rs`: `show_process_manager`, `hide_process_manager`, `list_processes`, and `kill_process`.
- Window label/dimensions live in `src-tauri/src/shell_windows.rs`: `PROCESS_MANAGER_LABEL = "process-manager"`, `PROCESS_MANAGER_WIDTH_LOGICAL = 720.0`, `PROCESS_MANAGER_HEIGHT_LOGICAL = 520.0`.
- `show_process_manager` finds the `process-manager` and `bottom-bar` windows, clamps popup size to the current/primary monitor, anchors the right edge to `anchorLeft + anchorWidth`, positions it above the bottom bar with physical margins, shows/focuses it, and emits `process-manager:open` on the popup.
- `hide_process_manager` emits `process-manager:closed` then hides the popup.
- `main.rs` also handles `WindowEvent::Focused(false)` for `process-manager` by emitting `process-manager:closed` and hiding the popup so the frontend stops polling when focus leaves.
- Windows process enumeration uses `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)`, `Process32FirstW`/`Process32NextW`, limited `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`, `QueryFullProcessImageNameW`, `K32GetProcessMemoryInfo`, and `GetProcessTimes`. Process icons are extracted from executable paths through the shared task-window shell icon helper and cached by executable path in `PROCESS_ICON_DATA_URLS`.
- CPU percent is derived from process-time deltas stored in `PROCESS_CPU_SNAPSHOTS: OnceLock<Mutex<HashMap<u32, ProcessCpuSnapshot>>>` and pruned to current PIDs after each enumeration.
- Start time is `GetProcessTimes` creation time converted from Windows FILETIME ticks to Unix milliseconds and serialized as `startTimeMs`.
- Process enrichment derives parent names, child/descendant counts, best-effort command lines, listening TCP ports, and workspace hints without persisting snapshots.
- Task-window metadata from `src-tauri/src/task_windows/mod.rs` includes `process_id` serialized to `processId`; Process Manager uses lightweight `list_taskbar_process_windows` output only to prioritize/display taskbar-active rows.
- `kill_process` rejects PID 0 and JasonShell's current PID before attempting `OpenProcess(PROCESS_TERMINATE)` / `TerminateProcess`; it also requires `ProcessKillConfirmation` acknowledging the backend plan mode, descendant count, and workspace warning state. Tree-plan execution and stale descendant/workspace confirmations are rejected. Non-Windows returns an unsupported error and `list_processes` returns an empty list.

### Process manager tests and risks

- `tests/processManagerState.test.mjs` covers sort toggles/default directions, CPU/memory/GPU/start-time formatting, aggregate visible-row metric calculation, taskbar-active priority/grouping, stable volatile refresh ordering, volatile-column detection, and metric/start-time ordering with unknown values last under descending sort.
- `tests/processManagerUxState.test.mjs` covers filtering, process-tree rows, taskbar-window enrichment, safe kill button states, developer summary text, and kill-tree plan guardrails.
- `tests/processManagerWiring.test.mjs` verifies app/surface/command/button wiring, task-window process ID wiring, source-level exposure of visible process metadata, process icon payload wiring, aggregate metric headers, stable refresh ordering, and confirmed kill calls.
- `cargo test --manifest-path src-tauri/Cargo.toml process_manager` covers kill guardrail planning/execution, stale confirmation rejection, workspace warning acknowledgement, tree-plan rejection, and CPU-percent helper math.
- `cargo test --manifest-path src-tauri/Cargo.toml task_windows` covers task-window candidate filtering/activity helpers and now compiles the `process_id` task-window fixture contract.
- Residual risks: protected/elevated processes can omit metadata; command-line/port discovery is best-effort and Windows-specific; first CPU sample is unknown until a second snapshot; process list polling is one-second best-effort rather than real-time; live Tauri smoke remains needed for exact anchored placement/focus-loss close behavior on scaled or multi-monitor setups.

## Stack Browser / stack popup spec

### User behavior

- One hidden `stack-popup` webview is created at startup and reused for every top-bar pin.
- Opening a pin anchors the popup below the clicked pin/top bar; the popup focuses its details grid.
- Popup preserves Svelte history state while hidden, so navigating folders and hiding/reopening does not reset history unless a new request changes current path behavior.
- Shows a details grid with sortable `name`, `type`, `size`, and `modified` columns, with folders grouped before files.
- Supports Back, Forward, Up, breadcrumb navigation, reload, type-to-select, multi-selection, Select All, and retained rows during loading.
- File/folder metadata badges include hidden, system, read-only, symlink, and reparse-point state.
- Folder reads are paged: initial page limit is 80, subsequent background pages are 500.
- Large folder first paint keeps prior rows visible but non-interactive until the first real page for the requested folder arrives.
- Mouse buttons 4/5 drive back/forward navigation where WebView2 delivers them.
- Supports copy, cut, paste, delete, reveal in Explorer, open, file-only `Open with ▸` with installed useful app candidates plus `Choose app...`, pin, inline rename, inline new folder, New Text File, Open Terminal Here, and copy path actions.
- Supports native and HTML drag/drop: stack rows can carry stack path payloads; dropped paths can be pasted/copied into the current folder, and dragging file rows to Windows Explorer starts a Shell OLE copy drag rather than relying on clipboard mirroring.
- Delete confirmation and delete execution keep the popup visible; after confirmed delete, the current folder is refreshed so deleted entries disappear in-place. The popup still closes on Escape or ordinary click-away/focus loss.
- A bottom-right resize grip lets users enlarge or shrink the popup within monitor-clamped bounds; the final size persists across opens and restarts.

### Svelte/TypeScript implementation details

- UI: `src/components/StackPopupSurface.svelte`.
- Command wrapper and event constants: `src/lib/stackPopup.ts`.
- Reducer/state helpers: `src/lib/stackPopupState.ts`.
- Context menu viewport placement: `src/lib/contextMenuPosition.ts`.
- File/folder icon fallback mapping: `src/lib/stackFileIcons.ts`.
- Folder drag helpers: `src/lib/folderDrag.ts`.
- `StackPopupSurface.svelte` listens for `stack-popup:open` on the current window and also polls `getStackPopupRequest()` every 250ms to harden request delivery.
- Request de-duplication uses request keys and pending/handled request markers.
- Folder load sequence IDs prevent stale pages from replacing the active request.
- Row/background context menus are positioned after render and clamped/flipped into the viewport.
- Row context menus keep root `Open`; file rows expose `Open with ▸` as a hover/focus submenu whose app candidates are loaded through `listStackOpenWithCandidates(path)` and whose `Choose app...` action calls `openStackItemWithPicker`. The submenu has a CSS bridge between parent item and flyout so rightward pointer movement does not dismiss it while crossing the gap.
- Rename/new-folder use inline editor input; input pointer/key events must not bubble to popup-level selection/activation handlers.
- The resize grip is rendered by `StackPopupSurface.svelte` as `.stack-resize-grip`. Pointer moves call `resizeStackPopup(width, height, persist=false)` through `requestAnimationFrame`; pointer up/cancel repeats the request with `persist=true`.
- Popup request delivery:
  - `StackPopupSurface.svelte` listens for `stack-popup:open` on `getCurrentWindow()` and calls `getStackPopupRequest()` immediately after listener installation.
  - It also polls `getStackPopupRequest()` every 250ms. This is intentional redundancy for hidden webview delivery races and must not be removed without live multi-webview proof.
  - `stackPopupOpenPath()` accepts legacy string payloads, `{ path }`, and `{ folderPath }`; `stackPopupRequestKey()` prefers `requestId` and falls back to `legacy:${path}`. `lastHandledOpenRequestKey` and `pendingOpenRequestKey` prevent duplicate handling.
- Reducer/state model:
  - `defaultStackPopupViewState` owns `currentPath`, `entriesPath`, sorted `entries`, `sortColumn`, `sortDirection`, single and multi-selection, `selectionAnchorPath`, history list/index, and status.
  - `openStackFolder()` trims and appends to history unless opening the current history item, and always clears selection while retaining prior rows until pages arrive.
  - `applyStackEntries()` rejects listings whose folder path does not match current path, sorts entries, intersects selection with visible paths, and updates `entriesPath`. `stackPopupHasRetainedRows()` is true when rows are visible but belong to a previous folder.
  - `mergeStackFolderListings()` resets aggregation if path changes or page offset <= 0; otherwise appends entries and warnings. This prevents stale pages from unrelated folders from merging.
- Paging semantics:
  - `listStackFolder()` requests an initial page of 80 and subsequent pages of 500. It invokes optional `onPage` with every page and stops if `hasMore` is false or if backend returns no offset progress.
  - `folderLoadSequence` in the Svelte component rejects stale page callbacks/final results after a newer `loadFolder()` begins.
  - During retained-row loading, row selection, drag, context menu, type-to-select, and keyboard movement are disabled to prevent actions against the wrong folder.
- Navigation and selection:
  - Back/Forward use reducer history cursor; Up/Backspace use `parentStackPath()`. XButton mouse buttons 3/4 map to back/forward on `svelte:window` mousedown when available.
  - Selection supports single, Ctrl/Cmd toggle, Shift range from `selectionAnchorPath`, Select All, Home/End/PageUp/PageDown, and type-to-select with a 700ms buffer.
  - Sorting keeps folders before files for all columns; column toggles asc/desc, then compares by name/type/nullable size/nullable modified using locale numeric comparison. `stackSortHeaderState()` is the single helper for active sort class, `aria-sort`, and visible indicator; inactive columns render no arrow, active ascending renders up, and active descending renders down.
- Context menus and inline editing:
  - Row/background menus are Svelte-rendered because Stack Browser has enough height; top-bar pin menus remain native because 26px clipping makes Svelte menus unsuitable there.
  - `positionContextMenuInViewport()` measures after render, clamps/flips menus into visible viewport, and `rowSubmenuOpensLeft` flips the Open With submenu when insufficient right-side width remains.
  - Row menu always exposes root `Open`; file rows enable `Open with >` app candidates plus `Choose app...`. Background menu exposes Paste, New Folder, New Text File, Copy Folder Path, Open Terminal Here, and selection-aware actions when selection exists.
  - Inline editor form uses `on:click|stopPropagation` and `on:mousedown|stopPropagation` on its input, and keydown ignores popup-level handlers for `.inline-editor` except Escape. Preserve this isolation when changing editing UI.
- Drag/drop and clipboard:
  - Stack rows publish `application/x-jasonshell-stack-paths`, `text/plain`, `text/uri-list`, and `DownloadURL` payloads. `handleRowDragStart()` also calls `prepareStackFileDrag(paths)` so Windows file rows can hand off to the Rust OLE drag source for Explorer copy drops. Drop `shiftKey` selects move/cut; otherwise copy.
  - `lastHtmlDropAt` suppresses duplicate Tauri native file-drop handling for 500ms after an HTML drop.
  - `pasteDroppedPaths()` writes selected paths to the internal/native clipboard through `copyStackItems(paths, move)` and then calls `pasteStackItems(destinationPath)`; it refreshes current folder or reloads when dropping into a child folder.
- File-operation UX:
  - Operations catch and surface first/fallback error text through `operationErrorMessage()`. Paste can partially succeed and surfaces `Paste completed with N failures: first...`. New Text File creates the next available `New Text Document.txt` / `New Text Document (n).txt` under the current folder and selects it after refresh.
  - Delete starts a Stack Browser focus-loss hold before the selected-path loop, keeps that hold through source-folder refresh and details-grid refocus, then releases it. This keeps confirmed delete visible through the full delete-and-refresh cycle while preserving normal Escape/click-away dismissal afterward. `delete_stack_item` also has a per-command nested guard for direct IPC safety.
  - Pin selected folder calls `pinStackFolder()`; top bar update depends on the wrapper emitting authoritative pins.

### Rust implementation details

- Backend: `src-tauri/src/stack_popup.rs`.
- Runtime state: `StackPopupRuntimeState { latest_request, clipboard, focus_loss_hold_count, restore_focus_after_hold }`.
- Persistence files under the app local data directory: `stack-folders-v1.json` for pinned folders and `stack-popup-geometry-v1.json` for the last user-resized Stack Browser logical size.
- `PinnedStackFolder`: `{ id, name, path }`.
- `ShowStackPopupRequest`: `{ path, anchorLeft, anchorWidth, requestId? }`.
- `StackPopupLogicalSize`: `{ width, height }`.
- `StackItem`: `{ path, name, kind, typeLabel, iconDataUrl?, sizeBytes?, modifiedAt?, isHidden, isReadonly, isSystem, isSymlink, isReparsePoint }`.
- Pinned-folder mutations return the complete next `Vec<PinnedStackFolder>` to avoid stale frontend list reloads.
- Pin persistence uses temp-file-and-rename semantics and backs up corrupt JSON before falling back.
- `show_stack_popup` normalizes the request, stores latest request, sizes and positions the popup under the top bar, shows/focuses it, and emits `stack-popup:open` directly on the popup window. Size uses persisted geometry when present and is clamped to the current monitor and available height below the top bar.
- `resize_stack_popup` receives logical `width`, `height`, and `persist`; it clamps the size against the current monitor and popup position, calls `set_size`, and writes `stack-popup-geometry-v1.json` only when `persist` is true.
- `read_stack_folder` normalizes existing directories and returns `StackFolderPage` with `offset`, `limit`, `total`, `hasMore`, and partial warnings.
- `open_stack_item_with_picker` normalizes an existing file path, rejects directories, and delegates to `shell_paths::open_shell_path_with_picker`; on Windows that uses ShellExecuteW with the `openas` verb to display the OS Open With picker.
- `list_stack_open_with_candidates` normalizes an existing file path, rejects directories, and returns installed candidate apps from `open_with.rs` for supported extensions. Text/developer files prefer Notepad, Notepad++, and Visual Studio Code when resolvable; images include Paint/VS Code; unsupported app IDs are not emitted by the backend.
- `open_stack_item_with_app` normalizes an existing file path, rejects directories, resolves the app ID through the same candidate registry, and launches the selected executable with the file path as a literal argument.
- `prepare_stack_file_drag` normalizes existing paths and starts a Windows Shell OLE drag (`SHCreateDataObject` + `SHDoDragDrop` with `DROPEFFECT_COPY`) through `native_drag.rs`; it reports `mechanism: "ole-do-drag-drop"`. This is the native Explorer copy-drag path and is separate from CF_HDROP clipboard copy/cut.
- `new_stack_text_file` creates a collision-safe text file in the selected/current folder. `open_stack_terminal_here` opens Windows Terminal at the folder when available, then falls back to PowerShell and `cmd`.
- Rename rejects root paths, invalid child names, separators, and collisions.
- Paste implements Explorer-style collision names with `- Copy (n)` suffixes and preserves unresolved failures.
- Copy/cut store internal runtime clipboard and attempt native Windows file clipboard interoperability where available.
- `StackPopupRuntimeState` stores `latest_request`, an internal `StackClipboard`, and transient delete focus-loss guard flags. It is not a durable history store.
- Path normalization accepts trimmed/quoted paths, `file://` URIs, localhost file URIs, UNC file URIs, extended Windows prefixes (`\\?\`, `\\?\UNC\`, `\??\`, `\??\UNC\`), and supported `shell:` aliases. Canonicalization is required for existing paths; display strings strip extended prefixes.
- Pin defaults are Desktop and Downloads from `USERPROFILE`/`HOME` and are written only when the pin store does not exist. Corrupt pin JSON is renamed to `stack-folders-v1.json.corrupt-<millis>` and an empty/default set is used.
- Pin writes use pretty JSON and atomic temp-write/rename. On Windows `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)` is used with documented UTF-16 pointer lifetimes.
- Stack popup geometry writes use pretty schema/version-tagged JSON and the same temp-write/rename replacement pattern. Corrupt geometry JSON is backed up to `stack-popup-geometry-v1.json.corrupt-<millis>` before falling back to default sizing.
- Pinned-folder identity is path-based: `id == path`, duplicate detection is case-insensitive on canonical paths, and stale/offline unpin falls back to raw normalized path keys so unavailable pins can still be removed.
- `read_stack_folder_page()` reads all child summaries, sorts folders first by lowercase name, then materializes only requested page items. Per-entry failures become warnings rather than failing the entire listing.
- `StackItem` metadata is symlink/reparse-aware: symlink/reparse targets are probed for kind/size while path/name preserve the original link path; badges expose hidden, readonly, system, symlink, and reparse state.
- `validate_child_name()` rejects empty names, separators, trailing dot/space, control characters, Windows-illegal characters, and reserved DOS device basenames (`CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`) even on non-Windows test paths.
- Paste rejects copying a real directory into itself/descendants, rejects symlink/reparse copy for now, uses Explorer-style ` - Copy (n)` collision suffixes up to 999, falls back from `fs::rename` to copy+delete for moves, and preserves unresolved cut clipboard failures.
- Windows delete uses Recycle Bin through `SHFileOperationW(FO_DELETE | FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI)` outside tests; tests/non-Windows use permanent delete helpers.
- `begin_stack_popup_focus_loss_hold` and `end_stack_popup_focus_loss_hold` let the frontend hold focus-loss dismissal across the full confirmed delete loop and listing refresh. `delete_stack_item` accepts `AppHandle` and `StackPopupRuntimeState`, increments a nested per-command focus-loss hold before filesystem deletion, decrements it afterward, and refocuses `stack-popup` when a guarded focus-loss event was suppressed.
- Native clipboard interop uses CF_HDROP plus `Preferred DropEffect` for copy/cut/paste. `Copy` maps to effect 1; `Cut` maps to effect 2; reading effect with move bit set maps to cut. Native Explorer drag-out uses Shell OLE drag and does not use clipboard as its primary transfer mechanism.
- `open_stack_item_with_picker` normalizes an existing path, rejects directories, and calls `ShellExecuteW` with `openas`. Generic `open_stack_item` delegates to default `open_shell_path`.

### Stack Browser tests

- `tests/stackPopupState.test.mjs`: history, branch navigation, stale payload rejection, retained rows, selection, sorting, formatting, request-key behavior.
- `tests/contextMenuPosition.test.mjs`: viewport clamp/flip placement helpers.
- `tests/overlayDismissalWiring.test.mjs`: source-level regression for in-webview delete confirmation, delete-time focus-loss hold wiring, search-panel dismissal handshake, and persisted Stack Browser resize grip/command wiring.
- `tests/stackPopupContextMenu.test.mjs`: source-level regression that rejects misleading `Open width`/`Default width` labels, verifies the `Open with` picker/app-candidate wrapper is wired, verifies the submenu bridge, native-friendly file drag payloads, and background developer actions.
- `tests/folderDrag.test.mjs`: folder drag/drop payload behavior.
- `tests/stackBrowserTopBarPinFlow.test.mjs`: pin mutation publication to top bar.
- `cargo test --manifest-path src-tauri/Cargo.toml stack_popup`: Rust helper coverage for rename validation, folder ordering/metadata, pin persistence/reorder/corrupt-backup, resize geometry clamp/persistence, paste collision naming, clipboard mode behavior, Open With candidate resolution, new text file naming, and native drag mechanism contract.

### Known risks

- Large-folder rows are virtualized in the frontend via `src/lib/stackPopupViewModel.ts` / `src/features/stack-browser/viewModel.ts`. Keyboard and type-to-select movement must call the scroll-into-view helper so the selected row stays mounted in the virtual window.
- External file-system changes require explicit reload or operation-triggered refresh; there is no long-lived watcher.
- Live WebView2 geometry/input smoke remains useful for native drag cursor, XButton delivery, and context-menu placement on scaled displays.
- Stack Browser intentionally has no long-lived filesystem watcher. If adding one, preserve retained-row semantics, stale page rejection, virtual row mount behavior, and existing reducer/unit tests.
- Copying symlinks/reparse points is deliberately unsupported to avoid unsafe recursive/cross-volume behavior. Do not silently follow these without a cycle/reparse policy and tests.

## Search panel/index spec

### User behavior

- Search input lives on `top-bar`; rich results render in a separate `search-panel` webview anchored under the search control.
- Result kinds: `app`, `window`, `folder`, `file`, `command`.
- Results are keyboard navigable; top-bar owns selected index and Enter activation.
- Search panel rows support click select, double-click activate, Enter activate, Space select.
- Folder results expose a `Pin` button and can be dragged to the top-bar pin rail.
- Status messages distinguish loading, empty, indexed search, unavailable, and refresh states.

### Frontend implementation

- `src/components/SearchPanelSurface.svelte` renders result rows, selected state, icons, folder pin buttons, and draggable folder payloads.
- `src/lib/searchPanel.ts` defines payload types, result types, event constants, and IPC wrappers.
- `src/lib/searchPanelState.ts` applies payloads and decides when to reveal selected rows.
- `src/lib/searchCatalog.ts` combines local launcher/window/command results with backend indexed results.
- `src/lib/searchRanking.ts` ranks results and records usage.
- `src/lib/systemSearch.ts` calls `search_system` and maps `SystemSearchResult` into `SearchPanelResult`.
- `src/lib/systemSearchState.ts` suppresses stale responses and coordinates retry after index refresh.
- Query flow:
  - Top bar owns query and selected index. Search panel owns only payload rendering and emits row intents by result id/path.
  - Search result activation path is: top bar `activateResult()` -> record usage -> branch by `system:*`, `app:*`, `window:*`, `folder`/`file`, or command id -> clear query -> close panel -> reload local catalog.
  - Folder search results can be pinned via `search-panel:pin-folder` or dragged using `setFolderDragPayload()`. Pinning goes through `pinStackFolder()` so Rust validates the path and publication carries the full authoritative pin array.
- Panel payload ownership:
  - `publish_search_panel` stores arbitrary JSON in `SearchPanelRuntimeState.latest_payload` and emits `search-panel:update` to the `search-panel` label.
  - `SearchPanelSurface.svelte` listens for the event, fetches latest payload on mount, and polls every 120ms. The polling fallback is intentional because the panel can be hidden or not yet listening when top bar publishes.
- Result identity conventions:
  - Pinned apps: `app:${shortcutPath}` and launch by stripping `app:` back to the shortcut path.
  - Open windows: `window:${hwnd}` and activate by matching current `openWindows`.
  - System index/provider results: `system:${kind}:${path}` with explicit `path`; `isSystemPathResult()` checks this prefix and path presence.
  - Static folders: `folder:${shellAlias}` with `path` equal to `shell:Profile`, `shell:Desktop`, `shell:Personal`, or `shell:Downloads`.
  - Commands: `command:refresh-search` and `command:hide-search`.

### Rust implementation

- `src-tauri/src/search_panel.rs` owns search panel geometry, show/hide, latest-payload storage, and `search-panel:update` event emission.
- `src-tauri/src/search_sources.rs` is the public command facade; it trims queries below length 2 and delegates to the persistent index.
- `src-tauri/src/search_sources/index.rs` owns the warmed app/file index and persisted cache file `search-index-v1.json`.
- `src-tauri/src/search_sources/apps.rs`, `files.rs`, `scoring.rs`, and `windows_search.rs` provide source collection, scoring, and Windows Search/SystemIndex integration where available.
- Search index warming emits `search-index:refreshed` when relevant so TopBar can refresh active search without recursively scanning on each keystroke.
- `search_system` trims input and returns an empty result set for trimmed query length < 2.
- `search_sources/index.rs` keeps in-memory `entries`, `provider_results_by_query`, `provider_queries_in_flight`, `loaded_cache`, `refreshing`, and `refreshed_at` behind a `Mutex`.
- Refresh TTL is 300 seconds. `ensure_refresh()` loads cache once if present, emits `search-index:refreshed` for cache availability, builds a fresh app/file index on a background thread, writes `search-index-v1.json`, updates memory, and emits another refresh event with `{ entryCount, generatedAtEpochSecs }`.
- Local index scope is intentionally bounded: apps max 4,000 results, app directory visits max 8,000 per root; files max 25,000 results, directory visits max 12,000 per root. Skip directories include `$Recycle.Bin`, VCS dirs, `AppData`, caches, build output, `node_modules`, `target`, temp dirs.
- Windows Search provider path is per-query, cached by normalized query, and in-flight de-duplicated. Provider results merge before local snapshot and are de-duplicated by lowercased id. Provider failure is cached for 30 seconds before retry.
- Windows Search uses `SystemIndex` through `ISearchManager`/query helper, OLE DB `Search.CollatorDSO.1`, selected columns `System.ItemUrl`, `System.ItemPathDisplay`, `System.FileName`, `System.ItemTypeText`, `System.KindText`, max provider rows clamped 1..100, and maps rows to `app`/`folder`/`file` with rank-derived priority.
- COM handling in Windows Search accepts `RPC_E_CHANGED_MODE` as already initialized and skips `CoUninitialize` in that case; otherwise it initializes STA with `COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE` and uninitializes on drop.

### Tests and risks

- `tests/searchPanelState.test.mjs` covers search panel state application and selection reveal behavior.
- Rust tests under `search_panel.rs` and `search_sources/*` cover anchoring, payload storage, scoring, indexing, and Windows Search fallback where compiled.
- Known risks: Windows Search availability varies by machine; index fallback behavior must remain fast and non-blocking. Avoid turning each keypress into broad recursive filesystem scans.
- Provider fallback currently discards the textual reason at the facade level; user-facing status remains generic. Keep this unless a richer non-noisy diagnostic UX is designed.

## Workspace and developer tooling spec

### Workspace core behavior

- Workspace profiles are durable, validated shell context records persisted through `settings.rs` inside `jasonshell-settings-v1.json`.
- Workspace schema fields are camelCase over IPC and Rust uses `WorkspaceProfile`: `id`, `name`, `rootPath`, `aliases`, `pins`, `toolDefaults`, `tasks`, `startup`, and `restoration`.
- Workspace activation updates `ui.activeWorkspaceId` in settings and returns a `WorkspaceActivationPlan` instead of directly changing unrelated surfaces. The plan includes layout identity, search bias roots/aliases, top-bar pins, exposed tasks, startup non-execution reason, and reserved window/app restoration status.
- If a workspace has no explicit pins, activation exposes the workspace root as a default top-bar pin plan. Explicit pins are validated absolute local paths.
- Workspace startup is safe-by-default. `startup.mode` can be `manualOnly` or `suggestOnly`; activation never executes startup commands. `willExecute` must remain false until a later explicit task-runner UX is designed.
- Window/app restoration is reserved, not implemented. `restoration.status` must remain `reserved-not-implemented`.
- Workspace data must not persist secrets. Env names/values and task/startup args reject secret-like keys/values such as token/secret/password/credential/api key/authorization/cookie markers and common bearer/token prefixes.

### Developer tooling and task execution

- `src-tauri/src/dev_tools/tool_plans.rs` builds terminal/editor launch plans as argv arrays and does not execute them. Template substitution is limited to known placeholders such as workspace path and file path/line; shell metacharacters in executable templates and path traversal placeholders are rejected.
- `src-tauri/src/dev_tools/git_status.rs` reads git repository state for a workspace path and reports clean/dirty, branch/upstream/head, ahead/behind, conflicts, merge, and rebase flags.
- `src-tauri/src/dev_tools/task_runner.rs` owns JasonShell-launched task execution and bounded task history in `jasonshell-task-history-v1.json`.
- `src-tauri/src/automation.rs` is a contract/validation layer only; it does not persist data and the single-instance forwarding contract remains `planned-not-wired`.
- `src-tauri/src/providers.rs` resolves in-memory provider registry contracts only; it does not persist provider config, execute plugin code, or store provider secrets.
- `spawn_workspace_task` is intentionally identity-only. The IPC request accepts `workspaceId` and `taskId`; unknown executable/args fields are denied, and Rust resolves the executable/args/cwd from the currently active persisted workspace declaration before spawning. Direct renderer-supplied arbitrary commands must stay rejected.
- Task spawning uses `Command` without a shell, pipes stdout/stderr, emits `task:started`, bounded `task:output`, and `task:completed`, and records bounded history. Output chunks are bounded and task output sequence numbers are monotonic across streams for a task.
- `cancel_workspace_task` kills the direct child process and marks the task canceled. Descendant tree cancellation is not implemented; future work must add an explicit Windows process-tree policy and tests before claiming full tree cancellation.
- `list_jasonshell_task_process_metadata` exposes currently running JasonShell task process metadata so Process Manager can associate workspace-owned task processes without persisting PID snapshots.

### Developer providers and Stack Browser action planning

- `src/features/search/developerProviders.ts` aggregates bounded developer provider results for workspace files, recent files, git changes, task history, commands, settings, and processes.
- Developer providers must be bounded per source and across the merged result set. Active workspace matches are ranked above external matches, and providers can be scoped to the active workspace when requested.
- Saved-search contracts include scope metadata so a saved search can be global or workspace-specific without guessing at activation time.
- `src/lib/stackPopupViewModel.ts` provides context-action plans for editor launch, terminal launch, copy path, template creation, and git-aware operations. These helpers plan actions for UI rendering; destructive git restore remains confirmation-gated and is not executed by the planning helper.

### Workspace/tooling tests and risks

- `tests/workspaces.test.mjs` covers workspace wrapper command names, activation top-bar/search/task plans, search biasing, and non-executing startup summaries.
- `tests/devTools.test.mjs` covers centralized IPC/event contracts, safe terminal/editor launch requests, identity-only task request helpers, task history/process metadata ranking, and command registration/capability source checks.
- `tests/developerProviders.test.mjs` covers provider bounds, active-workspace ranking, filtering, grouping, and saved-search scope contracts.
- `tests/stackBrowserContextActions.test.mjs` covers Stack Browser editor/terminal/copy/template/git-aware action plans.
- `tests/automationProviders.test.mjs` covers frontend automation forwarding assertions, safe CLI request shapes, provider budget normalization, provider secret rejection, and provider executable/plugin rejection.
- `tests/controlPlaneState.test.mjs` covers control-plane view-model sections, bounded rendering, provider budgets, keyboard tab navigation, component authority limits, and secret redaction for key/value, flag/value, bearer, and token-shaped values.
- `tests/controlPlaneRouting.test.mjs` covers `control-plane` routing, shell surface metadata, show/hide wrapper command names, backend command/source registration, and per-window capability presence.
- `cargo test --manifest-path src-tauri/Cargo.toml workspaces` covers workspace schema/path/startup/restoration/secret validation and activation plans.
- `cargo test --manifest-path src-tauri/Cargo.toml dev_tools` covers argv template expansion, git parsing/temp-repo status, declared task resolution, direct arbitrary command rejection, bounded task history, output chunk/sequence behavior, and spawn/cancel helpers.
- `cargo test --manifest-path src-tauri/Cargo.toml layout` covers Phase 9 pure multi-monitor planning helpers for mixed-DPI shell strips, primary/secondary monitor ownership, popup anchoring, and task-strip monitor assignment.
- `cargo test --manifest-path src-tauri/Cargo.toml automation` covers explicit local automation opt-in, read-only/mutating/destructive boundaries, confirmation phrases, arbitrary executable payload rejection, and plan-only single-instance forwarding.
- `cargo test --manifest-path src-tauri/Cargo.toml providers` covers provider registry budgets, duplicate ids, secret-like config rejection, executable/plugin config rejection, and enum-bounded provider types.
- `cargo test --manifest-path src-tauri/Cargo.toml control_plane` covers control-plane size clamping and monitor-centered positioning helpers.
- Residual risks: live Windows/Tauri smoke is still needed for VS Code/Windows Terminal launch-plan handoff UX, real task output event delivery, direct-child cancellation behavior under wrapper processes, dirty/clean git status in large local repos, actual multi-monitor mixed-DPI hardware behavior, live control-plane show/focus placement, and real single-instance CLI forwarding once it is wired beyond the current plan-only contract.

## Backend/Rust command and module map

### Command registrations in `src-tauri/src/main.rs`

- Launchers: `list_pinned_taskbar_apps`, `launch_pinned_taskbar_app`.
- Task windows: `list_open_task_windows`, `activate_task_window`, `maximize_task_window`.
- Task preview: `show_task_window_preview`, `hide_task_window_preview`.
- Menus: `show_task_window_context_menu`, `show_launcher_context_menu`, `show_top_bar_pin_context_menu`.
- Search panel: `show_search_panel`, `hide_search_panel`, `publish_search_panel`, `get_search_panel_payload`.
- Process manager: `show_process_manager`, `hide_process_manager`, `list_processes`, `kill_process`.
- Audio controls: `show_audio_panel`, `hide_audio_panel`, `get_audio_state`, `list_audio_devices`, `list_audio_sessions`, `set_master_volume`, `set_master_volume_percent`, `set_master_mute`, `set_app_volume`, `set_app_session_volume_percent`, `set_app_session_mute`, `set_default_audio_device`, `set_default_audio_input_device`, `set_default_audio_output_device`.
- Control plane: `show_control_plane`, `hide_control_plane`.
- System search: `search_system`.
- Shell paths: `open_shell_path`.
- Stack popup: `list_pinned_stack_folders`, `pin_stack_folder`, `unpin_stack_folder`, `reorder_pinned_stack_folders`, `show_stack_popup`, `hide_stack_popup`, `get_stack_popup_request`, `begin_stack_popup_focus_loss_hold`, `end_stack_popup_focus_loss_hold`, `resize_stack_popup`, `read_stack_folder`, `open_stack_item`, `open_stack_item_with_picker`, `list_stack_open_with_candidates`, `open_stack_item_with_app`, `rename_stack_item`, `copy_stack_items`, `prepare_stack_file_drag`, `cut_stack_items`, `paste_stack_items`, `delete_stack_item`, `new_stack_folder`, `new_stack_text_file`, `open_stack_terminal_here`, `reveal_stack_item`.
- Settings: `load_shell_settings`, `save_shell_settings`.
- Workspaces: `list_workspaces`, `create_workspace`, `update_workspace`, `delete_workspace`, `activate_workspace`.
- Developer tools/tasks: `build_terminal_launch_plan`, `build_editor_launch_plan`, `get_workspace_git_status`, `spawn_workspace_task`, `cancel_workspace_task`, `list_workspace_task_history`, `list_jasonshell_task_process_metadata`.
- Automation: `parse_automation_cli`, `validate_automation_request`, `get_single_instance_forwarding_contract`.
- Provider registry: `resolve_provider_registry`.
- Diagnostics: `record_diagnostic`, `export_diagnostics`.

### Audio control implementation notes

- Frontend wrapper: `src/lib/audio.ts`; top-bar sound button wiring: `src/components/TopBar.svelte` and `src/components/TopBar.css`; audio dropdown controls: `src/components/AudioPanelSurface.svelte`.
- Native backend: `src-tauri/src/audio.rs` for Core Audio commands and `src-tauri/src/audio_panel.rs` for panel positioning/show/hide. Command registration and command constants must stay synchronized across `src-tauri/src/main.rs`, `src-tauri/src/contracts.rs`, and `src/ipc/commands.ts`.
- `audio-panel` is a hidden persistent webview routed through `src/App.svelte`/`src/lib/shellSurface.ts`. Do not render the full sound dropdown inside `top-bar`; content below the 26px AppBar webview is clipped and cannot be used reliably.
- `get_audio_state` returns camelCase `{ masterVolumePercent, outputDevices, inputDevices, defaultOutputDeviceId, defaultInputDeviceId, sessions }`. Device rows include `id`, `name`, `flow`, `isDefault`, and `state`; session rows include `id`, `name`, `processId`, `volumePercent`, `muted`, and `state`.
- Master volume uses documented `IAudioEndpointVolume`; per-application session volume uses `IAudioSessionManager2` and `ISimpleAudioVolume` for the default render endpoint.
- Default input/output switching uses the narrow Windows `IPolicyConfig` COM bridge with CLSID `870af99c-171d-4f9e-af0d-e63df40c2bc9` and IID `f8679f50-850a-41cf-9c72-430f290290c8`, setting `eConsole`, `eMultimedia`, and `eCommunications` roles for the selected active endpoint. Keep this bridge small because it is an undocumented Windows interface even though it is the practical route for immediate default-device switching.
- Runtime metrics: `report_shell_surface_runtime_metrics`.

### Rust modules

- `main.rs`: Tauri builder, state management, command registration, window lifecycle cleanup.
- `shell_windows.rs`: webview creation, labels, logical dimensions, context-menu suppression script.
- `appbar.rs`: Windows AppBar reservation, work-area setting, Explorer taskbar hiding/restoration, startup stabilization, runtime metrics.
- `explorer.rs`: Explorer taskbar snapshot/hide/restore helpers.
- `layout.rs`: shell bar rectangle calculation.
- `launchers.rs`: Explorer pinned taskbar shortcut enumeration, icon extraction, shortcut launch.
- `task_windows/mod.rs`: public task-window structs and command facade.
- `task_windows/windows.rs`: HWND enumeration/filtering/activity state.
- `task_windows/actions.rs`: focus/minimize/maximize/close task-window actions.
- `task_windows/icons.rs`: shell/file/window icon extraction and PNG data URL conversion.
- `task_windows/previews.rs`: DWM preview capture.
- `task_preview.rs`: preview window positioning/state.
- `taskbar_menu.rs`: native context menus and menu event routing.
- `search_panel.rs`: search panel positioning, visibility, latest payload.
- `process_manager.rs`: process-manager positioning/show/hide, process enumeration, CPU snapshot deltas, start-time conversion, and guarded termination.
- `control_plane.rs`: control-plane show/hide commands, monitor-centered positioning, and size clamping.
- `automation.rs`: safe local automation CLI parsing/validation, explicit opt-in boundary, read-only/mutating/destructive action levels, confirmation phrases, and plan-only single-instance forwarding contract.
- `providers.rs`: config-driven provider registry contract with enum-bounded provider types, per-provider budgets, duplicate-id rejection, secret-like config rejection, and arbitrary executable/plugin config rejection.
- `contracts.rs`: backend command/event/surface constants and Rust tests for command/event uniqueness.
- `diagnostics.rs`: bounded backend diagnostics ring buffer with recursive field/text redaction and export command.
- `settings.rs`: versioned shell settings load/save/migration, corrupt-file backup, atomic write, and secret-key rejection.
- `workspaces.rs`: workspace profile schema, CRUD/activation commands, path/task/startup validation, secret-like workspace data rejection, activation plan construction, and reserved restoration enforcement.
- `dev_tools/mod.rs`: developer-tool module root.
- `dev_tools/tool_plans.rs`: safe terminal/editor argv launch-plan construction without execution.
- `dev_tools/git_status.rs`: git status parsing and repository state detection for workspace paths.
- `dev_tools/task_runner.rs`: declared workspace task execution, output events, cancellation, process metadata, and bounded task-history persistence.
- `search_sources.rs` and `search_sources/*`: app/file/Windows Search persistent indexing and search result scoring.
- `shell_paths.rs`: safe shell path opening and Windows Open With picker launch via ShellExecuteW `openas`.
- `stack_popup.rs`: facade for Stack Browser commands and runtime state. Implementation is split under `src-tauri/src/stack_popup/` into models, paths, items, paging, file operations, pins, clipboard, native drag, Open With, and popup-window responsibilities while preserving command names and payload shapes.
- `system_tray.rs`: parked/experimental Windows notification-area relay prototype. It is compiled only for Windows Rust tests via `#[cfg(all(target_os = "windows", test))]` and its Tauri commands are intentionally not registered in `main.rs`.

### Backend implementation contracts and invariants

- Serialization convention: Rust request/response structs exposed to frontend use `#[serde(rename_all = "camelCase")]` unless they are intentionally raw `serde_json::Value` pass-through payloads (`publish_search_panel`). Frontend wrappers should use camelCase fields.
- Error convention: Tauri commands return `Result<T, String>` with user/action-oriented context. Native helper functions wrap OS failures with the operation and target when practical. Avoid `unwrap`/`expect` in command paths except for poisoned internal mutexes where continued execution is not meaningful.
- HWND convention: frontend and JSON payloads represent HWNDs as decimal strings. Rust parses to `isize` then `HWND`; never expose raw pointer-shaped values to TS.
- Path convention: commands that act on existing filesystem entries canonicalize or validate in Rust. Frontend normalization is for UX/data-transfer compatibility only and is not a security boundary.
- Unsafe/FFI boundary rules:
  - Every `ShellExecuteW`, `MoveFileExW`, `SHFileOperationW`, clipboard, COM/OLE DB, DWM, GDI, AppBar, and window-message call must keep UTF-16 buffers/structs alive for the entire call and document pointer/lifetime assumptions when the safety is not obvious.
  - GDI objects/DCs/icons/accessors/row handles/clipboard handles must be released on all success/failure paths. Existing modules use explicit cleanup, RAII `Drop`, or post-call destruction; preserve that pattern.
  - Do not block UI event handling with long COM/filesystem scans on the main thread. Launchers use STA worker threads; search indexing/provider lookup uses background threads; preview capture is gated by request id.
- COM/threading:
  - Shell link enumeration/launch is run in a spawned STA thread in `launchers.rs` and joins before returning the command result.
  - Windows Search provider initializes COM in its worker context and tolerates pre-initialized incompatible mode by not uninitializing.
  - Tauri command functions can be invoked concurrently; shared mutable runtime state must stay behind `Mutex` and must be held only for short critical sections.
- AppBar/work-area invariants:
  - Register AppBars before reserving, track each registered HWND immediately after `ABM_NEW`, and roll back partial registrations if later setup fails.
  - Restore Explorer taskbar and baseline work area during cleanup even if unregistering one AppBar fails; collect cleanup errors rather than early-returning.
  - `SetWindowPos` should occur after reservation and before show/stabilization, with `SWP_NOACTIVATE` to avoid stealing focus.
- Native menu invariants:
  - `taskbar_menu.rs` is the sole native menu router. Menu IDs use `task-window:<action>:<hwnd>`, `launcher:<action>:<base64url path>`, and `top-bar-pin:<action>:<base64url path>`.
  - Launcher reveal and launch both canonicalize the `.lnk` under Explorer's taskbar pin folder. Do not add arbitrary path reveal/execute menu actions without equivalent validation.
- Workspace/task invariants:
  - Workspace activation returns a plan and persists active workspace identity; it must not execute startup commands or restore windows.
  - Workspace task execution authority stays in Rust and is resolved from persisted active workspace declarations. Renderer IPC must not provide arbitrary executable/args payloads for task spawning.
  - Workspace/task/startup data must reject secret-like env names/values and secret-like task/startup argument keys or values before persistence or execution.
  - Task output streams must stay bounded and sequence-stable; task history must stay bounded and must not persist raw unbounded output.
- Automation/provider invariants:
  - Local automation must be explicitly opted in with `--allow-local-automation`; unsupported flags and arbitrary executable payload fields are rejected.
  - Read-only automation can validate after opt-in, mutating automation requires authenticated or user-present boundaries, and destructive automation requires authenticated plus user-present boundaries and an exact confirmation phrase such as `delete-workspace:<id>`.
  - Single-instance forwarding is `planned-not-wired`: it accepts argv-only contract metadata and must not execute forwarded payloads or arbitrary plugins until a later implementation supersedes this contract with tests.
  - Provider registry configuration is enum-bounded to first-party provider types and must reject secret-like keys/values plus executable/plugin loading keys such as command, script, pluginPath, dllPath, and entrypoint.
  - Provider budgets are bounded to 1..100 results and 1..500ms per provider; disabled providers remain listed but do not contribute to active total/max timeout budgets.
- Search/index invariants:
  - Queries under 2 chars must not hit provider search.
  - Index warming and provider search must be bounded and cache-aware. Adding sources requires explicit root/depth/limit/skip policy.
- Provider and local results must de-duplicate by id and sort by priority descending then title.
- Provider registry contracts must remain first-party and bounded. Do not add external executable, dynamic plugin, network, or secret-bearing provider config without a separate security design, tests, and spec update.
- Stack/file-operation invariants:
  - Mutations return authoritative updated entities or arrays. Pin mutations return complete pin arrays; rename/new folder return `StackItem`; paste returns successes/failures and wrapper reloads the folder.
  - Path aliases are resolved only in stack path normalization. Generic shell open should not interpret additional shell aliases unless the backend validates them.
  - Clipboard mirrors are volatile; failure to publish native clipboard should fail copy/cut because paste semantics would otherwise diverge from user expectation.
- Process-manager invariants:
  - Process enumeration and termination authority stay in Rust; frontend sorting/formatting must not infer killability or synthesize missing process metadata.
  - `kill_process` must require backend-verifiable confirmation for executable single-process kills and reject tree-plan execution, no-confirmation direct IPC, stale descendant counts, and unacknowledged workspace warnings.
  - CPU history is volatile and keyed by PID only for current sampling; stale PIDs are pruned after enumeration and must not be persisted.
  - Focus-loss close must emit `process-manager:closed` before hiding so the Svelte interval stops even when the native window is hidden outside the Close button path.

## Frontend JS/TS/Svelte module map

- `src/App.svelte`: label-based surface router.
- `src/components/TopBar.svelte` / `TopBar.css`: top shell rail.
- `src/components/BottomBar.svelte` / `BottomBar.css`: bottom taskbar rail.
- `src/components/SearchPanelSurface.svelte` / `SearchPanelSurface.css`: search result webview.
- `src/components/StackPopupSurface.svelte` / `StackPopupSurface.css`: stack browser webview.
- `src/components/TaskPreviewSurface.svelte` / `TaskPreviewSurface.css`: hover preview webview.
- `src/components/ProcessManagerSurface.svelte` / `ProcessManagerSurface.css`: process-manager popup webview.
- `src/components/ControlPlaneSurface.svelte` / `ControlPlaneSurface.css`: settings and developer dashboard webview.
- `src/lib/runtimeMetrics.ts`: frontend metric capture and backend reporting.
- `src/lib/shellSurface.ts`: surface metadata and label resolver.
- `src/lib/taskbarLaunchers.ts`: pinned launcher IPC wrapper.
- `src/lib/taskbarWindows.ts`: open-window IPC wrapper.
- `src/lib/taskbarGroups.ts`: grouping, activity eligibility, reorder math.
- `src/lib/taskbarTilePointer.ts`: drag-vs-click state machine.
- `src/lib/taskbarUi.ts`: labels, refresh events, preview timing.
- `src/lib/taskbarMenus.ts`: native menu IPC wrappers and top-bar pin menu event contract.
- `src/lib/taskbarPreview.ts`: preview IPC wrapper.
- `src/lib/processManager.ts`: process-manager IPC wrapper and event constants.
- `src/lib/processManagerState.ts`: process-manager sort and formatter helpers.
- `src/lib/controlPlane.ts`: control-plane show/hide IPC wrapper and label constant.
- `src/lib/automation.ts`: local automation CLI/validation/forwarding-contract types, wrappers, and safety assertions.
- `src/lib/providerContracts.ts`: provider registry types, frontend budget normalization, secret/executable config guards, and registry safety assertion.
- `src/lib/settings.ts`: frontend settings schema, default settings, secret-key guard, and settings IPC wrappers.
- `src/lib/themes.ts`: shell theme registry, theme normalization, pre-mount application, localStorage persistence under `jasonshell.theme`, and cross-webview `BroadcastChannel`/`storage` synchronization for renderer CSS variables.
- `src/lib/shellPreferences.ts`: renderer-only shell preference registry, Open Sans default font selection, date/time formatting helpers, localStorage persistence under `jasonshell.uiPreferences`, and cross-webview `BroadcastChannel`/`storage` synchronization for font/density/focus/transparency/search-hint preferences.
- `src/lib/workspaces.ts`: workspace profile/activation types, workspace IPC wrappers, top-bar pin/task plan helpers, search-bias helper, and startup non-execution summary.
- `src/lib/devTools.ts`: terminal/editor launch-plan wrappers, git status wrapper, declared task execution wrapper, task-history/process-metadata types, task event constants, and workspace-derived helper builders.
- `src/lib/searchPanel.ts`: search panel payload/event/IPC contract.
- `src/lib/settingsPanel.ts`: top-left settings dropdown show/hide IPC wrapper for `settings-panel`.
- `src/lib/searchPanelState.ts`: search panel view-state reducer.
- `src/lib/searchCatalog.ts`: local result catalog composition.
- `src/lib/searchRanking.ts`: search result ranking/usage.
- `src/lib/systemSearch.ts`: indexed system search IPC wrapper.
- `src/lib/systemSearchState.ts`: stale response/retry gates.
- `src/lib/stackPopup.ts`: stack popup and stack pin IPC/event wrapper.
- `src/lib/stackPopupState.ts`: stack browser state reducer.
- `src/lib/stackPopupViewModel.ts`: Stack Browser virtual-row, breadcrumb-overflow, delete-prompt, context-action plan, and scroll-into-view helpers.
- `src/lib/stackFileIcons.ts`: icon fallback logic.
- `src/lib/contextMenuPosition.ts`: menu clamp/flip math.
- `src/lib/folderDrag.ts`: folder and stack path drag/drop normalization.
- `src/lib/systemTray.ts`: frontend normalization/click-request helper for the parked tray prototype; covered by Node tests but not wired to shipped Tauri commands or UI.
- `src/ipc/commands.ts`, `src/ipc/events.ts`, `src/ipc/surfaces.ts`, `src/ipc/diagnostics.ts`: shared frontend IPC command/event/surface constants and frontend diagnostics ring-buffer/redaction helpers. Production wrappers should import `IPC_COMMANDS` rather than embedding command string literals.
- `src/features/top-bar/*`, `src/features/bottom-bar/*`, `src/features/search/*`, `src/features/stack-browser/*`, `src/features/process-manager/*`, `src/features/control-plane/*`: pure feature seams for UX state, grouping, developer providers, virtualization, process-manager tree/kill planning, control-plane dashboard summaries, and view-model helpers.

### Frontend module ownership categories

- Surface components with Svelte state and DOM/event ownership:
  - `TopBar.svelte`: search query/selection, local catalog refresh, pin rail view/drag/reveal state, search and stack popup orchestration.
  - `BottomBar.svelte`: launcher/window loading state, task group order, preview timers, click-vs-drag transient state, native menu triggers.
  - `StackPopupSurface.svelte`: folder browser history/selection/sort/load state, context menus, inline editor, drag/drop, keyboard navigation.
  - `SearchPanelSurface.svelte`: render latest payload, selected row reveal, row activation/selection/pin/drag intent events.
  - `TaskPreviewSurface.svelte`: render latest preview payload, maximize/hide interactions.
  - `ProcessManagerSurface.svelte`: open/closed refresh lifecycle, process list sorting/display, stale refresh rejection, kill status, and close behavior.
  - `ControlPlaneSurface.svelte`: bounded, read-only dashboard rendering from passed-in settings/workspace/git/task/process/provider data, section filtering, ARIA tab semantics, keyboard section navigation, and secret-redacted text output.
  - `SettingsPanelSurface.svelte`: renderer-only app settings dropdown for live theme selection, font selection, date/time formatting, density/focus/transparency/search-hint preferences, reset, Done, Close, and Escape dismissal.
- Thin Tauri IPC/event wrappers:
  - `taskbarLaunchers.ts`, `taskbarWindows.ts`, `taskbarMenus.ts`, `taskbarPreview.ts`, `processManager.ts`, `controlPlane.ts`, `automation.ts`, `providerContracts.ts`, `searchPanel.ts`, `settingsPanel.ts`, `systemSearch.ts`, `stackPopup.ts`, `runtimeMetrics.ts`.
  - These modules should stay boring: define exported types, constants, and `invoke`/`emit` wrappers. Use `IPC_COMMANDS` from `src/ipc/commands.ts` for command names. Add tests when wrapper behavior includes event target construction, payload translation, pagination, or publication side effects.
- Pure reducers/helpers covered by Node tests:
  - `stackPopupState.ts` (`tests/stackPopupState.test.mjs`) for history, request keys, stale/retained rows, sorting, selection, breadcrumbs, formatting.
  - `searchPanelState.ts` (`tests/searchPanelState.test.mjs`) for payload application and selected-row reveal decisions.
  - `taskbarGroups.ts` (`tests/taskbarGroups.test.mjs`) for grouping, order reconciliation, drag displacement, activity eligibility.
  - `taskbarTilePointer.ts` (`tests/taskbarTilePointer.test.mjs`) for click-vs-drag release suppression.
  - `processManagerState.ts` (`tests/processManagerState.test.mjs`) for process sort/default direction and CPU/memory/start-time formatting.
  - `controlPlaneState.ts` (`tests/controlPlaneState.test.mjs`) for dashboard section assembly, bounded summary rendering, secret redaction, provider budget display, filtering, and keyboard action mapping.
  - `shellPreferences.ts` (`tests/shellPreferences.test.mjs`) for Open Sans defaults, preference normalization/application, date/time formatting, and cross-webview sync cleanup.
  - `folderDrag.ts` (`tests/folderDrag.test.mjs`) for drag payload URI/path normalization.
  - `topBarPins.ts` (`tests/topBarPins.test.mjs`) for explicit WebviewWindow event target and reveal-path logic.
  - `contextMenuPosition.ts` (`tests/contextMenuPosition.test.mjs`) for viewport clamp/flip math.
  - `stackFileIcons.ts` is pure fallback presentation logic; add tests if icon categorization becomes behaviorally significant.
- Mixed helpers:
  - `searchRanking.ts` is pure scoring plus browser `localStorage`; tests should mock/storage-isolate if expanded.
  - `searchCatalog.ts` is pure result construction; update tests if result ids/kinds/commands change because top-bar activation depends on these exact ids.

## Event contracts and IPC boundaries

| Event/Command | Direction | Payload | Notes |
| --- | --- | --- | --- |
| `search-panel:update` | Rust/app -> `search-panel` | `SearchPanelPayload` | Latest payload is also stored in Rust for polling fallback. |
| `search-panel:activate` | `search-panel` -> app/global | result id string | TopBar activates matching result. |
| `search-panel:select` | `search-panel` -> app/global | result id string | TopBar updates selected index. |
| `search-panel:pin-folder` | `search-panel` -> app/global | folder path string | TopBar pins and reveals folder. |
| `search-index:refreshed` | Rust/app -> app/global | none/unspecified | TopBar refreshes active indexed query. |
| `stack-popup:open` | Rust/TS -> `stack-popup` | `{ path, anchorLeft, anchorWidth, requestId? }` | Popup also polls latest request for delivery hardening. |
| `stack-pins:updated` | TS -> global and `top-bar` WebviewWindow target | `StackPin[]` | Must carry authoritative backend mutation result. |
| `top-bar:pin-menu-action` | Rust menu -> `top-bar` | `{ action, path }` | `action` currently `open` or `unpin`. |
| `taskbar:refresh-windows` | Rust menu -> `bottom-bar` | unit | Refresh open windows after task menu actions. |
| `taskbar:refresh-launchers` | Rust menu -> `bottom-bar` | unit | Refresh launchers after launcher menu actions. |
| `task-preview:hover-enter` | preview -> app/global | unit | BottomBar cancels scheduled preview hide. |
| `process-manager:open` | Rust -> `process-manager` | unit | ProcessManagerSurface starts polling and refreshes immediately. |
| `process-manager:closed` | Rust -> `process-manager` | unit | ProcessManagerSurface stops polling after explicit hide or focus loss. |
| `show_control_plane` / `hide_control_plane` | frontend -> Rust | none | Explicitly shows/focuses or hides the hidden `control-plane` webview; no event payload fallback is required. |
| `show_settings_panel` / `hide_settings_panel` | `top-bar` or `settings-panel` -> Rust | `{ anchorLeft, anchorWidth }` for show; none for hide | Shows/focuses or hides the hidden `settings-panel` webview. Rust aligns the dropdown below the top-left JasonShell button and clamps it inside the top-bar host width. |
| `parse_automation_cli` / `validate_automation_request` | frontend -> Rust | argv list or `AutomationRequest` | Parses safe first-party automation intents and enforces opt-in/security-boundary rules; it does not execute arbitrary commands. |
| `get_single_instance_forwarding_contract` | frontend -> Rust | none | Returns the plan-only forwarding contract with `executesForwardedPayloads = false`. |
| `resolve_provider_registry` | frontend -> Rust | `ProviderRegistryConfig` | Resolves bounded first-party provider definitions and rejects secret/executable/plugin config. |

IPC wrappers should remain thin and explicit: TypeScript modules in `src/lib/*` call Rust commands through `IPC_COMMANDS` from `src/ipc/commands.ts`; Rust command/event/surface constants live in `src-tauri/src/contracts.rs`. Rust commands return serializable camelCase payloads. Avoid introducing ad-hoc command/event names inside components without updating the frontend IPC modules, backend contracts, tests, and this section.

Capability and CSP contract:

- Tauri capability files are split per current surface under `src-tauri/capabilities/*.json`: `top-bar` remains in `default.json`; `bottom-bar`, `task-preview`, `search-panel`, `stack-popup`, `process-manager`, and `control-plane` each have a single-window capability file.
- The current permission set remains `core:default` plus `core:window:default` because every surface needs event/invoke/window-label primitives, but future tightening should happen per capability file rather than returning to one all-window capability.
- `src-tauri/tauri.conf.json` defines both production `csp` and development `devCsp`. Production allows self, Tauri IPC, data/asset images, and inline styles required by the current Svelte/webview styling model; development additionally allows localhost dev-server connect/image/script evaluation paths.
- Contract tests in `tests/contractsSettings.test.mjs` check capability file shape, CSP non-null/dev split, wrapper use of `IPC_COMMANDS`, diagnostics redaction, settings defaults, and command registration.

### Advanced event/IPC rules

- Command ownership:
  - Window geometry/show/hide commands for auxiliary surfaces are owned by Rust (`show_search_panel`, `hide_search_panel`, `show_stack_popup`, `hide_stack_popup`, `show_task_window_preview`, `hide_task_window_preview`, `show_process_manager`, `hide_process_manager`). Components may request with anchor data but must not position native windows themselves.
  - File, launcher, task-window, process, and shell-path actions are owned by Rust. Frontend should pass selected ids/paths and display errors; backend validates.
  - View-only payload publication can be frontend-driven (`publish_search_panel`) but Rust stores a latest-payload fallback.
- Target windows:
  - Use `emit_to(label, ...)` or `emitTo({ kind: 'WebviewWindow', label }, ...)` whenever the event is meant for one webview. Global `emit()` is acceptable only as a compatibility duplicate or when all surfaces may listen.
  - `stack-pins:updated` must be emitted globally and explicitly to `top-bar` because a previous regression showed top-bar pin state can miss indirect publication.
  - `stack-popup:open` is delivered by both Rust `popup.emit()` and TS `emitTo(STACK_POPUP_LABEL, ...)`, plus stored latest request fallback. This triple path is intentional.
  - `process-manager:open` and `process-manager:closed` are emitted directly to the `process-manager` webview; there is no stored payload because the surface can refresh the authoritative process list after opening.
- Payload shape/staleness:
  - Events carrying selected rows generally carry result id/path strings, not full mutable objects. The owning surface maps ids against its latest state and safely no-ops if absent.
  - Request-style payloads must include a `requestId` when duplicate or stale handling matters. Existing examples: stack popup open payload and task preview request id.
  - Latest-payload commands (`get_search_panel_payload`, `get_stack_popup_request`) should remain idempotent and side-effect free; polling callers depend on that.
- Introducing a new event:
  1. Add/export the event constant in the owning `src/lib/*.ts` wrapper.
  2. Document direction, target, payload, and staleness/idempotency here.
  3. Add a focused pure/source test when event target shape or payload translation matters.
  4. If Rust emits it, define a serializable payload struct with camelCase fields and emit to a specific label where possible.
  5. Decide whether direct emit is enough or whether a stored latest payload/request fallback is required for hidden-window delivery.

## Persistence/data files/configuration

- Stack pins: `stack-folders-v1.json` in Tauri app local data directory, managed by `src-tauri/src/stack_popup.rs`.
- Search index: `search-index-v1.json` in app-managed persistent storage, managed by `src-tauri/src/search_sources/index.rs`.
- Shell settings foundation: `jasonshell-settings-v1.json` in Tauri app local data directory, managed by `src-tauri/src/settings.rs` and wrapped by `src/lib/settings.ts`.
- Task history: `jasonshell-task-history-v1.json` in Tauri app local data directory, managed by `src-tauri/src/dev_tools/task_runner.rs` and wrapped by `src/lib/devTools.ts`.
- Explorer taskbar pins source: `%APPDATA%\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar`.
- Runtime-only state: task preview request state, latest search panel payload, latest stack popup request, stack clipboard, task-window activity snapshots, process-manager CPU snapshots.
- OpenCode orchestrator instructions: `C:\Users\jnev1\.config\opencode\AGENTS.md`. As of the 2026-04-26 spec migration, future agents should use this `master_spec.md` ledger workflow instead of `CONTINUITY.md`.
- Repo-local future agent template: `C:\dev\jasonshell\new_agent.md`. It is a skill-first orchestrator profile that requires `master_spec.md` preflight, mandatory relevant Codex skill loading, subagent-based specialist execution, and QA before completion.
- Do not delete `CONTINUITY.md` unless explicitly requested by the user; current instruction is to forget it going forward, not remove it.

### Persistence details and migration expectations

- `stack-folders-v1.json`:
  - Stored under `app_handle.path().app_local_data_dir()` and owned exclusively by `stack_popup.rs`.
  - Format is pretty JSON array of `PinnedStackFolder { id, name, path }`; `id` is currently the normalized display path.
  - Missing file triggers default pins for Desktop/Downloads when resolvable and writes them. Existing empty file/list should remain respected as user choice.
  - Corrupt JSON is renamed with `.json.corrupt-<millis>` extension and not overwritten in place. Future migrations should preserve corrupt backups and add versioned filenames rather than silently reinterpreting v1.
- `search-index-v1.json`:
  - Stored under app local data dir and owned exclusively by `search_sources/index.rs`.
  - Format is `SearchIndexCache { version: 1, generatedAtEpochSecs, entries }`. `read_cache()` returns `None` on version mismatch or parse failure; there is no corrupt backup for this performance cache.
  - Cache is an acceleration hint, not source of truth. It can be deleted safely and rebuilt.
- `jasonshell-settings-v1.json`:
  - Stored under `app_handle.path().app_local_data_dir()` and owned by `settings.rs`; current commands are `load_shell_settings` and `save_shell_settings`.
  - Format is pretty JSON object `{ schema: "jasonshell.settings", version: 1, ui, workspaces, taskHistory }` with camelCase fields over IPC and Rust `task_history` internally.
  - Version 1 defaults are `ui.activeWorkspaceId = null`, `ui.enableDiagnosticsExport = false`, empty `workspaces`, and empty `taskHistory`.
  - `workspaces` is now a typed `Vec<WorkspaceProfile>` validated through `workspaces.rs`; `taskHistory` remains a reserved compatibility array in settings while executable task history is owned by `jasonshell-task-history-v1.json`.
  - Unversioned/legacy settings migrate to v1 defaults while preserving `ui.activeWorkspaceId` if present. Unsupported future versions return an error rather than silently truncating.
  - Corrupt JSON is renamed to `*.corrupt-<epoch>.bak` and defaults are returned.
  - Secret-like keys containing token, secret, password, credential, api key, authorization, or cookie are rejected recursively by Rust before load/save returns persisted settings; the frontend wrapper has the same guard before invoking save.
- `jasonshell-task-history-v1.json`:
  - Stored under `app_handle.path().app_local_data_dir()` and owned by `dev_tools/task_runner.rs`.
  - Format is `TaskHistoryFile { schema: "jasonshell.taskHistory", version: 1, entries }`.
  - Entries are bounded to the latest 50 task runs and contain task metadata, executable/args resolved from declared workspace tasks, process id, started/finished timestamps, exit code, and cancellation flag. Raw stdout/stderr output is not persisted.
- Durable ownership rule: settings owns typed workspace profiles and active workspace id; task history, stack pins, and search index cache keep their existing files/owners until a later migration explicitly supersedes them.
- `stack-popup-geometry-v1.json`:
  - Stored under `app_handle.path().app_local_data_dir()` and owned by `stack_popup::popup_window`.
  - Format is `{ schema: "jasonshell.stackPopupGeometry", version: 1, size: { width, height } }`, where size is logical CSS-pixel geometry from the Stack Browser resize grip.
  - Corrupt geometry JSON is backed up as `stack-popup-geometry-v1.json.corrupt-<millis>` and ignored; missing/corrupt geometry falls back to the default monitor-clamped Stack Browser size.
- Browser localStorage:
  - `jasonshell.search.usage` is a frontend usage boost map owned by `searchRanking.ts`. It should remain small, result-id keyed, and non-sensitive.
  - `jasonshell.theme` is a frontend cosmetic theme id owned by `src/lib/themes.ts`. It is normalized against the first-party theme registry, applied before Svelte mount, and synchronized across shell webviews by `storage` events plus `BroadcastChannel` `jasonshell.theme.changed` when available. It must remain a non-secret renderer preference and should not be promoted to Rust settings without a versioned migration note.
  - `jasonshell.uiPreferences` is a frontend renderer preference object owned by `src/lib/shellPreferences.ts`. It stores only UI preferences: `fontId`, `dateFormat`, `use24HourTime`, `showSeconds`, `compactDensity`, `strongFocusRing`, `reducedTransparency`, and `showSearchShortcutHint`. It is normalized before use, applied before Svelte mount, synchronized by `storage` events plus `BroadcastChannel` `jasonshell.uiPreferences.changed`, and must remain non-secret.
- Local bundled font assets:
  - `src/assets/fonts/open-sans/` contains Open Sans WOFF2 assets downloaded from Google Fonts plus the source CSS/manifest used during download. `src/app.css` imports only the local font files used by the shell and must not depend on runtime network font fetching.
- External configuration/source data:
  - Explorer taskbar pins are read from `%APPDATA%\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar`; JasonShell does not write Explorer pins.
  - Search roots come from `APPDATA`, `PROGRAMDATA`, `LOCALAPPDATA`, `ProgramFiles`, `ProgramFiles(x86)`, and `USERPROFILE`.
- Volatile runtime state not to persist: HWNDs, active/minimized/busy state, task group preferred order, preview request id/payload, latest search panel payload, latest stack popup request, Stack Browser history/selection/sort, process-manager rows/sort/CPU snapshots/kill actions, drag/drop state, `StackClipboard`, Stack Browser focus-loss guard flags, AppBar registered HWNDs, Explorer taskbar snapshot, and runtime metrics.

## Validation/test commands and coverage map

### Commands

- `npm run check`: Svelte/TypeScript check via `svelte-check --tsconfig ./tsconfig.json`.
- `npm run build`: `tsc --noEmit && vite build`.
- `npm run test:node`: `tsc -p tsconfig.test.json` plus `node --test tests/*.test.mjs`. `tsconfig.test.json` emits `src/lib/*.ts`, `src/features/**/*.ts`, and `src/ipc/**/*.ts` into `dist-tests`; tests should import the emitted `dist-tests/lib/**`, `dist-tests/features/**`, and `dist-tests/ipc/**` paths so stale root helper artifacts cannot mask omitted source files.
- `npm run test:search`: compatibility alias for `npm run test:node`; prefer `test:node` in new docs and CI because the suite covers more than search:
  - `tests/searchPanelState.test.mjs`
  - `tests/stackPopupState.test.mjs`
  - `tests/folderDrag.test.mjs`
  - `tests/taskbarGroups.test.mjs`
  - `tests/taskbarTilePointer.test.mjs`
  - `tests/topBarPins.test.mjs`
  - `tests/stackBrowserTopBarPinFlow.test.mjs`
  - `tests/contextMenuPosition.test.mjs`
  - `tests/stackPopupContextMenu.test.mjs`
  - `tests/stackPopupViewModel.test.mjs`
  - `tests/processManagerState.test.mjs`
  - `tests/processManagerWiring.test.mjs`
  - `tests/processManagerUxState.test.mjs`
  - `tests/searchUxState.test.mjs`
  - `tests/taskbarUxState.test.mjs`
  - `tests/contractsSettings.test.mjs`
  - `tests/systemTray.test.mjs`
  - `tests/workspaces.test.mjs`
  - `tests/devTools.test.mjs`
  - `tests/developerProviders.test.mjs`
  - `tests/stackBrowserContextActions.test.mjs`
  - `tests/themeRegistry.test.mjs`
  - `tests/shellPreferences.test.mjs`
  - `tests/settingsPanelWiring.test.mjs`
- `npm run cargo:test`: Rust tests via `cargo test --manifest-path src-tauri/Cargo.toml`.
- `npm run cargo:check`: Rust compile check via `cargo check --manifest-path src-tauri/Cargo.toml`.
- `npm run validate`: full bundle: check, build, JS tests, Rust tests, Cargo check.
- Windows CI: `.github/workflows/windows-ci.yml` runs checkout, Node 20 setup with npm cache, stable Rust setup, `npm ci`, `npm run check`, `npm run build`, `npm run test:node`, `npm run cargo:test`, and `npm run cargo:check` on `windows-latest` for pushes to `main` and pull requests.
- Live smoke checklist: `docs/smoke-test-windows.md` covers manual post-static-validation checks for shipped surfaces and explicitly notes system tray as parked/experimental; update it when adding or promoting auxiliary surfaces such as `settings-panel`.

Run strategy:

- Documentation-only changes: read back edited headings/sections, search for required headings/terms, and run `git status --short`. Full build is not required unless code snippets/contracts changed in a way that should be cross-checked.
- Pure frontend helper changes: run `npm run test:node` or a focused `npx tsc -p tsconfig.test.json && node --test tests/<file>.mjs`, plus `npm run check` if Svelte/component typing may be affected.
- Svelte component changes: run `npm run check` and the focused Node tests for any helper touched; run `npm run build` when import graphs, payload types, or component syntax changed.
- Rust command/native changes: run focused `cargo test --manifest-path src-tauri/Cargo.toml <module-or-test-filter>` when available, then `npm run cargo:check`; run full `npm run cargo:test` for cross-module command/payload/native helper changes.
- Cross-boundary command/event changes: run at least `npm run check`, `npm run test:node`, `npm run cargo:check`, and focused Rust tests for the command owner. Full `npm run validate` is preferred before declaring a feature slice complete.
- Settings-panel or shell-preference changes: run focused Node coverage for `tests/shellPreferences.test.mjs` and `tests/settingsPanelWiring.test.mjs`, `npm run check`, focused Rust `cargo test --manifest-path src-tauri/Cargo.toml settings_panel` when placement changes, and full `npm run validate` before declaring the slice complete.
- AppBar/window geometry/native shell changes: static validation is insufficient. After check/test/build, run live `npm run tauri dev` and inspect terminal runtime metrics plus actual top/bottom geometry. Use Win32/UIA inspection when debugging zero-height or blank WebView2 regressions.
- Search/index changes: run Rust search tests through `npm run cargo:test` or a focused search filter, and Node tests if top-bar/search-panel state changed. Avoid live broad scans as a substitute for bounded-root tests.
- Stack Browser file-operation or popup-geometry changes: run focused `cargo test --manifest-path src-tauri/Cargo.toml stack_popup`, relevant Node tests (`stackPopupState`, `folderDrag`, `contextMenuPosition`, `stackPopupContextMenu`, `stackBrowserTopBarPinFlow`, `overlayDismissalWiring`), and `npm run check` for Svelte wiring.
- Stack Browser native Open With or drag-out changes: also run `cargo check --manifest-path src-tauri/Cargo.toml` because Windows Shell/OLE bindings depend on Cargo feature gates, and live-smoke real Explorer copy drops plus installed-app launch when possible.
- Process manager changes: run focused process-manager Node tests (`npx tsc -p tsconfig.test.json && node --test tests/processManagerState.test.mjs tests/processManagerUxState.test.mjs tests/processManagerWiring.test.mjs`), focused `cargo test --manifest-path src-tauri/Cargo.toml process_manager`, and `npm run check`; add `npm run cargo:check` when Rust or command registration changed.
- Workspace/tooling changes: run focused Node tests (`tests/workspaces.test.mjs`, `tests/devTools.test.mjs`, `tests/developerProviders.test.mjs`, and `tests/stackBrowserContextActions.test.mjs` as applicable), focused Rust tests (`cargo test --manifest-path src-tauri/Cargo.toml workspaces` and/or `cargo test --manifest-path src-tauri/Cargo.toml dev_tools`), and `npm run cargo:check`. Full `npm run validate` is required before declaring workspace activation or task-runner changes complete.
- Formatting note: as of 2026-04-28, `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passes for the integrated Rust code. Re-check before relying on this; do not mix broad formatting churn into unrelated feature slices unless explicitly in scope.

### Coverage map

- Top bar pins and Stack Browser pin flow: `tests/topBarPins.test.mjs`, `tests/stackBrowserTopBarPinFlow.test.mjs`. `topBarPins` also guards that `applyStackPins` does not recursively reload pins during hydration.
- Folder drag/drop: `tests/folderDrag.test.mjs`.
- Stack Browser reducer and retained-row semantics: `tests/stackPopupState.test.mjs`.
- Context menu positioning: `tests/contextMenuPosition.test.mjs`.
- Stack Browser context menu labels/Open-with wrapper/native drag wiring: `tests/stackPopupContextMenu.test.mjs`.
- Stack Browser delete/overlay and resize command wiring: `tests/overlayDismissalWiring.test.mjs`; Rust `stack_popup` tests cover size clamping and geometry-file roundtrip.
- Search panel state: `tests/searchPanelState.test.mjs`.
- Taskbar grouping/reorder/activity: `tests/taskbarGroups.test.mjs`.
- Taskbar pointer click-vs-drag: `tests/taskbarTilePointer.test.mjs`.
- Process manager sort/format/source wiring: `tests/processManagerState.test.mjs` and `tests/processManagerWiring.test.mjs`.
- Process manager filtering/tree/kill-plan UX: `tests/processManagerUxState.test.mjs`.
- Workspace activation/search/task wrappers: `tests/workspaces.test.mjs`.
- Developer tooling/task wrappers and IPC contracts: `tests/devTools.test.mjs`.
- Developer search providers: `tests/developerProviders.test.mjs`.
- Stack Browser context action planning: `tests/stackBrowserContextActions.test.mjs`.
- Parked system-tray prototype normalization/click request helpers: `tests/systemTray.test.mjs`.
- Shell theme registry, theme count/options, DOM application, pre-mount application order, localStorage key, and cross-webview sync cleanup: `tests/themeRegistry.test.mjs`.
- Shell preferences and Settings panel wiring: `tests/shellPreferences.test.mjs` covers Open Sans default/local asset wiring, preference normalization/application, date/time formatting, storage, BroadcastChannel sync, and listener cleanup. `tests/settingsPanelWiring.test.mjs` covers `settings-panel` surface routing, top-left button show command wiring, settings controls, capability file presence, and Rust placement test presence.
- Melt UI migration wiring: `tests/meltMigrationWiring.test.mjs` covers the `melt` dependency choice for Svelte 5, rejects mixed legacy `@melt-ui/svelte` imports, verifies local Melt-backed select/toggle/radio-group/progress/action-button wrappers, confirms Settings, Control Plane, Process Manager, top-bar action/pin controls, and bottom-bar launcher/task/process command buttons consume those wrappers where direct headless primitives fit, and confirms the Control Plane still avoids privileged IPC while top-bar pin buttons preserve `button[data-path]`, draggable, native menu, and Stack Browser open flows. The same source test also verifies BottomBar preserves task group wrapper attrs/handlers, `.task-button` keyboard focus queries, process-manager anchor rect flow, action buttons avoid unconditional Tooltip trigger/`aria-describedby` wiring when no tooltip content exists, and avoids state-owning Melt builders.
- Rust stack popup file/pin operations: `cargo test --manifest-path src-tauri/Cargo.toml stack_popup`.
- Rust settings-panel placement helpers: `cargo test --manifest-path src-tauri/Cargo.toml settings_panel`.
- Rust process-manager helpers: `cargo test --manifest-path src-tauri/Cargo.toml process_manager`.
- Rust workspace and developer tooling helpers: `cargo test --manifest-path src-tauri/Cargo.toml workspaces` and `cargo test --manifest-path src-tauri/Cargo.toml dev_tools`.
- Rust task-window filtering/activity/close helpers: `src-tauri/src/task_windows/tests.rs` through cargo tests.
- Geometry/runtime metrics and exact settings-panel anchoring/focus delivery require live Tauri/WebView2 inspection; static tests cannot prove actual native rect behavior.

### Defect classes caught by validation

- `npm run check`: Svelte syntax/type errors, missing imports, stale prop/types in components and TS wrappers.
- `npm run build`: TypeScript project build plus Vite production bundling/import graph issues.
- `npm run test:node`: pure TS helper/source regressions for search panel state, stack popup state, drag/drop normalization, taskbar grouping/reorder, pointer click suppression, process-manager sorting/formatting/wiring, system-tray prototype normalization, pin publication/reveal helpers, context-menu placement, and source-level Stack Browser menu contract.
- `npm run cargo:test`: Rust helper/unit regressions for layout, AppBar geometry/rollback/stabilization helpers, launcher utilities, task-window filtering/activity, process-manager guardrails/CPU math, icon/preview helpers, search cache/scoring/provider row mapping, stack popup path/file/pin/clipboard semantics, and shell/native wrappers that have tests.
- `npm run cargo:check`: command registration/type integration, Windows crate API usage, cfg fallback compile paths, and Rust borrow/type errors not exercised by focused tests.
- Live smoke: OS-owned behavior: AppBar reservation and Explorer taskbar restoration, topmost/no-activate behavior, real WebView2 event delivery to hidden windows, native menu placement, ShellExecute/Open With UI, DWM preview capture, native file drops, XButton mouse navigation, scaled-display placement.

## Advanced implementation invariants and modification checklist

Use this checklist before touching major shell surfaces:

### AppBar/native window changes

- Do keep top/bottom bars as the only AppBar-registered windows unless adding a deliberately designed new reserved shell edge.
- Do keep AppBar cleanup idempotent and reachable from close/destroy and process exit.
- Do preserve rollback for partial activation after `ABM_NEW` succeeds but later positioning/work-area mutation fails.
- Do validate native rects and frontend heights when changing startup order, window visibility, `SetWindowPos`, or AppBar calls.
- Do not add extra `ABM_SETPOS`, `SPI_SETWORKAREA`, or `SetWindowPos` calls from unrelated modules; this has caused double-reservation and blank/zero-height startup regressions.
- Do not make top/bottom windows focus-stealing; use no-activate semantics unless a user-visible interaction explicitly requires focus.

### Top-bar pins/search changes

- Do keep pin mutations authoritative from Rust and publish full arrays through `stackPopup.ts`.
- Do keep explicit `{ kind: 'WebviewWindow', label: 'top-bar' }` targeting for `stack-pins:updated`.
- Do separate startup hydration from user-initiated add/reveal; first pin load must not scroll.
- Do route search activation through top bar; search panel should emit intents only.
- Do debounce/stale-gate backend search responses; stale query responses must be dropped.
- Do not scan broad filesystem roots directly on every keypress.

### Bottom-bar/task-window changes

- Do preserve candidate filtering for no-activate, cloaked, owner/tool, shell, DWM, non-primary-monitor, and identity-less windows.
- Do keep HWNDs as strings over IPC and parse in Rust.
- Do keep click-vs-drag suppression tests updated when changing task tile pointer behavior.
- Do keep activity/busy indicators gated to meaningful workloads; generic CPU deltas are too noisy for shell UI.
- Do not persist task group order or HWND state without a product decision; HWNDs are volatile.

### Process manager changes

- Do keep process enumeration and kill authority in Rust. Svelte may sort/display current rows but must not infer killability beyond backend fields.
- Do keep process refresh conservative while the popup is open; avoid broad polling while hidden and avoid concurrent stacked `list_processes` requests.
- Do preserve reader stability for automatic volatile-metric refreshes; use explicit sort changes or explicit refreshes when the user asks to reapply order.
- Do keep taskbar-active process priority display-only. It may promote rows and badges, but killability must still come only from Rust process guardrails.
- Do derive CPU percent from process-time snapshots rather than heavyweight per-row sampling loops.
- Do guard destructive actions: never kill PID 0 or the current JasonShell process, and refresh after every kill attempt.
- Do keep non-Windows fallbacks compiling with empty/unsupported behavior.
- Do not persist process IDs, executable paths, CPU history, or kill actions.

### Control-plane changes

- Do keep `ControlPlaneSurface.svelte` authority-light. It should summarize existing contract data and must not invoke settings saves, process kills, task spawns, or arbitrary automation directly.
- Do pass all control-plane text that can include user/task/provider input through bounded redaction before rendering; include tests for key/value, flag/value, bearer, and token-shaped secret strings.
- Do update `src/ipc/surfaces.ts`, `src/lib/shellSurface.ts`, `src-tauri/src/contracts.rs`, capability files, routing tests, and this spec together when changing the `control-plane` label or window behavior.
- Live smoke remains required for exact show/focus/centering behavior because helper tests do not instantiate WebView2.

### Automation/provider changes

- Do keep automation parsing and validation from becoming an arbitrary command launcher. New actions require explicit enum variants, target validation, security-level classification, and tests.
- Do keep destructive automation authenticated, user-present, and exact-confirmation gated. Mutating actions must retain at least an authenticated or user-present boundary.
- Do not mark single-instance forwarding wired until there is an actual forwarding implementation plus tests proving argv-only parsing, opt-in, and no plugin/payload execution.
- Do keep provider config enum-bounded, budgeted, and secret/executable/plugin rejecting on both frontend and backend wrappers.

### Stack Browser changes

- Do keep latest-request polling fallback and request-key de-duplication.
- Do keep `folderLoadSequence` stale-page rejection for every async folder load path.
- Do keep retained rows non-interactive until the requested folder page arrives.
- Do keep folders-first sorting across all sort columns unless intentionally changing Explorer-like behavior.
- Do validate path/name/file operations in Rust, not only in Svelte.
- Do keep inline editor event isolation so input clicks/keys do not bubble into selection/activation handlers.
- Do not silently follow/copy symlink or reparse-point trees without cycle/reparse policy and tests.

### Search/index changes

- Do keep local index roots bounded by max depth, max entries, and max visited directories.
- Do keep Windows Search provider optional and failure-cached; machines without SystemIndex must remain fast.
- Do update result id conventions and activation code together.
- Do not expose provider SQL/OLE DB failures noisily in normal UX unless diagnostics are explicitly designed.

### Native menus and IPC changes

- Do route native menu events through `taskbar_menu.rs` and emit refresh/action events to specific surface labels.
- Do base64url-encode path-bearing menu IDs and validate paths before native actions.
- Do update wrapper constants, event table, tests, and this spec when adding event names.
- Do decide whether hidden-window delivery needs direct emit plus stored latest payload/request fallback.
- Do not put native action authority in Svelte by passing unchecked command strings, executable paths, or menu IDs.

### Persistence/configuration changes

- Do version new durable formats through filename or explicit version field.
- Do preserve corrupt stack pin backups and cache rebuild behavior.
- Do not persist secrets, machine-specific transient handles, clipboard contents beyond runtime memory, or raw logs.
- Do not delete `CONTINUITY.md` unless the user explicitly requests deletion.

## Current known residual risks

- 2026-04-26 `[ASSUMPTION]` Live Windows/Tauri smoke coverage is still needed for Stack Browser right-click Pin immediate top-bar display, native Explorer drag acceptance cursor, native popup placement on the top bar, and mouse XButton delivery across webview focus states.
- 2026-04-26 `[CODE]` Search index uses SystemIndex OLE DB rows when available and app-managed persistent index fallback; machine-specific Windows Search availability can alter result mix.
- 2026-04-25 `[CODE]` AppBar/work-area behavior is sensitive to Explorer taskbar state and WebView2 startup timing; preserve cleanup and stabilization paths.
- 2026-04-24 `[CODE]` Primary monitor is the active target; multi-monitor parity is not complete.
- 2026-04-27 `[CODE]` Large stack folders progressively load and the frontend virtualizes large row sets, but live WebView2 smoke remains useful for exact row-height alignment, scroll feel, and native input behavior.
- 2026-04-28 `[CODE]` Process manager metrics and developer metadata are best-effort: first CPU snapshot can be unknown, protected/elevated processes may omit path/command-line/ports/memory/start metadata, and live Tauri smoke is still needed for exact popup placement/focus-loss polling stop behavior.
- 2026-04-28 `[CODE]` Workspace activation, developer providers, git status, declared task execution, and Process Manager guardrails have static/unit coverage. Live Windows/Tauri smoke remains needed for VS Code/Windows Terminal handoff UX, real task output event delivery, direct-child cancellation behavior under wrapper processes, dirty/clean git status in large local repos, and Process Manager command-line/port discovery on real developer workloads.
- 2026-04-27 `[CODE]` System tray support is explicitly parked/experimental. The backend prototype is test-only, frontend helpers are covered by Node tests, and no shipped UI or registered Tauri commands expose tray behavior.
- 2026-04-27 `[TOOL]` Static validation passed for the Phase 1 visual/accessibility baseline, but manual keyboard-only, narrow-width, reduced-motion, and screenshot passes still require live Windows/WebView2 smoke using `docs/smoke-test-windows.md`.
- 2026-04-27 `[TOOL]` Static validation and adversarial QA passed for Phase 2-5, but live Windows/Tauri smoke remains required for per-surface capabilities, production/development CSP behavior, Stack Browser virtual scroll feel and row-height alignment, drag/drop/menu behavior in WebView2, and restart persistence of `jasonshell-settings-v1.json`.
- 2026-04-27 `[CODE]` `src/lib/systemTray.ts` still exposes a typed helper for `invoke_system_tray_icon` as parked/test-covered frontend utility code, but `main.rs` intentionally does not register that command until system tray becomes a shipped surface with capabilities and live smoke coverage.
- 2026-04-28 `[TOOL]` Phase 6-8 static validation and adversarial re-review passed after QA fixes. Residual risk is limited to the live Windows/Tauri smoke items above; no remaining static/blocking Phase 6-8 findings are known.
- 2026-04-28 `[CODE]` Phase 9 multi-monitor support is a pure planning/architecture foundation, not live multi-monitor AppBar activation. Mixed-DPI monitor ownership, popup anchoring, and task-strip assignment helpers are covered by Rust tests; live hardware smoke is still needed before claiming runtime multi-monitor parity.
- 2026-04-28 `[CODE]` Phase 10 automation and provider registry contracts are safe-by-default foundations. Local automation requires explicit opt-in and security boundaries, destructive automation requires authenticated/user-present confirmation, single-instance forwarding is `planned-not-wired`, and provider config rejects secrets plus arbitrary executable/plugin loading.
- 2026-04-28 `[CODE]` The `control-plane` surface is routed and command-showable, but it is authority-light and currently renders bounded summaries from provided contract data rather than directly invoking privileged settings/task/process actions. Live Tauri/WebView2 smoke remains needed for exact show/focus/centering behavior.

## Change Ledger

- 2026-04-28T15:18:00-05:00 `[USER]` IN_PROGRESS: Worker A top-bar slice for the exhaustive Melt UI port. Scope: add or reuse shared `src/components/melt/*` action/icon/tooltip button wrapper(s), migrate `src/components/TopBar.svelte` shell-home/settings, pin-rail scroll, and pinned folder buttons to Melt-backed controls where a direct `melt/builders` fit exists, preserve 26px top-bar geometry, `button[data-path]` queryability, drag/drop, native pin context menu, `showStackPopup` flow, search/settings/stack state machines, and strengthen `tests/meltMigrationWiring.test.mjs` source assertions.
- 2026-04-28T15:32:00-05:00 `[CODE]` IMPLEMENTED: Added `src/components/melt/MeltActionButton.svelte`, a local `melt/builders` `Tooltip` wrapper that renders a real DOM `<button>`, forwards click/contextmenu/drag/drop event objects, and supports `data-path`, `draggable`, `title`, `aria-label`, and `aria-haspopup` attributes. Migrated `src/components/TopBar.svelte` shell-home/settings button, rail scroll buttons, and pinned folder buttons to `MeltActionButton` while preserving the existing top-bar CSS selectors, 26px layout, pin rail keyboard/wheel behavior, native top-bar pin context menu, and `showStackPopup` anchoring flow. Strengthened `tests/meltMigrationWiring.test.mjs` and updated `tests/settingsPanelWiring.test.mjs` for the wrapper-based source shape.
- 2026-04-28T15:35:00-05:00 `[TOOL]` VALIDATED: Focused `node --test tests/meltMigrationWiring.test.mjs` passed 4/4; focused `node --test tests/meltMigrationWiring.test.mjs tests/settingsPanelWiring.test.mjs` passed 8/8 after updating the stale settings-button source assertion; `npm run check` passed with Svelte check 0 errors/0 warnings; `npm run test:node` passed 168/168 Node tests. Live WebView2 smoke remains useful for real tooltip popover clipping/placement in the 26px top-bar webview and native drag cursor feel.
- 2026-04-28T00:00Z `[USER]` IN_PROGRESS: Continue the Melt UI port exhaustively across JasonShell renderer surfaces so most suitable controls use Melt-backed primitives, including top-bar folder/action buttons and bottom-bar launcher/task/process controls, while preserving existing UI positioning, bar geometry, IPC/event contracts, native shell behavior, drag/context-menu semantics, and validation coverage. Expected affected surfaces/modules: `src/components/melt/*`, `src/components/TopBar.svelte`, `src/components/TopBar.css`, `src/components/BottomBar.svelte`, `src/components/BottomBar.css`, other renderer surfaces only where direct Melt primitive fits exist, `tests/meltMigrationWiring.test.mjs`, focused source tests, and this spec. Constraints: act as the skill-first AGENTS.md orchestrator, delegate frontend implementation and QA to skilled subagents, preserve unrelated dirty worktree changes, avoid Rust/backend changes unless discovery proves required, and do not return until implementation, validation, and QA are complete.
- 2026-04-28T00:00Z `[USER]` IN_PROGRESS: Continue the in-progress Svelte Mint/Melt UI migration after partial Settings Panel and Control Plane conversion; move more suitable renderer components to local Mint/Melt-backed primitives and modernize display styling while preserving existing shell positioning, geometry, IPC, event contracts, and behavior. Expected affected surfaces/modules: `src/components/melt/*`, renderer Svelte surfaces under `src/components/`, related CSS files, focused wiring/source tests, and this spec. Constraints: act as the skill-first AGENTS.md orchestrator, preserve unrelated dirty worktree changes, use relevant frontend/testing/QA skills and subagents, avoid Rust/backend changes unless discovery proves required, and validate before completion.
- 2026-04-28 `[USER]` IN_PROGRESS: Research and install Melt UI for Svelte and migrate JasonShell's current Svelte surfaces to functional-equivalent Melt-backed UI primitives for a more modern, industry-standard design. Expected affected surfaces/modules: `package.json`, package lockfile, `svelte.config.js` if required, shared Svelte UI primitive helpers/components, `src/components/*.svelte`, component CSS files, related frontend tests, and this spec. Constraints: start from this master spec, inspect the whole codebase, preserve existing shell behavior/geometry/IPC/event contracts, use relevant installed skills and skilled subagents, and validate before completion.
- 2026-04-28 `[CODE]` IMPLEMENTED: Installed current Svelte 5 `melt@0.44.0` and intentionally did not add legacy `@melt-ui/svelte` or a Melt preprocessor. Added local JasonShell wrappers `src/components/melt/MeltSelect.svelte` and `src/components/melt/MeltToggle.svelte` around `melt/builders` `Select` and `Toggle`, styled by existing JasonShell CSS tokens. Migrated `SettingsPanelSurface.svelte` theme/font controls to Melt-backed selects and boolean preferences to Melt-backed toggles. Migrated `ControlPlaneSurface.svelte` section navigation to Melt `Tabs` and its theme picker to `MeltSelect`, while preserving existing Tauri webview boundaries, settings/search/stack/process/taskbar native behavior, custom grids/context menus, and authority-light Control Plane IPC constraints. Added `tests/meltMigrationWiring.test.mjs` and updated this spec.
- 2026-04-28 `[TOOL]` VALIDATED: Relevant skills loaded for this slice were `senior-frontend`, `tdd-guide`, and `adversarial-reviewer`; Rust/backend/prompt/cost/image/plugin skills were not loaded because no Rust, backend API, prompt, cost, image, or plugin work was required. Subagents performed read-only frontend migration mapping, QA gate planning, and final adversarial review dispatch. Official Melt docs and installed package metadata confirmed `melt@0.44.0` as latest with peer dependencies `@floating-ui/dom ^1.6.0` and `svelte ^5.30.1`; `npm ls melt @melt-ui/svelte --depth=0` showed only `melt@0.44.0`; `npm audit --omit=dev` found 0 vulnerabilities. `npm run check`, `npm run build`, `npm run test:node` (167 Node tests), and full `npm run validate` passed, including Svelte check 0 errors/0 warnings, production Vite build, 153 Rust tests with 1 ignored live tray diagnostic, and Cargo check. Bounded live smoke launched `src-tauri\target\debug\jason-shell.exe`, observed repo-local `jason-shell.exe` responding with a JasonShell window, then stopped only the repo-local debug processes.
- 2026-04-28 `[CODE]` QA FOLLOW-UP: Addressed adversarial review warnings for the Melt migration by wiring Control Plane sections through `sectionTabs.getContent(section.id)` so tab `aria-controls` targets exist, adding `select.getOptionId(option.value)` IDs to `MeltSelect` options for active-descendant semantics, making Control Plane child-select styling intentionally global, and strengthening source tests so semantic Melt wiring assertions no longer pass on comments.
- 2026-04-28 `[TOOL]` REVALIDATED: Final adversarial review found no critical blockers but returned CONCERNS for accessibility/test-integrity warnings; the follow-up fixes above resolved those warnings. `git diff --check` now exits 0 with only normal CRLF conversion warnings. Final `npm run validate` passed after the follow-up fixes: Svelte check 0 errors/0 warnings, production build passed, `npm run test:node` passed 167/167 Node tests, `npm run cargo:test` passed 153 Rust tests with 1 ignored live tray diagnostic, and `npm run cargo:check` passed.
- 2026-04-28T13:15:58-05:00 `[USER]` IN_PROGRESS: Add a JasonShell top-left dropdown from the `jasonshell` button with app settings, live theme selection, font selection/default Open Sans, custom date format string support, and additional useful shell settings. Expected affected surfaces/modules: `src/components/TopBar.svelte`, `src/components/TopBar.css`, `src/app.css`, `src/lib/themes.ts`, new settings/font/date-format helpers and tests if needed, downloaded Open Sans font assets, `tests/*`, and this spec. Constraints: preserve existing top-bar icon/tab/component positions except for the requested dropdown anchored to the existing button; theme changes must apply immediately to the UI; use downloaded Open Sans from Google Fonts as the default application font; keep settings renderer-safe and non-secret.
- 2026-04-28T13:30:58-05:00 `[CODE]` IMPLEMENTED: Added a hidden persistent `settings-panel` webview opened by the existing top-left `jasonshell` top-bar button. `TopBar.svelte` now opens settings instead of search from that button while preserving existing icon/tab positions; search remains available from the search input and Ctrl/Cmd+K. `SettingsPanelSurface.svelte` exposes live shell settings for theme, font, custom date format, 24-hour time, seconds, compact density, strong focus ring, reduced transparency, and search shortcut hint, with Done/Close/Escape dismissal. `src/lib/shellPreferences.ts` owns renderer-only preference normalization, localStorage persistence under `jasonshell.uiPreferences`, BroadcastChannel/storage sync, and date/time formatting. Open Sans is now the default font through local WOFF2 assets downloaded from Google Fonts under `src/assets/fonts/open-sans/` and declared in `src/app.css`; runtime UI does not fetch network fonts. Added `settings_panel.rs`, command/capability/surface contracts, settings-panel routing, placement tests, and source-level Node coverage.
- 2026-04-28T13:35:17-05:00 `[TOOL]` VALIDATED: Relevant skills loaded for this slice were `senior-frontend`, `tdd-guide`, and `rust-skills`; no subagent was dispatched because the latest user request did not explicitly ask for subagent delegation in this tool session. Validation passed after updating a stale capability-list assertion for the new `settings-panel`: full `npm run validate` passed with `npm run check` 0 errors/0 warnings, production build passed and emitted local Open Sans font assets, `npm run test:node` passed 164/164 Node tests, `npm run cargo:test` passed 153 Rust tests with 1 ignored live tray diagnostic, and `npm run cargo:check` passed. Live smoke launched `src-tauri\target\debug\jason-shell.exe`, confirmed the repo debug process stayed running after 6 seconds with responding `JasonShell Bottom Bar`, then stopped only the repo-local debug process. `git diff --check` passed with CRLF conversion warnings only.
- 2026-04-28T12:42:39-05:00 `[USER]` IN_PROGRESS: Run a frontend-led visual polish pass across JasonShell without moving existing icon/tab/component positions or changing shell behavior, then add a shell-wide theme system with base dark, base light, and 10-15 popular application/editor-inspired themes such as Monokai, Atom, and Nord. Expected affected surfaces/modules: `src/app.css`, extracted surface CSS files under `src/components/*Surface.css`, top/bottom bar CSS, theme token helpers/tests if added, theme persistence or selection UI only if it preserves existing layout positions, visual-system tests, and this spec. Constraints: preserve shell geometry, positions, IPC, event behavior, keyboard behavior, and existing component ownership; use frontend specialist implementation and QA.
- 2026-04-28T13:07:23-05:00 `[CODE]` IMPLEMENTED: Frontend agent landed the visual polish/theme slice and orchestration follow-up integrated QA fixes. `src/app.css` now defines a richer global visual token system, shared control/form/focus/scrollbar styling, and 16 renderer themes: `base-dark`, `base-light`, `monokai`, `atom-one-dark`, `atom-one-light`, `nord`, `dracula`, `solarized-dark`, `solarized-light`, `github-dark`, `github-light`, `gruvbox-dark`, `gruvbox-light`, `tokyo-night`, `catppuccin-mocha`, and `ayu-dark`. Surface CSS now consumes the tokens for bars, popups, panels, menus, buttons, rows, badges, meters, and selected/hover states without moving existing icons/tabs/components. Added `src/lib/themes.ts` for theme normalization, pre-mount application, `localStorage["jasonshell.theme"]` persistence, and cross-webview `storage`/`BroadcastChannel("jasonshell.theme.changed")` synchronization. `ControlPlaneSurface.svelte` exposes the theme picker in the existing toolbar without privileged settings IPC.
- 2026-04-28T13:07:23-05:00 `[TOOL]` VALIDATED: Relevant skills used were `senior-frontend`, `tdd-guide`, and `adversarial-reviewer`; the implementation ran through a frontend worker and QA subagent. Focused `npx tsc -p tsconfig.test.json; node --test tests/themeRegistry.test.mjs` passed 4/4. Full `npm run validate` passed: Svelte check 0 errors/0 warnings, production build passed, `npm run test:node` passed 156/156 Node tests, `npm run cargo:test` passed 151 Rust tests with one ignored live tray diagnostic, and `npm run cargo:check` passed. Fresh live debug-binary smoke launched `src-tauri\target\debug\jason-shell.exe`, confirmed it stayed running after 6 seconds, exposed `JasonShell Bottom Bar`, and then stopped repo-local debug processes. Adversarial re-review returned CLEAN after theme-sync test and spec follow-ups.
- 2026-04-28T00:00Z [USER] IN_PROGRESS: Keep Stack Browser visible after delete execution and add persistent resizing. Expected affected surfaces/modules: `src/components/StackPopupSurface.svelte`, `src/components/StackPopupSurface.css`, `src/lib/stackPopup.ts`, `src/ipc/commands.ts`, `src-tauri/src/stack_popup.rs` or split modules, `src-tauri/src/main.rs`, `src-tauri/src/contracts.rs`, stack-popup capability files if command scope changes, related tests, and `master_spec.md`. Requirements: deleting a file should refresh the visible folder in real time without closing the Stack Browser; the popup should remain until the user clicks away from it or presses Escape; users should be able to resize the Stack Browser and have that size persist so empty-space paste targets can be made available.
- 2026-04-28T00:00Z [CODE] IMPLEMENTED: Stack Browser confirmed delete now keeps `stack-popup` visible through delete execution by guarding Rust focus-loss hiding inside `delete_stack_item`, refreshing the source folder after the delete loop, and refocusing the popup if an internal delete operation stole focus. Added persisted Stack Browser resizing through a Svelte `.stack-resize-grip`, `resizeStackPopup()` frontend wrapper, `resize_stack_popup` Rust command, monitor-clamped logical sizing in `src-tauri/src/stack_popup/popup_window.rs`, and `stack-popup-geometry-v1.json` under app-local data. Updated Stack Browser command contracts, source-wiring tests, Rust geometry tests, and durable spec sections.
- 2026-04-28T00:00Z [TOOL] VALIDATED: Worker 1 used loaded relevant skills (`senior-frontend`, `rust-skills`, `tdd-guide`). Validation passed: `npm run check` 0 errors/0 warnings, `npm run test:node` passed 142/142 Node tests including Stack Browser delete/resize wiring coverage, `npm run build` passed, focused `cargo test --manifest-path src-tauri/Cargo.toml stack_popup` passed 28/28 Stack Browser Rust tests, `cargo test --manifest-path src-tauri/Cargo.toml core_event_contracts_are_stable` passed 1/1 focused contract test, `cargo check --manifest-path src-tauri/Cargo.toml` passed, `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passed, and in-scope `git diff --check -- ...` exited 0 with CRLF conversion warnings only. Full `git diff --check` is blocked by pre-existing out-of-scope `src/components/TopBar.svelte` trailing whitespace from earlier dirty search-overlay edits. Residual risk: live WebView2 smoke is still useful for exact drag-resize feel and real Recycle Bin focus timing.
- 2026-04-28T00:00Z [CODE] IMPLEMENTED: Superseded the initial Stack Browser delete/resize implementation after adversarial QA. Added explicit `begin_stack_popup_focus_loss_hold` and `end_stack_popup_focus_loss_hold` commands plus `beginStackPopupFocusLossHold()` / `endStackPopupFocusLossHold()` wrappers so confirmed delete holds focus-loss dismissal across the full selected-path loop, folder refresh, and details-grid refocus instead of only during each `delete_stack_item` IPC call. Serialized resize IPC with `resizeRequestChain` so an older non-persist drag-frame resize cannot complete after and visually override the final persisted pointer-up resize. Removed trailing whitespace from the earlier `TopBar.svelte` search overlay diff so full diff hygiene is clean.
- 2026-04-28T00:00Z [TOOL] VALIDATED: Re-ran validation after the QA follow-up fixes. `npm run check` passed with 0 errors/0 warnings, `npm run test:node` passed 142/142 Node tests, `npm run cargo:check` passed, `npm run cargo:test` passed 151 Rust tests with 1 ignored live desktop diagnostic, `npm run build` passed, and full `git diff --check` exited 0 with only normal CRLF conversion warnings. Adversarial QA re-review found no blockers or warnings, verified that the focus-loss hold now spans delete loop plus refresh/refocus, verified serialized resize IPC resolves the stale non-persist resize race, and ran `node --test tests/overlayDismissalWiring.test.mjs` with 3/3 passing. Residual risk remains live WebView2/Recycling Bin focus ordering and drag-resize feel.
- 2026-04-28T00:00Z [USER] IN_PROGRESS: Fix Stack Browser and search overlay dismissal behavior. Expected affected surfaces/modules: `src/components/TopBar.svelte`, `src/components/SearchPanelSurface.svelte`, `src/components/StackPopupSurface.svelte`, `src-tauri/src/main.rs`, `src-tauri/src/search_panel.rs`, `src-tauri/src/stack_popup.rs`, related frontend/Rust tests, and `master_spec.md`. Requirements: opening a Stack Browser delete confirmation must not close the Stack Browser before the user can choose yes/no; clicking away from the top-bar search input or search results pane should close the results pane; preserve existing file-operation, pin, and search result activation behavior; run QA/validation before completion.
- 2026-04-28T00:00Z [CODE] IMPLEMENTED: Replaced Stack Browser native `window.confirm` deletion with an in-webview `deleteConfirmation` dialog in `src/components/StackPopupSurface.svelte` styled by `StackPopupSurface.css`, preserving selected paths and the source folder until the user chooses Cancel or Delete. Added search dismissal coordination across `TopBar.svelte`, `SearchPanelSurface.svelte`, `src/lib/searchPanel.ts`, `src/ipc/events.ts`, `src-tauri/src/search_panel.rs`, `src-tauri/src/main.rs`, and `src-tauri/src/contracts.rs`: top-bar outside pointerdown closes the panel, search-input blur uses a short delayed close, search-panel interactions emit `search-panel:interaction`, and native search-panel focus loss emits `search-panel:closed` before hiding. Added `tests/overlayDismissalWiring.test.mjs` and updated durable search/Stack Browser lifecycle, event, accessibility, validation, and ledger sections.
- 2026-04-28T00:00Z [TOOL] VALIDATED: Worker 1 used loaded relevant skills (`senior-frontend`, `rust-skills`, `tdd-guide`) and ran focused validation: `npm run check` passed with 0 errors/0 warnings, `npm run test:node` passed 141/141 Node tests including `tests/overlayDismissalWiring.test.mjs`, `cargo test --manifest-path src-tauri/Cargo.toml stack_popup` passed 26/26 focused Stack Browser Rust tests, `cargo test --manifest-path src-tauri/Cargo.toml core_event_contracts_are_stable` passed 1/1 focused Rust event-contract test, and `cargo check --manifest-path src-tauri/Cargo.toml` passed. Residual live-smoke risk remains for exact WebView2 focus delivery and real desktop click-away behavior.
- 2026-04-28T00:00Z [USER] IN_PROGRESS: Implement `action_plan.md` Phase 9 through Phase 10 without returning before integration and testing. Expected affected surfaces/modules: multi-monitor AppBar/window placement architecture, monitor ownership and popup anchoring tests, automation/CLI command parsing and safe forwarding contracts, provider contracts and performance budgets, settings/dashboard control-plane helpers or surfaces, Tauri command/event/capability contracts, tests, documentation, and `master_spec.md`. Constraints: act as the skill-first orchestrator from `AGENTS.md`, use relevant Codex skills and subagents, preserve unrelated dirty worktree changes from earlier phases, enforce explicit security boundaries for automation/destructive actions, and run QA/validation before completion.
- 2026-04-28T00:00Z [CODE] IMPLEMENTED: Completed `action_plan.md` Phase 9 through Phase 10 integration. Phase 9 added pure Rust multi-monitor planning helpers in `layout.rs` plus `docs/architecture/phase-9-multi-monitor-architecture.md` for mixed-DPI monitor ownership, source-monitor popup anchoring, and stable task-strip monitor assignment while leaving live AppBar activation single-monitor. Phase 10 added safe local automation parsing/validation, plan-only single-instance forwarding contracts, config-driven provider registry contracts with bounded budgets and secret/executable/plugin rejection, a hidden persistent `control-plane` webview routed through `App.svelte`/surface contracts/capability files, frontend automation/provider wrappers, and bounded secret-redacted control-plane dashboard view-model/component code. QA follow-up hardened control-plane redaction for raw bearer/API-token shaped values and updated durable spec sections for the new surface, commands, modules, validation, and residual risks.
- 2026-04-28T00:00Z [TOOL] VALIDATED: Phase 9-10 specialist subagents used loaded relevant skills (`senior-frontend`, `senior-backend`, `rust-skills`, `tdd-guide`, `spec-driven-workflow`, `adversarial-reviewer`) for implementation and QA. Integrated validation passed before QA follow-up, then final `npm run validate` passed again after the redaction/spec fixes with `npm run check` 0 errors/0 warnings, production `npm run build` passed, `npm run test:node` passed 139/139 Node tests, `npm run cargo:test` passed 149 Rust tests with 1 ignored live system-tray diagnostic, and `npm run cargo:check` passed. Hygiene passed: `cargo fmt --manifest-path src-tauri/Cargo.toml --check` and `git diff --check` reported only normal CRLF conversion warnings.
- 2026-04-28T00:00Z [USER] IN_PROGRESS: Implement `action_plan.md` Phase 6 through Phase 8 without returning before integration and testing. Expected affected surfaces/modules: workspace profile schema and activation APIs, settings/workspace/task-history persistence, TopBar workspace pill and search biasing, workspace pins and task exposure, terminal/editor/git/task-runner Rust modules and TypeScript wrappers, task output streaming/cancellation/history, developer Stack Browser context actions, bounded workspace/search providers, saved searches, Process Manager developer process details/ownership/kill-tree guardrails, tests, documentation, and `master_spec.md`. Constraints: act as the skill-first orchestrator from `AGENTS.md`, use relevant Codex skills and subagents, preserve unrelated dirty worktree changes, avoid storing secrets in workspace/task data, and run QA/validation before completion.
- 2026-04-28T00:00Z [CODE] IMPLEMENTED: Completed `action_plan.md` Phase 6 through Phase 8 integration. Phase 6 added typed workspace profiles, CRUD/list/activate commands, activation plans for layout/search/top-bar pins/tasks/startup/restoration, safe non-executing startup semantics, reserved restoration status, settings-backed workspace persistence, and TS workspace helpers. Phase 7 added `dev_tools` Rust modules and TS wrappers for safe terminal/editor launch plans, git status parsing, identity-only declared workspace task spawning, task output events, cancellation, bounded task-history persistence, and JasonShell task process metadata. Phase 8 added bounded developer search providers, saved-search scope contracts, Stack Browser context-action planning, Process Manager command-line/port/parent/descendant/workspace metadata, and backend-enforced process kill confirmation guardrails. QA follow-up hardened task execution against arbitrary renderer-supplied commands, rejected secret-like workspace task/startup args, bounded task output chunks with monotonic sequencing, and enforced Process Manager kill guardrails for direct IPC.
- 2026-04-28T00:00Z [TOOL] VALIDATED: Phase 6-8 specialist subagents used loaded relevant skills (`senior-frontend`, `senior-backend`, `rust-skills`, `tdd-guide`, `spec-driven-workflow`, `adversarial-reviewer`) for implementation and QA. Final local validation passed: `npm run validate` completed with `npm run check` 0 errors/0 warnings, production `npm run build` passed, `npm run test:node` passed 130/130 Node tests, `npm run cargo:test` passed 132 Rust tests with 1 ignored live system-tray diagnostic, and `npm run cargo:check` passed. `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passed. `git diff --check` reported only normal CRLF conversion warnings. Adversarial re-review returned CLEAN after verifying declared-task-only spawning, secret-like workspace arg rejection, and backend-enforced Process Manager kill confirmations.
- 2026-04-27T00:00Z [CODE] IMPLEMENTED: Completed `action_plan.md` Phase 2 through Phase 5 integration. Phase 2 added frontend feature seams under `src/features/*`, Stack Browser virtual row/window helpers, breadcrumb overflow, explicit delete prompt state, keyboard scroll-into-view for virtualized rows, and split Rust Stack Browser implementation under `src-tauri/src/stack_popup/` while preserving public command names/payloads. Phase 3 added frontend/backend IPC contract modules, per-surface Tauri capability files, production/development CSP split, and frontend/backend diagnostics ring buffers with redaction/export. Phase 4 upgraded bottom-bar overflow/focus state, Process Manager filtering/tree rows/metric bars/two-step kill UX, grouped keyboard-first search helpers, and top-bar shell identity/status affordances without workspace coupling. Phase 5 added versioned settings persistence through `src-tauri/src/settings.rs` and `src/lib/settings.ts` with defaults, migration, corrupt backup, atomic save, and secret-key rejection. Follow-up QA fixes enforced emitted test imports, `src/ipc/**/*.ts` compilation, wrapper use of `IPC_COMMANDS`, recursive frontend diagnostics redaction, non-empty workspace/task-history settings types, and the Stack Browser virtual keyboard mount guarantee.
- 2026-04-27T00:00Z [TOOL] VALIDATED: Phase 2-5 specialist subagents used relevant loaded skills (`senior-frontend`, `senior-backend`, `rust-skills`, `tdd-guide`, `spec-driven-workflow`, `adversarial-reviewer`) for implementation and QA. Final local validation after QA fixes passed: `npm run validate` completed with `npm run check` 0 errors/0 warnings, production `npm run build` passed, `npm run test:node` passed 111/111 Node tests, `npm run cargo:test` passed 103 Rust tests with 1 ignored live system-tray diagnostic, and `npm run cargo:check` passed. `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passed for the integrated Rust code. Adversarial re-review found no remaining Phase 2-5 blockers; residual concerns are live Windows/Tauri smoke for CSP/capabilities, Stack Browser virtualization feel, restart persistence, and the explicit parked system-tray frontend helper whose Rust commands remain intentionally unregistered.
- 2026-04-27T00:00Z [USER] IN_PROGRESS: Implement `action_plan.md` Phase 2 through Phase 5 without returning before integration and testing. Expected affected surfaces/modules: extracted `src/features/*` frontend feature modules, Stack Browser Rust module split/scalability polish, centralized IPC/event/surface contracts, Tauri capabilities/CSP/diagnostics, bottom-bar/process-manager/search/top-bar UX upgrades, durable versioned settings/persistence foundation, tests, documentation, and `master_spec.md`. Constraints: act as the skill-first orchestrator from `AGENTS.md`, use relevant Codex skills and subagents, preserve unrelated dirty worktree changes, and run QA/validation before completion.
- 2026-04-27T00:00Z [USER] IN_PROGRESS: Implement `action_plan.md` Phase 0 and Phase 1 without returning before integration and testing. Expected affected surfaces/modules: validation scripts and Node test coverage, TopBar pin hydration state, taskbar group tests, system tray status, Windows CI workflow, README/docs/action-plan-aligned documentation, `src/app.css` design tokens, surface CSS/Svelte accessibility/responsive/reduced-motion updates, and durable `master_spec.md` sections. Constraints: act as skill-first orchestrator, use relevant Codex skills, delegate specialist work to subagents, preserve unrelated changes, and run validation before completion.
- 2026-04-27T00:00Z [CODE] IMPLEMENTED: Completed `action_plan.md` Phase 0 and Phase 1 integration. Phase 0 repaired the Node test compilation model by compiling all `src/lib/*.ts`, added canonical `npm run test:node` with `test:search` as a compatibility alias, resolved taskbar group test drift, fixed TopBar pin hydration recursion with source-level regression coverage, parked the system-tray prototype as Windows-test-only with no registered shipped commands, added Windows CI, refreshed current-vs-historical product docs, and added `docs/smoke-test-windows.md`. Phase 1 added global `src/app.css` design tokens/status/focus/reduced-motion/high-contrast support, extracted Search Panel/Stack Browser/Task Preview CSS files, improved ARIA/keyboard semantics across current surfaces, and fixed QA-found CSS/ARIA regressions before closure.
- 2026-04-27T00:00Z [TOOL] VALIDATED: Specialist subagents implemented frontend, validation/docs/CI, Rust system-tray, adversarial QA, and QA follow-up slices using relevant loaded skills (`senior-frontend`, `rust-skills`, `tdd-guide`, `spec-driven-workflow`, `adversarial-reviewer`). Final local `npm run validate` passed: `npm run check` 0 errors/0 warnings, production `npm run build` passed, `npm run test:node` passed 91/91 Node tests, `npm run cargo:test` passed 93 Rust tests with 1 ignored live system-tray diagnostic, and `npm run cargo:check` passed. `npm run test:search` passed as alias. `cargo fmt --manifest-path src-tauri/Cargo.toml --check` still reports pre-existing formatting diffs in `src-tauri/src/stack_popup.rs`; `git diff --check` still reports pre-existing trailing-whitespace/CRLF issues in dirty out-of-scope files, while the in-scope StackPopupSurface trailing-whitespace issue was fixed.
- 2026-04-27 [USER] IN_PROGRESS: Create `new_agent.md` as a future-use agent file. Scope: repo-root `new_agent.md`; adapt the master-spec-ledger workflow from `C:\Users\jnev1\.config\opencode\AGENTS.md`; require `master_spec.md` as first-read context; require relevant Codex skill loading and subagent-based specialist execution; do not force irrelevant skills such as frontend for backend-only fixes.
- 2026-04-27 [CODE] IMPLEMENTED: Added `new_agent.md` as a skill-first orchestration profile that requires session skill inspection on every request, mandatory loading of every relevant installed Codex skill, strict no-direct-implementation delegation rules, QA or verification before completion, and an adapted `Master Spec Ledger (compaction-safe)` section rooted in `master_spec.md`.
- 2026-04-27 [TOOL] VALIDATED: Read back `new_agent.md`, confirmed the required `master_spec.md` preflight, relevant-vs-irrelevant skill rules, mandatory use of installed `C:\Users\jnev1\.codex\skills` plus bundled or plugin session skills, and strict subagent-only specialist execution model. QA document review found no content defects; residual risk is limited to unverified live Codex discovery and runtime loading behavior for the new agent file.
- 2026-04-26T00:00Z [USER] IN_PROGRESS: Verify process-manager QA follow-ups are fully resolved before completion. Scope: inspect Start Time UI/sort/test/source wiring in `src/components/ProcessManagerSurface.svelte`, `src/components/ProcessManagerSurface.css`, `src/lib/processManagerState.ts`, `tests/processManagerState.test.mjs`, `tests/processManagerWiring.test.mjs`; verify `master_spec.md` includes process-manager as sixth surface/window in durable functional sections with no stale five-window wording; run focused Node/Rust validation where feasible. Constraints: do not commit and preserve unrelated changes.
- 2026-04-26T00:00Z [TOOL] VALIDATED: Follow-up QA inspected process-manager Start Time UI/sort/format/test wiring and durable `master_spec.md` process-manager surface/window/command/lifecycle/test/risk sections. Searched `master_spec.md` for stale five-window wording and confirmed six-window/process-manager functional coverage. Ran focused `npx tsc -p tsconfig.test.json && node --test tests/processManagerState.test.mjs tests/processManagerWiring.test.mjs`, focused `cargo test --manifest-path src-tauri/Cargo.toml process_manager`, and `npm run check`; all passed. No in-scope follow-up defects found; live Tauri smoke remains the only residual risk for actual popup placement/focus-loss behavior.
- 2026-04-26T00:00Z [USER] Objective: address in-scope QA follow-ups for the newly implemented process-manager popup. Expected affected surfaces/modules: `src/components/ProcessManagerSurface.svelte`, `src/lib/processManagerState.ts`, `src-tauri/src/process_manager.rs`, process-manager tests, and durable process-manager architecture/spec sections. Status: IN_PROGRESS.
- 2026-04-26T00:00Z [CODE] IMPLEMENTED: Exposed the collected `startTimeMs` process metadata as a visible sortable `Start Time` table column in `src/components/ProcessManagerSurface.svelte`, added `formatProcessStartTime()` in `src/lib/processManagerState.ts`, compacted the process table grid in `ProcessManagerSurface.css`, and extended process-manager Node tests for start-time sort/formatting plus source wiring from Rust `start_time_ms` to the visible Svelte column. Expanded `master_spec.md` functional architecture sections so `process-manager` is documented as the sixth webview/surface, with bottom-bar behavior, frontend/Rust ownership, command/event map entries, lifecycle/focus-loss behavior, validation coverage, persistence exclusions, invariants, and residual risks.
- 2026-04-26T00:00Z [TOOL] VALIDATED: Focused `npx tsc -p tsconfig.test.json && node --test tests/processManagerState.test.mjs tests/processManagerWiring.test.mjs` passed 6/6 tests before and after the finite-start-time formatter hardening; focused `cargo test --manifest-path src-tauri/Cargo.toml process_manager` passed 2/2 process-manager Rust tests; `npm run check` passed with 0 errors/0 warnings; `cargo check --manifest-path src-tauri/Cargo.toml` passed; full `npm run test:search` passed 86/86 Node tests. Status: VALIDATED.

- 2026-04-26 `[USER]` REQUESTED: Create `master_spec.md` as the durable, granular master specification for future AI sessions; include top bar, bottom bar/taskbar, Stack Browser, search, Rust commands/events/tests/config; update `C:\Users\jnev1\.config\opencode\AGENTS.md` to replace the old `CONTINUITY.md` workflow with this master spec ledger workflow. Constraints: do not delete `CONTINUITY.md`, preserve unrelated worktree changes, use `apply_patch`, do not commit.
- 2026-04-26 `[CODE]` IMPLEMENTED: Added initial repository-specific `master_spec.md` covering architecture, surfaces, top/bottom bars, Stack Browser, search, backend/frontend maps, IPC, persistence, validation coverage, residual risks, and maintenance rules. Updated OpenCode orchestrator instructions to use `master_spec.md` as the compaction-safe ledger going forward.
- 2026-04-26 `[TOOL]` VALIDATED: Read back `master_spec.md` section headings and required-section matches; read back the updated `AGENTS.md` Master Spec Ledger section; searched `AGENTS.md` for `Continuity Ledger`, `CONTINUITY.md`, `master_spec.md`, and `Master Spec Ledger` to confirm the old section heading was removed and remaining continuity mentions only explain non-deletion/non-reliance after migration.
- 2026-04-26 `[USER]` IN_PROGRESS: Perform mandatory QA on the documentation/workflow migration. Scope: inspect `master_spec.md` and `C:\Users\jnev1\.config\opencode\AGENTS.md`; verify master-spec coverage, ledger protocol, AGENTS migration away from canonical `CONTINUITY.md`, and preservation of `CONTINUITY.md`.
- 2026-04-26 `[TOOL]` VALIDATED: QA read `master_spec.md` and `AGENTS.md`; searched required master-spec sections, top/bottom bar Rust and Svelte/TypeScript details, ledger protocol, AGENTS master-spec migration instructions, and remaining `CONTINUITY.md` references. Confirmed `CONTINUITY.md` remains present and no in-scope defects were found.
- 2026-04-26 `[USER]` IN_PROGRESS: Fix Stack Browser right-click row/background menus so they fit within visible screen space, move Open With options behind a hover submenu, preserve all menu button actions, and prevent rename/new-folder text input clicks from collapsing or activating the Stack Browser. Expected surfaces: `src/components/StackPopupSurface.svelte`, `src/lib/stackPopup.ts`, Stack Browser menu positioning helpers/tests, and Rust/Tauri commands if Open With requires native support.
- 2026-04-26 `[CODE]` IMPLEMENTED: Stack Browser row/background context menus now measure after render and flip/clamp into viewport space via `src/lib/contextMenuPosition.ts`; rename and new-folder now use an inline editor with guarded pointer/key propagation; row context menus preserve root `Open` and expose file-only `Open with ▸` -> `Choose app...` backed by `open_stack_item_with_picker`.
- 2026-04-26 `[TOOL]` VALIDATED: Mandatory implementation and QA subagents verified the Stack Browser menu/input changes. Focused placement and context-menu source regressions passed, `npm run check` passed, `npm run test:search` passed with 80 Node tests, `npm run build` passed, focused `cargo test --manifest-path src-tauri/Cargo.toml stack_popup` passed with 26 Rust tests, and `cargo check --manifest-path src-tauri/Cargo.toml` passed. Live Tauri smoke remains useful for exact WebView geometry and Windows Open With picker behavior.
- 2026-04-26 `[USER]` IN_PROGRESS: Correct Stack Browser row context menu after a misleading `Open width ▸` / `Default width` submenu was introduced. Scope: `src/components/StackPopupSurface.svelte`, `src/lib/stackPopup.ts`, Rust/Tauri command registration if feasible, tests, and durable ledger/spec updates. Constraints: preserve context-menu positioning and inline editor fixes; do not revert unrelated worktree changes.
- 2026-04-26 `[CODE]` IMPLEMENTED: Replaced misleading width submenu with file-only `Open with ▸` -> `Choose app...`; added `openStackItemWithPicker` TS wrapper, `open_stack_item_with_picker` Tauri command, and `shell_paths::open_shell_path_with_picker` backed by Windows ShellExecuteW `openas` with safety comments; added `tests/stackPopupContextMenu.test.mjs` and included it in `npm run test:search`.
- 2026-04-26 `[TOOL]` VALIDATED: `npm run check`, focused `npx tsc -p tsconfig.test.json && node --test tests/stackPopupContextMenu.test.mjs`, `npm run test:search` (80 Node tests), focused `cargo test --manifest-path src-tauri/Cargo.toml stack_popup` (26 Rust tests), `cargo check --manifest-path src-tauri/Cargo.toml`, and `npm run build` passed. `cargo fmt --manifest-path src-tauri/Cargo.toml --check` still reports pre-existing formatting diffs in `src-tauri/src/stack_popup.rs`.
- 2026-04-26 `[USER]` IN_PROGRESS: Follow-up QA validation for the Stack Browser `Open with` correction. Scope: inspect changed Svelte/TS/Rust/test files, verify no `Open width`/`Default width` UI remains, verify handler/command path, review ShellExecuteW `openas` safety/lifetimes, confirm directory rejection and existing actions remain intact, and run focused checks if feasible.
- 2026-04-26 `[TOOL]` VALIDATED: Follow-up QA inspected `src/components/StackPopupSurface.svelte`, `src/lib/stackPopup.ts`, `src-tauri/src/stack_popup.rs`, `src-tauri/src/shell_paths.rs`, `src-tauri/src/main.rs`, `tests/stackPopupContextMenu.test.mjs`, and `package.json`; searched for stale `Open width`/`Default width` labels and Open With command wiring; ran focused `npx tsc -p tsconfig.test.json && node --test tests/stackPopupContextMenu.test.mjs`, `cargo test --manifest-path src-tauri/Cargo.toml stack_popup`, and `cargo check --manifest-path src-tauri/Cargo.toml`, all passing. No in-scope defects found; live Windows Open With picker smoke remains residual runtime risk.
- 2026-04-26 `[TOOL]` VALIDATED: Final orchestration verification ran `npm run validate`; Svelte check passed with 0 errors/0 warnings, production TypeScript/Vite build passed, `npm run test:search` passed 80 Node tests, full `cargo test --manifest-path src-tauri/Cargo.toml` passed 78 Rust tests, and `cargo check --manifest-path src-tauri/Cargo.toml` passed.
- 2026-04-26 `[USER]` IN_PROGRESS: Expand `master_spec.md` with substantially more advanced technical detail for senior engineers and future AI agents. Scope: repository architecture, implementation contracts, Rust/Tauri/Win32 internals, Svelte/TypeScript state/data flows, IPC/event semantics, invariants, edge cases, validation expectations, and guidance needed to modify the system without rediscovering concepts.
- 2026-04-26 `[CODE]` IMPLEMENTED: Expanded `master_spec.md` with deeper implementation-oriented details covering system reasoning boundaries, Tauri startup/lifecycle, AppBar activation/cleanup, top-bar search and pin flows, bottom-bar launcher/window/preview state machines, Stack Browser reducer/paging/file-operation semantics, search indexing/provider behavior, backend/frontend ownership maps, advanced IPC/event rules, persistence ownership, validation strategy, and concrete modification invariants for future agents.
- 2026-04-26 `[TOOL]` VALIDATED: Documentation expansion was source-backed by inspecting the relevant Rust, Svelte, TypeScript, test, and package script files listed in the request. Lightweight validation read back edited spec regions, searched for required new headings/contract terms, and checked `git status --short`; no code/build validation was required because this pass edited only `master_spec.md`. `git status --short` also showed numerous pre-existing modified/untracked files from earlier work, which were preserved.
- 2026-04-26 `[USER]` IN_PROGRESS: Add a small button at the rightmost edge of the bottom bar that opens a Task Manager-like process popup. Expected surfaces/modules: `src/components/BottomBar.svelte`, `src/components/BottomBar.css`, new or existing frontend IPC wrappers, Rust process enumeration/termination commands registered in `src-tauri/src/main.rs`, tests, and `master_spec.md`. Requirements: list running processes with CPU usage and useful metrics, support sortable columns, allow killing processes, remain highly performant under machine slowdown, and match the existing JasonShell theme.
- 2026-04-26 `[CODE]` IMPLEMENTED: Added dedicated hidden persistent `process-manager` webview routed by `src/App.svelte`/`src/lib/shellSurface.ts`, created from `src-tauri/src/shell_windows.rs`, opened by the rightmost bottom-bar `.process-manager-button`, and rendered by `src/components/ProcessManagerSurface.svelte` / `.css`. Added `src/lib/processManager.ts` IPC wrapper and `src/lib/processManagerState.ts` sort/format helpers. Added `src-tauri/src/process_manager.rs` with `show_process_manager`, `hide_process_manager`, `list_processes`, and guarded `kill_process`; Windows enumeration uses Toolhelp snapshots, limited process queries, cached CPU-time deltas, working-set memory, parent PID, thread count, start time, executable path, and PID kill guardrails with non-Windows fallbacks.
- 2026-04-26 `[TOOL]` VALIDATED: Ran `cargo fmt --manifest-path src-tauri/Cargo.toml`; `npm run test:search` passed 85 Node tests including new process-manager sort/wiring coverage; `npm run check` passed with 0 errors/0 warnings; `npm run build` passed; `npm run cargo:test` passed 80 Rust tests including process-manager CPU/kill-guard tests; `npm run cargo:check` passed; final `npm run validate` passed the full bundle.
- 2026-04-26 `[USER]` IN_PROGRESS: Perform independent QA review of the newly implemented bottom-bar Task Manager-like `process-manager` popup. Scope: inspect changed Rust/Svelte/TypeScript/test/spec files, verify window routing/IPC registration, process enumeration performance/concurrency behavior, kill guardrails, sortable helper coverage, non-Windows fallback, durable `master_spec.md` updates, and run focused validation where feasible. Constraints: preserve unrelated worktree changes, do not commit, only fix clear safe in-scope defects.
- 2026-04-26 `[TOOL]` VALIDATED: Independent QA inspected process-manager Rust/Svelte/TypeScript/test/spec wiring and ran `npm run check`, focused process-manager Node tests, focused `cargo test --manifest-path src-tauri/Cargo.toml process_manager`, and `cargo check --manifest-path src-tauri/Cargo.toml`; all commands passed. QA found in-scope follow-up needs: expose/test the claimed Start Time sortable column in the UI, and expand `master_spec.md` functional sections/maps from five surfaces/windows to include `process-manager` durably rather than only ledger/invariant/risk notes.
- 2026-04-28T11:45:32-05:00 `[USER]` IN_PROGRESS: Add Stack Browser background/margin right-click directory actions, change bottom-bar active task tiles from uniform width to content-sized tiles with a minimum width, and make Process Manager more Task Manager-like by prioritizing taskbar-active processes, preserving guarded kill actions, and preventing automatic refresh from constantly reordering volatile sorts such as CPU. Expected affected surfaces/modules: `src/components/StackPopupSurface.svelte`, `src/components/StackPopupSurface.css`, Stack Browser context-menu helpers/tests, `src/components/BottomBar.svelte`, `src/components/BottomBar.css`, taskbar window grouping/state helpers/tests, `src/components/ProcessManagerSurface.svelte`, `src/components/ProcessManagerSurface.css`, `src/lib/processManagerState.ts`, `src/features/process-manager/processManagerUxState.ts`, `src/lib/processManager.ts`, Rust process/task-window command helpers if active-task metadata is needed, related tests, live smoke notes, and this spec.
- 2026-04-28T12:05:11-05:00 `[CODE]` IMPLEMENTED: Worker 1 added Stack Browser background/margin context-menu handling in `src/components/StackPopupSurface.svelte`; row menus still stop propagation, and background menus now expose Paste/New Folder plus selected-item Copy/Cut/Rename/Delete/Reveal actions while ignoring toolbar/editor/dialog/resize controls. Bottom-bar task buttons in `src/components/BottomBar.css` now use content-sized flex sizing with a 6.2rem minimum, 14rem maximum, and ellipsis truncation. Added focused regressions in `tests/stackPopupContextMenu.test.mjs` and `tests/taskbarUxState.test.mjs`.
- 2026-04-28T12:05:11-05:00 `[TOOL]` VALIDATED: Worker 1 ran `npm run test:node` and it passed 150 Node tests after concurrent Process Manager changes, then ran `npm run check` and Svelte check passed with 0 errors and 0 warnings. Live Tauri smoke remains useful for exact WebView right-click/focus behavior and visual task-tile sizing.
- 2026-04-28T12:22:00-05:00 `[CODE]` IMPLEMENTED: Worker 2 made Process Manager prioritize taskbar-active processes by adding `process_id`/`processId` to task-window metadata, enriching process rows with taskbar badges/counts/titles in `src/features/process-manager/processManagerUxState.ts`, and ordering rows through `src/lib/processManagerState.ts`. Automatic refreshes for volatile metric sorts now preserve existing relative row order while updating metric values; explicit sort, manual refresh, popup open, and post-kill refresh reapply ordering. Kill actions still use existing two-step confirmation and Rust guardrails.
- 2026-04-28T12:22:00-05:00 `[TOOL]` VALIDATED: Worker 2 ran focused `npx tsc -p tsconfig.test.json; node --test tests/processManagerState.test.mjs tests/processManagerUxState.test.mjs tests/processManagerWiring.test.mjs` with 17/17 tests passing; `npm run check` passed with 0 errors/0 warnings; `cargo test --manifest-path src-tauri/Cargo.toml process_manager` passed 12/12; `cargo test --manifest-path src-tauri/Cargo.toml task_windows` passed 18/18; `npm run test:node` passed 150/150; and `cargo check --manifest-path src-tauri/Cargo.toml` passed. Live Tauri smoke remains useful to visually confirm badge placement and active-window prioritization with real Windows taskbar data.
- 2026-04-28T12:35:00-05:00 `[TOOL]` VALIDATED: Orchestration integration added top-level promoted process-tree rows for taskbar-active processes, ran `npm run validate`, and passed Svelte check, production build, 151 Node tests, 151 Rust tests with one ignored live tray diagnostic, and Cargo check. A bounded live smoke first confirmed `npm run tauri dev` could compile/launch `jason-shell.exe` against the already-running Vite server while a second dev server could not bind ports 1420/1421; after cleanup, a direct debug-binary smoke confirmed `target/debug/jason-shell.exe` stayed running, responded to Windows, and exposed the `JasonShell Bottom Bar` window title before it was stopped.
- 2026-04-28T12:45:00-05:00 `[TOOL]` VALIDATED: Adversarial QA reviewed the integrated diff and returned CONCERNS rather than BLOCK. Follow-up fixes added lightweight `list_taskbar_process_windows` so Process Manager no longer polls full icon/activity taskbar window data every second, tightened Process Manager name/badge flex shrinking, exported and tested Stack Browser background-context ignore selectors, and reran final `npm run validate`; Svelte check, production build, 152 Node tests, 151 Rust tests with one ignored live tray diagnostic, and Cargo check all passed.
- 2026-04-28T14:18:00-05:00 `[CODE]` IMPLEMENTED: Worker 1 continued the Mint/Melt migration with direct-fit renderer primitives only. Added `src/components/melt/MeltRadioGroup.svelte` around `melt/builders` `RadioGroup` and migrated Settings Panel date-format preset chips to it while preserving custom date-format input behavior. Added `src/components/melt/MeltProgress.svelte` around `melt/builders` `Progress` and migrated Process Manager CPU/memory meters to it while preserving the process grid, sorting, taskbar badges, refresh order, and guarded kill flow. Custom/native shell menus, Stack Browser state machines, resize handling, and taskbar window behavior were intentionally not converted because they are native or bespoke shell state machines rather than direct Melt primitive fits.
- 2026-04-28T14:18:00-05:00 `[TOOL]` VALIDATED: Worker 1 loaded `caveman`, `senior-frontend`, and `tdd-guide`; no Rust/backend skills were loaded because this slice stayed renderer-only. Focused `node --test tests/meltMigrationWiring.test.mjs` passed 3/3, `npm run check` passed with 0 errors/0 warnings, and `npm run test:node` passed 167/167 Node tests.
- 2026-04-28T14:32:00-05:00 `[USER]` IN_PROGRESS: Fix `src/components/melt/MeltProgress.svelte` so the Melt `Progress.progress` `--neg-progress` percentage string is used directly instead of double-scaled in CSS, strengthen `tests/meltMigrationWiring.test.mjs` to prevent regression, preserve unrelated dirty changes, and rerun focused validation.
- 2026-04-28T14:36:00-05:00 `[CODE]` IMPLEMENTED: Updated `MeltProgress.svelte` to use the Melt-documented `transform: translateX(var(--neg-progress))` value directly. Strengthened `tests/meltMigrationWiring.test.mjs` to require the direct `--neg-progress` transform and reject the invalid double-scale `calc(var(--neg-progress) * 100%)` pattern.
- 2026-04-28T14:36:00-05:00 `[TOOL]` VALIDATED: Focused `node --test tests/meltMigrationWiring.test.mjs` passed 3/3 and `npm run check` passed with Svelte check 0 errors/0 warnings.
- 2026-04-28T15:00:00-05:00 `[CODE]` QA FOLLOW-UP: QA found `tests/meltMigrationWiring.test.mjs` still asserted against raw source text, so commented-out wiring could satisfy migration checks. Tightened the test to strip Svelte/HTML, block, and line comments before source-pattern assertions so the Melt migration coverage checks real code.
- 2026-04-28T15:00:00-05:00 `[TOOL]` VALIDATED: QA reran focused `node --test tests/meltMigrationWiring.test.mjs`, `npm run check`, `npm run test:node`, and full `npm run validate`; all passed. Full validation passed Svelte check, production build, 167 Node tests, 153 Rust tests with 1 ignored live tray diagnostic, and Cargo check.
- 2026-04-28T15:12:00-05:00 `[USER]` IN_PROGRESS: Worker B for the exhaustive Melt UI port will migrate bottom-bar launcher buttons, task window buttons, and the process-manager button to `src/components/melt/MeltActionButton.svelte` without changing taskbar behavior. Expected affected surfaces/modules: `src/components/melt/MeltActionButton.svelte`, `src/components/BottomBar.svelte`, bottom-bar CSS only if selectors require it, `tests/meltMigrationWiring.test.mjs`, `tests/taskbarUxState.test.mjs`, and this spec. Constraints: preserve 36px bottom-bar geometry, task group wrappers, pointer drag/reorder/click suppression, keyboard focus via `.task-button`, native context menus, preview delays, IPC/native preview/menu/process-manager contracts, and avoid state-owning Melt builders such as Toggle, Tabs, Popover, or SpatialMenu.
- 2026-04-28T15:28:00-05:00 `[CODE]` IMPLEMENTED: Extended `src/components/melt/MeltActionButton.svelte` to forward native `disabled`, pointerdown, mouseenter, and mouseleave behavior while still rendering a real `<button>` with Melt `Tooltip` trigger/content. Migrated `BottomBar.svelte` launcher, task window, and process-manager command buttons to `MeltActionButton` while preserving `.launcher-button`, `.task-button`, `.process-manager-button`, task group wrapper markup/attrs/handlers, native launcher/task context menus, preview hover timing, task tile pointer state/click suppression, keyboard focus queries, and process-manager anchor rect flow. Strengthened `tests/meltMigrationWiring.test.mjs` for bottom-bar Melt wiring and forbidden state-owning builders; `BottomBar.css` and Rust/backend files were unchanged.
- 2026-04-28T15:28:00-05:00 `[TOOL]` VALIDATED: Worker B loaded `caveman`, `senior-frontend`, and `tdd-guide`; no backend/Rust skill was loaded because this slice stayed renderer-only. Focused `node --test tests/meltMigrationWiring.test.mjs tests/taskbarUxState.test.mjs` passed 9/9, `npm run check` passed with Svelte check 0 errors/0 warnings, and `npm run test:node` passed 169/169 Node tests. Live WebView2 smoke remains useful for visual tooltip placement and exact hover/context-menu feel in the 36px bottom bar.
- 2026-04-28T15:36:00-05:00 `[USER]` IN_PROGRESS: Worker C for the JasonShell Melt UI migration will finish remaining safe renderer action-button conversions to `src/components/melt/MeltActionButton.svelte`. Expected affected surfaces/modules: `MeltActionButton.svelte`, `SearchPanelSurface.svelte`, `TaskPreviewSurface.svelte`, `SettingsPanelSurface.svelte`, `ProcessManagerSurface.svelte`, `ControlPlaneSurface.svelte`, `StackPopupSurface.svelte`, `tests/meltMigrationWiring.test.mjs`, and this spec. Constraints: preserve real button semantics, existing classes, ARIA roles/states, event handlers, drag/drop, keyboard behavior, state, and leave complex Stack Browser grid/file-row or ref-bound controls raw when conversion would risk shell behavior.
- 2026-04-28T15:51:00-05:00 `[CODE]` IMPLEMENTED: Worker C extended `MeltActionButton.svelte` to forward additional generic button attributes/events used by renderer controls, including role, aria-sort, aria-colindex, aria-current, aria-selected, aria-disabled, aria-controls, aria-expanded, name/value/style, keydown, dblclick, mousedown, pointerup/move/cancel, and lostpointercapture. Migrated safe action buttons in Search Panel, Task Preview, Settings Panel, Process Manager, Control Plane section actions, and Stack Browser breadcrumbs/toolbar/sort headers/inline editor/context menus/delete-confirm Delete while preserving existing classes, ARIA semantics, handlers, disabled state, and real `<button>` output. Left Stack Browser file-row buttons, delete-confirm Cancel, and resize grip raw by design because they own grid row selection/drag/drop, focus ref, or pointer-capture resize behavior.
- 2026-04-28T15:51:00-05:00 `[TOOL]` VALIDATED: Worker C loaded `caveman`, `senior-frontend`, and `tdd-guide`; no backend/Rust skill was loaded because this slice stayed renderer-only. Focused `node --test tests/meltMigrationWiring.test.mjs` passed 7/7, `npm run check` passed with Svelte check 0 errors/0 warnings, and `npm run test:node` passed 171/171 Node tests after updating stale Stack Browser context-menu source assertions to expect migrated `MeltActionButton` menu items. Focused `node --test tests/meltMigrationWiring.test.mjs tests/stackPopupContextMenu.test.mjs` passed 10/10.
- 2026-04-28T16:04:00-05:00 `[USER]` IN_PROGRESS: Follow-up Worker C blocker fix for `src/components/melt/MeltActionButton.svelte` so omitted `ariaLabel` props do not render `aria-label=""` and erase visible text accessible names. Expected affected surfaces/modules: `MeltActionButton.svelte`, `tests/meltMigrationWiring.test.mjs`, and this spec. Constraints: preserve explicit labels for icon or ambiguous controls, keep changes inside owned scope, and rerun focused Melt wiring plus Svelte check validation.
- 2026-04-28T16:08:00-05:00 `[CODE]` IMPLEMENTED: Changed `MeltActionButton.svelte` `ariaLabel` from an empty-string default to `string | undefined = undefined`, so text buttons without explicit labels omit `aria-label` and retain their visible text accessible name while explicit labels still render for icon or ambiguous controls. Strengthened `tests/meltMigrationWiring.test.mjs` to require the optional aria-label default and reject `export let ariaLabel = ''`.
- 2026-04-28T16:08:00-05:00 `[TOOL]` VALIDATED: Focused `node --test tests\meltMigrationWiring.test.mjs` passed 7/7. `npm run check` passed with Svelte check 0 errors/0 warnings.
- 2026-04-28T16:20:00-05:00 `[CODE]` QA FOLLOW-UP: Adversarial QA found `MeltActionButton.svelte` still spread the Melt Tooltip trigger unconditionally, which attached `aria-describedby` to text buttons that rendered no tooltip content. Changed the wrapper to spread Tooltip trigger attributes only when `tooltip`, `title`, or explicit `ariaLabel` produces tooltip content, and strengthened `tests/meltMigrationWiring.test.mjs` to require the conditional trigger guard and reject unconditional `actionTooltip.trigger` spreading.
- 2026-04-28T16:20:00-05:00 `[TOOL]` VALIDATED: Orchestrator-reported full `npm run validate` for the Melt UI migration passed before QA: Svelte check 0 errors/0 warnings, production Vite build passed, Node tests 171/171 passed, Rust tests 153 passed with 1 expected ignored live tray diagnostic, and Cargo check passed. Adversarial QA then inspected `src/components/melt/*`, migrated top/bottom/search/task-preview/settings/process/control/stack surfaces, `tests/meltMigrationWiring.test.mjs`, related source assertions, and this spec; after the small Tooltip/ARIA follow-up, focused `node --test tests\meltMigrationWiring.test.mjs` passed 7/7 and `npm run check` passed with Svelte check 0 errors/0 warnings. No remaining critical blockers were found; residual risk is live WebView2 visual/interaction smoke for exact tooltip placement and native shell feel.
- 2026-04-28T16:30:00-05:00 `[TOOL]` REVALIDATED: Final full `npm run validate` passed after the adversarial Tooltip/ARIA follow-up: Svelte check 0 errors/0 warnings, production Vite build passed, `npm run test:node` passed 171/171 Node tests, `npm run cargo:test` passed 153 Rust tests with 1 expected ignored live tray diagnostic, and `npm run cargo:check` passed. `git diff --check` reported only normal CRLF conversion warnings.
- 2026-04-28T17:05:00-05:00 `[USER]` IN_PROGRESS: Remove all UI gradients in favor of uniform colors; make top-bar folder pin switching update the existing Stack Browser without close/open flicker; make Process Manager group Applications, Background processes, and Windows processes like Task Manager; replace Stack Browser breadcrumbs with an editable path textbox that still supports breadcrumb-segment navigation; fix Stack Browser sort indicators for all details columns; hide JasonShell fully while another app is fullscreen; fix taskbar hover previews so each task captures its own process/window instead of the whole screen. Expected affected surfaces/modules: `action_plan_new.md`, global/component CSS, `TopBar.svelte`, `StackPopupSurface.svelte`, `stackPopupState.ts`, `ProcessManagerSurface.svelte`, `processManagerState.ts`, `processManagerUxState.ts`, Rust appbar/fullscreen helpers, task preview/window capture modules, related tests, and this spec.
- 2026-04-28T16:04:31-05:00 `[USER]` IN_PROGRESS: Worker C native shell behavior slice: hide/release JasonShell top/bottom AppBar surfaces while a non-JasonShell foreground window is fullscreen on the primary monitor, restore reservation/surfaces when fullscreen exits, and fix taskbar hover preview capture to target the requested HWND/process with sane window bounds. Expected affected surfaces/modules: `src-tauri/src/appbar.rs`, `src-tauri/src/main.rs`, `src-tauri/src/task_preview.rs`, `src-tauri/src/task_windows/previews.rs`, `src-tauri/src/task_windows/windows.rs`, focused Rust tests, and this spec. Constraints: avoid Process Manager, Stack Popup, CSS, and Worker A/B owned frontend files; preserve unrelated dirty changes.

- 2026-04-28T17:18:00-05:00 `[USER]` IN_PROGRESS: Worker A frontend slice: remove active UI gradients from owned app/surface CSS, replace Stack Browser breadcrumb chrome with an editable current-path textbox that still exposes clickable segment navigation, fix details sort active indicators and `aria-sort`, and add focused Node regressions. Expected affected surfaces/modules: `src/app.css`, `src/App.svelte`, `src/components/*Surface.css`, `src/components/TopBar.css`, `src/components/BottomBar.css`, `src/components/StackPopupSurface.svelte`, `src/components/StackPopupSurface.css`, `src/lib/stackPopupState.ts`, frontend tests under `tests/`, and this spec.
- 2026-04-28T17:34:00-05:00 `[CODE]` IMPLEMENTED: Worker A replaced active gradient app/component UI styling with uniform CSS token backgrounds, added `--js-bg-*` surface/control tokens, updated Stack Browser to render an editable current-path textbox with Enter navigation, Escape draft reset, and clickable path-segment buttons, and made details sort headers expose active classes plus visible indicators while preserving `aria-sort` for Name/Type/Size/Modified. Added `tests/frontendUiPolicy.test.mjs` and updated `tests/meltMigrationWiring.test.mjs` source assertions.
- 2026-04-28T17:34:00-05:00 `[TOOL]` VALIDATED: Worker A ran focused `npx tsc -p tsconfig.test.json; node --test tests/frontendUiPolicy.test.mjs tests/stackPopupState.test.mjs tests/meltMigrationWiring.test.mjs` with 33/33 tests passing, `npm run test:node` with 177/177 Node tests passing, `npm run check` with Svelte check 0 errors/0 warnings, and `rg -n "linear-gradient|radial-gradient|conic-gradient|repeating-linear-gradient|repeating-radial-gradient|background-image|--js-gradient" src` with no matches.
- 2026-04-28T17:42:00-05:00 `[USER]` IN_PROGRESS: Worker A adversarial QA follow-up: typed Stack Browser path submit must validate/list the target before mutating `stackState.currentPath` or history, invalid typed paths must keep the previous folder and show an error without retained rows under the bad path, and behavior coverage should move into state/helper tests beyond source-text wiring. Expected affected surfaces/modules: `src/components/StackPopupSurface.svelte`, `src/lib/stackPopupState.ts`, Stack Browser/frontend tests, and this spec. Constraints: preserve top-bar pin switching/openFolder retained-row behavior, do not touch Process Manager or Rust.
- 2026-04-28T17:50:00-05:00 `[CODE]` IMPLEMENTED: Worker A added `commitValidatedStackFolderListing` and `stackSortHeaderState` helpers in `src/lib/stackPopupState.ts`; typed Stack Browser path submit now calls `listStackFolder(folderPath)` before committing `stackState.currentPath`, history, and entries, so failed typed paths leave the previous folder state intact and only set an error message. Existing `openFolder` remains the retained-row path for top-bar pin switching, breadcrumb/segment navigation, row folder activation, parent navigation, and history.
- 2026-04-28T17:50:00-05:00 `[TOOL]` VALIDATED: Worker A ran focused `npx tsc -p tsconfig.test.json; node --test tests/stackPopupState.test.mjs tests/frontendUiPolicy.test.mjs tests/meltMigrationWiring.test.mjs` with 36/36 tests passing, then ran `npm run check` with Svelte check 0 errors/0 warnings.
- 2026-04-28T16:13:45-05:00 `[CODE]` IMPLEMENTED: Worker C updated `src-tauri/src/appbar.rs` with a primary-monitor fullscreen guard that stores the active shell layout, ignores JasonShell/current-process foreground windows, releases/hides top and bottom AppBars while a non-JasonShell fullscreen foreground window owns the primary monitor, restores AppBar reservation/surfaces when fullscreen exits, and remains compatible with cleanup. Updated `src-tauri/src/task_windows/previews.rs` so task previews validate the requested HWND, reject minimized/hidden/cloaked/hung/unavailable targets, choose sane target bounds, capture from the target window DC before falling back to bounded screen-region `BitBlt`, and test preview bounds plus `SRCCOPY | CAPTUREBLT` ROP behavior.
- 2026-04-28T16:13:45-05:00 `[TOOL]` VALIDATED: Worker C ran `cargo fmt --manifest-path src-tauri/Cargo.toml`; focused `cargo test --manifest-path src-tauri/Cargo.toml appbar` passed 12/12; focused `cargo test --manifest-path src-tauri/Cargo.toml task_windows` passed 21/21; focused `cargo test --manifest-path src-tauri/Cargo.toml task_preview` compiled and ran 0 matching tests; `cargo check --manifest-path src-tauri/Cargo.toml` passed. Live fullscreen/DWM smoke remains useful for real Firefox/game behavior and exact preview pixels.
- 2026-04-28T18:05:00-05:00 `[CODE]` IMPLEMENTED: Completed the cross-surface request and marked `action_plan_new.md` `VALIDATED`. Renderer changes remove active gradient styling in favor of uniform tokens, keep Stack Browser visible while top-bar folder pins switch, add a validated-before-commit editable Stack Browser path box with clickable path segments, and expose active sort indicators plus `aria-sort` for all details columns. Process Manager now groups rows as Applications, Background processes, and Windows processes with per-group expand/collapse while filtering and sorting inside each group. Native changes hide/release JasonShell appbars while a non-JasonShell foreground app is fullscreen on the primary monitor, restore appbar reservation afterward, and make taskbar hover previews capture the requested HWND before bounded fallback.
- 2026-04-28T18:05:00-05:00 `[TOOL]` QA: Adversarial QA initially returned CONCERNS for typed Stack Browser path precommit, source-only sort/path tests, missing Process Manager group expand/collapse, and live-smoke residual risk. Follow-up implementation fixed the code/test blockers; QA recheck returned CLEAN. Residual risk is limited to live desktop smoke for real Firefox/game fullscreen timing, DWM preview pixels, hidden WebView2 Stack delivery/focus-loss timing, and Process Manager popup focus-loss/poll behavior.
- 2026-04-28T18:05:00-05:00 `[TOOL]` VALIDATED: Final scan `rg -n "linear-gradient|radial-gradient|conic-gradient|repeating-linear-gradient|repeating-radial-gradient|background-image|--js-gradient" src` returned no matches. Final `npm run validate` passed: `npm run check` reported Svelte check 0 errors/0 warnings, production build passed, `npm run test:node` passed 181/181 Node tests, `npm run cargo:test` passed 161 Rust tests with 1 ignored live tray diagnostic, and `npm run cargo:check` passed. `git diff --check -- src-tauri/src/appbar.rs` reports whitespace on newly added CRLF lines because that tracked file remains `i/crlf w/crlf`; line endings were preserved to avoid unrelated churn.
- 2026-04-28T18:20:00-05:00 `[USER]` IN_PROGRESS: Fix Stack Browser Open With flyout persistence and useful application choices, enable dragging Stack Browser files to native Windows Explorer as copy operations, make the settings panel scroll so bottom controls are reachable, correct Stack Browser sort arrows so ascending and descending are visually distinct and inactive columns show no arrow, add useful developer/Explorer-rival right-click actions including New Text File, investigate taskbar close failures such as Task Manager, and filter bottom-bar task tiles so internal/system windows like DWM Notification Window do not appear. Expected affected surfaces/modules: `StackPopupSurface.svelte`, `StackPopupSurface.css`, `src/lib/stackPopupState.ts`, `src/lib/stackPopup.ts`, `SettingsPanelSurface.svelte`, `SettingsPanelSurface.css`, `BottomBar.svelte`, `src/lib/taskbarMenus.ts`, Rust stack popup/file-operation/native drag modules, `src-tauri/src/task_windows/*`, `src-tauri/src/taskbar_menu.rs`, related Node/Rust tests, and this spec.
- 2026-04-28T18:28:00-05:00 `[USER]` IN_PROGRESS: Worker B native slice will add Rust Stack Browser Open With candidate/launch commands, New Text File creation, Explorer-compatible file-drag preparation where feasible, taskbar close hardening, and Windows-taskbar-like filtering for internal/system windows. Expected affected surfaces/modules: `src-tauri/src/stack_popup.rs`, `src-tauri/src/stack_popup/*`, `src-tauri/src/main.rs`, `src-tauri/src/contracts.rs`, `src-tauri/src/task_windows/*`, `src-tauri/src/taskbar_menu.rs`, `src/lib/stackPopup.ts` only for command wrappers, `src/ipc/commands.ts`, and focused Rust/Node source tests.
- 2026-04-28T19:08:00-05:00 `[CODE]` IMPLEMENTED: Completed the targeted Stack Browser/settings/taskbar fixes. Stack Browser Open With now keeps the submenu reachable with a CSS hover bridge, loads installed useful app candidates through Rust (`list_stack_open_with_candidates`) and launches by app ID (`open_stack_item_with_app`), keeps fallback suggestions limited to supported app IDs, creates collision-safe New Text File entries, exposes developer folder actions, and starts native Windows Explorer copy drag with a Shell OLE data object from `prepare_stack_file_drag`. Settings panel content is vertically scrollable. Sort arrows come from `stackSortHeaderState` so inactive columns show no arrow and active asc/desc are distinct. Bottom-bar task-window filtering now suppresses DWM/internal/helper windows, and task-window close verifies the HWND actually closes before reporting success.
- 2026-04-28T19:08:00-05:00 `[TOOL]` VALIDATED: Ran `cargo fmt --manifest-path src-tauri\Cargo.toml`; `cargo check --manifest-path src-tauri\Cargo.toml` passed after adding the narrow `Win32_UI_Shell_Common` Windows feature; focused `cargo test --manifest-path src-tauri\Cargo.toml stack_popup` passed 31/31; focused `cargo test --manifest-path src-tauri\Cargo.toml task_windows` passed 25/25; focused `npx tsc -p tsconfig.test.json` passed; focused `node --test tests/stackPopupContextMenu.test.mjs tests/stackPopupState.test.mjs tests/settingsPanelWiring.test.mjs tests/frontendUiPolicy.test.mjs tests/meltMigrationWiring.test.mjs tests/taskbarUxState.test.mjs` passed 51/51.
- 2026-04-28T19:08:00-05:00 `[TOOL]` VALIDATED: Final `npm run validate` passed: Svelte check 0 errors/0 warnings, production build passed, Node tests 185/185 passed, Rust tests 168 passed with 1 ignored live tray diagnostic, and Cargo check passed. Adversarial QA re-review returned CLEAN. Residual live-smoke risks are real WebView2-to-Explorer copy drag behavior, installed-app Open With launch on this machine, exact settings-panel scroll reachability in the live webview, and Windows-protected/elevated windows such as Task Manager that can still veto `WM_CLOSE`.
- 2026-04-29T00:00:00-05:00 `[USER]` IN_PROGRESS: Fix desktop clicks incorrectly hiding the top/bottom GUI while preserving hide only for real fullscreen applications; expand bottom-bar quick-launch right-click options such as Run as administrator and useful launcher actions; add a top-bar sound icon/dropdown left of the time with master volume, per-application volume, and immediate input/output device selection; improve Process Manager at-a-glance CPU/memory/GPU percentage metrics and CPU accuracy. Expected affected surfaces/modules: `src-tauri/src/appbar.rs`, `src-tauri/src/taskbar_menu.rs`, `src-tauri/src/launchers.rs`, audio/session native module(s), `src/components/TopBar.svelte`, `src/components/TopBar.css`, `src/lib/*audio*`, `src/components/ProcessManagerSurface.svelte`, process manager state/tests, backend process metrics, command contracts, validation tests, and this spec.
- 2026-04-29T01:10:00-05:00 `[CODE]` IMPLEMENTED: Fixed the desktop-click fullscreen regression by excluding desktop shell foreground classes from the AppBar fullscreen guard while preserving real fullscreen hide/restore behavior. Expanded launcher native menus with Run as administrator, Properties, shortcut/target reveal, and copy shortcut path actions. Added `src-tauri/src/audio.rs`, `src/lib/audio.ts`, top-bar sound dropdown UI, audio IPC constants/commands, Core Audio master and app-session volume controls, and a narrow Windows `IPolicyConfig` default-device bridge for immediate input/output device switching. Improved Process Manager metrics with Rust memory percent, exact CPU-delta percentage handling, PDH GPU Engine per-PID sampling, GPU/memory meters, and Task Manager-style grouped rows. Fixed sound dropdown semantics from menu to dialog because it contains sliders/selects.
- 2026-04-29T01:15:00-05:00 `[TOOL]` VALIDATED: Full `npm run validate` passed after QA follow-ups: Svelte check 0 errors/0 warnings, production build passed, `npm run test:node` passed 187/187 Node tests, Rust tests passed 171 with 1 ignored live tray diagnostic, and `cargo check` passed. `git diff --check` returned no whitespace errors after normalizing `src-tauri/src/appbar.rs`; remaining output is only line-ending conversion warnings.
- 2026-04-29T01:18:00-05:00 `[TOOL]` QA: Adversarial QA initially blocked completion because GPU was contract-only, and warned on AppBar diff hygiene, sound dropdown role semantics, and CPU exactness coverage. Follow-up implementation added real PDH GPU Engine sampling, fixed the sound dropdown role, strengthened CPU percent test expectations, and cleared `git diff --check` whitespace errors. Live smoke is still useful for real Windows fullscreen timing, actual audio-device route switching, PDH GPU readings on active GPU workloads, and WebView popup behavior.
- 2026-04-29T12:00:00-05:00 `[USER]` IN_PROGRESS: Fix top-bar sound icon click/dropdown behavior so the sound control opens usable master, per-application, input-device, and output-device controls; add process icons at the left of Process Manager names; show Task Manager-style aggregate column percentages for CPU, GPU, memory, and other supported metric columns; align process metric cells directly under their headers. Expected affected surfaces/modules: `src/components/TopBar.svelte`, `src/components/TopBar.css`, `src/lib/audio.ts`, `src/components/ProcessManagerSurface.svelte`, `src/components/ProcessManagerSurface.css`, `src/lib/processManagerState.ts`, `src/features/process-manager/processManagerUxState.ts`, `src/lib/processManager.ts`, `src-tauri/src/process_manager.rs`, `src-tauri/src/audio.rs`, tests, and this spec.
- 2026-04-29T12:20:00-05:00 `[USER]` IN_PROGRESS: Worker B Process Manager slice: add process icons at the far-left of process names, show Task Manager-style aggregate percentages in CPU/GPU/memory column headers, and center metric values under their headers. Expected affected surfaces/modules: `src/components/ProcessManagerSurface.svelte`, `src/components/ProcessManagerSurface.css`, `src/lib/processManager.ts`, `src/lib/processManagerState.ts`, `src/features/process-manager/processManagerUxState.ts`, `src-tauri/src/process_manager.rs`, Process Manager tests, and this spec. Constraints: avoid TopBar/audio files and preserve unrelated dirty-tree changes.
- 2026-04-29T12:35:00-05:00 `[USER]` IN_PROGRESS: Worker A focused fix for top-bar sound icon click/dropdown behavior so the sound control opens usable master, per-application, input-device, and output-device controls. Expected affected surfaces/modules: `src/components/TopBar.svelte`, `src/components/TopBar.css`, `src/lib/audio.ts`, `tests/audioControls.test.mjs`, narrowly necessary audio panel IPC/Rust files, and this spec. Constraints: avoid Process Manager files and preserve unrelated dirty-tree changes.
- 2026-04-29T12:48:00-05:00 `[CODE]` IMPLEMENTED: Worker B added `iconDataUrl`/`icon_data_url` to Process Manager process payloads, extracted executable-path icons through the shared task-window shell icon helper with a per-path cache, rendered far-left process icons with a generic fallback, added visible-row aggregate CPU/memory/GPU percentages in the corresponding column headers, and centered metric cells/header content in `ProcessManagerSurface.css`. Added `aggregateProcessMetrics()` coverage and source wiring assertions for icon/header contracts.
- 2026-04-29T12:48:00-05:00 `[TOOL]` VALIDATED: Worker B ran `npx tsc -p tsconfig.test.json`; focused `node --test tests\processManagerState.test.mjs tests\processManagerUxState.test.mjs tests\processManagerWiring.test.mjs tests\meltMigrationWiring.test.mjs` passed 30/30; `npm run check` passed with Svelte check 0 errors/0 warnings; `cargo test --manifest-path src-tauri\Cargo.toml process_manager` passed 13/13; `cargo fmt --manifest-path src-tauri\Cargo.toml --check` passed. Live Tauri smoke remains useful for actual icon extraction coverage across protected/system processes and exact WebView visual alignment.
- 2026-04-29T13:05:00-05:00 `[CODE]` IMPLEMENTED: Worker A moved the top-bar sound dropdown out of the 26px `top-bar` webview into a hidden persistent `audio-panel` webview. Added `AudioPanelSurface.svelte`, `audio_panel.rs`, `audio-panel` routing/metadata/window creation, `show_audio_panel`/`hide_audio_panel` wrappers, IPC constants/contracts, focus-loss closed event handling, and focused tests. `TopBar.svelte` now keeps a real Melt-backed sound button next to the time, opens the panel from the button rect, syncs `aria-expanded` from `audio-panel:closed`, and leaves master/per-app/input/output controls in the audio panel where sliders/selects are usable.
- 2026-04-29T13:05:00-05:00 `[TOOL]` VALIDATED: Worker A ran `npx tsc -p tsconfig.test.json`; focused `node --test tests\audioControls.test.mjs` passed 4/4; focused `node --test tests\contractsSettings.test.mjs` passed 5/5; `npm run check` passed with Svelte check 0 errors/0 warnings; `cargo fmt --manifest-path src-tauri\Cargo.toml --check` passed; `cargo test --manifest-path src-tauri\Cargo.toml audio_panel` passed 2/2; `cargo check --manifest-path src-tauri\Cargo.toml` passed. Live Tauri smoke remains useful for exact audio-panel placement/focus-loss behavior and actual Windows audio device route switching.
- 2026-04-29T13:22:00-05:00 `[TOOL]` QA: Adversarial QA returned CONCERNS for stale failed audio command promises overwriting newer successful updates, and for Process Manager Windows-process classification because Rust emits extensionless names while the TS Windows-process name set expected `.exe` suffixes. QA also noted the computed thread-count aggregate should be visible if other supported metric columns are included.
- 2026-04-29T13:30:00-05:00 `[CODE]` QA FOLLOW-UP: Worker A sequence-gated audio command failure handling in `AudioPanelSurface.svelte` and refreshes authoritative audio state after the current command fails, preventing stale failed slider/device promises from poisoning newer successful state. Worker B normalized Process Manager Windows-process names by stripping optional `.exe` before classification, added visible aggregate thread totals to the Threads header, and strengthened focused tests for extensionless Rust process names plus thread aggregate wiring.
- 2026-04-29T13:38:00-05:00 `[TOOL]` VALIDATED: Final full `npm run validate` passed after QA follow-ups: Svelte check 0 errors/0 warnings, production build passed, `npm run test:node` passed 191/191 Node tests, `npm run cargo:test` passed 173 Rust tests with 1 expected ignored live tray diagnostic, and `npm run cargo:check` passed. Adversarial QA recheck returned CLEAN. Residual live-smoke risks are exact `audio-panel` WebView2 placement/focus-loss behavior, actual Windows audio route switching, and real protected/system-process icon extraction.

## Maintenance instructions for keeping the spec accurate

- Prefer updating existing functional sections over appending disconnected notes.
- When adding or renaming a Tauri command, update `Backend/Rust command and module map` and any affected surface section.
- When adding or renaming an event, update `Event contracts and IPC boundaries` and add/adjust focused tests where practical.
- When changing user-visible behavior, update the relevant `User behavior` subsection and the `Current system snapshot` if the change is architectural.
- When changing persistence format or file location, update `Persistence/data files/configuration` and add migration/corruption behavior notes.
- When adding tests, update `Validation/test commands and coverage map`.
- When resolving a risk, either remove it with a dated ledger entry or supersede it explicitly; do not silently erase active risk history.
- Keep the change ledger concise but specific. Long implementation detail belongs in the functional spec sections, not only in ledger bullets.
- Do not use this file for secrets, raw logs, or chat transcripts.
