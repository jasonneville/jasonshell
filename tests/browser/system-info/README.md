# Production-mounted System Info acceptance

Run existing Vite only: `node node_modules/vite/bin/vite.js --config tests/browser/system-info/vite.config.ts`.
Open `http://127.0.0.1:4181` using `wmux browser open`, snapshot, click **Run acceptance journey**, re-snapshot and capture `#result` plus ledger.
The fixture mounts the production component/CSS; only Tauri IPC is replaced. Delayed promises cover targeted reopen and stale snapshot completion; the journey visits both tabs, cancellation, failure, dismissal and recovery. No actual processes are terminated.

Browser run is blocked when `wmux` is absent. Do not silently use an invisible alternate browser. A successful fixture build is not E2E evidence, WebView2 evidence, or native telemetry certification. No live Tauri shell startup permitted without human consent.
