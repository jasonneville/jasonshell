# Remaining UI consistency: renderer baseline

Start `node node_modules/vite/bin/vite.js --host 127.0.0.1 --port 5199 --strictPort`. Use an isolated Chromium CDP endpoint on 9295 (Edge setup in `../close-parity/README.md`). Run sequentially:

```text
node tests/browser/ui-audit/escape.mjs
node tests/browser/ui-audit/command-layout.mjs
node tests/browser/ui-audit/inventory.mjs
node tests/browser/ui-audit/menu.mjs
node tests/browser/ui-audit/quick-launch.mjs
node tests/browser/ui-audit/command-nested.mjs
node tests/browser/ui-audit/terminal-toolbar.mjs
```

Escape was RED at baseline. The layout runner is GREEN after the CSS fix; pass a distinct output directory to preserve the RED artifacts. The inventory is observational, not an acceptance pass; JSON records missing IPC, computed paint, bounds and a Tab focus sample. All suites load production Svelte via the real surface loader and replace only IPC. Normal sizes follow `shell_windows.rs` (or representative monitor/gallery dimensions); translucent generally reduces width by 15%. CSS metrics are not a test of actual Windows sizing or composition.

## Evidence and gaps

- `test-results/ui-audit-quick-launch-lifecycle-red/`: Three RED states. A current-nonce close left Beta's menu mounted; reopen with a new nonce retained it; fresh Escape sent zero hides. A stale close did not erase the newer menu.
- `test-results/ui-audit-command-nested/`: The populated menu and confirmation fit at 460 × 420. Escape closed the menu without hiding the panel but left focus on `BODY`, not the originating Edit button (RED). Confirmation Escape kept the entry and restored the Delete button. Pending-input Escape cleared the draft, kept the prompt and input focus, and sent no input or hide IPC; Delete was disabled during the mock run. No executable operation was invoked. Loading/error was not exercised on this probe.
- `test-results/ui-audit-terminal-toolbar/`: Uses the dedicated terminal from the close-parity fixture (serve that config on 5198). At 360 × 520, the focused Open terminal actions menu button spans x=355.52–377.75; the viewport ends at x=360, clipping 17.75 CSS px. All six observed toolbar controls fit at 860, 480, and 320 CSS px. The native code requests 860 logical px but clamps the physical width to the top-bar host width minus two 8 px insets, with only a 1 px floor (`terminal_panel.rs:39–59,88–91`). At 100% scale the formula allows 360 px with a 376 px host; this is source evidence, not an observed Windows host size.

- `test-results/ui-audit-escape/`: inert renderer idle Escape in Commands vs Calendar; each explicit close still sent its original hide once. Native routing and nested modal/reorder/pending-input interactions not established.
- `test-results/ui-audit-command-layout/`: Previous runs receives trusted Tab focus but stays horizontally clipped: dark default tab right 461.95 vs clipping boundary 438.80; light default 452.47 vs 438.80; translucent narrow 452.47 vs 391. This is clipping of an actionable tab, not intentional content divergence.
- `test-results/ui-audit-command-layout-green/`: Both labels fully fit and Space activates Previous runs in all three states. Previous runs now ends at 423.69 vs clipping boundary 438.80 at 460 CSS px, and at 309.88 vs 369.80 at 391 CSS px; the focused tab paints a 2 CSS px ring.
- `test-results/ui-audit-menu/`: Populated production task-window overlay, including disabled actions and a separator. Trusted Tab and arrows traverse enabled rows, and Escape sends exactly one hide request with origin restoration requested. RED: End moves focus to the last enabled action in all states but fails to reveal it in the 320 × 145 CSS px constrained viewport (row y=143.56–173.95, menu bottom y=137). At 360 × 360 it fits (row bottom y=346.95, menu bottom y=351.95). This test proves renderer clipping in an explicitly constrained viewport, not that native placement produces that exact viewport or restores focus.
- `test-results/ui-audit-menu-green/`: All three states pass after the scroll fix. At 320 × 145, End moves the menu scroller 42 CSS px and reveals the focused row at y=101.56–131.95; Home returns scroll to zero and reveals the first row. Both have nontransparent focus paint. The document and fixture root do not scroll; Tab, arrows, disabled skipping, and the single Escape restore request pass.
- `test-results/ui-audit-quick-launch/`: All three states show empty-to-populated rows, ArrowDown selection, ContextMenu opening, and Tab focus on the action. The 320 CSS px menu reaches x=320 without cropping and the action ends at x=315. RED: Escape on the nested menu action sends the panel hide IPC once, leaves the menu mounted, and does not refocus the selected row. No launcher action or elevation executed.
- `test-results/ui-audit-inventory/`: 60 snapshots across 20 labels, zero setup crashes, 36 snapshots with missing mocked commands. Blank or fallback frames are not observed active states.

| Label(s) | Observed baseline and not verified |
|---|---|
| Top bar, bottom bar | Shell chrome; resize IPC missing; native AppBar sizing, launcher arbitration, work area and task interactions not verified. |
| Search | Initial frame; payload missing; centered results/scope interactions not captured in this inventory. |
| Quick Launch | Initial frame; no launch payload; list/context menu and OS launch/elevation not verified. |
| Tray | Empty state; populated grid and physical notification-area relay not verified. |
| Audio | Mocked volume state without devices or sessions; device/session controls and hardware not verified. |
| Calendar, Speech History | Calendar and empty history; history items/copy and native placement not verified. |
| Commands | Empty/default editor; idle Escape and tab clipping separately tested. Reorder, history, input, confirmation and real execution not verified. |
| Stack | Shell frame without popup request; folder grid, editor, Git, terminal and nested menus not verified in inventory. |
| Process Manager | Initial frame without native context; populated processes/System Info and hardware unread. |
| Control Plane, Settings | Default frames; Settings status mocks incomplete; capture, drafts, persistence and submenu branches not verified. |
| Terminal | Hidden/default frame without session open; terminal search covered separately by existing close-parity evidence. Native ConPTY not verified. |
| Task Gallery, Task Preview | Gallery blank without session and preview placeholder; no populated layout or DWM frames/native window actions. |
| Snip Overlay, Snip Preview | Initial/fallback with missing context; no capture, real image, or nonactivation evidence. |
| Speech Indicator | No status payload: inactive/blank, not recording/WebGL or hardware evidence. |
| Context Menu Overlay | Blank without payload; no menu/submenu density, separator, shortcut or focus evidence. |
