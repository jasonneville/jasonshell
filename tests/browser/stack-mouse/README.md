# Isolated Stack mouse browser evidence

No production entrypoint, new dependency, Tauri process, filesystem mutation, or shell hook. Real popup, Git, confirmation and CodeMirror surfaces mount against an allowlisted deterministic Tauri substitute. Unknown commands throw and enter the ledger.

Start from repository root:

```powershell
npx vite --config tests/browser/stack-mouse/vite.config.ts
wmux browser open http://127.0.0.1:4178
wmux browser snapshot
wmux browser eval "window.stackMouse.run('git')"
wmux browser snapshot
wmux browser eval "window.stackMouse.run('editor')"
wmux browser snapshot
```

Alternatively, with explicit user consent for a visible browser, prefer the visible `wmux browser` panel when available. If OpenChamber animation frames stall, use the separately installed headed agent-browser session (no new dependency):

```powershell
agent-browser --session jasonshell-viewport --headed open http://127.0.0.1:4178
```

Then click **Run Git multi-layer journey**, **Run dirty editor journey**, or **Run large directory viewport journey** in the visible browser. Do not hide the page, substitute timers for animation frames, or treat a frame stall as a pass; report the journey as failed/browser-blocked. The parent session owns this browser execution; component-test execution must not duplicate it or imply it passed. Each journey resets bridge state and remounts the real popup. PASS requires pointerdown ownership, inert compatibility mousedown/auxclick, single-layer unwind, unchanged folder path/read ledger, and no hide/save/destructive calls. Git journey: confirmation Forward inert -> Back cancels -> branch picker -> Changes diff -> Git panel -> original folder. Editor journey uses actual CodeMirror text insertion -> dirty Back prompt -> Back cancels preserving draft -> Back prompt -> Discard closes editor only.

`window.stackMouse.events` records browser capture/bubble/dispatch-return ordering and visible layer state; `ledger` records bridge commands. Synthetic browser PointerEvents establish DOM routing, not hardware XBUTTON delivery, trusted-event generation, or browser-native history suppression. Those need explicit consent-gated hardware/native smoke.

Fixture compilation alone: `npx vite build --config tests/browser/stack-mouse/vite.config.ts` (output in ignored node_modules cache).

## Large-directory viewport regression

The fixture imports the production `src/app.css` global font, theme-variable, box-sizing and document-height defaults, as the real entrypoint does. Its test-only grid host bounds the popup to at most 600px and keeps the result ledger below it in a separate, at-most-120px scroller. No folder row or production popup styles are overridden. Verify `.details-body` has positive finite `clientHeight` below host height and `scrollHeight > clientHeight`; the journey asserts these and actual row-height agreement with the production virtualization constant.

`Running` appears immediately. Real animation frames remain required, with a 3s diagnostic timeout; no timer substitutes for layout/focus proof. The previously observed visible-panel frame timeout is not proven fixed by host sizing. If it persists after reload, report the browser/compositor limitation and FAIL; build/component passes do not establish browser E2E.

Click **Run large directory viewport journey** in the visible panel, or invoke `window.stackMouse.run('viewport')` through the authorized visible browser. It resets to a deterministic 400-file directory, scrolls near the bottom, selects/opens Markdown Preview, and returns with mouse Back. It then repeats through actual CodeMirror dirty editing, Cancel, and Discard, and finally captures a different latest folder offset for a repeated open. `#result` must say `viewport: PASS`; `window.stackMouse.viewports` records pre-open and post-return offsets, real body height, rendered row count and actual row visibility. Expected: same folder offset within 1px after focus, same selected file visibly inside the body, virtualization active, no additional folder reads or native save/hide/destructive calls. Editor scroll must not replace folder scroll.

Component cases mock body geometry explicitly (30px production row height, 240px viewport, 400 rows) and assert DOM scroll/selection/virtual-window state; they do not establish real browser layout. Browser journey supplies that separate layout/remount/focus evidence. This is only in-memory same-directory restoration across layer replacements, not durable persistence or cross-folder history scroll memory.

Component coverage includes rapid physical-press/compatibility sequences, delayed Stash detail/diff responses, busy Git ignore confirmation, cancellation focus restoration, destructive-call exclusion, ordinary click/keyboard behavior, and popup unmount/remount cleanup. Busy ignore starts exactly one explicitly confirmed bridge operation; side buttons must add no commands while it remains pending.

Terminal limitation: current popup imports `StackTerminalPane` and keeps terminal-mode functions/state, but renders neither a terminal pane nor a control invoking its mode switch. No test-only production hook is added. Cleanup tests use an isolated terminal input target after popup unmount; this establishes no leaked capture listener, **not** live mounted-terminal integration or native terminal behavior.
