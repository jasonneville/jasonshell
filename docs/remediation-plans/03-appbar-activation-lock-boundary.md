# 03 AppBar Activation Lock Boundary

## Metadata

- Status: Implementation-ready plan, requires careful Rust refactor.
- Order: 03 of 13.
- Owner: Native shell/AppBar implementer.
- Dependencies: Plans 01-02 MUST be complete so regressions are distinguishable from baseline failures.
- Related audit finding: P0-1 AppBar activation holds global shell mutex across slow side effects.

## Objective and evidence

Objective: refactor AppBar activation so `ShellRuntimeState` mutex is held only for short planning/state-transition/commit/rollback windows, not across Win32/Tauri calls, while preserving recovery, fullscreen guard, Explorer taskbar suppression, AppBar reservation, and cleanup semantics.

Evidence:

- Audit lines 118-129 identifies lock held across `src-tauri/src/appbar.rs:214-323`.
- Source shows state lock acquired at activation line 214 and retained while calling `get_work_area`, `explorer::all_taskbar_snapshots`, taskbar hide/guard start, AppBar reserve, `set_work_area`, `move_window_to_rect`, `windows.top.show`, styling, stabilization, and fullscreen guard start.
- `master_spec.md` AppBar section requires recoverable reserved/released/parked states, fullscreen guard retries, Explorer taskbar handling, resize rejection outside reserved state, and cleanup restoration.

## Scope

Exact files/symbols likely in scope:

- `src-tauri/src/appbar.rs`
- `ShellRuntimeState`
- `activate_shell_runtime` or equivalent activation function containing lines 214-323
- `cleanup_runtime_state`
- `resize_shell_bar` path if it relies on activation state
- `start_taskbar_guard`, `start_taskbar_guard_v2`, `start_fullscreen_guard`
- `register_tracked_appbar`, `reserve_appbar`, `set_work_area_with_retry_or_warn`, `stabilize_runtime_window_rect`
- Existing tests: `tests/backendBlockingLockBoundaries.test.mjs`, `tests/backendBlockingLockBoundariesP6.test.mjs`, `tests/explorerTaskbarSuppression.test.mjs`, Rust unit tests in `appbar.rs` if present

Explicit out of scope:

- Changing AppBar visible geometry, default bar heights, fullscreen policy, Explorer taskbar ownership semantics, or multi-monitor runtime ownership.
- Live shell startup smoke unless user explicitly consents.
- Rewriting all AppBar code; goal is lock boundary and state coherency.
- Persistent top JSON/shell terminal and Stack Browser Git workbench feature work.

## Current-state contract

- Top/bottom shell bars reserve primary monitor work area, hide/guard Explorer taskbar only when owned, and restore on cleanup.
- Fullscreen guard can release/park/restore AppBars and must remain retryable.
- Bar resize is rejected outside reserved state.
- Activation failure hides JasonShell bars and runs cleanup rollback.
- Current defect: one mutex guard spans long native calls, blocking cleanup/resize/recovery and risking deadlock.

## Requirements

### Functional

1. Activation MUST NOT hold `ShellRuntimeState` mutex across Win32 calls, Tauri window calls, sleeps/retries, filesystem/process enumeration, or guard thread startup side effects.
2. Activation MUST mark an explicit activation-in-progress state before unlocked side effects begin.
3. Concurrent activation attempts MUST fail fast or join/observe the in-progress state without duplicate AppBar registration or duplicate guard startup.
4. Cleanup during activation MUST remain coherent: it MUST either wait for a bounded safe point or request cancellation/rollback without corrupting owned taskbar/AppBar state.
5. Successful activation MUST commit `baseline_work_area`, hidden taskbar ownership, registered AppBars, shell layout, and fullscreen state under a short lock.
6. Failed activation MUST rollback all side effects already performed and leave state cleaned or retryable.
7. Existing AppBar rectangles, work-area calculations, top/bottom show/style behavior, and fullscreen guard behavior MUST remain compatible.
8. Resize commands MUST continue to reject unless runtime is in committed reserved state.

### Nonfunctional

9. New tests MUST prove lock wait is bounded under injected slow native operations.
10. Refactor SHOULD introduce small plan/result structs rather than passing a mutable mutex guard through side-effect code.
11. Logging MUST avoid noisy repeated failure spam; preserve existing first-failure/backoff style.
12. Unsafe Win32 handling MUST not expand without tests/review.

## Decisions and implementation approach

- Recommended model: `ActivationPlan` built from immutable inputs; `ActivationSideEffects` tracks owned handles/snapshots/rects; `ActivationCommit` applied under lock.
- Add enum state, recommended: `ShellActivationPhase::{Idle, Activating { generation }, Reserved, CleaningUp}` or integrate with existing fields. Stop/escalate if broad state-machine rewrite becomes necessary.
- Start guard threads only after side effects succeed, but store guard ownership under lock immediately after creation if thread needs shared state.
- If cleanup can race activation, recommended default: cleanup sets cancellation generation and returns controlled error if activation has not reached cancellable checkpoint. Escalate if product owner wants blocking cleanup.

## Phases with RED-first tests

1. RED source-contract test: add/extend test proving activation does not pass `&mut state` or hold lock through side-effect calls. Expected fail now.
2. RED Rust unit with injected slow side effect: if AppBar code is injectable, simulate slow taskbar enumeration/reserve and assert `try_lock`/resize-state read completes within recommended 50 ms. Expected fail now.
3. Refactor activation into short-lock plan/start, unlocked side effects, short-lock commit.
4. Add rollback guard/drop helper for partial side effects.
5. Refactor cleanup/resize to understand activation-in-progress.
6. GREEN focused Rust/source tests.
7. Run full Rust/Node validation as feasible.
8. Optional manual smoke only with explicit consent: `npm run smoke:fullscreen` or app startup.

## Exact tests and assertions

- Add Rust test in `appbar.rs`: `activation_planning_releases_state_lock_before_native_side_effects` asserts injected slow side-effect does not block independent state lock acquisition.
- Add Rust test: `activation_failure_rolls_back_partial_appbar_and_taskbar_state` asserts failure after top reservation unregisters/restores and leaves retryable state.
- Add Rust test: `resize_rejects_while_activation_in_progress` asserts no resize mutation during in-progress activation.
- Add/extend Node source test `tests/backendBlockingLockBoundaries.test.mjs`: `appbarActivationDoesNotHoldShellRuntimeStateAcrossWindowOrWin32Calls` asserts no long block lives inside mutex guard.
- Existing `tests/explorerTaskbarSuppression.test.mjs`: ensure ownership semantics unchanged.

## Edge and failure cases

- Explorer taskbar hide succeeds, AppBar reserve fails -> taskbar restored or guard-owned state cleaned.
- Top AppBar reserve succeeds, bottom reserve fails -> top unregistered, work area restored.
- Tauri `show()` fails after reservation -> windows hidden if visible, AppBars unregistered, state retryable.
- Cleanup arrives during activation -> no double restore/unregister.
- Fullscreen guard startup fails or duplicates -> activation must not leave duplicate worker loops.

## Data/API/event compatibility impacts

- No public Tauri command payload changes expected.
- Internal state enum/fields may change.
- AppBar event/log timing may change but visible behavior should not.

## Validation commands

Focused:

```bash
cargo test --manifest-path src-tauri/Cargo.toml appbar -- --nocapture
node scripts/clean-dist-tests.mjs && tsc -p tsconfig.test.json && node --test tests/backendBlockingLockBoundaries.test.mjs tests/explorerTaskbarSuppression.test.mjs
```

Full:

```bash
npm run cargo:test
npm run cargo:check
npm run test:node
npm run validate
```

Manual only with consent:

```bash
npm run smoke:fullscreen
```

## Acceptance criteria

- Given injected slow native operation, when activation runs, then unrelated state lock acquisition/resize rejection completes within bounded time. Covers req 1, 8-9.
- Given concurrent activation, when second activation starts, then no duplicate AppBar registration or guard starts occur. Covers req 2-3.
- Given failure at each side-effect checkpoint, when activation returns error, then cleanup restores taskbar/work-area/AppBars and retry can start. Covers req 4-6.
- Given normal activation, when committed, then layout/fullscreen/taskbar state equals previous visible contract. Covers req 5, 7.

## Risks and rollback

- Risk: partial rollback misses Explorer taskbar ownership. Rollback: revert refactor and keep plan branch until rollback tests complete.
- Risk: cleanup/activation race causes double unregister. Rollback: add generation ownership and idempotent side-effect handles.
- Risk: source-contract tests too brittle. Rollback: keep Rust injectable behavior tests as authority.

## Master spec, changelog, docs updates

- Update AppBar section in `master_spec.md` to state activation lock boundary and activation-in-progress behavior.
- Add known limitation if cleanup blocks or returns controlled busy error during activation.
- Append `changelog.md` entries for behavior/tests/validation.

## Handoff checklist

- [ ] Read docs/source/tests.
- [ ] Added RED lock-boundary tests before refactor.
- [ ] Refactored to plan/effects/commit with short locks.
- [ ] Verified rollback checkpoints.
- [ ] Ran focused tests and full feasible validation.
- [ ] Updated durable docs.
- [ ] Adversarial QA for races/deadlocks/desktop mutation leaks.

## Copy/Paste Implementation Prompt

```text
Implement remediation plan docs/remediation-plans/03-appbar-activation-lock-boundary.md. First read master_spec.md, docs/current-state-technical-audit-2026-08-28.md, CHANGELOG_POLICY.md, package.json, and this plan. Preserve unrelated worktree changes. Use RED-first: add failing tests that prove AppBar activation holds ShellRuntimeState too long and rollback/race behavior is unsafe, then refactor src-tauri/src/appbar.rs so native/Tauri side effects occur outside the mutex with short planning/commit/rollback locks. Do not change visible AppBar geometry, fullscreen policy, Explorer taskbar ownership semantics, or multi-monitor scope. Do not implement persistent top JSON/shell terminal or Stack Browser Git workbench feature work. Run focused Rust/Node tests listed in the plan, then npm run cargo:test/cargo:check/test:node/validate as feasible. Update master_spec.md AppBar contract and changelog.md per policy. Perform adversarial QA focused on races, deadlocks, duplicate guard startup, and partial desktop mutation rollback. Report files changed, commands run, pass/fail evidence, and unresolved blockers.
```
