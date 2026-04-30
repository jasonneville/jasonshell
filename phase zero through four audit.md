# Phase Zero Through Four Search Upgrade Audit

Date: 2026-04-30
Scope: `search_upgrade_plan.md` phases 0 through 4 only, with current repository state checked before moving to later upgrade phases.
Verdict: BLOCK

## Reviewed Scope

- Phase 0 red-baseline tests and query matrix.
- Phase 1 display intent preservation in `src/features/search/searchUxState.ts`, `src/components/SearchPanelSurface.svelte`, and `src/components/TopBar.svelte`.
- Phase 2 progressive local-first search in `src-tauri/src/search/mod.rs`, `src/lib/searchEngine.ts`, `src/lib/searchPanelState.ts`, `src-tauri/src/search_panel.rs`, and `TopBar.svelte`.
- Phase 3 persistent warm app index in `src-tauri/src/search/providers/apps.rs`, `src-tauri/src/main.rs`, and frontend/Rust timing contracts.
- Phase 4 fuzzy matching and highlights in `src-tauri/src/search/matcher.rs`, `src-tauri/src/search/scoring.rs`, small-catalog providers, `src/lib/searchEngine.ts`, and `SearchPanelSurface.svelte`.

I did not review Phase 5 as accepted implementation scope, except where current validation output and current files make Phase 0-4 claims hard to separate from later edits.

## Critical Findings

### 1. Progressive updates can keep old-query rows ahead of new-query local rows

Files:

- `src/components/TopBar.svelte:659`
- `src/components/TopBar.svelte:665`
- `src/components/TopBar.svelte:667`
- `src/components/TopBar.svelte:765`
- `src/lib/searchPanelState.ts:37`

Phase 2 claims useful local rows appear before Everything completes. The current frontend keeps prior visible rows during typing, then merges `local` and `provider` progress into the existing `searchResults` array for every non-`complete` phase:

```ts
searchResults = payload.phase === 'complete'
  ? nextPayload.results
  : mergeSearchPanelResultsByStableKey(searchResults, nextPayload.results);
```

`mergeSearchPanelResultsByStableKey` preserves existing row order first. That means this sequence is possible:

1. User searches `spotify`; Spotify rows are visible.
2. User changes query to `display settings`.
3. Typing payload intentionally keeps old Spotify rows.
4. Local progress returns `Display Settings`.
5. Merge appends/updates local rows after old Spotify rows instead of replacing the working set for the new query.
6. `Best match` renders old-query rows until the final complete payload replaces the list.

This defeats the core Phase 2 purpose for an advanced launcher replacement: exact new intent can be visually buried during the progressive phase. It also means "local rows appear before slow Everything" is technically true in state, but not reliably true in the first visible rows.

Required fix before next phases: track the query attached to the current result set and replace, rather than merge, when a `local` phase belongs to a newer query. Preserve old rows only for the `typing` placeholder, empty recoverable errors, or explicit current-best presentation that is visually marked as stale. Add a regression where `spotify` visible rows are followed by a `display settings` local payload and the first visible row becomes `Display Settings` before `complete`.

### 2. Full validation gate is currently red

Command:

```powershell
npm run validate
```

Result: failed during `npm run test:node` after `svelte-check` and production build passed.

Failing tests observed:

- `tests/searchOverhaulPhase0.test.mjs:133` - phase 7 audit expectations are explicit and validated
- `tests/searchOverhaulPhase0.test.mjs:159` - phase 8 QA and performance expectations are testable from the overhaul plan
- `tests/searchOverhaulPhase0.test.mjs:234` - rapid typing publishes pending or current-best payload before provider resolution and gates stale responses
- `tests/searchOverhaulPhase0.test.mjs:327` - phase 3 everything provider has cached health, bounded simple-name request, and timings
- `tests/searchOverhaulPhase6.test.mjs:76` - phase 6 top-bar publishes pending payload before deferred engine request and applies latest response only

Some failures look like stale source-regex expectations from the older overhaul plan, not necessarily broken runtime behavior. That distinction does not make the repo safe to advance. The canonical validation gate is red, and stale guard tests are still stray issues because they block future confidence and can hide real search regressions in noise.

Required fix before next phases: either update these tests to the current `search_upgrade_plan.md` phase contracts or retire them into explicit historical/quarantine tests. Then rerun `npm run validate`.

## Warnings

### 3. Duplicate backend ids are DOM-safe but not selection-safe

Files:

- `src/components/SearchPanelSurface.svelte:140`
- `src/components/SearchPanelSurface.svelte:174`
- `src/components/TopBar.svelte:819`
- `src/components/TopBar.svelte:846`
- `tests/searchUxState.test.mjs:198`

Phase 1 added duplicate-safe `rowKey` and `domId`, but panel events still emit only `result.id`. TopBar selection and activation then use `find((result) => result.id === event.payload)`, which always resolves the first duplicate.

The test at `tests/searchUxState.test.mjs:198` explicitly preserves duplicate raw activation ids, but it does not prove a user can click, select, or activate the second duplicate. If providers ever produce duplicate ids with different `recordKey`/path/action, the visible row can be rendered uniquely but the wrong result can be selected or launched.

Required fix: event payloads should carry a duplicate-safe identity such as `recordKey` plus `resultIndex` or visible `rowKey`, while activation still validates against the current result array.

### 4. Phase 4 hidden/alias matches can have no visible highlight explanation

Files:

- `src-tauri/src/search/matcher.rs:121`
- `src-tauri/src/search/matcher.rs:129`
- `src-tauri/src/search/matcher.rs:154`
- `src-tauri/src/search/matcher.rs:162`
- `src-tauri/src/search/scoring.rs:96`

The matcher intentionally returns empty highlight data for hidden-field matches. This keeps implementation bounded, but it weakens the Phase 4 acceptance statement that highlight spans explain why a fuzzy result matched. Title/subtitle matches are covered; alias/path/term matches can appear with no highlighted reason.

This is not a blocker for the accepted Phase 4 query gate (`sptfy`, `vs code`, `disp set`) because those are title/title-like matches. It is still a residual UX risk for a launcher intended to replace Windows-key search. If hidden aliases remain part of matching, add a compact match-reason affordance or highlight the best visible field fallback.

## Clean Findings

- Phase 1 visible ordering model does put backend-ranked top 1-3 rows under `Best match`, then renders remaining Apps/Folders/Files/Settings/Windows/Commands groups in stable group order.
- Top-bar ArrowUp/ArrowDown/Enter now uses the same visible-row helper model as the panel.
- Phase 2 Rust emits local progress before Everything and final complete progress after ranking.
- Search-panel Rust state and frontend panel state both reject stale sequence/query regressions and allow `complete` after recoverable `error`.
- Phase 3 app cache is persistent, bounded, non-secret metadata only, startup-hydrated before async warm refresh, and query-time app search remains cache-only.
- Phase 4 fuzzy support is scoped to apps/settings/commands/local folders in Rust scoring; Everything rows are not rescued by broad Rust fuzzy matching.
- Action safety for `ms-settings:*` and `control.exe` remains validated in Rust and TypeScript contracts.

## Validation Performed

Passed:

```powershell
npx tsc -p tsconfig.test.json
node --test tests\searchUxState.test.mjs tests\searchPanelState.test.mjs tests\searchEngineContracts.test.mjs
cargo test --manifest-path src-tauri\Cargo.toml search::
cargo check --manifest-path src-tauri\Cargo.toml
cargo fmt --manifest-path src-tauri\Cargo.toml --check
git diff --check
```

Focused results:

- Focused Node search suites: 43 passed.
- Focused Rust search tests: 64 passed.
- Rust cargo check: passed.
- Rust format check: passed.
- Diff whitespace check: passed.

Failed:

```powershell
npm run validate
```

The failure is in full Node tests, after `svelte-check` and build succeeded.

## Persona Notes

Saboteur: stale progressive merging is the production break. It can make a user type an exact settings query and still see old app/file rows in the prime launcher slot until final completion.

New Hire: phase boundaries are hard to trust because old search overhaul source-regex tests still run and fail against newer phase implementation. This creates noisy validation where a future developer cannot tell whether the system is broken or the tests are stale.

Security Auditor: no new obvious command-injection path was introduced in phases 0-4. Settings/control actions remain constrained, and app-cache persistence stores path/title/alias metadata rather than secrets. Duplicate-id activation is a correctness risk, not a direct security issue.

## Recommendation

Do not start the next implementation phases yet. Fix progressive new-query merging and make `npm run validate` green or intentionally quarantine stale historical tests. Then add regressions for duplicate-id selection and hidden-match explanation before expanding more search scope.
