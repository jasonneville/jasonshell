# Tight seams browser acceptance

Safe static CSS fixture; no Tauri, IPC, launches, dependencies, or production edits. Imports actual `src/app.css` and `src/components/BottomBar.css`. Markup mirrors the current production root/strip/group/button hierarchy, not a restyled design facsimile. Existing mocked real-component suite owns interaction regression. Static markup does not certify component state generation or native geometry.

Two terminals from repository root:

```sh
node tests/browser/bottom-bar-insets/serve.mjs
node tests/browser/bottom-bar-insets/run.mjs
```

Runner uses preinstalled agent-browser, isolated `bottom-bar-insets` session; emits JSON and exits 1 on assertion failures. Server binds only localhost 5191 and allowlists four files. Stop server with Ctrl+C; `agent-browser --session bottom-bar-insets close` closes browser.

Direct inspection:

```sh
agent-browser --session bottom-bar-insets open http://127.0.0.1:5191/tests/browser/bottom-bar-insets/index.html
agent-browser --session bottom-bar-insets eval 'window.checkBottomBarInsets()' --json
```

Acceptance: dark/light × 24/32.4/48px; unchanged host/bar height; 2px top padding measured after computed bar top border (selected design: 3px outer top with 1px border), 2px bottom inset; 3px intergroup/direct gutters; 4px endcap outer insets; 2px tile corners; fine bottom border; inset faces; no painted active left pseudo-element stripe; active tile retains distinct nontransparent fill and solid accent border; equal/direct/capsule bounded widths; long-title truncation; count containment; disabled/minimized cues; narrow overflow clipping; source-only preview bridge at -1px reaching source tile and matching width, pointer-transparent.

Preview expectation: bridge height must reach actual source top, including inset and group border. Existing one-pixel root mask cannot connect an inset tile. Keep connected-source shadow/attention semantics from existing contract tests.

Focused regression commands (direct local binaries avoid this harness's npm PATH issue):

```sh
node node_modules/typescript/bin/tsc -p tsconfig.test.json
node --test tests/taskbarPreviewContract.test.mjs tests/taskbarUxState.test.mjs tests/taskbarGalleryContract.test.mjs
node C:/dev/jasonshell/node_modules/vitest/vitest.mjs run --config vitest.config.ts tests/components/taskbar-group-menu.test.ts
```

Initial evidence: 128 browser checks, 54 failures on old CSS; 38/38 Node regressions pass. Component baseline: 41/41 pass with uppercase-drive absolute CLI entrypoint above. Relative CLI from this harness's lowercase `c:` cwd failed before collection: `Vitest failed to find the current suite` at `tests/components/setup.ts:15`; both default forks and threads reproduced. `DEBUG=vite:resolve` showed Vitest resolved to uppercase `C:/dev/jasonshell/node_modules/vitest/dist/index.js`, while Node's relative CLI path was lowercase. Matching CLI drive casing to Vite's resolved module path removes duplicate runtime identity. No config/setup/assertion change needed.

Final acceptance after production-complete signal: 221/221 browser checks pass; focused Node regressions 38/38; real-component mocked interaction regressions 41/41. Added actual `task-icon` image children, image loading/vertical fit at all heights, source-only connected edge/attention, frameless group gaps, toast/busy/drop/drag computed cues, static reduced-motion busy cue, keyboard focus on Quick Launch/direct/capsule/attention/Processes. Waits use animation completion and rendering frames, not fixed sleeps. Global app reduced-motion policy sets transitions to 1ms, so drag no-motion assertion permits that existing policy only under reduced motion. Browser static-state checks do not prove drag gestures, focus navigation, native preview compositing, or runtime group selection. Existing focused tests cover their logic/contracts; reduced-motion busy cue retains existing Node protection. Full native E2E skipped: style-only, no journey logic change; live shell/AppBar smoke requires explicit consent.
