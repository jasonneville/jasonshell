# Wave 2: Pin, Drop, And Open Interoperability

## Objective

Make folder pinning from search, internal drag, Windows Explorer drag/drop, file URI payloads, UNC/network paths, and stale pin lifecycle behavior consistent and reliable. Every valid dropped folder should become visible on the top bar and immediately traversable in Stack Browser.

## Phase 1: Unified Top-Bar Drop Behavior

- Route all top-bar folder drop paths through one pin-and-open flow.
- Ensure `DataTransfer.files`, internal JasonShell folder payloads, `text/uri-list`, plain text paths, and Tauri native drop events all pin valid folders.
- Open Stack Browser for the first successfully pinned dropped folder using the drop target as the anchor.
- Keep multi-folder drops deterministic: all valid folders pin, and the first valid dropped folder opens.
- Surface a visible top-bar status/error message when no valid folder can be pinned.

### Acceptance Criteria

- Dropping a folder from Windows Explorer onto the top bar pins it and opens Stack Browser to that folder.
- Dropping multiple folders pins all valid folders and opens the first valid folder.
- Dropping a file or invalid path does not create a pin and produces visible feedback.
- Internal search-result drag/drop preserves current behavior and uses the same pin-and-open path.

## Phase 2: Robust File URI And UNC Normalization

- Normalize local file URIs: `file:///C:/path`, encoded spaces, and mixed slash paths.
- Normalize localhost file URIs: `file://localhost/C:/path`.
- Preserve UNC/network semantics: `file://server/share/path` becomes `\\server\share\path`.
- Keep shell alias resolution for supported `shell:` aliases.
- Apply equivalent normalization in frontend drag parsing and backend path normalization.

### Acceptance Criteria

- Frontend parser converts `file:///C:/Dev/My%20Repo` to `C:\Dev\My Repo`.
- Frontend parser converts `file://localhost/C:/Dev/Repo` to `C:\Dev\Repo`.
- Frontend parser converts `file://server/share/Repo` to `\\server\share\Repo`.
- Backend normalization accepts equivalent URI forms when the resolved path exists.
- Tests cover parser behavior without requiring an actual network share.

## Phase 3: Stale Pin Removal

- Allow unpinning a stale, missing, renamed, offline, or unavailable folder.
- Preserve canonical comparison for existing paths when possible.
- Fall back to raw normalized case-insensitive comparison when canonicalization fails.
- Keep `list_pinned_stack_folders` tolerant of stale paths.

### Acceptance Criteria

- A missing pinned path can be removed without resolving to an existing directory.
- Existing-path unpin behavior remains canonical/case-insensitive.
- Tests cover stale-path removal logic through a deterministic helper.

## Phase 4: Wave 2 Tests And Verification

- Add TypeScript parser tests for file URI, localhost, UNC, encoded paths, and multi-line URI lists.
- Add Rust tests for URI normalization helpers and stale pin removal helper behavior.
- Run `npm run test:search`, `npm run cargo:test`, and `npm run validate`.

### Acceptance Criteria

- All new tests pass.
- Full validation passes.
- No Wave 3 tasks begin until Wave 2 QA passes.
