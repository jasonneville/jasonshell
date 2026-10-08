# Ribbon/Matrix production browser acceptance

**Evidence boundary:** real `BottomBar.svelte`, `TaskGallerySurface.svelte`, shared menu components, and production global/component CSS; mocked Tauri IPC only. Captured-image responses are explicitly synthetic test PNGs. No real HWND discovery, DWM composition, WM_CLOSE delivery, immutable native identity, WebView2 behavior, or shell/AppBar certification. Native live-shell E2E is skipped pending explicit human consent. Never launch Tauri from this fixture.

## Run after implementation handoff

Use preexisting Vite and agent-browser; no installs. Separate terminals:

```powershell
node node_modules/vite/bin/vite.js --config tests/browser/task-gallery/vite.config.ts
node tests/browser/task-gallery/run-tight-seams.mjs
```

URL: `http://127.0.0.1:4179/`. Runner uses isolated `task-gallery-ribbon-matrix` browser session and closes that session afterward. Outputs JSON, JUnit XML, HTML, and screenshots under ignored `test-results/task-gallery/`. Nonzero exit preserves every failed assertion. Current file name `run-tight-seams.mjs` is retained as the existing entrypoint; its old tab-strip acceptance has intentionally been replaced.

Preparation phase: browser acceptance authored but not yet executed. Initial component RED: nine legacy-runtime failures. Additional RED: failed captured-image error did not degrade to unavailable. Do not interpret these artifacts as GREEN or coverage percentages. Final full `npm run validate` belongs solely to the same E2E runner after both implementation reports.

## Geometry and scenarios

Native-sized hosts follow authoritative layout, never the superseded 24/32.4px gallery strip. The **bottom bar** still uses 24px in mocked settings; it does not control gallery height.

- Ribbon 2/3: `384×184` / `572×184` when fitting.
- Matrix 4+: at most three columns and two visible rows; `584×328` at full width.
- Narrow wrapping: available widths 396 and 144; small groups also wrap into Matrix.
- Insets 8, header 24 plus gap 8, card gap 8, cards 180×136, thumbnail 172×96, footer 32, independent 24×24 Close. Matrix reserves 12px vertical scrollbar; undersized width reduces card/thumbnail, not Close target.
- Dark/light × 2/3/4/6/30 × wide/two-column/undersized-width matrix. Tests measure actual DOM dimensions, vertical scroll, horizontal containment, title truncation, exact snapshot order, theme colors and visible Close targets.
- Spatial ArrowUp/Down, short final-row clamping, Home/End scrolling, 3→4→3 preserving surviving Close DOM focus, 6→30 overflow.
- Inline current-revision sync, bounded fully contained native frame set, scroll/resize newer request IDs, empty native set before context overlays and cleanup.
- Captured/minimized/unavailable states, failed app/capture images, no separate hover preview.
- Exact close nonce/HWND, duplicate suppression, visible failure and focus preservation, successful close retaining card until authoritative snapshot removal.
- Stale snapshot/async capture responses inert, Escape/context dismissal, unmount cleanup.
- Production BottomBar tile → native mock show → gallery → activate/close → authoritative BottomBar snapshot removal → stale update rejection.
- Browser-generated click/Tab/Enter on sibling Close; separate reduced-motion geometry/animation check.

Selectors use accessible controls for browser-generated actions; card/viewport geometry uses stable component hooks where roles cannot express layout. No gallery CSS is copied or overridden in the fixture. `#surface` sets only native host dimensions; fixture controls/ledger live outside it.

## Controllable test-only state

`window.galleryFixture` exposes `open`, `snapshot`, `bottomSnapshot`, `configure`, `measure`, `publish`, `unmount`, `mountGallery`, `settle`, `pendingSync`, and the IPC ledger. `snapshot(rows, revision)` can inject an older revision without resizing the host. `state.deferSync` enables explicit late-response tests; resolve using the captured request's nonce/revision/requestId. Mock close acceptance never removes rows. Only snapshots do.

The mock owns no OS resources; `state.liveFrames` is a command ledger aid, not evidence of native resource cleanup. Native pure tests separately cover serialized geometry and request/frame admission. Injectable fake-resource ownership remains a requested seam; static Drop/source guards alone cannot establish native lifecycle behavior.
