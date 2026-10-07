# Isolated gallery close fixture

## Tight-seams browser acceptance

```sh
node node_modules/vite/bin/vite.js --config tests/browser/task-gallery/vite.config.ts
node tests/browser/task-gallery/run-tight-seams.mjs
```

Uses actual mounted `TaskGallerySurface.svelte` and production global CSS with existing Tauri mock aliases, not a restyled facsimile. Isolated agent-browser session `task-gallery-tight-seams`; JSON results; exit 1 if any assertion fails. No installs/native IPC.

Matrix: dark/light × 24/32.4px × normal 960px/3 windows, narrow 240px/3 windows, stress 240px/30 windows. Asserts unchanged panel height; 2px vertical/4px horizontal inset; 3px tile gaps; 2px corners; firm theme border/beveled face; no painted active left stripe; distinct active fill/border; horizontal scroll; 24px close width with vertical containment; icon containment; long-title truncation. Scrollbar stress must not clip close/icon targets vertically. Close height may shrink to actual available height, width stays 24px.

Initial RED: 600 checks, 240 failures on legacy gallery appearance, including clipped many-window controls. Final production rerun: 600/600 pass. At 24px, actual hover exposes a 24×18px close target fully within strip; activation height is 18px. Actual Tab from activation to close + Enter keeps keyboard focus/ring, dispatches one nonce/HWND close with no activation, sets aria-busy/aria-disabled, and preserves tile/close geometry while pending. Focused Node regressions 47/47 and gallery-close/group-menu component regressions 58/58 pass. Optional task-preview component suite independently reproduces an unrelated header-icon failure at line 154 (13/14 pass); no preview source/test modified or assertion weakened. Source contract `tests/taskbarGalleryContract.test.mjs` now checks tile-specific 2px corners and firm 1px theme border rather than legacy square/left-only borders. Existing close, preview, keyboard and mocked IPC tests remain final regression requirements.

Full native E2E skipped for styling-only change; mocked component browser required. Live Windows compositing/WM_CLOSE/shell smoke remains consent-gated.


```powershell
npx vite --config tests/browser/task-gallery/vite.config.ts
```

URL: http://127.0.0.1:4179/. Production component + global CSS; test-only Tauri aliases, no native calls or shell startup. No added dependency.

Repeatable setup via URL: `/?height=24&width=240&theme=light` (validated against the selectable fixture choices).

Controls: native-height simulation 24/32.4px, width 144/240/480/960px, dark/light colors, three windows (long title), deferred close success/failure, same-nonce removal, replacement, hide, unmount. IPC ledger and `window.galleryFixture.measure()` expose control geometry/truncation/focus/busy state. `window.galleryFixture.settle()` accepts pending requests without removing tiles; `.snapshot([rows[0],rows[2]])` alone removes Beta.

Theme selects actual production `data-theme="base-dark"` / `base-light`; no partial fixture token overrides. **Focus activation** and **Focus X** invoke actual first-control DOM `.focus()` and report focus-within/reveal state (not real hardware Tab/Enter/Space). **Snapshot 30 windows** publishes a same-session stress snapshot. Summary reports count, zero-width titles, scroll/client dimensions, focused control, theme, and completely visible close targets; full geometry reports actual target bounds and colors. If the visible panel has not advanced animation frames, Capture then **Measure geometry** resamples settled opacity without modifying CSS.

## Historical stress limit (resolved by production CSS)

The measurements below describe the legacy gallery before Tight-seams styling, not current behavior.

| Snapshot / fixture width / height | Historical production browser result |
|---|---|
| 3 / 144px / 24px | Three 48px tiles; title widths zero, icons/X intact; all X boxes 24×24px and within strip. |
| 3 / 240px / 24px | Three 80px tiles; long title truncates; all three X targets fit. |
| 30 / 144px or 240px / 24px | Existing horizontal scrollbar; scrollWidth 1440px, clientHeight ~14px (tile ~13.6px). All title widths zero; 24px-tall X boxes vertically clipped. Zero fully visible X targets. |
| 30 / 240px / 32.4px | Existing scrollbar; clientHeight 22px; X boxes extend 1px above tiles, vertically clipped. Zero fully visible X targets. |

Production CSS resolved vertical clipping with the new insets, non-space-taking horizontal scrollbars, and close controls fitted to available tile height while retaining 24px width. This is a production fix, not a fixture scrollbar/target/overflow workaround. Current 600/600 browser acceptance verifies vertical close/icon containment for the documented matrix, including 30 windows at 240px width and both heights; horizontally offscreen tiles still require scrolling. These results do not certify native monitor geometry or actual native user reachability. The historical 144px cases above were not rerun as part of the current acceptance matrix.

Original close investigation used a visible browser. Tight-seams acceptance explicitly authorizes agent-browser against this mocked localhost fixture; never start Tauri.

Check 24px right slot and hit target at both heights, title truncation, no layout shift on hover/focus, selected activation→X Tab pair, arrow/Home/End navigation, exact nonce/HWND close without activation, pending duplicate suppression, successful dispatch keeping all tiles, authoritative removal preserving surviving focus/preview, replacement/hide/unmount late settlement. Fixture settings buttons steal DOM focus intentionally; use `window.galleryFixture` in visible-browser eval for focus-preservation samples. Mock hide emits closed; pointer departure may auto-dismiss, so Open resets a fresh session.

This is DOM/layout/IPC evidence only. It cannot certify real HWND identity, WM_CLOSE delivery, save prompts, Win32 focus, UAC absence, or native shell coexistence. Actual destructive cross-native journey remains consent-gated and unauthorized.
