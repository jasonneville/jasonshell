# Renderer custom-theme acceptance fixture

Run `npx vite --config tests/browser/custom-themes/vite.config.ts` and use
`http://127.0.0.1:4186/?surface=settings`, `?surface=control`, or
`?surface=observer` in separate tabs of **one browser session**. This mounts the
production Settings/Control Plane components and CSS, mocks only native IPC, and
uses real same-origin localStorage, storage events and BroadcastChannel.

Pre-mount hydration is captured in `#bootstrap`. Read-only
`window.themeFixture.snapshot()` exposes durable bytes and resolved paint;
`paint(token)` resolves color/shadow aliases through actual browser CSS. Fixture
Hide/Show preserves the mounted component (not a remount), like a hidden shell
window. Reload is a renderer restart, not a native-process restart. Real browser
profile restart additionally verifies storage survives browser teardown.

Required journey: Create copy → edit Accent → observer receives preview without
canonical write → hide/show retains text → invalid hex keeps last valid paint →
late observer joins preview → Cancel restores all → Save & apply writes one
canonical collection+selection → fresh page pre-mount captures saved accent →
unsaved edit/reload restores saved only → delete Cancel byte-identical → confirm
delete makes every observer Base Dark. Also run strong focus/reduced transparency,
unexpected owner closure, stale replay, hostile palette recovery, reset and all
34-preset computed-equivalence probes when engine/UI are ready.

This fixture cannot establish Tauri/WebView2 transport behavior, native hidden
window scheduling, HWND/AppBar behavior or native restart. No live native smoke
is authorized by this fixture.

## Repeatable automation (no added dependencies)

Prerequisites: repository `node_modules`, Node 24 with built-in WebSocket, and an
already-installed Chrome/Chromium. Start the fixture server in one terminal:

```powershell
npx vite --config tests/browser/custom-themes/vite.config.ts
```

Start an **isolated test browser**, never a personal browser/profile. These are
the installed browser/version and approved temporary directory used for evidence:

```powershell
& 'C:\Users\jnev1\.agent-browser\browsers\chrome-148.0.7778.97\chrome.exe' --headless=new --remote-debugging-port=9227 --user-data-dir='C:\Users\jnev1\AppData\Local\Temp\opencode\custom-theme-cdp-profile' --no-first-run --no-default-browser-check about:blank
```

In another terminal run:

```powershell
node tests/browser/custom-themes/run.mjs http://127.0.0.1:9227
node tests/browser/custom-themes/restart.mjs 'C:\Users\jnev1\.agent-browser\browsers\chrome-148.0.7778.97\chrome.exe'
```

`run.mjs` accepts any isolated Chrome CDP endpoint. The evidence run initially
used agent-browser's already-launched isolated Chrome at port **62505**; its CLI
daemon transport timed out (`os error 10060`) and `doctor --offline --quick`
also timed out. The fallback uses only Node's WebSocket and Chrome's CDP, without
installing or importing browser automation packages. The runner opens/closes its
own separate pages, clears this fixture origin's storage, and fails nonzero on
assertion/paint mismatches. Do not target a personal browser with this command.

`restart.mjs` owns a separate temporary profile and two actual Chrome launches;
it saves a custom theme, starts an unsaved preview, exits Chrome, reopens the
same profile, asserts pre-mount saved-only hydration and byte-identical storage,
then closes Chrome and removes its own profile. This is **browser-process**
restart, not Tauri/native Windows restart or shell certification.

Artifacts are written under `C:/Users/jnev1/AppData/Local/Temp/opencode/`:

- `custom-theme-browser-report.json`: assertions and all 3,876 comparisons
  (34 presets × 38 tokens × default/strong-focus/reduced-transparency).
- `custom-theme-restart-report.json`: actual browser-process restart result.
- `custom-theme-editor-preview.png`, `custom-theme-editor-320px.png`,
  `custom-theme-editor-360px.png`, `custom-theme-settings-saved.png`, and
  `custom-theme-recovery-hostile.png`: production UI evidence.

The independent CDP test browser can be closed after the runner; never use a
machine-wide Chrome/process cleanup command.

## Leader-approved baseline repair rendering checks

With the same CDP browser/custom-theme server, additionally start the existing
gallery/BottomBar production fixture in a separate terminal:

```powershell
npx vite --config tests/browser/task-gallery/vite.config.ts
node tests/browser/custom-themes/baseline-repairs.mjs http://127.0.0.1:9227
```

The second command belongs in the automation terminal (the Vite server stays
running at port 4179). It mounts actual shared ContextMenu/SpeechIndicator
components at `?surface=baseline` with mocked native status/window calls, then
uses the existing production BottomBar/gallery fixture. Settled browser paint
checks dark/light menu alpha+blur under normal/reduced transparency; quiet and
speaking 40×40 shell / 24×24 artwork; flat bottom/gallery tiles with retained
180×136 gallery geometry, bevel and approved outer gradient. CSS transitions are
awaited before paint samples. This does not exercise native window geometry.

Outputs: `custom-theme-baseline-render-report.json`,
`custom-theme-baseline-menu-speech.png`, and
`custom-theme-baseline-flat-tiles.png` in the same approved Temp/opencode folder.
