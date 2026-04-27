# Wave 3: Explorer-Grade Operations

## Objective

Expose the core file-management operations already expected from Explorer and harden backend operation semantics. Wave 3 makes Delete, New Folder, and Reveal available in the Stack Browser UI, supports Explorer-origin clipboard paste, improves move behavior across volumes, and returns partial-failure summaries instead of hiding operation errors.

## Phase 1: Frontend Command Surface

- Add TypeScript wrappers for `delete_stack_item`, `new_stack_folder`, and `reveal_stack_item`.
- Add Stack Browser toolbar buttons for Delete, New Folder, and Reveal.
- Refresh the current folder after each operation.
- Select newly created folders after creation.
- Surface backend error messages in the Stack Browser status region.

### Acceptance Criteria

- Users can create a folder from the Stack Browser toolbar.
- Users can delete a selected item from the Stack Browser toolbar; Windows backend routes deletion through the Wave 1 Recycle Bin path.
- Users can reveal a selected item in Windows Explorer.
- Operation failures show visible, specific status text.

## Phase 2: Native Explorer Clipboard Paste

- Read `CF_HDROP` from the Windows clipboard when JasonShell's internal stack clipboard is empty.
- Read `Preferred DropEffect` to distinguish copy vs move when available.
- Keep internal JasonShell copy/cut clipboard behavior intact.
- Use internal clipboard preferentially immediately after Stack copy/cut to preserve current behavior.

### Acceptance Criteria

- Copying files in Explorer and pasting into Stack Browser is supported on Windows.
- Cutting files in Explorer and pasting into Stack Browser moves when drop effect indicates move.
- Non-Windows builds continue to return a clear empty-clipboard error when no internal clipboard exists.
- Unit-testable helpers cover drop-effect mode selection.

## Phase 3: Move, Collision, And Failure Semantics

- Add cross-volume move fallback: if `fs::rename` fails, copy the source to the collision-safe target and delete the original only after copy succeeds.
- Preserve existing collision naming behavior.
- Return per-item operation failures in `StackPasteResult` while preserving successful items.
- Show partial-failure summaries in the UI after paste.

### Acceptance Criteria

- A move fallback can be unit-tested without needing multiple physical volumes.
- Paste can report successful and failed item operations in one response.
- Existing recursive-copy safety and self/descendant guards remain intact.

## Phase 4: Windows Child Name Validation

- Harden `validate_child_name` for Windows Explorer-compatible child names.
- Reject invalid characters: `<`, `>`, `:`, `"`, `|`, `?`, `*`, path separators, and control characters.
- Reject reserved device basenames: `CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, and `LPT1`-`LPT9`, including extension variants.
- Reject names ending in a dot or space.

### Acceptance Criteria

- Rename and New Folder use the same validator.
- Tests cover invalid characters, trailing dot/space, and reserved device names.
- Valid names with spaces and normal extensions remain accepted.

## Phase 5: Wave 3 Tests And Verification

- Add Rust tests for validation, move fallback helper behavior, partial paste result structures, and clipboard drop-effect mode mapping.
- Add frontend build/type coverage through existing validation.
- Run `cargo fmt --manifest-path src-tauri/Cargo.toml` and full `npm run validate`.

### Acceptance Criteria

- Full validation passes.
- No Wave 4 tasks begin until Wave 3 QA passes.
