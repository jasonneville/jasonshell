# Stack Browser File Actions (P3)

## Goal

Add implementation-ready Stack Browser file actions: extract `.zip`/`.rar` archives and show smarter context menu actions including Properties.

## Source Items

- `updates.md` item 12: extract zip and RAR files with Windows or 7-Zip.
- `updates.md` item 13: smarter context menu actions including Properties.

## Priority Rationale

P3. New file operations should happen after P2 fixes Stack Browser navigation/performance correctness.

## Implementation Map

- Stack UI/menu: stack popup Svelte component, `src/lib/stackPopupViewModel.ts`, `src/features/stack-browser/viewModel.ts`.
- Stack IPC wrapper: `src/lib/stackPopup.ts`.
- Backend: `src-tauri/src/stack_popup.rs`; add small module only if command bodies become large, e.g. `src-tauri/src/archive_actions.rs`.
- Shell/path helpers: `src-tauri/src/shell_paths.rs`.
- IPC constants: `src/ipc/commands.ts` and `src-tauri/src/contracts.rs` if repository uses explicit command allowlists.

## Phase 1: Archive Extraction RED Tests

### Work

- Define pure archive type helper: `.zip`, `.rar`; reject directories and unsupported extensions.
- Define command input: `{ archivePath: string, destinationMode: 'here' | 'folder' }`.
- Define extraction plan output for tests before execution: executable/tool, args vector, destination path, expected created folder.
- Define tool preference: built-in Windows zip when safe; 7-Zip for RAR; 7-Zip fallback for zip if needed.

### Tests

- Add Rust unit tests for `ArchiveKind::from_path`: `.zip`, `.ZIP`, `.rar`, unsupported `.7z`, extensionless, directory path.
- Add Rust tests for 7-Zip discovery candidates: `%ProgramFiles%\7-Zip\7z.exe`, `%ProgramFiles(x86)%\7-Zip\7z.exe`, PATH fallback if existing helper supports it.
- Add Rust tests proving extraction args are vectorized: paths with spaces produce separate args and never build a shell string.
- Add Node/source test proving Stack Browser context menu shows Extract actions only for supported file rows.

### Acceptance Criteria

- RED tests fail because archive command/menu does not exist.
- Tests cover missing archive, unsupported type, missing 7-Zip for RAR, and destination collision.

## Phase 2: Archive Extraction GREEN Implementation

### Work

- Add command `extract_stack_archive` in Rust with serde camelCase input if that is project convention.
- Validate archive path: absolute, exists, file, supported extension.
- Compute destination: `here` uses parent dir; `folder` uses parent + archive stem, with safe conflict rule (`error` first, no overwrite in minimal GREEN).
- Implement `.zip` through existing Windows-safe extraction path or 7-Zip if easier and already available.
- Implement `.rar` through 7-Zip only; return `MissingTool` if unavailable.
- Add TS wrapper and command constant.
- Add menu items: `Extract here` and `Extract to <archive-name>\` for archive rows.

### Tests

- Run `cargo test --manifest-path src-tauri/Cargo.toml archive` or focused module tests.
- Run Stack Browser menu Node tests.
- Run `npm run test:node` if source tests added.

### Acceptance Criteria

- Supported archive rows show extraction actions.
- Unsupported rows do not show extraction actions.
- Command rejects invalid paths before spawning tools.
- Missing 7-Zip error is visible and non-crashing.

## Phase 3: Properties And Smart Menu RED Tests

### Work

- Define menu model by item metadata: folder, file, archive, executable, shortcut, multi-selection, background/current folder.
- Define Properties command input: `{ path: string }`.
- Define Properties backend behavior: validate path exists, call Windows shell properties verb or equivalent safe native API; no shell string concatenation.
- Define ordering: Open/Open With, Reveal, Extract when archive, Rename/Delete, Properties near bottom.

### Tests

- Add Node pure tests for `buildStackContextMenuActions(item, selection, currentFolder)` if helper exists; otherwise extract it.
- Add source test proving Properties appears for valid file row, folder row, and background current folder.
- Add source test proving Properties is omitted/disabled for missing path and virtual/non-filesystem rows.
- Add Rust tests for `show_stack_item_properties` command validation and safe command plan.

### Acceptance Criteria

- RED tests fail because Properties/smart action model is missing or not explicit.
- Tests cover single row, multi-selection, background, archive, missing path.

## Phase 4: Properties And Smart Menu GREEN Implementation

### Work

- Extract or extend a pure menu-action builder in stack view-model code.
- Add action ids as constants, not repeated strings in Svelte markup.
- Implement `show_stack_item_properties` Rust command and TS wrapper.
- Wire menu action dispatch in stack Svelte component; show inline/toast error while keeping panel open.
- Preserve existing Open, Reveal, Rename, Delete, Copy path, and drag/drop actions.

### Tests

- Run Stack Browser menu Node tests.
- Run Rust properties tests.
- Run `npm run test:node` and `npm run cargo:test`.

### Acceptance Criteria

- Right-click file/folder/current folder shows Properties.
- Archive rows show Extract actions plus common file actions.
- Errors do not close Stack Browser or mutate selection.

## Phase 5: Refactor, Spec, Validation

### Work

- Keep archive spawning code isolated from UI/menu code.
- Add command names to `src-tauri/src/contracts.rs` and capability JSON under `src-tauri/capabilities/stack-popup.json` if Tauri permissions require it.
- Update `master_spec.md` Stack Browser behavior and Change Ledger.
- Add smoke steps: zip extract here, zip extract folder, RAR missing 7-Zip, Properties file/folder.

### Tests

- Run `npm run test:node`.
- Run `npm run cargo:test`.
- Run `npm run validate`.

### Acceptance Criteria

- Full validation passes.
- Archive extraction and Properties are covered by unit/source tests and manual smoke checklist.
