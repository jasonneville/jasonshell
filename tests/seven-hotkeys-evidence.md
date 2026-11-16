# Seven-action hotkeys: test-owner handoff

## Panel-toggle follow-up: current acceptance

The evidence below records earlier open/focus semantics, not acceptance for the
new toggle request. Current acceptance supersedes the earlier repeated-open
claims; historical results are not rewritten as toggle results.

Run the safe existing Vite fixture, then:
`pwsh -NoProfile -File tests/browser/seven-hotkeys/run-panel-toggles.ps1`.
The focused production-mounted journey requires both Alt+O/Settings launcher
and Alt+P/Process Manager launcher to open → hide → reopen. Settings must keep
its mounted editor and unsaved theme name. PM hide must not create an open
generation; only reopening resets System Info to Processes. Wrong-target events
must remain inert. Quick Commands must show → hide → show on repeated clicks.

Initial artifacts: `%LOCALAPPDATA%\Temp\opencode\panel-toggles-browser-red\`
(`summary.json`, `evidence.json` IPC/event trace, `accessibility.txt`,
`completed.png`), with runner log `panel-toggles-browser-red.log` beside it.
These are mocked boundary events and DOM activation, never Windows global keys.

Current safe browser results: focused toggle journey **10/10 PASS**;
adapted seven-action composite journey **7/7 PASS**. Both commands exit 0.
Current artifacts are under `%LOCALAPPDATA%\Temp\opencode\`:
`panel-toggles-browser-current\` and `panel-toggles-seven-current\`, with
matching `.log` files beside the directories. Original RED artifacts remain intact.

The boundary mock honors optional `request.toggle` on the existing show commands;
omission remains explicit show. Production explicit-show wrappers are exercised
twice while visible, with PM PID 101 reaching the real renderer filter. External
blur dismissal is synthetic; the next hotkey reopens each panel. Quick Commands
repeat clicks and native-closed notification/reopen both pass. The composite
journey now verifies toggles and registered Settings hotkey capture diversion.
Mocked native capture diversion is not independent native suppression evidence.

Focused components: **99/99 PASS** in Settings hotkeys, taskbar group menu and
System Info (`panel-toggles-components.log`), preserving real targeted menu paths
and capture lifecycle behavior. `npm run check` passes with zero diagnostics;
`npm run build` passes (`panel-toggles-check.log`, `panel-toggles-build.log`).

Native visibility and deferred blur decisions require actual-source native
evidence from the implementation owner. This browser mock intentionally does not
copy/certify the native launcher-focus timing policy. Live Windows remains
unauthorized and unperformed. Review remains parent-owned: nested reviewer spawn
is blocked by the configured subagent-depth limit.

### Pointer activation follow-up (fixture failure resolved)

The expanded browser journey has **14/16 passing**, exit 1, reproduced twice
after the production pointerdown changes. PM passes long hold (220ms), ignored
compatibility click, pointercancel→detail-zero activation, secondary/non-primary
inertness. Settings fails primary pointerdown default/bubbling prevention and
pointerdown opening before cancellation. Settings programmatic `.click()` toggle
checks still pass. Even a fresh mounted Settings launcher receives a primary
cancelable bubbling `PointerEvent` with `isPrimary:true, button:0` without
`defaultPrevented` or a panel action. Production source contains the intended
handler; this records browser behavior, not an inferred root cause.

Artifacts: `%LOCALAPPDATA%\Temp\opencode\panel-toggles-pointer-current\`
(`summary.json`, `evidence.json`, `accessibility.txt`, `completed.png`), runner log
`panel-toggles-pointer-current.log`. Pointer/mouse events are synthetic DOM events
executing the real handlers, not trusted OS input or native blur execution.

New actual-native source contracts: `tests/panelToggleNativeContract.test.mjs`,
**5/5 PASS**, log `panel-toggles-native-contract.log`. They pin native actual
visibility before show, opt-in defaults, reused Settings capture/PM context
cleanup, explicit PID menu routes, concrete-window deferred hide, atomic revision
invalidation on focus/destroy, visible/focus/snipping rechecks, and immediate PM
retirement before async visual dismissal. These are explicitly source-wiring
evidence, not runtime HWND timing certification.

The 14/16 failure above was traced by the parent to the fixture's incomplete
Tauri window API: missing `onDragDropEvent` threw during TopBar onMount and
aborted deferred pointer-listener installation. The bridge now provides the
correct asynchronous no-op unlistener. No production fix was required. Each
surface records uncaught errors/rejections before mounting; both browser journeys
reject startup exceptions, and the focused journey rejects unexpected IPC too.

Final corrected runs: **17/17 focused PASS**, **7/7 composite PASS**, both exit 0.
Both Settings and PM now pass immediate primary pointer toggle, default/bubbling
prevention, 220ms hold followed by pointerup/detail-1 compatibility click (no
second toggle), pointercancel followed by repeated detail-0 activations, and
secondary/non-primary inertness. No startup errors or unexpected IPC recorded.
Artifacts: `%LOCALAPPDATA%\Temp\opencode\panel-toggles-pointer-fixed\` and
`panel-toggles-seven-fixed\`, matching runner logs beside them. Earlier RED and
fixture-failure artifacts remain historical, not evidence of a production defect.
Native source contracts remain **5/5 PASS**; focused components **99/99 PASS** and
check/build unchanged-source results above are reused. Browser sessions closed;
the parent-owned fixture server remains available. Parent review is still needed.

### Adversarial native blur follow-up: actual-source RED

The browser 17/17 and source contracts 5/5 did not establish native queued-effect
ordering or retained PM refocus. Independent review identified those gaps.
`node --test tests/panelBlurActualSource.test.mjs` now generates an isolated
offline Rust crate in approved temp, inserts the **current actual** delayed
`main.rs` branch and full `retire_system_info` function, and imports production
`system_info.rs` (`RuntimeInner`, coordinator, real token retirement). It compiles
that code against controlled window/main-thread queue ports. No native app,
Tauri event loop, hook, global keys or shell state is started. The facade queues
off-thread `hide` just as a UI-dispatched effect; UI delivery is explicitly held
until after the test-controlled refocus/reopen. It does not copy the blur reducer.

Initial native RED: **1 pass / 3 fail**, Rust exit 101 / Node exit 1. Ordinary
unfocused callback hides. Failing observable assertions:
- `stale queued blur hid a refocused visible panel`;
- `stale queued blur hid a reopened generation`;
- `refocused visually retained panel has a closed coordinator`.

Artifacts: `%LOCALAPPDATA%\Temp\opencode\panel-blur-actual-source-red.log`;
`panel-blur-actual-source\latest.log`, `latest-artifact.txt`, and generated
`run-p0ZfA0\src\lib.rs`/manifest. The production snippets are regenerated on
every run, not checked-in snapshots. Only repository-existing serde functionality
is used, resolved from the local Cargo cache with `--offline`.

Implementation acceptance:
1. Queue delayed UI callback, then refocus/advance revision before delivery:
   callback must not hide. A newer generation/revision must likewise survive.
   Unfocused/current delivery must still hide. Final revision, focus, visibility
   and snipping guards plus hide must execute together in the main-thread task.
2. PM blur must reject old System Info tokens immediately. Refocus while visually
   retained must expose an active production context, permit Processes/System Info
   tab transitions and fresh live-work admission, and preserve renderer usability.
   Either separate cache retirement from close or restore/publish active context.

The implementation owner must provide/use the actual refocus lifecycle seam.
Current facade extracts the delayed `if let (Some(revision), Some(expected))`
branch; if the fix introduces a new module/helper or a separate refocus branch,
the extractor must be adapted to execute that actual source too, not a test-only
replacement policy. Browser retained-refocus recovery/publication checks follow
the selected native contract. Final gates/review remain pending this fix.

### Adversarial native blur follow-up: fixed-source GREEN

`node --test tests/panelBlurActualSource.test.mjs tests/panelToggleNativeContract.test.mjs`
now passes: **6/6 actual-source Rust behavioral cases**, plus **5/5 source-wiring
contracts** (the Node wrapper reports six passing top-level tests). The generated
facade additionally extracts production `suspend_system_info`, the actual native
scheduler admission/cache-warming lock block, and the actual collector
focus-inactive branch. Production `system_info.rs` remains imported directly.

The same held UI queue interleavings that failed RED now pass: refocus/new revision
before delivery cannot hide; reopened generation survives; normal unfocused UI
delivery hides once and retires PM context. Blur suspends work/cache without
changing visible/tab/generation/revision/PID. PM tab transitions and fresh work
admission succeed after retained refocus. The actual scheduler rewarms a readable
cache on the same context; all pre-blur worker completions reject without
clearing resumed tokens. The independent collector-inactive path also suspends
without silently closing the context and permits subsequent refresh.

Final artifacts under `%LOCALAPPDATA%\Temp\opencode\`:
- `panel-blur-actual-source-final.log`: wrapper + wiring pass;
- `panel-blur-actual-source\latest.log`, `latest-artifact.txt`: six native case
  results and exact generated actual-source crate path;
- `panel-toggles-pointer-native-fixed\` and matching `.log`: **17/17 browser PASS**;
- `panel-toggles-seven-native-fixed\` and matching `.log`: **7/7 composite PASS**.

Both browser reruns executed after the native fixes, with no startup errors or
unexpected IPC. They remain mocked-boundary evidence; native queue/lifecycle
behavior is established separately by the actual-source facade. Frontend
check/build and **99/99** focused components are reused because no frontend
production code changed. Implementation owner reports cargo-check passing with
264 warnings; that command was not duplicated by the test owner. No task-owned
defect remains in this focused evidence. Parent review remains pending; no live
Windows shell, OS registration, hook, hardware telemetry or HWND delivery is
certified. Browser sessions are closed; parent-owned fixture server is untouched.

### Volume-probe suspension follow-up: actual-source RED

Native-only review found the retained context cannot alone authorize storage
work after suspension. `node --test tests/volumeProbeSuspensionActualSource.test.mjs`
now extracts the **actual current** volume `collect` and `unavailable` functions,
imports production `system_info.rs` and `probe_executor.rs` directly, and runs an
offline Windows Rust fixture. Only root enumeration and the helper executable
are test ports: the executor adapter forwards the **production cancellation
closure** to real `execute_command_observed` with the existing isolated `stall`
helper. No shell, Tauri app, hardware query or user process is launched/killed.

Initial result: **0/3 pass, 3 fail**, Rust exit 101, Node exit 1:
- suspended-before-collect work consumes a probe reservation ID (2 vs expected 1)
  and reaches the executor adapter; stale work must be rejected before reservation;
- suspension after real child spawn returns `TimedOut`, expected `Cancelled`;
- retiring only the ProbeToken while WorkToken/context remain current also
  returns `TimedOut`, expected `Cancelled`.

Both running-child cases already pass kill→before-wait→reaped ordering, exact
event counts, held OS process-object signaled/exited checks, budget slot retained
through BeforeWait and Reaped notification, and release only after collector
observes executor completion. Their final assertion fails on the wrong reason:
they wait for deadline instead of recognizing suspension/retired ownership.

Artifacts under `%LOCALAPPDATA%\Temp\opencode\`:
`volume-probe-suspension-red.log`, `volume-probe-suspension\latest.log`,
`latest-artifact.txt`, and generated `run-Y75qNi\src\lib.rs`/manifest.
Exact acceptance: current WorkToken ownership must gate probe reservation and
executor cancellation; a retired ProbeToken must cancel even when the context
and WorkToken still match. Retired live-child reservations stay held until actual
kill/wait/reap completes; context must remain unchanged by suspension. A valid
unsuspended work/probe path must remain admitted (add positive regression with fix).

Prior **6/6** blur lifecycle, **5/5** wiring, **17/17 + 7/7** browser checks remain
valid for unchanged surfaces but do not cover this volume seam. Browser evidence
is reused for this native-only follow-up; focused native correction/review pending.

### Volume-probe suspension follow-up: fixed-source GREEN

Final command:
`node --test tests/volumeProbeSuspensionActualSource.test.mjs tests/panelBlurActualSource.test.mjs tests/panelToggleNativeContract.test.mjs`
passes (seven top-level Node tests): **6/6 actual-source volume cases**,
**6/6 actual-source blur/resume cases**, **5/5 wiring contracts**. Expanded volume
cases cover suspension before reservation, suspension during a real child,
independent retired-probe cancellation, old WorkToken ID versus a replacement
with identical context/lane, valid current work decoding/publishing without a
kill, and legitimate current-work timeout without false cancellation. Child
process exit and reservation retention through wait/reap remain asserted.

Existing operational cleanup regression:
`npm run cargo:test -- --test system_info_probe_executor`
passes **7/7**, exit 0 (264 application build warnings). This includes bounded
timeout, flood/overflow, exact/empty output, nonzero exit, wire decoding,
pre-cancellation/spawn failure, and old-epoch slot retention through reap.

Final logs under `%LOCALAPPDATA%\Temp\opencode\`:
- `panel-native-volume-final.log`: both actual-source wrappers + wiring PASS;
- `volume-probe-suspension\latest.log`, `latest-artifact.txt`: six case results,
  generated crate `run-IN9y1O`;
- `panel-blur-actual-source\latest.log`: six blur/resume cases PASS;
- `panel-probe-executor-final.log`: operational cleanup seven PASS.

Original volume **0/3 RED** remains recorded above. No task-owned defect remains
in final focused evidence. Browser **17/17 + 7/7**, frontend **99/99** components,
check/build are reused for this native-only correction; their production surfaces
are unchanged. Implementation owner reports cargo-check PASS. Parent final review
remains required. No live Tauri shell, global hooks/keys, hardware query or user
process kill was performed; only isolated test-owned helper children were run.

## Evidence boundary

No live Tauri app, Windows shell hook, AppBar, or native smoke is authorized.
Initial RED tests use the real Settings component with mocked IPC and actual
Rust settings load/update/reload functions with private temporary JSON files.
They do not establish OS registration, native identity, expiry, or routing.

## Initial RED commands and artifacts

Logs are under `C:\Users\jnev1\AppData\Local\Temp\opencode\`:

- `npm run test:node`: `seven-hotkeys-node-baseline.log` (before test changes).
  Fresh result: 1,393 tests, 1,380 passed, 7 failed, 3 skipped, 3 todo, 0 cancelled.
  Historical 10-failure baseline is **not** this checkout's result.
  Failure locations: `centeredSearchSurface.test.mjs:84,104`,
  `changelogPolicyHygiene.test.mjs:32`, `searchClearButtons.test.mjs:44`,
  `searchPanelState.test.mjs:536,545`, `sourceContractIntentRegistry.test.mjs:152`.
- `npm run test:component -- tests/components/settings-hotkeys.test.ts`:
  `seven-hotkeys-component-red.log`. Five tests failed: missing new controls,
  reset writes immediately, and capture never requests native lease ACK.
- `npm run cargo:test -- seven_hotkey_acceptance`:
  `seven-hotkeys-rust-red.log`. Compiled successfully; 1 passed, 4 failed,
  exit 101. RED failures establish absent new defaults, ignored explicit new
  duplicate fields, absent migrated new fields, and an unrelated write
  materializing omitted snipping. Narrow explicit-snipping rejection passed.

## Agreed wire shape

Commands: `get_standard_hotkey_status`, `save_standard_hotkeys`,
`begin_standard_hotkey_capture`, `renew_standard_hotkey_capture`,
`end_standard_hotkey_capture`. New commands must authorize only the exact
registered visible/focused Settings instance and remain outside legacy build
permissions. No Process Manager ACL expansion.

Status: `{ revision, bindings: [{ action, id, binding, active, reason, state }] }`,
where `state` is `active`, `inactive`, or `capturePaused`.
`active` describes verified native registrations, not configured wishes.
Save request: `{ request: { hotkeys, revision } }`;
response `{ committed: true, settings, status, statusError }`, where status is
nullable only when the postcommit observation is unavailable. Registration and persistence failures must
return/refetch truthful current status, including incomplete rollback.
Capture ACK: `{ leaseId, revision, sequence, expiresInMs, registeredBindings }`.
The ACK's verified `registeredBindings`, not configured settings, owns renderer
deduplication. Begin sends a monotonic capture-button revision. Runtime status
revision advances independently on capture transitions; Save drains all pending
lease releases and refetches status before the strict Save command.
Native targeted capture event: `{ leaseId, revision, sequence, action, binding }`.
Renew/end include lease ID and revision. Timers are native-owned; ACK does not
make a renderer expiry timer authoritative. Capture events never replay actions.
These contracts match `src/lib/standardHotkeys.ts` and the native owner.

## Implemented evidence paths

| Claim | Minimum decisive evidence |
| --- | --- |
| IDs 1–5 unchanged; PM=6/Alt+P, Settings=7/Alt+O | Real registry construction/ID dispatch fixture, all seven assertions |
| Inferred internal/OS conflicts inactive without startup failure | Inject OS registration failures; assert exact verified slots/status; mixed missing snipping/new fields |
| Unrelated writes preserve migration provenance | Real general save, bar height/lock, quick-command update, restart; old custom chord and omissions unchanged |
| Strict repair Save then restart | Validate/register before disk; all seven persisted only after successful explicit Save |
| Rollback truth | Registration failure, disk failure, rollback partial failure; exact old vacant/active slots and monotonic status revision |
| Draft can swap duplicates | Temporary duplicate accepted into draft, Save blocked until repaired; swap then one seven-way Save |
| Unrelated controls do not commit draft | Change profile/bar control mid-draft; persisted hotkeys remain committed and draft survives response |
| Capture handshake and stale rejection | Deferred begin ACK; no local chord before ACK; registered chord only from native event; old lease/revision/sequence ignored |
| Capture lifecycle safety | Native monotonic expiry independent of renderer; renew/end; blur/hide/native focus loss/destruction/disposal; queued/held/repeat events never replay |
| Exact Settings authorization | Wrong label, stale/recreated instance, hidden/unfocused caller rejected for each command; no expanded process ACL |
| Paired routing and opening | Target-aware mock emitter/listener; PM event bottom only, Settings event top only; repeated press focuses/opens, never hides |
| Surface state retained | PM opens Processes from System Info; Settings remains mounted and theme draft survives repeated opening |
| Integrated multistep journey | Production-mounted TopBar, BottomBar, Settings and PM browser fixture, target-aware mocked IPC; open twice, capture/swap/save/fail/retry/cancel/hide/reopen |

Browser artifacts must include IPC/event trace, assertions/results and screenshots.
Use existing local browser tooling only; no dependency installation. Full final
gates: check, build, components, Node, Rust tests, cargo check, plus focused QA.

### Runnable current acceptance

- `npm run test:component -- tests/components/settings-hotkeys.test.ts`:
  initial expanded **25/25 pass**, `seven-hotkeys-component-final.log`. The
  current file has **27 passing cases**, included in the full 572-case gate
  below; the two additions verify nullable committed status and clearing stale
  inactive claims. In addition to staged
  reset/cancel/save/failure and duplicate swaps, this covers ACK-authoritative
  registered/local deduplication, foreign targets, stale lease/revision/sequence,
  before-ACK input, late ACK disposal, stale rejected begin and stale rejected
  release without cancelling a newer capture, pending normal/stale/invalid lease
  release blocking Save/status refresh, renewed/failed/hung renewal, expiry,
  held/repeat input, blur/hide/unmount, and truthful inactive feedback.
- `npm run cargo:test -- seven_hotkey`: **25/25 pass** in the first expanded
  run, `seven-hotkeys-rust-focused.log`. Current fixtures use the **actual
  production reducers**, settings writers and explicit-Save transaction, not a
  copied capture algorithm or the superseded legacy save transaction.
  `seven_hotkey_settings_acceptance.rs` covers migration and every unrelated
  writer/restart, strict explicit repair/restart, validation/OS/persistence
  failure ordering and opaque verified-slot rollback snapshot pass-through.
  `seven_hotkey_registry_acceptance.rs` covers stable IDs, mixed internal/OS
  inferred conflicts, strict explicit conflicts, exact vacant-slot rollback and
  partial truth, exact-instance eligibility, seven-way diversion, ACK verified
  chords, stale transitions, independent injected monotonic expiry/renewal,
  lifecycle retirement, stale queued/packed/vacant messages and uptime rollover.
  Final additions cover ineligible old-instance calls not cancelling a new
  owner's lease, ancient uptime fences after long idle, and exact all-seven IPC
  Save decoding (disk migration defaults are not accepted for explicit Save).
  A real transaction case additionally retains private Quick Commands through
  unrelated panel writes and explicit shortcut repair, while redacting the
  Settings response. Postcommit focus-loss coverage combines the actual
  transaction, a source-wiring contract for the trusted committed snapshot and
  public status guard, and the four-surface browser retry journey.
- `pwsh -NoProfile -File tests/browser/seven-hotkeys/run.ps1`: **PASS**, seven
  composite journey assertions, four mounted production surfaces in separate
  documents. Server command and precise evidence boundary live beside the
  fixture in `tests/browser/seven-hotkeys/README.md`. Latest trace and screenshots:
  `C:\Users\jnev1\AppData\Local\Temp\opencode\seven-hotkeys-browser-e2e\`.
  `report.html` and `junit.xml` summarize only those seven browser assertions;
  they do not claim the full repository gate is green.
  The journey rejects wrong-target opening/capture, opens/focuses repeatedly
  without toggling, resets PM to Processes, retains Settings mount/theme draft,
  captures/swaps/saves/fails/retries/cancels through real handlers, checks
  committed-only unrelated controls, loses focus after successful persistence
  without falsely failing Save, and verifies every new listener target.

### Regression maintenance and full gates

Existing speech tests keep their assertions but scope status/alerts to Speech;
theme Cancel queries scope to the theme region. Their mocks now implement the
new status command. Snipping duplicate feedback intentionally moves from its
special-case alert to seven-way draft validation without remapping either chord.
Static five-way/default/event inventories and literal per-row capture markup
checks are updated for the agreed seven-way/data-driven editor. Existing
unrelated baseline failures remain untouched.

- `npm run check`: pass, zero errors/warnings; `seven-hotkeys-check.log`.
- `npm run build`: pass; `seven-hotkeys-build.log`.
  These runner logs establish the earlier full gate. The final nullable-response
  renderer delta additionally passed the frontend owner's check/build/autofixer
  handoff; that unchanged-source evidence is reused rather than rerun. The test
  owner's final full component gate below includes the completed renderer delta.
- Full component gate: **572/572 pass**, 30 files;
  `seven-hotkeys-components-full.log`.
- `npm run test:node`: **1,382 pass / 7 fail / 3 skipped / 3 todo**, 1,395 tests;
  `seven-hotkeys-node-current.log`. The exact seven failure locations match the
  fresh pre-change baseline above. No new Node failure remains.
- `npm run cargo:check`: pass; `seven-hotkeys-cargo-check.log`.
- Initial unfiltered `npm run cargo:test` hit missing example frontend artifacts,
  not a test failure. Built only `vite.p03-experiment.config.ts` into
  `dist-p03-experiment` and `scripts/stack-text-editor/vite.config.mjs` into
  `dist-text-probe`; **never launched either native example**. The next run
  exposed two obsolete source-wiring assertions requiring generic Settings
  writes to register submitted drafts; those intentional contract changes now
  pin committed-only generic writes and the production strict Save seam instead.
  Earlier full Cargo rerun: **1,325 pass, 30 ignored**, before final response
  additions. Final full Cargo on the completed implementation and test files:
  **1,329 pass, 30 ignored**, 29 targets, exit 0;
  `seven-hotkeys-rust-full.log`. Serialized latest-source focused/response-contract
  evidence is in `seven-hotkeys-rust-focused-final.log` and
  `seven-hotkeys-response-contract.log`: **32/32 acceptance** and **3/3 response
  wiring** pass on the latest source. The subsequent serialized `cargo:check`
  also passes. No ignored native acceptance test was opted into.

All requested automated evidence is complete. No reproduced task-owned defect
or missing critical test seam remains. The overall Node command still exits 1
solely for the exact seven freshly confirmed pre-existing failures; the
machine-readable comparison is `seven-hotkeys-node-baseline-comparison.json` in
the approved temp artifact directory. Browser and its owned fixture server were
closed after artifact capture.

Native live OS delivery/focus, real registration collision, and live timer/HWND
behavior remain consent-gated and unperformed. Pure reducer/injected OS tests
and mocked-IPC browser evidence do not certify those native Windows journeys.

### Postcommit observation failure regression

The initial trusted snapshot still propagated a native channel error after disk
commit. The native owner added production-used `standard_hotkey_save_response`:
precommit errors stay errors and never observe status; committed saves always
return `committed:true`, redacted committed settings, nullable status and a safe
`statusError`. New tests execute the actual transaction's disk write through this
response seam, force observation failure, verify no fabricated bindings/revision
or raw diagnostic disclosure, and retain private owner data. The browser journey
adds the same response case. Renderer RED was reproduced before adaptation:
25 pass / 1 fail; `seven-hotkeys-postcommit-component-red.log`. The committed
nullable-status warning branch now passes in the full 572-component gate and
seven-check production-mounted browser journey. Native response-seam cases pass
in the latest 32-case focused Rust run. The originally reported response defect
is closed by this evidence; live native certification remains outside scope.

## Master integration verification

Integration checkout: merge `4a57860`, ordinary merge of master `a18cabd` into
hotkeys `9f1e4fc`. Master contributes the snipping Escape observer, generation
mailbox/arming cancellation and raw-device-event filtering; no production merge
resolution was edited by the test owner.
Results apply to that merge plus the test-only coexistence/import/intent-wiring
changes recorded here; these test changes must accompany the integration commit.

The added coexistence case lives in `seven_hotkey_registry_acceptance.rs` and
uses both production `NativeState` and production snipping
`EventRouter`/`SnipRuntime`/`Coordinator` in one test. Its test-only bridge reuses
the existing synthetic runtime fixture; it does not copy a keyboard/cancellation
model or install a hook. All seven registered actions remain captured in idle
and Selecting phases. Idle Escape admits no snipping work; active Escape cancels
the actual selection while preserving the independent native shortcut lease.
The companion source contract preserves both startup/shutdown owners, the
production `DeviceEventFilter::Always` setting and unconditional
`CallNextHookEx` forwarding. Real OS key delivery remains unverified.

Merged logs use approved-temp prefix `seven-hotkeys-merge-`:

- `check.log`, `build.log`: pass.
- `components.log`: **572/572 pass**, 30 files.
- `node.log`: **1,385 pass / 7 fail / 4 skipped / 3 todo**, 1,399 tests. Failure
  identities exactly match the seven fresh pre-hotkey baseline failures. The
  master-added input-filter contract is now tagged in the intent registry; the
  unrelated pre-existing Search placement inventory omission remains intact.
- `browser-e2e/`: **seven checks PASS**, including strict capture/save routing,
  failure/retry, committed nullable status and theme-draft retention. Includes
  three screenshots, trace, accessibility snapshot, HTML and JUnit.
- Native focused hotkey/coexistence, existing Escape reducer, diagnostic, full
  Rust and cargo-check logs are recorded by the serialized native command.
  Hotkeys/coexistence **33/33**, existing Escape reducers **16/16**, and default-off
  diagnostic acceptance **4/4** pass. The first full native compile exposed an
  omitted `diagnostics` module in the standalone `snipping_native_registry`
  harness after master's new coordinator import. Adding that actual-source
  test module dependency fixes harness wiring without a production behavior edit.
  Repaired standalone registry harness: **20/20 pass**. Final merged full Rust
  gate: **1,446 pass / 51 ignored**, 30 targets, exit 0. Final merged
  `cargo:check` exits 0. All ignored opt-in native tests remain unexecuted.

Native opt-in flags remained disabled. No live app, hook installation, global
key injection, native snipping example or shell smoke was launched. Existing
generated-permission line-ending noise was left untouched.
No actionable production incompatibility remains in the automated evidence.
Master advancement/committing these test artifacts remains the Git owner's job.
