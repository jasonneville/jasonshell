# JasonShell

JasonShell is a Windows shell-foundation prototype built with Tauri 2, Svelte 5, TypeScript, Rust, and Win32 APIs. It behaves like a small native shell rather than a single-page web app: six Tauri webview windows cooperate through explicit commands and events.

## Current Product

- `top-bar`: primary-monitor top AppBar with pinned folder stacks, date/time, and search entry.
- `bottom-bar`: primary-monitor bottom AppBar with Explorer taskbar `.lnk` launchers, grouped open-window task tiles, previews, menus, reorder gestures, and a process-manager button.
- `search-panel`: hidden auxiliary webview for richer search results from pinned apps, open windows, static commands, and indexed system app/file/folder sources.
- `stack-popup`: hidden auxiliary folder browser with navigation, sorting, selection, file operations, inline rename/new-folder, drag/drop, context menus, and pin updates.
- `task-preview`: hidden auxiliary preview webview for open-window hover previews.
- `process-manager`: hidden auxiliary process table with sortable process metadata and guarded kill actions.

The top and bottom bars reserve primary-monitor edge space through the Windows AppBar API while the app is running. Multi-monitor parity is not complete.

## Documentation Authority

1. `master_spec.md` is the canonical current architecture, behavior, invariants, risks, and validation reference.
2. Current source code and tests are authoritative for implemented behavior when docs are stale.
3. `README.md` is the setup and product overview.
4. `action_plan.md` and `future_enhancements.md` describe roadmap sequencing.
5. `features.md`, `plan.md`, `wave_*.md`, and `stack_browser.md` may contain historical or aspirational notes unless explicitly marked current.

## Requirements

- Windows
- Node.js 20+
- Rust toolchain
- Microsoft WebView2 runtime

## Install

First run: bootstrap rather than `npm install`.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\bootstrap-windows.ps1
```

That script installs missing Rust, Node.js/npm, MSVC build tools, and WebView2 with winget when needed, imports MSVC dev env, then runs the lockfile-safe repo install step (`npm ci`) before launching JasonShell.

## Run

```powershell
npm run tauri dev
```

If prerequisites are already installed and repo deps are present, you can also run the app directly with the dev command above.

During `npm run tauri dev`, the primary shell surfaces report live runtime metrics to the terminal. Use those metrics to catch zero-height WebView/native-window regressions.

## Validation

```powershell
npm run check
npm run build
npm run test:node
npm run cargo:test
npm run cargo:check
npm run validate
```

`npm run test:search` remains as a compatibility alias for `npm run test:node`; the Node harness now covers more than search helpers.

Live Windows behavior still needs smoke testing for WebView2 delivery, exact AppBar/popup geometry, Explorer drag/drop cursor behavior, native menus, process-manager focus-loss behavior, and mouse XButton delivery. See `docs/smoke-test-windows.md`.

## Current Caveats

- Primary-monitor AppBar behavior is the active target; multi-monitor parity is future work.
- System tray integration is parked/experimental until a visible surface and registered command path are confirmed.
- Historical Cairo-derived docs may describe future direction rather than shipped JasonShell behavior.
