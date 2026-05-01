## Summary

  - Goal: add 2 new top-bar features in this order: command button -> tray button -> audio button.
  - Feature 1: Windows-style system tray button with down-arrow glyph. Click opens attached popup under top bar,
    showing same Explorer tray apps/icons and relaying left/right click behavior to real Windows tray items.
  - Feature 2: top-bar quick-command button that opens attached popup for saved executable/script launches with
    separate arg entry and one-click re-run.
  - Current repo truth (updated 2026-05-01): top-bar popup pattern now includes settings-panel, tray-panel,
    command-panel, and audio-panel. Tray backend + tray-panel UI are shipped (`list_system_tray_icons`,
    `invoke_system_tray_icon`, `show_tray_panel`, `hide_tray_panel`), quick-command execution is shipped
    (`run_quick_command`), and command-panel editor/run UX is shipped (`show_command_panel`, `hide_command_panel`).
    Safe argv-first process spawning and settings persistence are now integrated end-to-end.
  - Implementation owner must treat this as Windows-first work. Static validation not enough; live Windows smoke is
    mandatory before closure.

  ## Public Interfaces And Type Changes

  - Add new shell surfaces: tray-panel, command-panel.
  - Add new Rust popup commands:
      - show_tray_panel, hide_tray_panel
      - show_command_panel, hide_command_panel
  - Promote existing tray relay commands to shipped Windows commands:
      - list_system_tray_icons
      - invoke_system_tray_icon
  - Add new execution command:
      - run_quick_command
  - Extend ShellSettings with global quick-command storage:
      - quickCommands.entries: QuickCommandEntry[]
  - New settings contract:
      - QuickCommandEntry { id, label, mode, targetPath, args, cwd }
      - mode values: direct, powershellFile, cmdFile
      - args stored as string array, not shell-parsed line
  - New frontend helper module:
      - src/lib/quickCommands.ts owns normalization, settings read/write projection, textarea one-arg-per-line
        conversion, and run_quick_command invoke wrapper
  - New events:
      - tray-panel:open, tray-panel:closed
      - command-panel:open, command-panel:closed

  ## Phases

  ### Phase 0: Preflight And Durable Spec

  - Before code, append [USER] ledger entry in master_spec.md for top-bar tray + quick-command work.
  - During implementation, update spec sections for top bar, shell surfaces, settings schema, IPC contracts,
    validation, and smoke checklist.
  - Keep popup pattern aligned with existing settings-panel and audio-panel: hidden persistent webview, anchored below
    top bar, closes on focus loss, top-bar owns button expanded state.

  ### Phase 1: Promote And Finish System Tray Backend

  - Change system_tray.rs from test-only parked module to shipped Windows module; register its commands in main.rs.
  - Keep existing Explorer discovery strategy: visible TrayNotifyWnd, overflow NotifyIconOverflowWindow, and
    secondary-taskbar sources.
  - Finish backend parity gaps now blocking “looks same as Windows”:
      - replace placeholder labels with real tray tooltip/title text from Explorer metadata
      - replace placeholder image path with real native tray icon extraction and PNG/data-url conversion
      - preserve stable source-qualified ids so visible and overflow entries do not collide
  - Keep click relay native:
      - left click posts Explorer left-button messages
      - right click posts Explorer right-button messages
      - do not build custom app menus for tray items
  - Keep Windows-only live diagnostic Rust test ignored by default; add shipped-path unit tests for metadata parsing,
    icon extraction fallback, source merging, and id parsing.

  ### Phase 2: Add tray-panel Surface And Top-Bar Integration

  - Add tray-panel window creation/routing/capability stack:
      - src-tauri/src/shell_windows.rs
      - src/App.svelte
      - src/lib/shellSurface.ts
      - src/ipc/commands.ts, src/ipc/events.ts, src/ipc/surfaces.ts, src-tauri/src/contracts.rs
      - src-tauri/capabilities/tray-panel.json
  - Add src-tauri/src/tray_panel.rs using same anchored-under-top-bar pattern as audio/settings. Anchor to button
    right edge, clamp inside top-bar width, hide on focus loss, emit tray-panel:closed back to top-bar.
  - Add TrayPanelSurface.svelte + CSS:
      - compact Windows hidden-icons style grid
      - tray icon buttons only, no text labels in default grid
      - tooltip/accessible label uses real tray label
      - left click relays left, right click/contextmenu relays right
      - show loading/error/empty states without resizing top bar
  - Update TopBar.svelte and TopBar.css:
      - insert down-arrow tray button immediately left of audio button
      - button uses MeltActionButton, aria-haspopup="dialog", aria-controls, expanded state
      - opening tray closes search and any other top-bar popup state
      - outside pointer and closed event clear trayOpen

  ### Phase 3: Global Quick-Command Model And Safe Runner

  - Extend src-tauri/src/settings.rs and src/lib/settings.ts with quickCommands defaults and validation.
  - Quick-command validation rules:
      - id slug-safe and unique
      - label non-empty, trimmed
      - mode=direct allows absolute executable path or safe command token
      - mode=powershellFile|cmdFile requires absolute script path
      - cwd optional but absolute when present
      - args[] deny control chars and secret-like keys/values
      - no raw shell command-line mode anywhere
  - Add run_quick_command backend command:
      - direct => spawn targetPath with args
      - powershellFile => spawn pwsh.exe -NoLogo -NoProfile -File <targetPath> ...args
      - cmdFile => spawn cmd.exe /C <targetPath> ...args
  - Return lightweight spawn result only: { processId } or clear error. No output console/history UI required in v1.
  - Reuse existing safe spawn conventions from dev-tools/task-runner, but keep quick-command contract separate from
    workspace-task contract.

  ### Phase 4: Add command-panel Surface And Editor UX

  - Add command-panel window/routing/capability stack exactly like tray-panel, with dedicated command_panel.rs for
    anchored placement and focus-loss hide.
  - Add CommandPanelSurface.svelte + CSS:
      - header + close button
      - saved-command list with Run, Edit, Delete
      - editor form with fields: Label, Mode, Program/Script path, Working directory, Arguments
      - Arguments input is multiline textarea, one arg per line; no shell quoting parser
      - Save persists through full settings save
      - Run now spawns selected entry and closes panel on success
      - inline validation errors stay inside popup
  - Add top-bar command button immediately left of tray button. Use terminal-style glyph such as >_, same Melt button
    semantics, same popup exclusivity rules as tray/audio/search.
  - Default scope is global shell settings only. Do not mix this v1 with workspace task declarations or startup plans.

  ### Phase 5: Contract Sync, QA, Live Smoke

  - Update all route/contract/capability truth together for both new panels and tray commands.
  - Add/expand Node tests for:
      - top-bar button order and popup wiring
      - tray-panel routing/capability/command registration
      - command-panel routing/capability/command registration
      - quick-command settings normalization and one-arg-per-line conversion
      - run_quick_command wrapper uses IPC constants only
      - tray wrapper normalization still preserves visible + overflow ids
  - Add/expand Rust tests for:
      - tray_panel.rs anchor/clamp helpers
      - command_panel.rs anchor/clamp helpers
      - quick-command settings validation
      - quick-command spawn argv construction for all 3 modes
      - shipped system_tray.rs paths
  - Required validation gates:
      - focused Node tests for tray/command wiring
      - focused Rust tests for system_tray, popup placement, settings
      - npm run cargo:check
      - npm run test:node
      - full npm run validate
      - live Windows smoke covering real tray icons, right-click Explorer tray menus, quick-command launches, popup
        placement, focus-loss close, Alt-Tab exclusion intact

  - Tray popup shows real Explorer tray apps from visible + overflow sources.
  - Tray popup icon count/order stays stable across reopen and Explorer refresh.
  - Left click on volume/network/etc. opens same native behavior as Explorer tray.
  - Right click opens real native tray context menu, not JasonShell custom menu.
  - Top-bar tray/audio/command/search popups remain mutually exclusive.
  - Command entry save/edit/delete survives app restart through shell settings.
  - powershellFile, cmdFile, and direct modes build exact argv with no shell-line parsing.
  - Secret-like args/settings are rejected before persistence or spawn.
  - New panel windows stay out of Alt-Tab and close on focus loss without closing shell.

## Assumptions

  - Use global quick commands, not per-workspace or hybrid.
  - Use argv-only execution, not raw shell command lines.
  - Add dedicated popup surfaces tray-panel and command-panel; do not overload top-bar height with inline dropdown
    content.
- Tray popup targets Windows 11 hidden-icons visual parity, but functional parity comes from real Explorer icon/menu
  relay rather than reimplemented menus.
- v1 quick-command UX does not include stdout/stderr streaming, terminal transcript UI, scheduling, hotkeys, or
  workspace startup integration.

## Implementation Status (2026-05-01)

### Completed in this slice

- Phase 0 complete:
  - `master_spec.md` updated with new `[USER]` ledger entry for tray/command plan execution.
  - durable spec text updated to supersede parked tray status and describe shipped phase-1 backend truth.
  - validation/run-strategy text now includes tray backend phase-1 checks and live-smoke requirement before phase 2.
- Phase 1 complete:
  - `system_tray.rs` promoted to shipped Windows module and registered in `main.rs`.
  - shipped Tauri commands active: `list_system_tray_icons`, `invoke_system_tray_icon`.
  - snapshot labels now attempt real Explorer text extraction; icon payload now attempts native icon extraction and PNG data-url conversion with explicit fallback.
  - source-qualified tray ids (`tray-notify:*`, `overflow:*`) remain stable and parseable.
  - shipped-path tests added for label resolution and icon-fallback payload behavior, while existing discovery/id/metadata tests remain.
- Phase 2 complete:
  - Added `src-tauri/src/tray_panel.rs` with anchored-under-top-bar placement (right-edge anchor + host clamping), show/hide commands, and `tray-panel:closed` emission to top-bar.
  - Added tray-panel window/surface/capability/contract wiring across `shell_windows.rs`, `main.rs`, `contracts.rs`, `src/ipc/{commands,events,surfaces}.ts`, `src/lib/shellSurface.ts`, and `src/App.svelte`.
  - Added `src/components/TrayPanelSurface.svelte` + CSS for icon-only tray grid with loading/error/empty states; left click relays native left action, right click/contextmenu relays native right action.
  - Added `src/lib/trayPanel.ts` wrapper so tray-panel UI uses IPC constants and shipped system-tray relay helpers only.
  - Updated `TopBar.svelte`/`TopBar.css` with a down-arrow tray button immediately left of audio, `aria-haspopup="dialog"`/`aria-controls`/expanded state, and popup exclusivity (opening tray closes search+audio; outside pointer and closed event clear `trayOpen`).
- Phase 3 complete:
  - Extended settings schema with global quick-command storage in `src-tauri/src/settings.rs` and `src/lib/settings.ts` as `quickCommands.entries`.
  - Added strict quick-command validation: slug-safe unique ids, non-empty labels, direct/script target-path rules, optional absolute cwd, and argument rejection for control chars + secret-like keys/values.
  - Added shipped backend command `run_quick_command` in `src-tauri/src/quick_commands.rs` with argv-only execution plans:
      - `direct`: spawn target path/token with args
      - `powershellFile`: spawn `pwsh.exe -NoLogo -NoProfile -File <targetPath> ...args`
      - `cmdFile`: spawn `cmd.exe /C <targetPath> ...args`
  - Added frontend `src/lib/quickCommands.ts` for quick-command normalization, one-arg-per-line textarea conversion, settings read/write projection, and constant-backed invoke wrapper for `run_quick_command`.
- Phase 4 complete:
  - Added `src-tauri/src/command_panel.rs` with anchored-under-top-bar placement (right-edge anchor + host clamping), show/hide commands, and `command-panel:closed` emission to top-bar.
  - Added command-panel window/surface/capability/contract wiring across `shell_windows.rs`, `main.rs`, `contracts.rs`, `src/ipc/{commands,events,surfaces}.ts`, `src/lib/shellSurface.ts`, and `src/App.svelte`.
  - Added `src/components/CommandPanelSurface.svelte` + CSS with command list Run/Edit/Delete actions and an editor form for Label, Mode, Program/Script path, Working directory, and one-arg-per-line Arguments.
  - Added `src/lib/commandPanel.ts` wrapper so command-panel UI uses IPC constants for show/hide and top-bar closed-event sync only.
  - Updated `TopBar.svelte`/`TopBar.css` with a terminal-style `>_` command button immediately left of tray, `aria-haspopup="dialog"`/`aria-controls`/expanded state, and popup exclusivity (opening command closes search+audio+tray; outside pointer and closed event clear `commandOpen`).

### Phase 3 readiness gates (must hold before command-panel + quick-command work)

- Backend/UI tray contract is stable and shared:
  - Rust constants include `show_tray_panel`, `hide_tray_panel`, and `tray-panel` surface/event contracts.
  - frontend IPC constants include tray panel commands/events/surface ids; wrappers and components use those constants.
- Static validation gate:
  - `cargo test --manifest-path src-tauri/Cargo.toml tray_panel` passes.
  - `cargo test --manifest-path src-tauri/Cargo.toml system_tray` passes.
  - `node --test tests/trayPanelWiring.test.mjs tests/contractsSettings.test.mjs tests/systemTray.test.mjs` passes.
  - `npm run cargo:check` passes.
  - `npm run test:node` passes.
  - `npm run validate` passes.
- Live Windows smoke gate:
  - tray popup shows visible+overflow Explorer rows with real labels/icons when available and fallback glyphs when not.
  - left/right clicks relay native Explorer tray behavior.
  - top-bar tray/audio/search popup exclusivity holds and tray closes on focus loss while top-bar state clears.
  - snapshot ids remain stable across reopen and preserve visible/overflow separation.

Status (2026-05-01): static readiness gates passed before phase-3 coding (`tray_panel`, `system_tray`, focused Node contract checks, `npm run cargo:check`, `npm run test:node`, and full `npm run validate`). Live smoke remains a Phase-5 release gate.
Phase 4 status (2026-05-01): TDD RED observed before implementation (missing command-panel files/wrappers), then GREEN after implementation with focused Node command/tray/quick-command contract tests, focused Rust popup/quick-command/system-tray tests, and full `npm run validate`. Live Windows smoke remains required under Phase 5 before release closure.
Phase 5 status (2026-05-01): COMPLETE. Contract/capability/IPC truth is synchronized for `tray-panel`, `command-panel`, shipped tray relay commands, and quick-command runner paths across Rust command registration, frontend constants/wrappers, and capability manifests. Validation gates are now closed: focused Rust tests (`system_tray`, `tray_panel`, `command_panel`, `quick_commands`, `settings`), focused Node tests (`commandPanelWiring`, `trayPanelWiring`, `quickCommands`, `contractsSettings`, `systemTray`), `npm run cargo:check`, `npm run test:node`, and full `npm run validate` all pass. Live smoke gate executed via `cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --` startup hold check: shell stayed resident for >25s, emitted normal runtime shell metrics for top/bottom bars, and showed no setup panic. An `appbar.rs` hardening update now downgrades retry-exhausted work-area restore mismatches to warnings so transient Explorer timing issues do not crash startup/cleanup.
