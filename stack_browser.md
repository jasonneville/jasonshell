# Stack Popup / Stack Browser

## Objective

Implement the `features.md` Stack Popup as a persistent shell-owned folder browser opened from pinned folders in the top bar. The popup should support repeated short file-management tasks without launching a separate Explorer window.

## Implemented Behavior

- JasonShell creates one hidden `stack-popup` webview at startup and reuses it for every pinned folder.
- Pinned folders are stored in app-local data as `stack-folders-v1.json`.
- Folder search results expose a `Pin` action; pinned folders appear in the top-bar folder rail.
- Clicking a pinned folder opens the stack popup anchored below that top-bar button.
- The popup keeps its Svelte history state while hidden, so opening folder A, hiding it, opening folder B, and pressing Back returns to folder A.
- The popup shows a details grid with sortable name, type, size, and modified columns while keeping folders grouped before files.
- Hidden, system, read-only, symlink, and reparse-point rows carry metadata indicators; hidden/system rows render subdued and read-only/link rows get visual rails/badges.
- Opening a folder focuses the details grid so keyboard navigation applies immediately.
- Pinned folders can be reordered in the top-bar rail by dragging pins; the order is persisted.

## File Operations

- Folder rows open inside the popup.
- File rows open through the existing shell path launcher.
- Copy and cut set the Windows file clipboard where available and also store the operation in JasonShell runtime state.
- Paste applies the current JasonShell stack clipboard into the active folder and refreshes the visible rows.
- Rename validates that the new name is a child name, rejects path separators, and fails if the destination already exists.
- Paste chooses Explorer-style collision names with `- Copy (n)` suffixes.
- Folder reads consume all backend pages and surface partial-listing warnings when individual entries cannot be inspected.
- Stack item metadata distinguishes hidden, system, read-only, symlink, and reparse-point entries so display and copy safeguards can make those states visible.
- Pin persistence writes through a temporary file and rename, and corrupt pin JSON is backed up before the rail falls back to an empty/default load.

## Main Interfaces

- Frontend wrapper: `src/lib/stackPopup.ts`
- Frontend navigation reducer: `src/lib/stackPopupState.ts`
- Popup UI: `src/components/StackPopupSurface.svelte`
- Top-bar pins and search pin integration: `src/components/TopBar.svelte`
- Backend commands and file operations: `src-tauri/src/stack_popup.rs`
- Window registration: `src-tauri/src/shell_windows.rs`

## Validation

- TypeScript state tests cover persistent history, branch navigation, stale folder payload rejection, selection preservation, sort behavior, and size/name formatting.
- Rust tests cover rename validation, folder details ordering, attribute helpers, pin reorder/corrupt-backup helpers, paste collision naming, and clipboard-mode stability.
- Full project validation should run with `npm run validate`.

## Known Limits

- Very large folders are fully enumerated through backend pages, but the current UI still renders the accumulated rows directly rather than virtualizing them.
- Paste consumes JasonShell's runtime clipboard state; copy/cut also publishes native Windows clipboard data for Explorer interoperability.
- External folder changes are refreshed through explicit reload after operations, not a long-lived watcher.
