# JasonShell current-state brief

Status: current onboarding brief. Read this after `AGENTS.md` for every JasonShell task.

## Product and runtime boundary

JasonShell is a Windows shell prototype built with Tauri 2, Svelte 5, TypeScript, Rust, and Win32. It behaves like a native shell layer, not a normal app window.

Live runtime target is Windows on the primary monitor. The top and bottom bars reserve AppBar work area on that monitor. Multi-monitor behavior is planned material unless current source and tests prove otherwise.

Frontend code lives under `src/`. Native/Tauri/Win32 code lives under `src-tauri/src/`. Tests live under `tests/` for Node contract tests and under Rust modules for Rust tests. Bootstrap and smoke scripts live under `scripts/`.

## Active product truth

Use `README.md` for the user-facing product truth and exact setup/validation command list. Use source and tests as final authority for runtime behavior. Prose docs guide search and intent, but source/tests win when they conflict.

Current shipped surfaces include top and bottom bars, Quick Launch, task tiles and previews, Stack Browser, persistent terminal panel, Quick Commands, search panel, process manager, audio, calendar, settings, and tray surfaces.

Reserved or planning-only behavior:

- workspace restoration is reserved and not implemented
- workspace startup commands are not executed automatically
- automation forwarding is planned and not wired
- multi-monitor support is planning-only unless source/tests prove a narrower change

## Safety constraints

- Preserve unrelated dirty worktree changes.
- Keep Windows shell mutations behind explicit validation. Live Tauri smoke can reserve AppBars, alter work area, hide Explorer taskbars, or install hooks, so get human consent before live shell smoke.
- Treat process termination, shell hooks, AppBar work-area changes, persistence, startup commands, and automation forwarding as safety-sensitive.
- Do not turn planned persistence, restoration, startup execution, automation forwarding, or multi-monitor behavior into claimed current behavior through docs alone.
- Do not use changelog or legacy plans as current truth without confirming against source/tests.

## Validation basics

Package scripts are the command source of truth: `npm run test:node`, `npm run check`, `npm run build`, `npm run cargo:test`, `npm run cargo:check`, `npm run smoke:runtime`, `npm run smoke:fullscreen`, and `npm run validate`.

For docs-only changes, run focused docs tests and link/size checks. Do not claim runtime validation from docs tests.

## Feature lookup map

Use this map to find code, tests, and docs. Then inspect nearby files, not the whole repo.

| Feature or concern | Frontend | Backend/native | Tests | Docs |
|---|---|---|---|---|
| Top bar | `src/components/TopBar.svelte`, `src/lib/topBarPins.ts`, `src/features/top-bar/` | `src-tauri/src/shell_windows.rs`, `appbar.rs` | `tests/topBar*.test.mjs`, bootstrap/AppBar tests | `master_spec.md`, `README.md` |
| Bottom bar, taskbar, previews | `src/lib/taskbar*.ts`, preview/gallery surfaces | `src-tauri/src/task_windows/`, `task_preview.rs`, `task_gallery.rs`, `taskbar_menu.rs`, `launchers.rs` | `tests/taskbar*.test.mjs`, `tests/bottomBarPreviewRequestId.test.mjs` | `master_spec.md`, performance plans when relevant |
| Stack Browser and editor | `src/lib/stack*.ts`, `src/components/Stack*.svelte` | `src-tauri/src/stack_popup/`, `stack_popup.rs` | `tests/stack*.test.mjs` | `docs/stack-*`, `master_spec.md` |
| Terminal panel | `src/lib/terminalPanel.ts`, `persistentTerminal.ts`, terminal components | `src-tauri/src/terminal_panel.rs` | `tests/terminal*.test.mjs`, Stack terminal tests | `docs/terminal-panel-prewarm-idle-policy.md`, `master_spec.md` |
| Quick Commands and command panel | `src/lib/quickCommands.ts`, `commandPanel.ts` | `src-tauri/src/quick_commands.rs`, `command_panel.rs` | command and quick-command tests | named remediation plans if relevant |
| Search | `src/lib/search*.ts`, `SearchPanelSurface.svelte` | `src-tauri/src/search/`, `search_sources/`, `search_panel.rs` | `tests/*search*.test.mjs`, search source tests | `docs/search-functionality-*`, `master_spec.md` |
| Process manager | `src/lib/processManager*.ts`, `ProcessManagerSurface.svelte` | `src-tauri/src/process_manager.rs` | process manager tests | remediation plans 05 and 10 if still relevant |
| Workspaces and automation | `src/lib/workspaces.ts`, `automation.ts`, `providerContracts.ts` | `src-tauri/src/workspaces.rs`, `automation.rs`, `providers.rs`, `dev_tools/` | `tests/workspaces.test.mjs`, `tests/automationProviders.test.mjs` | `README.md`, `master_spec.md` |
| Audio, calendar, settings, tray | matching `src/lib` and surface files | matching `src-tauri/src` modules | matching `tests/*` files | `master_spec.md`, focused docs when present |
| Bootstrap, smoke, validation policy | none unless UI affected | `scripts/`, Tauri config, Rust test modules | `tests/bootstrapWindowsContract.test.mjs`, docs-policy tests | `README.md`, `CHANGELOG_POLICY.md`, this brief |
