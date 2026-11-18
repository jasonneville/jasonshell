# Graceful exit: isolated native owned-resource acceptance

Historical evidence for the retired draft/freeze protocol; not current one-way Exit verification.

Boundary: automated isolated native fixture, **not installed-shell smoke**.
No ordinary shell setup, AppBars/work-area changes, global hooks, desktop
capture, speech/audio, selected user processes, or user task windows are started.
Synthetic hidden WebView2 documents use an independent app identifier and
parent-owned temporary data directories. Installed settings are not modified;
actual settings persistence is redirected to a fixture-owned temporary file.

## Command

```powershell
$env:JASONSHELL_GRACEFUL_EXIT_OWNED_TESTS='1'
npm run cargo:test -- --bin jason-shell graceful_exit::owned_e2e::cross_window_freeze_drains_real_owned_jobs_and_both_terminals_only -- --ignored --exact --test-threads=1 --nocapture
```

The explicit opt-in ignored test is defined in
`src-tauri/tests/fixtures/graceful_exit_owned_e2e.rs`, included by the native owner
under `cfg(all(test, windows))`. Ordinary package Rust tests compile it but do
not run its destructive fixture. This is not permission to run live shell smoke.

## Real paths exercised

- Actual `ExitCoordinator`, native `NativeRegistry` concrete hidden-window
  binding/resolution and targeted Tauri JS event delivery. Every participant
  registers **and publishes** initial facts. Native snapshot/freeze challenges
  return through independent window JS listeners and concrete native callers.
- Actual `start_stack_terminal_session` creates both `terminal-panel` and
  `stack-popup` sessions. The test probe duplicates their exact owning ConPTY
  child handles before any drain; no PID-reuse assumption drives exit checks.
  Production `stop_all_owned_terminal_sessions` must reap both handles and
  empty active/draining registry slots before its receipt is accepted.
- Actual `spawn_quick_command` creates two fixture-owned job roots, each with
  a child and grandchild. Six retained observation handles must signal exit.
  One root deliberately exits normally while descendants are alive, exercising
  the real production root-exit worker's owned-job descendant cleanup.
- That real worker's injectable `on_exit` callback is held at a channel barrier.
  The first actual command drain therefore times out and retains exactly one
  job/worker. Coordinator stays Failed/fenced and final exit remains denied.
  Releasing the callback and retrying joins/reaps the retained worker, leaving
  zero owned command slots without repeating completed terminal/probe lanes.
- Actual `execute_command_observed` runs a fixture helper in an owned job,
  tracked in the real `SystemInfoRuntime` worker ledger using its real stop
  flag. Production `stop_and_drain` must cancel it, observe
  `Spawned → KillRequested → BeforeWait → Reaped`, signal its retained process
  handle and join its worker before completing.
- A separately owned independent PowerShell control process remains alive
  through dirty rejection, partial drain and successful retry. Only the
  fixture guard kills/waits that exact child during final fixture cleanup.
- Actual authorized Stack create/delete/Git branch and Settings bar lock/height
  mutation calls exercise the native admission boundary, not source matching.
  Running baseline operations succeed. Frozen calls must reject and leave exact
  fixture filesystem, repository and canonical settings bytes unchanged.
- A real Git post-checkout hook blocks the production blocking worker. Native
  exit must report Busy both before and after the IPC future is dropped, until
  the actual worker finishes. Temporary hook barriers release on fixture unwind.

## Oracles

The dirty Stack round produces zero cleanup effects and all owned processes
remain running. After unanimous clean freeze, the deliberately blocked worker
produces a truthful retained failure. Retry completes only unresolved work.
There are two terminal handle exits, six Quick Command handle exits, one probe
handle exit, zero retained terminal/command ownership, one surviving independent
control, and exactly one final-dispatch receipt/consumable permit. The parent
requires a nonzero-stage `OWNED_EXIT_E2E=PASS` marker, successful child exit, and
successful exact temporary profile/script-directory removal.

Late snapshot ACKs from a different WebView can legitimately arrive after a
dirty participant aborts the round. The fixture verifies that such old-phase or
old-attempt ACKs are rejected; it does not falsely classify safe rejection as a
transport failure or relax rejection. Wrong-target/malformed/native-identity
errors still fail the fixture.

## Evidence and limits

Initial compilation found a fixture enum typo (`powerShell` instead of native
`powershell`) and a fixture event-name type mismatch; corrected in tests only.
Production's missing `Manager` import was reported to/fixed by the native owner.
One earlier run reported a generic ACK-ledger failure; the fixture now diagnoses
it and explicitly verifies safe late-phase rejection. A subsequent actual run
passed all owned-resource oracles (child 5.22s, parent 5.46s).

The expanded admission regression then compiled and ran against intentionally
unpatched P1 production, failing after 7.56s with seven actual violations:
missing Busy reservations before and after dropping the still-running Git IPC
future, plus frozen create-folder, delete-file, Git-branch, settings-lock and
settings-height mutations. All five changed isolated fixture state. This is
decisive RED, not a setup or sourcegrep failure. After the native owner's P1
patch, the exact command passed (child 7.09s, parent 7.39s). Its markers report
`NATIVE_MUTATION_ADMISSION=PASS`, zero frozen FS/Git/settings writes, and actual
worker reservation retained after dropped IPC, followed by `OWNED_EXIT_E2E=PASS`
for all terminal/QC/probe/retry/survivor/zero-dirty-effect/final-dispatch oracles.
Final package validation subsequently passed all six stages (aggregate exit
zero), including 30 Cargo targets / 1,503 passes / zero failures / 54 explicit
ignored cases. Independent native, test/import and bounded speech source/test
reviews are CLEAN. The later speech-only serialization fix does not change the
terminal/QC/probe/admission paths this fixture certifies; speech capture remains
uninstalled here and is not claimed as live runtime evidence.

The fixture effects call the real process/probe drains above. Other lanes are
explicit empty-owner fixture receipts because those owners are never installed.
It **does not certify** real AppBar restoration, hotkey/hook retirement, taskbar
tracking, folder watcher, speech, snipping/clipboard/picker disposal, active
System Info UI/probe-budget ACL, or the production final `app.exit` adapter.
Passing fake-effect coordinator tests likewise does not certify those lanes.
Those need their own focused production-owner tests/source review; hardware and
installed-shell validation remain consent-gated.
