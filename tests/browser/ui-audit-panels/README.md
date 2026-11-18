# Populated panel renderer audit

From the repo root, use separate terminals for the local fixture and the runner:

```powershell
node node_modules/vite/bin/vite.js --config tests/browser/ui-audit-panels/vite.config.ts
# Separate terminal: launch only an isolated headless browser.
$edge = "${env:ProgramFiles(x86)}/Microsoft/Edge/Application/msedge.exe"
Start-Process $edge -ArgumentList '--headless=new','--remote-debugging-port=9397',"--user-data-dir=$env:TEMP/opencode/ui-audit-panels-9397",'about:blank'
node tests/browser/ui-audit-panels/run.mjs test-results/ui-audit-panels-red
```

Three themes per surface; translucent uses reduced viewport width. Real loader and app CSS, Tauri IPC mocked. Trusted CDP Tab and keyboard input; no native application or hardware. The focus probe measures settled paint against viewport and overflow clipping ancestors, checking simple solid spread shadows and outlines. Screenshots and JSON go to the supplied output directory.

- **Audio PASS (3):** reachable device triggers and master/session sliders; keyboard updates produce one mock command each. Narrow output trigger spans x=10.91–269.09 inside x=1–279, including its 2px ring.
- **Tray RED (3):** Shift+F10 sends one right relay per icon, no hide. Success reloads the grid but drops keyboard focus to `BODY`; rejected relay preserves the grid, focus and inline alert. First-row focus rings also clip against the grid scrollport: at 210px, the first button starts at x=9.80, y=50.77, exactly the left/top clip boundary, losing 2px of outward ring on those sides. Rejection intentionally adds an alert; this is distinct from the clipping and focus loss. The Explorer-owned menu itself, native foreground, and blur hold remain unverified.
- **Tray GREEN (3):** `node tests/browser/ui-audit-panels/run.mjs test-results/ui-audit-panels-tray-green tray-panel`. Successful reload preserves the same focused DOM button. At 210px, its 2px ring spans x=11.80–47.80 and y=52.77–88.77, inside the scrollport on every side. Rejected relay retains focus and all 12 icons, with an inline alert. Delayed success publishes a new snapshot but leaves focus where the user tabbed (header Close); duplicate Shift+F10 during the pending relay sends no extra request. No hide IPC. Close/reopen during a pending relay was not exercised; this run covers the Tab-away path.
- **Process Manager PASS (3):** populated process list and status, typed filtering, arrow switch to populated System Info with focus and selection, and return with filter retained. Horizontal grid scrolling is intentional; the probe tests filter and tabs, not every grid column or destructive action.

Tray read ordering: `node tests/browser/ui-audit-panels/tray-read-races.mjs` on the same ports. Baseline 4 RED: trusted pointer relay → Escape/accepted mock hide → reopen, and overlapping open reads without a hide, each with stale success and stale rejection. Revision 3 renders before settling deferred revision 2. Stale success replaces 3 with 2; stale rejection removes the grid and shows an unavailable alert. Neither should change the current grid, alert, or loading state. Close notification belongs to the top-bar owner; this fixture retains the Tray webview and mocks the hide and reopen boundary without exercising native visibility. Pass a distinct output directory for a post-fix run.
