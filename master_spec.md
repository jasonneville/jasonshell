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
- Primary surfaces: `top-bar`, `bottom-bar`, `task-preview`, `search-panel`, `stack-popup`, and `process-manager`, all routed through a shared `index.html` and selected by Tauri webview window label.
- Native shell reservation: the top and bottom bars reserve primary-monitor edge space via Windows AppBar APIs and adjust/restabilize the work area.
- Top bar: 26 logical pixels high; hosts pinned folder stack rail, date/time pill, and search input; opens search and stack browser auxiliary webviews.
- Bottom bar: 36 logical pixels high; hosts Explorer taskbar `.lnk` launchers, grouped open-window task tiles with previews/menus/activation/minimization/reorder gestures, and a right-edge process-manager button.
- Stack Browser: hidden persistent `stack-popup` webview opened from top-bar pinned folders; supports paged folder reads, navigation history, sorting, selection, file operations, inline rename/new folder, drag/drop, context menus, and pin updates.
- Process manager: hidden persistent `process-manager` webview opened from bottom-bar; lists live processes with CPU, memory, start time, thread count, status, executable-path title, sortable columns, guarded kill action, and focus-loss close behavior.
- Search: top-bar input drives a dedicated `search-panel` webview because the top bar is too short for rich results. Search merges pinned apps, open windows, static commands, and indexed system app/file/folder results.
- Backend: Rust commands are registered in `src-tauri/src/main.rs`; Windows-specific native logic is under `src-tauri/src/*` and `src-tauri/src/task_windows/*`.
- Validation bundle: `npm run validate` runs Svelte check, TypeScript/Vite build, `npm run test:node`, Rust tests, and Cargo check. `npm run test:search` remains a compatibility alias for the broader Node helper/source suite.
- System tray: parked/experimental as of 2026-04-27 Phase 0. The Rust module is compiled for Windows tests only and Tauri commands are intentionally not registered until a shipped surface, capabilities, and live smoke coverage exist.
- Current important residual risk: live Windows/Tauri smoke coverage is still useful for multi-webview event delivery, native popup placement, native Explorer drag cursor behavior, mouse XButton behavior, and exact WebView geometry despite strong unit/build validation.

## Deep technical orientation for future implementation agents

JasonShell should be reasoned about as a small native shell with six long-lived Tauri webview windows, not as a single-page web app. Most defects in this repository come from crossing one of these boundaries incorrectly:

- **Surface boundary:** `src/App.svelte` routes one shared `index.html` bundle to a surface by `getCurrentWindow().label`. A component must assume it only owns its own labeled window and must use explicit Tauri events/commands to affect another surface.
- **IPC boundary:** TypeScript wrappers in `src/lib/*.ts` are the canonical frontend boundary to Rust commands and app events. Components should not invent command strings or event names inline unless the wrapper, event table, tests, and this spec are updated together.
- **Native shell boundary:** `src-tauri/src/appbar.rs`, `explorer.rs`, `layout.rs`, `shell_windows.rs`, `task_windows/*`, `launchers.rs`, `taskbar_menu.rs`, `task_preview.rs`, `search_panel.rs`, `stack_popup.rs`, `process_manager.rs`, and `shell_paths.rs` are allowed to touch native window, process, ShellExecute, AppBar, DWM, OLE DB, COM, GDI, clipboard, and filesystem APIs. Treat these modules as failure domains with explicit rollback, stale-response, and pointer-lifetime invariants.
- **Source of truth:** Rust is authoritative for OS state, persisted stack pins, normalized filesystem paths, latest auxiliary-window payloads, task-window/process enumeration, process killability, launcher validation, and native menu selection. Svelte state is authoritative only for view state such as current selection, scroll/reveal intent, transient drag state, search query, history cursor, retained rows, process sort column/direction, refresh/killing status, and local result ranking usage.
- **Persistence ownership:** `stack_popup.rs` owns stack pin storage. `search_sources/index.rs` owns search index cache storage. `settings.rs` owns versioned shell settings/workspace/task-history foundation storage. `searchRanking.ts` owns only browser `localStorage` usage boosts. Do not persist secrets, HWNDs, active/minimized state, preview payloads, latest popup/search payloads, drag state, process snapshots, or native clipboard mirrors.
- **Staleness model:** Search, preview, stack popup, and process manager use request identifiers, latest-request fallbacks, or sequence gates. New async work must either be idempotent or explicitly rejected when stale. The correct default is to keep the previous visible state until the next authoritative payload arrives rather than clearing UI early.
- **Validation model:** Use pure JS/TS unit tests for reducers, event-target construction, drag/drop parsing, context-menu math, taskbar reorder/click state machines, and process-manager sort/format/source wiring. Use Rust tests for path validation, pagination, filesystem semantics, AppBar geometry helpers, task-window filtering, process-manager helper behavior, search scoring/cache/provider mapping, and Win32 wrapper invariants. Use live `npm run tauri dev` smoke only for WebView2 delivery, exact native geometry, DWM capture, Explorer taskbar/AppBar interaction, native menus, native process-manager placement/focus-loss behavior, native file drops, Open With picker, and mouse XButton delivery.

Implementation mental model by surface:

1. `top-bar` is the orchestration surface for search and stack pins. It owns query text, selected search result, pin rail view state, and reacts to events from `search-panel`, Stack Browser pin mutations, and native top-bar pin menus.
2. `bottom-bar` is the orchestration surface for launchers and open windows. It owns preferred task group order, drag/click disambiguation, preview request sequence, and refresh reactions after native menu actions.
3. `search-panel` is a render-only auxiliary surface for the latest `SearchPanelPayload`. It emits selection/activation/pin intents; top bar executes them.
4. `stack-popup` is a stateful auxiliary folder browser. It owns navigation history, selection, sorting, retained rows, virtualized visible-row calculation, inline editor state, HTML drop suppression, context menus, and background page merging; Rust owns canonical filesystem mutations and pin persistence.
5. `task-preview` is a render-only auxiliary preview surface. Rust owns preview positioning and image capture; bottom bar owns request ordering and hide/show timers.
6. `process-manager` is a stateful auxiliary process table. Rust owns process enumeration, CPU-time snapshots, metadata, positioning, focus-loss hide, and kill guardrails; Svelte owns open/closed refresh cadence, in-flight request gating, filtering, tree-aware display rows, metric mini bars, two-step kill confirmation state, sorting, formatting, and status text.

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
- `src/lib/shellSurface.ts` defines `ShellSurface = 'top-bar' | 'bottom-bar' | 'task-preview' | 'search-panel' | 'stack-popup' | 'process-manager' | 'unknown'` and surface metadata.
- `src/App.svelte` renders:
  - `src/components/TopBar.svelte` for `top-bar`,
  - `src/components/BottomBar.svelte` for `bottom-bar`,
  - `src/components/TaskPreviewSurface.svelte` for `task-preview`,
  - `src/components/SearchPanelSurface.svelte` for `search-panel`,
  - `src/components/StackPopupSurface.svelte` for `stack-popup`,
  - `src/components/ProcessManagerSurface.svelte` for `process-manager`.
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
  - `stack-popup` hides itself when it loses focus.
  - `process-manager` emits `process-manager:closed`, hides itself, and stops frontend polling when it loses focus.
  - App exit also attempts appbar cleanup.

Detailed lifecycle contract from `src-tauri/src/main.rs`:

- `tauri::Builder::default()` installs managed `Mutex` state before any command can run: Windows `ShellRuntimeState` or non-Windows `Mutex<()>`, `TaskPreviewRuntimeState`, `SearchPanelRuntimeState`, `StackPopupRuntimeState`, and `search_sources::SearchIndexRuntimeState`.
- `invoke_handler` registers all commands up front. If a command is added to Rust but not this list, frontend `invoke()` fails at runtime even when Rust compiles.
- `on_menu_event` is centralized in `taskbar_menu::handle_taskbar_menu_event`. Native menu IDs encode action type and payload; do not attach per-menu closures.
- `on_window_event` has three special cases: `stack-popup` hides on `WindowEvent::Focused(false)`, `process-manager` emits `process-manager:closed` then hides on `WindowEvent::Focused(false)`, and primary shell surfaces (`top-bar`, `bottom-bar`) clean AppBars and exit on close/destroy. Avoid adding generic close handlers that would make auxiliary windows terminate the shell.
- `.setup()` creates windows first, starts index warming second, then activates AppBars on Windows. This order matters: AppBar activation needs native HWNDs; search warming must not block surface creation; non-Windows fallback only shows top/bottom windows and leaves AppBar/runtime metric commands unsupported.
- `app.run()` repeats AppBar cleanup on `RunEvent::Exit` and `ExitRequested`. Cleanup is intentionally idempotent through `ShellRuntimeState.cleaned_up`.
- Frontend `reportShellSurfaceRuntimeMetrics(label)` calls the Windows-only `report_shell_surface_runtime_metrics` command about 250ms after `TopBar`/`BottomBar` mount. The returned/logged `ShellSurfaceRuntimeMetrics` contains native rect and frontend `outerHeight`, `innerHeight`, and `clientHeight` to catch zero-height WebView/native-window regressions early.

### Native window and geometry model

- `src-tauri/src/shell_windows.rs` creates all six webview windows as borderless, dark-theme, always-on-top, skip-taskbar windows.
- Window labels and dimensions:
  - `top-bar`: `TOP_BAR_HEIGHT_LOGICAL = 26.0`.
  - `bottom-bar`: `BOTTOM_BAR_HEIGHT_LOGICAL = 36.0`.
  - `task-preview`: `332x228` logical.
  - `search-panel`: `420x320` logical.
  - `stack-popup`: initial `980x430` logical; runtime stack popup height is recalculated by monitor height ratio in `stack_popup.rs`.
  - `process-manager`: `720x520` logical, clamped to the current monitor in `process_manager.rs` when shown.
- `src-tauri/src/layout.rs` computes preview rects for the top and bottom shell bars.
- `src-tauri/src/appbar.rs` handles AppBar registration (`ABM_NEW`, `ABM_QUERYPOS`, `ABM_SETPOS`, `ABM_REMOVE`), work-area mutation, Explorer taskbar hiding/restoring, topmost `SetWindowPos`, startup stabilization polling, and runtime surface metrics.
- Do not reintroduce redundant `SetWindowPos`/AppBar positioning that races WebView2 startup or double-reserves work area; prior continuity recorded regressions around hidden appbar reservations and WebView blank/zero-height startup.
- `shell_windows::create_shell_windows` reads the primary monitor once, converts physical monitor size to logical width using monitor scale factor, then calls `layout::build_shell_preview_rects` with physical top/bottom heights to size the top/bottom shell windows before AppBar activation.
- Window creation order is top bar, bottom bar, task preview, search panel, stack popup, process manager. Only top and bottom are returned to AppBar activation; auxiliary windows are hidden persistent webviews reachable by label.
- All six windows are created with `always_on_top(true)`, `decorations(false)`, `focused(false)`, `resizable(false)`, `maximizable(false)`, `minimizable(false)`, `skip_taskbar(true)`, dark theme, and a context-menu suppression initialization script. `task-preview`, `search-panel`, `stack-popup`, and `process-manager` keep `shadow(true)`; top/bottom bars use `shadow(false)`.
- AppBar activation registers top and bottom HWNDs with `ABM_NEW`, negotiates edge rectangles with `ABM_QUERYPOS`, forces requested thickness back into the returned rect, commits with `ABM_SETPOS`, mutates work area with `SPI_SETWORKAREA`, positions windows topmost with `SetWindowPos(..., SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOOWNERZORDER)`, shows windows, and then polls native rects until three stable matches or failure.
- `appbar.rs` snapshots Explorer primary taskbar state, may hide it, starts a guard thread that repeatedly enforces hidden state every 100ms while active, and restores both taskbar and baseline work area during cleanup. Partial activation failure hides shell windows and rolls back registered AppBars/taskbar/work area.
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

## Top bar spec

### User behavior

- Stays docked on the primary monitor top edge at roughly half taskbar height.
- Shows a horizontally scrollable pinned folder rail on the left/middle.
- Shows current time and date in a compact `time-pill` updated every second.
- Shows a search control on the right with placeholder `Search` and shortcut hint `Ctrl K`.
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
  - `stackPins`, `stackPinsLoaded`, `pendingVisiblePinPath`, `focusedPinIndex` for pin rail.
  - `draggingPinPath`, `pinDropStatus`, `pinRailHover`, scroll affordance flags for drag/drop and rail UI.
- TopBar polls `listOpenTaskWindows()` every second so search window results stay fresh.
- TopBar sends runtime metrics after 250ms via `reportShellSurfaceRuntimeMetrics('top-bar')`.
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
- `src-tauri/src/taskbar_menu.rs` shows native top-bar pin context menus and emits `top-bar:pin-menu-action` back to `top-bar`.
- `src-tauri/src/stack_popup.rs` persists pinned stack folders and returns full next pin arrays from `pin_stack_folder`, `unpin_stack_folder`, and `reorder_pinned_stack_folders`.
- Static shell folder search aliases (`shell:Profile`, `shell:Desktop`, `shell:Personal`/`shell:Documents`, `shell:Downloads`) are accepted by `stack_popup.rs` path normalization and should remain usable for search folder results.
- `open_shell_path` can open search path results directly through `ShellExecuteW`; do not route folders through Stack Browser unless the user action is explicitly pin/open stack.

### Events and IPC

- `search-panel:activate`: emitted by `SearchPanelSurface.svelte`, listened by `TopBar.svelte`, activates selected result.
- `search-panel:select`: emitted by search panel row click/keyboard, listened by top bar to update selection.
- `search-panel:pin-folder`: emitted by search panel pin button, listened by top bar to pin folder and reveal it.
- `search-index:refreshed`: emitted by backend search index warmer, listened by top bar to refresh active indexed search.
- `stack-pins:updated`: emitted by `src/lib/stackPopup.ts` both globally and explicitly to `top-bar` as a `WebviewWindow` target; `TopBar.svelte` listens with the same target shape.
- `top-bar:pin-menu-action`: emitted by Rust native menu handling to `top-bar` with `{ action: 'open' | 'unpin', path }`.
- `getCurrentWindow().onDragDropEvent`: top bar consumes Tauri native file drops and pins dropped folders.

### Top bar tests

- `tests/topBarPins.test.mjs`: event target shape and pin reveal helper behavior.
- `tests/folderDrag.test.mjs`: native/HTML folder drag recognition and path normalization.
- `tests/stackBrowserTopBarPinFlow.test.mjs`: regression for Stack Browser pinning requiring immediate top-bar update with authoritative backend mutation arrays.
- Search-related top-bar state is partly covered by `tests/searchPanelState.test.mjs` and `tests/systemSearch*` helpers when present in the test bundle.

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
- `task_windows/actions.rs` uses parsed HWND strings only. `PostMessageW(WM_CLOSE)` is asynchronous for close; focus/restore/maximize/minimize use `ShowWindowAsync` and `SetForegroundWindow` best-effort semantics.

### DWM/no-activate filtering

- `src-tauri/src/task_windows/windows.rs` rejects windows that are not visible/minimized, are not on the primary monitor, belong to JasonShell/current process, have owners, are DWM cloaked (`DWMWA_CLOAKED`), are tool windows without `WS_EX_APPWINDOW`, have `WS_EX_NOACTIVATE`, lack identity, are known Explorer shell classes (`Shell_TrayWnd`, `Shell_SecondaryTrayWnd`, `Progman`, `WorkerW`), or process name `dwm`.
- `src-tauri/src/task_windows/tests.rs` contains tests for the filtering rules, including no-activate windows.
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
- `tests/processManagerWiring.test.mjs`: source-level routing/command/process-manager button wiring, including the visible Start Time column wired to `startTimeMs` metadata.
- `tests/processManagerState.test.mjs`: process sort toggling, metric/start-time sorting, and CPU/memory/start-time formatter behavior.
- Rust task-window tests in `src-tauri/src/task_windows/tests.rs`: candidate filtering and activity state helpers.
- Rust process-manager tests in `src-tauri/src/process_manager.rs`: kill guardrails and CPU-percent snapshot math.
- Additional validation through `npm run cargo:test`, `npm run check`, and `npm run validate`.

### Known risks

- Live taskbar behavior still depends on Win32 window enumeration quirks, multi-process UWP/modern app behavior, monitor placement, and DWM states.
- This milestone targets the primary monitor only; multi-monitor taskbar parity is not implemented despite feature-document references to broader Cairo behavior.
- Launcher parity with Explorer is conservative: unsupported/unresolvable pins are skipped.

## Visual system and accessibility baseline

- Global visual tokens live in `src/app.css` with `--js-*` variables for text, surfaces, borders, accent, semantic status colors, radii, spacing, shadows, focus ring, scrollbar colors, and motion durations.
- `src/app.css` owns shared focus-visible treatment, `.surface-state` status classes (`loading`, `info`, `warning`, `error`), reduced-motion overrides, and high-contrast/forced-colors token substitutions.
- Surface CSS should prefer tokens from `src/app.css` over hard-coded colors, radii, shadows, spacing, or scrollbar colors. When extracting Svelte-scoped styles into a global CSS file, scope selectors to the surface root class to avoid leaking generic class names such as `.surface` into other webviews.
- Search panel, Stack Browser, and Task Preview styles are split into `SearchPanelSurface.css`, `StackPopupSurface.css`, and `TaskPreviewSurface.css`; their Svelte files import the CSS files explicitly.
- Accessibility baseline:
  - Top-bar rail scroll buttons are real named buttons and the pinned-folder toolbar exposes horizontal orientation.
  - Top-bar search input exposes popup state via `aria-haspopup="listbox"` and `aria-expanded`.
  - Search panel result list owns `aria-activedescendant`; folder pin buttons have explicit labels.
  - Stack Browser grid and status regions expose busy/status state; breadcrumbs mark the current crumb.
  - Process Manager uses a grid with sortable column headers and matching row gridcells, including the action cell.
  - Task Preview is a real button, not a generic div with button role.

## Process manager popup spec

### User behavior

- `process-manager` is a hidden persistent Tauri webview opened by the rightmost button in `bottom-bar`.
- It displays a compact Task Manager-like table with sortable `Name`, `PID`, `CPU`, `Memory`, `Start Time`, `Threads`, `Status`, and `Action` columns.
- Default sort is CPU descending; metric columns, including `startTimeMs`, default to descending while `name` defaults to ascending. Unknown numeric/start-time values sort last under the default descending direction.
- Rows show process executable path in the row title when available. Protected/elevated processes may omit path, memory, CPU, or start-time metadata and display `—` through frontend formatters.
- `Kill` is enabled only when Rust marks a process killable. Kill attempts refresh the table after both success and failure.
- The popup refreshes at `REFRESH_INTERVAL_MS = 1_000` only while open; Escape, Close, explicit hide, or native focus loss closes the popup and stops polling.

### Svelte/TypeScript implementation details

- UI: `src/components/ProcessManagerSurface.svelte` and `src/components/ProcessManagerSurface.css`.
- IPC wrapper and event constants: `src/lib/processManager.ts`.
- Sort/format helpers: `src/lib/processManagerState.ts`.
- `ProcessInfo` payload fields are camelCase in TS: `{ pid, parentPid?, name, executablePath?, cpuPercent?, memoryBytes?, threadCount?, startTimeMs?, status, isKillable }`.
- `ProcessManagerSurface.svelte` listens for `process-manager:open` and `process-manager:closed`, tracks `isOpen`, `isLoading`, `inFlightRequest`, `killingPid`, local `sortState`, and `statusMessage`.
- Refresh is concurrency-gated: if `isLoading` is true, a new refresh returns early; `inFlightRequest` rejects stale responses before replacing rows or status.
- Table formatting uses `formatProcessCpu`, `formatProcessMemory`, and `formatProcessStartTime`. Start time is visible in the UI and sortable through the same `startTimeMs` column already supplied by Rust.

### Rust implementation details

- Backend: `src-tauri/src/process_manager.rs`.
- Commands registered in `src-tauri/src/main.rs`: `show_process_manager`, `hide_process_manager`, `list_processes`, and `kill_process`.
- Window label/dimensions live in `src-tauri/src/shell_windows.rs`: `PROCESS_MANAGER_LABEL = "process-manager"`, `PROCESS_MANAGER_WIDTH_LOGICAL = 720.0`, `PROCESS_MANAGER_HEIGHT_LOGICAL = 520.0`.
- `show_process_manager` finds the `process-manager` and `bottom-bar` windows, clamps popup size to the current/primary monitor, anchors the right edge to `anchorLeft + anchorWidth`, positions it above the bottom bar with physical margins, shows/focuses it, and emits `process-manager:open` on the popup.
- `hide_process_manager` emits `process-manager:closed` then hides the popup.
- `main.rs` also handles `WindowEvent::Focused(false)` for `process-manager` by emitting `process-manager:closed` and hiding the popup so the frontend stops polling when focus leaves.
- Windows process enumeration uses `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)`, `Process32FirstW`/`Process32NextW`, limited `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`, `QueryFullProcessImageNameW`, `K32GetProcessMemoryInfo`, and `GetProcessTimes`.
- CPU percent is derived from process-time deltas stored in `PROCESS_CPU_SNAPSHOTS: OnceLock<Mutex<HashMap<u32, ProcessCpuSnapshot>>>` and pruned to current PIDs after each enumeration.
- Start time is `GetProcessTimes` creation time converted from Windows FILETIME ticks to Unix milliseconds and serialized as `startTimeMs`.
- `kill_process` rejects PID 0 and JasonShell's current PID before attempting `OpenProcess(PROCESS_TERMINATE)` / `TerminateProcess`; non-Windows returns an unsupported error and `list_processes` returns an empty list.

### Process manager tests and risks

- `tests/processManagerState.test.mjs` covers sort toggles/default directions, CPU/memory/start-time formatting, and metric/start-time ordering with unknown values last under descending sort.
- `tests/processManagerWiring.test.mjs` verifies app/surface/command/button wiring and source-level exposure of the visible Start Time column through `startTimeMs` from Rust to Svelte.
- `cargo test --manifest-path src-tauri/Cargo.toml process_manager` covers kill guardrails and CPU-percent helper math.
- Residual risks: protected/elevated processes can omit metadata; first CPU sample is unknown until a second snapshot; process list polling is one-second best-effort rather than real-time; live Tauri smoke remains needed for exact anchored placement/focus-loss close behavior on scaled or multi-monitor setups.

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
- Supports copy, cut, paste, delete, reveal in Explorer, open, file-only `Open with ▸` -> `Choose app...`, pin, inline rename, and inline new folder.
- Supports native and HTML drag/drop: stack rows can carry stack path payloads; dropped paths can be pasted/copied into the current folder.

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
- Row context menus keep root `Open`; file rows expose `Open with ▸` as a hover/focus submenu whose `Choose app...` action calls `openStackItemWithPicker`.
- Rename/new-folder use inline editor input; input pointer/key events must not bubble to popup-level selection/activation handlers.
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
  - Sorting keeps folders before files for all columns; column toggles asc/desc, then compares by name/type/nullable size/nullable modified using locale numeric comparison.
- Context menus and inline editing:
  - Row/background menus are Svelte-rendered because Stack Browser has enough height; top-bar pin menus remain native because 26px clipping makes Svelte menus unsuitable there.
  - `positionContextMenuInViewport()` measures after render, clamps/flips menus into visible viewport, and `rowSubmenuOpensLeft` flips the Open With submenu when insufficient right-side width remains.
  - Row menu always exposes root `Open`; file rows enable `Open with > Choose app...`. Background menu exposes Paste/New Folder.
  - Inline editor form uses `on:click|stopPropagation` and `on:mousedown|stopPropagation` on its input, and keydown ignores popup-level handlers for `.inline-editor` except Escape. Preserve this isolation when changing editing UI.
- Drag/drop and clipboard:
  - Stack rows publish `application/x-jasonshell-stack-paths`, `text/plain`, and folder drag payloads for folders. Drop `shiftKey` selects move/cut; otherwise copy.
  - `lastHtmlDropAt` suppresses duplicate Tauri native file-drop handling for 500ms after an HTML drop.
  - `pasteDroppedPaths()` writes selected paths to the internal/native clipboard through `copyStackItems(paths, move)` and then calls `pasteStackItems(destinationPath)`; it refreshes current folder or reloads when dropping into a child folder.
- File-operation UX:
  - Operations catch and surface first/fallback error text through `operationErrorMessage()`. Paste can partially succeed and surfaces `Paste completed with N failures: first...`.
  - Delete loops selected paths independently, then refreshes current folder and reports partial failures.
  - Pin selected folder calls `pinStackFolder()`; top bar update depends on the wrapper emitting authoritative pins.

### Rust implementation details

- Backend: `src-tauri/src/stack_popup.rs`.
- Runtime state: `StackPopupRuntimeState { latest_request, clipboard }`.
- Persistence file: `stack-folders-v1.json` under the app local data directory.
- `PinnedStackFolder`: `{ id, name, path }`.
- `ShowStackPopupRequest`: `{ path, anchorLeft, anchorWidth, requestId? }`.
- `StackItem`: `{ path, name, kind, typeLabel, iconDataUrl?, sizeBytes?, modifiedAt?, isHidden, isReadonly, isSystem, isSymlink, isReparsePoint }`.
- Pinned-folder mutations return the complete next `Vec<PinnedStackFolder>` to avoid stale frontend list reloads.
- Pin persistence uses temp-file-and-rename semantics and backs up corrupt JSON before falling back.
- `show_stack_popup` normalizes the request, stores latest request, sizes and positions the popup under the top bar, shows/focuses it, and emits `stack-popup:open` directly on the popup window.
- `read_stack_folder` normalizes existing directories and returns `StackFolderPage` with `offset`, `limit`, `total`, `hasMore`, and partial warnings.
- `open_stack_item_with_picker` normalizes an existing file path, rejects directories, and delegates to `shell_paths::open_shell_path_with_picker`; on Windows that uses ShellExecuteW with the `openas` verb to display the OS Open With picker.
- Rename rejects root paths, invalid child names, separators, and collisions.
- Paste implements Explorer-style collision names with `- Copy (n)` suffixes and preserves unresolved failures.
- Copy/cut store internal runtime clipboard and attempt native Windows file clipboard interoperability where available.
- `StackPopupRuntimeState` stores only `latest_request` and an internal `StackClipboard`. It is not a durable history store.
- Path normalization accepts trimmed/quoted paths, `file://` URIs, localhost file URIs, UNC file URIs, extended Windows prefixes (`\\?\`, `\\?\UNC\`, `\??\`, `\??\UNC\`), and supported `shell:` aliases. Canonicalization is required for existing paths; display strings strip extended prefixes.
- Pin defaults are Desktop and Downloads from `USERPROFILE`/`HOME` and are written only when the pin store does not exist. Corrupt pin JSON is renamed to `stack-folders-v1.json.corrupt-<millis>` and an empty/default set is used.
- Pin writes use pretty JSON and atomic temp-write/rename. On Windows `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)` is used with documented UTF-16 pointer lifetimes.
- Pinned-folder identity is path-based: `id == path`, duplicate detection is case-insensitive on canonical paths, and stale/offline unpin falls back to raw normalized path keys so unavailable pins can still be removed.
- `read_stack_folder_page()` reads all child summaries, sorts folders first by lowercase name, then materializes only requested page items. Per-entry failures become warnings rather than failing the entire listing.
- `StackItem` metadata is symlink/reparse-aware: symlink/reparse targets are probed for kind/size while path/name preserve the original link path; badges expose hidden, readonly, system, symlink, and reparse state.
- `validate_child_name()` rejects empty names, separators, trailing dot/space, control characters, Windows-illegal characters, and reserved DOS device basenames (`CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`) even on non-Windows test paths.
- Paste rejects copying a real directory into itself/descendants, rejects symlink/reparse copy for now, uses Explorer-style ` - Copy (n)` collision suffixes up to 999, falls back from `fs::rename` to copy+delete for moves, and preserves unresolved cut clipboard failures.
- Windows delete uses Recycle Bin through `SHFileOperationW(FO_DELETE | FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI)` outside tests; tests/non-Windows use permanent delete helpers.
- Native clipboard interop uses CF_HDROP plus `Preferred DropEffect`. `Copy` maps to effect 1; `Cut` maps to effect 2; reading effect with move bit set maps to cut.
- `open_stack_item_with_picker` normalizes an existing path, rejects directories, and calls `ShellExecuteW` with `openas`. Generic `open_stack_item` delegates to default `open_shell_path`.

### Stack Browser tests

- `tests/stackPopupState.test.mjs`: history, branch navigation, stale payload rejection, retained rows, selection, sorting, formatting, request-key behavior.
- `tests/contextMenuPosition.test.mjs`: viewport clamp/flip placement helpers.
- `tests/stackPopupContextMenu.test.mjs`: source-level regression that rejects misleading `Open width`/`Default width` labels and verifies the `Open with` picker wrapper is wired.
- `tests/folderDrag.test.mjs`: folder drag/drop payload behavior.
- `tests/stackBrowserTopBarPinFlow.test.mjs`: pin mutation publication to top bar.
- `cargo test --manifest-path src-tauri/Cargo.toml stack_popup`: Rust helper coverage for rename validation, folder ordering/metadata, pin persistence/reorder/corrupt-backup, paste collision naming, and clipboard mode behavior.

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

## Backend/Rust command and module map

### Command registrations in `src-tauri/src/main.rs`

- Launchers: `list_pinned_taskbar_apps`, `launch_pinned_taskbar_app`.
- Task windows: `list_open_task_windows`, `activate_task_window`, `maximize_task_window`.
- Task preview: `show_task_window_preview`, `hide_task_window_preview`.
- Menus: `show_task_window_context_menu`, `show_launcher_context_menu`, `show_top_bar_pin_context_menu`.
- Search panel: `show_search_panel`, `hide_search_panel`, `publish_search_panel`, `get_search_panel_payload`.
- Process manager: `show_process_manager`, `hide_process_manager`, `list_processes`, `kill_process`.
- System search: `search_system`.
- Shell paths: `open_shell_path`.
- Stack popup: `list_pinned_stack_folders`, `pin_stack_folder`, `unpin_stack_folder`, `reorder_pinned_stack_folders`, `show_stack_popup`, `hide_stack_popup`, `get_stack_popup_request`, `read_stack_folder`, `open_stack_item`, `open_stack_item_with_picker`, `rename_stack_item`, `copy_stack_items`, `cut_stack_items`, `paste_stack_items`, `delete_stack_item`, `new_stack_folder`, `reveal_stack_item`.
- Settings: `load_shell_settings`, `save_shell_settings`.
- Diagnostics: `record_diagnostic`, `export_diagnostics`.
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
- `contracts.rs`: backend command/event/surface constants and Rust tests for command/event uniqueness.
- `diagnostics.rs`: bounded backend diagnostics ring buffer with recursive field/text redaction and export command.
- `settings.rs`: versioned shell settings load/save/migration, corrupt-file backup, atomic write, and secret-key rejection.
- `search_sources.rs` and `search_sources/*`: app/file/Windows Search persistent indexing and search result scoring.
- `shell_paths.rs`: safe shell path opening and Windows Open With picker launch via ShellExecuteW `openas`.
- `stack_popup.rs`: facade for Stack Browser commands and runtime state. Implementation is split under `src-tauri/src/stack_popup/` into models, paths, items, paging, file operations, pins, clipboard, and popup-window responsibilities while preserving command names and payload shapes.
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
- Search/index invariants:
  - Queries under 2 chars must not hit provider search.
  - Index warming and provider search must be bounded and cache-aware. Adding sources requires explicit root/depth/limit/skip policy.
  - Provider and local results must de-duplicate by id and sort by priority descending then title.
- Stack/file-operation invariants:
  - Mutations return authoritative updated entities or arrays. Pin mutations return complete pin arrays; rename/new folder return `StackItem`; paste returns successes/failures and wrapper reloads the folder.
  - Path aliases are resolved only in stack path normalization. Generic shell open should not interpret additional shell aliases unless the backend validates them.
  - Clipboard mirrors are volatile; failure to publish native clipboard should fail copy/cut because paste semantics would otherwise diverge from user expectation.
- Process-manager invariants:
  - Process enumeration and termination authority stay in Rust; frontend sorting/formatting must not infer killability or synthesize missing process metadata.
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
- `src/lib/settings.ts`: frontend settings schema, default settings, secret-key guard, and settings IPC wrappers.
- `src/lib/searchPanel.ts`: search panel payload/event/IPC contract.
- `src/lib/searchPanelState.ts`: search panel view-state reducer.
- `src/lib/searchCatalog.ts`: local result catalog composition.
- `src/lib/searchRanking.ts`: search result ranking/usage.
- `src/lib/systemSearch.ts`: indexed system search IPC wrapper.
- `src/lib/systemSearchState.ts`: stale response/retry gates.
- `src/lib/stackPopup.ts`: stack popup and stack pin IPC/event wrapper.
- `src/lib/stackPopupState.ts`: stack browser state reducer.
- `src/lib/stackPopupViewModel.ts`: Stack Browser virtual-row, breadcrumb-overflow, delete-prompt, and scroll-into-view helpers.
- `src/lib/stackFileIcons.ts`: icon fallback logic.
- `src/lib/contextMenuPosition.ts`: menu clamp/flip math.
- `src/lib/folderDrag.ts`: folder and stack path drag/drop normalization.
- `src/lib/systemTray.ts`: frontend normalization/click-request helper for the parked tray prototype; covered by Node tests but not wired to shipped Tauri commands or UI.
- `src/ipc/commands.ts`, `src/ipc/events.ts`, `src/ipc/surfaces.ts`, `src/ipc/diagnostics.ts`: shared frontend IPC command/event/surface constants and frontend diagnostics ring-buffer/redaction helpers. Production wrappers should import `IPC_COMMANDS` rather than embedding command string literals.
- `src/features/top-bar/*`, `src/features/bottom-bar/*`, `src/features/search/*`, `src/features/stack-browser/*`, `src/features/process-manager/*`: Phase 2/4 pure feature seams for UX state, grouping, virtualization, and view-model helpers.

### Frontend module ownership categories

- Surface components with Svelte state and DOM/event ownership:
  - `TopBar.svelte`: search query/selection, local catalog refresh, pin rail view/drag/reveal state, search and stack popup orchestration.
  - `BottomBar.svelte`: launcher/window loading state, task group order, preview timers, click-vs-drag transient state, native menu triggers.
  - `StackPopupSurface.svelte`: folder browser history/selection/sort/load state, context menus, inline editor, drag/drop, keyboard navigation.
  - `SearchPanelSurface.svelte`: render latest payload, selected row reveal, row activation/selection/pin/drag intent events.
  - `TaskPreviewSurface.svelte`: render latest preview payload, maximize/hide interactions.
  - `ProcessManagerSurface.svelte`: open/closed refresh lifecycle, process list sorting/display, stale refresh rejection, kill status, and close behavior.
- Thin Tauri IPC/event wrappers:
  - `taskbarLaunchers.ts`, `taskbarWindows.ts`, `taskbarMenus.ts`, `taskbarPreview.ts`, `processManager.ts`, `searchPanel.ts`, `systemSearch.ts`, `stackPopup.ts`, `runtimeMetrics.ts`.
  - These modules should stay boring: define exported types, constants, and `invoke`/`emit` wrappers. Use `IPC_COMMANDS` from `src/ipc/commands.ts` for command names. Add tests when wrapper behavior includes event target construction, payload translation, pagination, or publication side effects.
- Pure reducers/helpers covered by Node tests:
  - `stackPopupState.ts` (`tests/stackPopupState.test.mjs`) for history, request keys, stale/retained rows, sorting, selection, breadcrumbs, formatting.
  - `searchPanelState.ts` (`tests/searchPanelState.test.mjs`) for payload application and selected-row reveal decisions.
  - `taskbarGroups.ts` (`tests/taskbarGroups.test.mjs`) for grouping, order reconciliation, drag displacement, activity eligibility.
  - `taskbarTilePointer.ts` (`tests/taskbarTilePointer.test.mjs`) for click-vs-drag release suppression.
  - `processManagerState.ts` (`tests/processManagerState.test.mjs`) for process sort/default direction and CPU/memory/start-time formatting.
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

IPC wrappers should remain thin and explicit: TypeScript modules in `src/lib/*` call Rust commands through `IPC_COMMANDS` from `src/ipc/commands.ts`; Rust command/event/surface constants live in `src-tauri/src/contracts.rs`. Rust commands return serializable camelCase payloads. Avoid introducing ad-hoc command/event names inside components without updating the frontend IPC modules, backend contracts, tests, and this section.

Capability and CSP contract:

- Tauri capability files are split per current surface under `src-tauri/capabilities/*.json`: `top-bar` remains in `default.json`; `bottom-bar`, `task-preview`, `search-panel`, `stack-popup`, and `process-manager` each have a single-window capability file.
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
  - Unversioned/legacy settings migrate to v1 defaults while preserving `ui.activeWorkspaceId` if present. Unsupported future versions return an error rather than silently truncating.
  - Corrupt JSON is renamed to `*.corrupt-<epoch>.bak` and defaults are returned.
  - Secret-like keys containing token, secret, password, credential, api key, authorization, or cookie are rejected recursively by Rust before load/save returns persisted settings; the frontend wrapper has the same guard before invoking save.
- Durable ownership rule: settings may reserve arrays for future workspaces/task history, but stack pins and search index cache keep their existing files/owners until a later migration explicitly supersedes them.
- Browser localStorage:
  - `jasonshell.search.usage` is a frontend usage boost map owned by `searchRanking.ts`. It should remain small, result-id keyed, and non-sensitive.
- External configuration/source data:
  - Explorer taskbar pins are read from `%APPDATA%\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar`; JasonShell does not write Explorer pins.
  - Search roots come from `APPDATA`, `PROGRAMDATA`, `LOCALAPPDATA`, `ProgramFiles`, `ProgramFiles(x86)`, and `USERPROFILE`.
- Volatile runtime state not to persist: HWNDs, active/minimized/busy state, task group preferred order, preview request id/payload, latest search panel payload, latest stack popup request, Stack Browser history/selection/sort, process-manager rows/sort/CPU snapshots/kill actions, drag/drop state, `StackClipboard`, AppBar registered HWNDs, Explorer taskbar snapshot, and runtime metrics.

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
- `npm run cargo:test`: Rust tests via `cargo test --manifest-path src-tauri/Cargo.toml`.
- `npm run cargo:check`: Rust compile check via `cargo check --manifest-path src-tauri/Cargo.toml`.
- `npm run validate`: full bundle: check, build, JS tests, Rust tests, Cargo check.
- Windows CI: `.github/workflows/windows-ci.yml` runs checkout, Node 20 setup with npm cache, stable Rust setup, `npm ci`, `npm run check`, `npm run build`, `npm run test:node`, `npm run cargo:test`, and `npm run cargo:check` on `windows-latest` for pushes to `main` and pull requests.
- Live smoke checklist: `docs/smoke-test-windows.md` covers manual post-static-validation checks for all six shipped surfaces and explicitly notes system tray as parked/experimental.

Run strategy:

- Documentation-only changes: read back edited headings/sections, search for required headings/terms, and run `git status --short`. Full build is not required unless code snippets/contracts changed in a way that should be cross-checked.
- Pure frontend helper changes: run `npm run test:node` or a focused `npx tsc -p tsconfig.test.json && node --test tests/<file>.mjs`, plus `npm run check` if Svelte/component typing may be affected.
- Svelte component changes: run `npm run check` and the focused Node tests for any helper touched; run `npm run build` when import graphs, payload types, or component syntax changed.
- Rust command/native changes: run focused `cargo test --manifest-path src-tauri/Cargo.toml <module-or-test-filter>` when available, then `npm run cargo:check`; run full `npm run cargo:test` for cross-module command/payload/native helper changes.
- Cross-boundary command/event changes: run at least `npm run check`, `npm run test:node`, `npm run cargo:check`, and focused Rust tests for the command owner. Full `npm run validate` is preferred before declaring a feature slice complete.
- AppBar/window geometry/native shell changes: static validation is insufficient. After check/test/build, run live `npm run tauri dev` and inspect terminal runtime metrics plus actual top/bottom geometry. Use Win32/UIA inspection when debugging zero-height or blank WebView2 regressions.
- Search/index changes: run Rust search tests through `npm run cargo:test` or a focused search filter, and Node tests if top-bar/search-panel state changed. Avoid live broad scans as a substitute for bounded-root tests.
- Stack Browser file-operation changes: run focused `cargo test --manifest-path src-tauri/Cargo.toml stack_popup`, relevant Node tests (`stackPopupState`, `folderDrag`, `contextMenuPosition`, `stackPopupContextMenu`, `stackBrowserTopBarPinFlow`), and `npm run check` for Svelte wiring.
- Process manager changes: run focused process-manager Node tests (`npx tsc -p tsconfig.test.json && node --test tests/processManagerState.test.mjs tests/processManagerWiring.test.mjs`), focused `cargo test --manifest-path src-tauri/Cargo.toml process_manager`, and `npm run check`; add `npm run cargo:check` when Rust or command registration changed.
- Known formatting note: as recorded on 2026-04-26, `cargo fmt --manifest-path src-tauri/Cargo.toml --check` reported pre-existing diffs in `src-tauri/src/stack_popup.rs`. Re-check before relying on this; do not mix broad formatting churn into unrelated feature slices unless explicitly in scope.

### Coverage map

- Top bar pins and Stack Browser pin flow: `tests/topBarPins.test.mjs`, `tests/stackBrowserTopBarPinFlow.test.mjs`. `topBarPins` also guards that `applyStackPins` does not recursively reload pins during hydration.
- Folder drag/drop: `tests/folderDrag.test.mjs`.
- Stack Browser reducer and retained-row semantics: `tests/stackPopupState.test.mjs`.
- Context menu positioning: `tests/contextMenuPosition.test.mjs`.
- Stack Browser context menu labels/Open-with wrapper: `tests/stackPopupContextMenu.test.mjs`.
- Search panel state: `tests/searchPanelState.test.mjs`.
- Taskbar grouping/reorder/activity: `tests/taskbarGroups.test.mjs`.
- Taskbar pointer click-vs-drag: `tests/taskbarTilePointer.test.mjs`.
- Process manager sort/format/source wiring: `tests/processManagerState.test.mjs` and `tests/processManagerWiring.test.mjs`.
- Parked system-tray prototype normalization/click request helpers: `tests/systemTray.test.mjs`.
- Rust stack popup file/pin operations: `cargo test --manifest-path src-tauri/Cargo.toml stack_popup`.
- Rust process-manager helpers: `cargo test --manifest-path src-tauri/Cargo.toml process_manager`.
- Rust task-window filtering/activity: `src-tauri/src/task_windows/tests.rs` through cargo tests.
- Geometry/runtime metrics require live `npm run tauri dev` inspection; static tests cannot prove actual WebView2 native rect behavior.

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
- Do derive CPU percent from process-time snapshots rather than heavyweight per-row sampling loops.
- Do guard destructive actions: never kill PID 0 or the current JasonShell process, and refresh after every kill attempt.
- Do keep non-Windows fallbacks compiling with empty/unsupported behavior.
- Do not persist process IDs, executable paths, CPU history, or kill actions.

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
- 2026-04-26 `[CODE]` Large stack folders progressively load but are not DOM-virtualized.
- 2026-04-26 `[CODE]` Process manager metrics are best-effort: first CPU snapshot can be unknown, protected/elevated processes may omit path/memory/start metadata, and live Tauri smoke is still needed for exact popup placement/focus-loss polling stop behavior.
- 2026-04-27 `[CODE]` System tray support is explicitly parked/experimental. The backend prototype is test-only, frontend helpers are covered by Node tests, and no shipped UI or registered Tauri commands expose tray behavior.
- 2026-04-27 `[TOOL]` Static validation passed for the Phase 1 visual/accessibility baseline, but manual keyboard-only, narrow-width, reduced-motion, and screenshot passes still require live Windows/WebView2 smoke using `docs/smoke-test-windows.md`.
- 2026-04-27 `[TOOL]` Static validation and adversarial QA passed for Phase 2-5, but live Windows/Tauri smoke remains required for per-surface capabilities, production/development CSP behavior, Stack Browser virtual scroll feel and row-height alignment, drag/drop/menu behavior in WebView2, and restart persistence of `jasonshell-settings-v1.json`.
- 2026-04-27 `[CODE]` `src/lib/systemTray.ts` still exposes a typed helper for `invoke_system_tray_icon` as parked/test-covered frontend utility code, but `main.rs` intentionally does not register that command until system tray becomes a shipped surface with capabilities and live smoke coverage.

## Change Ledger

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
