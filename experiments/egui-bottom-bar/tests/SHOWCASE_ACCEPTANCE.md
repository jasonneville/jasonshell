# Populated simulated demo acceptance — 2026-10-08

Approved extension: normal demo starts with task tiles/groups, illustrative generated previews and **dummy** Process Manager popup. Not native task enumeration/capture/process management, not S+I connected parity. Tests own `tests/` only; no shipping changes, settings/profile I/O, external launches/pins/process actions or shell mutation. Prior Default-based tests remain valid.

## Constructors and exact new public seam

`FixtureApp::default()` remains empty, image-unavailable and Process Manager-unavailable for test isolation. Existing 36 behavior tests/renderer regression need no expectation weakening. **Normal binary and finite native first-frame smoke use `FixtureApp::showcase()`**, a safe populated constructor. Dummy capability is scoped to this constructor; `live_capabilities` remains false everywhere. Existing `Input::Processes`, `TaskMenu`/`GroupMenu` + `MenuAction(Action::Process)` open dummy only in showcase. Missing PID or departed/reused task menu cannot target it. Do not quietly turn prior unavailable diagnostics into real-process success.

Required additions to public `fixture` API:

```rust
FixtureApp::showcase() -> Self
// Public view field; borrowed/owned view contract otherwise unchanged:
View::process_manager: Option<DemoProcessView>
DemoProcess { id: String, identity: u64, name: String,
    cpu_percent: f32, memory_mib: u32, killable: bool } // Debug + Clone
DemoProcessView { rows: Vec<DemoProcess>, selected: Option<String>,
    target: Option<String>, confirmation: Option<String>, revision: u64 }
DemoProcessSort { CpuDescending } // Debug + Clone + Copy; more columns allowed
Input::DemoProcessFilter(String)
Input::DemoProcessSort(DemoProcessSort)
Input::DemoProcessSelect(String)
Input::DemoProcessRefresh
Input::DemoProcessEnd
Input::DemoProcessConfirm(bool)
Input::DemoProcessClose
Input::DemoProcesses(Vec<DemoProcess>) // controlled fake snapshot seam for identity races
Action::EndDemoProcess // recorded Outcome::Simulated, opaque fake row ID only
```

`rows` is current visible filtered/sorted copy; master fake rows are local bounded fixture data. Bar Processes opens unselected full fake list, target None. Context menus select/target representative fake row. Case-insensitive name filter; empty filter restores list; CPU descending sort; manual Refresh increments revision and shows clearly fake metrics. No polling real metrics. At least three finite CPU 0–100 rows with memory values and a protected non-killable row. Fake confirmation captures row ID+identity; cancel, no selection, protected row, stale/removed/reused row and duplicate confirmation dispatch nothing. Confirmed fake End records exactly one simulated entry and may remove only fake row; never mutate authoritative task-window snapshot or claim real termination. Escape cancels confirmation before closing popup. Closing/disposal clears pending intent and late operations remain inert.

Deterministic showcase seeds:

- single `demo-browser`, process Browser, title **Browser — Downloads**
- single `demo-terminal`, process Terminal (title designer-owned)
- Editor capsule: `demo-editor-1`, `demo-editor-2`, `demo-editor-3`; initial semantic name **Editor, 3 windows**
- suitable fake pins; title/state showcase variants can remain available in demo controls
- task `demo-browser` maps to fake process **fake-browser** (name includes Browser), Editor group maps to **fake-editor**; protected row has another fake opaque ID

Initial seed control labels above have no appended active/attention suffix so deterministic probes reach them; other scenario cues remain unchanged. Generated artwork is an egui illustration, **not live/captured imagery**, with visible `Simulated preview` text. Existing hover delays/maximize/close/group keyboard contracts remain. Single and gallery illustrations contain at least three actual canvas-local graphic rectangles, not only a placeholder caption. No texture dependency required; decoded-image budget remains truthful if painters allocate no images.

Actual `widget_rect` Response observations required:

- `Simulated preview artwork` (single canvas)
- `Simulated gallery artwork demo-editor-1` / `demo-editor-2` / `demo-editor-3`
- `Filter simulated processes` (real egui TextEdit)
- `Refresh simulated processes`
- `Dismiss Simulated Process Manager`
- `Open Simulated Process Manager` (task/group menu action)
- `End simulated process`
- `Confirm simulated end`

These are semantic labels on actual controls/canvases, never a separate fake dispatcher. Menu action target-specific identity remains opaque, not title-derived. Popup visibly paints **Simulated Process Manager** and fake-metric warning. Designer may mirror shipping surface header/table/theme/focus treatment without native imports.

## Scope references inspected

`src/lib/processManager.ts` defines shipping CPU/memory/process identity fields and authorized list/kill services; those services are **not imported**. `ProcessManagerSurface.svelte` shows CPU-descending sort, filter, selection, manual Refresh, disabled Kill and cancel-first Escape. `ProcessManagerSurface.css` uses theme surface/border/raised bevel, compact header/filter/table/focus rings. Borrow comparable presentation, not full real process tree, executable paths, port/workspace discovery, immutable native kill contract or native polling. Those remain outside dummy scope.

## RED and traceability

`tests/showcase.rs` contains **11** acceptance tests: seven model contracts and four actual RawInput journeys. Covers populated/default separation; finite fake metrics/filter/sort/refresh; task/group targeting; missing PID/stale task admission; fake confirmation/cancel/duplicate/protected/reused identity; Escape/disposal; hover 180ms/illustration; Processes filter/refresh/keyboard dismissal; task-menu keyboard entry; group-menu fake End cancel/confirm; illustrative Ribbon cards.

Initial command:

```text
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test showcase
exit 101
E0599: no associated function or constant named `showcase` ... FixtureApp
E0609: no field `process_manager` on type `&View`
E0599: DemoProcessFilter/Sort/Refresh/Select/End/Confirm/Close/DemoProcesses absent
E0433: DemoProcessSort absent
E0599: Action::EndDemoProcess absent
```

This is **API-level RED**: new assertions do not compile until designer supplies actual implementation. No production stubs fabricated. Standalone test rustfmt check passes. Existing available/unavailable behavior is preserved via constructor distinction, not weakened old assertions. Normal-binary startup wiring still needs implementation/source review plus native smoke; constructor tests alone do not prove `main` chooses showcase.

## Required GREEN validation after implementation

```powershell
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test showcase
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests
cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
# Each real renderer command bounded by external 60s timeout:
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test renderer_startup -- --ignored --nocapture
cargo run --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml -- --smoke-first-frame
```

Repeat all RawInput journey suites five times (existing `headless_journeys` and new `showcase`). GPU/ordinary first-frame E2E required: previous headless suites missed backend startup. Source/native smoke must establish normal/showcase composition, not empty Default; readiness marker may add populated snapshot summary. Designer records actual rendered populated bar/preview/popup evidence and scoped theme/layout comparison; generated art tests establish shapes/labels, not full visual quality or pixel parity. Reviewer verifies bounded fake rows/art caches, no native process/launch path, no source-profile persistence, clear simulation labels and unchanged production shell.

No dummy-popup test establishes real process metrics/end or I integration. No performance claims or native shell/Narrator certification; live shell remains unapproved/skipped. Coverage remains unmeasured; unavailable LLVM tooling is not inferred from assertion count. Stop if adding real services, OS process discovery, arbitrary paths or shipping changes becomes necessary.

## Independent populated-demo GREEN — 2026-10-08

Designer supplied showcase constructor, illustrated single/gallery canvases, fake Process Manager model/UI, and normal/smoke startup wiring. Runner inspected `main.rs`: both ordinary and smoke creator choose `FixtureApp::showcase`. Inspected local fake-end reducer: captured fake ID+identity+killable must still match before only fake row removal + `Outcome::Simulated` ledger entry. Disposal clears dummy popup/rows/confirmation. No real process/launch service found in scoped runtime source. Native smoke verifies shared constructor reports three task groups.

Original 11 showcase tests and prior 37 default tests independently passed (48 total), including four actual RawInput journeys. Runner identified one evidence gap: declared row/string/CPU bounds and **removed** fake-confirmation target were source-checked but not executable assertions. Added `fake_rows_are_bounded_normalized_and_removed_confirmation_is_inert`: removed pending target dispatches nothing; 1000 input fake rows retain exactly 256; names bounded to 256 characters; NaN/200% CPU becomes finite 0–100; ledger remains empty and live capabilities false. This added test passes against existing implementation; no RED or production fix claimed for it. No original assertion weakened.

Final commands used separate preexisting `CARGO_TARGET_DIR=C:\dev\jasonshell\experiments\egui-bottom-bar\target\showcase` to avoid the user's running binary lock; existing app was not killed/replaced.

```text
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests
49 passed, 0 failed; 1 opt-in GPU smoke ignored by default; exit 0
(24 contracts + 12 headless + 12 showcase + 1 backend regression)
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test headless_journeys --test showcase
five consecutive repeats: 120/120 passed; each exit 0
cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
exit 0
cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
exit 0

# Each runtime command independently executed with external 60,000ms timeout:
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test renderer_startup -- --ignored --nocapture
1 passed, 0 ignored; exit 0
AMD Radeon RX 6950 XT; driver 32.0.21045.5002; backend Dx12
instance + adapter + device + command submission ready
cargo run --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml -- --smoke-first-frame
initialized wgpu render state; 3 native draw updates; populated showcase 3 task groups;
closing ordinary fixture; exit 0
```

Repeated 120 executions comprise 16 actual RawInput journeys plus 8 showcase model contracts per repeat; don't label every repeated test a native/headless E2E journey. Actual new draw journey establishes immediate mixed singles+Editor capsule, Browser hover absent at179ms/shown at180ms, canvas-local illustration blocks and simulation caption, dummy popup filter/refresh, keyboard dismissal; other journeys cover task/group menu targeting and fake End cancel/confirm. Missing PID/stale task/reused or removed fake identity/protected row/duplicate confirm/disposal remain inert; fake confirmed End leaves task snapshot intact.

Current focused validation **Complete**, suitable for independent review. GPU/default opt-in test was explicitly executed, not silently skipped. Limits: simulated rows/artwork, not real metrics/captured/live previews or full native/I parity; no presented-pixel/performance/savings claims. Offscreen PNG evidence is designer-owned raster evidence, not desktop certification. Live shell/Narrator unapproved/skipped; coverage unmeasured. Implementation-owner README/M0 may require count update from48/115 to49/120 after this additional safety test.

## Retained process-popup origin RED — 2026-10-08

Reviewer found `open_demo` prefers previously retained `menu_origin_focus` even for a later direct Processes invocation. Added actual RawInput regression `direct_processes_dismiss_restores_current_origin_not_previous_task_menu`: Tab-focus Browser; secondary Browser menu; Tab/Enter opens targeted dummy; Escape dismisses; positive control verifies Browser focus restored. Then Tab-focus Processes (distinct measured Response ID), Enter opens untargeted dummy, Escape dismisses. Must restore this Processes ID, never prior Browser/menu focus. Ledger stays empty and live capabilities false.

```text
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test showcase direct_processes_dismiss_restores_current_origin_not_previous_task_menu -- --exact --nocapture
FAILED, exit 101 (12 other showcase tests filtered)
direct Processes dismissal restores this invocation's origin, not stale Browser menu origin
  actual focused ID: Some(12E3) [previous Browser]
  expected focused ID: Some(4D1C) [current Processes]
cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
exit 0
cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
exit 0
```

Ran with separate `target/showcase`; user's existing app untouched. No production edits/new API/fake focus injection. Positive first-origin assertion passed; final current-origin assertion reproduces reviewer finding. Current suite has 50 default tests (13 showcase + prior37), plus explicitly opt-in GPU smoke. Prior49 GREEN/120 repeats are historical; this new journey is RED. Designer fixes invocation-owned focus capture/retirement, then runner reruns all50 and five selected-suite repeats (125 executions). Native shell unapproved/skipped; real GPU/populated smoke evidence must not be confused with native focus certification. RED authoring complete; implementation validation Needs changes.

## Final invocation-origin independent GREEN — 2026-10-08

Owner implemented explicit invocation-owned origin transfer. Runner reran unchanged regression: Browser task-menu popup dismissal restores actual Browser egui focus ID; subsequent direct Processes popup dismissal restores actual current Processes egui focus ID, distinct from Browser. This is the real RawInput/Response/Context-memory assertion, not a model-only origin assertion. Original RED retained above; now resolved.

Using `CARGO_TARGET_DIR=C:\dev\jasonshell\experiments\egui-bottom-bar\target\showcase`:

```text
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --tests
50 passed, 0 failed; 1 opt-in GPU test ignored by default; exit 0
cargo test --quiet --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test headless_journeys --test showcase
five consecutive repeats: each 12+13 passed; 125/125 executions; each exit 0
cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
exit 0
cargo clippy --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
exit 0
# Each of the following commands separately bounded by external 60,000ms timeout:
cargo test --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml --test renderer_startup -- --ignored --nocapture
1 passed; exit 0; AMD Radeon RX 6950 XT, driver 32.0.21045.5002, Dx12
instance + adapter + device + command submission ready
cargo run --locked --manifest-path experiments/egui-bottom-bar/Cargo.toml -- --smoke-first-frame
initialized wgpu render state; 3 native draw updates; populated showcase 3 task groups;
closing ordinary fixture; exit 0
```

Final focused validation **Complete**, evidence ready for reviewer. The repeated25 tests include17 actual RawInput journeys and8 model contracts;125 executions are not125 native E2E runs. Runner changed only these acceptance artifacts on final rerun; test assertions and production source unchanged. User's existing app untouched. No real process actions or live shell; shell/Narrator remain unapproved/skipped. Ordinary startup/draw/return does not certify presented pixels or physical/native OS focus. Coverage unmeasured; simulated demo remains neither native process parity nor I integration.
