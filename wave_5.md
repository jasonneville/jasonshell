# Wave 5: Display, Metadata, Accessibility, And Persistence Polish

## Objective

Close the remaining medium-severity Explorer-grade gaps: sortable/details display, Windows metadata visibility, focus management, ARIA/grid semantics, pin rail management polish, and resilient pin persistence.

## Phase 1: Metadata And Attributes

- Extend Stack item data to expose hidden, readonly, system, and link/reparse indicators through the frontend type layer.
- Use Windows file attributes where available instead of dot-name-only hidden detection.
- Render visible indicators or subdued styling for hidden/system/readonly/link items.
- Preserve existing symlink/reparse safety behavior from Wave 1.

### Acceptance Criteria

- Backend `StackItem` reports Windows hidden/system/readonly/link metadata where available.
- Frontend `StackEntry` carries the metadata without dropping it.
- Rows visually distinguish hidden/system/readonly/link items.
- Tests cover attribute helper behavior where deterministic.

## Phase 2: Sortable Details Display

- Add clickable column headers for Name, Type, Size, and Modified.
- Track sort column and direction in Stack Popup state/UI.
- Sort loaded entries client-side while preserving folders-first behavior.
- Render a visible sort indicator and announce sort direction.

### Acceptance Criteria

- Clicking a column changes sort column or toggles direction.
- Folders remain grouped before files.
- Name, Type, Size, and Modified sorts are deterministic.
- Tests cover sort helper behavior.

## Phase 3: Focus And ARIA Grid Hardening

- Focus the Stack Browser grid/root after opening a folder from a pin/drop/search.
- Replace weak table semantics with grid/list semantics appropriate for interactive rows.
- Expose `aria-selected`, row/column roles, and sort state.
- Ensure context menus and row focus are keyboard-accessible.

### Acceptance Criteria

- After Stack Popup opens, keyboard navigation applies to the browser without requiring a mouse click.
- Rows expose selected state to assistive technologies.
- Column headers expose current sort state.
- Svelte check reports no accessibility or TypeScript warnings.

## Phase 4: Pin Rail Management And Persistence Resilience

- Add basic pin reorder support in the top bar and persist reordered pins.
- Keep pin overflow/status behavior intact.
- Make pin persistence atomic with temp-file write plus rename.
- Recover from corrupt pin JSON by backing it up and returning default/empty pins instead of failing the whole rail.

### Acceptance Criteria

- Users can reorder pinned folders in the rail and the order persists.
- Pin store writes are atomic from the app's perspective.
- Corrupt pin JSON is backed up and does not prevent JasonShell from loading pins.
- Tests cover pin-store corrupt recovery and reorder helper behavior where practical.

## Phase 5: Documentation And Verification

- Update `stack_browser.md` to remove outdated limitations and document new display/interaction behavior.
- Run full `npm run validate`.

### Acceptance Criteria

- Documentation reflects current Stack Browser behavior.
- Full validation passes.
- Final QA can map every critical, high, and medium finding to an implemented fix or explicitly tested behavior.
