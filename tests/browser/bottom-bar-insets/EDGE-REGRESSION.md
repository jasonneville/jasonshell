# Bottom-edge target regression

## Safe reproduction

No install/new dependencies; no Tauri/native shell. Terminal one:

```powershell
node node_modules/vite/bin/vite.js --config tests/browser/bottom-bar-insets/edge.vite.config.ts
```

Before production edits, terminal two:

```powershell
node tests/browser/bottom-bar-insets/run-edge.mjs --baseline
```

After production edits:

```powershell
node tests/browser/bottom-bar-insets/run-edge.mjs --compare
```

`--artifacts <directory>` and `--session <name>` isolate independent runs. The
sealed pre-change acceptance run uses `--artifacts test-results/bottom-edge-final
--session bottom-edge-final`; use that same artifact directory for its compare.
`--states-only` omits the configuration/overflow matrices and runs the positive
face controls plus stateful regression tail. The corrected pre-change state run
uses `--baseline --states-only --artifacts test-results/bottom-edge-states-corrected
--session bottom-edge-states-corrected`.
The final state-oracle refinement uses `test-results/bottom-edge-states-final/`.
`--overflow-only` isolates the six real-overflow cells. Its corrected baseline is
`test-results/bottom-edge-overflow-corrected/`; it first resizes 321→320px through
the browser so production updates the native-host resize/overflow state.

Preserve `test-results/bottom-edge/baseline.json` and `baseline-*.png` before
the rerun; `--baseline` deliberately replaces them. `--compare` writes
`current.json` and `current-*.png`, checks geometry against that baseline,
and exits nonzero for any failed acceptance assertion. Both reports contain
per-assertion diagnostic IPC traces. Do not treat the intentionally RED
baseline as product acceptance.

`AGENT_BROWSER_EXECUTABLE` optionally overrides the existing Windows binary.
The runner uses an isolated browser session and integer mouse coordinates
(the installed CLI rejects fractional coordinates). Browser viewport is
600px high and the native-client host is fixed flush to its bottom.

## Contract and evidence

The fixture imports production global CSS and mounts **production BottomBar**,
including native MeltActionButton buttons and OverflowTicker. Only the native
IPC boundary is mocked. Unexpected commands reject rather than silently succeed.
Synthetic rows include active/attention direct tasks and a busy five-window
group with a minimized member. The dedicated busy-cue state changes the synthetic
group process to `Windows Terminal`, because production intentionally filters
activity indicators by eligible process/title metadata. No static recreation is
used for action claims.

The configuration matrix is 24/32.4/48 logical px × dark/light × 960/320px.
Each configuration checks Quick Launch, direct window, grouped capsule and
Process Manager at both horizontal ends and center of each face, using
trusted mouse down/up at `viewportHeight - 1`, plus native hover state.
Face-center positive controls first prove every mocked IPC action is reachable
through its actual production handler, preventing a broken mock from masquerading
as bottom-edge RED.
The same native button must own the extension: a separate forwarding control
or global coordinate action router does not satisfy the hit assertion.
An additional 90-group snapshot establishes **actual** scroll-width overflow
at each height/theme, checks its status, records separate screenshots/geometry,
and verifies clipping cannot steal Quick Launch/Process Manager edge actions or
leak into the right outer margin. Ordinary narrow reflow is not called overflow.

Direct/group edge hover must reach preview/gallery IPC; face↔extension crossing
must preserve its session. Direct/group context-menu requests must emit once.
Outer margins, inter-control gutters and blank task-strip space stay inert.
Stateful checks cover edge reorder drag suppression, Escape cancellation and
fresh-press recovery, disabled pending activation, keyboard Enter/Space,
toolbar Home/End, focus/hover ticker continuity, reduced motion, and top-only
resize admission/persistence. Read JSON for assertion-level outcomes; an
aborted runner is never complete coverage.

Visible geometry and hit geometry are deliberately separate: the native
button's existing rect is the visual face, still ending **2px** above viewport
bottom. The baseline records face/child positions and bar dimensions; compare
requires these unchanged. Transparent native-button hit extension is expected
to change hit ownership without moving icon/title/count/glyph, painting into
the inset, changing horizontal positions, or changing bar height. Screenshots
are retained for independent visual review; JSON equality alone does not prove
unchanged shadows/colors. No visual pixel-diff claim is made by this runner.

This establishes Chromium DOM/layout/input and production renderer handlers
under mocked IPC. It does not establish native HWND, WebView2, AppBar ownership,
native windows/actions, DWM previews, physical DPI/hardware or screen-reader
delivery. Native shell smoke requires separate human consent.

## Pre-change identity and focused gates

SHA-256 before production editing:

- `BottomBar.svelte`: `6ABC26362AD4D5F7995B25D80884BC800FB7ADC452AA9C5A65337B9813746B7E`
- `BottomBar.css`: `5E721EE31A0D6CA3D10EAA19B9C347E99FEFE79DA065FD535D29C4C8D03841CC`
- `melt/MeltActionButton.svelte`: `E93DBC920621060FDA24441B0092DDB219F3EDF95203831F0AE93334C6D46520`

Fresh focused baseline (not the wider suite):

```powershell
node node_modules/typescript/bin/tsc -p tsconfig.test.json
node --test tests/taskbarPreviewContract.test.mjs tests/taskbarUxState.test.mjs tests/taskbarGalleryContract.test.mjs
# 38 passed, 0 failed
npm run test:component -- --run tests/components/taskbar-group-menu.test.ts tests/components/overflow-ticker.test.ts
# 51 passed, 0 failed
node --check tests/browser/bottom-bar-insets/run-edge.mjs
npx --no-install tsc --noEmit --skipLibCheck --target es2022 --module esnext --moduleResolution bundler tests/browser/bottom-bar-insets/edge-bridge.ts
# both exit 0
```

The first direct Node attempt failed at module loading (three missing
`dist-tests` entrypoints); compiling `tsconfig.test.json` first resolved that
setup problem. No full Node/component/Rust suite or native smoke was run.

Independent trusted-pointer positive/negative control, 24px/dark/960×600:
Quick Launch face `(49,590)` emitted one `show_quick_launch_panel`; pointer-down
was trusted and targeted the native Quick Launch button. Same x at `(49,599)`
emitted zero IPC; trusted pointer-down targeted `.surface.bottom-bar`. This
refutes edge-target acceptance without relying on static CSS assertions.

The exploratory full-matrix run in `test-results/bottom-edge` completed 407
assertions (137 pass, 270 RED). All configured face/bar geometry passed;
bottom-edge native hits/actions/hover/menu requests failed. This is not the
extended acceptance artifact directory.

The extended raw run in `test-results/bottom-edge-final` completed **584 checks:
148 pass, 436 RED**, with all 18 geometry records and 18 screenshots retained.
Its 12 ordinary configuration cells passed every bar-height/2px-face geometry
assertion. Raw non-edge observations superseded by the targeted refinements below
are not being counted as new production defects.

The corrected six-cell overflow run completed **24 checks: 12 pass, 12 RED**.
All six have true overflow (`scrollWidth > clientWidth`), active overflow status,
and an inert right outer margin. Only the 12 expected Quick Launch/Process Manager
bottom-edge actions failed. Use these six corrected baseline screenshots, rather
than the raw extended run's pre-resize overflow screenshots, for visual review.

Harness findings were corrected after launching the extended baseline:
the unlocked resize handle begins inside the existing 1px top border (the old
oracle incorrectly expected the outer border edge), and keyboard acceptance
must fresh-mount after the deliberately deferred disabled-activation stress
case rather than inherit that case's suppression state. Each independent key
activation explicitly refocuses: direct activation temporarily disables its
native button, which can move browser focus. Busy-cue coverage uses eligible
terminal metadata rather than the intentionally ineligible generic editor.
The raw extended
baseline retains those observations; the separate corrected state run
supersedes them. Neither is being claimed as a new production defect.

Gallery action counting distinguishes native-session retirement from activation:
production `openTaskGallery` may issue one retirement hide before one focused
show. That is one user activation, not two. The final oracle rejects multiple
focused shows or multiple hides, while permitting that one cleanup hide alongside
a focused show. Raw traces retain the earlier overstrict sum-of-IPC observation;
no production double-toggle defect or required handler expansion is claimed.

## Sealed handoff

Final corrected state-only run in `test-results/bottom-edge-states-final`:
**34 checks, 29 pass, 5 expected RED**. The remaining failures are the minimized
group's edge hit, direct/group fresh edge presses after cancellation, edge-started
ticker motion, and face↔extension ticker continuity. All four independent face
positive controls pass, as do busy/attention cues, disabled duplicate protection,
Enter/Space, Home/End, focus/reduced-motion checks, and top-only resize checks.
No runner abort occurred. Production files were not edited during evidence work.

Baseline is sealed; scoped production implementation may now begin. Do not rerun
`--baseline` against changed production. The native button must remain the hit
owner, with unchanged visible face/content coordinates, 2px bottom gap, horizontal
gaps/endcaps, bar height and chrome. Preserve horizontal clipping, native disabled
semantics, existing menus/gestures, ticker engagement and the top-only resize area.

Post-fix full comparison (the existing localhost server may be reused):

```powershell
node node_modules/vite/bin/vite.js --config tests/browser/bottom-bar-insets/edge.vite.config.ts
node tests/browser/bottom-bar-insets/run-edge.mjs --compare --artifacts test-results/bottom-edge-final --session bottom-edge-postfix
```

Require every current assertion to pass; review current screenshots against the
12 normal sealed baseline images plus the six corrected overflow baseline images.
Then repeat the focused Node/component gates above. Independent test/code review
remains parent-owned: nested reviewer delegation was blocked by the agent depth
limit. No full-suite or native-shell certification is claimed.

## First post-fix acceptance

Production-mounted full compare completed **604 checks: 603 pass, 1 failure**.
All native edge hit/hover/action matrix, ordinary/overflow geometry comparisons,
menus, ticker continuity, disabled, keyboard, focus, reduced motion and resize
assertions pass except the grouped cancellation sequence below.

**Concrete behavior defect:** `group: Escape cancels edge drag without activation`.
After a held primary pointer on the grouped bottom-edge target moves 30px, Escape
cancels the drag; releasing the pointer nevertheless emits one
`show_task_gallery` with `args.focusGallery: true`. Expected zero gallery action.
The archived trace is
`test-results/bottom-edge-final/first-postfix-current.json`, `failed[0]`.
This is a focused activation after cancellation, not a retirement hide accompanying
an allowed activation. No production fix or weakened oracle was applied here.

Fresh focused post-fix gates: **38/38 Node**, **51/51 component**. The existing
static inset suite passes **233/233** via the direct native-binary launcher below;
its assertions are unchanged. The original shell-shim runner and CLI doctor calls
timed out, so the replacement launcher bounds each native CLI call to 45 seconds
and records `existing-insets-current.json` rather than changing acceptance logic.

```powershell
node tests/browser/bottom-bar-insets/run-existing-insets.mjs
node tests/browser/bottom-bar-insets/compare-edge-pixels.mjs
```

Exact screenshot inspection compares the 12 ordinary baseline images and six
corrected-overflow baseline images to the corresponding current images, including
the full bar/bottom inset and 8px above the bar for chrome/shadow. Result:
**6/18 byte-pixel-equal RGB samples; 12 ordinary samples differ**. The ordinary
wide samples have 38/54/86 changed pixels (24/32.4/48px); narrow samples have
157/165/181. Largest RGB channel delta is 39. All six corrected-overflow samples
are identical. Archived pixel report:
`test-results/bottom-edge-final/first-postfix-pixel-comparison.json`.

Differences are localized to task-group edge clipping/chrome, not moved content:
at 48px/light/960 only columns x=257 and x=420 differ over y=555…597; at
48px/light/320 differences occupy x=122…191 near the top edge and right border.
For these inspected examples no pixel in the 2px bottom gap differs. The unchanged
geometry assertions do not establish identical border/shadow paint; that visual
invariant remains failed, not silently relaxed. Baseline/current screenshots were
also inspected directly. Return these findings to the implementation owner.

`BottomBar.svelte` and `MeltActionButton.svelte` hashes still match the pre-change
identity above. No production file was edited by the test executor. No native
Tauri/Windows shell journey or broader suite certification is claimed.

## Final corrected acceptance

After the implementation owner corrected capsule cancellation suppression and
horizontal/vertical clipping, the original full compare passed **604/604**.
The already-existing trusted-pointer cancellation regression now observes zero
focused gallery opens after Escape/release, then exactly one action on the next
fresh press. No regression assertion or visual invariant was weakened.

The exact screenshot comparison passed **18/18**, with **zero differing RGB
pixels** in every inspected bar/chrome region: all 12 ordinary
24/32.4/48 × dark/light × wide/narrow samples plus all six corrected-overflow
samples. This includes the unchanged 2px bottom inset and an 8px top shadow
allowance. It establishes browser screenshot equality, not native compositing.

Fresh final gates:

| Gate | Result |
|---|---|
| Production-mounted browser compare | 604/604 |
| Exact bar/chrome screenshot comparison | 18/18, zero differing pixels |
| Focused Node contracts | 38/38 |
| Focused menu/ticker component tests | 51/51 |
| Existing static inset assertions | 233/233 |
| Three browser runner/script syntax checks | exit 0 |

Validated production SHA-256:

- `BottomBar.css`: `F4EC789BB8BFD8E66F3660A46E3416F348C9E570701F9CD91888E110171B1001`
- `BottomBar.svelte`: `A75DD2055E8F4B3FB9BD5BDB56F31491FB514FB084A2632D0BCAC1FF44149859`
- `melt/MeltActionButton.svelte`: `E93DBC920621060FDA24441B0092DDB219F3EDF95203831F0AE93334C6D46520`

Definitive artifacts under `test-results/bottom-edge-final/`:

- `current.json`: all 604 behavior/geometry results and diagnostic IPC traces.
- `pixel-comparison.json`: all 18 exact screenshot comparisons.
- `existing-insets-current.json`: all 233 unchanged static inset assertions.
- `current-*.png`: all 18 current screenshots.
- `first-postfix-current.json`, `first-postfix-pixel-comparison.json`: archived
  failed iteration, retained separately from the final reports.

Original sealed baseline JSON/screenshots, corrected overflow baseline and final
pre-change state baseline were preserved. Test executor made no production edits.
Evidence is ready for the parent's independent review. Native Tauri/Windows shell
smoke remains consent-gated and was not run; no wider-suite certification is made.
