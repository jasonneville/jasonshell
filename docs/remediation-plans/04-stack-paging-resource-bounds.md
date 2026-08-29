# 04 Stack Paging Resource Bounds

## Metadata

- Status: Implementation-ready plan; requires resource-bound design decisions before code.
- Order: 04 of 13.
- Owner: Stack Browser backend/resource implementer.
- Dependencies: Plans 01-02 MUST be complete.
- Related audit finding: P0-2 Stack paging does not bound discovery or retained memory.

## Objective and evidence

Objective: make Stack Browser folder/archive paging genuinely resource-bounded by clamping page size, bounding discovery/archive scans, bounding retained sessions by entries/estimated bytes, avoiding continuation full-vector clones, and defining truncation/eviction diagnostics without changing normal small-folder behavior.

Evidence:

- Audit lines 131-142: current paging enumerates/sorts full collection, clones into session, clones continuation vectors, and limits sessions only by count.
- Source `src-tauri/src/stack_popup/paging.rs`: `page_limit = limit.max(1)` no upper clamp; first page `entries.clone()` into session; continuation clones `session.entries`; `MAX_STACK_FOLDER_SESSIONS = 32` count only; zip collection loops over full archive length.
- `master_spec.md` Stack Browser and responsiveness sections describe paged reads and diagnostics but not hard discovery/session memory bounds.

## Scope

Exact files/symbols likely in scope:

- `src-tauri/src/stack_popup/paging.rs`
- `DEFAULT_PAGE_LIMIT`
- `MAX_STACK_FOLDER_SESSIONS`
- `read_stack_folder_page_with_session_and_downloads_detector`
- `collect_stack_folder_entries`
- `collect_zip_folder_entries`
- `StackFolderListingSessionStore::{start_session, continue_session, finish_session, trim_sessions}`
- `StackFolderPageDiagnostics`, `StackFolderWarning`, `StackFolderPage` in `src-tauri/src/stack_popup/models.rs`
- Frontend wrappers only if payload warnings/diagnostics fields need display; avoid speculative UI additions.
- Existing tests: `tests/stackPopupPagingPhase*.test.mjs`, `tests/stackPopupState.test.mjs`, Rust tests in paging module.

Explicit out of scope:

- Stack Git workbench behavior.
- Terminal behavior.
- New virtualized frontend architecture.
- Exact release performance acceptance; this plan creates bounds and tests, not full runtime perf proof.

## Current-state contract

- Folder reads return `StackFolderPage` with `items`, `offset`, `limit`, `total`, `has_more`, `warnings`, `diagnostics`, and `session_id`.
- Downloads sorting uses modified desc; ordinary sorting folders-first/name asc.
- Continuation requires active matching `session_id`; stale/mismatch errors are explicit.
- Current defect: first-page discovery and session retention are unbounded despite paged payload.

## Requirements

### Functional

1. Renderer-provided `limit` MUST be clamped to a bounded maximum. Recommended default: 200 items/page; stop/escalate if UX requires higher.
2. Filesystem discovery MUST enforce a maximum scanned/retained entry count. Recommended default: 10,000 entries per listing session.
3. ZIP/archive discovery MUST enforce a maximum archive entries scanned. Recommended default: 20,000 raw zip entries or 10,000 retained child entries, whichever trips first.
4. Session store MUST enforce total retained entry and estimated-byte budgets, not only session count. Recommended defaults: 32 sessions, 50,000 retained entries, 32 MiB estimated summaries.
5. Continuation MUST avoid cloning the entire retained entry vector per request.
6. Truncation MUST be explicit in warnings/diagnostics; `has_more` semantics MUST not promise undiscovered complete totals.
7. Sorting MUST remain deterministic for retained entries.
8. Stale session/path mismatch behavior MUST remain unchanged.
9. Small folders/archives below limits MUST return same ordering and fields as before except new diagnostics fields if added.

### Nonfunctional

10. Tests MUST not require creating 100k real files unless explicitly gated as slow/manual; use injectable collectors/fakes for unit proof.
11. Bounds SHOULD be constants documented in source and `master_spec.md`.
12. No public API commitment to exact constants beyond documented recommended defaults unless implementation confirms UX.
13. Memory estimates MAY be conservative; they MUST be monotonic enough to drive eviction.

## Decisions and implementation approach

- Recommended default constants:
  - `MAX_PAGE_LIMIT = 200`
  - `MAX_FILESYSTEM_LISTING_ENTRIES = 10_000`
  - `MAX_ZIP_RAW_ENTRIES_SCANNED = 20_000`
  - `MAX_STACK_FOLDER_SESSION_ENTRIES = 50_000`
  - `MAX_STACK_FOLDER_SESSION_BYTES = 32 * 1024 * 1024`
- Stop/escalate if product owner requires exact global sort over >10k entries; bounded paging cannot honestly claim complete sort without full scan.
- Prefer slice borrowing from session under lock only long enough to clone page slice, not full session.
- If diagnostics schema changes, keep fields optional/additive.

## Phases with RED-first tests

1. RED Rust test: extreme `limit` clamps to max. Expected fail now.
2. RED Rust test: fake collector over max entries returns truncated warning and bounded `total`. Expected fail now.
3. RED Rust test: continuation clones only requested page slice; use instrumented clone counter if feasible. Expected fail now or source-contract fallback.
4. RED Rust test: session store evicts by entry/byte budget before count limit. Expected fail now.
5. Implement constants, clamp, truncation warning/diagnostics.
6. Refactor session continuation to page-slice clone only.
7. Implement session budget accounting/eviction.
8. Update Node source/contract tests if payload/constant names are guarded.
9. Run focused/full validation.

## Exact tests and assertions

- Add Rust test in paging module: `read_stack_folder_page_clamps_extreme_page_limit` asserts requested huge limit returns at most max and records effective limit.
- Add Rust test: `filesystem_listing_stops_at_entry_bound_with_warning` asserts no more than max retained entries and warning explains truncation.
- Add Rust test: `zip_listing_stops_at_raw_scan_bound_with_warning` asserts archive loop stops early and reports truncation.
- Add Rust test: `continuation_clones_only_requested_page_slice` asserts no full-vector clone on continuation by source or instrumented summary clone count.
- Add Rust test: `session_store_evicts_by_entry_and_byte_budget` asserts oldest sessions removed when total retained budget exceeded.
- Existing `tests/stackPopupState.test.mjs`: deterministic sorting still passes.
- Existing `tests/stackPopupPagingPhase*`: paging wiring/diagnostics still pass or are updated for additive diagnostics.

## Edge and failure cases

- `limit = 0` -> still at least 1, then max clamp.
- `limit = usize::MAX` -> no overflow in offset/limit/has_more math.
- Offset beyond retained total -> returns empty page, `has_more=false`, and finishes session if appropriate.
- Truncated listing where real undiscovered items exist -> warning must say listing truncated; `total` means retained/discovered total unless schema adds `is_truncated`.
- ZIP with many nested entries under one child -> raw scan bound may truncate before all children discovered; warning required.
- Session eviction before continuation -> existing unknown/stale session error acceptable.

## Data/API/event compatibility impacts

- `StackFolderPage` may gain additive `is_truncated`/diagnostics fields only if needed; avoid breaking existing frontend.
- Existing fields should retain type/name.
- Warnings are already supported; prefer warnings for truncation to avoid API break.

## Validation commands

Focused:

```bash
cargo test --manifest-path src-tauri/Cargo.toml stack_popup::paging -- --nocapture
node scripts/clean-dist-tests.mjs && tsc -p tsconfig.test.json && node --test tests/stackPopupPagingPhase1Wiring.test.mjs tests/stackPopupPagingPhase2Wiring.test.mjs tests/stackPopupPagingPhase3Wiring.test.mjs tests/stackPopupPagingPhase4Wiring.test.mjs tests/stackPopupPagingPhase5Wiring.test.mjs tests/stackPopupPagingPhase6Responsiveness.test.mjs tests/stackPopupState.test.mjs
```

Full:

```bash
npm run cargo:test
npm run cargo:check
npm run test:node
npm run validate
```

Optional slow/manual with generated temp fixtures:

```bash
# Only if approved; create under ignored temp/test-results, not repo source.
```

## Acceptance criteria

- Given extreme renderer limit, when page is read, then returned items are capped at max. Covers req 1.
- Given oversized directory/archive fake, when first page is read, then discovery stops at configured bound and warning/diagnostic says truncated. Covers req 2-3, 6.
- Given continuation, when page N is requested, then implementation clones only needed page slice, not whole session. Covers req 5.
- Given many sessions, when budgets exceed entry/byte limits, then oldest sessions evict and active path map remains coherent. Covers req 4, 8.
- Given small folder, when paging all entries, then order/fields match previous behavior. Covers req 7, 9.

## Risks and rollback

- Risk: truncation surprises users in huge folders. Rollback: raise constants or add UI warning, not remove hard bounds.
- Risk: total/has_more semantics ambiguous. Rollback: add explicit additive `truncated` diagnostics and update frontend text.
- Risk: global sort no longer complete past cap. Rollback: escalate product decision; do not silently claim complete sort.

## Master spec, changelog, docs updates

- Update `master_spec.md` Stack Browser section with page clamp, scan/session bounds, truncation warning semantics, and validation coverage.
- Append changelog entries for behavior/tests.
- If constants remain recommended rather than final, document final chosen constants only after implementation.

## Handoff checklist

- [ ] Read docs/source/tests.
- [ ] Chosen constants confirmed or escalated.
- [ ] RED tests added for clamp, truncation, continuation clone, session budgets.
- [ ] Implemented bounds and warnings/diagnostics.
- [ ] Focused tests green.
- [ ] Full feasible validation run.
- [ ] Durable docs updated.
- [ ] Adversarial QA for resource bypass and API ambiguity.

## Copy/Paste Implementation Prompt

```text
Implement remediation plan docs/remediation-plans/04-stack-paging-resource-bounds.md. First read master_spec.md, docs/current-state-technical-audit-2026-08-28.md, CHANGELOG_POLICY.md, package.json, and this plan. Preserve unrelated worktree changes. Use RED-first: add failing tests for page-limit clamp, filesystem/archive scan bounds, no full-vector continuation clone, and session entry/byte budget eviction before code. Do not implement Stack Git or terminal behavior. Pick documented constants from the plan unless tests/product evidence require escalation; stop and ask before promising complete global sort beyond the cap. Run focused Rust paging and Node Stack paging/state tests, then npm run cargo:test/cargo:check/test:node/validate as feasible. Update master_spec.md Stack Browser contract and changelog.md per policy. Perform adversarial QA focused on huge directories, zip bombs/large archives, offset overflow, stale sessions, and API semantics. Report files changed, commands run, pass/fail evidence, final constants, and unresolved blockers.
```
