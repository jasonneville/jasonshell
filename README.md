# JasonShell

JasonShell is a Windows shell prototype built with Tauri 2, Svelte 5, TypeScript, Rust, and Win32.

It aims to feel like a native shell layer, not a normal app window.

## At a glance

Current shell surfaces:

- top and bottom app bars
- Quick Launch
- task tiles and previews
- Stack Browser
- persistent terminal panel
- Quick Commands
- search panel
- process manager
- audio, calendar, settings, and tray surfaces

Core shell rule: the top and bottom bars reserve primary-monitor AppBar work area.

Current truth boundary: workspace restoration is reserved and not implemented; startup commands are not executed automatically; automation forwarding is planned and not wired; multi-monitor support is planning-only, with live shell ownership remaining a single-monitor runtime.

## What this repo is for

This repo is a prototype of a Windows-native shell foundation.

It explores:

- pinned taskbar launchers as shell launch source
- taskbar tiles with previews and window actions
- folder and Git browsing inside Stack Browser
- persistent terminal sessions with ConPTY and xterm
- saved commands with live output history
- centered search across apps, windows, settings, folders, commands, and Everything results where available
- process management with guarded end-process actions
- shell-adjacent surfaces like audio, calendar, settings, and tray

## Requirements

- Windows
- Node.js 20+
- Rust stable MSVC
- Visual Studio Build Tools
- Microsoft WebView2 runtime
- Git optional for workbench and repo workflows

## First run bootstrap

Use this exact PowerShell bootstrap on first run:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\bootstrap-windows.ps1
```

Use the bootstrap rather than `npm install`; it preserves the lockfile-safe `npm ci` dependency path.

On managed machines, review the script first. It can request UAC elevation to install missing Node.js, Rust, Visual Studio Build Tools, or WebView2.

What it does:

- checks prereqs
- installs missing prereqs
- installs repo dependencies
- launches JasonShell

Expected first-start result:

- app opens as a native Windows shell window
- top bar and bottom bar reserve screen edge space on primary monitor
- shell panels become available from the bars

## Later runs

After prereqs and deps are ready, run:

```powershell
npm run tauri dev
```

Manual alternative only when prereqs are already done:

```powershell
npm ci
npm run tauri dev
```

## Practical workflows

### Open a terminal

1. Start JasonShell.
2. Use the top bar terminal button.
3. Reuse tabs or splits in the persistent terminal panel.

### Launch a taskbar app

1. Open the bottom bar.
2. Pick a pinned Explorer taskbar launcher.
3. Launch the app from Quick Launch or task tiles.

### Browse a Git repo

1. Open Stack Browser from a pinned folder.
2. Enter a repo folder.
3. Use Changes, History, Stashes, or Branches.
4. Fetch, pull, push, checkout, or create branches from the Git workbench.

### Run a saved command

1. Open Quick Commands.
2. Pick a saved command.
3. Run it and review the live transcript/history.

### Search for something

1. Use the centered search surface.
2. Search apps, windows, settings, folders, commands, or Everything results when available.
3. Open the result directly.

## Feature overview

### Top bar

- hosts shell controls and search entry
- opens settings, search, Stack Browser, terminal, Quick Commands, tray, audio, and calendar
- reserves AppBar work area on the primary monitor

### Bottom bar

- shows Explorer taskbar pins as the launcher source
- shows open-window task tiles and previews
- includes process manager access
- reserves AppBar work area on the primary monitor

### Quick Launch

- uses current Explorer taskbar pins
- launches pinned apps from the native panel
- stays tied to the bottom bar launcher source of truth

### Stack Browser

- browse folders
- file operations
- Changes, History, Stashes, and Branches views
- fetch / pull / push / checkout / create branch actions

### Persistent terminal panel

- ConPTY-backed terminal sessions
- xterm rendering
- tabs and splits
- profiles: Windows Terminal, Git Bash, PowerShell
- command/output history and shell integration support

### Quick Commands

- saved commands
- live transcript
- run history
- direct command replay

### Search

- centered search UI
- apps, windows, settings, folders, commands
- Everything results when available

### Process manager

- sortable process metrics
- guarded end-process action
- live process inspection

### Workspaces and automation

- workspace profiles persist metadata, pins, aliases, declared tasks, startup plans, and reserved restoration status
- workspace activation returns a plan; it does not restore windows or run startup commands
- startup commands are not executed automatically
- automation parsing and validation exist for safe first-party intents
- automation forwarding is planned but not wired; forwarded payloads are not executed

### Other surfaces

- audio
- calendar
- settings
- tray

## Project layout

Verified paths in this repo:

- `src/` - frontend app code
- `src-tauri/` - Rust backend and Tauri integration
- `scripts/` - bootstrap and smoke scripts
- `tests/` - automated tests
- `docs/` - repo documentation

## Config and environment notes

- `JASONSHELL_TERMINAL_SHELL_INTEGRATION=0` disables terminal shell integration
- `JASONSHELL_TERMINAL_SHELL_INTEGRATION=false` also disables it
- `JASONSHELL_TASKBAR_NATIVE_HOOKS=0` disables native taskbar hooks

## Validation

Exact package scripts:

```powershell
npm run dev
npm run build
npm run preview
npm run check
npm run test:node
npm run test:search
npm run cargo:check
npm run cargo:test
npm run smoke:fullscreen
npm run validate
npm run tauri
```

Validation note:

- Windows native behavior still needs manual smoke
- `npm run smoke:fullscreen` is part of the expected Windows smoke path

## Caveats

- prototype, not final shell product
- primary-monitor AppBar reservation is the current runtime target
- multi-monitor support is planning-only; live shell ownership remains a single-monitor runtime until implemented and live-tested
- workspace restoration is reserved and not implemented
- workspace startup commands are not executed automatically
- automation forwarding is planned and not wired
- tray behavior needs caution
- native Windows behaviors require manual smoke

## Documentation links

- `master_spec.md` - canonical behavior and architecture
- `package.json` - exact scripts and toolchain entrypoints
- `scripts/bootstrap-windows.ps1` - first-run bootstrap path
- `docs/` - repo docs and smoke references
