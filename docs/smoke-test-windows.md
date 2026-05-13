# Windows Live Smoke Test Checklist

Status: current live-smoke checklist for JasonShell lazy auxiliary-window validation and tray/command/process surfaces.

Use this after the static validation gates pass. These checks require a Windows desktop session, WebView2, and `npm run tauri dev`; they are not replaced by Node or Rust unit tests.

## Preflight

- Confirm `npm run check`, `npm run build`, `npm run test:node`, `npm run cargo:test`, and `npm run cargo:check` pass or record the exact failure.
- Start JasonShell with `npm run tauri dev`.
- Watch the terminal for top-bar and bottom-bar runtime metrics; both bars should report nonzero native and WebView heights.
- Keep a way to terminate the app if AppBar reservation or Explorer taskbar restoration misbehaves.

## Shell Surfaces

- Top bar appears on the primary monitor top edge and reserves work area.
- Bottom bar appears on the primary monitor bottom edge and reserves work area.
- Top and bottom bars remain topmost over ordinary windows.
- Top and bottom bars are excluded from Alt+Tab and task switching.
- Explorer taskbar is hidden only while JasonShell is active and is restored after exit.
- Cold startup creates only `top-bar` and `bottom-bar`; auxiliary windows are created on first use.

## Lazy Auxiliary Window Lifecycle Matrix

Record a PASS/FAIL/NOT RUN result for every cell during the release smoke. `first open` means the window did not exist before the action; `second open` means the existing hidden/shown Tauri window is reused without duplicate labels or missing events. Do not pre-fill PASS results without a live desktop run; for FN-002's automated worktree run, these cells were recorded as NOT RUN in the task document and covered by source-contract tests instead.

| Surface | First open | Second open | Hide/close | Focus-loss cycle | Required event/session notes |
| --- | --- | --- | --- | --- | --- |
| `search-panel` | NOT RUN: verify top-bar search/Ctrl+K creates and displays current query/results | NOT RUN: verify reopen without stale payload loss | NOT RUN: verify explicit close resets top-bar search state | NOT RUN: verify native blur emits `search-panel:closed` to `top-bar` only | Close uses shared reset path |
| `stack-popup` | NOT RUN: verify pinned folder creates and emits `stack-popup:open` | NOT RUN: verify same folder/history without duplicate window | NOT RUN: verify hide is idempotent before creation | NOT RUN: verify top-bar pinned-folder focus hold suppresses accidental close | Geometry persists across resize |
| `process-manager` | NOT RUN: verify bottom-bar process button creates and emits `process-manager:open` | NOT RUN: verify rows refresh on reopen | NOT RUN: verify close stops polling/invalidates in-flight refreshes | NOT RUN: verify focus loss hides existing panel without creating a missing one | Kill guardrails remain unchanged |
| `settings-panel` | NOT RUN: verify JasonShell button creates anchored panel | NOT RUN: verify persisted settings on reopen | NOT RUN: verify hide before first open is a no-op | NOT RUN: verify focus loss clears top-bar expanded state | No label/capability rename |
| `tray-panel` | NOT RUN: verify tray button creates and emits `tray-panel:open` | NOT RUN: verify icons reload on every show | NOT RUN: verify hide before first open is a no-op and emits top-bar close | NOT RUN: verify tray invoke suppresses exactly one focus-loss close | Native tray relay keeps panel open |
| `command-panel` | NOT RUN: verify `>_` button creates panel | NOT RUN: verify saved command list on reopen | NOT RUN: verify close emits `command-panel:closed` to top bar | NOT RUN: verify focus loss clears top-bar expanded state | Safe command validation unchanged |
| `audio-panel` | NOT RUN: verify sound control creates panel and emits `audio-panel:open` | NOT RUN: verify reopen without hidden polling | NOT RUN: verify close emits to top bar and existing audio-panel owner | NOT RUN: verify focus loss stops visible polling state | Own close listener stops refresh polling |
| `calendar-panel` | NOT RUN: verify clock/time pill creates panel | NOT RUN: verify current month state on reopen | NOT RUN: verify close emits to top bar and existing calendar owner | NOT RUN: verify focus loss clears top-bar expanded state | Date/time updates continue |
| `terminal-panel` | NOT RUN: verify terminal button creates/focuses panel and emits `terminal-panel:open` | NOT RUN: verify existing panel and tabs reopen | NOT RUN: verify hide/show does not stop live backend sessions | NOT RUN: verify focus loss hides panel without terminating sessions | Sessions start/attach according to `TerminalPanelSurface.svelte` and retained output replays |
| `control-plane` | NOT RUN: verify centered/clamped creation | NOT RUN: verify existing dashboard/settings surface reopens | NOT RUN: verify hide before first open is a no-op | NOT RUN: verify focus loss leaves shell stable | Capability remains `control-plane` only |
| `task-preview` | NOT RUN: verify first hover creates preview and publishes payload | NOT RUN: verify later hover reuses preview window | NOT RUN: verify hide before creation is a no-op and stale hides are ignored | NOT RUN: verify pointer leave/focus transitions respect generation guards | Native/GDI preview state clears without mutex-held window ops |

## Top Bar And Search

- Date/time updates once per second.
- Command button (`>_`) appears immediately left of tray; tray appears immediately left of sound.
- Opening command, tray, sound, or search closes any other top-bar popup (mutual exclusivity).
- Ctrl+K focuses search and opens `search-panel`.
- Typing a query updates search results without clearing visible state prematurely.
- Keyboard selection and Enter activation work for a result.
- Pinning a folder from search updates the top-bar pin rail immediately.

## Tray Panel

- Clicking the tray down-arrow opens `tray-panel` anchored under the top bar.
- Tray list shows visible + overflow Explorer notification-area entries with stable source-qualified ids.
- Left click relays native Explorer tray behavior (for example volume/network flyouts).
- Right click opens native Explorer tray context menus (not a JasonShell custom menu).
- Left/right tray icon activation keeps `tray-panel` open; stale icon failures show inline error text instead of collapsing the panel.
- `tray-panel` closes on focus loss and top-bar `aria-expanded` state clears.

## Command Panel

- Clicking the command button (`>_`) opens `command-panel` anchored under the top bar.
- Saved-command list supports Run/Edit/Delete and remains responsive while other top-bar popups are closed.
- Editor supports Label, Mode, Program/Script path, Working directory, and Arguments (one arg per line).
- Save persists entries through `load_shell_settings`/`save_shell_settings`; restart confirms persistence.
- Running a command in `direct`, `powershellFile`, and `cmdFile` mode succeeds for known-safe sample commands and closes the panel on success.
- Invalid entries (for example secret-like args, non-absolute script path modes, empty label/target) show inline validation errors inside the popup.
- `command-panel` closes on focus loss and top-bar `aria-expanded` state clears.

## Stack Browser

- Clicking a pinned folder opens `stack-popup` anchored below the pin.
- Folder navigation, back/forward/up, sorting, selection, and inline rename/new-folder behavior remain usable.
- Right-click row and background menus fit inside the visible screen.
- Native Explorer folder drops onto the top bar are accepted when valid.
- Mouse XButton back/forward behavior works when the stack popup has focus.

## Bottom Bar

- Explorer taskbar `.lnk` launchers enumerate and launch.
- App-managed quick icons render before Explorer pins; Terminal/Spotify/app-alias launch failures keep icons visible and show non-crashing error feedback.
- Right-click app-managed quick icons shows `Unpin from quick icons`; unpin removes only that app-managed entry and preserves Explorer taskbar pins.
- Open windows group by application identity and activate/minimize with taskbar-like behavior.
- Reordering task groups with pointer drag does not trigger accidental activation.
- Hover previews show for task groups, stay open while pointer moves into/within the preview, and hide after leaving both tile and preview.
- Preview red X closes the previewed external window, refreshes bottom-bar windows, hides the preview, and must refuse JasonShell/internal windows.
- Native group menus refresh window state after actions.

## Process Manager

- The rightmost bottom-bar process button opens `process-manager` above the bar.
- Process rows refresh while open and stop polling after focus-loss close.
- Sorting works for visible columns, including Start Time when values are available.
- Kill action refuses protected/current-shell targets and refreshes after attempts.

## Notes

- `npm run tauri dev` may fail when ports `1420`/`1421` are already occupied; stop the conflicting dev server and retry.
- If `cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --` reports AppBar/work-area warnings, capture logs and continue manual popup/relay checks before marking smoke complete.
