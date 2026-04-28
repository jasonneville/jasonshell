# Wave 1: Correctness And Safety

Status: historical Stack Browser roadmap note. Use `action_plan.md` for current phase ordering and `master_spec.md` plus source/tests for current behavior.

## Objective

Address the highest-risk correctness and safety blockers before adding broader Explorer-parity UI. Wave 1 makes Stack Browser directory views complete, prevents unsafe recursive traversal through links/reparse points, surfaces partial listing errors, and ensures Delete is not a permanent recursive filesystem removal by default.

## Phase 1: Folder Enumeration Contract

- Replace the frontend's single `limit: 500` folder read with a complete enumeration strategy.
- Preserve the backend paginated contract so large folders can be loaded incrementally without changing the Tauri command shape.
- Return and consume explicit page metadata: `path`, `items`, `offset`, `limit`, `total`, `hasMore`, and warning/error details.
- Update state status text to reflect total visible/reachable entries and partial-listing warnings.
- Ensure stale page responses cannot overwrite the active folder after navigation.

### Acceptance Criteria

- A folder with 501 or more entries is not truncated at 500.
- Frontend code continues requesting pages until `hasMore` is false, unless a hard folder-level error occurs.
- Status text reports the actual total once loading completes.
- Loading folder A, navigating to folder B, and receiving late folder A pages cannot replace folder B entries.

## Phase 2: Partial Listing And Entry Errors

- Stop silently discarding `read_dir` entry errors and per-entry metadata errors.
- Add a serializable warning shape for entries that cannot be inspected or read.
- Preserve all successfully inspected entries even if some siblings fail.
- Surface warnings in the Stack Browser status/error region without blocking navigation.

### Acceptance Criteria

- Backend page responses include warning messages when child entries cannot be read or inspected.
- Frontend displays a visible partial-listing warning count/message.
- Existing successful entries still render when sibling entries fail.
- Tests cover at least one partial-listing warning path through a deterministic helper.

## Phase 3: Reparse Point, Symlink, And Recursive Copy Safety

- Use `symlink_metadata` for item classification so symlinks/reparse points are not blindly followed.
- Add item metadata for `isSymlink` and `isReparsePoint` where supported.
- Treat symlink/reparse-point directory entries as link items for copy traversal decisions unless a later wave intentionally implements link-following semantics.
- Add recursive copy visited-path protection so cycles or canonical repeats cannot recurse indefinitely.
- Keep the self/descendant paste guard and strengthen it with canonicalized paths when possible.

### Acceptance Criteria

- Stack items expose symlink/reparse metadata to the frontend type layer.
- Recursive folder copy does not recurse into symlink/reparse-point child directories by default.
- Self/descendant copy/move protection remains intact.
- Tests cover symlink/reparse-safe copy behavior where the platform supports symlinks; unsupported platforms skip only the symlink-specific assertion.

## Phase 4: Recycle Bin Delete Semantics

- Replace permanent recursive delete as the default Windows delete path.
- On Windows, route delete through shell-backed Recycle Bin semantics.
- On non-Windows/test fallback, keep a deterministic delete helper but isolate it so Windows production does not use permanent recursive delete by default.
- Preserve clear error messages when the shell operation fails.

### Acceptance Criteria

- Windows `delete_stack_item` uses shell file operation flags that allow undo/recycle behavior.
- Permanent recursive deletion is not the default Windows code path.
- Existing backend command shape remains stable.
- Tests continue to verify delete behavior through the platform-appropriate fallback or helper without forcing destructive Windows shell behavior in unit tests.

## Phase 5: Wave 1 Tests And Verification

- Add Rust tests for large-folder pagination metadata, page iteration support, partial warnings, reparse/symlink-safe copy, and strengthened destination guards.
- Add TypeScript tests for complete paged folder accumulation behavior where possible without mocking Tauri internals; otherwise cover state application/status helpers.
- Run formatting and validation commands.

### Acceptance Criteria

- `npm run test:search` passes.
- `npm run cargo:test` passes.
- `npm run validate` passes or any environment-specific failure is documented with the narrower passing commands.
- No Wave 2 tasks begin until this wave passes verification.
