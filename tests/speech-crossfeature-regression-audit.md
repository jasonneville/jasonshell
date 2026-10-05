# Speech folder patch / shell-wide regression audit

Audit scope: current dirty worktree, with latest speech-folder change attributed
separately from preceding uncommitted snipping/IPC ACL work. This is NOT a universal
no-regression certificate. No prior dirty work reverted, staged, committed or pushed.
Test owner changed no production source or dependency for this audit.

## Current event expectation repair (test-only follow-up)

User explicitly authorized original test owner to repair ONLY the cfg(test) expected
inventory in `contracts::tests::core_event_contracts_are_stable`. Inserted four literal
strings snip:context/snip:armed/snip:prepare_bar/snip:release_bar immediately after
audio-panel:open in its expected vector. Literal expectations remain independent of
production constants; exhaustive assert_eq!, every old entry and ordering retained.
No production constant/events::ALL/handler/permission or other source edited by this
follow-up. Larger unrelated dirty changes preserved. Audit RED below retained as history.

Focused exact test executed:
`npm run cargo:test -- --bin jason-shell contracts::tests::core_event_contracts_are_stable -- --exact`
**1 passed,0 failed/ignored,869 filtered**,0.00s (11.75s compile). Full
`npm run cargo:test -- --no-fail-fast` completed **exit0:1009 passed,0 failed,26 ignored,
0 filtered** across main/helper/integration targets, against existing prepared experiment
assets. Main:863 passed/7 ignored,180.76s; helpers11+11 passed; all17 integrations124
passed/19 ignored. These are top-level reported counts, not unique coverage or extra
embedded-child counts. Independent `npm run cargo:check` **exit0**,6.42s; existing
warnings remain. Logs: approved Temp/opencode/speech-audit-event-{focused-green,
full-green,cargo-check}.log. The prior Rust event RED is resolved; only known unchanged
Node context-menu CSS baseline remains failing in the original full automated audit.
Unchanged Node/component/native evidence reused, not rerun. This expectation-only fix
has no runtime behavior change; E2E explicitly not repeated. Real runtime/manual bounds
below remain unverified. Independent reviewer ses_f025b2c60ffeEJaRSFIy2mkivp approved
exact test hunk: four independent literals, exhaustive equality/old entries/order retained,
no follow-up production event/handler/ACL change. Read-only review did not rerun checks
or approve full native feature; full Rust completion was pending at that review boundary,
and is now established GREEN above. Leader whole-change signoff remains separate.

## Scope attribution: latest patch is speech; entire worktree is not

Inspected current git status/diff and the recorded implementation-owner handoff.
HEAD is older than BOTH changes, so a HEAD diff alone cannot be called a folder-only
diff. No standalone committed before-folder snapshot exists in this session.

Latest approved folder production surface:
- `src-tauri/src/speech_model_install.rs`: bounded directory copy into shared immutable
  generations, same validate-before-publication/resolver contract as archive.
- `src-tauri/src/speech_runtime.rs`: path-free Settings folder command; shared production
  admission/picker/import/cleanup transaction and warmup completion; owner cfg(test) bridge.
- `src-tauri/src/main.rs`: ONE folder invoke-handler registration (other dirty hunks
  are prior snipping startup/helper/shutdown/eleven-command work).
- `src/ipc/commands.ts`: ONE folder IPC entry (other eleven additions are prior snipping).
- `src/lib/speech.ts`: ONE no-argument wrapper; existing archive wrapper unchanged.
- `src/components/SettingsPanelSurface.svelte`: folder button + shared speech busy/status
  handler/copy. Dirty fifth-hotkey UI/conflict hunks are prior snipping, NOT folder import.
- `README.md`/`master_spec.md`: speech folder setup/contracts; other dirty snipping prose
  pre-exists latest folder scope. Generated legacy command grant grows by one via existing
  build inventory; no new surface/global/snipping grant required by folder feature.

Current larger dirty tree also includes `build.rs`, capabilities/permissions, Cargo,
tauri config, contracts/events/surfaces, native settings/hotkey modules, TopBar, lazy
surface plumbing, snipping modules/components/examples/tests/handoffs. Those belong to
previous snipping/ACL work. Do not say "only speech changed in the whole worktree."

Actual main inventory comparison (HEAD/source, inventory only): HEAD191 handlers;
current203. Exactly12 added:11 snipping commands + import_speech_model_folder. ZERO
old handlers removed; existing archive import remains. Current generated permission
inventory is192 legacy +11 separate snip commands on18 explicit legacy labels. Real
RuntimeAuthority tests, not source parsing, establish authorization below.

## Safety and independent gate execution

Read project policy and current runtime/test contracts. Used installed scripts, not
`npm run validate` short-circuit chain. Each check/build/component/Node/Rust test/Rust
check executed independently. Default Rust ignores native clipboard, native IPC, real
model and external-artifact opt-ins; snipping journey env was not enabled. Full suite
tests compile binaries/examples, never execute production main. Harmless native test
opt-in below is explicitly separate, restored afterward. No AppBar/shell restart,
microphone/network/download, user folder/profile import, focus/display/private pixel or
clipboard mutation. Prior clipboard13 evidence reused as historical only, not rerun.

| Gate | Original audit result (Rust RED below is pre-repair history) |
|---|---|
| `npm run check` | exit0,0 errors/0 warnings |
| `npm run build` | exit0,6.00s; existing large-chunk advisory; final shell dist restored/rebuilt5.38s after experiment-asset preparation |
| `npm run test:component` | exit0,9 files/72 passed,0 failed;6.62s; jsdom canvas getContext not-implemented diagnostics, no new dependency added |
| `npm run test:node` | exit1,1213 total/1207 passed/1 failed/2 skipped/3 todo |
| `npm run cargo:test` initial default | exit101 at compile,0 bodies run; two existing experimental example asset directories missing |
| `npm run cargo:check` | exit0,7.76s; existing warnings remain |
| `npm run cargo:test -- --bins --tests` diagnostic | actual main unit870 collected:862 passed/1 failed/7 ignored,185.31s; failure stops subsequent targets |
| `npm run cargo:test` after local asset preparation | exit101,862 passed/1 failed/7 ignored,182.46s; same exact event-contract failure; assets compile successfully; later targets stopped by Cargo default behavior |
| `npm run cargo:test -- --test '*' --no-fail-fast` | exit0,17 integration targets/124 passed/0 failed/19 ignored; all later integrations executed independently |
| `npm run cargo:test -- --bin snip-clipboard-owner --bin snip-feasibility --no-fail-fast` | exit0,11+11 passed/0 failed/ignored; runs helper TEST modules, not helper mains or clipboard mutation |
| opt-in `shell_acl_native_ipc` all bodies | 3 passed/0 failed/ignored/filtered,10.43s; details below |

Logs: approved `%LOCALAPPDATA%/Temp/opencode/speech-audit-{check,build,components,node,
cargo-test,cargo-check,cargo-bins-tests,cargo-full-final,cargo-integration,cargo-helper-bins,native-ipc}.log`.

Rust inventory (top-level reported bodies, NOT unique-feature coverage percentage):
main870=862passed+1failed+7ignored; helpers22passed; integrations143=124passed+19ignored.
Combined1008passed/1failed/26ignored. Does not multiply embedded child acceptance cases
as extra top-level tests. Main includes all10 new speech-runtime bodies individually
GREEN; integration wrapper also checks actual-binary exact listing/run of all10 bodies.
Coordinator33 includes22 coordinator +11 Runtime bodies; not33 pure coordinator/native
OS evidence. Integration native-clipboard11 are pure protocol/core tests;13 mutation
bodies ignored. Harmless native IPC3 are initially ignored, then separately all executed
as recorded above; do not count them twice as both default passes and opt-in passes.

Integration exact counts:
- eou_artifact_spike:0passed/2ignored; settings_owner_hotkey_save_contract:2passed.
- shell_acl_native_ipc:0passed/3ignored(default); shell_app_acl:5passed.
- snipping_clipboard_native:11passed/13ignored; snipping_coordinator:33passed;
  snipping_geometry:5; snipping_hotkey_settings:1(wrapper requiring7 embedded bodies);
  snipping_native_registry:6; snipping_runtime_acceptance:1(wrapper requiring11 bodies);
  snipping_save:4; snipping_session:5.
- speech_model_directory:7; speech_model_folder_runtime:1(wrapper requiring10 bodies);
  speech_model_install:8; speech_model_load:2passed/1ignored;
  speech_streaming_tdt_fallback_contract:33passed. All0failures/0filtered.

Main7 ignored: search phase3/phase4 artifact bundles; physical Unicode clipboard;
real packaged ASR model; Stack external byte-oracle and populated-storage experiments;
live Explorer tray snapshot. Integration ignored: EOU licensed/model artifacts2,
harmless native3(default),native clipboard13,real packaged model1. Not pass evidence.

### Node failure: established baseline, not latest folder regression

`tests/contextMenuVisualRegression.test.mjs:87:1`:
`icon and placeholder share a centered 16px cell for every row` fails missing exact
combined CSS selector `:global(.js-context-menu .context-menu-icon),` plus newline and
`:global(.js-context-menu .context-menu-icon-placeholder)`; helper17/assertion91.
Same named error exists in preceding recorded full Node1206-pass/1-fail handoff.
Current git diff is EMPTY for this test and its consumed ContextMenu.svelte,
ContextMenuItem.svelte, StackPopupSurface.css/app.css. HEAD test has the same assertion.
This is direct source+historical evidence, not an assumption based on one failure.
No selector relaxed, expectation removed, test skipped or production CSS changed here.

Current skipped bodies: paused `snippingNativeJourney.test.mjs` synthetic journey and
`topBarMicControl.rendered.test.mjs` native browser qualification (known agent-browser
host timeout). Three TODO bodies are historical searchOverhaulPhase0 phase7/phase8/
Everything-provider literal checks, explicitly superseded by active plan/Rust checks.
None count as passes. Full Node adds folder command contract and default-disabled native
journey relative to earlier1211 report; do not conflate count increase with new failures.

### Rust build prerequisite: newly observed environment block, source predates folder

Initial default cargo test compiles `examples/stack_text_p03.rs:130` and
`examples/stack_text_probe.rs:148`; generate_context! panics because frontendDist
`../../../dist-p03-experiment` and `../../../dist-text-probe` do not exist. Example
code and configs have EMPTY dirty diff, and HEAD contains same asset paths. Not a
new speech API/compiler error; also NOT called a historically passed full gate.
Safely built installed local Vite experiment assets offline, WITHOUT launching either
experiment: exact P03 build command from existing launch script, probe config from its
existing Vite config. Initial P03 config-only invocation defaulted to dist; corrected
explicit outDir dist-p03-experiment immediately, then restored normal production dist
with npm run build. No experimental asset substituted for production in final state.
No sources/configs/package/dependencies changed to waive prerequisites.

### Retained pre-repair prior-snipping Rust event test mismatch: RED handoff

`contracts::tests::core_event_contracts_are_stable`, `src-tauri/src/contracts.rs:701`,
actual events::ALL has four prior-snipping events after audio-panel:open:
snip:context, snip:armed, snip:prepare_bar, snip:release_bar. Exact expected vector
then contained all OLD events only. Actual old events/order remain; latest folder adds
no event. Pre-repair contracts.rs dirty diff added ONLY prior snipping command constants,
four event constants and four ALL entries. This newly discovered test regression belongs
to larger dirty snipping work, not speech-folder scope; it was missed by focused gates.
No claim that this proves missing live menu/hotkey functionality. Handed RED to original
owner via investigator. Smallest acceptable fix: TEST expected inventory includes EXACT
four intentional new events at exact position, retaining exhaustive equality/all old
events/order. No production event removal, skip, broad-match or safety-assertion weakening.
Test owner did not edit source during original audit. Subsequent explicit authorization
allows the exact cfg(test) expectation repair documented at top; production remains unchanged.

## Actual native IPC evidence (not application activation)

Executed process-scoped `JASONSHELL_ACL_NATIVE_TESTS=1`:
`npm run cargo:test -- --test shell_acl_native_ipc -- --ignored --test-threads=1 --nocapture`.
All3 bodies passed; process env restored.
- Cross-surface:5 hidden/nonactivating frames,4 expected markers (top-bar pins/menu,
  bottom-bar list_open_task_windows/activate_task_window),7 actual ACL denials,0 snip
  handler calls. Generated production ACL unchanged; terminal handlers return metadata
  only and cannot list/activate private task windows or open actual menus.
- Folder:4 hidden frames/5 outcomes,1 Settings folder marker/4 ACL denials,0 snip calls.
- Forced timeout: exact owned child exits; parent-owned profile removed BEFORE timeout
  error. Normal native WebView profiles verified removed after child-process exit.

Live renderer IPC bridge/ACL error classification/host-independent exact label+command
oracle prevents false positives from missing JS/network/general script failures. Unknown
and owned remote loopback origins deny. No global grant, fake RuntimeAuthority or private
file access. This native transport plus mounted components covers actual transport and
rendered journeys separately; NOT a full real application taskbar/menu/hotkey E2E.

## Cross-feature coverage matrix and limits

All Node modules below are included in full1213 collection; one known context-menu CSS
failure is the sole Node failure. All safe main/helper/integration bodies executed through
independent follow-on targets despite the original default main-unit failure. Subsequent
test-only expectation repair/full no-fail-fast run is GREEN as recorded at top. Historical
results below do not override repaired Rust status or waive manual runtime gaps.

| Feature | Current automated evidence surfaces | Remaining OS/runtime limits |
|---|---|---|
| Taskbar/open windows/activation/pins/gallery/preview | taskbarWindows/taskWindowSnapshotPipeline/taskbarGroups/tilePointer/launcherReliability/gallery/preview/attention/UX Node; actual full handler ACL inventory + harmless native bottom list/activate markers | Real icon-click application activation and enumeration of actual private windows not run; controlled restart/manual observation pending |
| Top-bar/pins/menus | topBarPins/controlReorder/folderReorder/contextMenuOverlayAcceptance/position Node; actual TopBar component; native pins/menu markers; full generated ACL | CSS baseline remains RED; no real native menu handler/foreground/focus proof |
| Settings unrelated controls/hotkeys | contractsSettings/hotkeySystem/windowsKeyOverride Node; actual Settings21 mounted speech+old archive/status/style; Rust native settings/hotkey contracts and action IDs1-4/fifth-snip fixture coverage | Entire Settings has inline sections, not assumed unrelated tabs; arbitrary all-control interaction not separately mounted; real WM_HOTKEY dispatch/global registration/paste not run |
| Speech folder/archive | actual production installer7/archive8/runtime10 fixtures; UI21; static6; generated ACL5; native folder marker | Injected chooser/Err-only loader; no synthetic runtime Ready; native picker/real ONNX Ready/inference/restart-to-UI E2E REQUIRED but unverified |
| Search | search contracts/engine/ranking/UX/close/reset/cache/settings/typing Node; actual search-panel component; Rust search unit/integration suites | Everything external service/network/native launch not exercised; three historical TODOs |
| Terminal/Quick Commands | persistentTerminalPanel/terminalActions/shellIntegration/tabTitle/quickSelect/workbench Node; actual command-panel component; Rust terminal/commands state tests | Real terminal process/desktop focus and native dialogs not run |
| Stack/browser/editor | stack browser/git/text/editor/close/confirmation/paging/nativeDrag contracts Node; actual close-guard + confirm-dialog components; Rust stack/editor tests | Real Explorer/OLE, private file actions, native drag and editor hardware experiments not launched |
| Audio/calendar/tray | audioControls/autoRefresh/topBarCalendar/systemTray/trayPanelRetention/wiring Node; native ACL representative audio coverage | No live devices/audio changes/calendar tray OS actions; not independent mounted end-to-end panels |
| Process manager | processManager wiring/state/UX/a11y/close Node; actual process-manager component; ACL list_processes coverage | No private process enumeration/termination/native process action |
| Workspaces/automation | workspaces/automationProviders Node; Rust workspace/automation/provider unit/integrations | Native spawning/external providers not exercised; planned workspace restoration/startup remain unshipped |
| Snipping/ACL/hotkey migration | snipping Node and34 mounted snipping bodies; generated full command inventory/partition; runtime/reservation/coordinator/native-registry/save/session/geometry synthetic Rust; harmless native denials | Paused journey still unverified; no private capture/clipboard rerun; real AppBar/pointer/mixed-DPI/WM_HOTKEY/paste not established |

Prior taskbar/menu regressions were191 missing app-command grants after app ACL activation,
not explained away as speech missing-model errors. Current native transport markers show
legacy grant restoration for representative paths; source has not removed old handlers.
Speech model-not-found was absence of a shared model import, not proven hotkey-code fault.
No broad permission/backend authorization relaxation or rollback of prior dirty work here.

## Review / next action

Independent reviewer approved exact test expectation repair; leader reviews broader scope
attribution and whole-change signoff. Newly discovered Rust contract RED is resolved.
Complete audit is evidence of tested bounds, not certification of every shell function.
Actual Windows activation/hotkey/mic/model/native-picker E2E remains consent-dependent.
Do not automatically restart shell or revive paused synthetic snipping journey.
