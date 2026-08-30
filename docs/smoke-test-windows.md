# Windows Live Smoke Test Checklist

Status: current live-smoke checklist for JasonShell tray/command plus Stack Browser Phase 1 safety validation. Stack Browser Phase 1 smoke is pending unless a maintainer records live Windows evidence; static tests do not replace these checks. Plan 13 dry-run harness and contract gates pass; consent-gated live checks remain blocked/unimplemented.

Use this after the static validation gates pass. These checks require a Windows desktop session, WebView2, and `npm run tauri dev`; they are not replaced by Node or Rust unit tests. The safe default runtime-smoke contract is `npm run smoke:runtime` as a dry run: it must not launch JasonShell or mutate desktop/process state without explicit consent, and it writes timestamped artifacts under `test-results/runtime-smoke/<timestamp>/`.

## Runtime Smoke Contract

- `npm run smoke:runtime` is the intended non-destructive entry point. Default mode is dry run and may record planned checks/artifacts only.
- Runtime smoke artifacts live under ignored `test-results/runtime-smoke/<timestamp>/` and should include command, environment summary without secrets, consent flags, skipped checks, notes, and per-check status.
- Status vocabulary: `automated` means the harness directly observed the check; `manual` means a human recorded evidence; `blocked` means not run or skipped because prerequisites/consent/environment were missing.
- Before any live action that can reserve AppBars, hide/restore the Explorer taskbar, alter the Windows work area, install global hooks, or terminate processes, obtain explicit human consent for that action class. Consent for one class does not imply consent for UAC/admin or process termination.
- Do not claim automated assistive-technology support. NVDA, JAWS, DPI/scaling, and multi-monitor evidence is manual-only and must identify tester/tool, display setup, and observed result.
- Official Node test entry is `npm run test:node`; it cleans repo-local `dist-tests`, rebuilds `tsconfig.test.json`, then runs `node --test tests/*.test.mjs`. Direct `node --test ...` is unsafe unless `dist-tests` is known fresh because stale compiled helpers can mask source changes.

## Preflight

- Confirm `npm run check`, `npm run build`, `npm run test:node`, `npm run cargo:test`, and `npm run cargo:check` pass or record the exact failure.
- Run default `npm run smoke:runtime` only as dry run unless the implemented script contract and consent gates are present; record artifact path and status as automated/manual/blocked, not as live pass evidence.
### Manual Live Session

Continue only after explicit maintainer consent for each applicable action class. Dry-run evidence is not live evidence.

- Start JasonShell with `npm run tauri dev` only after that consent.
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
- Keyboard-only: Tab reaches tray icon buttons, Enter/Space relays native left action, and Context Menu key or Shift+F10 on a focused icon relays the native right action through the same guard. These relay checks require explicit consent and safe approved tray targets.
- Keyboard-only: Escape closes `tray-panel` through `hide_tray_panel`, and the visible labelled close button is reachable and closes through the same hide path.
- Left/right tray icon activation keeps `tray-panel` open; stale icon failures show inline error text instead of collapsing the panel.
- `tray-panel` closes on focus loss and top-bar `aria-expanded` state clears.

## Command Panel

- Clicking the command button (`>_`) opens `command-panel` anchored under the top bar.
- Live run checks require explicit consent before starting any process.
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
- Caller guards: from normal UI, verify top-bar pin/show flows still work, `stack-popup` file/Git/archive actions still work, and terminal-panel path open/reveal bridges still work for their intended targets. If testing via a dev console or injected invoke harness, verify a wrong-surface Stack Git command such as `stack_git_push` from `command-panel` fails with `Unauthorized caller for command stack_git_push`.
- Git safety: in a Git repo, open Git workbench, load status/log/tree/branches, stage a selected changed path, and verify deleted-file staging works only for a path shown in fresh status. Verify remote fetch/pull/push failures return bounded actionable errors and do not hang on credential prompts.
- Archive timeout/error: with a safe test archive, verify normal extraction still works. If timeout env override is set to a short clamped value for a controlled hanging extractor, verify JasonShell returns a timeout error and remains responsive.
- Clipboard copy/cut: copy and cut from Stack Browser, paste into Explorer, and verify Explorer sees the expected copy/move DropEffect. Paste back into Stack Browser and verify collision naming plus refresh. If a clipboard publish failure is induced, verify no partial unusable file-list data remains on the clipboard.
- Recovery journal: perform Stack Browser copy paste and cut/move fallback paste, then inspect app-local `stack-browser-recovery/` for per-operation JSON artifacts with planned/completed or failed/interrupted state. Verify no recovery UI appears and no automatic repair/rollback/source deletion occurs after restart.

## Bottom Bar

- Explorer taskbar `.lnk` launchers enumerate and launch.
- Quick Launch keyboard-only: open Quick Launch, Tab/Arrow through launcher rows, verify selected row remains background-only while keyboard `:focus-visible` shows the shared visible focus ring, then press Enter on a safe approved target to launch only with explicit consent.
- Quick Launch keyboard-only admin menu: with a row focused, press Context Menu key and Shift+F10 separately and verify the native menu opens with `Run as administrator` at the focused row. Do not activate admin/UAC unless the maintainer gives explicit consent and the target is safe and approved.
- Quick Launch visual focus: manually verify the selected/background-only state and distinct focus ring at 100% and 200% display scaling or browser zoom; record theme/scaling used. Do not claim assistive-technology evidence from this visual check.
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
- Pending/manual evidence: keyboard-check Process Manager at 100% and 200% zoom in light and dark themes. Tab/Shift+Tab should reach Filter, Refresh, sortable headers, group toggles, Kill buttons, the focusable grid, and Close with visible focus throughout.
- Pending/manual evidence: with NVDA or JAWS for 30 seconds while the process list auto-refreshes, verify timer ticks do not announce every successful refresh. Trigger one manual Refresh and record one bounded status announcement. Record any auto-refresh error announcement separately as intentional error status.

## Notes

- Stack Browser Phase 1 manual Windows smoke status: pending/not run in this docs pass.
- `npm run tauri dev` may fail when ports `1420`/`1421` are already occupied; stop the conflicting dev server and retry.
- If `cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --` reports AppBar/work-area warnings, capture logs and continue manual popup/relay checks before marking smoke complete.
