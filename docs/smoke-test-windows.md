# Windows Live Smoke Test Checklist

Status: current live-smoke checklist for JasonShell tray-and-command phases 0-5 validation.

Use this after the static validation gates pass. These checks require a Windows desktop session, WebView2, and `npm run tauri dev`; they are not replaced by Node or Rust unit tests.

## Preflight

- Confirm `npm run check`, `npm run build`, `npm run test:node`, `npm run cargo:test`, and `npm run cargo:check` pass or record the exact failure.
- Start JasonShell with `npm run tauri dev`.
- Watch the terminal for top-bar and bottom-bar runtime metrics; both bars should report nonzero native and WebView heights.
- Keep a way to terminate the app if AppBar reservation or Explorer taskbar restoration misbehaves.

## Shell Surfaces

- Top bar appears on the primary monitor top edge and reserves work area.
- Bottom bar appears on the primary monitor bottom edge and reserves work area.
- Explorer taskbar is hidden only while JasonShell is active and is restored after exit.
- `task-preview`, `search-panel`, `stack-popup`, and `process-manager` open from their owning surfaces and close or hide without terminating the shell.

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
- Editor supports Label, Mode, target program/command block, Working directory, and Arguments (one arg per line for direct mode).
- Save persists entries through `load_shell_settings`/`save_shell_settings`; restart confirms persistence.
- Running known-safe `direct` and `commandBlock` entries starts a live run, shows bounded merged transcript updates, preserves UI responsiveness, and records completed history. Legacy `powershellFile`/`cmdFile` entries are migration-only and should not appear as new UI save modes.
- Quick-command normal input: run a safe script that emits versioned OSC `Request-JasonShellInput` with current backend runId, enter `hello`, submit, and verify backend writes `hello\r\n` to piped stdin, transcript shows submitted input, child output arrives in backend-arrival order, and completed history preserves bounded transcript/output.
- Quick-command secret input: run a safe script that requests secret input, submit a test value, and verify JasonShell transcript/history shows redacted submitted input. If the child echoes the value in stdout/stderr, record that as expected raw child output, not a JasonShell redaction failure.
- Quick-command malformed marker: run a safe script that emits a malformed/unsupported `Request-JasonShellInput` OSC marker. Verify no prompt is accepted for submission, malformed marker is handled without crashing, and later valid output/completion still renders.
- Quick-command stale input: start a prompt run, stop it or start a replacement run, then attempt to submit the old prompt. Verify backend rejects the submission by command/run/request validation and does not write to the new/stopped child stdin.
- Quick-command stop during input: while an input prompt is pending, click red Stop. Verify stop validates active command-id/PID/run ownership, kills the process tree, clears/settles pending prompt UI, and does not accept late input for the stopped run.
- Quick-command push/poll fallback: observe live output through push events under normal conditions; if event delivery is missed or panel is reopened, verify the 1100 ms aggregate polling fallback catches up without duplicating transcript rows.
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
