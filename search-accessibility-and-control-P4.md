# Search Accessibility And Control (P4)

## Goal

Add practical search controls: clear buttons in both search inputs and an app-scoped Windows-key override that opens JasonShell search while the app runs.

## Source Items

- `updates.md` item 7: Clear button in top-right search and middle search view.
- `updates.md` item 15: Windows key should trigger JasonShell search instead of Windows default search while active.

## Priority Rationale

P4. Useful interaction work, but Windows-key interception is native/global enough that shell stability and core file/launch fixes come first.

## Implementation Map

- Top-bar search: `src/components/TopBar.svelte`.
- Centered search panel: `src/components/SearchPanelSurface.svelte`.
- Search state helpers: `src/features/search/searchUxState.ts`, `src/lib/searchEngine.ts`.
- Events: `src/ipc/events.ts`, current search-panel query/close/open events.
- Native keyboard hook candidate: new small Rust module or existing native input module under `src-tauri/src/`, registered in `src-tauri/src/main.rs`.
- Capabilities/contracts: `src-tauri/src/contracts.rs`, Tauri capability JSON if commands/events change.

## Phase 1: Clear Button RED Tests

### Work

- Trace top-bar input state: `searchQuery`, `searchInputDraft`, sequence/freshness counters, provider request start.
- Trace centered panel local draft and event emit path to `top-bar`.
- Define clear contract: empty visible input immediately, clear visible results or show idle state, cancel/reject stale provider responses, keep focus in active input.

### Tests

- Add `tests/searchClearButtons.test.mjs` source test proving `TopBar.svelte` renders a clear button when query/draft is non-empty with `aria-label="Clear search"`.
- Add source test proving `SearchPanelSurface.svelte` renders its own clear button for centered input.
- Add state test in `searchUxState` helpers: clearing increments/uses a newer sequence so old `search_engine` response for prior query is rejected.
- Add source test proving clear button handler does not call provider search directly; it routes through existing query update/close/reset path.

### Acceptance Criteria

- RED tests fail because clear controls are missing.
- Tests cover top-bar, centered panel, pending search response, empty query hidden-state.

## Phase 2: Clear Button GREEN Implementation

### Work

- Add clear button next to top-bar search input; render only when draft/query non-empty.
- Add clear button inside centered search view near input/results header.
- Implement `clearSearch()` in `TopBar.svelte` using the same sequence gate as normal query updates with `query=''`.
- In centered panel, emit a query event with empty query and latest `inputSequence`; do not mutate top-bar state directly.
- Keep focus via `tick()`/input ref after clear; do not close panel unless existing empty-query behavior already does so.

### Tests

- Run focused search clear tests.
- Run existing search tests: `npm run test:node` or narrower search suite if available.

### Acceptance Criteria

- Top-bar and centered clear buttons empty search with one click.
- Stale provider responses cannot repopulate cleared results.
- Keyboard focus remains in active search input.

## Phase 3: Windows-Key Override RED Tests

### Work

- Choose minimal native design: low-level keyboard hook installed on app startup, removed on shutdown.
- Define bare Windows-key only: intercept `VK_LWIN`/`VK_RWIN` tap with no modifier chord; preserve Win+L, Win+D, Win+R, etc.
- Define event emitted to top-bar: reuse current search-open event/command if one exists; otherwise add one constant.
- Define failure behavior: if hook install fails, app logs/returns structured error and continues.

### Tests

- Add Rust tests for key classifier: left win down/up tap -> `OpenSearch`; right win -> `OpenSearch`; Win+R/Win+D -> `PassThrough`; repeated keydown -> no duplicate spam.
- Add Rust test for lifecycle state: install once, uninstall idempotent, shutdown cleanup.
- Add Node/source test proving emitted event targets `top-bar` and opens centered search through existing search open path.
- Add test for config/feature flag only if the repo already gates native hooks; do not invent setting unless needed.

### Acceptance Criteria

- RED tests fail because hook/classifier/event path absent.
- Tests explicitly protect common Windows shortcuts from interception.

## Phase 4: Windows-Key Override GREEN Implementation

### Work

- Implement pure key classifier first; keep hook callback minimal and call classifier.
- Register hook during Tauri setup after windows/event targets exist.
- On bare Windows-key tap, suppress default only for that tap and emit/open JasonShell search via top-bar target.
- Do not suppress modified shortcuts.
- Store hook handle in managed state and unhook on exit/drop.
- Add command or diagnostic state only if needed for tests/smoke.

### Tests

- Run focused Rust key hook tests.
- Run search event routing Node tests.
- Run `npm run cargo:test` and `npm run test:node`.

### Acceptance Criteria

- Bare Windows key opens JasonShell search while app runs.
- Common Win+ shortcuts still pass through.
- Closing JasonShell restores default Windows key behavior.

## Phase 5: Refactor, Spec, Validation

### Work

- Keep keyboard hook code isolated in one Rust module with a pure tested classifier.
- Add security/safety note to `master_spec.md`: hook is app-scoped lifecycle and does not log keystrokes.
- Update Search section and Change Ledger.
- Add smoke steps for clear buttons, stale response rejection, bare Win key, Win+R pass-through, shutdown restore.

### Tests

- Run `npm run test:node`.
- Run `npm run cargo:test`.
- Run `npm run validate`.

### Acceptance Criteria

- Full validation passes.
- Manual smoke verifies search clear behavior and Windows-key lifecycle.
