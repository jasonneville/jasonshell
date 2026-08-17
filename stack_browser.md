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
- Paste copy and cut/move fallback paths write backend-only app-local recovery journals under `stack-browser-recovery/` unless `JASONSHELL_RECOVERY_JOURNAL_DISABLE` is set. Journals are private recovery artifacts, not user-visible history, and JasonShell classifies stale running records without automatic repair, rollback, or source deletion.
- Folder reads consume all backend pages and surface partial-listing warnings when individual entries cannot be inspected.
- Stack item metadata distinguishes hidden, system, read-only, symlink, and reparse-point entries so display and copy safeguards can make those states visible.
- Pin persistence writes through a temporary file and rename, and corrupt pin JSON is backed up before the rail falls back to an empty/default load.
- Windows clipboard interop uses RAII cleanup for clipboard sessions/global locks/owned memory, writes `Preferred DropEffect` before CF_HDROP, and clears partial publish state if file-list publish fails after DropEffect succeeds.

## Git And Subprocess Safety

- Stack Browser commands are backend caller-label guarded. Pin/show commands are limited to their owning surfaces, file/Git/archive commands are limited to `stack-popup`, search pinning remains allowed from `search-panel`, and terminal commands validate the stored session target rather than trusting request payload target labels.
- Stack Git keeps fixed argv and NUL-delimited pathspec stdin. Repo roots and absolute path requests are canonicalized before staging; missing/deleted relative paths are accepted only when present in a fresh backend Git status path set.
- Git and archive extraction use a bounded subprocess runner that drains stdout/stderr concurrently, retains at most 64 KiB per stream, applies timeouts, and kills owned processes on timeout. Git defaults: 10s read/probe, 30s local mutation, 90s remote. Archive extraction default: 10min. Env overrides are clamped.

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

- Very large folders are read through backend pages and the Svelte view owns virtualized visible-row calculation for the accumulated listing state; memory and row-state pressure can still grow with very large accumulated snapshots.
- Paste consumes JasonShell's runtime clipboard state; copy/cut also publishes native Windows clipboard data for Explorer interoperability.
- External folder changes are refreshed through explicit reload after operations, not a long-lived watcher.
