## Frontend architecture & performance audit

Scope: `src/` (Svelte 5 + TS), `vite.config.ts`, `tsconfig*.json`, `package.json`, `index.html`, plus build output from `npx vite build` written to a scratch directory (the repo `dist/` was not touched). I did not read docs/ or root plan files. I touched IPC only where it shapes frontend architecture. No repository files were changed.

Baseline facts (all measured or read directly):

- Installed versions: svelte 5.57.0, vite 6.4.3, @sveltejs/vite-plugin-svelte 5.1.1, typescript 5.9.3, melt 0.44.0, @xterm/xterm 6.0.0, @xterm/addon-webgl 0.19.0, @tauri-apps/api 2.10.1, svelte-check 4.7.6, vitest 4.1.11, @floating-ui/dom 1.8.0.
- `svelte-check --tsconfig ./tsconfig.json` result: **408 files, 0 errors, 0 warnings**. This is a good baseline.
- No `{@html}` and no `innerHTML` anywhere in `src/`. This is a good baseline.
- Rust creates **18 webviews eagerly at startup** (`src-tauri/src/shell_windows.rs:110-160`, `create_shell_windows`). The quick-command notification window and snip windows are created later, and all load the same `index.html`.
- Build output: the main JS chunks, raw / gzip:
  - entry `index-*.js`: 169 kB / 40 kB. Every webview loads it.
  - `StackPopupSurface`: 228 kB / 66 kB.
  - `stack-editor-core`: 367 kB / 120 kB.
  - the xterm chunk, which Rollup named `TerminalSearchCloseButton-*.js`: 366 kB / 94 kB.
  - `MeltActionButton`: 84 kB / 28 kB.
  - `TerminalPanelSurface`: 64 kB.
  - `TopBar`: 49 kB.
  - global CSS `index-*.css`: 50 kB. Every webview loads it.
  - `StackPopupSurface.css`: 69 kB.
- Rune adoption census, counted by grep:
  - Runes mode (`$state`, `$props`): ProcessManagerSurface, QuickCommandNotificationSurface, SnipOverlaySurface, SnipPreviewSurface, ThemeEditor, ThemeColorField, SpeechIndicatorOrb, OverflowTicker, SystemInfoView, AnchoredPanelCloseButton, TerminalSearchCloseButton.
  - Legacy mode (`export let`, `$:`, `on:`, `<slot>`): every large surface, i.e. StackPopupSurface, StackGitPanel, TerminalPanelSurface, TopBar, BottomBar, CommandPanelSurface, SettingsPanelSurface, SearchPanelSurface, TaskGallerySurface, TaskPreviewSurface, CalendarPanelSurface, AudioPanelSurface, TrayPanelSurface, ControlPlaneSurface, StackTerminalPane, StackTextEditor, StackConfirmDialog, ContextMenu*, and all five `melt/*` wrappers. `App.svelte` is legacy too.
  - There are no `.svelte.ts` rune modules and no `svelte/reactivity` usage (`SvelteMap`/`SvelteSet`).
- Script/template split in the giants:
  - `StackPopupSurface.svelte`: script is lines 1-3281 of 3739. 127 top-level `let`s, 203 functions.
  - `TerminalPanelSurface.svelte`: script is lines 1-2490 of 2653, with 174 functions.
  - `TopBar.svelte`: script is lines 1-2170 of 2346, with 114 functions.
  - `StackGitPanel.svelte`: script is lines 1-1537 of 3294, with 97 functions.
  - `BottomBar.svelte`: script is lines 1-1371 of 1495.

---

### A. Component size, decomposition, separation of concerns

- **FE-01 — StackPopupSurface.svelte is a 3.7k-line god component with ~10 independent state machines** (High, L).
  - Evidence: `src/components/StackPopupSurface.svelte`.
    - 127 top-level `let` declarations, at lines 163-312.
    - 203 functions; the `</script>` tag is at line 3281.
    - Imports ~90 symbols from 10 modules (lines 1-134).
    - It hosts all of the following in one script:
      - folder listing and navigation (`stackState`, history)
      - folder watch with epochs and chains (`folderWatchEpoch`, `desiredFolderWatch`, `activeFolderWatch`, `folderWatchChain`, lines 293-311)
      - icon hydration queue and diagnostics (17 `iconHydration*`/`iconDiagnostics*` vars, lines 241-260)
      - git summary and context (`gitRepositoryGeneration`, `gitContextSequence`, `gitSummaryPending`, lines 262-275)
      - drag/drop, both local and native (`localDrag`, `pendingLocalDrop`, `rowDragIntent`)
      - marquee selection with autoscroll
      - path autocomplete
      - context menus and the Open-with submenu
      - file operations with progress and undo
      - window resize
      - editor exit coordination
      - terminal view mode
  - Problem:
    - Every piece of state lives in one reactive scope, so any assignment can re-run the ~20 `$:` statements at lines 287-339.
    - Reasoning about the cancellation tokens across features is very hard.
    - Unit testing is only possible through source-regex tests (see FE-44).
  - Recommendation: extract per-domain controllers as `.svelte.ts` classes holding `$state` fields, each owning its own cleanup:
    - `FolderListingController` (load, sort, select, history)
    - `FolderWatchController`
    - `IconHydrationQueue`
    - `StackGitSummary`
    - `StackDragDropController`
    - `MarqueeSelection`
    - `PathAutocomplete`
    - `FileOperationController` (progress, undo)
    - `StackContextMenus`
    
    Then split the template into `StackToolbar`, `StackBreadcrumbs`, `StackDetailsGrid` (virtual list plus row component), `StackStatusBar`, `StackRowMenu`, `StackOpenWithMenu`. The surface becomes a ~300-line composition root. Migrate one controller at a time behind the existing behavior tests.

- **FE-02 — TerminalPanelSurface.svelte keeps two parallel terminal implementations: legacy single-terminal and per-pane runtime** (High, L).
  - Evidence: `src/components/TerminalPanelSurface.svelte` has 25 functions with a `*ForRuntime` twin. Examples:
    - `ensureTerminalView` (964-1000) and `ensureTerminalViewForPane` (1066-1100)
    - `startPolling` (2103) and `startPollingForRuntime` (2085)
    - `pollTerminalOutput` (2158) and `pollTerminalOutputForRuntime` (2119)
    - `writeTerminalData`/`ForRuntime`, `copySelection`/`ForRuntime`, `pasteClipboard`/`ForRuntime`, `scheduleFit`/`ForRuntime`
    
    Both paths construct `new Terminal({...})` with identical options (977-988 and 1070-1081). `ensureTerminalView` delegates to the pane path whenever `activeRuntime()` exists (965-968), so the single path is a fallback that doubles the bug surface.
  - Problem: every fix has to land twice, and the two copies already diverge (e.g. the pane version calls `commitRuntime`). Script length is 2490 lines.
  - Recommendation:
    - Make the pane runtime the only model, and treat "single terminal" as a tree with one pane.
    - Extract a `TerminalPaneRuntime` class (`.svelte.ts`) that owns: xterm instance, addons, ResizeObserver, polling/watchdog, write queue, shell-integration parser, and dispose.
    - Extract `TerminalTabs`, `TerminalToolbar`, `TerminalQuickSelect`, and `TerminalSplitTree` components.

- **FE-03 — The Stack terminal (StackTerminalPane) is a third, separate xterm implementation** (High, M).
  - Evidence: `src/components/StackTerminalPane.svelte:2-27` imports xterm plus fit/search/serialize/unicode11/web-links/webgl. It implements its own:
    - polling (`startPolling` 521-526)
    - output listener (`listen('stack-terminal:output')` 195-200)
    - fit and ResizeObserver (310)
    - context menu
    - clipboard
    - search
  - Duplicate function names with TerminalPanelSurface: `terminalOutputHasVisibleText`, `stripTerminalAnsiControls`, `writeTerminalData`, `enqueueTerminalWrite`, `scheduleFit`, `fitTerminal`, `ensureTerminalView`, `disposeTerminalView`, `openSearch`, `closeSearch`, `pasteClipboard`, `copySelection`, `openTerminalContextMenu`, `closeTerminalContextMenu`, `pasteClipboardFromContextMenu`, `copySelectionFromContextMenu`, `startPolling`, `stopPolling`.
  - Problem: the two terminal experiences have different capabilities. The Stack terminal has WebGL, unicode11, serialize and a link provider; the panel has none of these (FE-24). There are also three copies of the polling and flow-control logic.
  - Recommendation:
    - Build one `<XtermView>` component plus a `TerminalSession` controller in `src/features/terminal/`. Parameterize it with an addon set and a key-binding profile.
    - Make both surfaces consumers of it. This is the single largest code-dedup opportunity (~1.5k lines).

- **FE-04 — StackGitPanel.svelte is 3.3k lines, with the diff renderer copy-pasted four times** (High, M).
  - Evidence: `src/components/StackGitPanel.svelte` lines 1763, 1810, 1870 and 1947. Each is a ~700-815 character single-line `<pre>{#each renderedDiffRows ...}` block with identical markup for changes, staged, history and stash. The component has 97 functions, 21 `$:` statements and 45 `on:` handlers. Pure logic is exported from `<script context="module">` (lines 1-80: `reconcilePendingChangeRows`, `restorePendingChangeRow`, `settleRowOperationDisplay`).
  - Problem:
    - Duplicate markup.
    - Logic is housed in a component module script, which cannot be tree-shaken separately and is awkward to import from tests.
    - `context="module"` is deprecated in Svelte 5 (`<script module>`).
  - Recommendation:
    - Extract `<GitDiffView rows gutterWidth label>`.
    - Split by tab: `GitChangesView`, `GitHistoryView`, `GitStashView`, `GitBranchesView`, `GitCommitBox`.
    - Move the module-script helpers to `src/lib/stackGitPanelState.ts`, which already exists at 185 lines.

- **FE-05 — TopBar.svelte (2.3k lines) is both the top bar UI and the search controller for another webview** (High, L).
  - Evidence: `src/components/TopBar.svelte:1-171` imports from 25 modules: search engine, search panel, taskbar windows and launchers, stack pins, audio, calendar, tray, terminal, command panel, settings, shell bar resize, control plane, context-menu overlay, snipping, mic. The `onMount` spans 216 lines (1953-2169).
    - Search state and engine handling live here: `$: visibleRows = buildVisibleSearchRows(...)` at 384, `loadSearchCatalog` at 536, and listeners for `SEARCH_PANEL_ACTIVATE/SELECT/EXPAND` at 2003-2012.
    - The rendered search UI is in `SearchPanelSurface`.
  - Problem: one component acts as a hub for half the shell. A crash or slow path in any feature (search, snip, audio) affects the always-visible top bar.
  - Recommendation:
    - Extract a `TopBarSearchController`, or better, move search ownership into the search panel itself (FE-29).
    - Extract `TopBarPins`, `TopBarControls` (drag-reorder), `TopBarClock`, and `TopBarLaunchers` components.
    - Each panel opener (`openCommandPanel`, `openCalendarPanel`, ...) should be one generic `togglePanel(surface, anchor)` helper.

- **FE-06 — CommandPanelSurface templates are written as multi-kilobyte single lines** (Med, S).
  - Evidence: `src/components/CommandPanelSurface.svelte`:
    - line 1059 is 2,749 characters (the whole configuration form)
    - line 1061 is 2,663 characters (the whole history/transcript view)
    - lines 1049, 1066 and 1038 are 844-1,188 characters
    - line 1012 is the entire onMount cleanup as one statement
    
    StackGitPanel 1763-1947 and StackMarkdownPreview 26-27 follow the same style.
  - Problem: these lines are unreviewable, diffs are meaningless, and merge conflicts are guaranteed. It looks like formatter output was deliberately avoided.
  - Recommendation: add Prettier with `prettier-plugin-svelte` (or the Biome Svelte support) to the repo and to the `check` script, then reformat once in a dedicated commit. Extract `CommandEditorForm` and `CommandRunHistory` components.

- **FE-07 — BottomBar.svelte mixes taskbar grouping, drag-reorder, gallery orchestration, quick launch, process manager opening and glyph rasterization** (Med, M).
  - Evidence: `src/components/BottomBar.svelte`:
    - script is 1371 lines
    - a `sizeTaskGroupGlyph` action (107-127) with a ResizeObserver plus a window resize listener per glyph
    - the `taskbar:windows-snapshot` handler (1270-1302) recomputes groups, queries the DOM with `document.querySelector(...)` and re-issues `showTaskGalleryNative`
  - Recommendation:
    - Extract a `TaskbarModel` (`.svelte.ts`) that owns windows, group order and the derived groups.
    - Make `TaskGalleryController` own nonce, anchor and layout.
    - Make `TaskGroupButton` a component. Pass anchor rects through a bound element ref instead of global `querySelector`.

- **FE-08 — Inconsistent layering between `src/lib` and `src/features`** (Low, S).
  - Evidence:
    - `src/lib/processManagerState.ts` (362 lines) vs `src/features/process-manager/processManagerUxState.ts` (385)
    - `src/lib/stackPopupViewModel.ts` (765) vs `src/features/stack-browser/viewModel.ts`
    - `src/lib/searchPanelState.ts` vs `src/features/search/searchUxState.ts` (543)
    - `src/lib/stackPopup.ts` (932 lines, 45 `invoke<` calls) is imported by TopBar, TerminalPanelSurface (30) and StackPopupSurface
  - Problem: there is no rule for where code lives, and "stackPopup" has become a grab-bag IPC client used by unrelated surfaces (terminal, top bar).
  - Recommendation: adopt one convention:
    - `src/features/<domain>/{api.ts, model.svelte.ts, view-model.ts, components/}`
    - `src/lib/` only for cross-cutting utilities (theme, prefs, positioning)
    
    Split `stackPopup.ts` into `features/stack-browser/api.ts`, `features/terminal/api.ts` and `features/git/api.ts`.

- **FE-09 — Pure-logic helpers live inside components instead of tested modules** (Med, M).
  - Evidence:
    - `StackPopupSurface.svelte`: `formatModified` (1997), `stackAttributeLabels` (2449), `stackGitStatusForEntry` (2514)
    - `TerminalPanelSurface.svelte`: `terminalOutputHasVisibleText` (2281), `stripTerminalAnsiControls`
    - `CommandPanelSurface.svelte`: `transcriptBodySegments` (849)
  - Recommendation: move these to `features/*` modules with unit tests. This also unblocks FE-44 (behavior tests instead of regex tests).

### B. Svelte 5 runes usage and reactivity

- **FE-10 — The codebase is mostly in Svelte 4 legacy mode under Svelte 5, with no migration plan** (High, L).
  - Evidence: see the census above.
    - `$:` counts: StackPopupSurface 20+, StackGitPanel 21, SearchPanelSurface 12, TopBar 12, BottomBar 8.
    - `on:` counts: StackPopupSurface 64, StackGitPanel 45, TerminalPanelSurface 27.
    - `export let` in all `melt/*` wrappers (MeltActionButton has 38).
    - `<slot>` in ContextMenu, ContextMenuItem, MeltActionButton, MeltToggle and StackConfirmDialog.
  - Problem:
    - Legacy mode is deprecated-track in Svelte 5.
    - `$:` dependency tracking is static, so dependencies hidden behind function calls are missed (FE-12).
    - `on:` and `<slot>` cannot be mixed with snippets and callback props idiomatically.
    - melt 0.44 builders are runes-native, and the wrappers bridge them from legacy props.
  - Recommendation:
    - Run `npx sv migrate svelte-5` per file, leaves first: melt wrappers, ContextMenu*, StackConfirmDialog, icon. Then the mid-size panels.
    - Do the giants only together with their decomposition (FE-01/02/04/05).
    - Set `compilerOptions.runes: true` per migrated file, or globally once done, through `svelte.config.js` `vitePlugin.dynamicCompileOptions`.

- **FE-11 — `afterUpdate` in StackGitPanel forces layout after every state change** (Med, S).
  - Evidence: `src/components/StackGitPanel.svelte:85` imports `afterUpdate`, and `:298-301` runs `resizeCommitTextarea()` plus `syncBranchDropdownResizeObserver()` after every update. `resizeCommitTextarea` (1394-1400) writes `style.height='auto'`, reads `scrollHeight`, then writes the height again. That is a forced synchronous reflow on any change anywhere in a 3.3k-line component, e.g. each pending-row tick or diff load.
  - Problem: layout thrash on unrelated updates. `afterUpdate` is also deprecated in Svelte 5 runes.
  - Recommendation:
    - Resize the textarea only on its own `input` event, or use CSS `field-sizing: content`, which ships in Chromium 123+ and is therefore available in WebView2.
    - Sync the ResizeObserver from a `$effect` that depends only on `branchDropdownOpen` and `branchPickerElement`.

- **FE-12 — Hidden dependencies in legacy `$:` statements** (Med, S).
  - Evidence:
    - `TerminalPanelSurface.svelte:194-195`: `$: visibleRecentCommands = recentTerminalCommands(activeRuntime()?.commandState ?? commandState)`. The `activeRuntime()` call (374-377) reads `paneRuntimes` and `activePaneId`, which the compiler cannot see. The statement only re-runs when `commandState` changes.
    - `StackPopupSurface.svelte` template calls `sortHeader('name')` three times per column header (3544-3547). The function reads sort state invisibly.
  - Problem: the result is stale UI that is masked only because `setActiveRuntime` happens to also assign `commandState`. This is fragile.
  - Recommendation: in runes mode, `$derived(recentTerminalCommands(activeRuntime?.commandState ?? commandState))` with `activeRuntime` itself a `$derived` tracks correctly. Until then, pass the dependencies explicitly as arguments.

- **FE-13 — Immutable Map/Set copying used to trigger reactivity, on hot paths** (Med, M).
  - Evidence:
    - `TerminalPanelSurface.svelte:414-421`: `commitRuntime` does `paneRuntimes = new Map(paneRuntimes).set(...)`. It is called 2-4 times per poll tick per pane (`pollTerminalOutputForRuntime` 2119-2156 calls it on in-flight set, on queue and in `finally`) and on every state change.
    - `appendSessionReplayBuffer` (2207-2211) builds a new Map plus a 256 KB string slice per output chunk for unattached sessions.
    - `CommandPanelSurface.svelte:545,575,916-925` rebuilds `new Set([...activeRunIds, ...])`.
  - Problem: there is O(n) copying per event. Every Map replacement also invalidates every `$:` that mentions `paneRuntimes`, such as `terminalActionState` (181-192, 10 inputs), even when nothing visible changed. With a 1 s watchdog per pane (FE-21), this runs constantly while idle.
  - Recommendation:
    - Use `SvelteMap`/`SvelteSet` from `svelte/reactivity`, or make `TerminalPaneRuntime` a class with `$state` fields, so mutation is fine-grained.
    - Keep non-rendered buffers (`sessionReplayBuffers`) as plain non-reactive fields.
    - Use a ring buffer or chunk array, joined lazily, instead of `(old + new).slice(-262144)` per chunk.

- **FE-14 — `$: if` side-effect statements used as effects** (Med, M).
  - Evidence:
    - `StackGitPanel.svelte:234-236`: `$: if (mounted && folderPath) void refreshStatus();`
    - `:238-243` resets pending rows on folder change
    - `:1534-1536` contains `ensure*Selection()`
    - `StackPopupSurface.svelte:328-335`: cancel drag on any of 11 flags, and sync `pathDraft`
    - `BottomBar.svelte:801-804` closes the gallery reactively
    - `StackTextEditor.svelte:53-54`: `$: onDirtyChange(dirty); $: void loadFile(path);`
  - Problem: these are effects disguised as derivations. Their ordering relative to other `$:` statements is topological and opaque, and async work kicked off from them has no cancellation tied to the dependency change.
  - Recommendation:
    - Convert to `$effect` with explicit cleanup (AbortController per run).
    - Convert pure derivations to `$derived`.
    - Move "on folder change" logic into the controller method that changes the folder, which is event-driven rather than reactive.

- **FE-15 — Hand-rolled cancellation tokens everywhere** (Med, M).
  - Evidence:
    - `StackPopupSurface.svelte` alone has `folderLoadSequence`, `iconHydrationJobToken`, `gitRepositoryGeneration`, `gitRefreshRevision`, `gitContextSequence`, `openWithRevision`, `pathSuggestionRequestSeq`, `popupVisibilityEpoch`, `folderWatchEpoch` and `folderWatchStartingEpoch` (lines 206, 242, 265-267, 176, 236, 296-297, 308).
    - `TerminalPanelSurface` uses `isRuntimeCurrent`.
    - `CommandPanelSurface` uses `readRevision`.
    - `StackTextEditor.svelte:42` uses `requestSequence`.
  - Problem: each feature re-invents "latest request wins" with subtle differences, and missing checks cause stale writes.
  - Recommendation: add one tiny primitive, e.g. `createLatest()` returning `{ run(fn: (signal) => Promise<T>) }` backed by AbortController, or a `Task` class in `.svelte.ts` that exposes `$state` status. Use it uniformly. Pass `signal` through to IPC wrappers so that superseded results are dropped in one place.

- **FE-16 — Mixed runes/legacy components bridging melt builders** (Low, S).
  - Evidence:
    - `src/components/melt/MeltSelect.svelte:20-30`: `new Select({ value: () => value, ... })` where `value` is a legacy `export let`.
    - `ControlPlaneSurface.svelte:50-58`: a `Tabs` builder reading a legacy `let`.
  - Problem: it works because Svelte 5 compiles legacy props to signals, but it relies on legacy internals and blocks idiomatic `bind:value` and snippet APIs.
  - Recommendation: migrate the `melt/*` wrappers first. They are small and used everywhere.

- **FE-17 — A TopBar reactive statement assigns to its own dependency** (Low, S).
  - Evidence: `TopBar.svelte:383`: `$: selectedIndex = Math.min(selectedIndex, Math.max(searchResults.length - 1, 0));`
  - Problem: a self-referencing reactive clamp works in legacy mode but is an anti-pattern, and it does not port to `$derived`.
  - Recommendation: clamp at the write sites, or derive `clampedSelectedIndex`.

### C. Lifecycle, cleanup, listeners, timers

- **FE-18 — The Stack popup polls `getStackPopupRequest` over IPC every 250 ms forever, including while hidden** (High, S).
  - Evidence: `StackPopupSurface.svelte:343-345` sets up `setInterval(() => void reconcileLatestStackPopupRequest(), 250)`. `reconcileLatestStackPopupRequest` (680-687) awaits `getStackPopupRequest()` unconditionally. The visibility check `popup.isVisible()` happens inside `handleOpenRequest` (699), and only after a new request key arrives.
  - Problem:
    - The window is persistent and hidden most of the time, so this costs 4 IPC round-trips per second, 24/7, plus JSON serialization. It keeps the renderer and the Rust command thread awake.
    - WebView2 also throttles timers in hidden pages, which makes this "fallback" unreliable exactly when it matters.
  - Recommendation:
    - Delete the interval.
    - Use the existing `STACK_POPUP_OPEN_EVENT`, plus a pull on `visibilitychange`/`onFocusChanged` (already registered at 431), or a Tauri `Channel` subscription with snapshot-on-subscribe (precedent: `QuickCommandNotificationSurface.svelte:57-61`).
    - If a safety net is still wanted, gate it on `document.visibilityState === 'visible'` and back off to ≥2 s.

- **FE-19 — Systemic "event + pull + polling fallback" pattern across surfaces** (High, M).
  - Evidence:
    - Stack popup: the 250 ms reconcile (FE-18).
    - `SearchPanelSurface.svelte:136-172`: `scheduleFallbackPoll` with `getSearchPanelPayload()`.
    - `CommandPanelSurface.svelte:1004-1011`: pull after listener registration, plus a 1.1 s `historyPollTimer`.
    - Terminal watchdog polls: `TerminalPanelSurface.svelte:2085-2108` (1 s), `StackTerminalPane.svelte:521-526` (1.5 s).
    - `AudioPanelSurface.svelte:74-78` (2 s).
    - Comments such as "The first read runs after registration so an earlier wake cannot be lost" (CommandPanel 1005).
  - Problem: each surface independently works around the race between `listen()` registration and a backend emit to a just-shown webview. The result is N slightly different polling loops and recovery paths.
  - Recommendation: standardize on one primitive. The backend holds a versioned snapshot per surface/topic. The frontend `subscribe(topic, onSnapshot)` passes a `tauri::ipc::Channel`, and the command returns the current snapshot atomically with registration, so no window is lost. Then remove the per-surface polling. This is consistent with Tauri 2 guidance that Channels suit ordered and streamed data better than global events.

- **FE-20 — Calendar clock ticks every second while the panel is hidden** (Low, S).
  - Evidence: `CalendarPanelSurface.svelte:60-64` sets `setInterval(() => now = new Date(), 1000)` for the component's lifetime, and the webview is persistent.
  - Recommendation: start on show and stop on hide (`visibilitychange`), or schedule to the next minute boundary. TopBar (1976-1978) is always visible, but if seconds are off (`showSeconds` pref) it can also align to minute boundaries.

- **FE-21 — Terminal output is both pushed and polled, with no visibility gating** (Med, M).
  - Evidence:
    - `TerminalPanelSurface.svelte:491-501` listens to `stack-terminal:output`, while `startPollingForRuntime` (2085-2092) polls `pollStackTerminal` every 1 s per pane for the whole session lifetime. It is started at 585, 757, 892 and 2440.
    - `StackTerminalPane.svelte:521-526` polls every 1.5 s, commented "Watchdog only".
  - Problem: idle panes still cost one IPC call per second each, plus the Map copies from FE-13. Panes in a hidden terminal panel keep polling.
  - Recommendation: run the watchdog only when no push chunk has been seen for more than X seconds and the document is visible, or drop it once FE-19's channel model guarantees delivery. Use a Tauri `Channel` per session for output, which is ordered and avoids global event fan-out.

- **FE-22 — Listener registration promises lack `.catch` in several places** (Low, S).
  - Evidence:
    - `TopBar.svelte:1967-1974`: `registerAsyncUnlistener` does `.then` with no catch.
    - `StackPopupSurface.svelte:371-379`: `onDragDropEvent(...).then(...)` with no catch.
    - `StackPopupSurface.svelte:356-372`: the file-op progress listener has no catch.
    - Grep counts 18 `.then((unlisten` sites in components and only 5 followed by a catch.
    - The same helper `registerAsyncUnlistener` is re-implemented in 3 components.
  - Problem: there will be unhandled promise rejections, and a failed registration silently leaves features dead.
  - Recommendation: one shared `useTauriListeners()` helper (or a `ListenerBag` class) with `add(promise)`, catch-plus-report, and dispose-safe late unlisten. Use it in all surfaces.

- **FE-23 — Errors are mostly swallowed to `console.*`, with no user-visible or backend error channel** (Med, M).
  - Evidence:
    - 127 `console.error/warn` calls.
    - `TerminalPanelSurface.svelte` has 22 `console.debug` calls that swallow user-initiated failures: copy (1652, 1661), paste (1667, 1678), open target (1945), reveal (1973), open VS Code (1983), and so on.
    - 31 empty or no-op catch blocks.
    - Persistent webviews' consoles are effectively invisible to the user in production.
  - Problem: user actions fail silently, and there is no telemetry or log file for post-mortem.
  - Recommendation:
    - Add `reportError(scope, error, { userVisible })` in `src/lib/`, which logs to a Rust command writing a rotating log (the `tauri-plugin-log` pattern).
    - For user-initiated actions, show a transient inline status or toast. Use `console.debug` only for truly expected conditions.

### D. Rendering performance

- **FE-24 — Terminal renderers are inconsistent: the main terminal panel uses the DOM renderer, the Stack terminal uses WebGL** (Med, S).
  - Evidence: `TerminalPanelSurface.svelte:5-7` imports only fit and search, and never loads `WebglAddon`. `StackTerminalPane.svelte:318-335` loads WebGL with context-loss fallback.
  - Problem: the main terminal (8000-line scrollback, multiple panes) gets the slowest renderer, and the secondary one gets the fastest. Each WebGL addon is one WebGL context. Chromium caps active contexts at ~16 per renderer, so many panes in one webview could hit the cap and evict contexts.
  - Recommendation: in the unified terminal component (FE-03), load WebGL for the focused or visible panes. Fall back to DOM for background panes, or dispose WebGL on hide (`onContextLoss` is already handled). Also load unicode11 in the panel for correct width handling.

- **FE-25 — Stack listing rows do O(rows × git entries) work and build a new `Intl.DateTimeFormat` per row per render** (High, S).
  - Evidence: in `StackPopupSurface.svelte`:
    - `formatModified` (1997-2006) constructs `new Intl.DateTimeFormat(undefined, {...})` on every call. It is called per visible row (3617).
    - `stackGitStatusForEntry` (2514-2528+) loops over all `status.entries` per row, normalizing both paths through `stackGitStatusPathMatchesEntry` (`stackPopupState.ts:150-154`, which calls `normalizeStackPathKey` twice per comparison).
    - `stackState.selectedPaths.includes(entry.path)` is evaluated twice per row (3567, 3583).
    - `stackAttributeLabels(entry)` is called three times per row (3606-3609).
  - Problem: every scroll frame re-renders the virtual window, so (~40 rows plus overscan) × (git entries, possibly thousands in a big repo) × 2 normalizations. Intl formatter construction is one of the more expensive JS builtins.
  - Recommendation:
    - Hoist one module-level `const modifiedFormatter = new Intl.DateTimeFormat(...)`, refreshed on locale or prefs change.
    - Precompute `gitStatusByPath: Map<normalizedPath, kind>` once per `gitStatus` change, including folder roll-up.
    - Precompute `selectedSet = new Set(stackState.selectedPaths)`.
    - Precompute per-entry display fields (labels, formatted size and date) once per listing, not per render. A `StackRow` component receiving precomputed props would make this natural.

- **FE-26 — The Stack virtual list re-renders on every raw scroll event** (Med, S).
  - Evidence:
    - `StackPopupSurface.svelte:326`: `$: virtualEntries = stackBrowserVirtualWindow(visibleEntries, detailsBodyScrollTop, detailsBodyHeight)`
    - `:3558`: `on:scroll={handleDetailsBodyScroll}`, which assigns `detailsBodyScrollTop` (2316)
    - `stackBrowserVirtualWindow` (`stackPopupViewModel.ts:216-260`) allocates a new `rows` array of `{item,index}` each call
  - Problem: even when the visible start index has not changed, as happens on most scroll events within a row height, the whole window is recomputed and every row's reactive expressions re-evaluate.
  - Recommendation:
    - Quantize: only update state when `floor(scrollTop/rowHeight)` changes, and coalesce with rAF.
    - Memoize the rows array by `(startIndex, endIndex, visibleEntries)`.
    - The positive: virtualization exists. Keep it.

- **FE-27 — Git diff renders up to 4,000 rows × 4 spans, unvirtualized and keyed by index** (Med, M).
  - Evidence: `StackGitPanel.svelte:127` sets `maxRenderedDiffLines = 4_000`. `:801-802` parses all rows, and `diffGutterWidth` spreads `Math.max(3, ...renderedDiffRows.flatMap(...))` over up to 8,000 arguments, which risks a call-stack argument limit on large inputs. The `{#each renderedDiffRows as rendered, index (index)}` blocks are at 1763/1810/1870/1947.
  - Problem: there are 16k DOM nodes per diff. Re-selecting a file re-creates them, and keying by index defeats reuse semantics.
  - Recommendation: reuse the Stack virtual-window helper for diffs, which have a fixed row height. Compute the gutter width with a reducing loop. Alternatively, render diffs with a read-only CodeMirror instance that has `@codemirror/merge` unified view; CodeMirror is already bundled for the editor and is virtualized natively.

- **FE-28 — MeltActionButton is heavy for a button: 38 props, 18 always-attached handlers and one Tooltip builder per instance** (Med, M).
  - Evidence: `src/components/melt/MeltActionButton.svelte`:
    - lines 4-44 hold 38 `export let`s
    - lines 83-101 attach 18 `on:` handlers, defaulting to noop closures
    - line 46 runs `new Tooltip({...})` per instance, even when there is no tooltip text
    - it is used about 102 times across 18 components (StackPopupSurface 21, SettingsPanelSurface 14, TopBar 13, ProcessManagerSurface 11)
    - its chunk is 84 kB raw and is loaded by nearly every surface, including tiny ones (AnchoredPanelCloseButton, SnipPreviewSurface)
  - Problem: per-instance overhead (Tooltip state, floating-ui wiring, listeners), a wide prop-forwarding surface that must be kept in sync by hand, and bundle weight for surfaces that only need a close button.
  - Recommendation:
    - Rewrite as a runes component: `let { tooltip, children, ...rest } = $props()`, spreading `{...rest}` onto `<button>` so all aria and on* attributes pass through natively.
    - Instantiate the Tooltip lazily only when `tooltip` is non-empty, or use one shared tooltip singleton per surface driven by `data-tooltip` and event delegation, the way VS Code's hover service works.
    - Expect to remove about 100 lines and most per-instance cost.

- **FE-29 — The search UI is remote-controlled across two webviews; every keystroke and selection round-trips through Rust and the TopBar** (High, L).
  - Evidence:
    - `SearchPanelSurface.svelte:3`: `emitTo(topBarTarget, ...)` for activate, key, select, interaction and pin (184, 234, 239, 248, 260, 336).
    - TopBar listens (2003-2012), runs the search, then calls `publishSearchPanel(payload)` (`searchPanel.ts:198`). That is an invoke, after which Rust emits `SEARCH_PANEL_UPDATE_EVENT` back to the panel (`SearchPanelSurface.svelte:109`).
    - `optimisticQueryDraft` (71) exists to hide the round-trip latency.
  - Problem: input latency is bounded by two IPC hops plus TopBar's main thread. Busy TopBar work (clock, pins, drag) delays search typing. State is split, and payloads carry the whole result list on each update.
  - Recommendation: make the search panel own query state, the engine calls (`searchEngine` already goes to Rust) and result rendering. TopBar only sends "open with initial text/anchor". This mirrors Raycast and the VS Code quick-open model, where the input and the result list live in the same renderer.

- **FE-30 — The Process Manager re-renders the full process list every second, unvirtualized** (Low/Med, M).
  - Evidence: `ProcessManagerSurface.svelte:181-186` refreshes every 1 s (`REFRESH_INTERVAL_MS = 1_000`, line 56); it is correctly gated on `document.visibilityState`. The groups and rows `{#each}` at 520-540 are unvirtualized, and each row has MeltActionButtons (11 instances in the template).
  - The positive: it uses runes properly (`$state.raw`, `$derived`, lines 58-85), which is the best-structured surface.
  - Recommendation: virtualize rows once more than ~150 are shown, reuse the shared VirtualList, and keep `$state.raw` with structural sharing by pid so unchanged rows are not re-rendered. Consider 2 s refresh by default.

- **FE-31 — The CommandPanel transcript renders every line of every expanded run** (Med, M).
  - Evidence: `CommandPanelSurface.svelte:1061`: `{#each history as run}` → `{#each run.transcript as line}` → `{#each transcriptBodySegments(...)}`, with no line cap or virtualization. Segment results are LRU-cached (127-128, 815-817, 849-885), which is good.
  - Problem: long-running commands with chatty output grow the DOM without bound while the run is live, and autoscroll forces layout per update.
  - Recommendation: cap rendered lines per run (tail N plus "show more"), or render the transcript through a virtualized list. For truly long logs, render via xterm read-only, which reuses the terminal stack.

- **FE-32 — Stack text editor copies the whole document on every keystroke** (Med, S).
  - Evidence: `src/features/stack-browser/stackTextEditorAdapter.ts:155-158` runs `draft = update.state.doc.toString(); onChange(draft)` in an `updateListener`. `StackTextEditor.svelte:73` sets `draft = currentDraft`. `:52` computes `dirty = isStackBasicTextEditorDirty(draft, initialContent)`, which is a full string compare (`basicTextEditorDirtyState.ts:1-3`).
  - Problem: there is O(document) allocation plus comparison per keystroke. Typing in multi-MB files will jank, and it doubles memory.
  - Recommendation: track dirtiness with `update.docChanged` plus `state.doc.eq(savedDoc)` (a `Text` comparison that short-circuits on shared structure). Serialize to string only on save, and keep `draft` out of reactive state.

- **FE-33 — File and app icons travel as base64 data URLs held in reactive state** (Med, M).
  - Evidence:
    - `StackPopupSurface.svelte:3597-3598` uses `<img src={entry.iconDataUrl}>`. `StackIconCache` (`src/lib/stackIconCache.ts:1-40`) budgets 2 MB of UTF-16 strings, 256 entries.
    - Search results validate `data:image/...;base64` (`searchEngine.ts:528`).
    - Tray icons use data URLs (`systemTray.ts:20`).
  - Problem: base64 adds 33% to the IPC payload, then double storage as JS strings, then decode per `<img>`. Icons are duplicated per webview (Stack, search, tray, taskbar), with no HTTP-cache reuse.
  - Recommendation: serve icons from a Rust custom URI scheme (`register_asynchronous_uri_scheme_protocol`, e.g. `icon://path-hash?size=16`) with `Cache-Control`, and keep only the URL in state. All webviews then share the WebView2 HTTP and image cache. This is the standard Tauri pattern for binary assets. Note that the CSP `img-src` already allows `asset:`; a custom scheme needs adding there.

### E. Bundle and loading

- **FE-34 — The Stack popup eagerly loads xterm (366 kB), CodeMirror core (367 kB) and the git panel before any of them is used** (High, S).
  - Evidence:
    - `StackPopupSurface.svelte:13-16` statically imports `StackGitPanel`, `StackConfirmDialog`, `StackTerminalPane` and `StackTextEditor`.
    - `StackTerminalPane.svelte:3-11` statically imports all xterm addons. `StackTextEditor.svelte:3-6` pulls in `stackTextEditorAdapter`, which statically imports `@codemirror/view` and state.
    - The built `StackPopupSurface-*.js` has static imports of `stack-editor-core-*.js` and `TerminalSearchCloseButton-*.js` (the xterm chunk), and `__vite__mapDeps` preloads both on surface load.
    - The `vite.config.ts:24-31` comment says the intent is to keep the editor runtime out of the Stack chunk, which it does, but it is still loaded eagerly.
  - Measured: about 1.2 MB raw (~330 kB gzip) of JS parses on first Stack popup open, versus ~480 kB that the listing actually needs.
  - Recommendation: convert all four to dynamic imports, e.g. `{#await import('./StackTextEditor.svelte') then m}<m.default .../>{/await}`, or a small `lazy()` helper. Prefetch on hover or intent (hovering a text file, the Terminal tab button). Language packs are already lazy (`stackTextEditorLanguages.ts:12-21`), so extend the same idea upward.

- **FE-35 — Every webview's entry chunk carries 34 full theme palettes and the theme editor's machinery** (Med, S).
  - Evidence:
    - `src/main.ts:5` imports `./lib/themes`, which imports `themePalette.js`, which imports `builtinThemePalettes.generated.js` (77 kB of source, 34 themes).
    - `themes.ts:2-4` also pulls in `themeSelectionAuthority` and `themePreview` (18.6 kB).
    - `themePalette.ts:89` eagerly computes a `Set` over all presets at module load.
    - The built entry chunk is 169 kB and contains all 34 palettes; a grep found 34 occurrences of `--js-color-bg`.
    - The same palettes also exist as CSS `:root[data-theme='...']` blocks in `src/app.css:133+` (34 blocks), so they ship twice in every webview.
  - Problem: there is duplicate payload and parse work in all 18+ renderers. Only Settings/ThemeEditor needs the JS palettes.
  - Recommendation:
    - Keep CSS as the runtime source of truth (theme switching is just `data-theme`).
    - Dynamic-import `themePalette`, `builtinThemePalettes` and `themePreview` only in ThemeEditor and Settings.
    - Split `themes.ts` into `themeRuntime.ts` (apply, sync, ids) and `themeAuthoring.ts`.

- **FE-36 — Circular module graph in the theme subsystem** (Med, S).
  - Evidence:
    - `themes.ts:1-4` imports `themePersistence`, `themePalette` and `themePreview`.
    - `themePersistence.ts:1` imports `themes`.
    - `themePalette.ts:2` imports `themes`.
    - `themePreview.ts:3` imports `themes`.
  - Problem: ESM cycles create evaluation-order and TDZ hazards. A top-level use like `themePalette.ts:89` running before `themes.ts` has finished initializing is exactly the failure mode. Cycles also defeat code-splitting (FE-35).
  - Recommendation: extract `themeIds.ts` (ids, normalize, `shellThemeById`, storage keys) with no imports, and have every theme module import it. Add `madge --circular src` (or `dpdm`) to `npm run check`.

- **FE-37 — The shared xterm chunk is named after a tiny component, so chunking is accidental** (Low, S).
  - Evidence: the build emits `TerminalSearchCloseButton-*.js` at 366 kB because Rollup hoisted xterm into the first shared module between TerminalPanelSurface and StackPopupSurface.
  - Recommendation: add explicit `manualChunks` groups for `xterm` (`@xterm/*`), `melt` plus `@floating-ui/*`, and `tauri-api`, so chunk names are stable for caching and diagnosis. Add `rollup-plugin-visualizer` behind an env flag, and a bundle-size budget check in validation.

- **FE-38 — One HTML entry for 21 surfaces causes a dynamic-import waterfall at every surface start** (Low/Med, M).
  - Evidence:
    - `index.html` has one module script.
    - `App.svelte:31-43` resolves the label, then `loadSurfaceComponent(surface)` dynamic-imports the surface after `onMount`.
    - The surface then loads its data in its own `onMount`.
    - The chain is: entry parse → App mount → import() → surface chunk and deps → surface mount → IPC.
  - Problem: the deps cannot be modulepreloaded from HTML, which adds startup latency to all 18 webviews created at once, competing for the CPU at shell boot.
  - Recommendation:
    - Option 1: Vite multi-page build, one tiny HTML per surface (`surfaces/top-bar.html`) with Rust pointing `WebviewUrl::App("surfaces/top-bar.html")`. Vite then emits `<link rel=modulepreload>` for exactly that surface's graph, and each entry skips App routing.
    - Option 2 (minimal): kick off `loadSurfaceComponent` at module top level in `main.ts` before `mount`, rather than in `onMount`.

- **FE-39 — Experiments and dead code live in `src/` and the repo root** (Med, S).
  - Evidence:
    - Modules with no import from app code (grep), referenced only by tests:
      - `src/lib/searchRanking.ts` (329 lines, including localStorage usage tracking)
      - `src/lib/searchCatalog.ts` (133)
      - `src/lib/providerContracts.ts` (131)
      - `src/lib/taskbarDiagnostics.ts` (161)
      - `src/lib/automation.ts` (87)
      - `src/lib/systemSearch.ts` (30)
      - `src/features/terminal/terminalQuickFixes.ts` (43)
      - `src/features/stack-browser/textEditorProjectionExperiment.ts` (195)
    - Experiment app: `src/p03-experiment.ts`, `src/components/TextEditorProjectionExperiment.svelte`, `src/features/stack-browser/p03NativeHarness.ts`, `p03-experiment.html`, `vite.p03-experiment.config.ts`.
    - Stale build outputs at the root: `dist-p03-experiment/`, `dist-text-probe/`, `dist-tests/`.
    - `public/loading-spinner-concepts.html` (40 kB) is copied into production `dist/`.
  - Problem: about 1.1k lines of dead or tested-but-unused code, which keeps passing CI and misleads readers into thinking features exist. The design-concept HTML ships inside the app bundle.
  - Recommendation: delete them, or move them under `experiments/` with its own Vite config outside the `src` tsconfig include. Gitignore `dist-*`, and move concept HTML out of `public/`. Add `knip` to `check` to detect unused files and exports continuously.

- **FE-40 — Dependency hygiene** (Low, S).
  - Evidence:
    - Both `bun.lock` and `package-lock.json` are committed.
    - CodeMirror and Lezer are in `devDependencies` while xterm is in `dependencies` (`package.json:25-61`). For a Vite app this is functionally irrelevant, but inconsistent.
    - `@tauri-apps/api` is pinned to `"2.10"`, a loose minor pin that is not exact.
    - `melt ^0.44.0` is a pre-1.0 caret, so minor bumps may break.
    - Vite 6 and vite-plugin-svelte 5 are a major behind the current Vite 7 / plugin 6 line.
  - Recommendation:
    - Pick one package manager and delete the other lockfile.
    - Pin melt exactly (`0.44.0`) until 1.0.
    - Schedule a Vite 7 plus plugin-svelte 6 upgrade together with the runes migration. Vite 7 requires Node 20.19+.
    - Verify the latest versions at upgrade time; I did not query the registry.
  - Positive: `vite-plugin-terminal` is only added for `command === 'serve'` (`vite.config.ts:10-17`), so it is not in production.

### F. State management and cross-window synchronization

- **FE-41 — Four persistence authorities for user state, with localStorage as primary for several** (High, M).
  - Evidence:
    1. Rust `ShellSettings` (`src/lib/settings.ts:56-66`; `load_shell_settings`/`save_shell_settings`).
    2. `shellPreferences` (font, date format, 24h, density, focus ring, transparency) only in localStorage (`shellPreferences.ts:22-33, 292-330`).
    3. Themes and custom themes in localStorage (`themePersistence.ts:24-69`), guarded by a BroadcastChannel "selection authority" protocol (`themeSelectionAuthority.ts`).
    4. Ad-hoc localStorage keys:
       - taskbar launcher order: `BottomBar.svelte:235,330`
       - top-bar control order: `TopBar.svelte:441,1960`
       - centered search size: `searchPanel.ts:343,364`
       - search usage: `searchRanking.ts:313,324`
  - Problem:
    - Rust cannot read preferences, so it cannot pre-size or theme native UI before a webview loads, cannot export or backup them, and cannot migrate them with `ShellSettings.version`.
    - WebView2 profile resets or data-folder changes silently wipe them.
    - All webviews must share one user-data folder and origin, an invisible coupling.
    - The theme "authority" protocol (nonces, discovery, 30 s deadlines) exists only because there is no single writer.
  - Recommendation: move all durable user state into Rust-owned settings, as sections of `ShellSettings` or sibling files with schema versions. Make the frontend a cache:
    - read the snapshot at startup (Rust can inject it via `initialization_script` to keep the no-FOUC theme apply in `main.ts:13-14`)
    - write through commands
    - receive a `settings:changed` event or Channel with a revision
    
    The theme selection authority, preview presence pings and storage-event plumbing then collapse into one code path. Keep localStorage only for disposable UI caches.

- **FE-42 — Cross-window sync fires listeners multiple times per change, and Rust-originated changes do not propagate** (Med, M).
  - Evidence:
    - `settings.ts:200-215`: `broadcastShellSettings` dispatches a window CustomEvent, writes plus removes a localStorage key (which triggers storage events in other windows), and posts on a fresh BroadcastChannel.
    - `addShellSettingsChangeListener` (165-198) listens to all three.
    - A BroadcastChannel delivers to other channel objects in the same document too, so the saving window gets the CustomEvent plus BC, and other windows get the storage event plus BC.
    - Preferences behave the same way: `applyShellPreferences` writes localStorage, which triggers storage events, and `broadcastPreferences` posts on BC (`shellPreferences.ts:325-330, 357-390, 470-478`).
    - Only TopBar (2136) and BottomBar (1341) subscribe to settings changes. StackPopupSurface reads settings once (1660), so the terminal profile goes stale.
    - Changes saved by Rust paths, e.g. hotkey registration or bar-height commands invoked elsewhere, are only broadcast if the invoking frontend calls `broadcastShellSettings`.
  - Recommendation: Rust emits `settings:changed {revision, settings}` after every successful save, whatever the caller. The frontend has one `settingsStore` (`.svelte.ts`) per webview that applies a payload only if `revision > current`. Delete the triple-channel broadcast.

- **FE-43 — Theme preview presence pings run in every webview every 15 s** (Low, S).
  - Evidence: `themePreview.ts:8` sets `PRESENCE_INTERVAL_MS = 15_000`. `:235` runs `setInterval(queryThemePreviewPresence, ...)`, installed by `installShellThemeSync` (`themes.ts:165`) from `App.svelte:32` in all surfaces.
  - Problem: there are 18+ wakeups and broadcast fan-outs every 15 s, forever, to detect an editor that is open only rarely.
  - Recommendation: drive preview from the Settings webview through a Rust or BC message on start and end, with presence only while an editor is open. This disappears with FE-41.

- **FE-44 — 101 of 188 node tests assert on component source text (6,178 regex assertions), which blocks refactoring** (High, M).
  - Evidence:
    - `tests/*.test.mjs`: 101 files `readFileSync` a `.svelte` source.
    - There are 6,178 `assert.match`/`doesNotMatch` assertions in total.
    - Example: `tests/stackPopupContextMenu.test.mjs:5-17` matches `/openWithCandidates/` and `/app\.label/` against `StackPopupSurface.svelte`.
    - Example: `tests/meltMigrationWiring.test.mjs:43-45` matches `new Select<string>` and `select.getOptionId(option.value)`.
    - Only 40 files exist under `tests/components` (vitest plus testing-library).
  - Problem: every decomposition in this report (FE-01..07, FE-10) will break hundreds of tests that verify spelling rather than behavior. This is the strongest force keeping god components in place.
  - Recommendation:
    - Before refactoring, triage the regex tests. Convert behavior-relevant ones into component tests (testing-library plus mocked `@tauri-apps/api`) or pure-module unit tests (FE-09). Delete pure "shape" assertions.
    - Keep a small number of architectural guard tests, e.g. "no direct `invoke` in components", expressed as lint rules (eslint-plugin-svelte plus `no-restricted-imports`) instead of regexes.

- **FE-45 — Surface identity is defined in two places, with diverging metadata** (Low, S).
  - Evidence:
    - `src/lib/shellSurface.ts:1-24` defines a `ShellSurface` union, a metadata map, and `resolveSurfaceFromLabel` with a hard-coded label list (120-141).
    - `src/ipc/surfaces.ts:1-27` defines `SHELL_SURFACES`, another `ShellSurface` type, and `SHELL_SURFACE_TITLES` with different titles ('JasonShell Task Gallery' vs 'Task Gallery').
    - `surfaceLoader.ts:15-37` has a third list, and Rust `shell_windows.rs` a fourth.
  - Recommendation: one registry (`ipc/surfaces.ts`) holding `{label, title, loader}` drives the type, routing and titles, with a contract test against Rust labels (the IPC lane likely already has parity tests to extend).

- **FE-46 — ControlPlaneSurface is a prop-driven component mounted as a surface with no props, so it is effectively a stub occupying a persistent webview** (Med, S).
  - Evidence: `ControlPlaneSurface.svelte:28-37` declares 10 `export let` data props (settings, workspaces, gitStatuses, processes, ...) defaulting to empty. `surfaceLoader.ts:28` mounts it via `<SurfaceComponent />` with no props (`App.svelte:58`). `onMount` only subscribes to themes (45-48). Rust still builds its webview at startup (`shell_windows.rs`: `build_control_plane_window`).
  - Problem: it costs a renderer process and a 14 kB chunk plus a 553-line `controlPlaneState.ts` for a surface that cannot show data.
  - Recommendation: either wire a data loader or remove the surface and window until the feature is real. At minimum, create the window lazily (Big bet 1).

### G. TypeScript strictness and type safety

- **FE-47 — Svelte 4-era ambient module typing erases component prop types, which is why the surface loader uses `any`** (Med, S).
  - Evidence:
    - `src/vite-env.d.ts:3-8` declares `*.svelte` as `ComponentType<SvelteComponent>`, the Svelte 4 class model.
    - There is no `/// <reference types="vite/client" />`.
    - `src/lib/surfaceLoader.ts:3` has `export type SurfaceComponent = any;`.
    - `SettingsPanelSurface.svelte:2` has `// @ts-ignore: CSS side-effect import handled by bundler`.
  - Problem: TS files importing components get untyped props, and the missing vite/client types break CSS, asset and `import.meta.env` typing, which forces `@ts-ignore`.
  - Recommendation: replace with `/// <reference types="svelte" />` plus `/// <reference types="vite/client" />` only. svelte-check and the Svelte TS plugin provide real `Component<Props>` types. Type loaders as `() => Promise<{ default: Component }>` from `svelte`. Then remove the `@ts-ignore`.

- **FE-48 — The tsconfig could be stricter, and Node types leak into browser code** (Low/Med, S).
  - Evidence: `tsconfig.json:3-17` has `strict: true` (good), plus `types: ["node"]` for all of `src/`. It lacks `noUncheckedIndexedAccess`, `noImplicitOverride`, `noFallthroughCasesInSwitch`, `noUnusedLocals`, `noUnusedParameters` and `exactOptionalPropertyTypes`.
  - Problem: `types: ["node"]` lets browser code reference `process`, `Buffer` or `NodeJS.Timeout` without error; the code already mixes `ReturnType<typeof setInterval>` and `number` timers. Index access into Records and arrays, such as `visibleRows[selectedVisibleIndex]?.result` and `pathSuggestions[...]`, is typed non-undefined.
  - Recommendation:
    - Use `types: []` in the app tsconfig, and keep node types for `tsconfig.node.json` and tests only.
    - Enable `noUncheckedIndexedAccess` and `noFallthroughCasesInSwitch` first, since they catch real bugs. Add `noUnusedLocals` alongside `knip`.
    - Expect a moderate fix-up; svelte-check is clean today, so the cost is measurable.

- **FE-49 — IPC results are mostly unchecked casts** (Med, M).
  - Evidence:
    - `invoke<T>` generics serve as type assertions (`stackPopup.ts` has 45, `snipping.ts` 11).
    - Event payloads are cast: `CommandPanelSurface.svelte:1009` `event.payload as QuickCommandRunUpdatedEvent`, `settings.ts:183,189` `JSON.parse(...) as ShellSettings` and `event.data as ShellSettings`.
    - Some modules do validate (`searchPanel.ts:254-330` `is*` guards, `isValidQuickLaunchPayload`), so the codebase has both styles.
    - Raw string event names are used directly in components: `'taskbar:windows-snapshot'` at BottomBar 1270, `'task-gallery:open'` at TaskGallery 449, `'stack-terminal:output'` at TerminalPanel 492 and StackTerminalPane 195. `IPC_EVENTS` constants also exist.
  - Problem: Rust/TS drift shows up as runtime `undefined` deep in UI code rather than at the boundary. (The IPC lane owns contracts; the frontend impact is that defensive `?.` is scattered through components.)
  - Recommendation: generate TS types from Rust (`specta` with `tauri-specta`, or `ts-rs`) and wrap invoke and listen in typed helpers. Validate untrusted cross-window payloads (BroadcastChannel, storage) with small guards at the boundary only. Ban raw `listen('...')` strings in components through lint.

### H. Miscellaneous quality

- **FE-50 — Stack terminal link activation is a no-op, and WebLinksAddon uses its default `window.open` handler** (Med, S).
  - Evidence:
    - `StackTerminalPane.svelte:260-272`: the custom link provider's `activate: () => console.debug('Stack terminal link selected', ...)` does nothing.
    - `:259`: `new WebLinksAddon()` with no handler uses xterm's default, which calls `window.open`. In a Tauri webview that would try to open a new webview window or be blocked, instead of opening the system browser.
    - TerminalPanelSurface has its own quick-select open path (1945) using `openStackItem`.
  - Recommendation: route both through `openStackItem`/shell open with the existing `isSafeStackTerminalOpenTarget` check. Pass `new WebLinksAddon((e, uri) => openExternal(uri))`.

- **FE-51 — `return` inside `finally` swallows exceptions in the terminal poll** (Low, S).
  - Evidence: `TerminalPanelSurface.svelte:2149-2155`: `finally { if (!isRuntimeCurrent(runtime)) return; ... }`.
  - Problem: a `return` in `finally` overrides any in-flight throw or return; it is flagged by `no-unsafe-finally`.
  - Recommendation: restructure the logic to sit after try/catch. Add ESLint (`eslint-plugin-svelte` plus typescript-eslint); there is no linter in `package.json` at all.

- **FE-52 — The App fallback renders the bottom bar when the window label is unavailable** (Low, S).
  - Evidence: `App.svelte:13-19`: `let label = 'bottom-bar'; try { label = getCurrentWindow().label } catch { label = 'bottom-bar' }`.
  - Problem: if routing fails (e.g. in a dev browser tab, or an unexpected Tauri error), a second full taskbar instance boots, with listeners, IPC and appbar semantics.
  - Recommendation: default to `'unknown'`, which already renders a harmless placeholder.

- **FE-53 — Native context menu suppressed globally, including in text inputs** (Low, S).
  - Evidence: `App.svelte:27-29, 59`: `<svelte:window on:contextmenu={suppressNativeContextMenu} />` calls `preventDefault()` for every target.
  - Problem: there is no cut/copy/paste/spellcheck menu in inputs or textareas (search, path bar, commit message, command editor) unless each surface re-implements one.
  - Recommendation: skip suppression when `event.target.closest('input, textarea, [contenteditable]')`, or provide one shared text-edit context menu.

- **FE-54 — Context menus are hand-rolled per surface even though melt is a dependency** (Low, M).
  - Evidence:
    - `ContextMenu`/`ContextMenuItem`/`ContextMenuSeparator` with manual positioning (`contextMenuPosition.ts`) are used in CommandPanel, QuickLaunch, StackGitPanel, StackPopup, StackTerminalPane, TaskGallery and TerminalPanel.
    - `StackPopupSurface.svelte:2286-2309` implements submenu open/close timers plus a ResizeObserver repositioning.
    - The bars use a separate overlay webview (`contextMenuOverlay.ts`). That is justified, since the bars are too short to host menus.
  - Recommendation: one `Menu` primitive (melt `Menu`, or a small in-house one) with roving focus, typeahead and submenu handling. It reduces per-surface keyboard-handling code (7 `handleKeydown` implementations exist).

---

### Big bets

**BB-1 — Webview consolidation and lazy surface lifecycle (biggest memory and startup lever).**

Today 18 webviews are built eagerly at boot (`shell_windows.rs:110-160`), each running the full entry chunk, theme sync, preference sync, preview presence, and its own polling. WebView2 shares one browser process, but each webview generally gets its own renderer process (tens of MB private working set each), plus GPU and compositor resources. That is likely several hundred MB idle for a shell that should be lean.

Measure first. Add a dev command that reports per-renderer private bytes, via WebView2 `GetProcessInfos` / `ICoreWebView2Environment8::GetProcessInfos`, or Task Manager grouping, before and after.

Strategy:
1. **Tiered lifecycle.** Keep the always-visible surfaces resident: top bar, bottom bar, speech indicator, context-menu overlay and search, since latency matters for those. Create transient panels (settings, control plane, process manager, calendar, audio, tray, speech history, quick launch, command panel, task gallery) **lazily on first open**, and destroy them after N minutes hidden. The frontend already reconciles state on open; FE-19's snapshot-on-subscribe makes cold start safe.
2. **Flyout host.** Merge mutually exclusive anchored panels (calendar, audio, tray, speech history, quick launch, settings) into one "flyout" webview that swaps the mounted component. Only one is visible at a time by design. This is how Windows 11's own shell flyouts and Raycast's single window behave. Tauri 2 also supports multiple webviews per window (`Window::add_child`, behind the `unstable` feature), which can be evaluated for co-locating panels with the bars.
3. Keep the heavy, stateful surfaces (Stack browser, terminal panel) as their own webviews, but load their sub-features lazily (FE-34).

**BB-2 — A single state architecture: Rust as the source of truth, versioned snapshots, rune stores per webview.**
- Rust owns durable state: settings, preferences, themes, orders and pins. It exposes `subscribe(topic) -> Channel<Snapshot>` with a monotonic revision.
- Each webview has `src/stores/<domain>.svelte.ts` classes (`SettingsStore`, `ThemeStore`, `TaskbarStore`, `TerminalSessionsStore`) that hold `$state.raw` snapshots, apply only newer revisions, and expose `$derived` view models.
- Components read stores and call store methods, never `invoke` directly. Lint can ban `@tauri-apps/api/core` imports in `components/`; today `BottomBar.svelte:3,1217`, `QuickLaunchPanelSurface.svelte:2,55,141` and `SettingsPanelSurface.svelte:6,599` invoke directly.
- This removes the BroadcastChannel and localStorage machinery (FE-41/42/43), the theme selection authority, and the per-surface polling (FE-18/19/21). It is the VS Code main/renderer model, where the main process owns configuration and renderers get change events.

**BB-3 — Shared UI kit and feature modules (`src/ui/`).**

Build runes-native primitives:
- `Button` (spread props, lazy tooltip)
- `Tooltip` service (one per surface)
- `Menu`/`ContextMenu`
- `Dialog`/`ConfirmDialog`
- `VirtualList` (from the Stack helper; reused for diffs, processes and transcripts)
- `XtermView` plus `TerminalSession`
- `DiffView`
- `PanelShell` (header, close, Escape, focus restore)
- `SegmentedTabs`

Coordinate with the visual-design lane so tokens and components converge. Expected effects: deleting the 18-handler MeltActionButton, the 3 terminal copies, the 4 diff copies and the 7 `handleKeydown` variants, while shrinking the shared chunk for tiny surfaces.

**BB-4 — Incremental runes migration coupled to decomposition, with tests first.**

Order:
1. Convert or retire source-regex tests for the target file (FE-44).
2. Extract pure logic into modules with unit tests (FE-09).
3. Extract controllers into `.svelte.ts` classes.
4. Split components.
5. Switch the file to runes (`sv migrate`).

Sequence by risk and value:
1. melt wrappers and ContextMenu
2. StackConfirmDialog and StackTextEditor
3. SearchPanel, together with search ownership (FE-29)
4. TerminalPanel and StackTerminalPane, unified (FE-02/03)
5. StackGitPanel
6. StackPopupSurface
7. TopBar and BottomBar

Enable a global `runes: true` once the last legacy file is gone.

**BB-5 — Performance guardrails in CI.**
- A bundle budget per surface: fail if the entry exceeds 120 kB, or if a surface's initial graph exceeds its budget.
- `madge --circular`.
- `knip` for dead files and exports.
- ESLint plus `eslint-plugin-svelte` (`no-unsafe-finally`, `svelte/valid-compile`, `no-restricted-imports`).
- Prettier for `.svelte`.
- A dev-only perf HUD per surface: count of active intervals, IPC calls per minute, listener count. That makes regressions like the 250 ms poll visible immediately.
