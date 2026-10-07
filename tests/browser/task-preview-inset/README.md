# Actual-component inset lens acceptance

No Tauri runtime, native shell smoke, or package installs. Mounts production TaskPreviewSurface and app.css, with fail-closed mocked Tauri imports.

Start existing Vite:

```sh
node node_modules/vite/bin/vite.js --config tests/browser/task-preview-inset/vite.config.ts
agent-browser --session preview-inset open http://127.0.0.1:4180
agent-browser --session preview-inset wait --fn 'window.previewReady === true'
agent-browser --session preview-inset eval 'window.previewFixture.run().then(r=>({failureCount:r.failures.length,failures:r.failures, evidence:r.evidence}))'
```

Acceptance matrix: dark/light × captured/native/unavailable × normal/long captions × present/missing decorative icon; broken icon error hides image. Checks border1/padding10, gradient, soft8 corners, filled close, 18px decorative icon, actual ellipsis/nonoverlap, independent focus, close containment, frame interior {left12,top48,right320,bottom216} at332x228. Reports scale projections at1/1.25/1.5/2; Rust unit tests check actual scale math. Browser scale projections are not hardware DPI or DWM compositing evidence.

RED: 221 failed matrix assertions against old styling, content {left6.59375,top45.78125,right325.40625,bottom221.40625}, icon absent. Screenshot: C:/Users/jnev1/AppData/Local/Temp/task-preview-inset-red.png.

## Final GREEN

- Focused Node: `node --test tests/taskPreviewBaseline.test.mjs tests/taskPreviewTextPolish.test.mjs tests/taskPreviewRetention.test.mjs tests/taskbarPreviewContract.test.mjs tests/popupAnchorAttachment.test.mjs` =>38 passed,0 failed.
- Canonical `npm run test:component -- tests/components/task-preview.test.ts` =>14 passed (unchanged component source; evidence reused). Invoke npm.cmd via PowerShell with existing Node/cargo PATH prefix if necessary.
- `window.previewFixture.run()` via agent-browser session preview-inset =>25 scenarios,0 failed assertions; dark/light × captured/native/unavailable × normal/long × present/missing, plus broken icon/new URL recovery/whitespace omission. Interior exactly {left12,top48,right320,bottom216}.
- Illustrative bottom-bar context measured8 CSS pixels below actual component; NOT native placement proof. Screenshots: C:/Users/jnev1/AppData/Local/Temp/task-preview-floating-green-dark-native.png and task-preview-floating-green-light-captured.png.
- Native authority: fixer task_preview::tests module11 passed; cargo check passed. Additional test-owner `cargo test --manifest-path src-tauri/Cargo.toml task_preview::tests::floating_preview_gap -- --nocapture` =>1 passed after adding1.1 (8.8 rounds9) and1.3 (10.4 rounds10), alongside1/1.25/1.5/2. Production uses rounded scaled logical margin8; above/below placement and monitor clamps asserted.
- `node tests/browser/bottom-bar-insets/run.mjs` =>233 checks passed,0 failed; production BottomBar.css, dark/light heights24/32.4/48, no bridge, unchanged connected-state background/border/bevel, active/attention, overflow, reduced motion and keyboard focus. State comparisons await CSS animation completion on BOTH sides; initial transient-state comparison failures corrected without changing acceptance.
- Logs in C:/Users/jnev1/AppData/Local/Temp/: preview-final-node-green.log, preview-final-browser-green.json, preview-floating-rounding-green.log, bottom-bar-insets-final-clean.json. Last file has a Node shell-deprecation warning after complete JSON report.

Floating amendment RED retained: focused Node21 passed/4 failed; native gap test actual0 expected8. Logs preview-floating-red.log and preview-floating-cargo-red.log in same Temp directory.

Native journey skipped: appearance/placement-only, existing IPC interaction regressions retained (pointer entry retention and sibling close do not schedule hide). Actual DWM compositing and real pointer crossing remain unverified and consent gated.
