# JasonShell Test Coverage Gap Audit

## Executive summary

`TEST_CASE_AUDIT.md` reports a large inventory: 124 Node test files, 77 Rust files containing tests, three smoke/test scripts, and 1,620 discovered entries. Audit artifact does not provide counting command or split that total into Node declarations, runtime-expanded cases, Rust tests, ignored tests, and scripts. Treat 1,620 as inventory estimate, not reproducible behavioral-test count.

Current suite is strongest in:

- Pure TypeScript state and transformation helpers.
- Rust state machines, validation, parsers, bounded-output behavior, and temporary filesystem/repository tests.
- Static enforcement of IPC names, capability declarations, security-sensitive implementation patterns, UI structure, and documentation policy.
- Focused native tests such as Quick Command Job Object descendant termination, ConPTY shell startup, and taskbar-attention hook smoke coverage.

Current suite is weakest at seams where JasonShell's highest risks live:

- Mounted Svelte component behavior and browser event ordering.
- Real Tauri command invocation and cross-webview event delivery.
- Real Win32 shell effects, including AppBar reservation, taskbar ownership, focus, activation, minimization, and cleanup.
- Crash/restart persistence and recovery.
- Concurrent filesystem, Git, terminal, and process operations.
- Automated accessibility, release performance, DPI, and multi-monitor validation.

Overall assessment: **strong foundation, fragile integration confidence**. More test entries are not first priority. Highest return comes from a thin executable integration layer across Tauri/WebView/Win32 boundaries, then consolidation of source-text tests.

## Scope and method

Reviewed:

- `TEST_CASE_AUDIT.md`
- `master_spec.md`
- Node test organization under `tests/*.test.mjs`
- Rust tests under `src-tauri/src/`
- Test scripts and package commands
- Representative assertions from high-risk and high-volume suites

Review snapshot: 2026-09-10 at Git `7d5d80b`, plus uncommitted workspace state visible during review. Sampling emphasized source-contract-heavy Node suites and highest-risk Rust/native modules. No prior pass result or coverage artifact was used as proof that tests currently pass.

This report evaluates what tests prove, not only what their names claim. Tests are classified by execution layer:

1. Behavioral unit: executes imported production logic.
2. Host integration: executes real local resources such as temp files, Git repositories, subprocesses, or ConPTY.
3. Component/runtime integration: mounts UI or invokes real Tauri/WebView flows.
4. Native OS integration: verifies observable Win32 effects.
5. Source contract: reads source and checks text, regex, ordering, or declarations.
6. Governance: validates docs, config, policy, or generated artifact hygiene.
7. Manual/smoke: requires environment or human observation.

No coverage report was available in the audit artifact, so this is behavioral and structural analysis rather than line/branch percentage analysis.

## Current coverage profile

| Layer | Current strength | What it proves | Main limitation |
|---|---:|---|---|
| TypeScript behavioral unit | Strong | Pure state transitions, sorting, ranking, normalization, stale-response handling | Does not prove Svelte wiring or browser behavior |
| Rust unit/state machine | Strong | Validation, transition rules, parsers, rollback planning, bounded data | Native calls often replaced by closures or synthetic handles |
| Local host integration | Moderate to strong in selected modules | Real temp filesystem/Git operations, subprocesses, ConPTY startup, and Job Object descendant termination | Uneven across subsystems; limited concurrency, failure injection, and environment matrix |
| Source contract | Very strong in volume | Symbols, command names, declared wiring, forbidden patterns, rough ordering | Can pass when code is dead, disconnected, or behaviorally wrong |
| Component/DOM integration | Very weak | Little direct evidence | No meaningful mounted-Svelte interaction layer |
| Tauri multi-window integration | Very weak | Little direct evidence | Contract parity does not prove delivery, focus, timing, or authorization ordering |
| Native Windows integration | Weak and narrow | Attention hook smoke and some real native resource lifecycles | AppBar, task lifecycle, Explorer restart, cleanup, DPI, multi-monitor remain thin |
| Persistence/restart recovery | Weak | Unit-level coercion, corruption handling, journal logic | Few real crash/interrupted-write/restart scenarios |
| Accessibility | Weak to moderate | Static ARIA/source contracts and pure keyboard state helpers | No automated accessibility engine or full keyboard/screen-reader journeys |
| Performance | Harness-heavy, acceptance weak | Harness schema, process control, metric collection rules | Release acceptance remains blocked for unmeasured scenarios |

## What is tested well

### Pure state and data behavior

Several suites execute production helpers with representative inputs, exact outputs, boundaries, and stale-state cases. These are comparatively robust:

- `tests/searchUxState.test.mjs`
- `tests/searchRanking.test.mjs`
- `tests/terminalWorkbenchState.test.mjs`
- `tests/processManagerState.test.mjs`
- `tests/processManagerUxState.test.mjs`
- `tests/stackPopupState.test.mjs`
- `tests/taskbarGroups.test.mjs`
- Behavioral portions of `tests/quickCommands.test.mjs`

These tests generally survive internal refactors because assertions target outputs and transitions rather than source formatting.

### Rust state machines and validation

Native modules have substantial failure-path coverage around planned transitions and validation. Examples include AppBar rollback/concurrency, task-window identity rules, output bounds, safe Git argv, path containment, paging, corruption recovery, terminal target validation, and Quick Command stop races.

Notable stronger integration examples:

- `src-tauri/src/quick_commands.rs`: Windows Job Object test proving nested descendant termination.
- `src-tauri/src/stack_popup/terminal.rs`: real PowerShell/cmd ConPTY startup smoke tests.
- `src-tauri/src/stack_popup/process_runner.rs`: real output-cap, timeout, and nonzero-exit subprocess tests.
- `src-tauri/src/stack_popup/git_status.rs`: real temporary Git repository tests.
- `scripts/smoke-taskbar-attention.ps1`: real executable + fixture-window attention-hook test.

### Defensive source contracts

Source tests provide useful architecture guardrails for:

- Centralized IPC/event names.
- Capability and surface parity.
- Shell-free fixed argv construction.
- Blocking-boundary placement.
- Secret-like settings rejection.
- Removed legacy pathways staying removed.
- Required ARIA and UI primitives.

These tests should remain as defense in depth where the constraint itself matters. They should not be counted as substitutes for executable behavior.

## Fragility findings

### P0: source-text checks frequently stand in for runtime behavior

Many Node tests read production files and use regex/string assertions. Examples:

- `tests/stackPopupPagingPhase1Wiring.test.mjs`
- `tests/stackPopupPagingPhase3Wiring.test.mjs`
- `tests/shellOpenCloseEvents.test.mjs`
- `tests/taskbarGalleryContract.test.mjs`
- `tests/persistentSurfaceLifecycle.test.mjs`
- `tests/centeredSearchSurface.test.mjs`
- `tests/stackGitPanelRedContract.test.mjs`

These checks can prove that names or snippets exist. They cannot prove that:

- Code is reachable.
- Correct handler calls it.
- Producer and consumer payloads agree at runtime.
- Guard runs before side effect.
- Pointer, keyboard, focus, timer, or lifecycle ordering works.
- Failure leaves observable state unchanged.

Two opposite risks result:

- False confidence: dead or disconnected code still passes.
- Refactor friction: harmless rename, extraction, markup change, or reordered declaration fails.

Recommendation: label these tests explicitly as `source-contract`; exclude them from behavioral coverage claims; replace highest-risk examples with executable integration tests.

### P0: no meaningful mounted-Svelte component layer

Current package test path compiles helpers and runs Node tests, but does not provide broad DOM/component mounting. Consequently, much UI behavior is inferred from source.

Weakly proven behaviors include:

- Mount/unmount and late async listener cleanup.
- Focus placement/restoration and real tab order.
- Pointer capture, drag thresholds, click suppression, and autoscroll.
- Blur/pointerdown/trailing-click races.
- `ResizeObserver`, CSS layout, virtual scrolling, and computed clipping.
- Real accessible names, roles, states, and keyboard activation.
- xterm mount/dispose/reflow behavior.

Example: `tests/persistentSurfaceLifecycle.test.mjs` contains a test-local async unlistener registry model. That proves copied test logic, not production component behavior.

Recommendation: add a small Svelte component test stack and target only high-risk flows first. Avoid attempting to convert all source tests at once.

### P0: real Tauri cross-window behavior is thin

Contract parity tests strongly validate names and declarations, but do not prove real multi-window operation:

- Correct target receives event.
- Unauthorized target does not receive/invoke it.
- Window is shown, focused, hidden, or closed in correct order.
- Stale nonce/request/event is ignored after actual delivery delay.
- Guard rejection occurs before filesystem/process/native side effect.

Highest-risk flows:

- Top bar to search panel open/query/close cycle.
- Quick Launch nonce + blur + native context-menu hold.
- Terminal panel open/prewarm/output routing.
- Stack popup open/close/focus-loss behavior.
- Persistent surface listener disposal.

Recommendation: create release-binary Tauri integration harness with controlled fixture windows and artifact output.

### P0: automated native shell behavior remains thin

`src-tauri/src/appbar.rs` has strong injected state-machine tests, but fake closures and synthetic handles do not prove actual calls to `SHAppBarMessage`, `SPI_SETWORKAREA`, window style changes, or Explorer restoration.

`scripts/smoke-fullscreen-appbar.ps1` does exercise real fullscreen hide/restore, repeated restoration, painted content, and bottom-edge geometry through interactive observation. This is useful native evidence, but this audit did not establish a fresh passing artifact, and script is not an automated CI assertion of AppBar registration or cleanup.

Likewise, task-window tests cover planning and identity predicates better than real focus/minimize/close effects.

Missing live matrix:

- AppBar register/query/set/remove.
- Top/bottom reservation geometry and resizing.
- Fullscreen release/park/restore.
- Explorer restart and recreated taskbar ownership.
- Graceful exit and forced-crash cleanup.
- `WS_EX_TOOLWINDOW` / Alt+Tab exclusion.
- Foreground denial and `AttachThreadInput` fallback.
- Minimized restore and owned modal activation.
- Hung close, delayed close, PID/HWND reuse, UAC cancel, access denial.

Recommendation: add consent-gated Windows integration fixture. Verify external observable state, not internal logs alone.

### P1: security assertions are often lexical

Security source tests correctly enforce patterns, but many do not execute hostile inputs. Examples include command authorization, blocking boundaries, fixed argv construction, and destructive confirmation wiring.

Specific gap: `src-tauri/src/stack_popup/auth.rs` central caller-label policy lacks a direct exhaustive unit matrix. Existing source checks can miss a command mapped to wrong policy or a guard placed after side effects.

Recommendation:

- Table-test existing `StackCommandAuth` policy and `STACK_GUARDED_COMMANDS` registry against every shipped surface.
- Prove registry completeness against registered commands and capabilities.
- Add representative real Tauri invokes from authorized and unauthorized windows.
- Assert rejected calls create no filesystem, Git, process, or terminal side effects.

### P1: some assertions are too weak for test titles

Example: `tests/stackPopupPagingPhase2Wiring.test.mjs` constrains relative page sizes but could accept unusable values such as initial `0`, subsequent `1`.

Recommendation: assert supported value ranges and execute pagination over empty, boundary, multi-page, and stale-page scenarios. Test invariants, not only constants.

### P1: phase-named tests duplicate historical implementation

Examples:

- Six `stackPopupPagingPhase*` suites.
- `backendBlockingLockBoundaries.test.mjs` plus `backendBlockingLockBoundariesP6.test.mjs`.
- `searchOverhaulPhase0.test.mjs`, `searchTypingFreezePhase1.test.mjs`, `searchOverhaulPhase6.test.mjs`.
- `quickIconsPhase3.test.mjs`.
- `settingsPowerActionsPhase5.test.mjs`.
- `vscodeFolderOpenPhase4.test.mjs`.

This accumulates project chronology in executable tests. Same implementation is checked repeatedly through old names and source shapes.

Recommendation: consolidate by current subsystem and current contract. Preserve phase history in `changelog.md`, not suite names/assertions.

### P1: conditional integration tests can silently become no-ops

Some Git integration tests return early when Git is unavailable or when repository initialization/commit setup fails. CI can stay green without running intended scenario.

Recommendation:

- On supported Windows CI, fail if required prerequisite is absent.
- Else emit explicit skipped-test result with reason.
- Never use silent early return for required release evidence.

### P1: concurrency and interruption coverage is uneven

State-machine concurrency tests are strong in selected modules, but real-resource races remain sparse.

Missing examples:

- Paste/delete/rename on same tree concurrently.
- Destination/source changing after validation.
- Sharing violations, ACL denial, disk/quota exhaustion, long paths, junction loops, UNC disconnect.
- Git index lock, repository movement, push rejection, credential prompt, network timeout.
- Concurrent PTY write/read/resize/stop and shell exit during operation.
- Generic process-runner descendant survival after timeout.

Recommendation: add injectable barriers/fault seams for deterministic race tests; supplement with smaller Windows integration matrix.

### P0: persistence tests do not fully prove crash recovery

Current tests cover coercion, corrupt-file backup, journal logic, migration helpers, and rollback decisions. They do not broadly prove process restart after interrupted writes.

High-value scenarios:

- Kill app during write.
- Temp file exists but replace did not complete.
- Corrupt current file plus valid backup.
- Old schema from real fixture profile.
- Migration persistence failure restores exact prior state.
- Recovery journal replay is idempotent across repeated restart.

Recommendation: run release binary against isolated app-data directories; terminate at deterministic fault points; restart and assert recovered visible/backend state.

### P1: accessibility coverage is mostly static

Current suite includes focused process-manager accessibility contracts, ARIA/role/focus-style checks, and pure keyboard-state helpers. `axe-core` may exist transitively, but package test config has no mounted axe execution. Missing:

- Automated axe-like checks against mounted surfaces.
- Keyboard-only journeys across windows and native menus.
- Focus return after close/error/destructive confirmation.
- Screen-reader smoke on search, taskbar, process manager, settings, and terminal.
- Validation of terminal decision to disable xterm accessibility mirror nodes.

Recommendation: add mounted accessibility checks plus a small manual NVDA checklist stored as release evidence.

### P1: performance harness quality exceeds performance evidence

`tests/performanceBaselineContract.test.mjs` thoroughly checks harness structure. It does not prove product stays within release budgets. Canonical spec records blocked complex/manual scenarios and unclosed release acceptance.

Recommendation: separate harness correctness from measured result. CI/reporting must say `not measured` rather than pass when required scenarios block.

### P2: governance tests inflate product test totals

Examples:

- `tests/productTruthDocs.test.mjs`
- `tests/changelogPolicy.test.mjs`
- `tests/changelogPolicyHygiene.test.mjs`
- `tests/masterSpecSearchHygiene.test.mjs`
- `tests/multiFixPhase6SpecValidation.test.mjs`
- `tests/distTestsHygiene.test.mjs`
- `tests/testModernizationPolicy.test.mjs`

These can be useful, but they validate repository governance or tests themselves. They should appear in a separate metric from product behavior.

## Risk-based gap matrix

| Product area | Existing confidence | Main unproven behavior | Priority |
|---|---|---|---:|
| AppBar/fullscreen/Explorer suppression | Strong state logic, manual smoke | Real registration, Explorer restart, crash cleanup, multi-monitor | P0 |
| Taskbar/window management | Strong model/identity logic, attention smoke | Real activation, minimize, close, foreground denial, UAC | P0 |
| Cross-window surfaces/events | Strong contract parity | Delivery, focus, timing, stale-event rejection | P0 |
| Terminal/ConPTY | Strong helper/state tests, basic real ConPTY smoke | Hidden/open race, real xterm rendering, concurrent lifecycle, cleanup | P0 |
| Persistence/recovery | Good helper-level handling | Interrupted writes and restart recovery across stores | P0 |
| Performance | Strong harness contract | Complete release-mode acceptance measurements | P0 |
| Search | Strong pure state/ranking/race logic | Real typing/focus/render timing | P1 |
| Stack Browser filesystem | Good temp-filesystem tests | OS dialogs, drag/drop, sharing/ACL/UNC/race failures | P1 |
| Stack Git | Good validation/parser/temp-repo tests | Remotes, auth, locks, concurrent ops, destructive worktree recovery | P1 |
| Quick Commands | Strong state/backend process tests | Elevated/breakaway variants and live prompt interaction | P1 |
| Quick Launch | Strong source/state contracts | Real ShellExecute, AppX, UAC/admin and focus hold | P1 |
| Security/authorization | Good deny-pattern and validation tests | Exhaustive policy matrix + no-side-effect runtime proof | P1 |
| Accessibility | Static contracts | Mounted scans, keyboard journeys, screen reader | P1 |
| Tray/audio/calendar/settings | Useful contracts and helpers | Native-menu/device/focus edge cases | P2 |
| Visual/DPI behavior | CSS/source checks | Screenshot geometry across DPI, zoom, monitors | P2 |
| Long-run resource behavior | Selected cleanup tests | Listener/polling/handle soak and leak detection | P2 |

## Recommended roadmap

### Phase 1: classify test evidence

Goal: stop treating every discovered entry as equal evidence.

- Tag or group suites as `unit`, `host-integration`, `component`, `tauri-integration`, `native-windows`, `source-contract`, `governance`, or `manual`.
- Report counts and pass/skip/block status per layer.
- Require explicit skip reasons.
- Keep `TEST_CASE_AUDIT.md` as inventory, but add layer and confidence fields.
- Do not use total test entry count as primary quality metric.

Exit criterion: CI summary states which runtime layers executed and which remain blocked.

### Phase 2: add six executable seam tests

Highest-value initial scenarios:

1. Search rapid typing, stale response rejection, Escape/blur, focus restoration.
2. Quick Launch pointerdown/blur/trailing-click and nonce lifecycle.
3. Persistent surface destroy before async listener resolution.
4. Stack paging/icon hydration with stale concurrent responses.
5. Caller-label rejection before a representative filesystem side effect.
6. Terminal hidden prewarm/open/resize/output-target lifecycle.

Split ownership and runners:

- `test:component`: mounted Svelte tests with mocked Tauri APIs for browser event/focus/lifecycle mechanics.
- `test:tauri-integration`: packaged fixture app for real window labels, `emitTo`, invoke authorization, and focus/show/hide behavior.
- `test:windows-integration`: designated Windows VM for ConPTY/native effects.

Each runner must define isolated temp/app-data roots, per-scenario timeout, process-tree teardown, structured pass/fail/skip/block JSON, and retained logs/screenshots on failure.

Exit criterion: each scenario fails when production wiring is disconnected, not merely renamed.

### Phase 3: build Windows native fixture pack

- Fixture windows: normal, minimized, owned modal, hung-close, delayed-close, attention-requesting.
- External probes: foreground HWND, iconic state, window existence, styles, monitor/work-area rects.
- Scenarios: AppBar reserve/resize/release/restore, Explorer restart, graceful exit, forced crash, task activation/minimize/close.
- Record release binary hash, OS build, DPI, monitors, HWND/PID, timestamps, and screenshots/logs.
- Run Explorer restart, forced crash, ACL/UAC, AppBar mutation, and destructive worktree scenarios only in disposable Windows VM/user profile.
- Capture baseline taskbar/work-area/Explorer state before execution; guarantee teardown and restoration attempts in `finally`; mark environment unsafe and stop remaining scenarios if restoration fails.
- Define admin policy explicitly; never elevate whole suite implicitly.

Exit criterion: repeatable automated pass on designated Windows runner; destructive/environment-sensitive cases explicit opt-in.

### Phase 4: persistence and subsystem integration

- Isolated app-data crash/restart matrix.
- Local bare Git remote for fetch/pull/push rejection and worktree operations.
- Filesystem lock/ACL/long-path/junction fixtures.
- Concurrent ConPTY and subprocess cleanup tests.

Exit criterion: each critical store and external-resource subsystem has success, rejection, partial failure, interruption, and retry coverage.

### Phase 5: consolidate and harden

- Merge phase suites into current subsystem suites.
- Remove duplicated regex checks once executable replacement exists.
- Replace copied test models with imported production helpers.
- Prefer AST/parity generation over broad regex when source contracts remain necessary.
- Add targeted mutation testing for authorization, stale-state guards, path containment, and destructive actions.

Exit criterion: lower source-format coupling; equivalent refactors no longer trigger broad unrelated failures.

### Phase 6: accessibility, performance, and environment matrix

- Mounted automated accessibility checks.
- Keyboard-only multi-window journeys.
- NVDA release checklist for critical surfaces.
- Release performance scenarios with explicit measured/blocked status.
- DPI/zoom/multi-monitor screenshot and geometry matrix.
- Long-running listener, polling, process, PTY, and handle soak tests.

Exit criterion: release decision includes functional, accessibility, performance, and environment evidence.

## Recommended suite policy

### Test naming

Name test after observable contract, not project phase or implementation mechanism.

Prefer:

`search ignores result from an older request after query changes`

Avoid:

`phase 6 keeps latestRequestId wiring`

### Source-contract rules

Use source tests only when validating a source-level invariant, such as:

- Forbidden shell execution API.
- Command/event registry parity.
- Capability membership.
- Required compile-time declaration.
- Removal of prohibited legacy path.

Do not describe source-text checks as proof of user-visible behavior.

### Assertion rules

- Assert exact externally visible state where practical.
- Include negative/no-side-effect assertions for rejected operations.
- Test boundary values, not only relative constants.
- Avoid broad `.*` or `[\s\S]*` across function boundaries.
- Avoid copying production logic into test-local models.
- Make skipped prerequisites visible.

### Coverage gates

Line coverage alone is insufficient for this repository. Recommended gates:

- Branch coverage on pure TypeScript and Rust logic.
- Required scenario pass list for Tauri/native integration.
- Zero silent skips in required Windows CI jobs.
- Mutation score on authorization, validation, stale-response, and destructive-operation modules.
- Explicit manual evidence for scenarios that cannot yet be automated.

## First backlog

| Order | Work item | Why first |
|---:|---|---|
| 1 | Exhaustive stack command/caller authorization matrix | High security value, relatively low implementation cost |
| 2 | Add test-layer classification/reporting | Makes current confidence honest; can run alongside item 1 |
| 3 | Mounted persistent-listener lifecycle test | Replaces copied model; catches leak/state-after-destroy bug |
| 4 | Mounted search stale-response/focus test | High-use UI with known async complexity |
| 5 | Real Tauri targeted-event smoke | Validates central architecture seam |
| 6 | AppBar/Explorer cleanup integration | Highest native shell failure impact |
| 7 | Task-window fixture matrix | Validates activation/minimize/close behavior users depend on |
| 8 | Crash/restart persistence matrix | Protects settings, pins, ordering, indexes, journals |
| 9 | Git lock/remote/worktree integration | Protects destructive and concurrent developer workflow |
| 10 | Terminal lifecycle integration | Protects PTY processes, routing, resize, and cleanup |

## Conclusion

JasonShell does not primarily need more static wiring tests. It needs stronger evidence at boundaries between already well-tested units.

Best next investment:

1. Separate behavioral, source-contract, governance, integration, and manual metrics.
2. Add a small mounted-Svelte layer.
3. Add a small real-Tauri multi-window layer.
4. Automate highest-risk Win32 and crash-recovery scenarios on a designated Windows runner.
5. Consolidate historical phase suites after executable replacements exist.

This preserves current useful coverage while shifting future effort toward failures current suite is least able to detect.
