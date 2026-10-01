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

## Speech model setup (before bootstrap or native build)

Speech model binaries are locally provisioned setup artifacts, not Git-managed files. The five model files are removed from this branch's unpublished history and ignored; fresh clones therefore need the bundle provisioned using the steps below before bootstrap (which launches the app) or a native build. Tauri still bundles `src-tauri/resources/speech-models/parakeet-tdt-0.6b-v2-int8/**/*`; ignoring them in Git does not change packaging.

Use the matching **Parakeet TDT 0.6b v2 int8 ONNX bundle**, not NVIDIA training weights or a different ONNX export. [Handy's v0.6.0 model registry](https://github.com/cjpais/Handy/blob/v0.6.0/src-tauri/src/managers/model.rs) maps model ID `parakeet-tdt-0.6b-v2` to directory `parakeet-tdt-0.6b-v2-int8` and download URL `https://blob.handy.computer/parakeet-v2-int8.tar.gz`. That endpoint was verified to respond with HTTP 200 and gzip content, 473,166,028 bytes (about 451 MiB compressed). The archive contents, inference compatibility, model revision, and license provenance have not been independently verified; confirm applicable usage terms before use. The registry citation identifies the download source, not a pinned artifact hash.

From the repository root, download and extract into a unique user-temporary staging directory. Windows `tar.exe` must be available. The archive root layout is not assumed: this finds exactly one directory containing all required filenames, otherwise it stops.

```powershell
$ErrorActionPreference = 'Stop'
$required = @('encoder-model.int8.onnx', 'decoder_joint-model.int8.onnx', 'vocab.txt')
$staging = Join-Path $env:TEMP ('jasonshell-speech-' + [guid]::NewGuid().ToString('N'))
$archive = Join-Path $staging 'parakeet-v2-int8.tar.gz'
$extracted = Join-Path $staging 'extracted'
New-Item -ItemType Directory -Path $extracted -Force | Out-Null
Invoke-WebRequest -Uri 'https://blob.handy.computer/parakeet-v2-int8.tar.gz' -OutFile $archive
tar.exe -xzf $archive -C $extracted
if ($LASTEXITCODE -ne 0) { throw 'Model archive extraction failed' }
$candidates = @(Get-ChildItem -LiteralPath $extracted -Recurse -File -Filter 'encoder-model.int8.onnx' |
    ForEach-Object { $_.Directory.FullName } |
    Where-Object {
        $directory = $_
        @($required | Where-Object {
            !(Test-Path -LiteralPath (Join-Path $directory $_) -PathType Leaf)
        }).Count -eq 0
    } | Select-Object -Unique)
if ($candidates.Count -ne 1) { throw 'Expected exactly one complete Parakeet model directory' }
$source = $candidates[0]
```

Alternatively, skip the download block and set `$source` to a trusted existing bundle copied from another device, or your populated Handy directory:

```powershell
$source = Join-Path $env:APPDATA 'com.pais.handy\models\parakeet-tdt-0.6b-v2-int8'
```

Handy is not required at JasonShell runtime. With `$source` set by either route, install only the model bundle files and verify the copy:

```powershell
$ErrorActionPreference = 'Stop'
$destination = Join-Path (Get-Location).Path 'src-tauri\resources\speech-models\parakeet-tdt-0.6b-v2-int8'
$required = @('encoder-model.int8.onnx', 'decoder_joint-model.int8.onnx', 'vocab.txt')
foreach ($name in $required) {
    $file = Join-Path $source $name
    if (!(Test-Path -LiteralPath $file -PathType Leaf) -or (Get-Item -LiteralPath $file).Length -eq 0) {
        throw "Missing or empty source model file: $file"
    }
}
New-Item -ItemType Directory -Force -Path $destination | Out-Null
foreach ($name in ($required + @('config.json', 'nemo128.onnx'))) {
    $file = Join-Path $source $name
    if (Test-Path -LiteralPath $file -PathType Leaf) {
        Copy-Item -LiteralPath $file -Destination $destination -Force
        $installed = Join-Path $destination $name
        if ((Get-FileHash -LiteralPath $file).Hash -ne (Get-FileHash -LiteralPath $installed).Hash) {
            throw "Model copy verification failed: $name"
        }
    }
}
foreach ($name in $required) {
    $file = Join-Path $destination $name
    if (!(Test-Path -LiteralPath $file -PathType Leaf) -or (Get-Item -LiteralPath $file).Length -eq 0) {
        throw "Missing or empty installed model file: $file"
    }
}
Get-ChildItem -LiteralPath $destination | Select-Object Name, Length
```

Required layout (files directly inside this directory, not another nested model folder):

```text
src-tauri/resources/speech-models/parakeet-tdt-0.6b-v2-int8/
  encoder-model.int8.onnx
  decoder_joint-model.int8.onnx
  vocab.txt
```

The imported bundle also contained `config.json` and `nemo128.onnx`; the copy above preserves those when available, while the runtime's explicit required-file check names the three files shown. Presence and copy hashes do not prove model compatibility or provenance. The known encoder alone is 652,184,014 bytes (about 622 MiB); allow roughly 632 MiB for the full bundle, plus the compressed download, staging, and build/package copies. Staging files remain in `$staging` for inspection; remove them manually when no longer needed. Transferring or downloading the bundle and provisioning build dependencies are setup costs and may require network access. Speech inference uses the installed local model offline; bootstrap does not download this model.

The model files are excluded from the outgoing branch history and ignored locally. Ignore rules alone do **not** remove already tracked files or large blobs from existing commits; a push containing such a committed model needs separate history remediation.

## First run bootstrap

After completing the speech model setup above, use this exact PowerShell bootstrap on first run:

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
