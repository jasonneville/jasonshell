# Shell Popup Stability (P0)

## Goal

Make existing popup surfaces dependable before adding new features. Tray icon activation must not collapse `tray-panel`; task previews must stay open while pointer is anywhere in the preview and expose a close button.

## Source Items

- `updates.md` item 1: tray icon click must not remove tray view underneath.
- `updates.md` item 5: task preview needs red X close control and full-preview hover retention.

## Priority Rationale

P0. Broken popup retention interrupts active shell interaction and makes later tray/preview work hard to smoke-test.

## Implementation Map

- Frontend tray surface: `src/components/TrayPanelSurface.svelte`.
- Top-bar popup ownership: `src/components/TopBar.svelte`.
- Tray IPC wrapper: `src/lib/systemTray.ts`.
- Tray backend: `src-tauri/src/system_tray.rs`, registration in `src-tauri/src/main.rs` if command/event changes.
- Task preview surface/state: `src/components/TaskPreviewSurface.svelte`, `src/lib/taskbarPreview.ts`, `src-tauri/src/task_preview.rs`.
- Task window close path: search existing `close_task_window`, `request_close`, or native task menu command before adding a new command.
- Tests: add focused Node tests under `tests/*.test.mjs`; add Rust unit tests beside changed Rust module.

## Phase 1: Tray Retention RED Tests

### Work

- Read `TrayPanelSurface.svelte`, `TopBar.svelte` tray event handlers, `src/lib/systemTray.ts`, and `src-tauri/src/system_tray.rs`.
- Write down current close triggers: focus loss, outside pointer, top-bar popup exclusivity, tray command failure.
- Define invariant in test names: `invoke tray icon from tray panel does not emit tray-panel:closed or call hideTrayPanel`.

### Tests

- Add `tests/trayPanelRetention.test.mjs` source test that extracts the tray icon click/invoke handler from `TrayPanelSurface.svelte` and fails if it calls `hideTrayPanel`, emits `tray-panel:closed`, or toggles top-bar popup state before invoke completes.
- Add source test that `TopBar.svelte` only handles `tray-panel:closed` from explicit close/focus-loss events, not from `invokeSystemTrayIcon` success/failure paths.
- Add Rust unit test in `system_tray.rs` for invoke error mapping: invalid/stale tray id returns `Err` and does not request panel close through any event payload.

### Acceptance Criteria

- RED command: `npm run test:node -- trayPanelRetention` or repo-compatible focused equivalent fails for missing guard.
- RED command: `cargo test --manifest-path src-tauri/Cargo.toml system_tray` fails only for new expected behavior.
- Tests cover success, backend error, stale icon id, and double-click/rapid-click cases.

## Phase 2: Tray Retention GREEN Implementation

### Work

- In `TrayPanelSurface.svelte`, add `isInvokingTrayIcon` or equivalent local guard around icon activation.
- Prevent focus-loss/outside-close handling from closing while pointer/focus transition originated inside tray icon activation.
- Await `invokeSystemTrayIcon(...)` from `src/lib/systemTray.ts`; on failure store an inline `invokeError` and keep snapshot visible.
- Do not mutate `TopBar.svelte` popup state except through existing explicit close event.
- If backend needs no changes, do not add new Rust commands.

### Tests

- Run focused tray Node tests.
- Run focused Rust tray tests if backend changed.

### Acceptance Criteria

- Clicking tray icon inside `tray-panel` leaves panel open after success and after failure.
- Clicking outside `tray-panel` still closes it.
- Popup exclusivity still closes tray when opening search/command/audio/settings.

## Phase 3: Task Preview RED Tests

### Work

- Read `TaskPreviewSurface.svelte`, `src/lib/taskbarPreview.ts`, `src-tauri/src/task_preview.rs`, and bottom-bar preview hover code in `src/components/BottomBar.svelte`.
- Identify exact hide timer functions and pointer boundary checks.
- Define close button contract: `button`, `aria-label="Close previewed window"`, top-right absolute placement, red X visual, stops propagation, calls existing safe close-window IPC.

### Tests

- Add `tests/taskPreviewRetention.test.mjs` source test asserting preview root owns `pointerenter`/`pointerleave` or equivalent for the entire preview, not only lower content/body.
- Add source test asserting no top-half selector/region schedules hide while pointer is inside preview root bounds.
- Add source/component test asserting close button markup/action exists with accessible label and red-X class.
- Add Rust/TS wrapper test for close action validating target window id/HWND and rejecting missing/internal JasonShell windows.

### Acceptance Criteria

- RED tests fail against current missing close button/top-half retention issue.
- Tests cover top half, bottom half, close button click, leave preview bounds, stale preview id.

## Phase 4: Task Preview GREEN Implementation

### Work

- Move hover-retention handlers to the outer preview container in `TaskPreviewSurface.svelte`.
- Ensure preview root covers rendered visual bounds; if native window geometry is wrong, fix in `src-tauri/src/task_preview.rs` rather than adding CSS-only hacks.
- Add close button in top-right; wire to existing task-window close wrapper or add minimal wrapper in `src/lib/taskbarPreview.ts` only if no wrapper exists.
- Stop propagation/default on close button so it does not activate preview, reset hover, or schedule hide.
- Keep hide timer behavior for pointer leaving both tile and preview.

### Tests

- Run `npm run test:node -- taskPreviewRetention` or focused equivalent.
- Run `npm run cargo:test -- task_preview` if native preview code changed.
- Run `npm run test:node` after focused pass.

### Acceptance Criteria

- Pointer can move anywhere inside preview without closing it.
- Red X closes only the previewed external task window.
- Preview still closes on true outside leave/focus-loss per current behavior.

## Phase 5: Refactor, Spec, Validation

### Work

- Remove duplicated close/hover flags introduced during GREEN.
- Keep event names centralized in `src/ipc/events.ts` if new events were added.
- Update `master_spec.md` tray-panel and task-preview sections plus Change Ledger.
- Add manual smoke notes to `docs/smoke-test-windows.md` only if no existing section covers tray/preview.

### Tests

- Run focused tray and preview tests.
- Run `npm run test:node`.
- Run `npm run cargo:test` if Rust changed.
- Run `npm run validate` before marking P0 complete.

### Acceptance Criteria

- All focused and full validation pass.
- Spec states tray icon invocation does not close tray-panel.
- Spec states full preview bounds retain hover and top-right close exists.
