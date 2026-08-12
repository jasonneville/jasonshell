# Code Context

## Files Retrieved
1. `master_spec.md` (lines 1-43) - canonical project/spec rules and current Stack Browser/control guidance; relevant notes: Stack Browser uses `stack-popup`, direct controls should use `MeltActionButton` when safe, Git dialog is a special raw-button exception.
2. `src/components/StackPopupSurface.svelte` (lines 1098-1110, 2368-2528) - Stack Browser close handler already exists and root/header markup shows where a new top-right X would be inserted.
3. `src/components/StackPopupSurface.css` (lines 1-24, 763-890) - root `.stack-popup` is `position: relative`; current toolbar/action layout and button styling.
4. `src/components/TaskPreviewSurface.svelte` (lines 67-80, 156-162) - task preview red rectangular X pattern with event suppression and `MeltActionButton`.
5. `src/components/TaskPreviewSurface.css` (lines 37-50, 121-143) - rectangular red X CSS: absolute top/right, danger red, `var(--js-radius-xs)`, min-width, not circular.
6. `src/components/ProcessManagerSurface.svelte` (lines 162-174, 288-308) - process manager hide/close flow and top-right `MeltActionButton` X.
7. `src/components/ProcessManagerSurface.css` (lines 15-25, 110-134) - header reserves right padding and close button CSS copied from preview style.
8. `src/lib/stackPopup.ts` (lines 320-321) - frontend wrapper for `hideStackPopup()`.
9. `src-tauri/src/stack_popup.rs` (lines 277-281) and `src-tauri/src/stack_popup/popup_window.rs` (lines 96+) - backend hide command path used by existing Stack close handler.
10. `tests/processManagerCloseButton.test.mjs` (lines 1-58) - source/CSS contract for process-manager rectangular X.
11. `tests/taskPreviewRetention.test.mjs` (lines 52-73) - source/CSS contract for task-preview X behavior.
12. `tests/taskPreviewTextPolish.test.mjs` (lines 23-38) - header padding/absolute positioning contract for preview close button.
13. `package.json` (lines 5-14) - validation scripts.

## Key Code

Stack Browser already has the behavior needed for an exit control:

```ts
// src/components/StackPopupSurface.svelte:1106-1110
async function closeStackPopupFromSurface() {
  await stopCurrentStackTerminal();
  stackBrowserViewMode = 'files';
  await hideStackPopup();
}
```

Escape already calls it when not closing inline/menu state:

```ts
// src/components/StackPopupSurface.svelte:2211-2215
} else if (rowMenu || backgroundMenu) {
  closeMenus();
} else {
  void closeStackPopupFromSurface();
}
```

Likely insertion point is inside/near the Stack Browser root header:

```svelte
// src/components/StackPopupSurface.svelte:2374-2382
<section ... class="stack-popup" aria-label="Stack browser" ...>
  <header class="stack-toolbar">
```

Use the direct-command `MeltActionButton` pattern (not raw `<button>`) because the Stack Browser spec says safe command controls use `MeltActionButton`, while raw buttons are called out as Git-dialog-only exceptions.

Reference markup patterns:

```svelte
// src/components/TaskPreviewSurface.svelte:156-162
<MeltActionButton
  class="preview-close-button"
  ariaLabel="Close previewed window"
  onClick={(event) => void handlePreviewClose(event)}
>×</MeltActionButton>
```

```svelte
// src/components/ProcessManagerSurface.svelte:303-307
<MeltActionButton
  class="process-manager-close-button"
  ariaLabel="Close process manager"
  onClick={() => void requestClose()}
>×</MeltActionButton>
```

Reference rectangular X CSS:

```css
/* src/components/ProcessManagerSurface.css:110-130 and TaskPreviewSurface.css:121-139 */
...-close-button {
  align-items: center;
  background: #dc2626;
  border: 1px solid rgba(255, 255, 255, 0.35);
  border-radius: var(--js-radius-xs);
  color: #fff;
  display: inline-flex;
  font-size: 0.75rem;
  font-weight: 800;
  height: 1.35rem;
  justify-content: center;
  line-height: 1;
  min-width: 2.1rem;
  padding: 0 0.42rem;
  position: absolute;
  right: 0.42rem;
  top: 0.42rem;
  z-index: 4;
}
```

Stack root CSS already supports absolute positioning:

```css
/* src/components/StackPopupSurface.css:1-13 */
.stack-popup {
  ...
  padding: var(--js-space-4);
  position: relative;
  width: 100%;
}
```

## Architecture

`stack-popup` is a hidden persistent Svelte/Tauri webview opened from top-bar pinned folders. `StackPopupSurface.svelte` owns UI state, uses wrappers from `src/lib/stackPopup.ts`, and calls the Tauri command `hide_stack_popup` through `hideStackPopup()`. The existing `closeStackPopupFromSurface()` also stops any legacy/current Stack terminal state and resets view mode before hiding; a new X should call this same function rather than invoking `hideStackPopup()` directly.

Task preview and process manager X buttons are local surface controls styled as absolute, red rectangular buttons at `top/right: 0.42rem`, with `min-width: 2.1rem`, `height: 1.35rem`, `border-radius: var(--js-radius-xs)`, hover `#ef4444`, and `z-index: 4`. Process manager reserves header space via right padding; preview reserves header text space with `padding-right`. Stack Browser currently has `.stack-toolbar` as a grid and `.stack-actions` flex wrapping; adding an absolute X likely requires reserving top/right room so it does not overlap path/action controls. Options: add right padding to `.stack-popup` or `.stack-toolbar`, or add a class like `.stack-browser-close-button` positioned absolute and adjust `.stack-toolbar`/`.stack-actions` for spacing.

## Start Here

Start with `src/components/StackPopupSurface.svelte`: add a `MeltActionButton` near the top of the `.stack-popup` section that calls `closeStackPopupFromSurface()`, then style it in `src/components/StackPopupSurface.css` by matching `.process-manager-close-button` / `.preview-close-button`.

Likely test to add: a source/CSS contract test such as `tests/stackBrowserCloseButton.test.mjs`, or extend `tests/stackPopupState.test.mjs` / `tests/meltMigrationWiring.test.mjs`. Assert:
- `class="stack-browser-close-button"` (or chosen class)
- `ariaLabel="Close stack browser"` / accessible label
- content `×`
- `onClick={() => void closeStackPopupFromSurface()}` (or event form)
- handler body still calls `stopCurrentStackTerminal()`, resets `stackBrowserViewMode = 'files'`, and awaits `hideStackPopup()`
- CSS has absolute top/right, red background, rectangular radius, min-width, not `border-radius: 999px`
- layout reserves room to avoid overlap.

Validation commands:
- Focused: `node --test tests/stackBrowserCloseButton.test.mjs` after compiling dist-tests if needed by the test style.
- Focused existing contracts: `npm run test:node` (runs `clean-dist-tests`, `tsc -p tsconfig.test.json`, then all `tests/*.test.mjs`).
- Svelte/type check: `npm run check`.
- Full frontend build: `npm run build`.

Spec/changelog implications:
- Behavior/UI change: update `master_spec.md` Stack Browser section to mention the top-right rectangular X exit button, its `closeStackPopupFromSurface()` behavior, and that it follows task-preview/process-manager rectangular X styling.
- Add a `changelog.md` entry per `CHANGELOG_POLICY.md` because visible behavior changed and tests likely changed.
- No backend/capability change expected; existing `hide_stack_popup` command/wrapper is already present.
