# Stack Open With browser acceptance

Production StackPopupSurface + production CSS, mocked Tauri IPC only. No native picker, handler, or live shell invoked. Uses existing agent-browser; installs nothing.

Start fixture: `node node_modules/vite/bin/vite.js --config tests/browser/stack-open-with/vite.config.ts`

Run: `node tests/browser/stack-open-with/run-acceptance.mjs`

Artifacts: approved temp `opencode/stack-open-with-e2e/results.json`, screenshots. Actual browser mouse movement crosses right/left/diagonal gaps; DOM geometry and trusted pointer ledger prove the path. Controlled IPC covers loading/empty/rejection, same-file out-of-order responses, stable chooser under pointer/focus, captured path, duplicate launch and errors. Browser/native Windows association behavior remains outside this evidence boundary.

The runner owns an isolated agent-browser session. `pointer-path.mjs` connects to that browser's published CDP endpoint for a 16ms-per-point trusted mouse path; separate CLI process startup must not consume the product's 220ms hover grace. Diagonals deliberately traverse an adjacent parent action before crossing to a lower child action. The path trace records hit targets and expanded state at every point. A separate settle case proves grace still closes the child over another parent action. The browser is closed in the runner's `finally` block.

Context-menu anchoring and controlled IPC use fixture DOM/API calls; pointer traversal, clicks and keyboard input are browser-level input. These tests do not establish hardware delivery, WebView2/native windows, or real association-provider behavior. Native identity dedupe/revalidation and error propagation are separately source-contract assertions; Rust runtime tests exercise safe extensionless early return and the exact serialized candidate schema without invoking handlers.
