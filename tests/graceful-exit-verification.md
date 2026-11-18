# Settings graceful exit: test-owner handoff

Historical handoff for the retired draft/freeze protocol; commands and results below do not describe the current one-way Exit.

Status: Complete for the assigned automated verification scope. Final exact six-stage validation, browser/native acceptance and independent reviews are green. Installed-shell/hardware limits remain explicit below.

## Final acceptance

`$env:RUST_TEST_THREADS='1'; npm run validate` exited zero after executing all six
stages: check PASS (zero Svelte errors/warnings), build PASS, component PASS
(38 files / 667 tests), Node PASS (1,438 passed, zero failed, four explicit skips,
three TODOs), cargo:test PASS (30 targets / 1,503 passed / zero failed / 54 explicit
ignored), cargo:check PASS. Final managed shell log:
`sh_1e8508a0b001o4funC1ripsg4K.out`.

The latest three-document browser gate passed 10/10 on the final renderer import
specifiers. Actual isolated native ownership/admission acceptance passed, including
both terminal handles, six Quick Command handles, probe cancellation/reap/join,
retained worker failure/retry, independent process survival, no dirty-round effects,
frozen FS/Git/settings rejection and real Git worker admission surviving dropped
IPC. This fixture does not install speech capture; the bounded later speech fix
is validated by 33/33 standalone contracts, 44 module passes / one ignored case,
the complete Cargo gates and independent source/deadlock review, not fabricated
live-microphone evidence. No installed-shell/AppBar/hardware smoke was run.

## Intake findings (pre-implementation, historical)

- `src-tauri/src/main.rs:677`: `ExitRequested` and final `Exit` share destructive cleanup. A prevented request can therefore already have shut down subsystems.
- `src-tauri/src/main.rs:690`: terminal cleanup selects only `terminal-panel`; Stack terminal ownership is omitted.
- `src-tauri/src/stack_popup/terminal.rs:642`: target stop removes sessions under lock, then kills/waits outside it, but discards both errors. Failure cannot currently retain truthful ownership for retry.
- `src-tauri/src/quick_commands.rs`: each run retains an `Arc<QuickCommandJobHandle>`; job-close kills descendants. Existing `windows_job_termination_kills_nested_listener_descendant` proves a nested listener cleanup primitive, not application shutdown coordination or independent-app survival. Shutdown needs explicit job drain/join rather than trusting eventual Arc destruction.
- `src-tauri/src/appbar.rs:355`: cleanup has a side-effect plan, failure merge-back, and `cleaned_up` idempotency. Preserve this instead of a second teardown implementation.
- Settings shortcut draft/capture/save, ThemeEditor draft/raw-invalid fields, and Stack editor dirty/save state are local. No cross-window revision/freeze adapter exists at intake. Theme `dirty` alone misses an untouched newly created copy and invalid raw color retained by ThemeColorField.
- Historical shutdown description: `docs/archive/master_spec_full_legacy_2026-09-23.md:225-241`. Historical unconditional cleanup is not the desired new contract.

## Interface handoff

The native and renderer interfaces landed in `src-tauri/src/graceful_exit.rs`,
`src/lib/shutdown.ts`, and `src/lib/shutdownDrafts.ts`; the full implementation
contract is `docs/graceful-exit-implementation-contract.md`. The initial
proposals below are historical test-owner requirements, not current API names.

`ExitCoordinator` now exposes a public injectable `ExitEffects` seam. Tests
compile its actual core (only serde metadata/framework imports removed) with
the actual production `Caller` definition. This does not exercise the Tauri
command wrapper, real HWND resolution or production drain adapters.

Names below are proposals, not assertions that production exports them.

- Path-free `request_shell_exit` admits only exact registered, visible/focused Settings; no renderer-selected PIDs, handles, labels, or child targets.
- Native coordinator seam with injected effects and states Running → Collecting → AwaitingDecision → Freezing → Stopping → Exiting; cancellation/rejection returns Running without teardown. Failed cleanup retains truthful recoverable state and completed-step ledger.
- A participant registration binds concrete native window identity, native-issued registration nonce, participant identity, and monotonic draft revision. Native snapshots all mounted draft owners, including hidden Stack and retained hidden Settings. Labels alone are not authority.
- Snapshot/freeze requests and acknowledgements carry native attempt nonce, participant registration nonce, and exact revision. Final decision references the same snapshot. Replacement, missing participant, changed revision, failed save, pending mutation, and stale/duplicate/malformed replies fail closed. Freeze must be enforced before cleanup, with new mutation/save admission rejected, not merely controls visually disabled.
- Participant adapter must synchronously expose dirty/new-copy/raw-invalid/capture/pending-save state, revision, freeze, thaw, and explicit user-authorized resolution. It must never auto-save or auto-discard as an incidental snapshot effect.
- Injected cleanup effects expose call counts/order, retained failures, and owned child handles for deterministic retry tests. Include both terminal owners, Quick Command jobs, speech/snipping/watch/hook ownership, and the existing AppBar cleanup path. No force-exit fallback.

## Evidence budget / remaining coverage

Dedicated test owner owns all rows; production owners supply narrow seams, never duplicate test implementations of coordinator logic.

| Claim | Minimum decisive evidence | Current status |
|---|---|---|
| Cancel initial focus, Escape/Cancel restore Exit focus, zero writes/hide/exit admission | Production-mounted `tests/components/settings-exit.test.ts` | Six cases PASS; mandatory lifecycle registration/update excluded from destructive-effect count |
| Theme new copy/name-only/invalid raw survives cancelled exit | Same component suite, exact inputs/storage | PASS |
| Cancellable request not unconditional teardown | `tests/gracefulExitRouting.test.mjs` plus effect-count native test | Three source/capability/paired-targeting checks PASS; actual coordinator cancellation counts PASS |
| Coordinator state, exact ACL seam, attempt/registration nonces, stale/duplicate replies | `tests/fixtures/graceful-exit/coordinator.rs` compiled by `tests/gracefulExitCoordinator.test.mjs` | 19/19 actual-core Rust cases PASS reported by native owner; original dispatch/status and AppBar REDs fixed |
| Hidden dirty Stack, revision change after decision, missing/replaced owner | Real coordinator + independent participant barriers; dirty draft content remains after cancellation | Core PASS; hidden dirty Stack/exact content preservation browser PASS |
| Theme pending save, shortcuts staged/capture/pending save; freeze races | Deferred promises/barriers at real mounted adapters and coordinator | Protocol/service pending receipt and revision/freeze races PASS; five real Settings shortcut/readiness cases PASS |
| Retry resumes failed owned cleanup once, successful steps not duplicated | Inject cleanup failure then retry; assert ledger and call counts | Core lane/AppBar/final retry PASS; native real retained-worker timeout/retry PASS in recorded run |
| Both terminal targets + Quick Command root/grandchild stop; independent app survives | Isolated destructive native integration using ONLY fixture-owned processes, independent control child, bounded wait/reap oracle | Final expanded production terminal/QC/probe/retry/survivor run PASS, child 7.09s / parent 7.39s |
| Frozen native mutations reject; admitted actual worker remains Busy after IPC cancellation | Authorized production calls + fixture FS/Git/settings observations + actual Git hook barrier | Seven actual P1 violations captured RED; final native patch rerun GREEN, zero frozen mutations |
| Cross-window destructive ordering, fail-closed missing/replacement | Isolated native integration with actual coordinator/window registry and fixture-owned resources, never installed shell/AppBars | Actual hidden-WebView transport + production owner fixture run PASS; core missing/replacement/deadlines PASS |
| Browser interaction/focus + cross-document draft coordination | `tests/browser/graceful-exit/`, independent production Settings/Stack/ControlPlane documents + actual Rust coordinator with mocked native boundary | Latest ten-journey run PASS including strict unavailable timeout and actual foreign theme writer |

Browser evidence cannot establish real native ACL/job/AppBar correctness. Source tests cannot establish behavior. Native isolated harness must not start the installed shell or alter desktop work area. Live Tauri/Windows shell smoke remains explicitly forbidden without consent.

## Commands / RED

```
npm run test:component -- tests/components/settings-exit.test.ts
node --test tests/gracefulExitRouting.test.mjs
node --test tests/gracefulExitCoordinator.test.mjs
npm run test:component -- tests/components/settings-exit.test.ts tests/components/shutdown-protocol.test.ts
npm run test:component -- tests/components/settings-exit-drafts.test.ts
npx vite --config tests/browser/graceful-exit/vite.config.ts
node tests/browser/graceful-exit/run.mjs
```

Initial component run exposed an overly strict test setup assertion (invoke passes undefined), corrected to check observed read command. Final rerun: six failures, all specifically because `Exit JasonShell` does not exist (11.24 seconds, exit 1). Node: one expected RED on current combined `Exit | ExitRequested` cleanup route (exit 1).

Current focused results (2026-11-17):

- Settings confirmation + production protocol/service admission + real shortcut
  capture/staged/pending/rejected Save/readiness: 22/22 PASS. Initial pre-load
  publication race was reproduced RED and fixed by the UI owner; the assertion
  is retained. Mandatory mount registration/update is not counted as teardown.
- Routing sentinel, Settings-only control/Settings+Stack participation capability,
  paired native/renderer WebviewWindow targeting: 3/3 PASS. These are source and
  configuration assertions, not real HWND/event ACL certification.
- Actual Rust core: original 18/18 PASS. The test-owner REDs for permanent dispatch-retry
  poisoning and ignored pre-drain status delivery failure were fixed by the
  native owner; original assertions remain. Expanded AppBar assertions captured
  17/19 PASS (AppBar incorrectly skipped after an independent cleanup failure).
  Native owner patched and reports 19/19 PASS. Assertions require AppBar last,
  even after earlier failure, and no repeated successful receipt on retry.
- Browser: ten journeys PASS, including mandatory cancel-first focus/Escape,
  hidden dirty Stack/exact draft bytes, untouched new copy, invalid raw fields,
  strict silent-Stack unavailable deadline, unanimous revision-locked freeze,
  exactly one mocked final dispatch, no-authority foreign selection rejection,
  successful unfrozen ControlPlane→Settings canonical persistence, and actual
  third-document writer no-write after freeze. The old read/no-op revision
  churn was RED and fixed without changing the existing 250ms timer. The foreign
  theme write was RED before Option A authority routing, then GREEN afterward.
  The complete ten-journey browser gate was rerun on the latest mechanical
  `.js` import corrections and passed again; latest-evidence.json was refreshed
  and the isolated browser/server/coordinator child were closed afterward.
- `tests/components/stack-exit-completion.test.ts`: 4/4 PASS, typed-folder
  success/failure and external refresh success/failure publish a newer completed
  revision and match the next native snapshot facts.
- `tests/components/theme-selection-authority.test.ts`: 6/6 PASS through actual
  canonical writer, replacing transport only: normal commit, frozen rejection,
  absent authority, replaced registration, deadline/request mismatch, replay and
  saturation retain exactly-once storage writes.
- `tests/components/theme-exit-recovery.test.ts`: 3/3 PASS on first current-source
  run, not an invented RED. A mutation races an old native snapshot; changed
  ACK rejects, abort/thaw restores ready authority, and real Save succeeds
  without remount. Initial unavailable registration and genuinely lost native
  registration both remain fail-closed (no canonical/legacy write).
- Legacy `theme-editor.test.ts` remains 14/14 PASS after only real native
  registration/update/status/listener fixture wiring; save/conflict expectations
  are unchanged. Broader component gate initially found stale editor stubs,
  async observation assertions, contradictory Git metadata fixture and a generic
  mock returning a malformed exit status. Test-owned corrections preserve the
  behavior assertions; focused affected four suites now pass 100/100.

### Native owned-resource evidence

`tests/graceful-exit-native-evidence.md` records the command, exact production
spawn/drain paths, immutable process-handle observations, retained real worker
failure/retry, probe executor cancellation and independent-child survival.
Native owner installed test-only include hooks; no production edits by the
test owner. The earlier initializer compile failure is resolved. The expanded
fixture now compiles and captured seven actual P1 violations (7.56s): missing
real Git worker reservations before/after dropping its IPC future, and frozen
authorized create/delete/Git branch/settings lock/settings height writes.
All writes target fixture-owned temporary state; installed settings are never
touched. Native owner patched after decisive RED. Final exact ignored fixture
now passes (child 7.09s, parent 7.39s), with both admission and owned-resource
PASS markers. No assertions were weakened.

The legacy inline Quick Command source sentinel previously required discarding
pipe-reader handles on Stop. Its test-only replacement requires owned-job
termination before joining both stdout/stderr workers and completed-state
publication only after both joins, rejecting the old discard tuple. The real
native fixture supplies runtime descendant/worker evidence beyond this sentinel.

Independent review is parent-owned (subagent depth limit). The full six-gate
package run completed every stage: check, build, component and cargo:check PASS;
Node and cargo:test FAIL. The Rust failures were missing actual shutdown_workers
includes in two standalone test crates; those test harnesses now include the
real module. Node failures include new extensionless emitted imports (corrected
by the frontend owner), stale source sentinels/extraction dependencies, and old
local-editor tests missing the now-required Settings authority. Focused repairs
and reruns are in progress; final all-green validation is not yet claimed.
The repaired compiled Node gate now passes 1,438 tests with zero failures,
four explicit skips and three TODOs (1,445 total). A serial Rust rerun reached
1,020 passed / four failed / 13 ignored in the application binary before
stopping later standalone execution. Those four were exact event inventory,
obsolete async-handler extraction in two power tests, and the new QC sentinel's
repeated-worker-marker extraction. Approved test-only corrections now pass all
four focused cases, retaining exact event equality, Settings ACL/redaction,
auth-before-admission-before-actual-power-worker, and QC terminate/join/complete
ordering. The subsequent complete `RUST_TEST_THREADS=1 npm run validate` ran
every stage: check, build, component, Node and cargo:check PASS; cargo:test FAIL.
The application binary now passes 1,024 tests with zero failures / 13 ignored;
later standalone speech source contracts reached 30 passed / three failed.

Correcting the two stop-test extractions to the actual delegated blocking worker
(and checking both `drop(commit)` and `drop(_commit)`) produces decisive focused
31 passed / two failed / zero ignored. The intake-close recovery assertion now
passes. The remaining failures are actual source ordering: explicit Stop releases
the lifecycle commit gate before Transcribing emission/intake close, and the
recording-cap worker releases it before tail handoff/intake close/terminal emit.
The cap assertion is unchanged; its drop is on the actual terminal path, not an
unrelated branch. Existing lock-through-handoff invariants were not weakened.
Native owner and parent received the RED. The bounded production fix removes
the two premature `drop(_commit)` calls, retaining the owner-local lifecycle
gate through handoff and emission without retaining the inner-state lock during
stream drop. Source review was reopened for this concrete regression, including
callback/stream-drop deadlock analysis. Fixed standalone: 33/33 PASS; speech
module: 44 PASS / one explicit ignored live-model case. The sole serial
`RUST_TEST_THREADS=1 npm run cargo:test -- --no-fail-fast` sweep passed all 30
targets: 1,503 passed, zero failed, 54 explicit ignored cases. This includes the
application's 1,024 passed / 13 ignored and later standalone targets through
`task_gallery_thumbnails`; none were left unknown behind an earlier failure.
The subsequent exact six-stage `RUST_TEST_THREADS=1 npm run validate` passed
every stage with aggregate exit zero; final counts are recorded above.
The parent reports the latest independent native source review CLEAN, with
original P1/P2 findings closed. The final test/harness and mechanical `.js`
import delta review is CLEAN. The reopened bounded speech source/test review is
also CLEAN: lifecycle commit remains held through actual stop/cap closure and
publication; inner is released before Stream drop; callback tail uses try_lock
and nonblocking intake without commit/inner access; no join cycle was identified.
Reviewed speech SHA-256: runtime
`EED964E5ADA66264962BF3501900CDCD1E1C92DC3A5EAED8DD4D591AA72F1DCA`;
test `6427F84A00B5E851224C6FAD4FAEC3AE0EA4DE7908D9D964905466EA87D19F6C`.
Untested installed
runtime lanes and live-shell limits are explicit in the native evidence; mocked
empty-owner fixture receipts are never counted as their runtime certification.

Requested `C:/Users/jnev1/.config/opencode/agents/e2e-runner.md` absent (`Test-Path` false); bound instructions used. No dependencies, production edits, live native smoke, git staging, or commits.
