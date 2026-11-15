# Seven-action production-mounted mocked-IPC E2E

Start the test-only server from the repository root:

```powershell
node node_modules/vite/bin/vite.js --config tests/browser/seven-hotkeys/vite.config.ts
```

Then use the preinstalled browser CLI:

```powershell
pwsh -NoProfile -File tests/browser/seven-hotkeys/run.ps1
```

No dependencies are installed. No Tauri process, native hook, AppBar, process
termination or shell smoke is launched. Four separate same-origin documents
mount the real TopBar, BottomBar, SettingsPanelSurface and ProcessManagerSurface.
Their IPC imports alone are aliased to a shared native-boundary fixture.

The fixture models Tauri's **any-target default**, records listener targets,
and rejects wrong-target opening/capture delivery through the production
listeners. Its journey exercises repeated opening, PM returning to Processes,
Settings mount/theme draft retention, registered/native versus unregistered/local
capture, a temporary duplicate swap, failed Save/retry, committed-only unrelated
controls, Reset/Cancel, hide/reopen, focus loss after persistence, and committed
success with nullable status when postcommit native observation fails. Assertions exercise real production
handlers; native events and DOM key events are explicitly synthetic. This does
not certify Windows OS delivery, native authorization, HWND focus, registration,
native lease expiry or persistence rollback; Rust fixtures own those contracts.

Artifacts default to `%LOCALAPPDATA%\Temp\opencode\seven-hotkeys-browser-e2e`:
three screenshots, accessibility snapshot, summary, full IPC/event trace,
HTML report and seven-case JUnit XML (the composite browser assertions only,
not a report claiming the entire repository is green).
Use a fresh page for each journey (the runner opens one); the journey intentionally
does not silently reset an existing user draft. The origin is a test-only local
Vite server, never a running JasonShell renderer.
