# Hotkey-origin regression handoff

## Final test-owner gate (supersedes intermediate gaps below)

Production-owned pure seams now exercise the actual observer transition/admission,
runtime focus-hold decision and registry construction admission without native UI.
Latest runtime inventory requires **23** named bodies and additionally asserts the
child's executed summary is exactly `23 passed; 0 failed; 0 ignored`. Direct run
also executed **23/23**, not merely a successful listing.

New review coverage establishes:

- A native external-origin start followed by registered topbar activation during
  prepare refuses capture and restoration; renderer topbar admission separately
  retains the external pre-click origin. These were added against the supplied
  corrected production seam, not falsely recorded as pre-fix RED executions.
- Injected concrete Process Manager/Stack Browser registration results give a
  positive hold during Preparing/Selecting only for exact helper token/identity;
  capture-ID mismatch, absent/completed phase, different PID, zero instance,
  retired source, user switch and a switch during resolution all refuse hold.
  This proves the shared production callback decision, not real HWND visibility,
  dirty-dialog pixels, native blur delivery, or native adapter property reads.
- Expiring the coordinator while create is paused invokes generation retirement
  before create returns. A returned stale fake WindowPort resource closes without
  context/show/arming; retry gets a new token and old ACK cannot disturb it.
- Native registry admission before/after retirement rejects late construction and
  obsolete-generation reserve while leaving a newer pending generation admissible
  through repeated old cleanup. Retired context commits return exactly `Stale`;
  deadline-only construction refusal remains exactly `ReadinessTimeout`.
  No test performs a real builder/bind/HWND operation.

Final commands/evidence:

| Command | Outcome |
| --- | --- |
| `npm run check` | PASS, 0 errors/warnings after frontend implementation |
| `npm run build` | PASS after frontend implementation |
| `npm run test:component -- tests/components/snipping-barrier.test.ts tests/components/snipping-product.test.ts tests/components/top-bar.test.ts` | PASS 60/60 |
| `npm run test:component` | PASS 537/537, 28 files, final rerun |
| `npm run test:node` | Final FAIL 1,371 pass / 8 fail / 6 skipped; seven isolated HEAD baseline failures plus unrelated deleted archive, attribution below |
| `node --test tests/contractsSettings.test.mjs` | Final PASS 10/10 after exact event inventory correction |
| `npm run cargo:test -- --bin jason-shell core_event_contracts_are_stable -- --exact contracts::tests::core_event_contracts_are_stable --nocapture` | PASS 1/1 after exact inline event snapshot correction |
| `npm run cargo:test -- --bin jason-shell snip_runtime_ -- --test-threads=1` | PASS 23/23, 0 ignored |
| `npm run cargo:test -- --tests --bins` | PASS 1,292 top-level test-body passes / 0 fail / 30 ignored across 28 summaries; registry 20/20, runtime child wrapper 1/1 requiring 23 executed bodies |
| `npm run cargo:check` | PASS against final native changes |
| `npm run cargo:test` | FAIL compiling examples: missing `dist-text-probe` and `dist-p03-experiment` frontendDist directories; no startup reached on this command |

The full component run initially had two missing SDK Webview mock failures and
one product-journey timeout under concurrent full Rust/Node/build load. Only the
TopBar test-owned SDK mock was corrected; no assertion or timeout was weakened.
Focused 60/60 and full 537/537 reruns passed. A runtime-test full compilation also
timed out at 240 seconds on the first broad attempt; subsequent bounded 600-second
runs completed. Intermediate registry failures from obsolete `ReadinessTimeout`
expectations were corrected to exact `Stale`, then the full tests/bins gate passed.
No `0xc0000139` was reproduced in the current integration execution.

### Exact Node baseline attribution

Read-only HEAD archive at `3203040e74073d9dccc39bf3e5c2de17a1bcb710` was
extracted to the approved temp directory
`snipping-head-baseline-5ea7167183234a3681e3c26ed1de552b`, with a junction to
already-installed node_modules. No checkout switch, branch, commit, dependency
installation, or unrelated worktree edit occurred. Full baseline `npm run test:node`
returned **1,376 passed / 7 failed / 6 skipped** (1,389 tests). All seven precise
baseline failure locations also fail in the current tree:

- `centeredSearchSurface.test.mjs:84` and `:104`;
- `changelogPolicyHygiene.test.mjs:32`;
- `searchClearButtons.test.mjs:44`;
- `searchPanelState.test.mjs:536` and `:545`;
- `sourceContractIntentRegistry.test.mjs:152`.

One remaining failure is not HEAD baseline:

- `changelogPolicy.test.mjs` cannot load because the unrelated deleted
  `docs/archive/master_spec_full_legacy_2026-09-23.md` is absent. Its unchanged
  baseline module can load that HEAD file and does not fail. Deletion preserved.

The earlier ninth failure was a concrete feature event-authority omission, now
resolved by the production owner adding `events::SNIP_HOTKEY_FEEDBACK` and its
`events::ALL` entry. Test owner added only the legitimate exact event string to
the Node expectedRuntimeEvents inventory and the Rust inline events snapshot.
Focused contractsSettings is **10/10**, exact Rust snapshot **1/1**. The final full
Node rerun has **1,371 passed / 8 failed / 6 skipped**; its eight failure locations
are precisely the seven reproduced HEAD failures above and deleted archive module.
No feature-specific Node contract failure remains. This does not turn the full
Node suite or standard `npm run validate` GREEN.

Final logs: approved temp `snipping-release-gate-{check,build,node,focused}.log`,
`snipping-release-gate-component-rerun.log`, `snipping-head-baseline-node.log`,
`snipping-review-final-{runtime,cargo-check,standard-cargo-test}.log`,
`snipping-review-final-rust-tests-bins-corrected.log`, and
`snipping-review-contracts.log`. Earlier logs remain intact.
Correction logs: `snipping-event-authority-{rust,final-contracts,final-node}.log`.
The final event-only correction did not change frontend/runtime dependencies, so
the preceding build/check/components/runtime evidence remains applicable.

Live cross-surface hotkey/independent native and renderer responsiveness E2E is
still required, unauthorized and unexecuted. Broad runner expansion was stopped
to address concrete review gaps; no new opt-in runner is claimed. Existing opt-in
native runners were not enabled. Persistent native freeze remains unresolved;
automated worker/registry completion does not certify UI responsiveness.

## First-phase evidence

Command: `npm run cargo:test -- --bin jason-shell snip_runtime_ -- --test-threads=1`

Observed before implementation: **12 passed, 2 failed, 0 ignored**. Initial cold
compilation exceeded 120 seconds; the completed retry compiled and ran all 14
bodies. These are production runtime/tracker tests with injected native edges;
they do not launch shell main, native windows, capture, picker, or clipboard.

- `snip_runtime_own_origin_without_external_candidate_can_reserve`: RED,
  `Some(Inactive)` because own-PID foreground contributes no origin.
- `snip_runtime_own_origin_must_not_inherit_previous_external_target`: RED,
  candidate remains external HWND 10 instead of own foreground HWND 200.
- `snip_runtime_failed_capture_retry_rejects_old_ack_and_preserves_new_generation`:
  GREEN; failed capture releases preparation, retry gets a new token, old ACK
  cannot acknowledge the new operation, cancellation leaves no owned resources.

The inventory wrapper now requires all 14 named bodies, preventing zero-test GREEN.

## Needed acceptance interface

The existing `Tracker::foreground(hwnd, external, own)` seam cannot distinguish
registered normal panels from transient snipping helpers or unregistered own-PID
windows. The two RED cases intentionally reproduce the current native callback's
information loss (`external=None`, `own=true`). **Do not authorize missing identity
or all own-PID windows to turn these GREEN.** Extend the production-owned seam to
carry resolved concrete native registration/instance metadata, then migrate the
two arrangements to explicitly registered Process Manager/Stack Browser inputs.
Keep transient helper activation distinct from a new user origin.

## Registered-seam follow-up

The production owner supplied `Tracker::registered_foreground(hwnd,pid,thread,instance)`.
Both RED arrangements now explicitly use that seam; legacy `own=true` is used
only for proven helper activation, never as a normal-origin authorization path.
The completed follow-up command above passed **18/18 bodies** with no ignores.
The inventory wrapper requires all 18 names. Added evidence covers registered
HWND/PID/thread/instance replacement, lifecycle/unknown-own/continuity invalidation,
same-panel observer refresh stability, switch-away-and-back retirement, exact
restore/no-user-switch, bounded hotkey failure/retry/repeat suppression, and
renderer-start exclusion from hotkey feedback. A delayed retired-attempt failure
is injected at the production failure-publication method to check staleness.

The existing external tracker epoch test now models a genuine switch away and
back; simply re-querying an unchanged foreground identity must not invalidate it.
These synthetic tests do not exercise `NativeRegistry::foreground_identity`
against real HWND properties or prove native feedback WebView targeting.

Positive passive-hold coverage still needs a non-live registry identity fixture:
the current runtime fixture owns fake WindowPort handles, whereas the native
registry's lookup validates real HWND/property/PID/thread state. Do not substitute
an unregistered handle and claim a positive hold pass. Real shell/native hold and
feedback targeting remain consent-gated; no live shell was executed.

## Remaining evidence budget

### Latest native and frontend RED gate

`npm run cargo:test -- --test snipping_native_registry --test snipping_runtime_acceptance`
passed: registry integration **18/18** (including 10 transitively included image
concurrency cases), runtime inventory wrapper **1/1**, requiring all **19** runtime
bodies. New direct registry cases prove unbound-entry retirement wakes context
waiters, rejects late context publication/window access, is idempotent, and allows
same-label retry only with a new epoch. Unbound registrations grant no helper or
foreground authority. New runtime case refuses passive hold for unregistered
sources in idle/preparing/selecting/shutdown; this is negative-path evidence only.
Feedback checks additionally require origin acquisition before started emission,
same-attempt error deduplication, and suppression of retired-attempt failures.

Before frontend feedback implementation, `npm run test:component -- tests/components/snipping-barrier.test.ts`
returned **10 passed, 6 failed**. Every new failure is the missing
`snip:hotkey_feedback` subscription, not a component collection/setup failure.
Designer acceptance expectations are executable in that file:

- Subscribe once via current topbar `WebviewWindow` target and dispose on unmount.
- Reuse `.snip-start-error[role=alert]` for sanitized timeout/generic retry feedback;
  receiving feedback never invokes start or barrier ACK and leaves controls visible.
- Newer started clears a previous click failure before any prepare event.
- Order canonical positive u64 decimal attempts without JavaScript Number rounding;
  stale started/errors cannot mutate the current state.
- The first error wins per attempt; duplicates and repeated same-attempt started
  neither clear it nor replace/reannounce the alert DOM node.
- Reject malformed shape/status/code, noncanonical/out-of-u64 attempts and raw
  diagnostic strings without advancing the attempt watermark. A later valid
  attempt must still clear and report normally.

Full gates run before adding these six frontend RED cases: `npm run check` passed
(zero diagnostics), `npm run build` passed, components **531/531** passed.
`npm run test:node` failed: **1,371 passed, 8 failed, 6 skipped** across 1,385 tests.
Failure locations: centeredSearchSurface lines 84/104, changelogPolicy module,
changelogPolicyHygiene line 32, searchClearButtons line 44, searchPanelState lines
536/545, sourceContractIntentRegistry line 152. Baseline attribution is not proven
by this run; no unrelated tests/docs were repaired. The checkout includes unrelated
documentation deletions and generated permission edits, left untouched.

Logs reside under `C:/Users/jnev1/AppData/Local/Temp/opencode/`:
`snipping-origin-final-{rust,check,build,component,node}.log` and
`snipping-hotkey-feedback-red.log`. This gate is not native responsiveness evidence.

| Claim | Minimum evidence / owner |
| --- | --- |
| Registered own origin, no external history | Actual runtime/tracker with full native registration metadata; test owner after seam exists |
| Exact identity and replacement | HWND/PID/thread/instance replacement, stale registry and unknown-own refusal; test owner |
| User switch wins | Existing before/after restore validation plus own-panel-to-own-panel switch; test owner |
| Visible panels/dialog state | Production-mounted component or narrow native hold fixture; test owner after hold interface exists |
| Picker/preview protection | Block start during pinned save with zero intent/capture/close side effects; retain previous preview on refused start; test owner |
| Cleanup/late creation | Existing cleanup/deadline tests plus delayed UI-creation callback after retirement; test owner after creation seam available |
| Retry/stale generation | New failed-capture retry case plus existing ACK/release generation guards |
| Failure feedback | Targeted native event containing bounded code/attempt identity only; renderer rejects older attempts and clears on new attempt; test owner after event contract agreed |
| Native responsiveness/freeze | Consent-gated real shell message-thread and renderer response, independently sampled during and after cross-window hotkey; native runner owner |

Native live Windows E2E remains required and **unauthorized**. No shell/hotkey,
capture, clipboard, or picker run occurred. Worker completion and UI-creation
timeout are not native message-loop responsiveness evidence. Persistent freeze
is unresolved, not reproduced or explained by these tests. Opt-in native runner
extension and broader checks belong to the next test-owner phase; do not claim
this first-phase handoff completes those journeys.
