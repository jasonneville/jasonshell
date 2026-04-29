# JasonShell Wide UI and Shell Behavior Plan

Status: VALIDATED
Date: 2026-04-28

## Objective

Implement the requested cross-surface cleanup and shell behavior fixes:

- Remove gradients from the UI and replace them with uniform colors.
- Switch top-bar pinned folders without Stack Browser hide/show flicker.
- Make Process Manager read more like Windows Task Manager: Applications first, Background processes next, Windows processes last, with each group independently expandable for detail.
- Replace Stack Browser breadcrumb chrome with an editable path box that still exposes clickable path segments.
- Fix Stack Browser sort indicators for every details column.
- Fully hide JasonShell shell bars while another app is fullscreen.
- Fix taskbar hover previews so each task tile captures its target window, not the whole screen.

## Acceptance Criteria

- AC-1: `rg "gradient|linear-gradient|radial-gradient|conic-gradient|background-image" src src-tauri tests` reports no active gradient UI styling except unavoidable test text if explicitly justified.
- AC-2: Clicking one top-bar folder pin while Stack Browser is already visible updates path, position, focus, and data without calling the native hide path and without clearing visible rows before the next folder payload arrives.
- AC-3: Process Manager renders three stable groups in this order: Applications, Background processes, Windows processes. Filtering and sorting work inside groups without destroying group order.
- AC-4: Stack Browser path control is editable, accepts typed absolute paths on Enter, supports Escape/reset behavior, and preserves clickable breadcrumb segment navigation inside the control.
- AC-5: Stack Browser details headers show correct active sort direction for Name, Type, Size, and Modified after each click and expose matching `aria-sort`.
- AC-6: JasonShell top and bottom appbar surfaces hide/release while a foreground fullscreen app owns the primary monitor, then restore shell surfaces/work-area reservation when fullscreen exits.
- AC-7: Taskbar previews capture the referenced HWND bounds and image content, with a fallback/error only when that specific target cannot be captured.
- AC-8: Focused Node/Rust tests pass for changed state machines, and final `npm run validate` passes.

## Work Breakdown and Delegation

### Worker A: Frontend visual and Stack Browser

Skills: `caveman`, `senior-frontend`, `tdd-guide`.

Ownership:

- `src/app.css`
- `src/App.svelte`
- `src/components/*Surface.css`
- `src/components/TopBar.css`
- `src/components/BottomBar.css`
- `src/components/StackPopupSurface.svelte`
- `src/components/StackPopupSurface.css`
- `src/lib/stackPopupState.ts`
- Stack Browser/frontend tests under `tests/`

Tasks:

- Replace gradient CSS variables and gradient backgrounds with uniform token colors.
- Add editable Stack Browser path textbox with click-segment navigation.
- Fix Stack Browser active sort header visuals/ARIA if state or wrapper prevents updates.
- Add/update tests for no-gradient policy, editable path behavior, and sort-header wiring.

### Worker B: Process Manager Task Manager view

Skills: `caveman`, `senior-frontend`, `tdd-guide`.

Ownership:

- `src/components/ProcessManagerSurface.svelte`
- `src/components/ProcessManagerSurface.css`
- `src/lib/processManagerState.ts`
- `src/features/process-manager/processManagerUxState.ts`
- process-manager Node tests under `tests/`

Tasks:

- Add group classification and grouped row model: Applications, Background processes, Windows processes.
- Keep existing taskbar-active process enrichment and guarded kill flow.
- Make sorting/filtering scoped within groups while group order remains stable.
- Add tests for group classification/order, filtering, and sort preservation.

### Worker C: Native fullscreen and preview capture

Skills: `caveman`, `rust-skills`, `tdd-guide`.

Ownership:

- `src-tauri/src/appbar.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/task_preview.rs`
- `src-tauri/src/task_windows/previews.rs`
- `src-tauri/src/task_windows/windows.rs`
- related Rust tests

Tasks:

- Add foreground fullscreen detection for primary monitor.
- Hide/release JasonShell shell bars while a fullscreen non-JasonShell app is active, then restore appbar reservation when fullscreen exits.
- Investigate preview capture path and prefer target-window capture over whole-screen BitBlt fallback when possible.
- Add helper tests for fullscreen geometry/identity decisions and preview bounds/ROP behavior.

### Worker D: QA and spec reconciliation

Skills: `caveman`, `adversarial-reviewer`, `tdd-guide`.

Ownership:

- Review final diff across all touched files.
- Confirm `master_spec.md` behavior updates and ledger entries.
- Run focused validation plus final `npm run validate`.

Tasks:

- Review for regressions in focus-loss, Tauri hidden webview delivery, appbar cleanup, and process kill guardrails.
- Report BLOCK/CONCERNS/CLEAN.
- Dispatch follow-up fixes if in-scope blockers appear.

## Priority

1. P0: Plan, spec ledger, no-gradient baseline, Stack Browser no-flicker path switch.
2. P1: Stack Browser editable path and sort indicators.
3. P1: Process Manager group model.
4. P1: Fullscreen hide/restore.
5. P1: Target-window task preview capture.
6. P0: Focused tests, full validation, adversarial QA, spec update.

## Validation Plan

- `npm run test:node`
- focused process-manager and stack-browser tests
- focused Rust tests for appbar/fullscreen and task-window previews
- `npm run check`
- `npm run cargo:test`
- `npm run cargo:check`
- final `npm run validate`

## Risks

- Live fullscreen behavior and DWM capture still benefit from real Firefox/game/WebView2 smoke beyond unit tests.
- AppBar release/restore remains guarded by focused Rust tests and full validation, but exact third-party fullscreen timing is desktop-environment dependent.
- Stack Browser typed-path navigation now validates before committing path/history/entries; top-bar pin switching and other folder navigation keep retained rows while new payloads load.

## Completion Summary

- Worker A completed the no-gradient UI pass, Stack Browser editable path control, retained-row pin switching path, validated typed-path commit, and details sort indicators for every column.
- Worker B completed the Task Manager-style Process Manager grouping with Applications, Background processes, and Windows processes, plus per-group expand/collapse state.
- Worker C completed foreground-fullscreen shell hide/restore and target-HWND task preview capture improvements.
- Worker D completed adversarial QA; initial concerns were fixed, and the recheck returned clean.

## Final Validation

- `rg -n "linear-gradient|radial-gradient|conic-gradient|repeating-linear-gradient|repeating-radial-gradient|background-image|--js-gradient" src` returned no matches.
- `npm run validate` passed: Svelte check 0 errors/0 warnings, production build passed, `npm run test:node` passed 181/181 tests, `npm run cargo:test` passed 161 Rust tests with 1 ignored live tray diagnostic, and `npm run cargo:check` passed.
- `git diff --check -- src-tauri/src/appbar.rs` still reports whitespace on newly added CRLF lines because that tracked file is `i/crlf w/crlf`; the file was left CRLF to avoid unrelated line-ending churn.
