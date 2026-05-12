# Shell Popup Layout Scroll P5

Status: Complete, validated 2026-05-06
Source: `findings.md`, Suggested Fix Order item 5  
Priority: P5, run after `persistent_surface_lifecycle_cleanup_p4.md`

## Goal

Fix clipping and unreachable content in popup/shell surfaces at real default and minimum window sizes.

## Issue

Findings show four layout risks:

- Stack Browser context menus can clip near the bottom of the popup.
- Tray panel can clip large icon sets because it lacks an internal scroll path.
- Process Manager header/body horizontal scroll can desync or right columns can become unreachable.
- Search result list height can clip bottom rows at minimum panel sizes.

JasonShell root surfaces commonly disable page scrolling, so each constrained popup must own an internal scroll region.

## Phase 1: RED Layout Contracts

Status: Complete, 2026-05-06. RED confirmed with `node --test tests\shellPopupLayoutScrollPhase1.test.mjs` failing all four layout contracts before implementation; `npx tsc -p tsconfig.test.json` passed.

Acceptance criteria:

- Tests fail for each affected surface before layout fixes.
- Tests check reachability properties, not just cosmetic class names.
- Fixed root windows remain fixed; scroll is internal to the content region.

Tests to write:

- CSS/source test requiring Stack context menu surfaces and submenus to clamp to viewport and use `max-height` plus `overflow-y: auto`.
- Source/layout test requiring Tray Panel grid or parent content area to use `min-height: 0` and `overflow: auto`.
- Source/layout test requiring Process Manager header and body to share one horizontal scroll container or a sticky header inside the same scroller.
- CSS/source test requiring Search Panel results area to use flex column layout with `min-height: 0` and internal overflow rather than approximate `max-height: calc(...)`.

Implementation tasks:

- No production changes in this phase.
- Make tests resilient to class rename by checking structural intent where possible.

Validation gate:

- Focused Node layout/source tests.
- `npx tsc -p tsconfig.test.json`

## Phase 2: Stack Context Menu Scroll And Clamp

Status: Complete, 2026-05-06. Stack root context menus clamp and scroll internally; row submenus stay anchored to the trigger row and compute max-height from trigger top so pointer travel remains reachable.

Acceptance criteria:

- Stack context menu is fully reachable when opened near bottom of a 980x430 popup.
- Submenus clamp/scroll and remain pointer reachable.
- Existing menu actions, keyboard behavior, and context targets remain unchanged.

Tests to write:

- Unit/helper test for menu placement when menu is taller than available viewport.
- Source/CSS test for submenu scroll constraints.

Implementation tasks:

- Reuse existing `contextMenuPosition` clamping.
- Add computed max height or CSS variable for available viewport height.
- Add internal vertical scrolling to menu and submenu surfaces.


Validation gate:

- Stack context menu tests.
- `npm run check`

## Phase 3: Tray And Search Scrollers

Status: Complete, 2026-05-06. Tray icon grids now scroll inside `.tray-content` while loading/error/empty states stay outside; Search Panel root/results use flex/min-height internal scrolling without approximate result `max-height: calc(...)`.

Acceptance criteria:

- Tray panel with 45 mocked icons can scroll to and invoke the last icon.
- Tray loading/error states stay visible and do not disappear under the scroll region.
- Search panel at minimum size can scroll to the last result or show-more row, fully visible.
- Search input/header geometry does not overlap results.

Tests to write:

- Tray panel source/layout test and mocked large-list behavior test if practical.
- Search CSS/source test for flex/min-height scroller.
- Browser/Tauri smoke checklist for minimum-size search panel.

Implementation tasks:

- Add a tray internal content scroller around the icon grid.
- Add `min-height: 0` to flex ancestors that constrain scroll children.
- Rework search panel root/body/results to a flex column layout.

Validation gate:

- `node --test tests/trayPanelWiring.test.mjs tests/searchPanelState.test.mjs`
- `npm run check`

## Phase 4: Process Manager Horizontal Scroll Sync

Status: Complete, 2026-05-06. Process Manager header and rows now live under one focusable `.process-table-scroll` grid scroller with sticky header/group rows and a 48rem content minimum for default-width horizontal reachability.

Acceptance criteria:

- Header and body columns stay aligned while horizontally scrolling.
- Rightmost action/status columns are reachable at the default 720 px width.
- Sticky header behavior, sorting, group rows, and kill guard remain intact.

Tests to write:

- Source/layout test requiring one shared horizontal scroller or sticky header inside the same scroller.
- Behavior test for sort header markup if structure changes.

Implementation tasks:

- Place header and rows under a single horizontal scroll container, or use CSS grid with shared column template and sticky header inside the same scroller.
- Preserve metric cell centering and icon/name layout.


Validation gate:

- Process manager tests.
- `npm run check`

## Phase 5: Visual QA

Status: Complete, 2026-05-06. Adversarial UI QA initially blocked on detached fixed submenus and non-focusable Process Manager scrollbox; both were fixed and re-reviewed CLEAN. Automated validation passed; live Tauri/browser smoke was not run in this turn.

Acceptance criteria:

- Stack menu bottom action clickable.
- Tray large icon set scrolls.
- Process Manager right columns reachable and aligned.
- Search minimum panel last row fully visible.

Tests to write:

- Add browser/Tauri smoke notes only if automated browser runner cannot safely launch Tauri.

Implementation tasks:

- Run adversarial UI review focused on clipping, keyboard reachability, and regressions from nested scroll containers.


Validation gate:

- `npm run validate` when feasible.
