# M0 source/version/font/cache freeze — 2026-10-07

Approved track: safe interactive M0–M1 fixture only. The broader plan remains a
future milestone reference, not authorization to reserve work area or act on OS tasks.

## Versions and reproducibility

Rust 1.97.1 MSVC / Cargo 1.97.1 observed. Direct runtime egui =0.32.3,
eframe =0.32.3; AccessKit + default fonts + wgpu, no eframe persistence.
Windows production wgpu =25.0.2 additionally enables **dx12** explicitly with
default features disabled; native options explicitly select Renderer::Wgpu.
This corrects the original empty backend feature graph without enabling glow,
Noop or test-only backends. Other target platforms/backends remain unvalidated.
Locked wgpu 25.0.2, egui-wgpu/egui-winit/epaint/emath/ecolor 0.32.3,
winit 0.30.13, AccessKit 0.19.0, accesskit_windows 0.27.0.
Dev-only image =0.25.10 (PNG) supports offscreen evidence; no runtime image I/O.
Cargo.lock is the full dependency freeze. Context7 official egui native App,
AccessKit and widget-info references were consulted; its index returned latest
examples, so pinned downloaded 0.32.3 definitions verified WidgetType (not latest
Role), Sense, Response, Area and ScrollArea signatures before compilation.

## Source mapping

`tools/freeze-tokens.mjs` reads only the five named source references and emits
SHA-256 + all 34 themes' CSS declarations with inherited `var(...)` aliases
resolved. `evidence/tokens-m0.json` is the exact source declaration mapping, not
guessed swatches from themes.ts. Notably base-dark's actual CSS accent is
**#c0d8e7**, not the inventory swatch #8aa7ff; some light/editor themes inherit
base-dark control-hover/border/overlay values. Those inheritances are preserved.

Renderer mapping in `src/theme.rs`:

- bg/text/strong/accent: source sRGB hex; muted: source RGBA alpha rounded to 8-bit
  and composited over bar; bar: raised surface composited over source bg.
- idle task face: sRGB bar/text 93/7; active: bar/accent 76/24.
- hover: source control-hover composited over bar; borders: source border
  composited over bar. Overlay is 8-bit source RGBA drawn as top inset highlight.
- Default's sunken preview placeholders use source sunken RGBA over bar and
  explicitly say unavailable. Showcase paints labelled simulated app illustrations
  directly in egui; no textures, captures or thumbnail destinations exist.
- 2px tile/control corners, 4px horizontal / 2px vertical bar padding, 3px gaps,
  160px cap, 32.4 default, 24–120 height bounds. Gallery nominal 180×136 card,
  4px thumbnail inset / 172×96, 8px gaps/insets, 24px header, 24×24 Close,
  Matrix 12px rail allocation. Preview nominal 332×228 with (12,48,12) content inset.

## Declared visual deviations (not hidden parity claims)

Shipping sans stack begins local Open Sans WOFF2, then Segoe UI Variable Text /
Segoe UI. egui cannot directly use those WOFF2 assets. M1 uses pinned egui bundled
Ubuntu-Light proportional, NotoEmoji-Regular + emoji-icon-font fallback, and Hack
monospace for the diagnostic ledger. No remote fonts, system-font reads or downloads.
This changes glyph metrics/weights (source 600–799 not equivalent), rasterization
and line height. Local controls explicitly use roughly 10px labels; rem values are
source-derived nominal 16px conversions, **browser root-font unobserved**.

Opaque ordinary-window compositing replaces transparent native-helper composition;
8-bit sRGB alpha/rounding replaces CSS float compositing. CSS gradients, raised
blurred shadows, Nord text shadow, bevel richness and 140ms face transitions are
not reproduced. Generic outlined icons (showcase B/T/E initials) replace unavailable application icons.
Eligible busy activity uses a static strip (no 1.15s CSS glow cycle). Idle clipped
titles clip rather than paint a synthesized ellipsis; full semantic label retained.
Demo-host controls intentionally use native egui widgets outside the comparison bar.
No claim that every theme passes contrast or that raster/geometry is within 1px of
the live shipping renderer. Remaining all-theme/DPI/font/height parity audit deferred.

## Resource/timing freeze

Decoded-image capacity constant **4,194,304 bytes**: enough for bounded future
small icon/backup fixtures; actual M1 allocated image-cache bytes **0**. No fake
cache contents/counts. Font atlas belongs to egui, not this decoded-image cache.
No DWM/native destination allocated (0, therefore <=12). Ledger latest **256**
entries; drawable tickers **64** entries maximum, keyed by stable widget identity
with text signatures, separate measured overflow/start time per label, oldest-entry
eviction and unseen-label retirement each frame; pending snapshot **1**, highest
sequence only. 1000-window snapshots and
100 gallery cycles tested. Authoritative tasks are in-memory current snapshot.
No sleeping/polling worker, subscriptions, native handles, persistence, or real
action adapter. Time is absolute monotonic fake/input clock. UI requests deadline
repaints only for pending hover/hide/ticker work; settled static frame does not
request a timer loop. No measured 60s native idle/wakeup claim.

## Observed validation / limits

Current `cargo test --locked ... --tests`: **24 contracts + 12 prior headless tests +
13 showcase tests + 1 Windows renderer backend regression pass** (50); opt-in GPU test ignored by
default, explicitly executed separately and passed. Headless and showcase suites
passed five consecutive focused repeats (125/125 executions, including model tests).
`cargo check --locked`: pass. `cargo fmt --check`: pass after formatting.
All-target strict Clippy: pass. Historical first implementation had 23 contracts +
2 journeys and a runner constant-assertion lint blocker; runner resolved that lint.
Independent review subsequently produced six RED drawable regressions, now green
after production gesture/identity/keyboard/ticker fixes. Original RED and interim
results remain in runner-owned ACCEPTANCE.md. No production-owner test edits.

### Observed Windows renderer startup correction — 2026-10-08

Context7 wgpu v25.0.2 consulted; exact downloaded Cargo features were definitive:
eframe's `wgpu` feature enables dependencies, while its disabled defaults normally
enable `egui-wgpu?/default`; wgpu's `dx12` feature activates its Windows backend.
Production (not dev) target dependency now unifies that feature into eframe's
actual wgpu renderer. Core/eframe/egui pins unchanged; lockfile adds DX12 support
dependencies including gpu-allocator 0.27.0 and Windows 0.58.0 bindings.

After build, both following commands ran with external tool process timeouts of
60s and exited 0:

```powershell
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test renderer_startup -- --ignored --nocapture
cargo run --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml -- --smoke-first-frame
```

Real adapter: AMD Radeon RX 6950 XT / DiscreteGpu / DX12 / driver
32.0.21045.5002. Instance, adapter, device and command submission succeeded.
Ordinary native smoke checked initialized wgpu render state, executed three
actual populated showcase FixtureApp draws (three task groups) from native App updates, printed its marker, requested
Close on its own viewport and exited cleanly. Finite smoke-only repaint requests;
no automatic close/repaint-loop change to normal launches. No shell services,
process/pin actions, AppBars, Explorer changes or Tauri startup involved.
No presented-pixel/completion fence, desktop screenshot, physical-input,
Narrator, benchmark or measured saving claimed from these startup checks.

Inspected generated CPU offscreen meshes at 1280/100% base-dark Matrix,
320/125% base-light constrained Matrix, 1920/200% Nord Ribbon. These show actual
egui geometry, typography, placeholder/caption/Close composition, separated demo
controls and visible Simulated labeling. CPU raster is nearest atlas sampling,
not GPU antialiasing/desktop capture. Final rerenders inspected against current
source: narrow light theme controls readable; two visible Matrix rows and Close
targets retained, no bar overlap; default/wide grouping cues and capped task face.

Hardware/OS GPU-driver matched baseline, 150% sample, all-theme contrast audit,
min/max-height visual audit, release memory/frame/input/startup benchmarks and
native accessibility inspection remain deferred. Browser/wmux unavailable on PATH;
no ordinary desktop screenshot or Narrator inspection claimed. Live Windows shell,
Tauri smoke and connected Process Manager journeys not run/authorized.
No performance/memory saving claim. Source coverage unmeasured; tool unavailable.
Independent final safety/code review remains Leader-owned.

## Populated demo extension

Normal app and finite native smoke now use `FixtureApp::showcase()` (five task
windows in three groups, three fake pins). `Default` stays empty with unavailable
imagery/process diagnostics for original contracts. Showcase fake Process Manager
has four initial rows, CPU-descending table, case-insensitive bounded filter,
manual deterministic Refresh and selection. Task/group menus target mapped fake
rows after task identity/PID validation. End requires captured fake identity,
killable row and confirmation; cancel/stale/reuse/repeat never dispatch. Only fake
rows plus simulated ledger change; authoritative tasks stay unchanged.
Escape cancels before closing; blur/disposal clear intent. Maximum 256 fake rows,
row names/IDs and filter bounded at 256; no process polling, native action,
capture, texture or persistent art cache.

Painter illustrations reuse source background/sunken/idle/accent tokens and
inset geometry. Browser downloads and Editor/Terminal content are schematic,
not app screenshots. Actual canvas Responses carry semantic names; headers say
`Simulated preview`. Delayed preview measures during hover delay to avoid Area's
invisible first sizing frame. Menus/dummy popup block delayed hover admission,
preventing gallery focus from stealing menu actions. Popup follows shipping
compact header/filter/CPU/memory rows, protected/disabled End and focus feedback;
native tree/paths/ports not reproduced.

Inspected final CPU mesh/atlas evidence: `evidence/showcase-bar.png`,
`showcase-preview.png`, `showcase-gallery.png`, `showcase-processes.png`
(1280 logical / 100% / base-dark), plus `showcase-processes-narrow.png`
(320 logical / 125% / base-light). Block artwork, distinct B/T/E icons, separate
Close actions, CPU progress cells and fake warning visible; narrow popup wraps
rows/actions/filter within window, retains focus and dismissal. Not desktop
screenshots, exact pixel parity or Narrator observations.

Checks used a separate ignored `target/showcase` build directory because the
user's existing executable locked the ordinary target. Existing app untouched.
Final suite: 50 default tests; selected suites five repeats (125/125), fmt and
all-target strict Clippy pass. GPU smoke again passed AMD Radeon RX 6950 XT DX12;
ordinary smoke logged populated three-group composition, three native draw
updates, clean close and exit 0 under separate external 60s bounds.
Independent review delegation unavailable here (subagent depth limit); Leader
owns runner confirmation and independent review.

Popup focus belongs to one invocation: the task/group menu route consumes its
captured menu origin, while the direct Processes route consumes the current button
origin. Opening clears retained menu ownership; close consumes the popup origin.
Actual RawInput regression confirms Browser focus after targeted dismissal, then
Processes focus after a later direct open/Escape, with no simulated actions.
All50/fmt/strict Clippy and five selected-suite repeats rerun green after this
correction; separate 60s GPU and populated ordinary-window smokes exit0.
