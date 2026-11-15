# Seven-action hotkeys: test-owner handoff

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
