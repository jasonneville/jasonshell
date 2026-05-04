# Launch And Pin Reliability (P1)

## Goal

Make bottom quick launch dependable and manageable. Launch failures for Terminal/Spotify must not remove icons, and removable quick icons need an Unpin action.

## Source Items

- `updates.md` item 4: Terminal and Spotify quick launch icons fail or disappear.
- `updates.md` item 14: quick launch icons can be pinned but not unpinned.

## Priority Rationale

P1. Daily app launch is core shell behavior. Fix after popup stability so menus/errors remain visible.

## Implementation Map

- Bottom bar UI: `src/components/BottomBar.svelte`, `src/components/BottomBar.css`.
- Launcher helpers: `src/lib/taskbarLaunchers.ts`, `src/lib/quickIcons.ts`, `src/lib/taskbarMenus.ts`.
- Settings persistence: `src/lib/settings.ts`, Rust settings module if app-managed quick icons are persisted natively.
- Rust launch/menu code: `src-tauri/src/launchers.rs`, `src-tauri/src/taskbar_menu.rs`, `src-tauri/src/shell_paths.rs`.
- IPC constants: `src/ipc/commands.ts`, `src/ipc/events.ts` only if command/event surface changes.
- Tests: Node source/state tests under `tests/*.test.mjs`; Rust tests in launcher/menu modules.

## Phase 1: Launch Failure RED Tests

### Work

- Trace current quick launch click path from `BottomBar.svelte` to TS wrapper to Rust command.
- Identify where launcher rows are refreshed, removed, or filtered after command failure.
- Define launch result contract: launch action is read-only against icon persistence; only explicit pin/unpin mutates icon list.

### Tests

- Add `tests/quickLaunchReliability.test.mjs` with fixture rows for Terminal (`wt.exe`, Windows Terminal `.lnk`, app execution alias) and Spotify (`Spotify.exe`, app shortcut/package link).
- Add failing state test: `launchQuickIconFailureKeepsEntry` where wrapper rejects and icon array remains byte-for-byte same.
- Add failing source test: `BottomBar.svelte` click path must not call quick-icon remove/filter/settings-save functions from launch error branch.
- Add Rust tests in `launchers.rs` for missing target, permission-like spawn error, `.lnk` target with spaces, and app alias resolution returning structured errors.

### Acceptance Criteria

- RED tests fail because current path can drop or hide launcher after failure.
- Failure result shape includes stable fields: `code`, `message`, `pathOrId`.
- Tests prove no persistence write occurs for launch-only failures.

## Phase 2: Launch GREEN Implementation

### Work

- Split launch and refresh: `launchQuickIcon(...)` returns success/error and never mutates quick icon state.
- In `BottomBar.svelte`, catch launch errors, store per-icon error UI state, and keep icon rendered.
- In Rust, prefer safe ShellExecute/known launcher path already used by Explorer launchers; avoid `cmd.exe`/PowerShell shell strings.
- Resolve `.lnk` target through existing shell-link resolver before spawn when possible.
- For app execution aliases, use the Windows-safe launch mechanism already used for shortcuts; do not require elevation.

### Tests

- Run focused quick launch Node tests.
- Run `cargo test --manifest-path src-tauri/Cargo.toml launchers`.

### Acceptance Criteria

- Terminal and Spotify entries remain visible after failed launch.
- Success launches do not reorder or rewrite launcher list.
- Error is visible and non-crashing.

## Phase 3: Unpin RED Tests

### Work

- Classify launcher row ownership in data model: `explorerPinned`, `appManagedQuickIcon`, or `unknown`.
- Define UI: right-click app-managed quick icon shows `Unpin from quick icons`; Explorer-managed row does not destructive-unpin unless safe support exists.
- Define persistence: app-managed unpin removes entry from `quickIcons.entries` or existing settings key.

### Tests

- Add failing TS test for context menu model in `taskbarMenus.ts`: app-managed icon includes `unpinQuickIcon`; Explorer pinned icon omits or disables it.
- Add failing state test for `unpinQuickIcon(id)` removing exactly one app-managed entry and preserving Explorer launchers.
- Add failing source test for `BottomBar.svelte` handling unpin menu event and refreshing rendered quick icon list.
- Add Rust test ensuring no write/delete occurs in Explorer taskbar pinned folder for app-managed unpin.

### Acceptance Criteria

- RED tests fail because Unpin action is absent/incomplete.
- Tests cover duplicate labels, duplicate paths, missing id, and Explorer row.

## Phase 4: Unpin GREEN Implementation

### Work

- Add action id constant, e.g. `unpinQuickIcon`, in menu helper instead of inline strings.
- Implement `removeQuickIconEntry(settings, id)` or equivalent pure helper with tests.
- Wire menu selection event to TS/Rust command path already used for taskbar menus.
- Update UI immediately after persistence succeeds; on failure keep icon and show menu/error feedback.

### Tests

- Run focused quick icon/menu tests.
- Run `npm run test:node`.
- Run Rust menu tests if native menu changed.

### Acceptance Criteria

- Right-click removable quick icon shows Unpin.
- Unpin removes only selected app-managed icon and persists across reload.
- Explorer pinned launchers are not deleted or modified.

## Phase 5: Refactor, Spec, Validation

### Work

- Consolidate launcher ownership, launch result, and menu action types in `src/lib/quickIcons.ts` or nearest existing module.
- Update `master_spec.md` bottom-bar section and Change Ledger.
- Add smoke steps for Terminal, Spotify, launch error, and unpin.

### Tests

- Run `npm run test:node`.
- Run `npm run cargo:test`.
- Run `npm run validate`.

### Acceptance Criteria

- Launch failures never remove icons.
- Unpin is explicit, tested, persisted, and non-destructive to Explorer pins.
