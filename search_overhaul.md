# JasonShell Search Overhaul Plan

Status: Draft plan, no implementation started
Date: 2026-04-29
Owner: future implementation agents
Source request: rewrite JasonShell search so Everything filesystem results remain fast and complete while Windows-like system intents such as Display Settings, Sound Settings, Windows Settings, Control Panel, apps, folders, open windows, commands, and developer entries all appear in the right order without legacy search paths interfering.

## 1. Problem Statement

Current search behavior is still an accumulation of earlier attempts:

- TypeScript builds a mixed local catalog in `src/lib/searchCatalog.ts`.
- TypeScript ranks visible results in `src/lib/searchRanking.ts`.
- Rust `search_system` calls `search_sources/index.rs`, which calls provider code and also retains warmed-cache/index responsibilities.
- Rust provider code still has Windows Search app supplementation and warmed-cache/index-era concepts.
- Everything is expected to be the fast filesystem lane, but visible search now also needs Windows-like system-control results such as Display Settings and Sound Settings that Everything will not naturally return.
- Some result classes are static and too small. `Windows Settings` and `Control Panel` exist, but specific settings pages such as Display Settings and Sound Settings do not.
- Some user queries that should hit folders, such as `jnev1`, may be lost or delayed when the mixed local/Everything pipeline clears or waits for async provider results.
- Result latency feels slower than standalone Everything Search, which suggests extra detection, orchestration, ranking, duplicate collapse, panel publishing, or stale legacy paths are on the critical path.

The overhaul must replace this with a clear architecture: one coordinator, explicit providers, one scoring model, one visible payload flow, and no broad filesystem scans or legacy fallback work on the keystroke path.

## 2. Goals

G1. Search MUST show fast filesystem results from Everything for normal file/folder queries such as `jnev1`.

G2. Search MUST show Windows-like settings intents for common settings queries, including at minimum:

- `settings`
- `windows settings`
- `display settings`
- `sound settings`
- `audio settings`
- `control panel`
- `network settings`
- `bluetooth settings`
- `privacy settings`
- `update settings`
- `apps settings`
- `power settings`

G3. Search MUST show applications before incidental file/folder matches for application-launch queries.

G4. Search MUST preserve immediate typed-character echo; provider work, scoring, metadata refresh, and panel native show work MUST NOT block input rendering.

G5. Everything provider latency SHOULD be close to direct Everything Search for simple filename/folder queries.

G6. Search implementation MUST remove or isolate legacy paths that can interfere with visible results, especially warmed-cache fallback display, Windows Search fallback display, duplicated TS/Rust ranking, and per-query app/filesystem rescans.

G7. Search MUST have focused tests proving visible result ordering and latency-safe state behavior for the problem queries named by the user.

## 3. Non-Goals

NG1. Do not replace Voidtools Everything itself.

NG2. Do not implement content search for realtime typing. Content search stays off the hot path.

NG3. Do not use Windows Search/SystemIndex as a visible fallback in the primary search surface. It can remain only as an explicit optional provider after architecture review, not as hidden legacy interference.

NG4. Do not broaden per-keystroke filesystem enumeration. File/folder search must come from Everything or bounded cached metadata.

NG5. Do not change unrelated shell surfaces except where search activation opens settings paths or folders.

## 4. Target Architecture

### 4.1 Single Search Coordinator

Create a single search coordinator contract that owns query lifecycle and visible payload production.

Proposed Rust module shape:

- `src-tauri/src/search.rs` or `src-tauri/src/search_engine.rs`: public Tauri command and engine coordinator.
- `src-tauri/src/search/providers/everything.rs`: Everything filesystem provider.
- `src-tauri/src/search/providers/apps.rs`: app/start-menu provider backed by a warmed app index.
- `src-tauri/src/search/providers/settings.rs`: Windows settings/control-panel provider backed by static data.
- `src-tauri/src/search/providers/open_windows.rs`: optional frontend-supplied or Rust-supplied open-window provider.
- `src-tauri/src/search/scoring.rs`: one shared scoring model for Rust-owned rows.
- `src-tauri/src/search/contracts.rs`: result, provider, health, timing, and diagnostics contracts.

Proposed TypeScript module shape:

- `src/lib/searchEngine.ts`: frontend wrapper around the coordinator command.
- `src/lib/searchPanel.ts`: panel payload/event contracts only.
- `src/features/search/searchUxState.ts`: grouping, keyboard, local UI state only.
- Remove or shrink `src/lib/searchCatalog.ts` and `src/lib/searchRanking.ts` after Rust coordinator owns canonical scoring.

### 4.2 Provider Lanes

Provider lanes must be explicit and independently measurable:

1. Instant local intents
   - Settings/control panel intents.
   - Shell folders.
   - Commands.
   - Cached app index.
   - Open windows, if available without slow native work.
   - Must return synchronously or from in-memory cache.

2. Everything filesystem lane
   - Files, folders, volumes, app-like shortcuts.
   - Must reuse detected SDK path and avoid repeated setup/detection on every query.
   - Must use bounded result limits and direct SDK calls.
   - Must report provider duration for diagnostics.

3. Optional deferred enrichment lane
   - Icons, usage metadata, extra details.
   - Must not block first visible rows.
   - Can update rows later with the same stable IDs.

Windows Search/SystemIndex and warmed file cache should not participate in normal visible result production unless a later approved phase adds them back behind explicit settings and tests.

### 4.3 Settings Provider

Add a real settings provider, not ad hoc static rows. It should be a curated dataset similar in spirit to Flow Launcher's Windows Settings plugin:

- Each setting row has `id`, `title`, `subtitle`, `kind`, `path`, `terms`, `category`, `priority`, `aliases`, and optional `controlPanelApplet`.
- `path` should be launchable through `openShellPath` or native ShellExecute:
  - Display Settings: `ms-settings:display`
  - Sound Settings: `ms-settings:sound`
  - Windows Settings: `ms-settings:`
  - Control Panel: `control.exe`
  - Network Settings: `ms-settings:network`
  - Bluetooth Settings: `ms-settings:bluetooth`
  - Apps Settings: `ms-settings:appsfeatures`
  - Privacy Settings: `ms-settings:privacy`
  - Windows Update: `ms-settings:windowsupdate`
  - Power and Sleep: `ms-settings:powersleep`
- Query matching must support common user language: `display settings`, `screen settings`, `monitor settings`, `sound settings`, `audio settings`, `volume settings`, `control panel`, `classic settings`.

Acceptance expectation: `display settings` returns Display Settings before folders/files named display; `sound settings` returns Sound Settings before audio folders.

### 4.4 Everything Provider Hot Path

Everything hot path must avoid repeated expensive work:

- Detect approved SDK DLL once at startup or first use, then cache the path and health state with TTL.
- Do not call full installation detection on every query unless health TTL expired.
- Keep Everything SDK lock narrow.
- Do not reset/reload DLL more than necessary if the SDK API requires reset only after query state.
- Measure duration from frontend query dispatch to Rust result return.
- Return first result batch directly; do not wait for apps/settings ranking work if those rows are already available.

Acceptance expectation: simple query such as `jnev1` produces an Everything folder/file batch without waiting for Windows Search, warmed cache, app rescans, or panel re-show.

### 4.5 Scoring Model

Use one canonical scoring model. Recommended order:

1. Hard filters:
   - Empty query returns no rows unless search home is explicitly designed later.
   - Result must match query tokens or provider-specific exact alias.
   - Provider/type boosts must not make non-matches visible.

2. Intent classification:
   - App launch intent.
   - Settings/system-control intent.
   - Folder/file intent.
   - Window focus intent.
   - Command/developer intent.

3. Field scoring:
   - Exact title/alias match.
   - Prefix title/alias match.
   - Acronym/fuzzy title match.
   - Terms/category match.
   - Path match.

4. Provider/type priority:
   - Settings rows win settings intents.
   - Apps win launch intents.
   - Everything folders/files win filesystem intents.
   - Open windows win active-window intents.

5. Usage/run-count boosts:
   - Bounded and never enough to override a stronger exact intent from another lane.

6. Stable tie-breakers:
   - Score desc.
   - Provider lane order.
   - Title asc.
   - Stable ID asc.

Score metadata should be available in tests and optional diagnostics, but not necessarily shown in UI.

### 4.6 Visible Payload Flow

Frontend should have a thin lifecycle:

1. Input event updates `searchQuery` immediately.
2. Frontend publishes a cheap pending payload if panel is open.
3. Frontend sends latest query to Rust coordinator with a sequence/request ID.
4. Rust returns ranked visible rows and timing metadata.
5. Frontend applies only latest response.
6. `search-panel` receives one canonical payload.

Remove duplicate local ranking after Rust ranking unless a tiny UI-only grouping step needs it. If TypeScript must keep scoring temporarily during migration, it must be explicitly transitional and deleted in a later phase.

## 5. Phased Work Plan

### Phase 0: Baseline Audit and Red Tests

Objective: prove current failures before rewrite.

Tasks:

- Add focused tests for problem queries:
  - `display settings`
  - `sound settings`
  - `jnev1`
  - `spotify`
  - `control panel`
  - `windows settings`
- Add latency/state tests proving input echo and latest-response gating.
- Add diagnostics hooks or test seams to assert no Windows Search/warmed-cache visible fallback for normal queries.
- Record current failing behavior in test names and comments.

Deliverables:

- Red tests in `tests/searchRanking.test.mjs`, `tests/searchPanelState.test.mjs`, and Rust search tests.
- A small query fixture table for expected top results.
- No production behavior changes except test seams if necessary.

Validation:

- Focused tests must fail for the documented current gaps before implementation.

### Phase 1: Define New Contracts

Objective: freeze result and provider contracts before moving code.

Tasks:

- Define `SearchQueryRequest` with `query`, `sequence`, `limit`, `presentation`, and optional context such as open windows/workspace.
- Define `SearchEngineResponse` with `query`, `sequence`, `results`, `providerTimings`, `health`, `generatedAt`, and optional `diagnostics`.
- Define `SearchResult` fields: `id`, `providerId`, `kind`, `title`, `subtitle`, `path`, `action`, `terms`, `aliases`, `score`, `matchReason`, `recordKey`, `iconDataUrl`.
- Decide which context is Rust-owned vs frontend-owned:
  - Rust-owned: Everything, settings, apps, shell folders, commands if possible.
  - Frontend-owned only if needed: currently open windows, workspace tasks.
- Update TypeScript validators and Rust serde contracts together.

Deliverables:

- Contract definitions and tests.
- Compatibility adapter only if needed to avoid one giant break.

Validation:

- `npx tsc -p tsconfig.test.json`
- Contract tests for schema shape and result action safety.

### Phase 2: Settings Provider Dataset

Objective: restore Windows-like settings results that Everything cannot provide.

Tasks:

- Add `settings` provider in Rust or a shared static TS/Rust JSON fixture.
- Include at least the required settings rows listed in section 4.3.
- Add alias expansion and category-aware matching.
- Ensure activation opens `ms-settings:*` and `control.exe` safely.
- Avoid calling Windows Search for settings rows.

Deliverables:

- Settings provider module/data.
- Tests for `display settings`, `sound settings`, `control panel`, and aliases.

Validation:

- Rust unit tests for settings provider matching.
- Node activation/contract tests for settings actions.

### Phase 3: Everything Provider Fast Path

Objective: make filesystem search feel like Everything Search.

Tasks:

- Cache Everything SDK detection and provider health with an explicit TTL.
- Avoid install/service/process probes on each keystroke.
- Reuse provider state where safe.
- Add provider timing metadata.
- Remove any wait on Windows Search/warmed cache/app scans before Everything returns.
- Verify one-character and normal folder queries still work.

Deliverables:

- Faster Everything provider path.
- Provider timing tests or diagnostics assertions.

Validation:

- Rust tests for cached SDK health and query path.
- Manual or automated timing log for `jnev1`, `exe`, `display`, `spotify`.

### Phase 4: App and Local Intent Index

Objective: keep app/system intent immediate without scanning each query.

Tasks:

- Build app/start-menu index once on startup and refresh by TTL or explicit refresh command.
- Move shell folders and command rows into a bounded local provider.
- Remove per-query Start Menu scanning.
- Ensure apps beat incidental Everything folders for app intent.

Deliverables:

- App/local provider with in-memory index.
- Refresh command/event retained only if useful.

Validation:

- Rust tests for app cache freshness and matching.
- Query tests for Spotify-style app result ordering.

### Phase 5: One Coordinator and One Ranking Path

Objective: delete duplicated ranking/collision logic.

Tasks:

- Implement coordinator that gathers provider rows, scores once, collapses duplicates once, and returns ranked rows.
- Migrate canonical scoring to Rust or a shared pure module; preferred Rust because providers live there and Tauri returns already-ranked rows.
- Reduce TypeScript ranking to grouping only, or remove it.
- Remove legacy `search_sources/index.rs` visible fallback behavior from the hot path.
- Keep warmed cache only if it has a specific non-visible startup role; otherwise delete or park it.
- Keep Windows Search only if used by a clearly named optional diagnostic/provider mode; otherwise remove from visible path.

Deliverables:

- `search_system` returns final visible ranking.
- TS no longer merges/reranks results in a way that can reorder or hide Rust output.

Validation:

- Tests proving provider/type boosts cannot surface non-matches.
- Tests proving setting/app/folder intent each wins its own query type.

### Phase 6: Frontend Query Pipeline Rewrite

Objective: make UI path simple and realtime.

Tasks:

- Replace current `TopBar.svelte` search timers with a small latest-only query controller.
- Keep input echo synchronous.
- Open centered panel only when closed or presentation changes.
- Publish pending payload immediately, then apply latest Rust response.
- Ensure stale responses cannot overwrite newer payloads.
- Keep result activation, pin folder, keyboard navigation, and close behavior unchanged.

Deliverables:

- Simpler `TopBar.svelte` search block.
- `SearchPanelSurface.svelte` still render-only.
- Tests for input -> pending -> latest response -> activation.

Validation:

- Node state tests for latest-only behavior and panel payload signatures.
- Svelte check.

### Phase 7: Remove Legacy Remnants

Objective: ensure old functionality cannot interfere.

Tasks:

- Delete or quarantine:
  - warmed-cache display fallback from normal search.
  - Windows Search fallback display from normal search.
  - duplicate TypeScript ranking if Rust ranking is canonical.
  - old `searchCatalog.ts` static rows replaced by provider rows.
  - old `search_sources/files.rs` broad filesystem cache if unused.
- Update `master_spec.md` and tests to match the new truth.
- Keep compatibility aliases only with deprecation comments and tests.

Deliverables:

- Smaller search module graph.
- Search behavior documented in `master_spec.md`.

Validation:

- `rg` checks proving removed legacy entry points are not imported by the hot path.
- Full `npm run validate`.

### Phase 8: QA, Live Smoke, and Performance Budget

Objective: prove rewrite meets user-visible behavior.

Tasks:

- Run adversarial QA against the new search diff.
- Run full validation.
- Run live search smoke in Tauri:
  - `display settings`
  - `sound settings`
  - `jnev1`
  - `spotify`
  - `control panel`
  - `downloads`
  - a one-character query
- Capture timing diagnostics:
  - input echo immediate.
  - first visible local intent batch under 50 ms after input in renderer.
  - Everything provider response target under 150 ms for typical indexed filename/folder queries on this machine, measured from Rust command start to response.
  - no query should block UI typing.

Deliverables:

- QA report.
- Validation results.
- Performance notes in `master_spec.md`.

## 6. Acceptance Criteria

AC1. Given query `display settings`, when search results render, then Display Settings is visible in top results and ranks above Everything folders/files named display.

AC2. Given query `sound settings`, when search results render, then Sound Settings is visible in top results and ranks above audio/sound folders/files.

AC3. Given query `jnev1`, when Everything is ready, then filesystem results containing `jnev1` appear without waiting for Windows Search or warmed-cache fallback.

AC4. Given query `spotify`, when Spotify is installed or indexed, then Spotify app ranks above incidental Spotify folders/files.

AC5. Given query `control panel`, then Control Panel ranks above incidental files/folders and launches `control.exe`.

AC6. Given rapid typing, when older provider responses return after newer query responses, then older responses are ignored.

AC7. Given the search panel is already centered/open, typing another character must not call native panel show again.

AC8. Given Everything SDK is ready, query handling must not re-run full SDK/install detection on every keystroke.

AC9. Given a non-matching row with high provider/type priority, ranking must not show it for the query.

AC10. Given full validation, `npm run validate` must pass before implementation is called complete.

## 7. Test Plan

Rust tests:

- Settings provider query/alias matching.
- Everything SDK detection cache and health TTL.
- Coordinator stale sequence and provider merge behavior.
- Canonical scoring for app/settings/folder intents.
- Duplicate collapse preference by provider, action, and stable path.

Node/TypeScript tests:

- Search panel payload shape and sequence gating.
- TopBar latest-only query controller.
- Activation routing for `ms-settings:*`, `control.exe`, files, folders, apps, windows.
- Search result grouping preserving ranked order.
- Source tests proving legacy fallback imports are absent from hot path.

Live smoke:

- Run Tauri dev build and manually verify target queries.
- Compare rough latency against standalone Everything Search for `jnev1`.
- Verify settings results launch expected Windows pages.

## 8. Implementation Ownership Split

Recommended skilled subagent split for implementation:

- Rust/search backend worker using `rust-skills`: contracts, providers, coordinator, scoring, Everything fast path.
- Frontend worker using `senior-frontend`: TopBar query pipeline, SearchPanel render contract, activation wiring.
- TDD worker using `tdd-guide`: red tests, fixtures, focused acceptance coverage.
- QA worker using `adversarial-reviewer`: final diff review, legacy-remnant search, performance-risk review.

Do not let multiple workers edit the same files at the same time unless their write scopes are explicitly split.

## 9. Risks

R1. Moving all scoring to Rust may require passing frontend-only open-window/workspace context into Rust. Mitigation: add request context or keep a temporary local-intent adapter.

R2. `ms-settings:*` URI availability can vary across Windows versions. Mitigation: keep tests contract-level and live-smoke actual pages.

R3. Everything SDK locking/reuse can be fragile. Mitigation: keep the provider trait mockable and test reset/cache behavior.

R4. Removing warmed-cache/Windows Search paths may remove fallback results when Everything is unavailable. Mitigation: make this an explicit product decision; current user request prioritizes removing interference and matching Everything speed.

R5. Large dirty worktree increases merge risk. Mitigation: inspect diffs before editing, keep file ownership narrow, avoid resets.

## 10. Open Questions Before Implementation

OQ1. Should Windows Search be fully removed from visible search, or kept behind a disabled-by-default setting for future optional use?

OQ2. Should settings provider data live in Rust code, a JSON fixture, or TypeScript shared data with generated Rust constants?

OQ3. Should open-window results be sent to Rust for ranking, or should frontend merge them after Rust ranking with strict ordering rules?

OQ4. What measured latency target should be considered acceptable compared with standalone Everything Search on this machine?

Recommended defaults if no answer is provided:

- Remove Windows Search from normal visible search.
- Store settings provider data in Rust with tests first; move to JSON only when row count grows.
- Pass open-window context into the Rust coordinator later; keep it frontend-only during first migration if needed to reduce blast radius.
- Use 150 ms Rust provider-response target for typical Everything queries and 50 ms for immediate local-intent rows.
