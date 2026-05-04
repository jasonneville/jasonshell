# Top Bar And Audio Polish (P5)

## Goal

Polish remaining top-bar surfaces: quick commands should match the active theme, audio input/output should auto-refresh, and top-bar folder pins should reorder by drag.

## Source Items

- `updates.md` item 2: quick commands view should match theme and GUI.
- `updates.md` item 6: sound settings should auto-update input/output instead of requiring Refresh.
- `updates.md` item 8: top-bar folders should drag left/right like bottom-bar open processes.

## Priority Rationale

P5. These improve feel and consistency but are less blocking than popup stability, launch reliability, Stack Browser correctness, and search control.

## Implementation Map

- Command panel: `src/components/CommandPanelSurface.svelte`, `src/components/CommandPanelSurface.css`, `src/lib/commandPanel.ts`.
- Theme tokens/preferences: `src/lib/themes.ts`, `src/lib/settings.ts`, shared CSS/global styles.
- Audio panel/sound settings: audio surface component found from `src/App.svelte`, `src/lib/audio.ts`, Rust audio module under `src-tauri/src/`.
- Top-bar pins: `src/components/TopBar.svelte`, `src/lib/topBarPins.ts`, `src/lib/folderDrag.ts`, `src/lib/settings.ts`.
- Bottom-bar reorder reference: `src/components/BottomBar.svelte`, `src/lib/taskbarUi.ts`, current task tile reorder helpers.

## Phase 1: Quick Commands Theme RED Tests

### Work

- Compare command panel CSS with settings/tray/audio/top-bar CSS variables.
- Define allowed styling source: shared CSS custom properties/theme classes, not hard-coded off-theme colors.
- Define component behavior that must remain unchanged: run, edit, delete, validation, close-on-success.

### Tests

- Add `tests/commandPanelTheme.test.mjs` source/style test asserting `CommandPanelSurface.css` uses shared variables such as theme/background/text/accent tokens and does not introduce raw mismatched hex colors for primary surfaces.
- Add source test asserting command panel buttons keep real `button` semantics and existing action labels.
- Add snapshot-like textual test for required themed class names only if repo already uses such tests; avoid pixel assertions.

### Acceptance Criteria

- RED tests fail for current mismatched styling.
- Tests do not require browser screenshot infra.

## Phase 2: Quick Commands Theme GREEN Implementation

### Work

- Replace hard-coded command panel colors with shared variables from existing theme system.
- Align borders, panel radius, shadows, input backgrounds, hover/focus states, and typography with neighboring popup panels.
- Keep markup minimal; do not rewrite command editor state.

### Tests

- Run focused command panel theme tests.
- Run `npm run test:node`.

### Acceptance Criteria

- Command panel visually follows active theme.
- Existing command CRUD/run behavior unchanged.

## Phase 3: Audio Auto-Refresh RED Tests

### Work

- Find current audio device refresh command and UI refresh button path.
- Define event payload: `{ reason: 'device-added' | 'device-removed' | 'default-changed' | 'session-changed' }` or use existing audio payload shape.
- Define frontend behavior: on event, refresh input/output lists and sessions without resetting user-selected control mid-interaction.
- Define fallback if native event notification is hard: bounded poll only while audio panel is open.

### Tests

- Add Rust unit test for audio event payload mapping from native/default-device changes if backend has testable abstraction.
- Add Node state/source test proving audio panel subscribes to audio refresh event on mount and unsubscribes on destroy.
- Add Node test with mocked `listAudioDevices`/existing wrapper: input/output list updates when event fires.
- Add test that manual Refresh button remains available if currently present.

### Acceptance Criteria

- RED tests fail because audio refresh is manual-only.
- Tests cover output default changed, input default changed, removed device, event burst/debounce.

## Phase 4: Audio Auto-Refresh GREEN Implementation

### Work

- Implement native audio notification hook in existing Rust audio module if practical; otherwise implement poll timer only while audio panel is visible/open.
- Emit typed event through `src/ipc/events.ts` if frontend event constants are centralized.
- In audio panel, subscribe on mount/open; call existing refresh wrapper; debounce bursts (e.g. 100-250ms) to avoid UI thrash.
- Preserve active slider/select editing: do not overwrite in-progress value unless device disappeared.

### Tests

- Run focused audio tests.
- Run `npm run cargo:test` if Rust audio changed.
- Run `npm run test:node`.

### Acceptance Criteria

- Input/output lists update without pressing Refresh.
- Manual Refresh still works.
- Rapid native events do not freeze or flicker panel.

## Phase 5: Top-Bar Folder Reorder RED Tests

### Work

- Read top-bar pinned folder rendering and persistence helper.
- Read bottom-bar reorder implementation and extract only reusable calculation if minimal.
- Define top-bar pin reorder contract: pointer drag threshold, left/right reorder, persistence, click suppression after drag, context menu unaffected.
- Define pure helper: `reorderPinnedFolders(pins, fromPathOrIndex, toIndex)` or use existing id/path shape.

### Tests

- Add `tests/topBarFolderReorder.test.mjs` pure test for reorder helper: first-to-last, last-to-first, middle move, no-op same index, missing source.
- Add source test asserting `TopBar.svelte` pinned folder buttons have pointer drag handlers or use shared drag action.
- Add source/state test proving reorder calls existing pin persistence/update wrapper.
- Add regression tests proving click opens Stack Browser and right-click opens folder menu when drag threshold is not crossed.

### Acceptance Criteria

- RED tests fail because top-bar pin reorder is not implemented.
- Tests cover duplicate folder names with different paths.

## Phase 6: Top-Bar Folder Reorder GREEN Implementation

### Work

- Add pure reorder helper in `src/lib/topBarPins.ts` or nearby existing pin module.
- Add drag state to `TopBar.svelte`: source path/index, over index, pointer start position, `didDrag` click suppression.
- Use same threshold semantics as bottom-bar if available.
- Persist reordered pins through existing settings/pin update path; do not create a second source of truth.
- Add visual drop indicator only if existing design has one; keep minimal.

### Tests

- Run focused top-bar reorder tests.
- Run existing top-bar/pin tests.
- Run `npm run test:node`.

### Acceptance Criteria

- Top-bar folders drag left/right to reorder.
- Reorder persists after reload.
- Normal click/context menu behavior unchanged.

## Phase 7: Refactor, Spec, Validation

### Work

- Share reorder math only if both top and bottom bars can use same pure helper without coupling UI state.
- Update `master_spec.md` command-panel, audio-panel, top-bar pins, and Change Ledger.
- Add smoke steps: theme check, audio hotplug/default switch, folder reorder persistence.

### Tests

- Run `npm run test:node`.
- Run `npm run cargo:test` if audio backend changed.
- Run `npm run validate`.

### Acceptance Criteria

- Full validation passes.
- Manual smoke confirms command-panel theme, audio auto-refresh, and top-bar folder drag reorder.
