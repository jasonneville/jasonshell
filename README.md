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
- screen snipping with a native capture overlay and persistent preview

Core shell rule: the top and bottom bars reserve primary-monitor AppBar work area.

Current truth boundary: workspace restoration is reserved and not implemented; startup commands are not executed automatically; automation forwarding is planned and not wired; multi-monitor support is planning-only, with live shell ownership remaining a single-monitor runtime.

Screen snipping is implemented as a scoped native flow, but its native Windows journey is not yet certified. The crop control is appended after Sound in the existing top-bar order; its configurable fifth hotkey defaults to `Alt+S`. A legacy `Alt+S` assigned to Search is preserved and reported as an actionable conflict rather than silently remapped (for example: “Legacy Alt+S on Search preserved; snipping inactive until shortcut conflict repaired”). The original four hotkeys and control order remain unchanged.

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

## Speech model setup

Fresh bootstrap and native builds do not require a model in the repository. Import the model once from the running app:

1. Start JasonShell with the normal [bootstrap](#first-run-bootstrap) or later-run command.
2. Open **Settings → Speech** and choose **Import speech model**.
3. Select a trusted Parakeet TDT 0.6b v2 int8 ONNX `.tar`, `.tar.gz`, or `.tgz` archive.
4. Wait for validation and installation to finish. The app validates the staged archive and loads the model before reporting it ready.

Alternatively choose **Import speech model folder** and select a trusted extracted folder containing `encoder-model.int8.onnx`, `decoder_joint-model.int8.onnx`, and `vocab.txt` directly at its root. Only these three regular, nonempty files are copied; extras such as `config.json` and `nemo128.onnx` are ignored. Linked/junction roots and required files are rejected. The folder is not modified or retained as an external model pointer; after successful import it is no longer needed.

The installed model is kept in JasonShell's per-user local application data, is ready for the current session, and is reused after restart and app updates. You can delete the original archive after a successful import; it is not needed again. Installed files remain until the app's local data is deleted (for example, by an uninstall or profile cleanup). Audio and transcripts are not newly persisted by this setup.

Optional download source: [Handy's v0.6.0 model registry](https://github.com/cjpais/Handy/blob/v0.6.0/src-tauri/src/managers/model.rs) lists [the Parakeet v2 int8 archive](https://blob.handy.computer/parakeet-v2-int8.tar.gz). Use only an archive you trust. The registry link is provenance guidance, not a pinned hash or a license clearance; archive contents, revision, compatibility, and applicable usage terms have not been independently verified.

## First run bootstrap

Use this exact PowerShell bootstrap on first run; import a speech model after the app starts.

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
- grouped tiles (two or more windows) offer **Open in Process Manager**, **Pin to taskbar**, and **Close all windows** on right-click, without opening the gallery first
- **Close all windows** sends normal close requests to the selected group's captured, still-current windows; apps can keep unsaved-work prompts open, and this action does not force-kill processes
- includes process manager access
- reserves AppBar work area on the primary monitor

### Quick Launch

- uses current Explorer taskbar pins
- launches pinned apps from the native panel
- stays tied to the bottom bar launcher source of truth

### Stack Browser

- browse folders
- file operations
- On Windows, drag a file or folder row to Explorer or another Shell drop target with the primary mouse button; dragging multiple selected rows from the current folder requests a copy-only native Shell/OLE drag. Dragging rows from different parent folders reports an unsupported operation rather than sending an invalid payload. Released gestures cancel silently; native errors appear in the popup. Incoming drops remain supported.
- Native outbound drag has only static/compile validation so far. Explorer/Teams transfer, cancellation, focus behavior, and symlink identity still require consent-gated live Windows smoke; a returned `copied` effect indicates the Shell accepted a copy effect, not that the destination contents were independently verified.
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

### Screen snipping

- starts only from the exact live top-bar instance or the trusted native snipping hotkey dispatcher
- freezes all currently enumerated monitors before creating capture overlays; normal top/bottom bars, panels, and their tooltips remain in the capture, while snipping overlays and the prior preview are capture-excluded
- binds the first backend-accepted monitor pointer-down to the generation; the crop is converted from trusted native monitor metadata
- publishes automatic Copy immediately after hidden preview preparation and before showing the preview; the reported result remains truthful for committed, warning, rejected, or publication-unknown outcomes
- keeps a non-activating preview with **Copy**, **Save**, and **Dismiss** actions; Save uses a backend native PNG picker and sibling-file atomic publication, and picker cancellation writes nothing
- keeps save operations pinned to their generation/image and refuses replacement while the picker is active
- uses path-free IPC, exact concrete-window authorization, binary per-monitor image responses, scoped snipping capabilities, bounded helper deadlines, and capture/staging budgets

Snipping has no annotations, spanning mode, autosave, capture history, upload, or renderer-supplied paths/handles. The runtime remains scoped to the existing primary-monitor AppBar shell ownership; multi-monitor capture is a snipping capture operation, not multi-monitor AppBar ownership.

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
npm run test:component
npm run test:search
npm run p03:experiment
npm run cargo:check
npm run cargo:test
npm run smoke:runtime
npm run smoke:fullscreen
npm run validate
npm run tauri
```

Validation note:

- Windows native behavior still needs manual smoke
- `npm run smoke:fullscreen` is part of the expected Windows smoke path
- snipping product/coordinator/save/hotkey/native-registry/runtime tests provide bounded automated evidence; the standalone native synthetic-region journey, real mouse/AppBar/mixed-DPI hardware, and consumer paste remain unverified

## Caveats

- prototype, not final shell product
- primary-monitor AppBar reservation is the current runtime target
- multi-monitor support is planning-only; live shell ownership remains a single-monitor runtime until implemented and live-tested
- workspace restoration is reserved and not implemented
- workspace startup commands are not executed automatically
- automation forwarding is planned and not wired
- tray behavior needs caution
- native Windows behaviors require manual smoke
- snipping's native journey remains pending; automated fixtures do not prove live HWND discovery, GDI capture, compositor timing, clipboard paste, or AppBar coexistence

## Documentation links

- `master_spec.md` - canonical behavior and architecture
- `package.json` - exact scripts and toolchain entrypoints
- `scripts/bootstrap-windows.ps1` - first-run bootstrap path
- `docs/` - repo docs and smoke references

## Credits

Speech indicator fluid background adapted in-app from [Rare UI](https://rareui.com)'s [Fluid Orb](https://rareui.com/components/fluidorb), copyright (c) 2026 Swami Malode. Native WebGL only; no added runtime dependencies. Full MIT + Commons Clause + Attribution terms: [`src/lib/speechIndicatorOrb.LICENSE`](src/lib/speechIndicatorOrb.LICENSE). Not distributed as a standalone component or component library.
