# JasonShell Search Upgrade Plan

Date: 2026-04-30
Status: Draft plan for implementation
Source inputs: `search_improvements.md`, current `master_spec.md`, prior search overhaul constraints

## Implementation Status

- 2026-04-30 `[USER]` IN_PROGRESS: Targeted implementation slice started for Phase 0 and Phase 1 only. Phase 0 must finish before Phase 1 starts; later phases remain planned.
- 2026-04-30 `[CODE]` COMPLETED: Phase 0 red baseline is now encoded in `tests/searchUxState.test.mjs`, `tests/searchPanelState.test.mjs`, and `tests/searchEngineContracts.test.mjs`. The new assertions capture the current grouped-render display ordering bug, the panel's raw-index keyboard/ARIA mismatch, and the remaining Rust progressive/app-cache/fuzzy gaps without touching Phase 1 rendering code.
- 2026-04-30 `[TOOL]` VALIDATED: Phase 0 validation used `npx tsc -p tsconfig.test.json` plus focused Node test runs against the search UX and search engine contract suites. The new Phase 0 assertions fail at the intended red gaps while the untouched baseline assertions remain intact.
- 2026-04-30 `[CODE]` COMPLETED: Phase 1 Display Intent Preservation is now implemented in `src/features/search/searchUxState.ts` and `src/components/SearchPanelSurface.svelte`. The panel builds a flat `visibleRows` model with a `Best match` section for the top 1-3 backend-ranked rows, removes those rows from later sections, renders remaining `Apps`/`Folders`/`Files`/`Settings`/`Windows`/`Commands` groups in stable backend order within each group, and drives selection/activation/scroll/ARIA from visible row order. Follow-up QA hardened visible row identity with duplicate-safe `rowKey`/DOM ids while preserving raw backend result ids for activation.
- 2026-04-30 `[TOOL]` VALIDATED: Phase 1 validation passed with `npx tsc -p tsconfig.test.json`; `node --test tests\searchUxState.test.mjs tests\searchPanelState.test.mjs tests\searchEngineContracts.test.mjs` passed 35 assertions with 2 explicit TODOs for deferred later phases; `cargo test --manifest-path src-tauri\Cargo.toml search::` passed 45/45. Helper-level tests now cover query-gate style top-row ordering, visible navigation order, duplicate backend ids, and visible group labeling. Adversarial re-review returned CONCERNS only for retained source-regex guard tests; duplicate-id and phase-boundary concerns were resolved.
- 2026-04-30 `[USER]` IN_PROGRESS: Targeted implementation slice started for Phase 2 and Phase 3. Phase 2 Progressive Local-First Search must finish and pass its acceptance validation before any Phase 3 Persistent Warm App Index implementation begins.
- 2026-04-30 `[CODE]` COMPLETED: Phase 2 Progressive Local-First Search is implemented. Rust `search_engine` remains the final compatibility command while emitting `search-engine:progress` payloads for local, provider, complete, and recoverable error phases. The frontend still publishes typing immediately, applies progress events through the existing `SearchProgressPayload` shape, merges rows by stable `recordKey`/`id`, preserves useful rows while newer input is pending, and rejects stale query/sequence/phase regressions in both frontend panel state and Rust search-panel storage.
- 2026-04-30 `[TOOL]` VALIDATED: Phase 2 acceptance passed with `npx tsc -p tsconfig.test.json`; `node --test tests\searchEngineContracts.test.mjs tests\searchPanelState.test.mjs tests\searchUxState.test.mjs` passed 38/38 with 1 TODO for the later app-cache/fuzzy gap; `cargo test --manifest-path src-tauri\Cargo.toml search::` passed 47/47. Phase 3 may now begin.
- 2026-04-30 `[CODE]` COMPLETED: Phase 3 Persistent Warm App Index is implemented. `search/providers/apps.rs` persists `search-app-index-v1.json` under app local data with only title/path/aliases/source/priority/indexed timestamp metadata, hydrates it at startup before async warm refresh, returns stale persisted rows with refresh state, ignores corrupt cache and schedules refresh, reports `Hit`/`Refresh`/`Miss`/`Disabled`/`Indexing` plus cache age, and keeps query-time app search cache-only.
- 2026-04-30 `[TOOL]` VALIDATED: Phase 3 acceptance passed with `cargo test --manifest-path src-tauri\Cargo.toml search::providers::apps` passing 8/8 and `cargo test --manifest-path src-tauri\Cargo.toml search::` passing 50/50. Cross-contract checks also passed: `npx tsc -p tsconfig.test.json`; `node --test tests\searchEngineContracts.test.mjs tests\searchPanelState.test.mjs tests\searchUxState.test.mjs` passed 40/40 with 1 TODO for deferred fuzzy work; `cargo test --manifest-path src-tauri\Cargo.toml search_panel` passed 9/9; `cargo fmt --manifest-path src-tauri\Cargo.toml --check` passed; scoped `git diff --check` reported line-ending warnings only.
- 2026-04-30 `[TOOL]` QA: Adversarial Phase 2/3 review returned CLEAN. Residual non-blocking risks: no live WebView mocked-slow-Everything smoke was run, and `cacheAgeMs` is coarse because it is derived from whole-second persisted timestamps.
- 2026-04-30 `[USER]` IN_PROGRESS: Targeted implementation slice started for Phase 4 and Phase 5. Phase 4 Fuzzy Matching and Highlights must finish and pass acceptance validation before any Phase 5 Everything Query Modes implementation begins.
- 2026-04-30T22:05:00-05:00 `[USER]` IN_PROGRESS: Narrow Phase 4-only implementation slice started. Phase 4 Fuzzy Matching and Highlights may change bounded small-catalog providers, Rust ranking/contracts, frontend highlight rendering, focused tests, and spec/plan status. Phase 5 Everything query modes remain blocked and untouched until a later request.
- 2026-04-30 `[CODE]` COMPLETED: Phase 4 Fuzzy Matching and Highlights implemented before Phase 5 work began. Added bounded launcher-style matching/highlight metadata for apps/settings and rank gating that keeps broad Everything rows out of Rust fuzzy matching.
- 2026-04-30 `[TOOL]` VALIDATED: Phase 4 gate passed via separate Cargo filters equivalent to the documented Rust target (`search::scoring`, `search::providers::apps`, `search::providers::settings`), `npx tsc -p tsconfig.test.json`, and `node --test tests\searchUxState.test.mjs tests\searchEngineContracts.test.mjs`.
- 2026-04-30 `[CODE]` COMPLETED: Phase 5 Everything Query Modes implemented after Phase 4 acceptance passed. Added simple-name, path-like, folder-navigation, and app-like query modes; kept content search disabled; kept full-path search path-like only; bounded overfetch; and boosted folder/app intent within Everything result mapping.
- 2026-04-30 `[TOOL]` VALIDATED: Phase 5 gate passed via `cargo test --manifest-path src-tauri\Cargo.toml search::providers::everything` passing 12/12 and `cargo test --manifest-path src-tauri\Cargo.toml search::` passing 64/64. Final hygiene also passed with `cargo fmt --manifest-path src-tauri\Cargo.toml --check`, `npx tsc -p tsconfig.test.json`, `node --test tests\searchUxState.test.mjs tests\searchEngineContracts.test.mjs` passing 26/26, and `git diff --check` reporting only normal CRLF conversion warnings.

## Summary

Goal: improve JasonShell search with performance and result accuracy first.

Priority order:

1. Keep the correct backend top result visible at the top.
2. Make search feel instant through progressive local-first updates.
3. Make app results trustworthy on cold start through persisted cache.
4. Improve matching quality with bounded fuzzy scoring and highlights.
5. Improve Everything recall with query-specific modes.
6. Expand settings coverage and polish icons, health, usage learning, and scopes.

Canonical constraints:

- Production typed search stays on `src/lib/searchEngine.ts` plus Rust `search_engine`.
- Do not route visible typed search back through legacy `search_system`, warmed-cache display fallback, Windows Search fallback, `searchCatalog.ts`, or TypeScript `rankSearchResults`.
- Input echo remains sacred: no provider, ranking, filesystem, localStorage, icon, SDK detection, or native show work may block typed-character rendering.
- Everything remains the broad filesystem lane; apps, settings, local folders, commands, and windows remain bounded high-intent lanes.
- Performance and accuracy outrank visual polish until P0/P1 gates pass.

## Subagent and Skill Assignment

Required skill/subagent map for implementation:

- Spec/TDD planner: `spec-driven-workflow`, `tdd-guide`
- Frontend search UI: `senior-frontend`, `tdd-guide`
- Rust search/backend: `rust-skills`, `senior-backend`
- Rust ranking and providers: `rust-skills`, `tdd-guide`
- QA/adversarial reviewer: `adversarial-reviewer`, `tdd-guide`
- Orchestrator/user updates: `caveman` ultra

Do not load irrelevant skills. `openai-docs`, `imagegen`, plugin skills, prompt engineering, and cost optimization are out of scope unless a later request explicitly touches those domains.

## Phase 0: Spec and Red Baseline

Owner: Spec/TDD planner

Skills: `spec-driven-workflow`, `tdd-guide`

Purpose: lock behavior before touching implementation.

Ordered tasks:

1. Add `master_spec.md` ledger entry for the implementation slice before coding.
2. Add/update a plan status section in this file when each phase starts and finishes.
3. Add red tests for visible ordering in `tests/searchUxState.test.mjs`.
4. Add red tests for visual keyboard order in `tests/searchUxState.test.mjs` or `tests/searchPanelState.test.mjs`.
5. Add red tests for progressive local-first payload behavior in `tests/searchEngineContracts.test.mjs` and targeted frontend state tests.
6. Add Rust red tests under `src-tauri/src/search/**` for app-cache persistence, fuzzy scoring, Everything query mode classification, and settings catalog coverage.
7. Build the fixed query matrix:
   - `display settings`
   - `sound settings`
   - `control panel`
   - `spotify`
   - `sptfy`
   - `vs code`
   - `disp set`
   - `jnev1`
   - `C:\dev`
   - `dev`

Acceptance targets:

- Current display grouping failure is captured.
- Current keyboard/visual order mismatch is captured.
- Current no-progressive-final-only response shape is captured.
- Current in-memory-only app cache limitation is captured.
- Fuzzy gaps are captured before scoring changes.

Validation commands:

```powershell
npx tsc -p tsconfig.test.json
node --test tests\searchUxState.test.mjs tests\searchPanelState.test.mjs tests\searchEngineContracts.test.mjs
cargo test --manifest-path src-tauri\Cargo.toml search::
```

Exit gate: planned red tests fail for the known gaps and pass for existing protected behavior.

## Phase 1: Display Intent Preservation

Owner: Frontend search UI subagent

Skills: `senior-frontend`, `tdd-guide`

Status: Completed 2026-04-30.

Purpose: fix the most user-visible accuracy bug first. Backend rank 1 must be visible row 1.

Ordered tasks:

1. Replace fixed group-first rendering with a display model that starts with `Best match`.
2. Put top 1-3 backend-ranked rows into `Best match` in exact backend order.
3. Remove those best-match rows from the remaining grouped sections to avoid duplicate visible rows.
4. Split remaining groups into clearer labels:
   - `Apps`
   - `Folders`
   - `Files`
   - `Settings`
   - `Windows`
   - `Commands`
5. Preserve backend order inside each remaining group.
6. Build a flat `visibleRows` model and use it for rendering, `aria-activedescendant`, scroll reveal, click selection, Enter activation, and ArrowUp/ArrowDown behavior.
7. Keep row height stable and preserve existing result row raw button/role semantics.

Acceptance targets:

- For every backend-ranked response, first visible selectable row is `results[0]`.
- Keyboard selection traverses the same order the user sees.
- Settings results are not buried under files/folders/apps for settings queries.
- Apps and folders remain separated for scanability.

Query gate:

| Query | Required top visible row |
| --- | --- |
| `display settings` | `Display Settings` |
| `sound settings` | `Sound Settings` |
| `control panel` | Windows `Control Panel`, not JasonShell Control Plane first |
| `spotify` | Installed Spotify app when app provider has it |
| `C:\dev` | `C:\dev` folder |
| `dev` | `C:\dev` unless stronger exact app/settings intent exists |

Validation commands:

```powershell
npx tsc -p tsconfig.test.json
node --test tests\searchUxState.test.mjs tests\searchPanelState.test.mjs
```

Exit gate: display and keyboard tests pass without touching backend ranking.

## Phase 2: Progressive Local-First Search

Owners: Rust search subagent plus frontend search UI subagent

Skills: `rust-skills`, `senior-backend`, `senior-frontend`, `tdd-guide`

Purpose: make useful rows appear before Everything completes.

Ordered tasks:

1. Keep `search_engine` final response as the compatibility command.
2. Use the existing `SearchProgressPayload` contract as the progressive transport, or add the smallest compatible event wrapper if needed.
3. Emit/apply phases:
   - `typing`: immediate pending/current-best payload
   - `local`: settings, apps, local folders, open windows
   - `provider`: Everything rows or provider-specific batch
   - `complete`: final merged/ranked result set
   - `error`: recoverable provider failure while keeping useful rows
4. Run local providers before Everything.
5. Run Everything as a later provider batch.
6. Merge batches by stable `id`/`recordKey`.
7. Never clear useful current rows while a newer query is pending.
8. Reject stale batches by query and sequence in both frontend and Rust search-panel state.
9. Add provider timing evidence per phase.

Acceptance targets:

- First typed character still echoes immediately.
- Local rows appear before mocked slow Everything resolves.
- Everything rows merge in later without flicker.
- Older provider batches cannot overwrite newer query state.
- No provider work serializes ahead of latest query input.

Validation commands:

```powershell
npx tsc -p tsconfig.test.json
node --test tests\searchEngineContracts.test.mjs tests\searchPanelState.test.mjs tests\searchUxState.test.mjs
cargo test --manifest-path src-tauri\Cargo.toml search::
```

Exit gate: mocked slow Everything proves local-first progressive rendering.

## Phase 3: Persistent Warm App Index

Owner: Rust search subagent

Skills: `rust-skills`, `senior-backend`, `tdd-guide`

Status: COMPLETED 2026-04-30. Persisted warm app cache now hydrates at startup, stale rows survive refresh, corrupt cache is ignored, cache age/state is reported, and query-time app search remains scan-free.

Purpose: first app query after launch should not miss installed apps because memory cache is cold.

Ordered tasks:

1. Add a disk-backed app index under the app data/config location used by JasonShell conventions.
2. Store only non-secret app metadata:
   - title
   - path
   - aliases
   - source
   - priority
   - indexed timestamp
3. Load persisted cache at startup before first query when present.
4. Keep current async warm/refresh path as fallback.
5. Return stale persisted rows while refresh runs.
6. Mark cache state as `Hit`, `Refresh`, `Miss`, `Disabled`, or `Indexing` with cache age.
7. Handle corrupt cache by ignoring it and scheduling refresh.
8. Ensure query-time app search never scans Start Menu or broad app roots.

Acceptance targets:

- First `spotify` query after launch can use last known app cache.
- First `vs code` query after launch can use last known app cache.
- Stale cache still returns rows while refresh is deferred.
- Corrupt cache does not break search.
- Query-time path remains in-memory/disk-loaded only.

Validation commands:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml search::providers::apps
cargo test --manifest-path src-tauri\Cargo.toml search::
```

Exit gate: cold-start app trust fixed without per-query filesystem scans.

## Phase 4: Fuzzy Matching and Highlights

Status: completed 2026-04-30 before Phase 5 started.

Owner: Rust ranking subagent

Skills: `rust-skills`, `tdd-guide`

Purpose: improve launcher-style accuracy for abbreviations, partial terms, and small typos.

Ordered tasks:

1. Add bounded fuzzy matching for small catalogs only:
   - apps
   - settings
   - commands
   - important local folders
2. Do not fuzzy scan broad Everything file/folder rows in Rust.
3. Score match tiers in this order:
   - exact
   - prefix
   - acronym
   - token-prefix
   - subsequence
   - small edit distance
4. Keep exact app/settings/folder intent above fuzzy and usage signals.
5. Add match metadata for title/subtitle highlight spans.
6. Add frontend rendering for highlight spans without changing row dimensions.

Acceptance targets:

- `sptfy` finds Spotify.
- `vs code` finds Visual Studio Code.
- `disp set` finds Display Settings.
- Highlight spans explain why the result matched.
- Fuzzy does not make random Everything rows outrank exact settings/apps/folders.

Validation commands:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml search::scoring search::providers::apps search::providers::settings
npx tsc -p tsconfig.test.json
node --test tests\searchUxState.test.mjs tests\searchEngineContracts.test.mjs
```

Exit gate: fuzzy improves short-catalog accuracy without broad filesystem cost.

## Phase 5: Everything Query Modes

Status: completed 2026-04-30 after Phase 4 acceptance passed.

Owner: Rust Everything provider subagent

Skills: `rust-skills`, `senior-backend`, `tdd-guide`

Purpose: improve filesystem recall while preserving realtime speed.

Ordered tasks:

1. Add query classifier:
   - simple name query
   - path-like query
   - folder-navigation query
   - app-like query
2. Keep current name-only fast mode for simple names.
3. Enable full-path search only for path-like queries.
4. Prefer folders for folder-navigation intent.
5. Keep content search off the realtime path.
6. Keep SDK/runtime health cached; no per-keystroke SDK detection.
7. Keep Everything request overfetch bounded.
8. Measure direct SDK latency for representative queries.

Acceptance targets:

- `jnev1` returns Everything filesystem rows without blank waiting.
- `C:\dev` and path-like variants can use path-aware behavior.
- `dev` keeps `C:\dev` high while broad Everything rows remain useful.
- Simple name queries stay fast.
- Everything provider degradation does not block local rows.

Validation commands:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml search::providers::everything
cargo test --manifest-path src-tauri\Cargo.toml search::
```

Optional live probe:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml everything_ffi -- --ignored
```

Exit gate: better Everything recall without breaking latency budget.

## Phase 6: Settings Catalog Expansion

Owner: Rust settings provider subagent

Skills: `rust-skills`, `spec-driven-workflow`, `tdd-guide`

Purpose: move from small settings list to Windows Start/Flow-like common coverage.

Ordered tasks:

1. Expand settings rows with:
   - display name
   - category
   - URI or safe executable/action
   - aliases
   - keywords
   - Windows build/support guard
   - optional icon/glyph metadata
2. Prefer JSON or generated constants if the catalog becomes too large for readable Rust literals.
3. Keep Rust-side safety validation for `ms-settings:*` and `control.exe`.
4. Add catalog schema tests.
5. Add ranking tests that settings beat incidental files/folders for settings intent.

Acceptance targets:

- Common settings/control-panel tasks appear.
- `display settings`, `sound settings`, `windows settings`, and `control panel` remain top intent results.
- Unsafe action data is rejected.

Validation commands:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml search::providers::settings
cargo test --manifest-path src-tauri\Cargo.toml search::
```

Exit gate: settings coverage improves without weakening action safety.

## Phase 7: Icons, Provider Health, Usage Learning, and Optional Scopes

Owners: Frontend search UI subagent plus Rust ranking subagent

Skills: `senior-frontend`, `rust-skills`, `tdd-guide`

Purpose: polish search after core speed and accuracy are stable.

Ordered tasks:

1. Add async icon enrichment after first paint:
   - app shortcut icon
   - folder icon
   - file extension icon
   - settings glyph
2. Keep stable row dimensions while icons load.
3. Show compact provider health footer only when provider is degraded/unavailable.
4. Keep health footer below results, never above `Best match`.
5. Add bounded usage learning:
   - `recordKey`
   - selection count
   - last-used timestamp
6. Feed usage into ranking as a tie/near-tie boost only.
7. Add local context sources:
   - pinned stack folders
   - recent folders
   - known shell folders
   - Quick Access-style roots when safely available
8. Add optional scopes/prefixes:
   - `app`
   - `folder`
   - `file`
   - `settings`
   - `command`
9. Keep default global search strong; scopes are optional power-user controls.

Acceptance targets:

- First paint does not wait for icons.
- Provider degradation is explainable without hiding results.
- Repeated user choices improve future tie ordering.
- Usage never beats exact stronger app/settings/folder intent.
- Scoped search narrows noise without changing default global behavior.

Validation commands:

```powershell
npx tsc -p tsconfig.test.json
node --test tests\searchUxState.test.mjs tests\searchPanelState.test.mjs tests\searchEngineContracts.test.mjs
cargo test --manifest-path src-tauri\Cargo.toml search::
```

Exit gate: polish improves scanability and personalization after core gates pass.

## Final QA Gate

Owner: QA/adversarial reviewer subagent

Skills: `adversarial-reviewer`, `tdd-guide`

Required review personas:

- Saboteur: stale progress races, slow Everything, corrupt app cache, provider failures, sequence mismatches.
- New Hire: display model clarity, phase boundaries, test readability, provider contract discoverability.
- Security Auditor: `ms-settings:*`, `control.exe`, path actions, provider health/setup actions, persisted cache content.

Blocking criteria:

- Any stale result can overwrite a newer query.
- Any exact settings/app/folder intent is visually buried.
- Any query-time provider path can block input echo.
- Any broad filesystem/app scan runs on every keystroke.
- Any unsafe action bypasses validation.

Final validation commands:

```powershell
npx tsc -p tsconfig.test.json
node --test tests\searchUxState.test.mjs tests\searchPanelState.test.mjs tests\searchEngineContracts.test.mjs tests\searchOverhaulPhase0.test.mjs
cargo test --manifest-path src-tauri\Cargo.toml search::
cargo check --manifest-path src-tauri\Cargo.toml
npm run validate
```

Live smoke gate:

1. Start JasonShell.
2. Type rapid sequences: `s`, `sp`, `spo`, `spotify`.
3. Verify input echo has no visible lag.
4. Verify `Best match` first row is correct.
5. Verify ArrowUp/ArrowDown follow visual order.
6. Verify Enter activates selected visible row.
7. Verify `jnev1` does not blank panel while Everything rows load.
8. Verify degraded provider footer does not cover results.
9. Run full query matrix manually.

Performance budgets:

- Input echo sync work: under 8 ms.
- First visible payload: under 50 ms.
- Warm local intent rows: under 10 ms.
- Typical Everything provider response: under 150 ms.
- No UI freezing while typing.

## Phase Dependency Graph

Implementation order:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Phase 5
7. Phase 6
8. Phase 7
9. Final QA gate

Reasoning:

- Phase 1 fixes visible accuracy before deeper backend work.
- Phase 2 fixes perceived performance and makes later provider work safer.
- Phase 3 fixes app trust and gives fuzzy matching a reliable app corpus.
- Phase 4 improves launcher-quality matching after core display/progress are stable.
- Phase 5 improves Everything recall once progressive provider handling exists.
- Phase 6 broadens settings after fuzzy/highlight data shape is available.
- Phase 7 adds polish, personalization, and power-user controls last.

## Explicit Assumptions

- Current clean `search_engine` path remains canonical.
- `master_spec.md` remains durable source of truth.
- `search_improvements.md` remains audit input, not implementation status.
- App-cache persistence stores metadata only, not secrets.
- Everything machine/runtime availability must be revalidated during implementation.
- Live Tauri/WebView smoke remains required before declaring search UX fully validated.
