# JasonShell Future Enhancements Roadmap

Generated: 2026-04-27

This document is a planning roadmap for JasonShell after reviewing `master_spec.md`, `features.md`, the current Svelte/TypeScript frontend, the Rust/Tauri backend, tests, and repository structure. It is written for future implementation sessions and for AI agents entering the project with no prior conversation context.

JasonShell should be treated as a small native Windows shell and developer workflow control plane, not as a single web app. The most valuable long-term direction is to become an Explorer/taskbar replacement that understands projects, processes, tasks, files, windows, search, terminals, editors, and automation for a power developer.

## Audit Inputs

- Primary spec: `master_spec.md`
- Feature reference: `features.md`, especially `## Visual System`
- Current product docs: `README.md`, `stack_browser.md`, `plan.md`, `wave_1.md` through `wave_5.md`
- Frontend surfaces: `src/App.svelte`, `src/app.css`, `src/components/*`, `src/lib/*`
- Backend surfaces: `src-tauri/src/main.rs`, `src-tauri/src/shell_windows.rs`, `src-tauri/src/appbar.rs`, `src-tauri/src/stack_popup.rs`, `src-tauri/src/search_sources/*`, `src-tauri/src/process_manager.rs`, `src-tauri/src/system_tray.rs`, `src-tauri/src/task_windows/*`
- Validation config: `package.json`, `tsconfig.test.json`, `tests/*.mjs`, `src-tauri/Cargo.toml`
- Security/config: `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`

Separate audit agents were used for repository structure, frontend/visual design, and developer-workflow/product direction. Their recommendations were reconciled with a direct source review.

## Executive Summary

JasonShell already has a strong foundation: six persistent Tauri webviews, native AppBar reservation, pinned Explorer launchers, open-window grouping/previews, indexed search, Stack Browser file operations, and a process manager. The most urgent work is not adding more features. The next phase should first restore validation trust, fix a current TopBar pin recursion bug, decide what to do with the unintegrated system tray slice, and reduce structural drift.

The highest leverage roadmap is:

1. Stabilize existing behavior and tests.
2. Introduce shared visual/design tokens and accessibility hardening.
3. Split large frontend/Rust modules along feature boundaries.
4. Add an explicit IPC/event/surface contract layer.
5. Add durable settings/config before workspace features.
6. Build a command registry and command palette.
7. Add workspace profiles as the central developer concept.
8. Integrate terminal/editor/git/task workflows.
9. Add CLI automation and extension/provider support only after contracts stabilize.

## Master Spec Readability Improvements

`master_spec.md` is already unusually useful. It gives future agents concrete file paths, event names, command names, invariants, tests, persistence ownership, and known risks. I would keep it as the repository-specific source of truth. The following changes would make it easier for a new AI session to understand and act safely.

### 1. Add A One-Page Quick Start For New Agents

Add a short section near the top titled `New Agent Quick Start` containing:

- Product target: Windows developer shell replacement.
- First source of truth: `master_spec.md`.
- Do not trust stale docs over code/tests.
- Important current risks: test harness sync, system tray status, primary-monitor limitation, live Tauri smoke needs.
- Mandatory validation command map.
- Clear statement that `master_spec.md` is Markdown, not Python, despite occasional references to `master_spec.py` in chat.

### 2. Add A Documentation Authority Order

Several docs are historical or aspirational. Add a clear order:

1. `master_spec.md` for current architecture and invariants.
2. Current source code and tests for implemented behavior.
3. `README.md` for user setup, once refreshed.
4. `features.md` for aspirational feature inventory unless status-marked.
5. `wave_*.md`, `plan.md`, and `stack_browser.md` as historical implementation notes unless marked active.

### 3. Separate Current Behavior From Future Direction

`features.md` includes settings, workspace profiles, developer dashboard, automation, multi-monitor taskbar behavior, and Spotify controls, but most are not currently implemented. `master_spec.md` should explicitly distinguish:

- Implemented current behavior.
- Accepted near-term roadmap.
- Aspirational or Cairo-derived reference features.
- Parked/experimental slices.

### 4. Add A Surface Capability Matrix

Add a table for every webview:

- Label.
- Svelte component.
- Rust owner module.
- Allowed commands.
- Allowed outbound events.
- Whether it is render-only or stateful.
- Persistence ownership.
- Whether it needs hidden-window payload fallback.

This would prevent future agents from giving render-only surfaces too much authority.

### 5. Add A Contract Index For Events And Commands

The current event/command table is useful, but it should become a concise index where each row includes:

- String literal.
- Owning frontend wrapper.
- Owning Rust module.
- Payload type.
- Target label or global behavior.
- Staleness strategy.
- Tests that cover it.

### 6. Add Test Harness Caveats

The spec says `npm run test:search` validates many helpers, but `tsconfig.test.json` currently emits only a subset of helpers while tests import more from `dist-tests`. Add a warning until fixed:

- `tests/folderDrag.test.mjs` imports `dist-tests/folderDrag.js` but `src/lib/folderDrag.ts` is not emitted by `tsconfig.test.json`.
- `tests/topBarPins.test.mjs` imports `dist-tests/topBarPins.js` but `src/lib/topBarPins.ts` is not emitted.
- `tests/taskbarGroups.test.mjs` imports `dist-tests/taskbarGroups.js` but `src/lib/taskbarGroups.ts` is not emitted.
- `tests/taskbarTilePointer.test.mjs` imports `dist-tests/taskbarTilePointer.js` but `src/lib/taskbarTilePointer.ts` is not emitted.
- `tests/systemTray.test.mjs` exists but is not included in `package.json` and `src/lib/systemTray.ts` is not emitted.

### 7. Clarify System Tray Status

`system_tray.rs`, `systemTray.ts`, and `tests/systemTray.test.mjs` exist, but `main.rs` does not register `mod system_tray` or the commands. The spec should mark this as one of:

- Experimental parked work.
- Active feature slice awaiting integration.
- Dead code to remove.

Until this is clarified, future agents may assume it is implemented.

### 8. Mark Historical Docs Clearly

Add status banners to:

- `plan.md`: historical bottom-bar v1 plan; grouping/previews now exist.
- `wave_1.md` through `wave_5.md`: Stack Browser wave plans, mostly implemented.
- `stack_browser.md`: current summary but should defer to `master_spec.md` for invariants.

### 9. Document Durable Config Ownership Before Adding Features

Before settings/workspaces/tasks/automation, add a master-spec section for:

- Config file names.
- Versioning/migration rules.
- Corrupt backup behavior.
- Secret-handling rules.
- Rust vs frontend persistence ownership.
- Which data must never be persisted.

### 10. Add A Live Smoke Checklist

The spec correctly says live Tauri smoke is required for OS/WebView2 behavior. Add a checklist file or section covering:

- AppBar reservation/restoration.
- Explorer taskbar hide/restore.
- Multi-webview event delivery while hidden.
- Stack popup placement and drag/drop.
- Search panel placement and selection.
- Process manager focus-loss polling stop.
- DWM previews.
- Native menus.
- DPI/scaled display behavior.

## Visual System Coverage From `features.md`

The `features.md` Visual System section says the shell should have:

- A consistent dark visual language across all shell surfaces.
- Shared visual tokens.
- Calm and readable design rather than loud decoration.
- Status badges, cards, and labels that communicate state quickly.
- Support for everyday desktop use and data-rich developer workflows.
- Cohesion across navigation, configuration, workspace management, and task-oriented views.

Current coverage:

- Consistent dark visual language: partially covered. All visible surfaces use dark glass/rail styling, but colors and spacing are duplicated.
- Shared visual tokens: not covered. `src/app.css` defines font/reset only; colors, radii, shadows, spacing, and row heights are hardcoded across components.
- Calm/readable design: partially covered. The shell is visually calm, but many texts are very small and muted.
- Status badges/cards/labels: partially covered. Stack Browser metadata badges and process status exist, but status hierarchy is inconsistent.
- Data-rich developer workflows: partially covered functionally by Stack Browser, search, process manager, and taskbar grouping, but not visually optimized for large datasets or developer context.
- Cohesion across navigation/configuration/workspace/task views: currently incomplete because settings, dashboard, workspace profiles, and automation are not implemented.

Priority visual conclusion: build a shared design-token system in `src/app.css`, migrate surfaces to tokens, then harden accessibility and constrained-width layouts.

## Priority Roadmap

Priorities are grouped by urgency:

- P0: correctness, validation, and trust blockers.
- P1: architectural foundations and high-value usability polish.
- P2: developer workflow product features.
- P3: automation, extensibility, and advanced shell parity.

## P0: Correctness And Validation Trust

### 1. Fix TopBar Pin Hydration Recursion

Finding: `src/components/TopBar.svelte` currently has `loadStackPins()` call `applyStackPins(await listStackPins(), false)`, while `applyStackPins()` calls `await loadStackPins()` when no reveal path exists. That is a recursion-prone flow and contradicts `master_spec.md` line guidance that `applyStackPins()` must not reload after successful authoritative pin application.

Why this matters: pinned folders are the top-bar file navigation backbone. Any workspace profile or Stack Browser improvement will depend on stable pin state.

Affected files:

- `src/components/TopBar.svelte`
- `src/lib/topBarPins.ts`
- `tests/topBarPins.test.mjs`
- `tests/stackBrowserTopBarPinFlow.test.mjs`

Recommended fix:

- Make `loadStackPins()` fetch once and apply once.
- Make `applyStackPins(nextPins, allowDetectedAdd)` only assign state, clear pending reveal, optionally reveal, then update rail scroll buttons.
- Only call `loadStackPins()` from explicit recovery paths such as failed reorder.
- Add source/helper coverage that proves initial hydration does not refetch recursively.

Validation:

- `npm run check`
- Focused TopBar pin tests after fixing the test harness.
- Live smoke: startup with existing pins, pin from search, pin from Stack Browser, drag/drop pin, reorder, unpin.

### 2. Repair `tsconfig.test.json` And Node Test Source Coverage

Finding: `package.json` runs many Node tests that import files from `dist-tests`, but `tsconfig.test.json` emits only six source files. Existing `dist-tests` artifacts can make tests pass against stale compiled files.

Why this matters: current validation can produce false confidence. Refactors may leave tests exercising old JavaScript rather than current TypeScript.

Affected files:

- `tsconfig.test.json`
- `package.json`
- `tests/*.mjs`
- `src/lib/*.ts`

Recommended fix:

- Add every tested source helper to `tsconfig.test.json`.
- Include at least: `folderDrag.ts`, `taskbarGroups.ts`, `taskbarTilePointer.ts`, `topBarPins.ts`, `stackPopup.ts` if source-level tests require wrappers, `systemTray.ts` if tray remains active.
- Rename `test:search` to `test:node` or `test:frontend` because it now covers more than search.
- Add a simple convention: every `../dist-tests/X.js` import must map to `src/lib/X.ts` in `tsconfig.test.json`.
- Consider deleting `dist-tests` before test compilation in the script so stale files cannot satisfy imports.

Validation:

- `npx tsc -p tsconfig.test.json`
- `node --test tests/*.test.mjs`
- `npm run validate`

### 3. Resolve `taskbarGroups.test.mjs` Drift

Finding: `tests/taskbarGroups.test.mjs` imports `taskbarGroupCloseAllRequest`, but `src/lib/taskbarGroups.ts` does not export it.

Why this matters: this is either an unimplemented feature expectation or a stale test. Either way, validation intent is ambiguous.

Affected files:

- `tests/taskbarGroups.test.mjs`
- `src/lib/taskbarGroups.ts`
- Potentially `src/components/BottomBar.svelte`
- Potentially `src-tauri/src/taskbar_menu.rs`

Decision needed:

- If grouped close-all is desired, implement the helper, frontend action, and Rust menu/action support.
- If not desired now, remove or quarantine the test and record the product decision.

Validation:

- `npx tsc -p tsconfig.test.json`
- `node --test tests/taskbarGroups.test.mjs`

### 4. Decide The System Tray Slice: Integrate Or Park

Finding: `src/lib/systemTray.ts`, `src-tauri/src/system_tray.rs`, and `tests/systemTray.test.mjs` exist. However, `src-tauri/src/main.rs` does not declare `mod system_tray`, does not register `list_system_tray_icons`, and does not register `invoke_system_tray_icon`. `tests/systemTray.test.mjs` is not run by `package.json`. `Cargo.toml` also lacks the Windows debug feature needed by `ReadProcessMemory` if `system_tray.rs` is compiled.

Why this matters: future agents may assume tray support exists, while the current app cannot invoke it.

Affected files:

- `src/lib/systemTray.ts`
- `src-tauri/src/system_tray.rs`
- `src-tauri/src/main.rs`
- `src-tauri/Cargo.toml`
- `tests/systemTray.test.mjs`
- `package.json`
- `tsconfig.test.json`
- `master_spec.md`

Option A, integrate:

- Add `mod system_tray` to `main.rs`.
- Register `list_system_tray_icons` and `invoke_system_tray_icon`.
- Add needed Windows crate features.
- Add UI in the bottom bar or tray overflow popup.
- Add tests to validation.
- Add spec coverage and live smoke checklist.

Option B, park:

- Mark the files experimental.
- Exclude tests clearly.
- Do not imply system tray coverage in docs.

Validation if integrated:

- `npx tsc -p tsconfig.test.json`
- `node --test tests/systemTray.test.mjs`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Live Windows smoke against Explorer notification area.

### 5. Add CI Before Major Refactors

Finding: no CI workflow is present under `.github/workflows`.

Why this matters: JasonShell crosses TypeScript, Svelte, Tauri, Rust, Win32 APIs, and generated test artifacts. CI is the only reliable way to prevent silent drift.

Affected files:

- `.github/workflows/ci.yml`
- `package.json`
- `README.md`
- `master_spec.md`

Recommended CI jobs:

- Windows primary job: `npm ci`, `npm run check`, `npm run build`, Node tests, Rust tests, Cargo check.
- Optional Linux compile job for non-Windows fallback if the project wants that path preserved.

Validation:

- CI green on a clean checkout.
- Local `npm run validate` remains the supported full validation bundle.

## P1: Architecture And Maintainability Foundations

### 6. Split Large Svelte Surfaces Into Feature Components

Finding: several components are doing too much:

- `src/components/StackPopupSurface.svelte` is a 1,400+ line component combining folder loading, selection, paging, context menus, drag/drop, keyboard navigation, inline editing, formatting, and CSS.
- `src/components/TopBar.svelte` owns search, pin state, drag/drop, pin context menus, stack popup anchoring, and search catalog refresh.
- `src/components/BottomBar.svelte` owns launcher loading, task-window polling, drag reorder, preview timers, menus, and process manager anchoring.

Why this matters: future features will become high-risk if all behavior continues accumulating in single Svelte files.

Recommended structure:

```text
src/features/top-bar/
src/features/bottom-bar/
src/features/search/
src/features/stack-browser/
src/features/process-manager/
src/ipc/
src/shared/
```

Stack Browser extraction targets:

- Toolbar component.
- Breadcrumb component.
- Details grid component.
- Context menu component.
- Inline editor component.
- Drag/drop controller.
- Keyboard controller.
- Folder loading controller.
- Keep reducer logic pure in `stackPopupState.ts` or move it into `features/stack-browser/state.ts`.

Validation:

- Extract one piece at a time.
- `npm run check`
- Focused Node tests.
- Live smoke for Stack Browser menus, keyboard, drag/drop, and retained rows.

### 7. Split `stack_popup.rs` By Responsibility

Finding: `src-tauri/src/stack_popup.rs` includes Tauri commands, window positioning, pin persistence, path normalization, folder enumeration, metadata, file operations, clipboard interop, delete/recycle behavior, and tests.

Why this matters: Stack Browser is central to the Explorer replacement goal. It will keep growing unless the Rust module is split.

Recommended module layout:

```text
src-tauri/src/stack_popup/mod.rs
src-tauri/src/stack_popup/commands.rs
src-tauri/src/stack_popup/window.rs
src-tauri/src/stack_popup/pins.rs
src-tauri/src/stack_popup/path_normalization.rs
src-tauri/src/stack_popup/listing.rs
src-tauri/src/stack_popup/metadata.rs
src-tauri/src/stack_popup/file_ops.rs
src-tauri/src/stack_popup/clipboard.rs
src-tauri/src/stack_popup/delete.rs
src-tauri/src/stack_popup/tests.rs
```

Validation:

- Preserve public command names and serde shapes.
- `cargo test --manifest-path src-tauri/Cargo.toml stack_popup`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run check` if wrapper types change.

### 8. Create A Single IPC/Event/Surface Contract Layer

Finding: surface labels, command names, and event names are duplicated across TS wrappers, Svelte components, Rust modules, `main.rs`, and `master_spec.md`.

Why this matters: duplication is manageable now, but future settings/workspaces/tasks/automation will multiply command/event contracts.

Recommended frontend files:

```text
src/ipc/surfaces.ts
src/ipc/events.ts
src/ipc/commands.ts
src/ipc/types.ts
```

Recommended backend file:

```text
src-tauri/src/contracts.rs
```

Minimal first step:

- Move string constants into contract modules.
- Make wrappers import constants instead of inline command/event strings.
- Add tests for target shapes and wrapper command names where source-level checks are useful.

Longer-term:

- Generate TypeScript contracts from Rust or from a small schema.
- Add a command/event manifest consumed by docs and tests.

Validation:

- `npm run check`
- Node tests for wrappers/targets.
- `cargo check --manifest-path src-tauri/Cargo.toml`

### 9. Harden Tauri Capabilities And CSP

Finding: `src-tauri/tauri.conf.json` has `"csp": null`, and `src-tauri/capabilities/default.json` lists only `top-bar` and `bottom-bar`, while the app creates six windows.

Why this matters: a shell replacement has powerful commands. Render-only surfaces should not receive unnecessary authority.

Affected files:

- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`
- TS wrappers and surface components if permissions are narrowed.
- `master_spec.md`

Recommended policy:

- Add all six intended windows to capabilities or split by surface.
- Give `task-preview` minimal authority.
- Keep `search-panel` intent-only where possible.
- Give `stack-popup` file-operation authority.
- Give `process-manager` process commands only.
- Document the CSP decision for development vs production.

Validation:

- `npm run tauri dev`
- `npm run build`
- Live smoke every surface after tightening permissions.

### 10. Introduce A Diagnostics Layer

Finding: frontend uses many `console.error` calls, and backend uses `eprintln!`. A daily-use shell needs better self-diagnostics.

Recommended modules:

- `src/lib/diagnostics.ts`
- `src-tauri/src/diagnostics.rs`

Capabilities:

- Ring buffer of recent events.
- Severity, surface, module, timestamp, and redacted details.
- Diagnostic command/search result.
- Optional diagnostics popup.
- Export recent logs for bug reports without secrets.

Validation:

- Unit tests for redaction/formatting.
- Live smoke: failed search, failed file operation, failed process kill, AppBar cleanup error.

### 11. Update README And Docs To Reflect Current Reality

Finding: `README.md` says the current milestone is two polished shell surfaces. Current code has six surfaces: `top-bar`, `bottom-bar`, `task-preview`, `search-panel`, `stack-popup`, and `process-manager`.

Affected files:

- `README.md`
- `features.md`
- `plan.md`
- `wave_*.md`
- `stack_browser.md`
- `master_spec.md`

Recommended changes:

- Update README to summarize all six surfaces.
- Mark `features.md` as aspirational/current-mixed or add status labels.
- Mark `plan.md` and wave docs historical/completed where applicable.
- Add `docs/smoke-test-windows.md`.

Validation:

- Docs readback.
- Search for stale two-surface/five-surface wording.

## P1: Frontend, Visual Polish, And Accessibility

### 12. Introduce Shared Visual Tokens

Finding: visual constants are hardcoded across CSS and Svelte style blocks. `src/app.css` has no design token layer beyond font/reset.

Why this matters: this directly misses the `features.md` Visual System promise of shared visual tokens.

Affected files:

- `src/app.css`
- `src/components/TopBar.css`
- `src/components/BottomBar.css`
- `src/components/SearchPanelSurface.svelte`
- `src/components/TaskPreviewSurface.svelte`
- `src/components/StackPopupSurface.svelte`
- `src/components/ProcessManagerSurface.css`

Recommended tokens:

```css
:root {
  --shell-bg-0: #06080d;
  --shell-bg-1: #0d1018;
  --shell-surface: rgba(23, 28, 39, 0.98);
  --shell-surface-raised: rgba(31, 38, 55, 0.98);
  --shell-border-subtle: rgba(255, 255, 255, 0.08);
  --shell-border-strong: rgba(144, 181, 255, 0.34);
  --shell-text: #f0f4ff;
  --shell-text-muted: rgba(218, 226, 248, 0.68);
  --shell-text-dim: rgba(218, 226, 248, 0.46);
  --shell-accent: #7ca0ff;
  --shell-danger: #ff7777;
  --shell-success: #25d366;
  --shell-radius-sm: 0.24rem;
  --shell-radius-md: 0.45rem;
  --shell-shadow-popup: 0 22px 46px rgba(0, 0, 0, 0.44);
}
```

Validation:

- `npm run check`
- `npm run build`
- Visual screenshot pass for all surfaces.

### 13. Improve Accessibility Semantics

Finding examples:

- `ProcessManagerSurface.svelte` uses `role="row"` without a complete table/grid structure or `aria-sort`.
- `SearchPanelSurface.svelte` uses `role="option"` rows containing an interactive `Pin` button.
- `StackPopupSurface.svelte` uses an ARIA grid with row buttons and nested gridcells; context menus do not focus/roving-key navigate.
- `TopBar.svelte` has clickable rail scroll buttons with `aria-hidden="true"`.

Recommended fixes:

- Use a semantic table or complete ARIA grid for process manager.
- Add `aria-sort` to sortable process headers.
- Add accessible labels to Kill buttons such as `Kill {name} PID {pid}`.
- Restructure search rows so secondary actions are not nested inside `role="option"`.
- Add `aria-activedescendant` where selection is owned by a container.
- Make context menus focus themselves and support arrow keys.
- Remove `aria-hidden` from interactive scroll buttons or make them non-interactive decorations.

Validation:

- `npm run check`
- Keyboard-only pass for every surface.
- Narrator/NVDA smoke on Windows.
- Axe-like checks where feasible.

### 14. Add Responsive And Constrained-Width Layouts

Finding: fixed windows still face DPI scaling, low-resolution displays, and future layout changes. Current grids have fixed columns and some strips hide overflow silently.

Priority fixes:

- Process manager: hide low-priority columns on narrow widths and keep Name, PID, CPU, Memory, Action.
- Stack Browser: collapse Type/Size/Modified under narrow widths and move secondary actions into overflow.
- Search panel: allow result kind/actions to stack or compress.
- Bottom bar: add overflow indicator for hidden tasks and icon-only fallback.
- Top bar: let search collapse when pin rail needs space.

Validation:

- Test 320, 420, 720, 980, and 1440 logical widths.
- Use long process names, long paths, many pins, many open windows, and large folder names.

### 15. Add Reduced-Motion And Contrast Preferences

Finding: busy animations and smooth scrolling do not respect `prefers-reduced-motion`.

Recommended changes:

- Global `@media (prefers-reduced-motion: reduce)` in `src/app.css`.
- Disable or shorten busy indicator animation.
- Disable smooth scroll for pin reveal and rail scroll.
- Add high-contrast token variants later.

Validation:

- Devtools reduced-motion emulation.
- Manual visual pass.

### 16. Improve Status Hierarchy

Finding: status messages exist but are visually inconsistent and often low emphasis.

Recommended shared patterns:

- Info/warning/error badges.
- Inline skeleton/loading rows.
- Persistent retryable error regions.
- Search source chips: Apps, Windows, Files, Commands, Indexing.
- Stack Browser count: total items, selected count, retained-loading state.
- Process manager first-sample state: `sampling CPU...`.

Validation:

- Simulate failed search, empty folder, large folder load, failed paste, failed kill, unavailable launcher.

### 17. Extract Inline Styles Into CSS Files

Finding: Search, Stack Browser, and Task Preview contain large inline style blocks. Stack Browser combines large logic and large CSS in one file.

Recommended files:

- `src/components/SearchPanelSurface.css`
- `src/components/StackPopupSurface.css`
- `src/components/TaskPreviewSurface.css`

Validation:

- `npm run check`
- `npm run build`
- Visual diff/screenshot pass.

## P1: Shell UX Improvements

### 18. Improve Bottom Bar Overflow And Group Behavior

Finding: `.task-strip` uses `overflow: hidden`; tasks can disappear without an affordance. Group count and active/minimized states are subtle.

Recommended features:

- Overflow chevron/menu for hidden task groups.
- Clear active and minimized indicators.
- Group flyout for multi-window groups.
- Keyboard roving focus across launcher/task groups.
- Respect reduced motion for busy animation.

Affected files:

- `src/components/BottomBar.svelte`
- `src/components/BottomBar.css`
- `src/lib/taskbarGroups.ts`
- `src/lib/taskbarUi.ts`

Validation:

- Many open windows.
- Multiple windows in one group.
- Keyboard-only switching.
- Reduced-motion pass.

### 19. Make Process Manager A Serious Developer Tool

Current process manager is useful but generic.

Recommended features:

- Filter input by name, path, PID.
- CPU and memory mini bars.
- Column visibility by width.
- Protected/unknown metadata badges.
- Safer Kill UX: confirmation, double-click, or secondary destructive menu.
- Process tree view using parent PID.
- Command line and executable path copy.
- Open executable folder.
- Later: ports/listening sockets and task ownership.

Affected files:

- `src/components/ProcessManagerSurface.svelte`
- `src/components/ProcessManagerSurface.css`
- `src/lib/processManagerState.ts`
- `src-tauri/src/process_manager.rs`

Validation:

- TS sort/filter tests.
- Rust guardrail tests.
- Live smoke with Node/Vite/Cargo processes and protected/elevated processes.

### 20. Improve Stack Browser Scalability And Polish

Recommended features:

- DOM virtualization for large folders.
- Filesystem watcher for the current folder only.
- Custom delete confirmation dialog instead of `window.confirm`.
- Keyboard-accessible context menu with roving focus.
- Toolbar hierarchy: navigation cluster, file actions, overflow menu.
- Selected count and item count.
- More prominent drag-over targets.
- Better breadcrumb overflow treatment.

Affected files:

- `src/components/StackPopupSurface.svelte`
- `src/lib/stackPopupState.ts`
- `src/lib/contextMenuPosition.ts`
- `src-tauri/src/stack_popup.rs`

Validation:

- Folders with 5,000 to 10,000 entries.
- Deep paths.
- Long names.
- Full keyboard file-operation flow.
- External file changes if watcher is added.

### 21. Make Search A Premium Command Palette

Current search is a simple list. It should become the primary keyboard workflow.

Recommended features:

- Result grouping by Apps, Windows, Files, Folders, Commands, Workspaces, Tasks.
- Matched text highlighting.
- Recent/frequent section.
- Secondary action menu per result.
- Keyboard shortcut hints.
- Distinct default actions: open file, open Stack Browser, reveal, pin, copy path.
- Better stale/indexing status.

Affected files:

- `src/components/SearchPanelSurface.svelte`
- `src/components/TopBar.svelte`
- `src/lib/searchCatalog.ts`
- `src/lib/searchRanking.ts`
- `src/lib/searchPanel.ts`
- `src/lib/searchPanelState.ts`

Validation:

- Search apps/windows/files/folders/commands.
- Keyboard selection under changing result sets.
- Pin folder by mouse and keyboard.
- Narrow panel pass.

### 22. Give Top Bar A Stronger Shell/Menu Identity

Finding: `features.md` describes Menu Bar, Cairo Menu, Places Menu, and Programs Menu. Current Top Bar is mostly pinned folders, clock/date, and search.

Recommended additions:

- JasonShell/Home menu.
- Places menu.
- Programs menu or apps flyout.
- Workspace/project status pill.
- Indexing/status indicators.
- Optional time display preference.

Affected files:

- `src/components/TopBar.svelte`
- `src/components/TopBar.css`
- Future settings/workspace modules.

Validation:

- Feature coverage pass against `features.md` Menu Bar, Cairo Menu, Places, Programs.
- Discoverability/user flow pass.

## P2: Durable Config, Settings, And Workspace Foundation

### 23. Add Versioned Settings Architecture Before Settings UI

Finding: current persistence is limited to stack pins, search index cache, and search usage in localStorage.

Why this matters: settings, workspaces, tasks, command aliases, tool integrations, and automation need a durable, versioned data model.

Recommended backend module:

- `src-tauri/src/settings.rs`

Recommended frontend wrapper:

- `src/lib/settings.ts`

Recommended files:

- `settings-v1.json`
- `workspaces-v1.json`
- `task-history-v1.json`

Initial settings domains:

- Search behavior.
- Stack Browser behavior.
- Top/bottom bar behavior.
- Editor and terminal defaults.
- Workspace profiles.
- Task history retention.
- Visual density and reduced-motion preference overrides.

Validation:

- Rust load/save/migrate tests.
- Corrupt-file backup tests.
- TS wrapper tests.
- Restart persistence live smoke.

### 24. Add Workspace Profiles As The Central Developer Concept

Why this matters: Explorer alternatives browse folders. A developer shell should understand projects.

Recommended modules:

- `src-tauri/src/workspaces.rs`
- `src/lib/workspaces.ts`
- Later: `src/features/developer-dashboard/`

Example workspace shape:

```json
{
  "id": "jasonshell",
  "name": "JasonShell",
  "rootPath": "C:\\dev\\jasonShell",
  "aliases": ["shell", "jshell"],
  "editorCommand": "code .",
  "terminalCommand": "wt -d .",
  "pins": ["src", "src-tauri", "tests"],
  "tasks": [
    { "id": "validate", "label": "Validate", "command": "npm run validate" },
    { "id": "test", "label": "Tests", "command": "npm run test:search" }
  ]
}
```

Core commands:

- `list_workspaces`
- `create_workspace`
- `update_workspace`
- `delete_workspace`
- `activate_workspace`
- `get_active_workspace`
- `open_workspace_in_editor`
- `open_workspace_in_terminal`
- `run_workspace_task`

Validation:

- Rust schema/path validation tests.
- TS search-result tests.
- Live smoke: activate workspace, update top-bar status, bias search, show workspace pins.

### 25. Make Workspace Activation Drive Shell Layout

Recommended activation behavior:

- Pin workspace folders.
- Bias search results to active workspace.
- Show active workspace in top bar.
- Offer project tasks in search.
- Open editor/terminal through configured tools.
- Optionally restore app/window groups later.

Important constraint: do not store secrets in workspace environment config. Use references or explicit per-command environment injection.

Validation:

- Pure tests for activation plan generation.
- Rust tests for rejecting unsafe startup commands unless confirmed.
- Live smoke with a sample workspace.

### 26. Add Terminal And Editor Integration

Recommended commands:

- `open_terminal_at_path`
- `open_editor_at_path`
- `open_file_in_editor`
- `open_workspace_tool`

Recommended settings:

```json
{
  "tools": {
    "terminal": { "kind": "WindowsTerminal", "command": "wt -d {path}" },
    "editor": { "kind": "VSCode", "command": "code {path}" }
  }
}
```

Security rules:

- Template placeholders must be strictly expanded and quoted.
- Do not execute arbitrary UI-provided command strings without config ownership and confirmation.

Validation:

- Rust template expansion/quoting tests.
- Security tests for path injection.
- Live smoke with Windows Terminal and VS Code.

### 27. Add Git-Aware Workspace Status

Recommended features:

- Branch name.
- Dirty state.
- Ahead/behind counts.
- Merge/rebase in-progress state.
- Quick commands: status, fetch, pull, checkout branch, open changes.

Recommended implementation:

- Initial version can shell out to `git` with strict cwd validation.
- Later version can use a Rust git crate if needed.

UI placement:

- Top-bar workspace pill.
- Search results.
- Developer dashboard.

Validation:

- Rust tests with temp git repos when `git` is available.
- Output parsing tests.
- Live smoke on clean/dirty/ahead/behind repos.

### 28. Add Workspace Task Runner And Task History

Recommended module:

- `src-tauri/src/tasks.rs`
- `src/lib/taskHistory.ts`

Features:

- Run workspace tasks with cwd/env.
- Stream output to a task surface or history panel.
- Record status, exit code, start/end time.
- Rerun last task from search.
- Cancel running tasks.
- Associate JasonShell-launched processes with process manager.

Validation:

- Rust spawn/cancel/history tests.
- TS task-result rendering and ranking tests.
- Live smoke with `npm run test:search` and `npm run validate`.

## P2: Developer-Centric File And Process Features

### 29. Add Developer Stack Browser Actions

Recommended context actions:

- New file.
- Duplicate.
- Copy absolute path.
- Copy relative path from active workspace.
- Copy import path.
- Open terminal here.
- Open in editor.
- Search within folder.
- Git status for file/folder.
- Create from template.
- Compress/extract.

Affected files:

- `src/components/StackPopupSurface.svelte`
- `src/lib/stackPopup.ts`
- `src-tauri/src/stack_popup.rs`
- Future settings/workspace modules.

Validation:

- Rust file-operation tests.
- TS menu visibility tests.
- Live clipboard/editor/terminal smoke.

### 30. Add Saved Searches And Search Providers

Recommended providers:

- Workspace files by bounded glob.
- Recent files.
- Git changed files.
- Task history.
- Commands.
- Settings.
- Processes.

Example saved search:

```json
{
  "id": "changed-files",
  "label": "Changed files",
  "provider": "gitChanged",
  "workspaceScoped": true
}
```

Validation:

- Provider unit tests with temp workspaces.
- Performance tests for bounded traversal.
- Live monorepo smoke.

### 31. Add Developer-Aware Process Details

Recommended process manager enhancements:

- Command line.
- Parent/child tree.
- Listening ports.
- Workspace/task ownership for JasonShell-launched processes.
- Filter by developer processes.
- Kill tree with explicit guardrails.
- Copy command line.
- Open executable folder.

Validation:

- Rust kill-tree guardrail tests.
- TS filter/group tests.
- Live smoke with Node, Rust, Python, terminal, browser, and protected processes.

## P2: Multi-Monitor Architecture

### 32. Design Multi-Monitor Before Implementing It

Finding: current app is explicitly primary-monitor-only. `features.md` mentions broader Cairo-style multi-monitor behavior.

Design decisions needed:

- One top/bottom bar per monitor vs primary-only shell plus secondary task strips.
- AppBar reservation per monitor.
- Window ownership by monitor.
- Popup anchoring across monitors.
- DPI scaling rules.
- Explorer taskbar interaction on secondary monitors.
- Per-monitor taskbar grouping/filtering.

Affected files:

- `src-tauri/src/appbar.rs`
- `src-tauri/src/layout.rs`
- `src-tauri/src/shell_windows.rs`
- `src-tauri/src/task_windows/windows.rs`
- `src-tauri/src/search_panel.rs`
- `src-tauri/src/stack_popup.rs`
- `src-tauri/src/process_manager.rs`
- `src/components/BottomBar.svelte`

Validation:

- Rust monitor-mapping tests.
- Live multi-monitor smoke with mixed DPI.

## P3: Automation, CLI, And Extensibility

### 33. Add `jasonshell` CLI Automation

Example commands:

```powershell
jasonshell workspace activate jasonshell
jasonshell task run validate
jasonshell open C:\dev\jasonShell
jasonshell search "process manager"
```

IPC options:

- Windows named pipe.
- Single-instance command forwarding.
- Deep link protocol.
- Loopback HTTP only if explicitly secured and disabled by default.

Security rules:

- Automation should be opt-in.
- Destructive commands require confirmation unless explicitly configured.
- Do not allow arbitrary unauthenticated local command execution.

Validation:

- Rust command parser and permission tests.
- Integration smoke against a running app.

### 34. Add Extension/Provider Model After Contracts Stabilize

Do not add arbitrary plugin execution early. Start with config-driven providers.

Potential provider types:

- Search provider.
- Command provider.
- Workspace detector.
- Status badge provider.
- Task provider.
- File template provider.

Validation:

- Provider contract tests.
- Security review.
- Performance budget tests.

### 35. Add Settings Panel And Developer Dashboard

These are described in `features.md` but should come after settings/workspaces/tasks exist.

Settings panel should cover:

- Top bar.
- Bottom bar/taskbar.
- Search/indexing.
- Stack Browser.
- Process manager.
- Workspaces.
- Tools.
- Automation.
- Visual density/accessibility.

Developer dashboard should show:

- Active workspace.
- Workspace list.
- Git state.
- Recent task runs.
- Running tasks/processes.
- Quick actions.

Validation:

- Settings persistence tests.
- Keyboard/a11y pass.
- Live workflow smoke.

## Suggested Implementation Sequence

1. Fix `tsconfig.test.json` and stale test artifact risk.
2. Fix `TopBar.svelte` pin recursion.
3. Decide `system_tray` integrate vs park.
4. Add CI.
5. Update README/docs status and master-spec caveats.
6. Add shared CSS tokens and migrate surfaces.
7. Fix process/search/stack accessibility semantics.
8. Add constrained-width layouts and reduced-motion support.
9. Extract Stack Browser frontend pieces.
10. Split `stack_popup.rs`.
11. Add IPC/event/surface contract modules.
12. Harden Tauri capabilities and CSP.
13. Add diagnostics ring buffer.
14. Add settings/config foundation.
15. Build command registry and richer command palette.
16. Add workspace profiles.
17. Add terminal/editor integration.
18. Add git-aware workspace status.
19. Add task runner/history.
20. Add developer Stack Browser actions.
21. Add developer-aware process manager details.
22. Design and implement multi-monitor support.
23. Add CLI automation.
24. Add extension/provider model.
25. Add settings panel and developer dashboard.

## Highest Priority Immediate Backlog

These are the items I would put at the top of the next implementation queue:

1. Repair test compilation/source coverage so `npm run test:search` cannot pass using stale `dist-tests` artifacts.
2. Fix the TopBar pin reload recursion.
3. Decide the status of `system_tray` and align code/tests/docs.
4. Add shared visual tokens in `src/app.css` and migrate surfaces.
5. Fix accessibility semantics in Process Manager, Search Panel, Stack Browser, and TopBar scroll controls.
6. Update README and mark historical/aspirational docs clearly.
7. Add Windows CI.
8. Extract Stack Browser into smaller components/controllers.
9. Split `stack_popup.rs` into submodules.
10. Add settings/config foundation before workspace/task features.

## Validation Notes For This Roadmap

This roadmap is documentation-only. No implementation changes were made. Full build/test validation is not required for this document itself, but the roadmap intentionally identifies validation commands for each future work item.

Before any code implementation starts, fix the test harness first. Otherwise, future work may appear green while testing stale JavaScript emitted in `dist-tests`.
