# Stack Browser Large-Folder Stability And Instant-Load Plan

## Summary

Fix Stack Browser large-folder freezes by changing load order first, then paging architecture, then icon delivery path, then instrumentation and regression coverage.  
Target outcome: opening folders like Downloads renders visible metadata immediately, never stalls at `80 of N items`, never does a 500-row synchronous burst, and keeps progressive growth stable under mixed file types.

Implementation default:
- single `gpt-5.3-codex` worker
- sequential phases
- each phase must pass its own acceptance criteria before next phase starts
- preserve current Stack Browser UX: history, path bar, selection, sort, delete flow, drag/drop, focus-loss behavior

## Phase 0 - Guardrails, Baseline, Durable Context

### Tasks
- Add `[USER]` ledger entry in `master_spec.md` for stack-browser large-folder freeze/perf work.
- Add summary of current root cause to `master_spec.md` Stack Browser section:
  - first batch `80`
  - second batch `500`
  - backend page request rescans whole dir
  - stack rows resolve shell icons inline
  - no stack-browser icon cache
- Create baseline measurement hooks before behavioral refactor:
  - folder open total duration
  - per-page duration
  - per-page item count
  - icon-resolution count/duration
  - payload size estimate or serialized item count
- Add one reproducible large-folder smoke target note:
  - Downloads-like folder with 500+ mixed items

### Tests To Write
- Rust test for instrumentation helper shape/field defaults.
- Node test that Stack Browser status logic still supports progressive counts like `X of Y items`.
- Node or Rust contract test that existing read/list payload contract still parses before refactor.

### Acceptance Criteria
- `master_spec.md` documents problem and intended phased fix order.
- baseline logging/diagnostic path exists without changing visible Stack Browser behavior yet.
- worker can capture timings for first page and follow-up pages on large folders.

## Phase 1 - Fast Metadata-Only First Paint

### Tasks
- Change folder-open hot path so initial visible rows do not depend on icon extraction.
- First open returns metadata needed for row text and sort:
  - path
  - name
  - kind
  - type label
  - size
  - modified time
  - flags
- Do not embed `iconDataUrl` in initial folder page response.
- Keep existing row layout stable by rendering fallback glyph/type icon until real icon arrives later.
- Keep current path open flow progressive:
  - first rows paint as soon as metadata page lands
  - UI never waits for icons to become available
- Keep `hasRetainedRows`, history, sort, selection, delete prompt, drag/drop behavior unchanged.

### Tests To Write
- Rust test: folder page response can omit icon payload and still serialize/deserialize correctly.
- Node test: row rendering uses fallback icon when `iconDataUrl` missing.
- Node test: opening folder updates visible rows/count without waiting for icon fields.
- Node test: status text progresses beyond empty/loading state as soon as metadata page lands.

### Acceptance Criteria
- large-folder open shows visible rows before any icon resolution finishes.
- no folder-open path synchronously resolves shell icons for initial first paint.
- Stack Browser remains functionally identical except icons may appear after text rows.

## Phase 2 - Replace Fake Paging With Real Progressive Paging

### Tasks
- Remove current "re-enumerate full dir on every page request" behavior.
- Implement real paging/streaming strategy for folder enumeration:
  - one enumeration/snapshot per open request
  - stable sorted order per listing session
  - subsequent page fetches continue from stored state/snapshot, not fresh full-dir rescan
- Replace first-page `80`, second-page `500` burst pattern with steady chunking.
- Default chunking:
  - small first paint batch sized for instant render
  - follow-up batches moderate and steady, not giant burst
- Preserve current frontend progressive merge flow.
- Add stale-request guard so switching folders cancels or ignores old page continuations cleanly.
- If folder contents change mid-listing, choose stable-session snapshot semantics for current open, then refresh explicitly later. Do not let mid-read mutations break pagination progress.

### Tests To Write
- Rust test: page 2+ does not trigger full-dir re-enumeration for same listing session.
- Rust test: large folder with 500+ entries returns all items across multiple pages with stable order and no duplicates/skips.
- Rust test: stale/cancelled listing session cannot overwrite newer open request.
- Node test: progressive merge grows visible count monotonically until total reached.
- Node test: switching folders mid-load never leaves old folder rows appended into new folder view.

### Acceptance Criteria
- page 2+ for same folder no longer rescans and resorts whole directory.
- no 500-row synchronous second burst remains.
- large-folder open grows steadily from first page to completion.
- UI no longer gets stuck at `80 of N items` due to paging architecture.

## Phase 3 - Lazy Icon Pipeline And Cache

### Tasks
- Add stack-browser-specific icon resolution pipeline decoupled from folder metadata paging.
- Introduce icon cache for Stack Browser, using existing search/process-manager cache pattern as model.
- Resolve icons lazily after rows are already visible.
- Cache key default:
  - normalized path for exact-path cache
  - optional later optimization for extension/type-level fallback if needed, but v1 should keep correctness first
- Add bounded concurrency for icon resolution; do not launch unbounded shell icon work for hundreds of rows at once.
- Publish icon updates incrementally to frontend for currently visible/pending rows.
- Prioritize visible rows first if viewport-aware ordering is feasible without major complexity; otherwise resolve in listing order with bounded queue.
- If shell icon lookup fails or is slow:
  - keep fallback icon
  - record diagnostic
  - do not block listing completion

### Tests To Write
- Rust test: cache returns reused icon for repeated path requests.
- Rust test: icon lookup failure returns fallback/no-icon result without failing folder load.
- Rust test: bounded queue/concurrency helper never schedules unbounded parallel work.
- Node test: row icon updates in place after metadata row already rendered.
- Node test: reopening same folder reuses cached icons and avoids full icon-refresh burst where possible.

### Acceptance Criteria
- icon extraction no longer blocks metadata listing path.
- repeated open of same folder avoids fresh icon work for cached rows.
- icon failures degrade gracefully without freeze/crash.
- visible rows stay interactive while icon updates stream in.

## Phase 4 - Frontend Progressive UX Hardening

### Tasks
- Keep current progressive `listStackFolder` merge contract, but align it to new steady paging model.
- Ensure loading/status UX reflects real state:
  - initial loading
  - progressive `X of Y items`
  - icon background work does not keep folder in misleading "loading folder..." state once metadata listing complete
- Keep virtualization compatible with progressive append and late icon updates.
- Ensure sort actions during or after progressive load behave deterministically.
- Ensure open-folder cancellation, back/forward navigation, typed path open, and delete-refresh paths all use new listing session model correctly.
- Prevent focus thrash from repeated viewport/focus calls during each page append if baseline shows it contributes to jank.

### Tests To Write
- Node test: virtualized window remains correct as entries append progressively.
- Node test: sort after partial load and after full load yields stable order.
- Node test: back/forward during active listing does not corrupt current view.
- Node test: typed-path open during active listing cancels old request cleanly.
- Node test: delete-refresh still reloads current folder using new listing path.

### Acceptance Criteria
- progressive append, sort, nav, typed-path open, and delete-refresh all remain correct.
- metadata completion and icon completion have distinct, sane status behavior.
- no UI regressions in selection, path bar, context menus, or retained-row logic.

## Phase 5 - Diagnostics, Regression Suite, Live Validation

### Tasks
- Keep instrumentation from Phase 0 and wire final metrics around:
  - first paint time
  - metadata listing complete time
  - icon queue complete time
  - per-page counts
  - cache hits/misses
- Add targeted regression coverage for mixed large folders with many archives/executables.
- Add one optional Windows-only ignored/live diagnostic test for shell-icon heavy folder scenarios if repo already uses that pattern elsewhere.
- Run full static validation and focused large-folder checks.
- Update `master_spec.md` with final architecture:
  - metadata-first listing
  - real paging session model
  - lazy cached icon pipeline
  - new diagnostics/tests

### Tests To Write
- Rust integration-style test for 500+ item mixed folder listing completion across many pages.
- Rust test for diagnostics counters/timings emitted on large-folder path.
- Node test for status transitions:
  - loading
  - progressive count
  - metadata done
  - icon completion/fallback stable
- Focused regression test that reproduces old `80 of N` failure shape and proves continued growth past 80.
- Existing validation gates plus any Stack Browser focused suites.

### Acceptance Criteria
- worker can demonstrate large-folder open progresses past 80 every run.
- no crash/freeze during Downloads-class folder open in live Windows smoke.
- metrics show first paint happens before full listing completion.
- `master_spec.md` fully updated with shipped behavior and validation notes.

## Worker Execution Notes

- Use TDD per phase: write failing tests first where practical, then implement phase, then run phase gates.
- Do not start icon-cache work before metadata-first and real paging are stable.
- Do not widen scope into new Stack Browser features; this plan is perf/stability only.
- Keep public contracts as compatible as possible. If payload shape changes, update frontend/runtime validators and tests in same phase.
- Favor smaller, reviewable commits/patches per phase even if one worker does full sequence.

## Assumptions

- Assume `gpt-5.3-codex` worker owns full implementation. Ensure to still use QA subagents.
- No new end-user settings needed for this work.
- Current Stack Browser UX and commands stay intact; only load architecture changes.
- Lazy icon loading with fallback glyphs is acceptable if text rows appear instantly.
- Stable per-open listing snapshot/session behavior is preferred over live resort/re-enumeration during same folder open.
