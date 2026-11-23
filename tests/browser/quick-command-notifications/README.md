# Quick Command notification renderer journey

Start the fixture: `npx vite --config tests/browser/quick-command-notifications/vite.config.ts`.

In another PowerShell terminal:

```powershell
try {
  agent-browser --session qc-notifications open http://127.0.0.1:4196
  if ($LASTEXITCODE) { throw 'Browser launch failed' }
  agent-browser --session qc-notifications wait --fn 'typeof window.runJourney === "function"'
  if ($LASTEXITCODE) { throw 'Fixture did not start' }
  agent-browser --session qc-notifications eval 'window.runJourney()'
  if ($LASTEXITCODE) { throw 'Journey failed' }
} finally { agent-browser --session qc-notifications close }
```

Each iframe mounts the actual renderer. Native completion, persisted history, panel visibility and targeted IPC delivery are mocked; no subprocess, Windows shell or WebView2 is launched. The journey uses DOM clicks and the same native wake as Alt+Q, not physical hotkey registration. It does not establish queue policy, HWND behavior, or audio.

Automated: `node tests/browser/quick-command-notifications/run.mjs` (headless Chromium over CDP; set CHROME_PATH if needed) runs the journey for base-dark/base-light with and without reduced motion, plus a real-pointer hover pause/resume check (VIS-20b). Forced-colors focus coverage lives in `tests/browser/forced-colors-focus/run.mjs` (VIS-10).
