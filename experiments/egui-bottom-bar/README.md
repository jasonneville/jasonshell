# Simulated egui bottom-bar fixture (M0–M1 only)

Ordinary resizable native app. **No shipping-shell changes or shell mode exists.**
Normal startup: mixed showcase with Browser/Terminal singles, three Editor windows
and three fake Quick Launch pins; locked 32.4-logical-pixel bar. The isolated test
constructor `Default` remains empty/unavailable. Fixture scenario controls are
outside the bar. Quick Launch left, equally shared task tiles capped at 160px,
Processes right. Showcase Processes opens a **Simulated Process Manager**, not the
connected integration track or a replacement for JasonShell.

## Run / validate (from repository root)

```powershell
cargo run --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests
cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
cargo check --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml
cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
```

No WebView/Tauri dependency. Pinned egui/eframe 0.32.3, wgpu backend, AccessKit
enabled. Windows production dependency explicitly enables pinned wgpu 25.0.2
`dx12`: eframe's `wgpu` feature alone, with defaults disabled, enables no backend.
Windows/DX12 is the validated native target; other platforms/backends unvalidated.
Lockfile fixes transitive versions. No persistence feature or original
profile access. Winit/wgpu/AccessKit necessarily have platform dependencies; those
are window/render/accessibility plumbing, **not** imported shell services.

## Workflows

Choose mixed showcase, Empty, single, 2/3-window Ribbon, 7-window Matrix/states, or 1000-window
stress. Demo pins sort alphabetically. Launch outcome selector programs Success,
Failed, Canceled, or UnelevatedFallback. Every attempted action remains Simulated;
failure/cancel stays retryable, fallback explicitly says not administrator.

Tab/Shift+Tab reaches enabled controls. Lists/galleries use arrows/Home/End;
Enter activates, Shift+F10 opens the current menu, Escape dismisses menu before
gallery, or cancels a drag/resize. Selected gallery activation and separate
24×24 Close are focusable; other cards are pointer-operable, not extra Tab stops.
Menus expose disabled Minimize/PID actions truthfully. Showcase task/group menus
target only mapped fake rows with surviving task identity and PID. Default-based
fixtures still report Processes unavailable without an action entry. No dormant
launcher rail or Pin action.

The dummy popup mirrors the shipping compact header/filter/CPU-sorted table and
focus treatment, without importing process services. CPU/memory are deterministic
fake values; Refresh is manual, not polling. Filter is case-insensitive. Select a
killable fake row, then End and Confirm; only the fake row disappears and one
Simulated ledger entry records its opaque ID. Protected/stale/reused rows cannot
dispatch. Escape cancels confirmation first, then closes; blur/disposal cancels all
pending fake intent. At most 256 fake rows and a 256-character filter are retained.
Generated preview/gallery canvases illustrate downloads, code or terminal output
and visibly say `Simulated preview`; they are not screenshots or live imagery.

Hover single: 180ms show / 140ms hide; capsule: 300ms / 140ms with crossing hold.
Task drag threshold 6px, measured slots, session-only order (Alt+Left/Right also reorders focused tiles), cancel rollback and
trailing-click suppression. Unlocked resize grows upward, clamps 24–120 and
coalesces ledger entries per tick; cancel restores captured height. Focused unlocked
resize handle also accepts Up/Down/Home/End. Tickers use
600ms / 40px/s / 2s / return / 3s, local clipping, reduced-motion static fallback.

Close records intent only; no authoritative window disappears. Group Close all
records RequestClose only for captured surviving ID+identity members, excluding
newcomers. Shipping standalone Close is force-capable: it is **never imported or
called here**. Every rendered external action has no external adapter.

## Evidence and limits

See [M0-FREEZE.md](M0-FREEZE.md) and runner-owned [tests/ACCEPTANCE.md](tests/ACCEPTANCE.md).
Current suite: 24 model contracts, 12 prior headless tests, 13 showcase tests
(eight model + five RawInput), and one Windows backend regression pass (50 default
tests). The separate
opt-in real GPU startup test also passes; intentionally ignored by default.
Both headless/showcase suites passed five consecutive repeats (125/125 executions,
including model tests). Popup dismissal restores the current invocation's focus:
task/group menu opening consumes its menu origin; direct Processes opening uses
that button's current origin, never a retained earlier task-menu origin.
This verifies pointer/key → drawable fixture → reducer → simulated ledger, not
OS focus, Narrator, DWM, launch/UAC, AppBar/Explorer, or native shell behavior.
Regression evidence covers secondary-menu-only gestures, duplicate-caption task
identities, snapshot/title-refresh Close focus, keyboard reorder/activation, and concurrent
independently timed hover/focus tickers. Drawable ticker state is capped at 64
engaged overflow labels, evicts oldest entries and retires unseen labels each frame.

### Explicit renderer / ordinary-window smoke

Use an external 60s process timeout for each command (GPU futures also have 10s
deadlines). These are ordinary fixture/GPU checks, **not live shell smoke**:

```powershell
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test renderer_startup -- --ignored --nocapture
cargo run --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml -- --smoke-first-frame
```

Observed 2026-10-08: DX12 adapter AMD Radeon RX 6950 XT, driver 32.0.21045.5002;
real instance/adapter/device/command submission succeeded. Ordinary `run_native`
initialized the wgpu render state, called actual fixture draw in three native App
updates with the populated showcase (three task groups), printed the readiness
marker and closed its own viewport with exit 0.
`--smoke-first-frame` schedules only those finite startup frames; normal launches
remain interactive. This is not screenshot/present-completion, physical-input,
accessibility, baseline or performance evidence.

```powershell
# Actual egui mesh/font atlas raster, CPU offscreen, not a desktop screenshot:
cargo run --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --example offscreen -- 1280 1 base-dark 7 experiments/egui-bottom-bar/evidence/base-dark-matrix-1280-100.png
# Optional final mode: bar, preview, gallery, processes (populated showcase):
cargo run --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --example offscreen -- 1280 1 base-dark 3 experiments/egui-bottom-bar/evidence/showcase-processes.png processes
node experiments/egui-bottom-bar/tools/freeze-tokens.mjs
```

The evidence example writes only its explicitly named PNG; it has no live services.
Inspected showcase evidence: bar, single preview, Ribbon gallery, dummy popup at
1280/100% base-dark; dummy popup at 320/125% base-light. Popup wraps at narrow
width, retains filter/action access, and does not overflow horizontally. These
are CPU mesh/atlas rasters, **not desktop screenshots**. No physical-input or
Narrator validation. If an existing demo executable locks the normal target,
set `$env:CARGO_TARGET_DIR='experiments/egui-bottom-bar/target/showcase'` for checks
and startup; no existing app needs to be terminated.
All 34 frozen source theme IDs are selectable; exact declaration freeze is
`evidence/tokens-m0.json`. Renderer's documented alpha/font/effect deviations mean
these are **recognizable fixtures, not pixel-parity certification**. Hardware,
fair matched baseline, performance budgets and savings remain unmeasured/deferred.
Coverage percentage unmeasured (cargo-llvm-cov unavailable). No live shell test run.
