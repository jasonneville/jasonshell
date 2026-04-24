# Stack Popup / Stack Browser

## Objective

Implement the `features.md` Stack Popup as a persistent shell-owned folder browser opened from pinned folders in the top bar. The popup should support repeated short file-management tasks without launching a separate Explorer window.

## Implemented Behavior

- JasonShell creates one hidden `stack-popup` webview at startup and reuses it for every pinned folder.
- Pinned folders are stored in app-local data as `stack-folders-v1.json`.
- Folder search results expose a `Pin` action; pinned folders appear in the top-bar folder rail.
- Clicking a pinned folder opens the stack popup anchored below that top-bar button.
- The popup keeps its Svelte history state while hidden, so opening folder A, hiding it, opening folder B, and pressing Back returns to folder A.
- The popup shows a details table with name, type, size, and modified date.

## File Operations

- Folder rows open inside the popup.
- File rows open through the existing shell path launcher.
- Copy and cut set the Windows file clipboard where available and also store the operation in JasonShell runtime state.
- Paste applies the current JasonShell stack clipboard into the active folder and refreshes the visible rows.
- Rename validates that the new name is a child name, rejects path separators, and fails if the destination already exists.
- Paste chooses Explorer-style collision names with `- Copy (n)` suffixes.

## Main Interfaces

- Frontend wrapper: `src/lib/stackPopup.ts`
- Frontend navigation reducer: `src/lib/stackPopupState.ts`
- Popup UI: `src/components/StackPopupSurface.svelte`
- Top-bar pins and search pin integration: `src/components/TopBar.svelte`
- Backend commands and file operations: `src-tauri/src/stack_popup.rs`
- Window registration: `src-tauri/src/shell_windows.rs`

## Validation

- TypeScript state tests cover persistent history, branch navigation, stale folder payload rejection, selection preservation, and size/name formatting.
- Rust tests cover rename validation, folder details ordering, paste collision naming, and clipboard-mode stability.
- Full project validation should run with `npm run validate`.

## Known Limits

- The popup currently loads a bounded folder page through the frontend wrapper rather than virtualizing every row.
- Paste consumes JasonShell's runtime clipboard state; copy/cut also publishes native Windows clipboard data for Explorer interoperability.
- External folder changes are refreshed through explicit reload after operations, not a long-lived watcher.
