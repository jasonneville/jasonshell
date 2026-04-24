# JasonShell

JasonShell is a Windows shell-foundation prototype built with **Tauri + Svelte + TypeScript + Rust**.  
This milestone delivers two polished dark-mode shell surfaces on the **primary display**:

- a **top menu bar** at roughly half normal taskbar height
- a **bottom taskbar-like bar** at roughly normal Windows taskbar height

While the app is running, the bars reserve screen edge space through the Windows **AppBar** API so ordinary windows can use the center workspace between them.

## Current milestone

- Two independent Tauri windows: `top-bar` and `bottom-bar`
- Native positioning on the primary monitor
- Windows AppBar reservation with cleanup on exit
- Conservative Explorer taskbar hiding/restoring when the default taskbar occupies the primary bottom edge
- Top bar right-side live date/time display
- Bottom bar pinned-launcher strip sourced from Explorer taskbar `.lnk` pins
- Modular Rust layout/native shell code for future expansion

## Requirements

- Windows
- Node.js 20+
- Rust toolchain
- Microsoft WebView2 runtime

## Install

```powershell
npm install
```

## Run the prototype

```powershell
npm run tauri dev
```

## Validation commands

```powershell
npm run check
npm run build
npm run cargo:test
npm run cargo:check
# or run the full validation bundle
npm run validate
```

During `npm run tauri dev`, each shell surface also reports live runtime metrics to the terminal, including its native window rectangle plus WebView height measurements. This is the fastest way to catch zero-height surface regressions.

## Notes

- This milestone targets the **primary monitor only**.
- If Explorer’s built-in taskbar already occupies the primary bottom edge, JasonShell hides it while the prototype is running and restores it during shutdown.
- The center workspace between the bars is intentionally left clear for normal application windows.
- Pinned launchers are currently enumerated from `%APPDATA%\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar`.
- JasonShell launches pinned apps by executing the `.lnk` itself through the Windows Shell instead of reconstructing shortcut targets.
- Unsupported or unresolvable pinned entries are skipped conservatively, so exact parity with Explorer taskbar pin classes and ordering is not guaranteed in this iteration.
