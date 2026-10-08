# M0–M1 safe fixture acceptance handoff

Scope approved in task: isolated pinned egui/eframe build; interactive simulated fixture. Tests own this directory only. No production stubs, shipping edits, native discovery, ShellExecute, hooks, AppBars, Explorer mutation, process actions, or original-profile reads/writes. Prior dirty documentation preserved.

## Initial RED evidence

Executed from `C:\dev\jasonshell`:

```text
cargo test --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests
error: manifest path `experiments/egui-bottom-bar/Cargo.toml` does not exist
exit code: 101
```

This is setup-level RED, **not** evidence that individual assertions compiled or failed. Designer supplies manifest/library/runtime next; runner must then obtain compile/behavior results. No fake implementation created merely to manufacture assertion-level RED.

Initial standalone `rustfmt --edition 2024 --check` on both Rust test files passed (exit 0). Initial suite contained 23 contract tests + 2 headless journeys. Coverage unmeasured: `cargo-llvm-cov` executable unavailable on PATH. Independent review dispatch was attempted but blocked by subagent depth limit (1); Leader must route final read-only review directly, after implementation/rerun. No reviewer pass claimed.

## Post-implementation rerun — 2026-10-07

Pinned egui/eframe 0.32.3 implementation inspected at `src/fixture.rs` and shared eframe/RawInput `src/ui.rs`. Test constant-capacity assertion moved to compile-time `const _: () = assert!(CACHE_LIMIT_BYTES > 0);`: preserves positive capacity requirement without Clippy constant-runtime-assertion lint.

Added actual Shift+F10 → Enter admin-menu journey: simulated cancel retains panel, retry with unelevated fallback closes and records `elevated: Some(false)`. All three headless journeys pass in combined run and five consecutive focused runs (15/15 repeated executions).

Added missing strict drag-displacement boundary golden from `src/lib/taskbarGroups.ts:345–353`: moved source center must be strictly past neighboring center. With default 100px model slots, 50.1px and exactly 100px must retain order, 100.1px must reorder. **Production defect:** `src/fixture.rs:706` uses `(delta / drag_slot).round()`, moving at half-slot instead of the neighbor-center boundary.

```text
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests
3 headless journeys passed; 23 contract tests passed; 1 contract FAILED; exit 101
drag_displacement_waits_until_source_center_passes_neighbor_center:
  displacement 50.1; actual first key "second", expected "first"

cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
exit 0
cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
exit 0
cargo llvm-cov --version
error: no such command: `llvm-cov`; exit 101
```

Current suite: 24 contract tests + 3 headless journeys. Original 25 tests remain passing, but overall validation is **Needs changes** until designer fixes displacement semantics. No production edit or test weakening performed. Coverage remains unmeasured; no installation attempted. Live shell/native OS focus/Narrator still skipped outside authorized M1. M0/README implementation-owner historical 25-pass statements precede this new regression; designer/Leader should reconcile them after fix/rerun.

### Final rerun after designer displacement fix — 2026-10-07

Designer changed `src/fixture.rs` and `src/ui.rs` to strict captured-center displacement. Runner reran unchanged regression: default 100px slots retain order at 50.1px and 100px, reorder at 100.1px. Initial RED and preceding defect evidence above retained as history, not current failure.

- `cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests`: **27/27 passed** (24 contracts + 3 headless journeys), exit 0; zero failed/ignored.
- `cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test headless_journeys`: five consecutive repeats, **15/15 passed**, each exit 0.
- `cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check`: exit 0.
- `cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings`: exit 0.

Current focused validation status: **Complete**; ready for Leader-owned independent review. No tests weakened, no runner production edits. Coverage percentage remains unmeasured (`cargo llvm-cov` unavailable). Required headless E2E establishes draw/input/model/simulated-ledger integration only. Live shell, OS focus/Narrator and other native certification remain unapproved/outside M1; no such evidence claimed. No broad shipping validation performed.

### Independent-review regressions RED — 2026-10-07

Review session `ses_ee630917bffe9cox7DSGfVIK6t` identified drawable-path gaps not protected by prior 27-pass evidence. Runner added seven RawInput tests (six fail, hidden ticker check passes), retaining original tests. Actual native modifiers now accompany key events in `RawInput.modifiers`, so Alt reorder genuinely executes. `focus_by_tab` sends actual Tab press/release, observes `Context::read_response(focused_id).rect` matching the current measured semantic widget rect; no model-only focus injection, label-derived guessed ID, or OS focus claim.

| Required regression | Actual RED / acceptance |
|---|---|
| Secondary single | Menu opens, but primary-action ledger is nonempty. Must menu only, no Focus/Minimize. |
| Secondary capsule | Menu opens, but gallery also opens. Must menu only, no gallery/primary ledger. |
| Duplicate gallery captions | Unique-caption positive control dispatches selected `a` Focus once. Same two identities with duplicate captions dispatch **zero** on Enter (not the reviewer's observed double dispatch). Must exactly one selected identity; duplicate labels cannot share interaction IDs. |
| Surviving Close focus after title change | Tab→Enter positive control dispatches `a` Close. Refresh same `a`+identity with new title loses actual egui focus (`Some(0150)` → `None`). Must retain Close focus/dispatch exactly once for `a`. |
| Keyboard reorder then activation | Alt+Right reorders; actual Tab refocuses original identity without any pointer gesture; Enter leaves ledger at 1 instead of 2. Keyboard reorder's suppression must not eat subsequent keyboard activation. |
| Simultaneous ticker focus/hover | Real first label focused by Tab, second differently sized overflow hovered. Painted `Shape::Text` positions at 1.0s/1.8s show focused title does not advance. Both must independently move, preserve disjoint local clip rects. |
| Hidden engaged tickers | Separate passing test checks no `animation_pending` at 1.1s/2s/3s after hiding with two engagements; not a 60s native-idle claim. |

Ticker observability uses existing `FullOutput.shapes` (`Shape::Text.pos`, actual galley text, `ClippedShape.clip_rect`) plus `View.animation_pending`. **No new production public API required.** Designer should implement bounded per-widget ticker state; source review must inspect its eviction/capacity rather than infer bound from a global decoded-image counter. Identity-specific gallery IDs should be stable by opaque task ID+identity+control role, not mutable captions. Existing semantic captions stay accessible; actual widget Response geometry remains authoritative.

Existing standalone-menu test strengthened: reopen menu separately for disabled Minimize and missing-PID Process, capture a fresh menu before removal and before identity reuse, then try Close. All negative cases assert empty ledger/no success diagnostic; independent positive controls assert valid Minimize simulated entry, valid-PID Process unavailable diagnostic/no action entry, valid Close simulated entry. Thus assertions no longer run after the prior menu was consumed. All 24 contracts pass, including this corrected test.

Final RED commands/results:

```text
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests
headless: 4 passed, 6 failed; exit 101 (Cargo stops before contract binary)
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test m1_contract
contracts: 24 passed, 0 failed; exit 0
cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
exit 0
cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
exit 0
```

Current suite: **34 tests** (24 contracts + 10 headless), observed **28 passed / 6 failed** across above invocations. RED authoring handoff complete; implementation validation **Needs changes**. Designer must fix six active regressions; runner then reruns all + five headless repetitions. Original 27-green record above is historical, not current approval. Coverage still unmeasured; live shell/OS Narrator skipped, not authorized. No production changes made by runner.

### Reviewer-fix independent GREEN rerun — 2026-10-07

Designer changed `src/ui.rs` and `src/fixture.rs`; runner reran all unchanged acceptance assertions. Six preceding failures now pass: secondary single/capsule menu-only, duplicate-caption selected identity dispatch, surviving focused Close after caption refresh, keyboard reorder followed by exactly-once activation, independent simultaneous focus/hover ticker movement with unchanged local clipping. Separate hidden-ticker and strengthened menu-admission checks also pass.

- `cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests`: **34/34 passed**, 24 contracts + 10 headless; zero failed/ignored; exit 0.
- `cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test headless_journeys`: five consecutive repeats, **50/50 passed**; each exit 0.
- `cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check`: exit 0.
- `cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings`: exit 0.

Current focused validation: **Complete**, suitable for re-review by original reviewer `ses_ee630917bffe9cox7DSGfVIK6t`. Earlier RED remains historical evidence. Runner changed this evidence document only; tests neither weakened nor rewritten during rerun. Bounded per-widget ticker capacity/retirement is production-source review evidence, not a numerical coverage claim from these tests. Coverage percentage remains unmeasured (LLVM tool unavailable). No native shell/OS focus/Narrator certification or broad shipping validation performed; those checks remain unapproved/outside M1.

### Remaining survivor-index focus defect RED — 2026-10-07

Original reviewer confirmed four of five findings resolved; stable identity IDs fix caption changes, but changing selected survivor index still requests activation focus. Added two actual RawInput journeys, preserving all prior tests:

- `reordered_gallery_survivor_retains_close_focus_not_activation`: open `[a,b]`, use actual Tab to focus measured `a` Close, publish `[b,a]` with unchanged identities, draw, Enter.
- `removal_before_gallery_survivor_retains_close_focus_not_activation`: open `[a,b]`, End selects `b`, actual Tab focuses measured `b` Close, publish `[b]`, draw, Enter.

Both setups dispatch nothing, capture actual focused egui ID, and require one Close for correct survivor, zero Focus, retained control-role focus. Both **FAIL**: one action reaches correct survivor but it is `Focus`, expected `Close`. Enter is deliberately exercised before the final retained-focus assertion, so RED proves user-visible wrong action rather than merely an internal ID mismatch. No new public API or production edits required.

```text
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test headless_journeys
10 passed, 2 failed; exit 101
both new failures: snapshot index change cannot replace Close focus with activation
  actual: Focus; expected: Close
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test m1_contract
24 passed; exit 0
cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
exit 0
cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
exit 0
```

Current suite **36 tests**: 34 passed / 2 failed across scoped invocations. RED authoring complete; overall implementation **Needs changes**. Designer must distinguish actual keyboard selection changes from authoritative snapshot index movement and preserve a surviving activation/Close role. Prior 34/34 GREEN and 50 repeats are historical; after fix rerun all36 and five repeats of all12 headless journeys. Native shell/OS verification remains unapproved/skipped; coverage unmeasured.

### Final survivor-focus independent GREEN — 2026-10-08

Designer changed only production `src/fixture.rs` and `src/ui.rs` to separate activation-focus intent from snapshot index/scroll movement. Runner independently reran unchanged assertions: both reordered-survivor and preceding-member-removal journeys retain actual focused Close and dispatch exactly one Close for the surviving identity, zero Focus.

- `cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests`: **36/36 passed** (24 contracts + 12 headless), zero failed/ignored, exit 0.
- `cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test headless_journeys`: five consecutive repeats, **60/60 passed**, all exits 0.
- `cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check`: exit 0.
- `cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings`: exit 0.

Current focused validation **Complete**, ready for original reviewer's final re-review. Historical RED and earlier GREEN snapshots above do not override this final result. Runner updated this evidence document only; no tests weakened or production files edited. Coverage remains unmeasured (LLVM tooling unavailable). Headless evidence establishes egui input/draw/model/simulated-ledger integration, not live OS focus, Narrator or shell certification; native shell/OS checks remain unapproved/outside M1. No broad shipping validation performed.

### Windows renderer backend startup RED — 2026-10-08

User's ordinary fixture `cargo run --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml` panics in pinned wgpu 25.0.2 `Instance::new`. Added `tests/renderer_startup.rs` using **eframe's actual wgpu reexport**, not a separate test dependency which could unify/enable features and conceal the broken production build. Pinned local `eframe-0.32.3/src/lib.rs:163` reexports wgpu; pinned `wgpu-25.0.2/src/api/instance.rs:65–87` defines compile-time backend flags, and line117 panics on empty flags. Context7 consulted; downloaded pinned definitions remain exact API authority.

```text
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test renderer_startup windows_eframe_renderer_has_implemented_backend_enabled -- --exact --nocapture
FAILED: enabled=Backends(0x0), expected intersection with DX12|VULKAN|GL
exit 101

cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test renderer_startup -- --ignored --nocapture
real_windows_instance_adapter_device_and_command_submission_start FAILED
wgpu-25.0.2/src/api/instance.rs:117:
No wgpu backend feature that is implemented for the target platform was enabled.
exit 101

cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
exit 0
cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
exit 0
```

First regression is always run on Windows, independent of hardware; it detects effective compiled feature graph, not Cargo text. Second is explicitly opt-in GPU/driver integration (marked ignored for default hardware-independent runs, **explicitly executed above**, not quarantined). It calls real Instance creation, real adapter request, real device creation and command submission; failure or missing adapter is never converted to pass. Futures each have 10s deadline. Use an external **60s process timeout** as well, since synchronous driver calls/first future poll cannot be interrupted safely inside Rust. Driver/software adapter information is printed; Noop/browser/Metal backend is rejected on Windows. Drop releases owned GPU objects; no shell/process/pin/window actions.

Current new test status: reproducible backend RED, runtime smoke RED at Instance creation (adapter/device not reached). Previous 36 passing RawInput/contracts did **not** certify native renderer startup. Implementation owner must enable at least one supported Windows renderer backend in the **production** isolated manifest/lockfile, then runner reruns both commands without injecting test-only backend features. No new dependency, production stub or production change authored by runner.

Renderer runtime E2E is **required** for this bug. The runnable smoke proves a real GPU initialization boundary, not eframe surface creation, pixels presented, physical input or native accessibility. After backend fix, additionally validate an ordinary fixture-window launch with external 30–60s bound: actual `run_native` window, initialized wgpu render state, at least one real `FixtureApp::draw` frame, then close only that fixture. If automation needs a seam, owner may provide an experimental-only smoke exit/readiness marker after first drawable frames; marker must not fire at window creation or from headless RawInput. Avoid a simple "process survived five seconds" success claim. Keep shell mode nonexistent; no Tauri, AppBars, hooks or Explorer mutations. Safe ordinary fixture/GPU smoke allowed by this task; native shell/Narrator remain excluded. Coverage still unmeasured.

### Windows renderer startup independent GREEN — 2026-10-08

Owner added production Windows-target pinned wgpu 25.0.2 `dx12` feature unification, lockfile dependencies, explicit `Renderer::Wgpu`, and finite ordinary-window `--smoke-first-frame`. Runner inspected the smoke: creator requires `wgpu_render_state`, each native App update calls actual `FixtureApp::draw`, third update emits readiness and requests only fixture viewport Close. No alternate test features or dependencies introduced.

Independently executed:

```text
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests
37 passed, 0 failed; 1 explicit GPU smoke ignored by default; exit 0
(24 contracts + 12 headless + 1 Windows compiled-backend regression)
cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
exit 0
cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
exit 0

# Executed separately with external shell timeout 60,000ms:
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test renderer_startup -- --ignored --nocapture
1 passed, 0 failed, 0 ignored; exit 0
AdapterInfo: AMD Radeon RX 6950 XT; DiscreteGpu; driver 32.0.21045.5002; backend Dx12
renderer smoke: instance + adapter + device + command submission ready

# Executed separately with external shell timeout 60,000ms:
cargo run --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml -- --smoke-first-frame
Simulated fixture native smoke: initialized wgpu render state; 3 native draw updates; closing ordinary fixture
exit 0
```

Current renderer-startup validation **Complete** on this Windows/DX12 machine: effective backend regression passes, real adapter/device/submission passes, actual eframe ordinary-window App draw/readiness and clean close pass. Both runtime commands completed within external 60s bounds; no timeout/cleanup fallback required. The default ignored hardware test was explicitly executed and passed, not left unverified. Historical empty-backend RED retained above. Runner updated evidence only, no production/test assertion changes during rerun.

Limits: native update marker plus successful `run_native` return establishes initialization/draw/close, not screenshot observation, exact presented pixels, frame presentation completion, other GPUs/backends, physical input or Narrator. No performance/savings claim; coverage unmeasured. No Tauri/native shell/AppBar/Explorer/process action occurred; only authorized ordinary fixture/GPU startup. Original reviewer should review manifest/runtime changes with this independent evidence.

### Approved populated showcase extension RED — 2026-10-08

User requested visible task tiles/previews/dummy Process Manager demo, not real backend/I parity. New runner-owned `tests/showcase.rs` and [SHOWCASE_ACCEPTANCE.md](SHOWCASE_ACCEPTANCE.md) provide 11 RED contracts/journeys and exact constructor/model/input/semantic-widget contract. `FixtureApp::default` retains original empty/unavailable semantics; normal binary must adopt safe `FixtureApp::showcase`, with `live_capabilities=false`. Existing tests unchanged. New focused Cargo test exits101 for absent showcase/Process Manager API; assertions not yet compiled. Standalone test formatting passes. Designer owns implementation/presentation/native first-frame evidence; runner owns later focused+renderer rerun. Prior renderer/36-behavior GREEN is not validation of this unimplemented extension.

### Populated showcase independent final GREEN — 2026-10-08

See [SHOWCASE_ACCEPTANCE.md](SHOWCASE_ACCEPTANCE.md#independent-populated-demo-green--2026-10-08) for complete final command evidence and scope limits. Original48 default tests passed; runner added one missing bounded-row/removed-fake-confirmation safety contract without modifying production. Final default suite **49 passed, 0 failed**, one hardware test explicitly opt-in. Selected `headless_journeys` + `showcase` suites passed five repeats, **120/120** executions (includes eight showcase model tests per repeat, not all E2E). Fmt and strict all-target Clippy exit0. Explicit real DX12 adapter/device/submission smoke and populated ordinary native three-draw-update/three-task-group clean-close smoke each exit0 under independent 60s bounds.

Builds used separate existing `target/showcase` output; user's running app untouched. No original tests weakened. Current focused validation **Complete**; ready for reviewer. No real process/launch/pin/native shell actions, no persistence or full-native/I parity certification. Coverage remains unmeasured; live shell/Narrator outside approval. Historical RED retained above.

### Subsequent direct Processes origin regression RED — 2026-10-08

Reviewer found stale menu-origin focus retained across dummy-popup invocations. New actual RawInput showcase journey passes Browser task-menu dismissal positive control, then direct Processes dismissal fails: focus returns previous Browser `Some(12E3)` instead of current Processes `Some(4D1C)`. Exact focused Cargo regression exits101; fmt/strict all-target Clippy exit0. See [SHOWCASE_ACCEPTANCE.md](SHOWCASE_ACCEPTANCE.md) for steps/commands. Tests-only authoring complete; current implementation Needs changes. New default test count50; prior49-pass result remains historical, not current approval. No new API or production changes, user's app untouched, native shell skipped.

### Final invocation-origin independent GREEN — 2026-10-08

Owner fix resolves above retained-origin RED without weakening assertions. Actual RawInput regression verifies first Browser-menu dismissal restores Browser and later direct Processes dismissal restores current Processes egui ID, not Browser. Independent full default suite **50 passed**,0 failed,1 opt-in GPU test ignored by default. Five selected-suite repeats **125/125** executions (17 RawInput journeys+8 model contracts per repeat). Fmt and strict all-target Clippy exit0. Explicit DX12 GPU adapter/device/submission test passed; populated ordinary native smoke initialized renderer, drew3 updates, reported3 task groups, closed cleanly; both runtime commands separately bounded60s and exit0.

Final validation **Complete**, ready for reviewer. Exact commands/limits in `SHOWCASE_ACCEPTANCE.md` final GREEN section. Separate `target/showcase`, user app untouched; final runner edits acceptance artifacts only. No production edits, real process actions, native-shell/Narrator certification, presented-pixel or full-native/I parity claims. Coverage unmeasured; historical RED retained.

## Exact small public testability seam

Package library crate name: `egui_bottom_bar`. Public module `fixture`. All models below have public fields; enums derive `Debug, Clone, Copy, PartialEq, Eq` where possible. String-bearing types need `Debug, Clone`; view returned by borrow or owned snapshot, not mutable access. Identifiers here are fixture-only opaque strings, never native HWND/PID/path authority. Do not generalize this into a production invoke API.

```rust
// Required signatures (implemented by designer, NOT test-owned stubs).
FixtureApp: Default
FixtureApp::input(&mut self, input: Input)
FixtureApp::tick(&mut self, now_ms: u64) // absolute monotonic fake time, starts at 0
FixtureApp::queue_snapshot(&mut self, sequence: u64, windows: Vec<TaskWindow>)
FixtureApp::view(&self) -> &View // owned View also acceptable
FixtureApp::ledger(&self) -> &[LedgerEntry]
FixtureApp::dispose(&mut self)
FixtureApp::draw(&mut self, ctx: &egui::Context) // same draw used by eframe app
FixtureApp::widget_rect(&self, accessible_name: &str) -> Option<egui::Rect>
pub const CACHE_LIMIT_BYTES: usize; // designer freezes positive capacity + rationale at M0

TaskWindow { id: String, identity: u64, title: String, process: String,
    pid: Option<u32>, active: bool, minimized: bool, attention: bool,
    busy: bool, toasts: u32 }
Pin { id: String, name: String }
Action { Focus, Minimize, Maximize, Close, RequestClose, CloseAll, Process,
    RunAs, Resize } // other actual rendered actions may be added
Outcome { Simulated } // no completed/native outcome in M1
LaunchOutcome { Success, Failed, Canceled, UnelevatedFallback }
Key { Up, Down, Left, Right, Home, End, Enter, Escape, ContextMenu }
LayoutMode { Ribbon, Matrix }
LedgerEntry { action: Action, target: Option<String>, status: Outcome,
    elevated: Option<bool> }
View { sequence: u64, groups: Vec<GroupView>, pressure: bool, height: f32,
    preview: Option<String>, gallery: Option<GalleryView>,
    quick_launch: Option<QuickLaunchView>, menu_open: bool,
    diagnostic: Option<String>, ticker_offset: f32, animation_pending: bool,
    live_capabilities: bool, pending_snapshots: usize,
    thumbnail_destinations: usize, cache_bytes: usize, disposed: bool }
GroupView { key: String, members: Vec<String>, toasts: u32, active: bool,
    minimized: bool, attention: bool, busy: bool, capsule: bool }
GalleryView { mode: LayoutMode, columns: usize, visible_rows: usize,
    close_size: f32, selected: usize }
QuickLaunchView { pins: Vec<String>, selected: Option<usize> }
Input {
    Snapshot { sequence: u64, windows: Vec<TaskWindow> }, Pins(Vec<Pin>),
    StripMetrics { available: f32, required: f32 }, HeightLocked(bool),
    ResizeStart { y: f32 }, ResizeMove { y: f32 }, ResizeEnd, ResizeCancel,
    TileActivate { id: String, pre_click_active: bool },
    HoverTask(Option<String>), HoverPreview(bool), PreviewAction(Action),
    Viewport { width: f32, height: f32 }, OpenGallery(String),
    HoverGroup(Option<String>), HoverGallery(bool), Key(Key), GalleryAction(Action),
    GroupMenu(String), TaskMenu(String), MenuAction(Action), ToggleQuickLaunch,
    QuickLaunchButtonDown, QuickLaunchButtonClick, LaunchOutcome(LaunchOutcome), Blur,
    DragStart { group: String, x: f32 }, DragMove { x: f32 }, DragEnd,
    DragCancel, TrailingClick(String),
    TickerMeasure { text_width: f32, viewport_width: f32 },
    TickerEngage { hover: bool, focus: bool }, ReducedMotion(bool), Visible(bool), Processes
}
```

`draw` translates real egui events into the **same** model used by `input`. `widget_rect` records actual widget `Response.rect` under semantic names, clears stale/hidden entries each frame; no alternate fake event handler or guessed test geometry. Required names: `Quick Launch`, `Processes`, `Editor, 7 windows`, `Close Document 6`. Report current state/count/name truthfully in native accessible labels. New default has no tasks/pins; height 32.4, lock explicitly set in resize tests; viewport defaults 584×328 for model gallery geometry. `tick` advances deadlines, never sleeps. `draw` uses `RawInput.time` via same clock without backward time. `LaunchOutcome` programs deterministic fixture-host next outcome (may persist until changed); all attempted launch entries remain **Simulated**, even fixture failure. Failure/cancel stays open; fallback sets `elevated: Some(false)`. Close records intent only, never removes authoritative tasks. Group CloseAll records RequestClose per still-current captured identity, never includes newcomers.

Resize moves before one tick coalesce to one simulated resize entry. Cancel restores captured height; ending commits clamped height in fixture memory only. Drag tests assume captured default two-item 100px slots for model displacement; UI drag uses measured actual slots. Cancel/lost capture/Escape must route to same cancel transition. At threshold 6px click suppressed even if no slot crossed. Below threshold activates once, including trailing click suppression. No persistent order.

M0 source audit confirmed shipping `perform_task_window_action(Close)` calls `close_window`, whose failure path reaches `terminate_window_process`/`TerminateProcess` and elevation fallback (`src-tauri/src/task_windows/actions.rs`). Do **not** include/call this module in fixture. M1 Close is only a labelled simulated intent; exact native close policy remains a later approval decision, not secretly resolved by fixture tests. Freeze dependency versions/font/cache declaration before implementation; fair matched baseline hardware/performance remains separate work, not an invented M1 measurement.

## Requirement traceability and evidence limits

| Claim | Evidence |
|---|---|
| AC-3/5 sequence, grouping, aggregate busy/toasts, pressure | `sequenced_*`, `grouping_*`, `busy_*`, `pressure_*`; golden values from current TS source |
| AC-2 locked/upward/clamped/coalesced/cancel height | `height_*`; host resize simulated, not AppBar geometry/persistence |
| AC-4/8 action identity + maximize vs focus + simulated close | `tile_toggle_*`, `preview_*`; no root-owner/modal Win32 assertion |
| AC-6 drag threshold/displacement/cancel/exactly once | `drag_*`; no OLE drag |
| AC-7/9 preview/gallery timers and crossing | `preview_*`, `newer_hover_*`, `gallery_hover_*`, `gallery_layout_*` |
| AC-11/12/13/14 focus index, menu Escape, duplicate Close, captured membership, disabled/stale actions | `gallery_spatial_*`, `gallery_refresh_*`, `group_menu_*`, `standalone_*` |
| AC-15 ticker 600ms/40px/s/2s/3s, fractional measurement, fitting/reduced/hidden | `ticker_*`; measurement supplied, painted local clipping must be designer-inspected |
| AC-16/17/18 Quick Launch sort/navigation/empty/remove/blur/Escape/button suppression/failure/retry/admin truth | `quick_launch_*`, first headless journey; only synthetic opaque pin IDs, no canonical path or UAC proof |
| AC-19/24 S unavailable + safe ledger | `processes_*`, both headless journeys; source/dependency review additionally required |
| AC-26/NFR-8 latest-only 100×1000 snapshot stress, 100 gallery cycles, disposal | `snapshot_stress_*`, `repeated_gallery_*`; real cache counts, zero retained native resources |
| M1 multi-step egui input E2E | `headless_journeys.rs`: real RawInput pointer press/release and key press/release through drawable app |

Headless E2E is **required** and must pass; it establishes egui event → production fixture draw → model → simulated host ledger integration, not compositor/GPU/native desktop behavior. Live OS focus, Narrator, forced-colors, screenshots, AppBar/Explorer, DWM, mixed-DPI/hardware and shell E2E **skipped: not authorized / outside M1**. Designer owns visual geometry/themes/fonts/accessibility inspection evidence, labels proof separately; M1 is not native certified. Native adapter identity, canonical shortcut validation, async session/nonce/revision/thumbnail clear-ack races, actual launch/UAC, profile persistence and I Process Manager belong later approved milestones; this suite must not be advertised as proving them. No optional dormant launcher rail/actions tests.

## Focused validation / coverage

After designer creates pinned manifest + lockfile, runner executes:

```powershell
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test m1_contract
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test headless_journeys
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests
cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
```

Repeat headless journeys five times to expose frame/focus state instability. Safety review inspects entire experiment source and locked dependency tree: no shipping module inclusion, native Win32 service/dependency, shell/process/filesystem-launch access, arbitrary imported paths, mode bypass, stale drawable response mapping, or user settings mutation. A false `live_capabilities` flag alone is NOT proof of absence of side effects.

Coverage method: requirement-to-test rows above plus LLVM source coverage of isolated library/reducers if preinstalled `cargo llvm-cov`/LLVM tools available. Target >=80% experiment-library lines; exclude external egui/eframe dependency code, generated platform entry glue, and unapproved native modules. Record denominator, misses and environment limits; do not infer percentage from test count. If tooling unavailable, report unmeasured coverage and explicit behavior coverage; no global tool install authorized. Never run shipping `npm run tauri`, smoke scripts or full native validation for this isolated fixture. Benchmark memory/frame/idle/startup budgets require later matched baseline, not these functional tests.
