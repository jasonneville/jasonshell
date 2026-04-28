# Windows Live Smoke Test Checklist

Status: current live-smoke checklist for JasonShell Phase 0 validation.

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
- Ctrl+K focuses search and opens `search-panel`.
- Typing a query updates search results without clearing visible state prematurely.
- Keyboard selection and Enter activation work for a result.
- Pinning a folder from search updates the top-bar pin rail immediately.

## Stack Browser

- Clicking a pinned folder opens `stack-popup` anchored below the pin.
- Folder navigation, back/forward/up, sorting, selection, and inline rename/new-folder behavior remain usable.
- Right-click row and background menus fit inside the visible screen.
- Native Explorer folder drops onto the top bar are accepted when valid.
- Mouse XButton back/forward behavior works when the stack popup has focus.

## Bottom Bar

- Explorer taskbar `.lnk` launchers enumerate and launch.
- Open windows group by application identity and activate/minimize with taskbar-like behavior.
- Reordering task groups with pointer drag does not trigger accidental activation.
- Hover previews show for task groups and hide reliably.
- Native group menus refresh window state after actions.

## Process Manager

- The rightmost bottom-bar process button opens `process-manager` above the bar.
- Process rows refresh while open and stop polling after focus-loss close.
- Sorting works for visible columns, including Start Time when values are available.
- Kill action refuses protected/current-shell targets and refreshes after attempts.

## System Tray Status

- System tray support is parked/experimental as of Phase 0 docs: repo evidence shows a test-only Windows backend module, `src/lib/systemTray.ts`, and `tests/systemTray.test.mjs`, but shipped commands are intentionally not registered.
- Do not mark tray behavior shipped until commands are registered, a visible surface consumes them, capabilities are scoped, and this checklist has tray-specific live checks.
