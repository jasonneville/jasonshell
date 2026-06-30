Model: openai-codex/gpt-5.5
Reasoning Level: high

# Split-Pane Terminal Binary Tree UI Implementation Plan

## 1. Requirements & Constraints

- **REQ-001**: Replace the current two-pane `TerminalSplitOrientation` model in `src/components/TerminalPanelSurface.svelte` with a binary split tree that supports more than two visible panes.
- **REQ-002**: Add split-right and split-down behavior for the active pane.
- **REQ-003**: Add draggable split gutters with persisted in-memory split ratios for the current panel instance.
- **REQ-004**: Add collapse-on-close: closing one pane promotes its sibling subtree without destroying unrelated visible pane sessions.
- **REQ-005**: Add zoom projection: zooming a pane renders only that pane while preserving the underlying tree, sessions, replay buffers, and ratios.
- **REQ-006**: Add directional focus over the tree geometry for left/right/up/down pane navigation.
- **REQ-007**: Preserve existing per-pane xterm runtime ownership, event routing, replay buffers, polling, resize IPC, shell integration, clipboard handling, and current tab behavior.
- **REQ-008**: Add RED-first coverage where practical before implementation.
- **CON-001**: Do not add backend terminal session caps; backend `list_stack_terminals("terminal-panel")` remains authoritative.
- **CON-002**: Do not persist split layout across app restart unless explicitly added in a future task.
- **CON-003**: Preserve `Alt+Up` / `Alt+Down` command mark navigation; use a non-conflicting shortcut such as `Alt+Shift+Arrow` for directional pane focus.
- **GUD-001**: Update `master_spec.md` for durable behavior changes.
- **GUD-002**: Append concise history to `changelog.md` under `CHANGELOG_POLICY.md`.
- **PAT-001**: Keep pure layout/tree logic in `src/features/terminal/` and keep Svelte xterm lifecycle wiring in `src/components/TerminalPanelSurface.svelte`.

## 2. Implementation Steps

### Implementation Phase 1

- **GOAL-001**: Add pure binary tree layout model and failing tests before Svelte integration.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-001 | Create `src/features/terminal/terminalPaneLayout.ts`. Define `TerminalPaneSplitDirection = 'right' \| 'down'`, `TerminalPaneLeafNode`, `TerminalPaneSplitNode`, and `TerminalPaneLayoutNode`. Implement `createTerminalPaneLeaf`, `createTerminalPaneLayout`, `listTerminalPaneLeaves`, `findTerminalPaneLeaf`, `layoutContainsPane`, `splitTerminalPaneLayout`, `collapseTerminalPaneLayout`, `setTerminalSplitRatio`, `projectZoomedTerminalPaneLayout`, `projectTerminalPaneRects`, and `findDirectionalTerminalPane`. | None | File exports only pure TypeScript functions with no Svelte, DOM, xterm, Tauri, or global state imports. |
| TASK-002 | Add `tests/terminalPaneLayout.test.mjs`. Import from `../dist-tests/features/terminal/terminalPaneLayout.js`. Cover split-right, split-down, nested splits, ratio clamping, collapse leaf promotion, zoom projection, and directional focus based on projected rectangles. | TASK-001 | `pnpm test:node -- tests/terminalPaneLayout.test.mjs` fails before implementation and passes after TASK-001 is complete. |
| TASK-003 | Use deterministic IDs in tests: `pane-a`, `pane-b`, `pane-c`, `split-1`, `split-2`. Set default split ratio to `0.5` and clamp ratios to `0.15..0.85`. | TASK-002 | Tests assert exact tree shapes and ratio values after each operation. |

### Implementation Phase 2

- **GOAL-002**: Replace current flat/two-pane Svelte state with binary tree state while preserving existing runtime behavior.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-004 | In `src/components/TerminalPanelSurface.svelte`, replace current `TerminalSplitOrientation` and two-pane assumptions near lines `33-44`, `112-127`, `351-380`, and `421-436` with imports from `src/features/terminal/terminalPaneLayout.ts`. Add state: `terminalPaneLayoutRoot: TerminalPaneLayoutNode \| null`, `zoomedPaneId: string \| null`, and derived visible leaves from `projectZoomedTerminalPaneLayout(...)`. | TASK-001 | `TerminalSplitOrientation` is no longer present. No `terminalPanes.length >= 2` cap remains. Existing `paneRuntimes` map remains keyed by `paneId`. |
| TASK-005 | Update `ensurePrimaryPaneForSession`, `openSessionAsTab`, `createSplitPaneSession`, `splitTerminal`, `stopTerminal`, and `removePaneRuntime` to mutate `terminalPaneLayoutRoot` through pure layout helpers. New tab activation must still reset the visible workbench to a single leaf for the selected session. | TASK-004 | Creating a tab renders one leaf. Splitting active pane adds one new backend session and one new leaf. Closing one leaf collapses only that leaf and promotes sibling subtree. |
| TASK-006 | Replace the template body currently rendering `{#each terminalPanes as pane (pane.sessionId)}` near `src/components/TerminalPanelSurface.svelte:1966` with recursive rendering of `TerminalPaneLayoutNode`. Use a Svelte snippet or local recursive component to render split nodes and leaf nodes. Leaf markup must keep `use:bindPaneHost={pane}`, `on:mousedown|capture`, `on:wheel|capture|nonpassive`, and `on:contextmenu` behavior. | TASK-004 | Nested split DOM renders all visible leaves. Existing pane status overlay and xterm host behavior remain present for each leaf. |
| TASK-007 | Update `src/components/TerminalPanelSurface.css` around `.terminal-pane-grid` and split orientation rules at lines `245-268`. Add `.terminal-layout-tree`, `.terminal-layout-node`, `.terminal-layout-node.split-right`, `.terminal-layout-node.split-down`, and `.terminal-split-gutter` rules using CSS grid. Remove reliance on `data-split-orientation="vertical"` and `"horizontal"`. | TASK-006 | CSS supports nested split rows/columns, visible gutters, `box-sizing: border-box`, `min-width: 0`, `min-height: 0`, and hidden overflow. |

### Implementation Phase 3

- **GOAL-003**: Add split-right/down actions, resizing, zoom projection, and directional focus controls.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-008 | Update `src/features/terminal/terminalActions.ts`. Add action IDs `splitRight`, `splitDown`, `togglePaneZoom`, `focusPaneLeft`, `focusPaneRight`, `focusPaneUp`, and `focusPaneDown`. Either remove old `splitHorizontal` / `splitVertical` IDs or keep them as aliases only if existing tests require compatibility. Update labels to “Split right”, “Split down”, and “Zoom pane”. | TASK-004 | `terminalActions` exposes all new IDs with state-gated enablement. `canSplit` no longer depends on a two-pane cap. |
| TASK-009 | Update `runTerminalAction(...)` in `src/components/TerminalPanelSurface.svelte` near line `1400` to route `splitRight` to `splitTerminal('right')`, `splitDown` to `splitTerminal('down')`, `togglePaneZoom` to zoom projection, and directional focus actions to `focusDirectionalPane(direction)`. | TASK-008 | Toolbar/menu actions call the new functions and do not call removed two-pane orientation functions. |
| TASK-010 | Add split gutter pointer handlers in `TerminalPanelSurface.svelte`: `startTerminalSplitResize(splitNodeId, direction, event)`, `updateTerminalSplitResize(event)`, and `finishTerminalSplitResize(event)`. Use pointer capture. Compute ratio from the split node element width for `right` and height for `down`. Clamp with `setTerminalSplitRatio(...)`. Schedule `resizeAllVisiblePanes()` through animation frame while dragging and once after release. | TASK-007 | Dragging a gutter updates inline ratio state, refits affected xterms, and sends `resizeStackTerminal(sessionId, cols, rows, width, height)` for visible panes. |
| TASK-011 | Implement zoom projection with `zoomedPaneId`. If no pane is zoomed, render the full tree. If active pane is zoomed, render only `projectZoomedTerminalPaneLayout(root, activePaneId)` while preserving original `terminalPaneLayoutRoot`. Closing the zoomed pane clears `zoomedPaneId` if the pane no longer exists. | TASK-006 | Toggling zoom does not stop hidden sibling sessions. Unzoom restores the previous tree and ratios. |
| TASK-012 | Implement `focusDirectionalPane(direction)` using `findDirectionalTerminalPane(terminalPaneLayoutRoot, activePaneId, direction)`. Use full tree geometry even when zoomed; if zoomed, keep zoom enabled and project the newly focused pane. Add key handling for `Alt+Shift+ArrowLeft/Right/Up/Down`. Preserve existing `Alt+Up` / `Alt+Down` command mark behavior. | TASK-001, TASK-004 | Directional focus selects the nearest pane in the requested direction and updates `activePaneId`, `terminalPanes.focused`, `session`, and active runtime. |

### Implementation Phase 4

- **GOAL-004**: Update source-level coverage, durable docs, and validation.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-013 | Update `tests/persistentTerminalPanel.test.mjs`. Replace assertions for `TerminalSplitOrientation`, `splitVertical`, `splitHorizontal`, and two-pane grid CSS with assertions for `terminalPaneLayoutRoot`, `splitTerminalPaneLayout`, recursive layout rendering, gutter handlers, zoom projection, collapse-on-close, and no two-pane cap. | TASK-004, TASK-012 | Source tests fail if implementation reintroduces flat two-pane-only state. |
| TASK-014 | Update `tests/terminalActions.test.mjs` to assert new action IDs and labels. Preserve existing action gating tests for cwd, command output, and no buffer scanning during reactive state updates. | TASK-008 | Test verifies `splitRight`, `splitDown`, `togglePaneZoom`, and directional focus actions exist and are state-gated. |
| TASK-015 | Update `master_spec.md` terminal workbench section. Document binary tree split layout, split-right/down semantics, gutter ratios, collapse-on-close, zoom projection, and directional focus shortcut. | TASK-012 | `master_spec.md` describes current behavior without adding changelog ledger entries. |
| TASK-016 | Append concise `[CODE]` and `[TOOL]` entries to `changelog.md` under `## Change Ledger` per `CHANGELOG_POLICY.md`. | TASK-015 | Changelog records behavior/test changes and validation commands without secrets or long logs. |
| TASK-017 | Run validation commands: `pnpm test:node -- tests/terminalPaneLayout.test.mjs tests/terminalActions.test.mjs tests/persistentTerminalPanel.test.mjs`, then `pnpm build`, then `pnpm check` if available. | TASK-013, TASK-016 | All commands exit `0`. Any failure is fixed before handoff. |

## 3. Alternatives

- **ALT-001**: Keep the current flat `terminalPanes` array and add more CSS grid cases. Rejected because arbitrary nested splits, collapse promotion, and directional focus require tree topology.
- **ALT-002**: Implement resizing by resizing xterm only, without storing split ratios. Rejected because the layout would reset on Svelte updates and would not support deterministic testing.
- **ALT-003**: Persist split layout to settings immediately. Rejected because current terminal-panel runtime policy is conservative and non-restoring.
- **ALT-004**: Override `Alt+Up` / `Alt+Down` for pane focus. Rejected because those shortcuts already jump between command marks.

## 4. Dependencies

- **DEP-001**: `src/components/TerminalPanelSurface.svelte` — current terminal panel runtime and Svelte rendering owner.
- **DEP-002**: `src/components/TerminalPanelSurface.css` — current terminal pane and split layout styles.
- **DEP-003**: `src/features/terminal/terminalActions.ts` — action registry for split, focus, close, zoom, and toolbar/menu state.
- **DEP-004**: `src/lib/persistentTerminal.ts` — existing Tauri wrappers for terminal start/read/write/resize/stop/list.
- **DEP-005**: `@xterm/xterm`, `@xterm/addon-fit`, and `@xterm/addon-search` — existing per-pane terminal runtime dependencies.
- **DEP-006**: `CHANGELOG_POLICY.md` — changelog entry policy.

## 5. Files

- **FILE-001**: `src/features/terminal/terminalPaneLayout.ts` — new pure binary tree layout and focus model.
- **FILE-002**: `tests/terminalPaneLayout.test.mjs` — new RED-first unit tests for layout operations.
- **FILE-003**: `src/components/TerminalPanelSurface.svelte` — replace two-pane state, actions, rendering, resize, zoom, and focus behavior.
- **FILE-004**: `src/components/TerminalPanelSurface.css` — nested split tree and gutter styling.
- **FILE-005**: `src/features/terminal/terminalActions.ts` — action IDs and gating for split-right/down, zoom, and directional focus.
- **FILE-006**: `tests/persistentTerminalPanel.test.mjs` — source-level terminal panel behavior coverage.
- **FILE-007**: `tests/terminalActions.test.mjs` — terminal action registry coverage.
- **FILE-008**: `master_spec.md` — durable terminal workbench specification update.
- **FILE-009**: `changelog.md` — concise change ledger entry.

## 6. Testing

- **TEST-001**: Add `tests/terminalPaneLayout.test.mjs` for pure layout behavior.
- **TEST-002**: Update `tests/persistentTerminalPanel.test.mjs` for source-level Svelte/CSS ownership checks.
- **TEST-003**: Update `tests/terminalActions.test.mjs` for new action IDs and labels.
- **TEST-004**: Run `pnpm test:node -- tests/terminalPaneLayout.test.mjs tests/terminalActions.test.mjs tests/persistentTerminalPanel.test.mjs`.
- **TEST-005**: Run `pnpm build`.
- **TEST-006**: Run `pnpm check` if Svelte diagnostics are available in the local environment.
- **TEST-007**: Manual smoke check in desktop Tauri session: open terminal panel, split right, split down, drag gutters, zoom/unzoom active pane, close leaf panes, and verify directional focus.

## 7. Risks & Assumptions

- **RISK-001**: Recursive Svelte rendering may accidentally dispose hidden xterm views during zoom or tree updates. Mitigation: rely on replay buffers, keep backend sessions running, and test that close/zoom does not call `stopStackTerminal` for siblings.
- **RISK-002**: Directional focus can choose surprising panes in nested layouts. Mitigation: define deterministic rectangle projection and test exact choices.
- **RISK-003**: Gutter drag can produce invalid terminal dimensions. Mitigation: clamp ratios to `0.15..0.85`, call existing fit logic, and keep `resizeTerminalToFitForRuntime(...)` bounds behavior.
- **ASSUMPTION-001**: Split layout is in-memory only for this task. Verify by confirming no settings/localStorage writes are added.
- **ASSUMPTION-002**: Existing backend terminal APIs support required sessions without Rust changes. Verify by reusing `startPersistentTerminal`, `stopStackTerminal`, `resizeStackTerminal`, and `listStackTerminals`.

## 8. Related Specifications / Further Reading

- **REF-001**: `master_spec.md` — terminal panel and terminal workbench current behavior.
- **REF-002**: `CHANGELOG_POLICY.md` — changelog rules.
- **REF-003**: `docs/terminal-panel-prewarm-idle-policy.md` — terminal startup and idle-prewarm behavior.
- **REF-004**: `src/components/TerminalPanelSurface.svelte` — current terminal panel runtime owner.
- **REF-005**: `src/features/terminal/terminalActions.ts` — action registry.
- **REF-006**: `tests/persistentTerminalPanel.test.mjs` — current source-level terminal panel coverage.

---
The plan file is located at c:\dev\jasonshell\.harness\tasks\task_9ddeefbd-dfcc-4410-a734-da4a4dcf4e6d.md