# Task: FN-001 - Label-specific frontend code splitting for JasonShell surfaces

**Created:** 2026-05-11
**Size:** M

## Review Level: 2 (Plan and Code)

**Assessment:** This is a focused frontend build/runtime refactor with measurable performance impact and moderate risk because `src/App.svelte` is the entry router for every Tauri surface. The change is reversible and should not alter Rust IPC behavior, but it touches surface bootstrapping, Svelte async rendering, and source-level routing tests.
**Score:** 4/8 — Blast radius: 1, Pattern novelty: 2, Security: 0, Reversibility: 1

## Mission

Implement label-specific frontend code splitting so each JasonShell Tauri webview loads only the Svelte surface needed for its window label instead of statically importing every heavy surface through `src/App.svelte`. This should reduce cold-start parse/load bytes for non-terminal surfaces such as `top-bar`, `bottom-bar`, `search-panel`, and `process-manager` while preserving the existing label-to-surface routing contract, unsupported-label fallback, and all surface lifecycle behavior.

## Dependencies

- **None**

## Context to Read First

- `master_spec.md` — current canonical architecture, especially the primary surface list, search, process manager, stack browser, terminal panel, and persistent surface lifecycle notes.
- `changelog.md` — recent change-history style and validation examples.
- `package.json` — exact project validation commands (`npm run build`, `npm run check`, `npm run test:node`, `npm run validate`).
- `src/App.svelte` — current static-import surface router to refactor.
- `src/main.ts` — single Vite/Svelte app entry point and global theme/preferences bootstrap.
- `src/lib/shellSurface.ts` — authoritative `ShellSurface`, metadata, and label resolution.
- `src/components/TopBar.svelte`, `src/components/BottomBar.svelte`, `src/components/SearchPanelSurface.svelte`, `src/components/ProcessManagerSurface.svelte`, `src/components/TerminalPanelSurface.svelte`, `src/components/StackPopupSurface.svelte` — representative light/heavy surfaces and lifecycle expectations.
- `tests/audioControls.test.mjs`, `tests/commandPanelWiring.test.mjs`, `tests/controlPlaneRouting.test.mjs`, `tests/persistentTerminalPanel.test.mjs`, `tests/processManagerWiring.test.mjs`, `tests/settingsPanelWiring.test.mjs`, `tests/topBarCalendar.test.mjs`, `tests/trayPanelWiring.test.mjs` — existing source tests that assert App routing and will need to remain meaningful after lazy loading.
- `vite.config.ts` — current Vite build configuration; inspect before deciding whether a manual chunk config is necessary.

## File Scope

- `src/App.svelte` (modified)
- `src/main.ts` (modified only if needed for lazy-loading or measurement support)
- `src/lib/shellSurface.ts` (modified only if a typed surface-component mapping helper belongs next to label metadata)
- `src/lib/surfaceLoader.ts` (new, if extracting lazy import mapping out of `App.svelte`)
- `tests/surfaceCodeSplitting.test.mjs` (new)
- Existing App routing/source tests under `tests/*.test.mjs` that currently assert static imports/render branches (modified as needed, without weakening coverage)
- `master_spec.md` (modified)
- `changelog.md` (modified)

## Steps

### Step 0: Preflight

- [ ] Required files and paths exist.
- [ ] Dependencies satisfied.
- [ ] Run `npm run build` once before implementation and record the baseline `dist/assets/*.js` chunk filenames and byte sizes, including the largest JS chunk and any surface-related chunks, for later before/after comparison.
- [ ] Read current source tests that inspect `src/App.svelte` so the refactor preserves their behavioral intent rather than simply deleting assertions.

### Step 1: Introduce a typed lazy surface loader

- [ ] Replace `src/App.svelte` static imports of surface components (`TopBar`, `BottomBar`, `SearchPanelSurface`, `StackPopupSurface`, `ProcessManagerSurface`, `TerminalPanelSurface`, `CommandPanelSurface`, `AudioPanelSurface`, `CalendarPanelSurface`, etc.) with label/surface-specific dynamic imports.
- [ ] Keep `resolveSurfaceFromLabel(label)` and `shellSurfaceMetadata[surface]` as the routing and title/subtitle authorities; unsupported labels must still resolve to `unknown` and render the existing fallback content.
- [ ] Implement the lazy mapping in `src/App.svelte` or a small helper such as `src/lib/surfaceLoader.ts`, with TypeScript coverage for every non-`unknown` `ShellSurface` value so future surfaces cannot silently fall through.
- [ ] Preserve global behavior from `App.svelte`: current-window label discovery, `<svelte:head>` title metadata, context-menu suppression, and synchronous theme/preferences listener cleanup from `installShellThemeSync()` and `installShellPreferencesSync()`.
- [ ] Add or update automated source tests that fail if `src/App.svelte` statically imports heavy surface components again, and that verify each known surface has a dynamic import path.

**Artifacts:**
- `src/App.svelte` (modified)
- `src/lib/surfaceLoader.ts` (new, if used)
- `tests/surfaceCodeSplitting.test.mjs` (new)

### Step 2: Preserve surface rendering and fallback behavior

- [ ] Render the lazily loaded component with Svelte 5-compatible dynamic component syntax/patterns, including a safe loading state while the component import resolves.
- [ ] Ensure import failures do not crash the whole app silently: show a clear unsupported/error-style fallback that includes the surface metadata and records the failure to `console.error` for diagnostics.
- [ ] Preserve all existing Tauri label outcomes: `top-bar`, `bottom-bar`, `task-preview`, `search-panel`, `stack-popup`, `process-manager`, `control-plane`, `settings-panel`, `tray-panel`, `terminal-panel`, `command-panel`, `audio-panel`, and `calendar-panel` must still render their same surface component.
- [ ] Update existing tests that match `src/App.svelte` so they assert lazy routing/component mapping instead of static imports, while retaining coverage for every surface currently checked by those tests.
- [ ] Run targeted tests for changed files: at minimum `npm run test:node` after updating source tests, or a focused equivalent `node scripts/clean-dist-tests.mjs && npx tsc -p tsconfig.test.json && node --test tests/surfaceCodeSplitting.test.mjs <changed-existing-tests>` during development.

**Artifacts:**
- `src/App.svelte` (modified)
- `tests/*.test.mjs` App-routing assertions (modified as needed)

### Step 3: Verify production chunk splitting and record metrics

- [ ] Run `npm run build` after implementation and verify Vite emits separate/lazy JS chunks for heavy surface modules instead of one monolithic chunk containing all surfaces.
- [ ] Compare before/after `dist/assets/*.js` byte sizes and record the results in a task document using `fn_task_document_write` with key `chunk-metrics`.
- [ ] Ensure the largest production JS chunk is meaningfully below the previously observed FN-004 856 kB single chunk, or document exactly why a remaining shared/vendor chunk is unavoidable.
- [ ] Inspect built output or Vite manifest/chunk filenames to confirm non-terminal surfaces do not eagerly include `TerminalPanelSurface`, xterm imports, Stack Browser, Process Manager, and Search Panel code unless their own label loads that surface.
- [ ] Add any source-level or build-output test that is practical and stable in this repository to guard against reintroducing static imports in `src/App.svelte`.

**Artifacts:**
- `dist/` build output (generated, not committed unless this repo already tracks it)
- Task document `chunk-metrics` via `fn_task_document_write`
- `tests/surfaceCodeSplitting.test.mjs` (new/modified)

### Step 4: Testing & Verification

> ZERO test failures allowed. Full test suite as quality gate.
> If keeping lint/tests/build/typecheck green requires edits outside the initial File Scope, make those fixes as part of this task.

- [ ] Run Svelte/type lint-equivalent check (`npm run check`).
- [ ] Run full automated test suite (`npm run test:node`).
- [ ] Run project typecheck/build (`npm run build`).
- [ ] Run Rust validation if frontend changes expose or depend on Tauri labels/events (`npm run cargo:test` and `npm run cargo:check`), or run full `npm run validate` if time permits.
- [ ] Fix all failures; do not mark this task complete with known failing tests.
- [ ] Confirm the task includes at least one real automated test with assertions that runs under the Node test runner; typecheck/build alone is not sufficient.

### Step 5: Documentation & Delivery

- [ ] Update `master_spec.md` to describe that `src/App.svelte` now resolves the Tauri window label first and lazy-loads only the matching surface component, while unsupported labels render fallback UI.
- [ ] Update `changelog.md` following `CHANGELOG_POLICY.md`, including before/after chunk-size summary and validation commands run.
- [ ] Save documentation deliverables as task documents via `fn_task_document_write` (key="docs", content=...), including a concise implementation summary, validation results, and chunk-size table or bullet list.
- [ ] Out-of-scope findings created as new tasks via `fn_task_create` tool.

## Documentation Requirements

**Must Update:**
- `master_spec.md` — add/update the frontend bootstrap/surface-routing note to state that surface components are label-lazy-loaded rather than statically imported by every webview.
- `changelog.md` — add an entry with the code-splitting implementation, measured before/after chunk sizes, and validation commands.

**Check If Affected:**
- `README.md` — update only if it currently documents startup/build architecture or surface routing in a way that becomes inaccurate.
- `docs/` — update only if any existing performance/startup or surface architecture doc mentions a single monolithic frontend bundle.

## Completion Criteria

- [ ] All steps complete.
- [ ] Each supported Tauri label still renders the same surface component via lazy loading.
- [ ] Unsupported labels still show fallback UI.
- [ ] `npm run build` emits separate/lazy chunks for heavy surfaces and the before/after chunk sizes are recorded in task documents.
- [ ] App routing/source tests cover the lazy mapping and prevent reintroducing static heavy-surface imports.
- [ ] `npm run check` passing.
- [ ] `npm run test:node` passing.
- [ ] `npm run build` passing.
- [ ] Documentation updated.

## Git Commit Convention

Commits at step boundaries. All commits include the task ID:

- **Step completion:** `feat(FN-001): complete Step N — description`
- **Bug fixes:** `fix(FN-001): description`
- **Tests:** `test(FN-001): description`

## Do NOT

- Expand task scope beyond frontend label-specific loading and measurement.
- Change Rust window creation, labels, IPC command names, event names, or permissions unless a test proves it is required for this frontend refactor.
- Remove or weaken surface lifecycle guards in persistent surfaces.
- Remove existing App routing coverage; update assertions to match lazy routing instead.
- Skip tests.
- Refuse necessary fixes just because they touch files outside the initial File Scope.
- Commit without the task ID prefix.
- Remove, delete, or gut modules, settings, interfaces, exports, or test files outside the File Scope.
- Remove features as "cleanup" — if something seems unused, create a task via `fn_task_create`.

## Changeset Requirements

If this task REMOVES existing functionality (deleting modules, settings, API endpoints, or exports), a changeset file is REQUIRED:
- Create `.changeset/fn-001-removal.md` explaining what was removed and why
- This is mandatory for any net-negative change (more deletions than additions to existing files)
