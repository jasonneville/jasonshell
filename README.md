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

## Notification Badges

Bottom-bar badges use Windows toast deliveries since each app was last focused. This is not a Teams unread count. Windows notification history requires package identity, so register the local development sparse package before testing badges:

```powershell
cargo build --manifest-path src-tauri/Cargo.toml
powershell -ExecutionPolicy Bypass -File .\scripts\register-notification-identity.ps1
```

The script creates a Current User development certificate, trusts it only for Current User, copies the debug executable into an untracked local loose package, registers it, then launches JasonShell through that package identity. Close any `npm run tauri dev` instance first. Re-run it after rebuilding the executable.

Toast counts, native taskbar attention, CPU/title activity, and Explorer taskbar suppression are independent signals. Attention renders amber, toast delivery count renders a red badge, and busy activity keeps its existing treatment.

## Taskbar Controls

Native taskbar attention is enabled by default so ordinary `FlashWindowEx` requests appear as amber attention cues. Set its kill switch to `0` only to disable the receiver. Multi-monitor Explorer taskbar suppression remains default-off pending the full Windows 10/11 release matrix:

```powershell
$env:JASONSHELL_TASKBAR_NATIVE_HOOKS = '0' # optional attention kill switch
$env:JASONSHELL_EXPLORER_SUPPRESSION_V2 = '1'
```

Attention hooks listen for native shell flash/foreground events unless explicitly disabled with exact `0`. Explorer suppression v2 activates only with exact `1`; it owns only taskbars it successfully hides, reconciles Explorer recreation/display changes, and identity-checks taskbars before restore.

Read bounded runtime health without opening UI through Tauri command `get_taskbar_runtime_diagnostics`. It reports native-hook health/last signal, snapshot sequence/reason/latency, attention count, toast listener/package identity/poll state, unresolved app-ID counts, and Explorer suppression counters. Exported strings are path-redacted and samples are bounded.

Troubleshooting:

- Missing toast badges: run the package-identity registration script, then inspect `toastListenerStatus`, `packageIdentityStatus`, and unresolved app-ID diagnostics.
- Missing attention: confirm `JASONSHELL_TASKBAR_NATIVE_HOOKS` is not `0`, restart JasonShell, inspect native-hook health, then run `powershell -ExecutionPolicy Bypass -File .\scripts\smoke-taskbar-attention.ps1` against a built debug executable.
- Legacy `NotifyIcon` balloon tips may lack a resolvable app identity, so their red toast badge is not guaranteed; a paired `FlashWindowEx` request still produces the amber attention cue.
- Explorer taskbar remains visible/reappears: confirm v2 switch, inspect tracked/hidden/recreation/hide-failure diagnostics, then disable the switch if identity or restore errors appear.
- Unsupported until release matrix passes: any general Windows 11 support claim, protected/elevated cross-integrity windows, and guaranteed toast identity resolution for every unpackaged app.

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
