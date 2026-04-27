# Wave 4: Selection, Context, Keyboard, And Drag-Drop Parity

## Objective

Bring the Stack Browser interaction model up to a developer-grade Explorer baseline. Wave 4 adds multi-selection, batch operations, Stack Browser drag/drop file operations, custom context menus to replace globally suppressed native menus, and keyboard parity for core file-manager workflows.

## Phase 1: Multi-Selection State Model

- Extend Stack Browser state to support ordered `selectedPaths` and a range-selection anchor while preserving a primary selected item.
- Support plain click single selection, Ctrl/Meta-click toggle selection, Shift-click range selection, and Select All.
- Preserve only still-visible selected paths after refresh and navigation.
- Keep existing single-selection tests passing through compatibility helpers.

### Acceptance Criteria

- Ctrl/Meta-click toggles individual rows without clearing other selections.
- Shift-click selects the contiguous range between anchor and clicked row.
- Ctrl+A selects all visible rows.
- Refresh preserves selections that still exist and drops stale selections.
- Existing copy/cut/delete commands operate on all selected rows.

## Phase 2: Batch Operations

- Update frontend copy/cut/delete to pass every selected path.
- Keep Rename and Reveal scoped to the primary selected item.
- Refresh after batch operations and report operation failures.
- Ensure empty selection disables selection-dependent buttons.

### Acceptance Criteria

- Copy and cut send all selected paths to backend commands.
- Delete applies to all selected paths with one confirmation.
- Rename and Reveal remain disabled when no primary item exists.
- Batch operation errors surface visible status messages.

## Phase 3: Stack Browser Drag And Drop

- Make selected rows draggable with a JasonShell stack payload and text/plain fallback path list.
- Allow dropping files/folders onto a folder row to paste into that folder.
- Allow dropping files/folders onto the current folder background to paste into the current folder.
- Accept Explorer/Tauri file paths and internal Stack row drags.
- Use copy by default and move when the Shift modifier indicates move.

### Acceptance Criteria

- Dragging selected Stack rows onto a folder row copies/moves selected items into that folder.
- Dragging Explorer files/folders into the Stack Browser copies/moves them into the target folder/current directory.
- Dropping onto a non-folder row does not trigger a file operation.
- After drop, the relevant folder refreshes and operation failures are visible.

## Phase 4: Context Menus

- Add a custom context menu for stack rows with Open, Copy, Cut, Rename, Delete, Reveal.
- Add a custom context menu for the folder background with Paste and New Folder.
- Replace the pin rail right-click confirm-only behavior with a custom pin menu for Open and Unpin.
- Ensure menus close on Escape, click-away, operation execution, or window hide.

### Acceptance Criteria

- Right-clicking a Stack row selects it when needed and shows file actions.
- Right-clicking empty Stack Browser space shows folder actions.
- Right-clicking a pin shows Open and Unpin without relying on native context menus.
- Context menu actions invoke the same commands as toolbar actions.

## Phase 5: Keyboard Parity

- Add Delete, Ctrl+A, Ctrl+Shift+N, Backspace/Alt-Up, Home, End, PageUp, PageDown, Space on pin buttons, and type-to-select.
- Keep existing Escape, Enter, ArrowUp/Down, Ctrl+C/X/V, and F2 behavior.
- Support the keyboard context-menu key and Shift+F10 for row context menus.

### Acceptance Criteria

- Keyboard shortcuts map to the same command handlers as toolbar/context menu actions.
- Home/End/Page keys update selection predictably.
- Backspace/Alt-Up navigates to the parent directory when available.
- Type-to-select jumps to the next visible item matching typed prefix.
- Pin rail Space opens the focused pin.

## Phase 6: Wave 4 Tests And Verification

- Add reducer tests for multi-select, range-select, select-all, and selection preservation.
- Add unit tests for type-to-select or helper behavior where practical.
- Run full `npm run validate`.

### Acceptance Criteria

- All new and existing tests pass.
- Full validation passes.
- No Wave 5 tasks begin until Wave 4 QA passes.
