# Persistent Surface Lifecycle Cleanup P4

Status: Complete, validated 2026-05-06
Source: `findings.md`, Suggested Fix Order item 4  
Priority: P4, run after `windows_key_chord_preservation_p3.md`

## Goal

Stop hidden persistent webviews from doing unnecessary work and prevent async Tauri listener leaks when components destroy before `listen(...).then(...)` resolves.

## Issue

Findings show:

- `AudioPanelSurface.svelte` starts polling on mount even though hidden persistent webviews mount at startup.
- Audio panel does not listen for its own close event, so polling can continue after native focus-loss close.
- Multiple components use `listen(...).then((unlisten) => unlisteners.push(unlisten))` without a disposed guard.
- `StackPopupSurface.svelte` already has the safer pattern to copy.

Context7 Svelte docs say `onMount` cleanup must be returned synchronously; async setup needs explicit destroy/disposed handling.

## Phase 1: RED Lifecycle Tests

Status: Complete, 2026-05-06. RED confirmed with `node --test tests\persistentSurfaceLifecycle.test.mjs tests\audioAutoRefresh.test.mjs` failing lifecycle assertions before implementation; `npx tsc -p tsconfig.test.json` passed.

Acceptance criteria:

- Tests fail because audio polling starts on mount.
- Tests fail because audio close does not stop polling.
- Tests fail for every persistent surface that registers async listeners without a disposed guard.

Tests to write:

- Node/source test requiring `AudioPanelSurface.svelte` initial visibility false and no `startAudioRefreshPolling()` from mount.
- Node/source test requiring `AudioPanelSurface.svelte` listens for `AUDIO_PANEL_CLOSED_EVENT` and calls `stopAudioRefreshPolling()`.
- Source contract scan for async listener registrations in:
  - `src/components/TopBar.svelte`
  - `src/components/BottomBar.svelte`
  - `src/components/TaskPreviewSurface.svelte`
  - `src/components/SearchPanelSurface.svelte`
  - `src/components/ProcessManagerSurface.svelte`
- The scan should accept the existing `StackPopupSurface.svelte` disposed guard as a reference pattern.

Implementation tasks:

- No production changes in this phase.
- Keep source contracts narrow: they should require cleanup behavior, not exact formatting.


Validation gate:

- Focused Node lifecycle tests.
- `npx tsc -p tsconfig.test.json`

## Phase 2: Audio Hidden Polling Fix

Status: Complete, 2026-05-06. `AudioPanelSurface.svelte` stays hidden-idle on startup, refreshes immediately on `audio-panel:open`, starts fallback polling after that refresh request, and stops polling on close and destroy.

Acceptance criteria:

- Audio panel is idle while hidden at startup.
- Audio state refresh starts on `audio-panel:open`.
- Polling stops on close and destroy.
- Open still refreshes immediately so the first visible state is not stale.

Tests to write:

- Behavior/source test for open -> refresh -> start polling.
- Behavior/source test for close -> stop polling.
- Regression for cleanup clearing interval id.

Implementation tasks:

- Initialize `audioPanelVisible = false`.
- Remove mount-time polling start.
- Keep one immediate `refreshAudioState()` on explicit open.
- Wire close listener after P2 event target work is available.

Validation gate:

- `node --test tests/audio*.test.mjs`
- `npm run check`

## Phase 3: Async Listener Disposed Guards

Status: Complete, 2026-05-06. `AudioPanelSurface.svelte`, `TopBar.svelte`, `BottomBar.svelte`, `TaskPreviewSurface.svelte`, `SearchPanelSurface.svelte`, and `ProcessManagerSurface.svelte` now guard async listener registration with a disposed flag so late-resolving unlisteners are called immediately.

Acceptance criteria:

- Every long-lived surface using Tauri `listen()` handles promise resolution after destroy.
- Cleanup calls all resolved unlisteners exactly once.
- If `listen()` resolves after destroy, the returned unlisten function is called immediately.
- No duplicate event reactions after HMR/reload.

Tests to write:

- Source contract requiring a `disposed` or equivalent guard around async listener registration.
- At least one helper/unit test if listener registration is abstracted.

Implementation tasks:

- Copy or extract the StackPopup disposed-guard pattern.
- Apply to TopBar, BottomBar, TaskPreviewSurface, SearchPanelSurface, and ProcessManagerSurface.
- Preserve existing event names and handlers.


Validation gate:

- Focused lifecycle/source tests.
- `npm run check`
- `npm run test:node` when feasible.

## Phase 4: QA And Smoke

Status: Complete, 2026-05-06. Initial adversarial QA found hidden audio refresh debounce work after close and stale Process Manager refresh mutation after close; both were fixed and re-reviewed CLEAN. Full `npm run validate` passed.

Acceptance criteria:

- Audio open/close does not leave polling active.
- HMR/manual reload does not double-fire TopBar, BottomBar, search, task preview, or process manager listeners.
- No current behavior changes except removing leaks and hidden work.

Tests to write:

- No additional tests unless QA finds an uncovered behavior.

Implementation tasks:

- Run adversarial review with focus on late promise resolution, duplicate cleanup, and missing close events.
- Record any live-smoke limitation.


Validation gate:

- `npm run validate` when feasible.
