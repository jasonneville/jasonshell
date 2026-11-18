# Isolated Settings Exit browser acceptance

From the repository root, run these in separate terminals:

```powershell
npx vite --config tests/browser/graceful-exit/vite.config.ts
node tests/browser/graceful-exit/run.mjs
```

Stop Vite with Ctrl+C afterward. Set `AGENT_BROWSER_BINARY` if needed.
The isolated browser mounts production Settings and Control Plane with mocked
IPC. It checks normal cross-document theme authority, Escape and focus return,
draft preservation on cancellation, and a single path-free confirmed request.
It does not start Tauri, invoke Windows power, or verify native process cleanup.
