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

Screen snipping is a current scoped surface with a pending native journey. It uses the existing primary-monitor AppBar shell scope, freezes all enumerated monitors for the capture, and does not change AppBar ownership. The native capture flow includes exact top-bar/overlay/preview instance authorization, binary image per monitor, first-monitor selection binding, truthful clipboard outcomes, a non-activating Copy/Save/Dismiss preview, and a path-free native picker save path. It has no annotations, spanning mode, autosave, history, upload, or renderer-controlled paths/handles. Legacy `Alt+S` bindings remain unchanged and disable the new fifth snipping binding until the conflict is repaired.

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
| Process manager and System Info | `src/lib/processManager*.ts`, `src/lib/systemInfo*.ts`, `ProcessManagerSurface.svelte`, `SystemInfoView.svelte` | `src-tauri/src/process_manager.rs`, `process_manager/{system_info,native_runtime,windows_telemetry,probe_executor,volume_probe}.rs` | process manager tests; focused component/native/ACL acceptance suites | `README.md`, `master_spec.md`, `docs/smoke-test-windows.md` |
| Workspaces and automation | `src/lib/workspaces.ts`, `automation.ts`, `providerContracts.ts` | `src-tauri/src/workspaces.rs`, `automation.rs`, `providers.rs`, `dev_tools/` | `tests/workspaces.test.mjs`, `tests/automationProviders.test.mjs` | `README.md`, `master_spec.md` |
| Audio, calendar, settings, tray | matching `src/lib` and surface files | matching `src-tauri/src` modules | matching `tests/*` files | `master_spec.md`, focused docs when present |
| Screen snipping | `src/lib/snipping.ts`, `src/lib/shellSurface.ts`, `src/lib/surfaceLoader.ts`, `src/lib/topBarControls.ts`, `Snip*Surface.svelte` | `src-tauri/src/snipping/`, `settings.rs`, `windows_key_hook.rs`, scoped capabilities | snipping component/Node/Rust suites and `tests/snipping-product-handoff.md` | `README.md`, `master_spec.md` |
| Bootstrap, smoke, validation policy | none unless UI affected | `scripts/`, Tauri config, Rust test modules | `tests/bootstrapWindowsContract.test.mjs`, docs-policy tests | `README.md`, `CHANGELOG_POLICY.md`, this brief |

### Process Manager System Info current contract

The existing Process Manager surface has only **Processes** and **System Info** tabs; it adds no launcher, icon, or window and opens on Processes. Tab transitions preserve the process filter, sort, expanded groups, and viewport. System Info renders Overview, Storage, Sensors, Network, and Hardware. Whole-machine readings do not follow the process filter, and network traffic remains per-adapter rather than an invented total.

System Info IPC is a versioned cache read. Native collection runs only while the registered Process Manager is visible and focused on the current generation/tab revision: live every 2 seconds, storage every 15 seconds, inventory every 60 seconds. Stale generations, revisions, hidden/unfocused contexts, and stale snapshot revisions are rejected. Storage uses selected-root helper probes with a 1,500 ms deadline and bounded root/concurrency/output limits; probe slots remain occupied until a child is killed and waited/reaped, including when Windows cannot reap it.

Close events carry the retired context (`{ context: retiredContext }`); renderer monotonic guards reject a stale generation-1 close after a generation-2 open. The storage UI label is **Caller-visible capacity (quota-aware)**, not physical or volume capacity.

Known truthful limits: CPU is unavailable for multiple processor groups or more than 64 logical processors; GPU is busiest physical engine, not adapter-total; dedicated VRAM excludes UMA shared memory. Sensors are unavailable as `provider_not_verified` without a verified already-running provider; JasonShell does not install, elevate, or launch one. Hardware collection is allowlisted and excludes serial/MAC/UUID/key data. There are no battery/history/chart/action features.

Validation evidence: operational child cleanup 7/7, cache/schema 20/20, ACL 6/6, 484 component tests, 1,180 Rust tests, full `check`, `build`, and `cargo:check` pass. Node has 1,270 passes and 10 proven baseline failures at HEAD `43af99a6009bd2dee5b7f27d7b425832e307841b`; `npm run validate` exits 1 and is not GREEN. A visible-browser production-mounted mocked-IPC journey plus 13 extended checks passed with verified portable wmux 2.60.0; reproduce with `pwsh -NoProfile -File tests/browser/system-info/run-visible.ps1 -WmuxProcessId <current wmux PID>` (observed PID 35164, exit 0). Six screenshots, accessibility/results artifacts, and a 62-entry IPC trace were written under the approved `system-info-visible-e2e` temp directory. Native hardware/HWND runtime and live shell smoke remain unverified and unauthorized.
