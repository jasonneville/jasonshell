# Speech theme cross-surface browser evidence

Run `npx vite --config tests/speech-theme-e2e/vite.config.ts` from repo root;
open `http://127.0.0.1:1447/tests/speech-theme-e2e/index.html` in visible browser.

This mounts **real** `main.ts` / `App.svelte`, Settings and indicator in separate
same-origin frames. Only native Tauri APIs are aliased. Theme selection clicks the
actual Settings Melt control; theme synchronization uses the real browser
BroadcastChannel and storage listeners. No synthetic theme event. Mic input is
only injected by explicit volume controls and the labeled contrast fixtures.
The isolated 1447 origin's theme preference is seeded to Dracula; no native launch.

1. Initial: both frames Dracula, GPU equals resolved `--js-color-accent` RGB.
2. Choose Monokai: actual Settings and indicator root become Monokai; GPU green.
3. Pause, choose Andromeda: new accent adopted, mic level stays zero.
4. Reload reduced-motion indicator; choose Dracula: accent updates without motion.
5. Reload GPU-unavailable indicator; choose Monokai then Andromeda: background
   changes with the theme, stays fully opaque; icon readable and 40px shell fixed.
6. Reload normal indicator: stored current theme hydrated before first frame.

Capture visible readout + both frames for initial/live states. Browser GPU and
cross-frame synchronization are covered; native WebView/Settings hardware is not.
Unit tests separately assert RAF gates, observer cleanup and context recovery.
Stop the dedicated Vite process after evidence collection. This test configuration
is not used by product build and introduces no dependencies.

The panel reports `document.hidden=true` even for an on-screen indicator frame.
Indicator frames therefore explicitly emulate visible documents (`visible=1`);
this is disclosed in the page/readout. Without it, the production hidden gate
correctly defers GPU redraw. Hidden/resume behavior is separately unit-tested.
Reduced motion and unavailable GPU are also explicit test emulations, not native
OS preference/hardware evidence. Contrast fixtures send native-boundary voice
packets at 0.05, 0.5 and 1 every 50ms, preserving real meter/hold behavior.

## GREEN evidence — 2026-10-04

Independent rerun: 68/68 focused component/renderer tests, 27/27 Node contracts,
focused strict typecheck and harness JavaScript syntax checks passed.
Visible browser journey passed with visibility caveat above:
- Initial Dracula: GPU `[189,147,249]/255`, matching resolved CSS accent.
- Real Settings → Monokai: live GPU `[166,226,46]/255`; no mic change required.
- Paused capture → Andromeda: GPU `[4,217,196]/255`, mic zero.
- Emulated reduced motion → Dracula and Monokai: theme adopted in static frame.
- Unavailable GPU → Monokai then Andromeda: opaque fallback changed from
  `rgb(166,226,46)` to `rgb(4,217,196)` with canvas hidden.
- Normal indicator reload retained stored Andromeda and its GPU accent.
- Real Settings → Base Light succeeded: root `base-light`, accent `#2563eb`,
  GPU `[37,99,235]/255`. Low/mid/full packets produced 0.05/0.5/1 fill.
- Geometry stayed 40px shell / 24px glyph; shell border 0px, outline/shadow none.

Reader judgment: low/mid/full mic silhouettes remain identifiable on Dracula,
Monokai and Base Light. Full light alloy is visually less distinct against bright
orb highlights than low fill; the internal dark keyline retains the silhouette.
Designer should review this tonal subtlety; no missing glyph or ring regression
observed. This is screenshot judgment, not a formal pixel contrast certification.

Final captures (actual-size widgets, browser screenshot scale 1.25):
- `.openchamber/screenshots/speech-theme-dracula-levels-green-2026-10-04T06-08-47-272.jpg`
- `.openchamber/screenshots/speech-theme-monokai-levels-green-2026-10-04T06-09-01-365.jpg`
- `.openchamber/screenshots/speech-theme-base-light-levels-green-2026-10-04T06-10-41-961.jpg`
- `.openchamber/screenshots/speech-theme-fallback-andromeda-green-2026-10-04T06-10-15-828.jpg`

Owned server stopped after final verification. Native WebView/hardware smoke not run.

## Presentation smoothing: deterministic browser frame sampling

Reuse the same Vite command. Open
`http://127.0.0.1:1447/tests/speech-theme-e2e/index.html?sampling=1` in the visible
OpenChamber browser. This is a localized presentation sanity check, not a new
Settings integration journey or uninterrupted animation playback claim.

The opt-in `sampleclock=1` indicator frame installs emulated `performance.now`,
queued RAF, visibility and dynamic reduced motion **before** importing `main.ts`.
Each step advances the manual clock and delivers only callbacks queued at its
start. Newly queued callbacks wait for the next step; cancellation is respected.
Real Svelte ticks flush the actual `--mic-level` binding before the readout updates.
The mic, original SVG mask, alloy gradient and orb are production components.
Native voice packets reuse the existing boundary; no smoother implementation is
duplicated in the harness. Real native-boundary packet/hold timers remain real
time; this sampling affordance does NOT establish exact hold timing (unit tests do).

1. Click **Run sampled assertions**. Latest readout must say PASS. Assertions
   cover monotonic rise/fall, bounded finite samples, exact endpoints, gentler fall,
   interruption without jumps, dynamic reduced-motion snap, zero queued RAF when
   hidden, resumed convergence and fixed ringless 40px shell/24px glyph.
2. Click **Freeze rise at 32ms**, **64ms**, then **endpoint**. Each resets the real
   session, settles raw 0.05 (display 0.1), targets raw 1 and advances 16ms steps.
3. After 32ms capture, click **Raw 0.2 → display 0.4**: displayed level stays unchanged
   without a frame. **Step 32ms** should produce an intermediate falling value.
4. Manual target, 16/32ms step, reduced-motion and hidden/visible controls remain
   available. All emulation is disclosed on the page and in the readout.

### Executed GREEN sampling — 2026-10-04 (historical, BEFORE double sensitivity)

Independent focused rerun: **82/82** component tests and **27/27** Node tests pass.
Strict helper/test typecheck and harness syntax checks pass. Browser assertions
passed against the actual surface DOM; all sampled scalars finite and within 0–1.

| Sample | Actual rendered level |
| --- | --- |
| 0.05 → 1, no frame | 0.05 |
| Rise 16ms | 0.32968 |
| Rise 32ms | 0.56072 |
| Rise 64ms | 0.87688 |
| Rise 112ms | exactly 1 |
| 1 → 0.05, fall 32ms | 0.6922469135802469 |
| Fall 192ms | exactly 0.05 |
| Retarget 0.2 at displayed 0.56072, no frame | unchanged 0.56072 |
| Interrupted fall 32ms | 0.44386453333333337 |

At 32ms, normalized rise progress is 0.5376 versus fall progress 0.3239506173:
fall is demonstrably gentler. Dynamic reduced motion snapped to 0.2 and subsequent
target 1 immediately. Hidden transition cancelled all RAF (orb + fill); target
changes and clock steps while hidden did not paint or schedule callbacks. Visible
resume converged exactly to 1. Geometry remained 40/24px, border 0px, shadow none.

Actual-size frozen-frame captures for designer visual judgment:
- 32ms: `.openchamber/screenshots/speech-fill-sampled-rise32-2026-10-04T15-15-06-529.jpg`
- 64ms: `.openchamber/screenshots/speech-fill-sampled-rise64-2026-10-04T15-15-55-668.jpg`
- Endpoint: `.openchamber/screenshots/speech-fill-sampled-endpoint-2026-10-04T15-16-11-533.jpg`
- Interrupted fall32: `.openchamber/screenshots/speech-fill-sampled-interrupt32-2026-10-04T15-15-38-698.jpg`

Screenshot scale is browser 1.25; actual widget dimensions are in the readout.
These establish intermediate gradient rendering, not uninterrupted real-time
playback smoothness, OS preference behavior, native WebView or audio hardware.
Stop only the owned dedicated Vite process after sampling.

## Double presentation sensitivity — RED acceptance handoff

Current acceptance maps raw audible levels to display targets before interpolation:
0.035 → 0.07, 0.1 → 0.2, 0.25 → 0.5, and raw >= 0.5 → 1. The raw meter,
0.035 speech threshold and 300ms hold remain unchanged; gain must NOT promote
raw 0.02/0.034 noise to speech. Zero/invalid input cannot activate or extend hold.
The generic fill helper continues to accept display targets unchanged.

Retained sampling assertions are recalibrated: raw 0.05 settles display 0.1;
raw 0.2 interruption/reduced-motion target becomes display 0.4. Theme contrast
fixtures still inject raw 0.05/0.5/1 (display 0.1/1/1). Historical screenshots and
tables above describe the former 1:1 scale, NOT current sensitivity evidence.
The prior animation mechanism evidence remains useful; its old numerical values
must not be treated as post-gain measurements. This scale-only request uses
deterministic actual-component rendered-value tests; no new browser/native journey.

Pre-implementation RED: 70 passed / 24 expected failures (94 focused component
tests). Node meter/native/presentation contracts remain 27/27 green. Strict helper/
test typecheck and harness syntax checks pass. New cases cover mapping and
saturation without changing generic smoothing, noise rejection or timing/reset
contracts. No production edits or browser/native execution in this handoff.

Focused commands:
```powershell
npm run test:component -- tests/components/speech-indicator.test.ts tests/components/speech-indicator-orb.test.ts
node --test tests/speechIndicator.test.mjs tests/speechIndicatorMeter.test.mjs
npx tsc --noEmit --target ES2022 --module ESNext --moduleResolution Bundler --strict --esModuleInterop --skipLibCheck --types node,vite/client src/lib/speechIndicatorFill.ts tests/components/speech-indicator.test.ts tests/components/speech-indicator-orb.test.ts tests/speech-theme-e2e/vite.config.ts
```

## RED evidence — 2026-10-04

Executed in visible OpenChamber browser before implementation. Initial Dracula,
real Settings → Monokai while recording, Andromeda while paused, and Dracula
with emulated reduced motion all propagated the selected theme and resolved CSS
accent into the indicator document, but GPU input remained `[26,115,242]/255`.
No microphone input was dispatched; fill remained zero and shell remained 40px.
GPU-unavailable Dracula → Monokai kept `rgb(16, 33, 58)` instead of updating the
opaque fallback. No console errors after completing the IPC boundary exports.

Captures:
- `.openchamber/screenshots/speech-theme-live-monokai-red-2026-10-04T05-48-17-260.jpg`
- `.openchamber/screenshots/speech-theme-fallback-monokai-red-2026-10-04T05-50-17-241.jpg`

Dedicated Vite process stopped after evidence collection. Re-run command above
after implementation, following all six steps; RED captures are not GREEN proof.
