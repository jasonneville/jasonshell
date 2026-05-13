# Task: FN-004 - Bound JasonShell Shell and Process Icon Caches

**Created:** 2026-05-12
**Size:** M

## Review Level: 2 (Plan and Code)

**Assessment:** This is a focused backend cache-policy change in two Rust surfaces with existing cache-hit and lock-scope tests. Risk is moderate because icon cache behavior affects Stack Browser and Process Manager responsiveness, but the change is reversible and can be validated with unit/smoke tests.
**Score:** 4/8 — Blast radius: 1, Pattern novelty: 1, Security: 0, Reversibility: 2

## Mission

Bound JasonShell's process-lifetime shell icon caches so browsing many Stack Browser folders or refreshing many Process Manager executable paths cannot grow memory without limit. Add constants-based LRU or size-cap eviction that preserves the current lock-light cache-hit path, keeps shell extraction outside cache locks, and documents before/after cache-size behavior for future maintainers.

## Dependencies

- **None**

## Context to Read First

- `master_spec.md` — read the Stack Browser icon hydration and Process Manager icon cache sections before changing behavior.
- `CHANGELOG_POLICY.md` — follow the repository changelog policy.
- `src-tauri/src/stack_popup/icons.rs` — current Stack Browser icon cache, batch limiting, cache-key normalization, and spawn-blocking path.
- `src-tauri/src/stack_popup.rs` — existing Stack Browser Rust tests around icon cache reuse and batch/smoke contracts.
- `src-tauri/src/process_manager.rs` — current Process Manager icon cache helpers and `icon_cache_tests` module.
- `src-tauri/Cargo.toml` — Rust test/build configuration.
- `package.json` — repository validation scripts.

## File Scope

- `src-tauri/src/stack_popup/icons.rs`
- `src-tauri/src/stack_popup.rs`
- `src-tauri/src/process_manager.rs`
- `master_spec.md`
- `changelog.md`

## Steps

### Step 0: Preflight

- [ ] Required files and paths exist
- [ ] Dependencies satisfied
- [ ] Confirm there is no existing reusable bounded-cache helper in `src-tauri/src/**`; if one exists, reuse it instead of introducing a duplicate abstraction.

### Step 1: Add a bounded cache policy for Stack Browser shell icons

- [ ] Replace `STACK_POPUP_ICON_CACHE`'s unbounded `HashMap<String, Option<String>>` storage with a constants-based bounded cache policy, e.g. `STACK_ICON_CACHE_CAPACITY`, using LRU or an equivalent recency-aware eviction strategy.
- [ ] Preserve the existing public behavior of `resolve_stack_item_icons_for_paths`, `resolve_stack_item_icons_for_paths_async`, `resolve_stack_item_icons_batch`, and `StackItemIconResolution.cache_hit`.
- [ ] Keep cache hits lock-light: lookup/recency update may occur inside the cache mutex, but `resolve_shell_icon_data_url(...)` must still run outside the cache lock and storage must reacquire the lock only after extraction.
- [ ] Ensure both successful icon data and `None` miss results are cacheable entries subject to the same cap.
- [ ] Add unit-testable helpers for bounded insertion/lookup/length behavior rather than relying only on the global `OnceLock` cache.
- [ ] Run targeted Rust tests for Stack Browser icon cache changes.

**Artifacts:**
- `src-tauri/src/stack_popup/icons.rs` (modified)
- `src-tauri/src/stack_popup.rs` (modified)

### Step 2: Add a bounded cache policy for Process Manager process icons

- [ ] Replace the Windows `ProcessIconCache = Mutex<HashMap<String, Option<String>>>` storage in `src-tauri/src/process_manager.rs` with a constants-based bounded cache policy, e.g. `PROCESS_ICON_CACHE_CAPACITY`, using LRU or an equivalent recency-aware eviction strategy.
- [ ] Preserve `process_icon_data_url_from_cache_with_extractor(...)` semantics: blank/missing executable paths return `None`, cache hits avoid extraction, misses call the extractor exactly once before storing.
- [ ] Keep the cache-hit path short and keep `extractor(Path::new(&icon_cache_key))` outside the cache mutex; update the existing source-structure test if needed so it verifies this lock boundary remains intact.
- [ ] Normalize or compare cache keys consistently with existing behavior unless a safer normalization is explicitly covered by tests.
- [ ] Add eviction tests proving the cap is enforced, recently accessed entries survive over older entries, and cached `None` entries count toward the cap.
- [ ] Run targeted Rust tests for Process Manager icon cache changes.

**Artifacts:**
- `src-tauri/src/process_manager.rs` (modified)

### Step 3: Large-folder and process-manager smoke coverage

- [ ] Add or update Rust tests that simulate resolving more unique Stack Browser icon paths than the cap and assert steady-state cache length never exceeds the cap.
- [ ] Add or update Process Manager cache smoke tests that simulate more unique executable paths than the cap and assert all returned icon values remain correct while cache length stays capped.
- [ ] Ensure smoke tests use deterministic fake extractors/test data and do not depend on Windows shell icon extraction succeeding on the test host.
- [ ] Verify existing tests still cover cache-hit reuse, batch truncation, async stack icon resolution, and Process Manager lookup/resolve/store ordering.

**Artifacts:**
- `src-tauri/src/stack_popup/icons.rs` (modified)
- `src-tauri/src/stack_popup.rs` (modified)
- `src-tauri/src/process_manager.rs` (modified)

### Step 4: Testing & Verification

> ZERO test failures allowed. Full test suite as quality gate.
> If keeping lint/tests/build/typecheck green requires edits outside the initial File Scope, make those fixes as part of this task.

- [ ] Run focused Rust tests for stack/process icon caches, e.g. `npm run cargo:test -- stack_icon` and `npm run cargo:test -- process_icon_cache` or the closest exact filters supported by Cargo.
- [ ] Run lint/check (`npm run check`)
- [ ] Run full automated test suite (`npm run test:node` and `npm run cargo:test`)
- [ ] Run project typecheck/build (`npm run build` and `npm run cargo:check`)
- [ ] Fix all failures
- [ ] Build passes

### Step 5: Documentation & Delivery

- [ ] Update `master_spec.md` Stack Browser and Process Manager sections to state that shell/process icon caches are bounded, name the cap constants, and summarize the eviction policy and lock-boundary guarantees.
- [ ] Update `changelog.md` according to `CHANGELOG_POLICY.md` with the user-visible/performance reliability change and validation performed.
- [ ] Document before/after cache-size behavior, including that previous process-lifetime HashMaps were unbounded and new steady-state size is capped by constants.
- [ ] Save documentation deliverables as task documents via `fn_task_document_write` (key="docs", content=...) summarizing changed files, cap values, eviction behavior, and validation commands/results.
- [ ] Out-of-scope findings created as new tasks via `fn_task_create` tool.

## Documentation Requirements

**Must Update:**
- `master_spec.md` — document bounded Stack Browser shell icon cache and Process Manager process icon cache behavior, cap constants, and lock-light extraction boundary.
- `changelog.md` — add a policy-compliant entry for FN-004 and validation results.

**Check If Affected:**
- `README.md` — update only if it already describes icon-cache behavior or performance characteristics affected by this change.

## Completion Criteria

- [ ] All steps complete
- [ ] Stack icon cache has a constants-based capacity cap and tested eviction behavior
- [ ] Process icon cache has a constants-based capacity cap and tested eviction behavior
- [ ] Cache hits still avoid shell extraction and extraction still happens outside cache locks
- [ ] Large-folder/process-manager smoke tests demonstrate capped steady-state size and no correctness regression
- [ ] Lint/check passing
- [ ] All tests passing
- [ ] Typecheck/build passing
- [ ] Documentation updated

## Git Commit Convention

Commits at step boundaries. All commits include the task ID:

- **Step completion:** `feat(FN-004): complete Step N — description`
- **Bug fixes:** `fix(FN-004): description`
- **Tests:** `test(FN-004): description`

## Do NOT

- Expand task scope beyond bounding shell/process icon caches and related tests/docs
- Skip tests
- Refuse necessary fixes just because they touch files outside the initial File Scope
- Commit without the task ID prefix
- Remove, delete, or gut modules, settings, interfaces, exports, or test files outside the File Scope
- Remove features as "cleanup" — if something seems unused, create a task via `fn_task_create`
- Hold cache mutexes while calling Win32 shell icon extraction or fake extractors in tests
- Replace Stack Browser icon hydration batching/concurrency behavior in the frontend as part of this backend cache-cap task
- Persist icon caches to disk or add user settings unless separately requested

## Changeset Requirements

If this task REMOVES existing functionality (deleting modules, settings, API endpoints, or exports), a changeset file is REQUIRED:
- Create `.changeset/fn-004-removal.md` explaining what was removed and why
- This is mandatory for any net-negative change (more deletions than additions to existing files)
