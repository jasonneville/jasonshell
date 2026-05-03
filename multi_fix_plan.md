  # JasonShell Multi-Issue Fix Plan

  ## Spec Snapshot

  - Goal: fix search typing freeze, Stack Browser marquee select, bottom quick-icon pin/unpin, VS Code folder actions,
    settings power actions.
  - Now: Stack Browser has virtual rows + context menus; search has optimistic draft but top-bar query path still does
    too much per input; bottom bar reads Explorer pins only; top-bar folder pins already support unpin.
  - Next: TDD phase-gated implementation, highest priority first.
  - Open Questions: none. Defaults locked: app-managed quick icons; confirm-before-power.

  ## Skill/Orchestration

  - Loaded: caveman ultra, spec-driven-workflow, tdd-guide, senior-frontend, rust-skills.
  - Skipped: image/OpenAI/plugin/prompt/cost skills; no domain touch.
  - Implementation split: frontend worker owns Svelte/state tests; Rust worker owns Tauri commands/menu/system
    actions; QA worker owns focused + full validation. In current Plan Mode no subagents dispatched.

  ## Public Interfaces

  - Add app-managed bottom-bar quick icons under shell settings as quickIcons.entries, default [], no Explorer pin
    mutation.
  - Add Rust/TS wrappers for list_quick_icons, pin_task_window_quick_icon, unpin_quick_icon, launch_quick_icon.
  - Extend task-window native context menu with Pin to quick icons.
  - Extend top-bar pin menu and Stack Browser folder menu with Open in VS Code.
  - Add open_stack_folder_in_vscode(path) or shared open_folder_in_vscode(path) command.
  - Add trigger_system_power_action({ action: "sleep" | "restart" | "shutdown" }), callable only after settings-panel
    confirmation.

  ## Phase 1: P0 Search Typing Freeze

  Tasks:

  - Write RED tests proving rapid Firefox input updates visible query immediately before search IPC/provider work.
  - Split top-bar input echo from search execution: input handler updates local draft only, then schedules latest-only
    query processing after yielding.
  - Remove synchronous publish/show/grouping work from direct input path; keep provider work, panel publish, and
    grouping behind latest-only timer/worker boundary.
  - Convert Rust search_engine to async/blocking-safe execution if profiling/source tests show sync command work can
    starve UI.
  - Keep centered search optimistic draft behavior, keyboard traversal, Escape close, stale response rejection.

  Tests:

  - Node source tests for forbidden direct-input-path calls: no publishSearchPanel, showSearchPanel, searchEngine,
    grouping, or heavy result merge before yield.
  - State tests for latest-only query queue: F, Fi, Fir, Fire, Firef, Firefo, Firefox keeps final draft instantly and
    only latest provider request wins.
  - Existing tests/centeredSearchSurface.test.mjs, tests/searchPanelState.test.mjs, tests/searchUxState.test.mjs.
  - Rust cargo test --manifest-path src-tauri/Cargo.toml search::.

  Acceptance:

  - Typing or pasting Firefox rapidly shows full word immediately in active input.
  - Search results may lag, but text echo never waits on provider/ranking/IPC.
  - Stale provider/progress payloads cannot overwrite latest query.
  - Move to Phase 2 only after focused search tests pass.

  ## Phase 2: P1 Stack Browser Rectangle Selection

  Tasks:

  - Add subtle selection-start gutter/background space in Stack Browser details area without disrupting current visual
    density.
  - Implement primary-button pointer capture marquee selection from gutter/background/spacer only; preserve row click,
    double-click, row drag/drop, context menu, resize grip.
  - Draw Windows Explorer-style selection rectangle overlay.
  - Select every visible virtual row whose DOM rect intersects marquee rect; support additive Ctrl behavior and
    replacing selection by default.
  - Add edge autoscroll while dragging near top/bottom of details body.

  Tests:

  - Unit tests for rect intersection and selected path calculation from row bounds.
  - Source/wiring tests that marquee starts from details background/gutter, not row buttons or resize grip.
  - Existing Stack Browser state/context tests plus focused new marquee test.

  Acceptance:

  - Left mouse drag in blank Stack Browser area selects all rows inside rectangle.
  - Existing row click, double-click open, native file drag, background right-click, delete confirm, resize still
    work.
  - Margins/gutters are usable but visually modest.
  - Move to Phase 3 only after Stack Browser focused tests pass.

  ## Phase 3: P2 Bottom Quick Icons Pin/Unpin

  Tasks:

  - Add app-managed quick icon store in settings, separate from Explorer taskbar pins.
  - Bottom bar renders quick icons first, then existing Explorer launcher rows or merges with duplicate suppression by
    executable/shortcut target.
  - Add Pin to quick icons to task-window right-click menu; backend resolves HWND/process to executable path and icon
    source, persists safe entry.
  - Add Unpin from quick icons to quick-icon right-click menu; Explorer pins keep existing menu without destructive
    unpin.
  - Launch quick icons through direct executable/shortcut path with safe absolute-path validation.

  Tests:

  - TS normalization tests for quick icon settings, duplicate suppression, pin/unpin state.
  - Rust tests for quick-icon schema validation, safe path handling, menu action parsing, non-mutating Explorer
    behavior.
  - BottomBar source tests for rendering quick icons and context menu actions.

  Acceptance:

  - Right-click open process tile -> Pin to quick icons adds icon to bottom quick section.
  - Right-click app-managed quick icon -> Unpin from quick icons removes it immediately and persists.
  - Existing Explorer pinned launchers still launch and keep existing context actions.
  - No code writes into Windows Explorer taskbar pin folder.

  ## Phase 4: P3 Open Folder In VS Code

  Tasks:

  - Add shared folder-open-in-VS-Code command using existing VS Code resolver patterns: %LocalAppData%,
    %ProgramFiles%, code.cmd, code.exe, PATH.
  - Stack Browser row context shows Open in VS Code only for folders; background menu shows it for current folder.
  - Top-bar pinned-folder context menu adds Open in VS Code.
  - Keep popup focus behavior stable; no native blocking dialogs.

  Tests:

  - Rust resolver tests for VS Code executable candidates and missing-code error.
  - Stack Browser context source tests for row-folder and background-current-folder menu items.
  - TopBar/taskbar menu tests for new native menu item and event payload.
  - Manual smoke: right-click repo folder in Stack Browser/top bar opens VS Code at that folder.

  Acceptance:

  - Folder row in Stack Browser can open directly in VS Code.
  - Stack Browser background can open current folder in VS Code.
  - Top-bar pinned folder right-click can open that folder in VS Code.
  - Missing VS Code returns visible non-crashing error.

  ## Phase 5: P4 Settings Power Actions

  Tasks:

  - Add Settings Panel Power section with Sleep, Restart, Turn Off.
  - Each action opens in-panel confirmation; no native confirm().
  - Backend command validates enum action and invokes safe OS path: sleep via Windows power API, restart/shutdown via
    non-shell argument-vector execution.
  - Show command failure inline and keep settings panel alive.

  Tests:

  - Settings panel source tests for three actions and confirmation dialog.
  - TS command wrapper tests for enum-only request shape.
  - Rust tests for enum deserialization, command plan construction, invalid-action rejection.
  - Manual smoke for confirmation UI; live power action only with explicit user approval.

  Acceptance:

  - User can trigger Sleep, Restart, Turn Off from GUI only after confirmation.
  - Settings panel remains keyboard accessible.

  ## Phase 6: Spec, QA, Validation

  Tasks:

  - Update master_spec.md ledger and functional sections after implementation: search responsiveness, Stack Browser
    selection, bottom quick icons, VS Code folder actions, power settings.
  - Run focused suites after each phase, then full npm run validate.
  - Run Windows smoke for search typing, Stack Browser selection, bottom pin/unpin, VS Code folder open, settings
    confirmations.
  - QA pass checks regressions: search keyboard, popup exclusivity, top/bottom bar Alt-Tab exclusion, Stack Browser
    drag/drop, taskbar preview/context menus.

  Final Acceptance:

  - All phase acceptance criteria pass.
  - npm run validate passes.
  - master_spec.md changed with durable behavior and validation notes.
  - No phase advances until previous phase tests and acceptance pass.