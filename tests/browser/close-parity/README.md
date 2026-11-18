# Browser close parity (AC-2 / NFR-1)

Run from repo root using existing tools only:

```powershell
node node_modules/vite/bin/vite.js --config tests/browser/close-parity/vite.config.ts
# Separate terminal; use an isolated browser profile, not your normal Edge session.
$edge = "${env:ProgramFiles(x86)}/Microsoft/Edge/Application/msedge.exe"
Start-Process $edge -ArgumentList '--headless=new','--remote-debugging-port=9295',"--user-data-dir=$env:TEMP/opencode/close-parity-edge",'about:blank'
node tests/browser/close-parity/run.mjs
node tests/browser/close-parity/terminal.mjs
node --test tests/browser/close-parity/focus-paint.test.mjs
```

Run suites sequentially for trusted keyboard focus. Both accept optional CDP URL, Vite URL, and output directory (in that order). Use a distinct output directory for baseline and post-change runs. JSON reports and PNGs go under `test-results/` by default. These mount production Svelte components with the official Tauri mock bridge, not native windows; neither native focus/blur nor DWM contrast can be certified here. Runner uses the existing CDP helper because the installed agent-browser CLI timed out.

Pinned `0194489` evidence: `test-results/close-parity-baseline/report.json` and PNGs captured all 24 anchored combinations. Click, Space, and Escape each dispatched their original hide command exactly once; Audio and Speech failed icon parity, and Tray used the same glyph but different chrome.

Paired terminal RED: `test-results/close-parity-terminal-baseline/` contains six screenshots and the JSON report (six states, 15 intended visual/accessibility failures, no setup failures). The Stack-only fixture resolver opts the real xterm constructor into `allowProposedApi` so the production Unicode11 addon can initialize; the production startup configuration itself is unchanged and not certified by this run. Stack rendered `Close` at approximately 42.36 × 24.39 CSS px; dedicated rendered `Esc` at approximately 29.16 × 23.19 CSS px. Neither used the required icon; Stack lacked the agreed search-specific accessible name and dedicated missed the minimum height. The runner preserves both existing Escape paths: it asserts that Escape in the search input leaves it open and focused with query intact, and separately seeds a query to verify that Escape from xterm clears it and closes search.

Review reruns: `test-results/close-parity-anchored-review-final/` (24 states, zero failures) and `test-results/close-parity-terminal-review-final/` (six states, zero failures). Both reject zero-width or transparent focus paint; the Node check exercises the invisible-paint cases without changing product CSS.
