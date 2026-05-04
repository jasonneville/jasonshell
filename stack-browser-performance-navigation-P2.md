# Stack Browser Performance Navigation (P2)

## Goal

Fix Stack Browser core navigation: margin drag selection, path autocomplete, nonblocking icon loading, and reliable Reveal in Explorer.

## Source Items

- `updates.md` item 3: drag selection from full outside margin.
- `updates.md` item 9: path autocomplete while typing.
- `updates.md` item 10: do not freeze while large-folder icons load.
- `updates.md` item 11: Reveal in Explorer always populated for folders.

## Priority Rationale

P2. Stack Browser is central file navigation. These are correctness/perf fixes needed before new file actions.

## Implementation Map

- Stack UI: `src/components/StackPopupSurface.svelte` or current stack surface component found from `src/App.svelte` route.
- Stack state/view model: `src/lib/stackPopupState.ts`, `src/lib/stackPopupViewModel.ts`, `src/features/stack-browser/viewModel.ts`.
- Stack IPC wrapper: `src/lib/stackPopup.ts`.
- Backend: `src-tauri/src/stack_popup.rs`, shell helpers in `src-tauri/src/shell_paths.rs` if reveal/open changes.
- Existing tests likely named `stackPopup*.test.mjs` and Rust stack tests.

## Phase 1: Margin Selection RED Tests

### Work

- Read current marquee selection implementation and identify hit-test function or pointerdown handler.
- Define allowed start targets: details body blank area, virtual top/bottom spacers, left/right margins up to visual edge.
- Define blocked start targets: row button, row checkbox/icon/text, inline editor, toolbar, path textbox, context menu, resize grip.

### Tests

- Add `tests/stackBrowserMarginSelection.test.mjs` pure tests for a new/existing helper like `getMarqueeStartZone(target, bounds)`.
- Add source test asserting pointerdown handler is attached to details body/margin container, not only a narrow spacer.
- Add regression source test asserting row buttons and resize grip still stop/avoid marquee start.

### Acceptance Criteria

- RED tests fail because current margin hit area is too small.
- Test cases include left edge, right edge, blank middle, virtual spacer, row, resize grip.

## Phase 2: Margin Selection GREEN Implementation

### Work

- Extract minimal helper in `src/features/stack-browser/viewModel.ts` or `src/lib/stackPopupViewModel.ts`: `classifyMarqueeStartTarget(...)`.
- Update pointerdown handler to call helper and start marquee from full margin/background.
- Do not alter row virtualization math unless tests prove it is needed.
- Preserve Ctrl/meta additive selection and default replace selection.

### Tests

- Run `npm run test:node -- stackBrowserMarginSelection` or focused equivalent.
- Run existing stack popup Node tests.

### Acceptance Criteria

- Drag from outside margin starts rectangle selection.
- Row click, double-click, context menu, resize, and native drag/drop still behave.

## Phase 3: Path Autocomplete RED Tests

### Work

- Find current path textbox and folder navigation command.
- Define TS contract: `getPathAutocompleteQuery(input, caret)` returns parent dir + typed segment.
- Define Rust contract: `suggest_stack_paths({ parentPath, segment, limit })` returns directories only, sorted case-insensitively, bounded, structured error.
- Define UI contract: suggestions popup under path input, Arrow keys move, Enter/Tab commit, Escape closes suggestions.

### Tests

- Add pure TS tests for query parsing: `C:\`, `C:\dev\ja`, UNC path if supported, relative/empty input, trailing slash.
- Add Rust tests in `stack_popup.rs` for bounded directory suggestions, inaccessible path, non-directory parent, hidden/system folder handling if existing listing respects it.
- Add source test proving path textbox subscribes to input and renders suggestions with keyboard handlers.

### Acceptance Criteria

- RED tests fail because suggestion command/UI absent.
- Tests cover max result limit and stale request sequence.

## Phase 4: Path Autocomplete GREEN Implementation

### Work

- Add IPC command constant to `src/ipc/commands.ts` and TS wrapper in `src/lib/stackPopup.ts` only after RED tests.
- Implement Rust `suggest_stack_paths` using existing normalized path/listing helpers; return directories only and cap at e.g. 20.
- Add frontend suggestion state: `suggestions`, `selectedSuggestionIndex`, `suggestionRequestSeq`.
- Reject stale suggestion responses if input changed.
- Commit suggestion by updating path input and invoking existing navigate/open folder flow.

### Tests

- Run focused autocomplete TS/Rust tests.
- Run existing stack tests.

### Acceptance Criteria

- Typing path segment displays directory completions.
- Keyboard/mouse commit navigates correctly.
- Inaccessible path shows no crash and no stale suggestions.

## Phase 5: Icon Loading And Reveal RED Tests

### Work

- Trace `read_stack_folder` and `resolve_stack_item_icons` flow.
- Define invariant: first folder payload contains metadata rows without waiting for icon extraction.
- Define Reveal contract: every valid folder row and background/current folder context menu includes `Reveal in Explorer`; disabled/error only for invalid/missing path.

### Tests

- Add source test blocking `resolveStackItemIcons` or icon batch await inside initial folder read/render path.
- Add state test: large folder metadata rows become visible while icon status remains `loading`.
- Add context-menu test for folder row and background menu containing reveal action.
- Add Rust test for `reveal_stack_path`/existing reveal command accepting directory paths and returning structured errors for missing paths.

### Acceptance Criteria

- RED tests fail for missing Reveal or any sync icon-blocking behavior.
- Tests cover large list, stale icon response, valid folder, missing folder.

## Phase 6: Icon Loading And Reveal GREEN Implementation

### Work

- If icons still block metadata, split backend response so `read_stack_folder` returns rows without icon data.
- Keep icon hydration in bounded batches; enforce request sequence/path key so stale icons cannot update newer folder.
- Populate Reveal menu from normalized `item.path` for folder rows and current `folderPath` for background.
- Use existing Windows Explorer reveal/open command; add wrapper only if no command exists.

### Tests

- Run focused Stack Browser Node tests.
- Run `cargo test --manifest-path src-tauri/Cargo.toml stack_popup`.
- Run `npm run test:node`.

### Acceptance Criteria

- Large folders render rows before icons finish.
- UI remains responsive while icons hydrate.
- Reveal in Explorer is always available for valid folders.

## Phase 7: Refactor, Spec, Validation

### Work

- Keep autocomplete helpers pure and colocated with stack view-model code.
- Avoid introducing broad filesystem scans or unbounded suggestion reads.
- Update `master_spec.md` Stack Browser section and Change Ledger.

### Tests

- Run `npm run test:node`.
- Run `npm run cargo:test`.
- Run `npm run validate`.

### Acceptance Criteria

- Full validation passes.
- Manual smoke confirms margin selection, autocomplete, large folder non-freeze, and Reveal in Explorer.
