# Search Functionality Implementation Plan

## 1. Metadata, status, owners, reviewers, dependencies

- **Document ID:** SEARCH-PLAN-2026-08-19
- **Status:** Core implementation complete (Phases 0-5); Phase 6 deferred.
- **Implementation authorization:** Phases 0-5 were owner-approved and implemented. Phase 6 remains optional, approval-gated, and unauthorized.
- **Repo:** `C:\dev\jasonshell`
- **Author role:** documentation-only writer
- **Source authority:** `master_spec.md` current behavior; `docs/search-functionality-investigation.md` findings; live inspected files/tests.
- **Product code impact from this document:** Phases 0-5 implemented; see `master_spec.md`, `changelog.md`, and phase artifacts for current behavior and evidence.
- **Owner:** Repository owner
- **Required reviewers:** search/frontend owner, Rust/search owner, QA owner, accessibility reviewer, privacy reviewer for Phase 6+
- **Dependencies:** current `search_engine` command, `search-panel` webview, Everything SDK availability, app-index cache, existing Node/Rust search test suites.
- **Durable-doc rule:** During actual implementation only, `master_spec.md` MUST be updated when behavior, commands, events, persistence, validation coverage, or known risks change. `changelog.md` MUST follow `CHANGELOG_POLICY.md`. This planning task does not change product behavior.

## Context

### Goal

Make search faster and better for short prefixes and rapid typing while preserving existing user-visible behavior, event names, command names, activation safety, panel ownership, and progressive result semantics.

### Definition of done

Core completion requires Phases 0-5 complete with approved evidence. Phase 6 is optional, approval-gated P1 expansion and is not part of core completion.

- **DoD-1:** Short-prefix settings/app/folder/file cases have deterministic characterization and regression tests.
- **DoD-2:** Provider signal is bounded, measured, and approved; it MUST NOT let raw provider score dominate canonical intent rank.
- **DoD-3:** Stale backend expensive-provider work is reduced in deterministic rapid-prefix scenario; candidate target is >=80% stale expensive-provider call reduction, but final accepted budget MUST be approved after Phase 0 baseline.
- **DoD-4:** Immediate input echo and pending payload behavior remain per current spec.
- **DoD-5:** Progressive local-first then provider-complete behavior remains.
- **DoD-6:** Existing activation, close/reset, stale query/sequence gates, event names, command names, and payload ownership remain unchanged unless separately approved.
- **DoD-7:** Required tests and perf artifacts exist for every completed phase.
- **DoD-8:** No phase MAY be marked complete if regression tests or perf evidence for touched behavior is missing.

### Canonical current behavior and non-regression invariants

### Current behavior facts

- **INV-CUR-1:** Search input is owned by `top-bar`; rich rows render in dedicated `search-panel` webview.
- **INV-CUR-2:** Default mode opens centered search through `show_centered_search_panel`; explicit top-right mode remains supported.
- **INV-CUR-3:** TopBar publishes pending `SearchPanelPayload` immediately for every non-empty typed query, including one-character queries.
- **INV-CUR-4:** Current behavior is not debounce-coalesced; each exact query/sequence starts async `search_engine` work.
- **INV-CUR-5:** Pending payloads clear old rows when normalized query changes; pending payloads MAY keep rows only for unchanged normalized query.
- **INV-CUR-6:** Stale responses are rejected by normalized query plus sequence gates; `search_panel.rs` also rejects stale sequenced payloads.
- **INV-CUR-7:** Rust `search_engine` gathers settings, apps, local folders/commands, open-window context, and Everything rows.
- **INV-CUR-8:** Rust emits progressive ranked snapshots: local/provider/complete. Snapshots replace working rows, not merge as deltas.
- **INV-CUR-9:** `search/scoring.rs` is canonical visible ranker and duplicate-collapser.
- **INV-CUR-10:** Production hot path MUST NOT use old `search_system`, warmed-cache display fallback, Windows Search display fallback, `buildSearchCatalog`, or `rankSearchResults`.
- **INV-CUR-11:** Everything is runtime-only and is authoritative broad filesystem provider when healthy; if unavailable, other providers plus health diagnostics determine visible typed results.
- **INV-CUR-12:** Result kinds remain `app`, `window`, `folder`, `file`, `command`, `setting`, `calculator`, `web`, `bookmark`.
- **INV-CUR-13:** Activation path remains TopBar-owned: row intent -> `activateResult()` -> usage recording -> safe action branch -> clear query -> close panel -> reload local catalog.
- **INV-CUR-14:** Existing action safety MUST remain for live IDs and branches: `setting:*`, `app:*`, `window:*`, `command:*`, `everything:{kind}:*`, local folder/file result IDs, and typed safe action arguments. Implementation MUST inspect current `activateResult()` and Rust contracts before freezing a complete ID table; it MUST NOT invent a generic `system:*` convention.

### Non-regression invariants

- **INV-NR-1:** Input echo and centered-panel local draft MUST remain immediate.
- **INV-NR-2:** Pending payload MUST remain immediate. Backend coalescing/cutoff MUST NOT hide typed chars.
- **INV-NR-3:** Panel MUST remain render-only for result rows and row intents; TopBar remains owner of query, selected index, activation.
- **INV-NR-4:** Panel open/show/publish MUST remain idempotent and sequence-gated.
- **INV-NR-5:** Query/sequence stale gates MUST remain at frontend and panel payload layers.
- **INV-NR-6:** Local results SHOULD continue to appear before slow provider completion.
- **INV-NR-7:** No broad recursive filesystem scan MAY be added per keypress.
- **INV-NR-8:** No Quick Command body/transcript, terminal history, browser history, or content search source MAY be enabled by default.
- **INV-NR-9:** Existing command/event names MUST stay unchanged unless approval gate explicitly allows contract change.
- **INV-NR-10:** No absolute latency pass threshold is valid until Phase 0 measures baseline and owner approves budgets.

## Out of Scope

### In scope

- **SCOPE-1:** Short-prefix matching fixes for settings and existing provider corpus.
- **SCOPE-2:** Bounded provider signal in canonical rank.
- **SCOPE-3:** Latest-only stale backend cutoffs at provider boundaries, especially before Everything.
- **SCOPE-4:** Everything overfetch use before canonical rerank.
- **SCOPE-5:** Visible rank order/a11y/provider health improvements in separated P1 subphases.
- **SCOPE-6:** Low-risk provider expansion from user-curated labels/descriptions/paths only, approval-gated.
- **SCOPE-7:** Deterministic benchmark/test affordances, preferably `#[cfg(test)]` or pure coordinator extraction.

### Explicit exclusions

- OS-1: No implementation before approval.
- OS-2: No production dynamic plugin architecture for fake providers.
- OS-3: No frontend debounce mandate. Phase 3 recommends backend latest-only cutoffs while UI remains immediate.
- OS-4: No default content search.
- OS-5: No default browser history, terminal history, Quick Command body/transcript, or unrestricted private sources.
- OS-6: No permanent runtime rollback flags unless live rollout risk warrants them.
- OS-7: No broad Windows Search fallback in core P0/P1.
- OS-8: No fabricated latency targets or metrics.

## Functional Requirements

- FR-1: System MUST preserve immediate query echo and pending panel payload for every accepted input event.
- FR-2: System MUST preserve TopBar ownership of query, selected index, activation, close/reset, and stale gates.
- FR-3: Settings provider MUST support approved short prefixes such as `w`, `wi`, `windows s`, `control p`, and `display s` without catalog flooding.
- FR-4: Search ranking MUST preserve canonical intent ranking while allowing bounded provider signal.
- FR-5: Provider signal cap MUST be evidence-based and approval-gated; initial experiment MAY evaluate `0..300`, but final cap MUST NOT be hardcoded as final without corpus evidence.
- FR-6: Backend coordination MUST skip expensive stale provider work when current latest sequence/query no longer matches.
- FR-7: Backend latest-only mechanism MUST be proven by deterministic fake-provider harness or pure coordinator test.
- FR-8: Implementation MUST NOT claim true cancellation of in-flight Tauri invoke Promises.
- FR-9: Everything provider MUST allow overfetched raw rows to reach canonical rerank before final truncation.
- FR-10: Visible top page order SHOULD be strict canonical rank; any grouped tail behavior MUST be approval-gated.
- FR-11: Search panel accessibility MUST use one coherent combobox/listbox focus model.
- FR-12: Provider health/status MUST be visible without hiding existing results.
- FR-13: New providers in Phase 6 MUST use only user-curated labels/descriptions/paths already present in app state/settings.
- FR-14: Existing event/command names MUST remain unchanged unless approval explicitly authorizes migration.

## Non-Functional Requirements

- **NFR-PERF-1:** Phase 0 MUST establish deterministic and live baseline artifacts before setting absolute latency budgets.
- **NFR-PERF-2:** P0 implementation MUST demonstrate baseline-relative non-regression for p50/p95 input-to-local, input-to-final, renderer cost, and queue wait.
- **NFR-PERF-3:** P0 implementation SHOULD target >=80% stale expensive-provider call reduction in deterministic rapid-prefix scenario, subject to Phase 0 approval.
- **NFR-A11Y-1:** Keyboard navigation MUST remain available for search rows.
- **NFR-A11Y-2:** Accessible names, roles, active descendant, and status announcements MUST be source-tested for changed markup.
- **NFR-PRIV-1:** New sources MUST NOT include sensitive histories or unbounded content by default.
- **NFR-PRIV-2:** Diagnostics and perf artifacts MUST NOT contain secret-like command text, transcripts, browser history, or file contents.
- **NFR-REL-1:** Stale gate failures MUST fail safe by ignoring older payloads, not by applying stale rows.
- **NFR-REL-2:** Everything unavailable/degraded MUST not break settings/apps/local/window results.
- **NFR-MAINT-1:** Test affordances MUST avoid production plugin complexity.

## API Contracts

### Unchanged contracts

- **CON-U-1:** Tauri command `search_engine` remains production search command.
- **CON-U-2:** Search panel commands remain `show_search_panel`, `show_centered_search_panel`, `resize_search_panel`, `hide_search_panel`, `publish_search_panel`, `get_search_panel_payload`.
- **CON-U-3:** Events remain exactly `search-engine:progress`, `search-index:refreshed`, `search-panel:update`, `search-panel:query`, `search-panel:key`, `search-panel:select`, `search-panel:activate`, `search-panel:pin-folder`, `search-panel:expand-group`, `search-panel:interaction`, `search-panel:closed`, and `search:toggle-centered`.
- **CON-U-4:** Result activation retains inspected live conventions: `setting:*`, `app:*`, `window:*`, `command:*`, `everything:{kind}:*`, and current local folder/file IDs. Exact dispatch branch, action ID, path, and arguments MUST be characterized before implementation.
- **CON-U-5:** `SearchPanelPayload.sequence` stale rejection remains.
- **CON-U-6:** Existing response/result/action shapes MUST remain backward-compatible.

### Proposed internal contract additions

- **CON-P-1:** Add internal coordinator latest-state check, scoped to backend search module or pure coordinator, not public API unless needed.
- **CON-P-2:** Current responses already contain `providerTimings`, `health`, and `diagnostics`. Add fields only when existing telemetry cannot express required stale/failure state, compatibility tests pass, and `master_spec.md` records any contract change.
- **CON-P-3:** Add bounded provider signal field or derived normalized signal inside `SearchResult`/scoring path only with compatibility tests.
- **CON-P-4:** Add test-only fake provider seam via `#[cfg(test)]` or pure coordinator extraction.
- **CON-P-5:** Prefer deriving status UI from current health/timing/diagnostic fields. Add payload fields only if unavailable/indexing/partial state cannot be represented safely.

## Data Models

No persistent data model or migration is planned. Existing transient contracts remain authoritative.

No HTTP endpoint exists for search; this is Tauri IPC, so HTTP method/path validation is not applicable. Existing interface-equivalent Rust/TypeScript contracts remain authoritative rather than duplicating potentially stale interfaces here.

| Model | Existing fields relevant to plan | Constraint |
|---|---|---|
| `SearchEngineRequest` | query plus frontend freshness sequence as currently wired | Latest ownership key is `{normalizedQuery, sequence}`; a newer sequence owns results even when normalized text is unchanged. |
| `SearchEngineResponse` | results, provider timings, health, diagnostics | Existing fields remain compatible; stale work never becomes visible current payload. |
| `SearchPanelPayload` | query, sequence, rows/progress state | Frontend and panel query/sequence rejection remains mandatory. |
| `SearchResult` / `SearchPanelResult` | id, kind, score, action/path/args, source metadata | IDs and safe activation args remain compatible; provider signal is bounded internal ranking input. |
| Baseline artifact | metadata, raw run samples, derived percentiles, quality metrics | Stored under ignored `test-results/search-performance/<timestamp>/`; user data redacted or hashed. |

## 7. Decision log, recommended defaults, approval gates

| ID | Decision | Recommended default | Gate |
|---|---|---|---|
| DEC-1 | Input responsiveness model | Keep immediate input echo + immediate pending payload. Do not mandate frontend debounce. | None; invariant. |
| DEC-2 | Stale work strategy | Backend latest sequence/query atomic check at provider boundaries, especially before Everything. | Phase 3 design review + fake-provider proof. |
| DEC-3 | Tauri cancellation claim | Do not claim true cancellation from invoke Promise. Async commands run as tasks; heavy work stays async/blocking-boundary safe. | None; invariant. |
| DEC-4 | Provider score signal | Experiment bounded additive signal. Start 0..300, lock only after corpus evidence. | Phase 2 ranking review. |
| DEC-5 | Visible order | Top visible page strict canonical rank; grouped tail only after approved separator. | Phase 5A approval. |
| DEC-6 | Rollback mechanism | Prefer small isolated commits and revert. Runtime flag only when live rollout requires disabling without rollback. | Per-phase rollout risk review. |
| DEC-7 | Provider expansion | Only labels/descriptions/paths user curated; no sensitive histories/default content. | Phase 6 privacy approval. |
| DEC-8 | Master spec/changelog | Not changed for plan; update during implementation when behavior/contracts/tests/risks change. | Implementation PR checklist. |

### Blocking approvals

- **BLOCK-1:** No Phase 1+ code until owner approves Phase 0 benchmark protocol and method for deriving latency budgets.
- **BLOCK-2:** No Phase 2 code until owner approves provider-signal experiment and cap-selection method; `0..300` is not pre-approved production behavior.
- **BLOCK-3:** No Phase 3 code until owner approves stale-reduction formula and target.
- **BLOCK-4:** No Phase 5A code until owner approves strict top-page order; no Phase 5C code until owner approves health wording/placement.
- **BLOCK-5:** No Phase 6 code until source list, privacy classification, data caps, and optional/non-core status are approved.

## 8. Work graph, dependencies, commit slicing, rollback

### Work graph

1. **Phase 0:** Baseline + characterization + deterministic affordance.
2. **Phase 1:** Settings short prefixes.
3. **Phase 2:** Bounded provider signal.
4. **Phase 3:** Backend latest-only stale provider cutoffs.
5. **Phase 4:** Everything overfetch rerank.
6. **Phase 5A:** Visible rank order.
7. **Phase 5B:** Accessibility.
8. **Phase 5C:** Provider health/status.
9. **Phase 6:** Low-risk provider expansion.

### Commit slicing

- **COMMIT-0A:** Baseline tests/artifacts only.
- **COMMIT-0B:** Test-only coordinator/fake-provider affordance.
- **COMMIT-1:** Settings short-prefix provider tests + fix.
- **COMMIT-2:** Provider signal ranking tests + bounded impl.
- **COMMIT-3:** Latest-only backend cutoff tests + impl.
- **COMMIT-4:** Everything overfetch tests + impl.
- **COMMIT-5A:** Visible order tests + impl.
- **COMMIT-5B:** A11y tests + markup/state impl.
- **COMMIT-5C:** Provider health tests + impl.
- **COMMIT-6N:** One provider source per commit.

### Rollback strategy

- **RB-1:** Default rollback is revert one isolated commit.
- **RB-2:** Runtime rollback flag MAY be added only if feature cannot be safely reverted during live rollout and behavior can be disabled without contract fork.
- **RB-3:** Any flag MUST be temporary, documented, tested both ways, and removed or justified before release.
- **RB-4:** Rollback evidence MUST include tests proving invariants restored.
- **RB-5:** Phases 0-5 default to revert-only. Runtime flags require owner approval before implementation and tests covering both states.

### Dirty-worktree preflight for every phase

```powershell
git status --short
git diff -- <exact phase files>
```

Touch only approved phase files. If a target file contains concurrent dirty work, stop for owner direction; never overwrite or revert unrelated work.

## Edge Cases

- EC-1: Empty and whitespace-only query preserves current close/reset behavior.
- EC-2: New sequence with same normalized query still owns final apply; current same-query cache retry remains allowed.
- EC-3: Rapid sequences supersede stale work before Everything lock/SDK query.
- EC-4: Everything SDK missing, IPC unavailable, query error, or timeout returns valid partial settings/apps/local/window results and final response.
- EC-5: App cache miss/indexing/refresh/hit preserves bounded retry behavior.
- EC-6: One-character settings input does not flood unrelated rows; path-like `c`, `c:`, and `c:\` do not create settings noise.
- EC-7: Everything raw ordering places best canonical match after requested limit but inside bounded overfetch.
- EC-8: Duplicate local/Everything rows remain deterministic after rerank.
- EC-9: Open-window context may be absent or include Brave, Firefox, terminal, or Settings windows.
- EC-10: Accessibility behavior covers zero rows, partial rows, degraded provider, changing active descendant, and close/reset.
- EC-11: Perf/privacy artifacts contain fixture data or redacted/hashed user-authored labels, descriptions, and paths; never bodies, transcripts, history, or file contents.

## 9. Phase 0 baseline, characterization, deterministic affordance

### Objective

Establish current behavior, quality, stale-work, and perf baseline before changing behavior.

### Prerequisites

- No product behavior changes. A minimal `#[cfg(test)]` seam or pure coordinator extraction may change source structure only after test-affordance approval.
- Dirty worktree preserved.
- Existing tests identified: `centeredSearchSurface`, `searchAppCacheRefresh`, `searchRanking`, `searchTypingFreezePhase1`, `searchSettings`, `searchUxState`, `searchEngineContracts`, `searchPanelState`, `searchOverhaulPhase0`, `searchOverhaulPhase6`, `searchContracts`, `searchCloseReset`, `searchClearButtons`, `performanceBaselineContract`, `masterSpecSearchHygiene`.

### RED tests

- Add passing characterization tests for current behavior. Future-target tests MUST first fail during their owning RED phase, then pass before that phase closes; failing, skipped, or todo target tests remain outside every required close gate and never count as evidence.

### Exact work

- Inspect current search files/tests.
- Add deterministic prefix corpus fixture.
- Add fake-provider seam using `#[cfg(test)]` or pure coordinator extraction. MUST NOT create production dynamic plugin architecture.
- Capture baseline artifacts for Node, Rust, renderer source contract, and live dogfood if package resources allow.
- Add failure fixtures for Everything SDK missing, IPC unavailable, query error, and timeout. Assert local progress plus settings/apps/local/window rows survive, final partial response is valid, and health/timing/diagnostic metadata is truthful.
- Add source/runtime guard proving no broad recursive filesystem scan occurs per keypress.

### Files likely touched during implementation

- `tests/searchSettings.test.mjs`
- `tests/searchRanking.test.mjs`
- `tests/searchUxState.test.mjs`
- `tests/searchEngineContracts.test.mjs`
- `src-tauri/src/search/mod.rs` tests or extracted pure coordinator module tests
- `src-tauri/src/search/providers/*.rs` inline tests

### Edge cases

- Everything unavailable.
- App cache miss/indexing/refresh/hit.
- Rapid prefix `w` -> `wi` -> `win` -> `windows settings`.
- Slow fake Everything blocks stale query.
- Open-window context present/absent.

### Acceptance gate

- Baseline artifacts stored in approved location.
- Owner approves latency/stale budgets after reviewing baseline.

### Rollback

- Revert test-only/baseline commit if it destabilizes CI.

### Evidence artifact

- `test-results/search-performance/<timestamp>/`, already covered by `.gitignore`, containing metadata, raw benchmark JSON, derived tables, command output, and redaction report.

## 10. Phase 1 P0 short-prefix settings

### Objective

Fix useful settings prefixes without broad noise.

### Prerequisites

- Phase 0 prefix corpus exists.

### RED tests

- Rust/provider tests for `w`, `wi`, `windows s`, `control p`, `display s`.
- Node/source contract if frontend assumptions about one-character queries are touched.

### Exact work

- Replace blanket any-token `<2` rejection in settings provider with safe matching policy.
- Permit one-character tokens only when they participate in known alias/token-prefix/acronym match against static settings rows.
- Preserve no-results behavior for empty query.
- Preserve setting actions: `ms-settings:` and `control.exe` safe args.

### Likely files

- `src-tauri/src/search/providers/settings.rs`
- `tests/searchSettings.test.mjs` if source fixtures exist
- inline Rust tests in `settings.rs`

### Edge cases

- Single letter too broad (`a`) MUST NOT flood irrelevant settings unless approved.
- Multi-token with one short token MUST match only plausible rows.
- `*` still routes to Everything per current behavior, not settings flood.

### Acceptance gate

- Required corpus rows appear top 3/top 1 per approved expectations.
- Existing action-safety tests pass.
- Exact close command: focused compile/tests from section 20 for Phase 1, `cargo test --manifest-path src-tauri/Cargo.toml search::providers::settings::`, and prefix before/after artifact. Skipped/todo tests do not count.

### Rollback

- Revert settings provider commit.

### Evidence artifact

- Prefix corpus result table before/after.

## 11. Phase 2 P0 bounded provider signal

### Objective

Preserve provider-specific priority/source/run-count signals without letting provider scores overrule canonical intent.

### Prerequisites

- Phase 0 corpus and ranking baseline.

### RED tests

- Ranking tests where pinned/source-priority app beats lower-quality incidental row.
- Tests where exact settings/app intent still beats high run-count unrelated Everything row.
- Tie-break/dedup tests unchanged.

### Exact work

- Define provider signal normalization.
- Experiment with cap range, starting `0..300` only as experiment.
- Choose final cap after corpus MRR/Recall/top-1 stability evidence.
- Document final cap in implementation PR and update `master_spec.md` if behavior changes.

### Likely files

- `src-tauri/src/search/scoring.rs`
- `src-tauri/src/search/providers/apps.rs`
- `src-tauri/src/search/providers/everything.rs`
- `tests/searchRanking.test.mjs`

### Edge cases

- Everything run count cannot outrank exact app/system-control intent alone.
- Provider signal cannot resurrect nonmatching rows.
- Dedupe order remains deterministic.

### Acceptance gate

- Cap approved with corpus evidence.
- No canonical-intent regression.
- Exact close command: focused compile/tests from section 20 for Phase 2 plus `cargo test --manifest-path src-tauri/Cargo.toml search::scoring::`; ranking artifact records MRR@k, Recall@k, prefix acquisition, and top-1 stability. Skipped/todo tests do not count.

### Rollback

- Revert scoring commit.

### Evidence artifact

- Ranking diff table with MRR@k, Recall@k, prefix acquisition length, top-1 stability.

## 12. Phase 3 P0 latest-only stale backend cutoffs

### Objective

Reduce stale expensive provider work while preserving immediate UI state. Do not mandate frontend debounce.

### Prerequisites

- Deterministic fake-provider seam exists.
- Phase 0 stale baseline exists.

### RED tests

- Rapid-prefix fake-provider test: stale sequence starts, newer sequence supersedes, stale query MUST NOT enter Everything boundary.
- Latest query still completes.
- Immediate pending payload remains per input event.
- No stale response applies even if old backend returns later.

### Exact work

- Add atomic/latest sequence/query registry in backend search coordinator or equivalent.
- Latest key is `{normalizedQuery, sequence}` from TopBar freshness sequence. Newer sequence owns result even for same normalized text or whitespace-only edits. A same-query retry may enter providers only while its retry sequence is current.
- Check latest before expensive provider boundaries, especially immediately before Everything lock acquisition and SDK query. Health-cache checks do not count as Everything query entry.
- Preserve local progress for current query.
- Frontend and panel stale gates remain. Backend stale progress may exist only when tagged/gated so it cannot become visible current progress. Stale work may return benign stale diagnostics but MUST NOT apply rows or a current complete payload.
- Do not claim Tauri invoke Promise cancellation.
- Test lock contention: stale request queued at Everything boundary cannot make latest request wait behind stale SDK work beyond approved budget.
- Repeat failure-isolation fixtures from Phase 0 after coordinator changes.

### Likely files

- `src-tauri/src/search/mod.rs`
- test-only coordinator module or inline tests
- `src/components/TopBar.svelte` only if necessary for wiring; frontend debounce not required
- `tests/searchTypingFreezePhase1.test.mjs`
- `tests/searchUxState.test.mjs`

### Edge cases

- Same normalized query with different whitespace.
- Same query freshness retry.
- App-cache refresh retry.
- Everything unavailable.
- Concurrent centered-panel inputSequence out-of-order.

### Acceptance gate

- Deterministic scenario meets approved stale-call reduction budget.
- Immediate UI tests unchanged.
- Everything lock queue and all provider-failure tests pass. Exact close commands: Phase 3 files from section 20 plus `cargo test --manifest-path src-tauri/Cargo.toml search::`; stale trace artifact present. Skipped/todo tests do not count.

### Rollback

- Revert coordinator commit; no API migration should be needed.

### Evidence artifact

- Stale count, queue wait, provider entry count before/after.

## 13. Phase 4 P1 Everything overfetch

### Objective

Make Everything overfetch feed canonical rerank before truncation.

### Prerequisites

- Phases 2 and 3 complete; ranking stable; stale budget locked; Everything provider-failure tests green.

### RED tests

- Raw Everything fixture intentionally puts best canonical row after `limit` but inside `limit + overfetch`; result MUST appear after rerank.
- Overfetch cap remains bounded.
- Icon extraction work remains capped/observable.

### Exact work

- Map overfetched rows before `.take(limit)`.
- Canonical-rank merged rows, then truncate.
- Avoid unbounded icon extraction from overfetch by cap or lazy icon strategy if needed.
- Start from current `limit + 25` overfetch with maximum `200`; any increase requires measured perf approval.
- Repeat SDK missing/IPC unavailable/query error/timeout isolation tests after provider changes.

### Likely files

- `src-tauri/src/search/providers/everything.rs`
- `src-tauri/src/search/scoring.rs` tests

### Edge cases

- Everything returns fewer than limit.
- Duplicates across overfetched rows.
- Everything degraded/unavailable.

### Acceptance gate

- Better row appears without latency regression beyond approved budget.
- Exact close commands: Phase 4 files from section 20 plus Everything/scoring Rust tests; overfetch and provider-failure artifacts present. Skipped/todo tests do not count.

### Rollback

- Revert Everything provider commit.

### Evidence artifact

- Overfetch fixture ranking diff + perf deltas.

## 14. Phase 5 P1 visible rank order, accessibility, provider health

Phase 5 MUST be split to limit blast radius.

### Phase 5A visible rank order

- **Objective:** Make top visible page strict canonical rank; grouped tail only if approved.
- **RED tests:** `searchUxState` test proving backend rank 4 is not buried below lower-ranked group rows.
- **Work:** Adjust `buildVisibleSearchRows` grouping policy.
- **Likely files:** `src/features/search/searchUxState.ts`, `tests/searchUxState.test.mjs`.
- **Gate:** Product owner approves visual ordering.
- **Rollback:** Revert UI ordering commit.

### Phase 5B accessibility

- **Objective:** Use coherent combobox/listbox pattern.
- **RED tests:** Source tests for input role/ARIA, popup ownership, active descendant, non-tabbable options or true roving model, keyboard behavior.
- **Work:** Input owns popup; focus model single source; status live region.
- **Likely files:** `src/components/SearchPanelSurface.svelte`, `tests/centeredSearchSurface.test.mjs`, `tests/searchUxState.test.mjs`.
- **Gate:** Source tests are contract-only. Runtime protocol MUST use Tauri webview DOM inspector or an approved scripted DOM probe plus screenshots/manual AT notes. Record DOM focus before/after Tab, ArrowUp/Down, Enter, Escape; focus remains input for active-descendant model; listbox/options tab order is correct; live-region text updates for pending/partial/degraded/complete. Accessibility reviewer signs artifact under phase directory.
- **Rollback:** Revert a11y commit.

### Phase 5C provider health

- **Objective:** Show provider state when results exist and when empty.
- **RED tests:** Progress/error/health visible when partial rows remain.
- **Work:** Surface Everything unavailable/cache indexing/refresh status without disrupting rows.
- **Likely files:** `src/components/SearchPanelSurface.svelte`, `src/lib/searchPanel.ts`, `src/lib/searchEngine.ts`, `src/features/search/searchUxState.ts`.
- **Gate:** Health status clear, non-noisy, source-tested.
- **Rollback:** Revert health UI commit.

## 15. Phase 6 optional approval-gated P1 expansion, not core

### Objective

Add low-risk sources with bounded privacy/perf and explicit approval.

### Allowed sources

- Saved Quick Commands **names/descriptions/paths only**.
- Pinned Stack Browser folders.
- Workspace/recent folders from existing user-curated settings.
- Richer JasonShell commands/settings labels.

### Prohibited by default

- Quick Command body/transcript.
- Terminal history.
- Browser history.
- Content search.
- Full filesystem crawls.

### RED tests

- Source-specific provider tests for labels only.
- Privacy tests proving prohibited fields absent.
- Perf tests proving bounded source count.

### Exact work

- Add one provider/source at a time.
- Each source MUST state data origin, max rows, privacy classification, freshness model.
- Each source MUST have health/timing diagnostics if it can degrade.
- Perf artifacts MUST hash/redact user-authored titles, descriptions, and paths. Never log body, transcript, browser/terminal history, or file contents.

### Likely files

- `src-tauri/src/search/providers/local.rs` or new bounded provider module
- settings/workspace modules only if already exposing approved data
- `tests/searchOverhaulPhase6.test.mjs`
- Rust inline provider tests

### Acceptance gate

- Privacy reviewer approval.
- No latency regression vs approved budget.
- Source-contract tests prove no sensitive fields searched.

### Rollback

- Revert one provider/source commit.

## 16. Optional P2/P3, not required for core completion

- **P2-1:** Recent docs bounded provider, opt-in or narrow approved default.
- **P2-2:** Calculator provider, pure local.
- **P2-3:** System actions with destructive confirmation and allowlist.
- **P2-4:** Windows Search fallback when Everything degraded, with timeout/cutoff, after P0/P1.
- **P3-1:** Browser bookmarks/history, opt-in only; history high privacy risk.
- **P3-2:** Terminal history, opt-in only; high privacy risk.
- **P3-3:** Content search, opt-in/deferred/non-realtime; Everything docs warn content is slow because not indexed.

## 17. Exact test matrix

| Req/edge ID | Test layer | Existing/new exact likely file | Fixture/input | Assertion | Failure meaning |
|---|---|---|---|---|---|
| FR-1, INV-NR-1 | Node source contract | `tests/searchOverhaulPhase6.test.mjs` existing | TopBar input handler source | Immediate pending payload publish remains before async result | UI echo regression |
| FR-2 | Node source contract | `tests/searchClearButtons.test.mjs` existing | clear/close flow | sequence invalidation preserved | stale rows can apply after clear |
| FR-3 | Rust unit | `src-tauri/src/search/providers/settings.rs` new inline tests | `w`, `wi`, `windows s`, `control p`, `display s` | expected setting rows match and rank | short-prefix fix broken |
| FR-3 edge | Rust unit | `settings.rs` new | `a`, `*`, empty | no unbounded settings flood; empty no rows | noise/privacy/perf regression |
| FR-4/5 | Rust unit | `src-tauri/src/search/scoring.rs` new | app/source priority vs incidental Everything | bounded signal affects tie but not intent domination | provider signal too weak/strong |
| FR-4 | Node contract | `tests/searchRanking.test.mjs` existing/new | fixture rows with provider scores | capped score math expected | frontend contract drift |
| FR-6/7 | Rust coordinator | `src-tauri/src/search/mod.rs` or extracted coordinator new tests | slow fake Everything, rapid sequences | stale sequence does not enter expensive provider | stale work not cut off |
| FR-8 | Node/Rust docs-source test | `tests/searchEngineContracts.test.mjs` or new | source scan | no claim/use of frontend Promise cancellation as backend cancellation | false cancellation assumption |
| FR-9 | Rust unit | `src-tauri/src/search/providers/everything.rs` new | raw rows best after limit within overfetch | best row reaches canonical rerank | overfetch still wasted |
| FR-10 | Node unit | `tests/searchUxState.test.mjs` existing/new | ranked rows across kinds | top visible page follows canonical rank | display reorders too early |
| FR-11, NFR-A11Y | Node source contract | `tests/centeredSearchSurface.test.mjs` existing/new | `SearchPanelSurface.svelte` markup | combobox/listbox attrs/focus model coherent | a11y focus conflict |
| FR-12 | Node unit/source | `tests/searchPanelState.test.mjs` existing/new | partial rows + provider warning | status visible with rows | degraded state hidden |
| FR-13, NFR-PRIV | Rust + Node source | `tests/searchOverhaulPhase6.test.mjs`, provider inline tests | Quick Command with sensitive body | body/transcript not indexed or rendered | privacy leak |
| INV-NR-6 | Rust/Node integration | new fake-provider harness + `searchUxState` | local fast, provider slow | local progress visible before complete | progressive UX regression |
| INV-NR-9 | Node source contract | `tests/searchContracts.test.mjs` existing/new | command/event registry scan | existing names unchanged | breaking IPC/event contract |
| Everything unavailable | Rust unit | `everything.rs`, `mod.rs` tests | degraded health | other providers still return | degraded provider breaks search |
| App cache cold/warm | Rust unit | `apps.rs` inline existing/new | miss/indexing/refresh/hit | timings and retry signals preserved | app warm retry regression |
| Same-query retry | Node unit | `tests/searchAppCacheRefresh.test.mjs` existing | refresh event same query | bounded rerun occurs | cache warm recovery broken |
| Source-contract limitation | N/A | all source-contract tests | static source text | Mark as contract-only, not live behavior proof | source tests cannot prove runtime latency/focus |
| FR-14, INV-NR-9 | Node source contract | `tests/searchContracts.test.mjs` | exact command/event registry | every name in `CON-U-3` and command list unchanged | IPC/event regression |
| NFR-PRIV-2 | Node/Rust artifact scrub | new artifact validation test | fixture + secret-like body/transcript/history/content | prohibited values absent; paths/titles redacted or hashed | benchmark/privacy leak |
| NFR-MAINT-1 | Node/Rust source contract | new coordinator affordance test | production build/source | no production dynamic provider registry/plugin API | test seam leaked into architecture |
| INV-NR-7 | Node source + runtime trace | `tests/searchTypingFreezePhase1.test.mjs` plus fake-provider trace | each keypress | no recursive FS scan/provider catalog build in input handler | input-path perf regression |
| INV-CUR-13/14 | Node unit/source + Rust contract | `tests/searchEngineContracts.test.mjs`, provider contract tests | each live result kind/ID/action args | exact safe branch and args preserved; unsafe args rejected | wrong or unsafe activation |
| EC-4, NFR-REL-2 | Rust coordinator | `src-tauri/src/search/mod.rs` tests | SDK missing, IPC unavailable, error, timeout | partial rows/progress/final survive; health/timings truthful | provider failure becomes global failure |
| App corpus | Rust provider/coordinator | `apps.rs`, `scoring.rs` tests | `b/br/bra/brave`, `f/fi/firefox`, `sp/spo/spotify`, VS Code, Windows Terminal, PowerToys; cold/warm cache | approved top-k and prefix acquisition | apps remain hard to find |
| Path corpus | Rust provider/coordinator | `local.rs`, `scoring.rs` tests | `c`, `c:`, `c:\`, `c:\dev`, `dev`, `doc`, `downloads`, `~`, `.`, `src/`, `src\` | bounded expected paths, no crash/no settings noise | path normalization/rank regression |
| Window corpus | Rust provider/coordinator | `open_windows.rs`, `scoring.rs` tests | Brave, Firefox, terminal, Settings windows | expected open-window row reaches approved top-k | window intent regression |
| Grouping loss | Node unit | `tests/searchUxState.test.mjs` | canonical rows spanning kinds | top-page rank delta `0` after Phase 5A | UI hides canonical result |

### Requirement traceability

| Requirement set | Acceptance criteria | Primary tests/evidence | Owner phase |
|---|---|---|---|
| FR-1/2, INV-NR-1..6 | AC-3-1, AC-3-4 | input/panel source contracts, stale fake-provider trace | 0, 3 |
| FR-3 | AC-1-1..4 | settings Rust tests, prefix corpus | 1 |
| FR-4/5 | AC-2-1/2 | scoring Rust tests, quality table | 2 |
| FR-6..8, NFR-REL-1 | AC-3-1..5 | coordinator/lock/failure tests, stale formula | 3 |
| FR-9, NFR-REL-2 | AC-4-1..3 | Everything overfetch/failure tests, perf artifact | 4 |
| FR-10 | AC-5A-1 | visible-order test, grouping-loss table | 5A |
| FR-11, NFR-A11Y-1/2 | AC-5B-1/2 | source contract + runtime focus/AT artifact | 5B |
| FR-12 | AC-5C-1 | partial-health unit/runtime evidence | 5C |
| FR-13, NFR-PRIV-1/2 | AC-6-1..3 | source privacy tests + artifact scrub | 6 optional |
| FR-14, INV-NR-9 | AC-X-1 | exact registry test | every phase |
| INV-NR-7, NFR-MAINT-1 | AC-X-2/3 | no-scan and no-production-registry tests | 0, every phase |
| INV-CUR-13/14 | AC-X-4 | activation dispatch/argument tests + safe smoke | every phase |

## 18. Performance benchmark protocol

### Principles

- Benchmarks MUST be deterministic where possible and release-live where needed.
- No absolute latency pass threshold MAY be enforced until Phase 0 baseline and owner-approved budgets.
- Metrics MUST be baseline-relative until budgets exist.
- Do not invent metrics; collect only defined artifacts.
- Existing `firstVisiblePayloadBudgetMs: 50` and `rendererSynchronousBudgetMs: 8` fixture assertions are legacy source-contract guardrails, not measured live-latency acceptance. Phase 0 MUST reclassify, retain, or retire them based on evidence; they cannot prove user-perceived performance.

### Reproducible run protocol

- Deterministic harness: warm up, then run at least 30 measured repetitions per scenario; retain every raw sample and derive p50/p95 from raw JSON.
- Release-live run is separate from deterministic debug/test runs. Compare before/after on same machine, app build mode, package state, Everything availability/version, and cache state.
- Artifact metadata MUST include timestamp, git commit, hash of approved phase-file diff only, redacted `git status --short`, coarse machine class or pseudonymous machine hash, OS, debug/release mode, app version, Everything state, app-cache state, reset steps, and command line. Never persist hostname, username, device serial, unrelated diff content, or full dirty-worktree patch.
- Cold runs use documented disposable test-profile/cache reset; warm runs perform one defined warm-up. Never delete user data to reset state.
- Stale reduction formula: `(baseline stale Everything-boundary entries - post-change stale Everything-boundary entries) / baseline stale Everything-boundary entries`. Boundary is immediately before Everything lock/SDK query; exclude health-cache checks. Zero baseline means metric is not applicable and requires a new discriminating scenario, never an automatic pass.
- Quality report MUST include MRR@k, Recall@k, top-1 stability across monotonic prefixes, prefix acquisition length, and grouping loss.

### Required metrics

- **MET-1:** input-to-local p50/p95.
- **MET-2:** input-to-final p50/p95.
- **MET-3:** stale expensive-provider call count.
- **MET-4:** stale expensive-provider duration.
- **MET-5:** queue wait before latest Everything/provider work.
- **MET-6:** renderer payload-to-visible-row cost.
- **MET-7:** cold/warm app cache state timings.
- **MET-8:** cold/warm Everything health/query timings.
- **MET-9:** Everything unavailable/degraded fallback timings.

### Scenarios

- Rapid prefix: `w` -> `wi` -> `win` -> `windows settings`.
- App prefix: `b` -> `br` -> `bra` -> `brave`; `f` -> `fi` -> `firefox`; `sp` -> `spo` -> `spotify`; VS Code, Windows Terminal, PowerToys.
- Path prefix: `c`, `c:`, `c:\`, `c:\dev`, `dev`, `doc`, `downloads`, `~`, `.`, `src/`, `src\`.
- Open windows: Brave, Firefox, terminal, Settings.
- Everything ordering: intentionally place strongest canonical row after `limit` but within `limit + 25`.
- Cache cold app index.
- Cache warm app index.
- Everything cold health cache.
- Everything warm health cache.
- Everything unavailable.
- Renderer with 0, 5, 50 rows.

### Artifacts

- Deterministic JSON/log from fake-provider harness.
- Command output for focused Node/Rust tests.
- Release live dogfood notes with machine state: Everything available/unavailable, app cache cold/warm, package resource status.
- p50/p95 table and stale-count table before/after.
- Redaction report proving artifacts contain no prohibited user content.

## Acceptance Criteria

### AC-0: Phase 0 baseline

- **AC-0-1 (NFR-PERF-1):** Given current repo, when baseline commands run, then artifacts record p50/p95, stale count, queue wait, renderer cost, cold/warm app, cold/warm Everything.
- **AC-0-2 (FR-7):** Given fake provider harness, when rapid sequences execute, then provider boundary entries are observable deterministically.

### AC-1: Phase 1 short-prefix settings

- **AC-1-1 (FR-3):** Given query `w`, when settings provider runs, then relevant Windows settings row appears per approved rank expectation.
- **AC-1-2 (FR-3):** Given query `control p`, when settings provider runs, then Control Panel row appears.
- **AC-1-3 (FR-3, NFR-PERF-2):** Given noisy one-letter query, when settings provider runs, then result count remains bounded by approved corpus expectation.
- **AC-1-4 (FR-3):** Given path-like `c`, `c:`, or `c:\`, when settings provider runs, then no unrelated settings flood occurs.

### AC-2: Phase 2 provider signal

- **AC-2-1 (FR-4, FR-5):** Given provider priority fixture, when canonical rank runs, then bounded provider signal changes only approved close calls.
- **AC-2-2 (FR-4):** Given exact settings/app intent and high-run-count Everything row, when rank runs, then exact intent remains above incidental file.

### AC-3: Phase 3 latest-only cutoffs

- **AC-3-1 (FR-1, FR-2, FR-6):** Given rapid typing, when each input fires, then pending payload appears immediately for each accepted query and TopBar ownership remains unchanged.
- **AC-3-2 (FR-6/7):** Given stale slow fake provider, when newer sequence supersedes, then stale sequence does not enter Everything boundary.
- **AC-3-3 (FR-8):** Given frontend invoke Promise exists, when stale query is superseded, then docs/tests do not claim true cancellation.
- **AC-3-4 (FR-6, NFR-REL-1):** Given same normalized query with newer sequence, when old work completes, then newer sequence alone may apply; current same-query retry remains allowed.
- **AC-3-5 (FR-6, NFR-REL-2):** Given stale request waits near Everything and provider fails or times out, when newer request arrives, then stale work does not enter lock/SDK boundary and latest partial response remains valid.

### AC-4: Phase 4 Everything overfetch

- **AC-4-1 (FR-9):** Given best Everything row is after `limit` but inside overfetch, when provider maps/ranks, then row can appear in final top results.
- **AC-4-2 (NFR-PERF):** Given overfetch, when benchmark runs, then icon/mapper work stays within approved budget.
- **AC-4-3 (NFR-REL-2):** Given Everything SDK missing, IPC unavailable, query error, or timeout, when overfetch path runs, then settings/apps/local/window rows and truthful final partial status remain.

### AC-5: Phase 5 visible order, accessibility, and health

- **AC-5A-1 (FR-10):** Given backend canonical rank order, when visible rows build, then top visible page preserves strict canonical rank.

#### Phase 5B accessibility

- **AC-5B-1 (FR-11/NFR-A11Y):** Given centered search open, when user arrows results, then DOM focus model and `aria-activedescendant` remain coherent.
- **AC-5B-2 (FR-11, NFR-A11Y-1/2):** Given pending, partial, degraded, and complete states, when runtime protocol runs Tab/Arrow/Enter/Escape, then focus/tab order, activation/close, and live announcements match approved combobox behavior.

#### Phase 5C provider health

- **AC-5C-1 (FR-12):** Given partial results and Everything unavailable, when panel renders, then provider health/status is visible without removing rows.

### AC-6: Optional Phase 6 expansion

- **AC-6-1 (FR-13/NFR-PRIV):** Given saved Quick Command has title, description, body, transcript, when provider indexes, then only approved title/description/path fields are searchable.
- **AC-6-2 (NFR-PERF):** Given expanded provider source, when benchmark runs, then source count and time remain bounded.
- **AC-6-3 (NFR-PRIV-2):** Given user-authored source values, when tests and benchmarks write artifacts, then titles/descriptions/paths are redacted or hashed and prohibited content is absent.

### AC-7: Cross-phase non-regression

- **AC-X-1 (FR-14, INV-NR-9):** Given exact command/event registry, when any phase completes, then all unchanged names match `CON-U-1..3`.
- **AC-X-2 (INV-NR-7):** Given each typed prefix, when input and backend traces run, then no broad recursive filesystem scan is started per keypress.
- **AC-X-3 (NFR-MAINT-1):** Given production build/source, when test affordance is inspected, then no dynamic provider registry/plugin API exists outside test configuration.
- **AC-X-4 (INV-CUR-13/14):** Given every live result kind and representative safe/unsafe action arguments, when activation dispatch is tested, then correct safe branch executes, unsafe input is rejected, query clears, and panel closes only after accepted activation.

## 20. Exact commands per phase and final gate

Commands are derived from `package.json`. Run commands sequentially in PowerShell; exact files avoid shell-glob and test-name filtering gaps.

### Per phase focused gate

```powershell
npm run check
npm run build
node scripts/clean-dist-tests.mjs
tsc -p tsconfig.test.json
node --test tests/searchSettings.test.mjs tests/searchRanking.test.mjs tests/searchTypingFreezePhase1.test.mjs tests/searchUxState.test.mjs tests/searchEngineContracts.test.mjs tests/searchPanelState.test.mjs tests/searchContracts.test.mjs tests/searchAppCacheRefresh.test.mjs tests/searchCloseReset.test.mjs tests/searchClearButtons.test.mjs tests/searchOverhaulPhase0.test.mjs tests/searchOverhaulPhase6.test.mjs tests/centeredSearchSurface.test.mjs tests/performanceBaselineContract.test.mjs tests/masterSpecSearchHygiene.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml search::
```

### Broader gate before completing P0 or P1 slice

```powershell
npm run test:node
npm run cargo:test
npm run cargo:check
```

### Final release gate

```powershell
npm run validate
```

Per-phase close MAY use only exact touched test files from this list plus named Rust module tests, but final P0/P1 slice runs full listed set. No phase complete if required command fails without documented, owner-approved unrelated-failure waiver. Skipped/todo tests never satisfy an acceptance criterion.

## 21. Manual/live dogfood matrix

| Scenario | Setup | Steps | Expected | Notes |
|---|---|---|---|---|
| Everything available | Everything process/service healthy | Type short prefixes and path prefixes | Progressive local then complete; broad file rows appear | Capture p50/p95. |
| Everything unavailable | Stop/disable Everything or remove SDK availability in test env | Type same corpus | Settings/apps/local/window still work; health visible | No crash. |
| App cache cold | Clear app index cache in approved test profile | Type app prefixes | miss/indexing status then refresh retry | Do not delete user data without backup. |
| App cache warm | Fresh cache | Type app prefixes | expected apps appear quickly | Compare baseline. |
| Keyboard/a11y runtime | Centered panel focused; DOM inspector/screen reader available | Record initial focus and tab order; Tab, ArrowDown/Up, Enter, Escape; observe active descendant and live region in pending/partial/degraded/complete | focus stays input for active-descendant model; options not tabbable; selection/activation/close and announcements correct | Save trace/screenshots/AT notes under phase artifact. |
| Real activation | Known safe app/folder/setting | Activate row | correct action, query clears, panel closes | Avoid destructive commands. |
| Stale query | Rapid type long prefix with slow Everything | Older results never apply; latest completes | Record stale count. |
| Package resource blocker | Packaged app dogfood | Launch package | Known blocker: package resources may miss `app-update.yml` per investigation | Fix packaging separately before relying on packaged dogfood. |

## 22. Risk register

| Risk ID | Trigger | Mitigation | Rollback | Evidence |
|---|---|---|---|---|
| R-1 Short-prefix noise | One-letter query floods settings | allowlist/prefix policy + bounded count tests | revert Phase 1 | prefix corpus diff |
| R-2 Ranking regression | Provider signal dominates canonical intent | cap evidence + tests for exact intent | revert Phase 2 | MRR/Recall/top-1 table |
| R-3 False cancellation assumption | Impl relies on Tauri Promise cancel | backend boundary checks only; doc tests | revert Phase 3 | fake-provider trace |
| R-4 Stale local/progress regression | latest-only suppresses current local progress | tests require local progress for latest | revert Phase 3 | progress trace |
| R-5 Overfetch latency | mapping/icons overfetched rows too expensive | cap map/icon work; measure p95 | revert Phase 4 | perf artifact |
| R-6 UI order surprise | users prefer grouping | approval gate; maybe grouped tail | revert Phase 5A | dogfood notes |
| R-7 A11y keyboard break | focus model change breaks nav | source + manual keyboard tests | revert Phase 5B | a11y checklist |
| R-8 Privacy leak | new provider indexes sensitive fields | privacy tests + reviewer gate | revert source commit | field-level tests |
| R-9 Everything degraded | provider unavailable blocks complete | health checks + skip fallback safe | revert touched provider | unavailable dogfood |
| R-10 Dirty worktree conflict | unrelated changes modified | inspect status/diff; touch only slice files | restore/revert own commit | git diff review |

## 23. Phase completion checklist and final release checklist

### Phase completion checklist

- [ ] Requirement IDs mapped to tests.
- [ ] RED/failing or characterization test added before implementation when practical.
- [ ] Exact files changed match phase scope.
- [ ] `git status --short` and `git diff -- <phase files>` reviewed; concurrent dirty target conflict resolved by owner.
- [ ] Existing behavior invariants reviewed.
- [ ] Focused commands from section 20 pass or waiver approved.
- [ ] Perf/stale evidence captured for perf-affecting phase.
- [ ] Accessibility evidence captured for markup/focus phase.
- [ ] Privacy evidence captured for provider/source phase.
- [ ] Rollback path documented and tested conceptually.
- [ ] Named phase tests, commands, artifacts, and required approvals all complete; skipped/todo tests counted as missing.
- [ ] `master_spec.md` updated if behavior/contracts/tests/risks changed.
- [ ] `changelog.md` updated per `CHANGELOG_POLICY.md` if implementation changed repo behavior.

### Final release checklist

- [ ] Core Phases 0-5 complete or explicitly descoped by owner; optional Phase 6 does not block core release.
- [ ] `npm run validate` passes.
- [ ] Baseline-relative p50/p95 and stale-work tables approved.
- [ ] Everything available/unavailable dogfood complete.
- [ ] App cache cold/warm dogfood complete.
- [ ] Keyboard/a11y dogfood complete.
- [ ] Real activation smoke complete for safe rows.
- [ ] Package resource blocker resolved or release notes mark packaged dogfood unavailable.
- [ ] No prohibited default sources added.
- [ ] No permanent rollback flags remain without written justification.
- [ ] Durable docs updated for actual behavior changes.

## 24. Unresolved approval questions

- **Q-1:** What final latency budgets are acceptable after Phase 0 baseline? Need p50/p95 input-to-local, input-to-final, renderer, queue wait.
- **Q-2:** Is >=80% stale expensive-provider call reduction accepted as P0 target for rapid-prefix deterministic scenario?
- **Q-3:** What provider signal cap should be locked after corpus experiment? Is initial `0..300` experiment approved?
- **Q-4:** Should top visible page be strict canonical rank, and where may grouped tail begin if grouping remains?
- **Q-5:** Is provider health UI allowed to show while results exist, and what wording/icons should it use?
- **Q-6:** Which Phase 6 sources are approved first: Quick Command labels, pinned folders, workspace/recent folders, richer JasonShell commands/settings?
- **Q-7:** Is a temporary runtime rollback flag warranted for any phase, or should revert-only remain policy?
- **Q-8:** Should CI retain/upload ignored local artifacts from `test-results/search-performance/<timestamp>/`, and for how long?
- **Q-9:** Should packaged-app dogfood block release until `app-update.yml` resource issue is fixed?
