# JasonShell Current-State Technical Audit

**Date:** 2026-08-28  
**Audit type:** Read-only source, contract, test, build, security, performance, accessibility, and product-completeness review  
**Verdict:** Technically ambitious and broadly effective prototype; not release-ready while validation is red and high-impact resource and synchronization risks remain.

## 1. Scope

Reviewed current JasonShell behavior and implementation across:

- Tauri command and capability boundaries
- Rust/Win32 lifecycle, process, filesystem, and shell integration
- Svelte surface behavior, lifecycle, keyboard access, and accessibility
- Search, Quick Launch, Quick Commands, Process Manager, Control Plane, tray, audio, calendar, workspaces, task previews, and shell bars
- Test design, test results, build output, documentation accuracy, and runtime evidence
- Product completeness, known limitations, performance exposure, and missing behavior

Explicitly excluded:

- Persistent top JSON/shell terminal
- Stack Browser Git menu/workbench

Those excluded areas were not used to determine feature quality. Shared Stack Browser infrastructure was reviewed where it affects non-Git behavior, security, or resource use.

No production code or behavior was changed during this audit.

## 2. Method and Confidence

Evidence sources:

1. `master_spec.md`, `README.md`, changelog, package scripts, and capability contracts
2. Current Svelte, TypeScript, Rust, CSS, and test source
3. Official clean test commands rather than direct tests against potentially stale `dist-tests`
4. Production frontend build output
5. Current Tauri and Svelte documentation for event cleanup, command behavior, and accessibility semantics
6. Independent architecture, frontend, security, Rust, validation, and adversarial review passes

Confidence labels used below:

- **Verified:** Directly established by source, compiler, test, or build evidence
- **Risk:** Source establishes exposure, but triggering severity depends on runtime conditions
- **Hypothesis:** Requires live Windows, assistive-technology, DPI, or performance testing

No live Tauri shell session was launched. Startup installs global hooks, reserves AppBars, changes work areas, and hides/restores Explorer taskbars. Doing that unattended would violate the read-only intent and could disrupt the active desktop. Browser-only rendering would not accurately reproduce Tauri window labels, capabilities, Win32 geometry, focus loss, or shell ownership. Runtime-only claims are therefore explicitly identified.

## 3. Executive Assessment

JasonShell has unusually strong architectural intent for a prototype. Persistent webview surfaces, narrow Tauri capabilities, renderer-to-native caller checks, path normalization, process identity handling in several subsystems, stale-response suppression, and documented lifecycle contracts show mature design work. Many issues from the May audit have been resolved.

Current quality is uneven at integration boundaries:

- Static frontend build succeeds, but Node and Rust test gates fail.
- AppBar activation holds the global shell-state lock across slow native operations.
- Stack paging limits response size but not discovery, sorting, cloning, or retained session memory.
- Process Manager can terminate a reused PID because kill-time identity is not immutable.
- Two long-lived surfaces do not satisfy the project's own async-listener disposal contract.
- Several keyboard and accessibility behaviors are incomplete or semantically incorrect.
- Workspace restoration, startup execution, automation forwarding, and multi-monitor runtime behavior remain intentionally incomplete.
- Runtime proof is much thinner than source-contract coverage.

Release decision: **block release** until validation is green and P0 safety/resource findings are fixed or explicitly accepted with measured evidence.

## 4. Current Capability Matrix

| Surface/domain | Effective behavior | Main limitation or risk |
|---|---|---|
| Top bar/AppBar | Native shell bar, launch controls, pins, search, resize, system integrations | Global state lock spans lengthy Win32/Tauri work; DPI crowding unverified |
| Bottom bar/tasks | Explorer pins, Quick Launch, task tiles, attention, previews, Process Manager | Attention CSS contract currently disagrees with test/spec |
| Quick Launch | Nonce-scoped native panel, allowed-path checks, launch and admin action | Admin action mouse-only; focus visibility intentionally weak |
| Search | Multiple providers, cached apps, Everything limits, provider health, latest-only gates | App-cache Rust test failing; live provider/focus smoke absent |
| Stack Browser files | File/archive browsing, operations, context menu, paging, icons, Open With | Full discovery/sort/session retention defeats bounded paging |
| Quick Commands | Saved commands, prompts, transcript, history, bounded output, stop | Forced tree kill and synchronous `taskkill`; listener lifecycle gap |
| Process Manager | Grouped process data, metrics, icons, guarded confirmation flow | PID-reuse race; noisy live grid; missing input focus style |
| Control Plane | Consolidated settings/workspace/provider views | Tab semantics do not match always-visible panels |
| Settings | Shell preferences, themes, power actions | No major new defect found in reviewed scope |
| Tray | Reload-on-open, scrollable icon grid, primary/secondary actions | Listener leak on rapid teardown; keyboard close/action parity missing |
| Audio | Device and volume controls with hidden-idle behavior | No major new defect found in reviewed scope |
| Calendar | Dedicated panel and close behavior | Live Windows geometry/focus behavior not exercised |
| Workspaces | CRUD, persisted metadata, activation planning, search bias, pins | Restoration and startup execution reserved, never performed |
| Automation | Parse/validation and plan contracts | Forwarding remains planned, not wired |
| Multi-monitor | Architecture/planning contracts | Runtime remains single-monitor |

## 5. Verified Strengths

### 5.1 Native and security boundaries

- Stack commands use caller authorization and normalized-path checks before sensitive operations.
- Quick Launch combines scoped events, nonces, and allowed-path authorization.
- Stack Open With uses an application-ID whitelist rather than renderer-provided executables.
- Task-window close paths have stronger immutable identity/elevation handling than conventional PID-only termination.
- Quick Command output and input are bounded in several important paths.
- Workspaces deliberately do not autoexecute startup commands, avoiding surprise local execution while capability remains incomplete.

### 5.2 Responsiveness and state handling

- Stack delete, paste, and archive operations have moved to blocking workers.
- Process icon cache locks use short scopes.
- Task-preview state avoids holding its mutex across window emission/show operations and guards stale generations.
- Search has latest-only response gates, cached app discovery, bounded Everything request sizes, and provider timing/health contracts.

### 5.3 UI improvements since prior audit

- Search and audio close routing is corrected.
- Tray content reloads when opened and can scroll internally.
- Process Manager header and rows share a scroller.
- Stack context menus clamp to the viewport and provide overflow scrolling.
- Core surfaces listed in the spec use stronger disposed-listener patterns.

### 5.4 Engineering process

- Canonical spec and changelog discipline is strong.
- Test inventory is broad: 744 Node tests and 560 Rust tests were discovered in current runs.
- Production frontend build succeeds with explicit chunks and type/accessibility analysis.
- Source-contract tests provide useful protection for command authorization and side-effect ordering where textual invariants are intentional.

## 6. Ranked Findings

### P0-1. AppBar activation holds global shell mutex across slow side effects

**Type:** Verified high reliability/performance risk  
**Evidence:** `src-tauri/src/appbar.rs:214-323`

`ShellRuntimeState` is locked near activation start. The guard remains held while code reads work areas, enumerates and hides Explorer taskbars, starts guards, registers and reserves AppBars, changes work areas, moves HWNDs, shows/styles Tauri windows, performs up to 15 100 ms stabilization attempts per bar, enforces final geometry, and starts the fullscreen guard. Cleanup and resize paths require the same mutex.

**Impact:** Slow or stalled Win32/Tauri calls block resize, cleanup, recovery, and other state users. Long UI stalls are directly credible. Deadlock is possible if a nested/callback path later needs the same state, but no deterministic deadlock was proven.

**Recommendation:** Split activation into a short locked planning/state-transition phase, unlocked native side effects, then short commit/rollback phases. Represent activation-in-progress explicitly so cleanup and retries remain coherent.

**Validation:** Inject slow/failing taskbar enumeration, AppBar reservation, and stabilization. Prove unrelated state operations remain bounded and partial failures roll back state and desktop mutations.

### P0-2. Stack paging does not bound discovery or retained memory

**Type:** Verified high performance/resource risk  
**Evidence:** `src-tauri/src/stack_popup/paging.rs:47-80,153-170,197-227,369-456`

Renderer `limit` is forced to at least one but has no upper clamp. First-page loading enumerates every folder or archive entry, sorts the full collection, clones it into a paging session, then returns a slice. Continuations clone retained session vectors. Up to 32 sessions are bounded only by count, not entries or bytes.

**Impact:** Very large directories or archives can cause long command latency, high CPU, allocation spikes, and memory amplification across sessions. A large renderer-provided limit further weakens the intended bound.

**Recommendation:** Clamp page size, cap scanned/archive entries, enforce a total session byte/entry budget, avoid full-vector clones, and define truncation behavior. If global sorting requires a complete snapshot, make that cost explicit and bounded rather than describing it as paging.

**Validation:** Use 100k-entry directory/archive fixtures, extreme `limit`, repeated continuation requests, and 32-session pressure. Assert bounded peak memory, latency, and deterministic truncation/eviction.

### P0-3. Validation gates are red and documented status is stale

**Type:** Verified release blocker  
**Evidence:** Official command results; `master_spec.md:30`

`npm run test:node` fails 11 tests. `npm run cargo:test` fails 3 tests. `master_spec.md` currently states only two Stack sorting failures, so canonical status no longer describes current validation.

Failures include command-contract drift, search cache behavior, VS Code candidate resolution, side-effect authorization contracts, Melt migration wiring, task-preview retention, and CSS/spec literal mismatches. Some may be stale source-contract expectations; others may expose behavioral regression. They must be triaged, not ignored as one class.

**Impact:** Current commit cannot establish its own claimed contracts. A passing build does not compensate for failing behavioral and safety tests.

**Recommendation:** Classify each failure as implementation defect, stale test, or stale spec. Fix the authoritative side, rerun all validation, and update transient validation status outside long-lived architecture prose where practical.

**Validation:** Require clean `npm run test:node`, `npm run cargo:test`, `npm run check`, `npm run build`, then `npm run validate`.

### P0-4. Process kill can target a reused PID

**Type:** Verified medium-high safety risk  
**Evidence:** `src-tauri/src/process_manager.rs:182-193,300-341,589-597`

Process Manager refreshes and validates confirmation metadata, then opens a process using only `PROCESS_TERMINATE` and kills by PID. Confirmation does not bind process creation time or canonical image identity to the terminating handle.

**Trigger:** Target exits after confirmation/list refresh and Windows reuses its PID before `OpenProcess`.

**Impact:** A different process can be terminated. The timing window may be uncommon, but consequence is high and the stronger identity pattern already exists elsewhere in JasonShell.

**Recommendation:** Capture immutable creation time plus image identity in the confirmation. Open one handle with query and terminate rights, revalidate identity on that same handle immediately before termination, and abort on any mismatch.

**Validation:** Fake a PID identity change between planning and execution. Add Windows integration coverage proving stale confirmations never terminate the replacement process.

### P1-1. Open With picker bypasses normal shell-path classification

**Type:** Verified medium security-boundary inconsistency  
**Evidence:** `src-tauri/src/shell_paths.rs:39-56,196-203,321-350`

Normal shell opening rejects protocols, nonexistent targets, and executable/script extensions. `open_shell_path_with_picker` only trims and checks nonempty input before passing it to native `ShellExecuteW` with `openas`.

**Impact:** Renderer-controlled input reaches a native shell boundary under a weaker policy. Direct exploitability was not proven, but inconsistent validation expands attack and error surface.

**Recommendation:** Reuse one validated local-path classifier. Require an existing local non-executable target unless a clearly documented picker use case needs a narrower exception.

**Validation:** Negative Rust tests for URL/protocol forms, executable/script extensions, and nonexistent paths; positive tests for normal documents.

### P1-2. Quick Command stop is broad, blocking, and transiently inconsistent

**Type:** Verified medium safety/responsiveness risk  
**Evidence:** `src-tauri/src/quick_commands.rs:980-1041`

Stop verifies root-process creation time, which is good. It then removes live run state, records stopped status before termination completes, and synchronously invokes `taskkill.exe /PID ... /T /F`. Failed termination reinserts state.

**Impact:** Default stop forcibly kills the descendant tree, whose members are not identity-bound. The Tauri command path blocks on an external utility. While it blocks, observers can see the run disappear before its final state is known.

**Recommendation:** Track a `stopping` state, execute termination off the command thread, and finalize only after result. Prefer an owned Windows Job Object or explicit root-only/tree modes over unconditional `/T /F`.

**Validation:** Simulate slow and failed termination; assert stable visible state. Test root PID reuse and child-process behavior for each stop mode.

### P1-3. Long-lived listener cleanup contract is incomplete on two surfaces

**Type:** Verified medium lifecycle defect  
**Evidence:** `src/components/CommandPanelSurface.svelte:391-406`; `src/components/TrayPanelSurface.svelte:56-67`; `master_spec.md:41`

Command Panel registers an async listener and later chains unlisten, but has no disposed guard to prevent callbacks after teardown but before registration resolves. Tray stores the unlisten function only when the registration promise resolves; if teardown already ran, that listener is never removed.

**Impact:** HMR, window destruction, or fast remount can retain stale callbacks, duplicate refresh work, or mutate destroyed component state.

**Recommendation:** Apply the same disposed async-unlistener helper/pattern used by compliant long-lived surfaces. Add these surfaces to lifecycle contract coverage.

**Validation:** Mock delayed `listen()` resolution after cleanup. Assert immediate unlisten exactly once and no retained callback behavior.

### P1-4. Control Plane exposes incorrect tab semantics

**Type:** Verified medium accessibility/UX defect  
**Evidence:** `src/components/ControlPlaneSurface.svelte:130-154`

Melt tab triggers and tabpanel content semantics are applied, but every panel is forced visible with `hidden={false}`. Selection therefore does not control one corresponding panel as the tab model promises.

**Impact:** Keyboard and screen-reader users receive misleading state and excess navigation. Visual behavior and accessibility semantics disagree.

**Recommendation:** Choose one model. Hide inactive panels for real tabs, or remove tab roles and present navigation/filter controls plus ordinary sections.

**Validation:** Keyboard and accessibility-tree test proving the selected control and visible content follow the chosen model.

### P1-5. Keyboard focus visibility is missing or intentionally weakened

**Type:** Verified medium accessibility defect; Quick Launch contrast severity needs live measurement  
**Evidence:** `src/components/ProcessManagerSurface.css:50-78`; `src/components/ControlPlaneSurface.css:103-112`; `src/components/QuickLaunchPanelSurface.svelte:306-311`; `master_spec.md:48`

Process Manager and Control Plane filter inputs remove outlines without replacement focus styling. Quick Launch intentionally suppresses its ring and relies on a subtle hover-equivalent row background.

**Impact:** Keyboard users can lose their position in dense interfaces. Quick Launch may fail visible-focus contrast expectations depending on theme and display.

**Recommendation:** Add clear `:focus-visible` treatment without duplicating selection state. Revisit the spec requirement that forbids a Quick Launch focus ring.

**Validation:** Keyboard-only test at all themes, 100% and 200% zoom, plus focus-indicator contrast measurement.

### P1-6. Process Manager live grid likely causes repeated assistive announcements

**Type:** Verified design exposure; runtime impact hypothesis  
**Evidence:** `src/components/ProcessManagerSurface.svelte:87-147,287,311`

The process list refreshes every second while open. The entire grid uses `aria-live="polite"`, in addition to a dedicated status live region.

**Impact:** Frequent row/value changes may generate continuous screen-reader announcements and accessibility-tree work, disrupting navigation and increasing rendering overhead.

**Recommendation:** Keep automatic grid refresh silent. Reserve a small live region for manual refresh results, errors, or meaningful state changes. Pause or throttle refresh whenever the surface is not visible.

**Validation:** Thirty-second NVDA and JAWS sessions on an active system; profile component updates and CPU while visible and hidden.

### P1-7. Quick Launch and tray lack keyboard action parity

**Type:** Verified medium accessibility defect  
**Evidence:** `src/components/QuickLaunchPanelSurface.svelte:71-115,169-203`; `src/components/TrayPanelSurface.svelte:70-100`

Quick Launch exposes `Run as administrator` only through right-click; Context Menu and Shift+F10 are not handled. Tray icons provide left-click and right-click actions, but no Menu-key equivalent. Tray also lacks an in-panel Escape handler and visible close control.

**Impact:** Keyboard-only users cannot reach secondary actions and may lack a clear local dismissal path.

**Recommendation:** Support Context Menu/Shift+F10 consistently or add visible More buttons. Add Escape and a visible close action to tray while retaining focus-loss behavior.

**Validation:** Full keyboard-only open, navigate, primary action, secondary action, and dismiss flows.

### P1-8. Stack Open With performs synchronous command-path work

**Type:** Verified medium responsiveness risk  
**Evidence:** `src-tauri/src/stack_popup.rs:682-720`; `src-tauri/src/stack_popup/open_with.rs:56-75,131-149`

The synchronous Tauri command authenticates and validates correctly, then performs filesystem existence/canonicalization checks and application spawning on the command path.

**Impact:** Slow disks, network-backed paths, antivirus interception, or application startup can reduce command responsiveness.

**Recommendation:** Preserve validation and app-ID allowlisting, but move resolution and launch to `spawn_blocking` behind an async command.

**Validation:** Inject a slow resolver/launcher and prove unrelated lightweight commands remain responsive.

### P2-1. Command transcript has invalid interaction semantics

**Type:** Compiler-confirmed low-medium accessibility/quality defect  
**Evidence:** `src/components/CommandPanelSurface.svelte:451`

Svelte reports keyboard and context-menu listeners on a noninteractive `<section>`. `npm run check` exits successfully but emits this warning; build repeats it.

**Recommendation:** Use the correct semantic interactive structure, focusability, and keyboard contract rather than suppressing the warning blindly.

**Validation:** Zero Svelte warnings plus keyboard/context-menu behavior test.

### P2-2. Test suite overuses source regex and CSS literals

**Type:** Verified validation-design risk

Source tests are valuable for security ordering, forbidden calls, event names, and capability boundaries. They are fragile when used for visual pixel values, broad framework migration shape, or behavior that can be represented by pure helpers or runtime tests.

**Impact:** Safe refactors fail without behavioral regression, while code can satisfy a regex and still fail at runtime. Direct `node --test` can also consume stale compiled helpers; only `npm run test:node` cleans and rebuilds `dist-tests`.

**Recommendation:** Keep textual tests for intentional source invariants. Replace visual literals with computed-style/layout assertions and move behavior to helper, component, Rust unit, or integration tests. Document the official test entry point.

### P2-3. Product documentation overstates incomplete capabilities

**Type:** Verified product/documentation gap  
**Evidence:** `src-tauri/src/workspaces.rs:7-9,142-217`; workspace tests; Phase 9 architecture material; `README.md`

Workspace restoration is hardcoded `reserved-not-implemented`; startup plans never execute. Automation forwarding is planned, not wired. Multi-monitor runtime remains single-monitor despite architecture/planning work. README workflow descriptions do not clearly expose these limits.

**Impact:** Users may interpret persisted plans and rich UI as active restoration/automation behavior.

**Recommendation:** Label planning-only behavior in UI and README. Do not imply restoration, startup execution, forwarding, or multi-monitor ownership until implemented and live-tested.

### P2-4. Suspected unused platform helper dependencies

**Type:** Low-confidence hygiene finding  
**Evidence:** `package.json` includes `afplay`, `bun`, `bunx`, and `osascript`; preliminary app-source search found no imports.

**Recommendation:** Run a dependency graph/usage audit before removal. Package-lock presence and string mentions alone are insufficient proof of runtime use or disuse.

## 7. Validation Results

| Command | Result | Evidence |
|---|---|---|
| `npm run check` | Pass with warning | 0 errors, 1 Svelte a11y warning at `CommandPanelSurface.svelte:451` |
| `npm run build` | Pass with same warning | 338 modules; build about 7.1 s |
| `npm run test:node` | Fail | 744 total, 730 pass, 11 fail, 3 todo |
| `npm run cargo:test` | Fail | 554 pass, 3 fail, 3 ignored |
| `npm run validate` | Fail by prerequisite gates | Full validation cannot be considered green |

### 7.1 Node failures

1. `tests/bootstrapWindowsContract.test.mjs:40` - README bootstrap/lockfile-safe install contract
2. `tests/meltMigrationWiring.test.mjs:40` - shared Melt primitives
3. `tests/meltMigrationWiring.test.mjs:189` - bottom-bar Melt command buttons
4. `tests/quickIcons.test.mjs:135` - Quick Launch scoped/camelCase events
5. `tests/stackBrowserPhase1Safety.test.mjs:90` - authorization before side effects
6. `tests/stackBrowserTerminal.test.mjs:272` - terminal authorization ordering; excluded feature, but still a repository gate
7. Two `tests/stackPopupState.test.mjs` sorting assertions
8. `tests/taskPreviewRetention.test.mjs:93` - close fallback termination contract
9. `tests/taskPreviewTextPolish.test.mjs:23` - expected `2.1rem`, implementation uses `2.4rem`
10. `tests/taskbarUxState.test.mjs:64` - expected 4 px `#ffd54f`, implementation uses 3 px `#7e610b` plus shared 2 px warning style

The count above contains 11 failing tests because the Stack Popup item represents two failures.

### 7.2 Rust failures

1. `contracts::tests::new_command_contracts_are_unique_and_stable` - missing `hide_quick_launch_panel_on_focus_loss` from expected command set
2. `search::providers::apps::tests::stale_app_cache_returns_existing_rows_while_refresh_is_deferred` - `snapshot.refresh_needed` unexpectedly false
3. `shell_paths::tests::vscode_resolver_uses_standard_candidate_order` - got `None`, expected `Some("C:\\Tools\\code.cmd")`

### 7.3 Build observations

Larger emitted chunks include:

- `contextMenuPosition`: 365.75 kB raw, 94.27 kB gzip
- `StackPopupSurface`: 130.55 kB raw, 35.31 kB gzip
- `MeltActionButton`: 84.61 kB raw, 28.07 kB gzip

Code splitting exists, and raw chunk size alone is not proof of user-visible slowness. The first chunk deserves bundle ownership inspection because its name does not explain its large dependency closure. Excluded Stack Git/terminal code may contribute to shared chunks, so no optimization target should be selected without a bundle graph and startup profile.

## 8. Runtime Evidence Gaps

Highest-value missing runtime checks:

1. AppBar startup, rollback, Explorer taskbar restoration, fullscreen, and cleanup under injected delay/failure
2. 125%, 150%, and 200% DPI/zoom on narrow and wide displays
3. Multi-monitor taskbar/work-area behavior, even while runtime support remains single-monitor
4. NVDA/JAWS navigation for Process Manager, Control Plane, Quick Launch, tray, and Quick Commands
5. Keyboard-only secondary actions and panel dismissal
6. Large directory/archive memory and latency profiling
7. PID-reuse termination test with real Windows process handles
8. Per-surface open/close, focus-loss synchronization, capability invocation, and console/IPC error smoke
9. Search provider health, stale cache, cancellation, and latest-only behavior with real provider timing
10. Task attention and task-preview layout screenshot/computed-style tests

Current scripts provide extensive static/unit validation and one explicit manual fullscreen path, but not a repeatable live Tauri/Win32 smoke suite.

## 9. UI and Accessibility Summary

Strong patterns:

- Search combobox/listbox semantics, active-descendant handling, status reporting, and selected-item scrolling
- Stack file-view keyboard navigation and context-menu key support
- Quick Command dialog/alert flow and transcript actions
- Task-preview activation/close affordances and task attention state design

Main weaknesses:

- Semantic mismatch in Control Plane tabs
- Removed focus outlines without replacement
- Mouse-only secondary actions in Quick Launch and tray
- Potentially noisy Process Manager live region
- Noninteractive transcript container handling keyboard/context events
- Quick Launch's no-ring contract prioritizes visual minimalism over robust keyboard visibility

Static top-bar review also found fixed widths and small controls that may crowd at high scaling. This remains a hypothesis until tested on real WebView2/Windows scaling combinations.

## 10. Security and Safety Summary

No hardcoded secret or conventional web-auth defect dominated this local desktop audit. Main trust boundaries are native process control, shell launch, filesystem access, and renderer-to-Tauri authorization.

Most important safety issues:

- PID-only termination after a time-separated identity check
- Weaker validation in the native Open With picker path
- Broad forced descendant termination in Quick Commands
- Unbounded directory/archive discovery and retained paging data as local resource exhaustion

Strong mitigating design:

- Narrow capability files and caller authorization
- Stack path and child-name validation
- App-ID allowlisting for Stack Open With
- Quick Launch nonce/allowed-path model
- Root creation-time check before Quick Command termination
- No automatic workspace command execution

Quick Commands intentionally provide arbitrary local execution from trusted saved configuration. UI/docs should describe this as trusted local automation and clearly mark imported or newly edited commands before execution.

## 11. Prioritized Roadmap

### P0: Release and safety gates

1. Triage all 14 failing Node/Rust tests; make full validation green.
2. Reduce AppBar lock scope and prove rollback under slow/failing native calls.
3. Add hard Stack scan, page, session-entry, and session-byte limits.
4. Bind Process Manager confirmation and kill to immutable identity on one handle.
5. Correct stale validation status in canonical docs.

### P1: Lifecycle, responsiveness, and accessibility

1. Fix Command Panel and tray async listener disposal.
2. Unify shell Open With validation.
3. Move Quick Command stop and Stack Open With blocking work off command paths.
4. Replace unconditional forced tree kill with explicit, owned semantics.
5. Resolve Control Plane tab model.
6. Restore visible focus and keyboard secondary-action parity.
7. Remove live-region behavior from the auto-refreshing process grid.
8. Resolve Svelte transcript warning semantically.

### P2: Product truth and runtime confidence

1. Add repeatable Tauri/Win32 smoke coverage for every persistent surface.
2. Test assistive technology, DPI, narrow displays, and monitor transitions.
3. Clearly label workspace, automation, and multi-monitor planning-only behavior.
4. Replace brittle visual/source-literal tests with behavioral evidence.
5. Audit bundle ownership and suspected unused dependencies.
6. Implement restoration/startup/forwarding only with explicit consent, auditability, rollback, and live tests.

## 12. Final Verdict

JasonShell is more robust than a typical shell prototype and has made substantial progress against its earlier audit. Its strongest work is native boundary design, stale-state control, path authorization, and documented architecture. Its weakest area is proof and execution at integration boundaries: global native-state synchronization, resource bounding, process identity at the final destructive action, long-lived listener teardown, keyboard semantics, and live Windows validation.

Current state is suitable for continued controlled development and technical demonstration. It is not ready for release as a dependable Explorer-shell replacement until validation is green, P0 risks are addressed, and live Windows evidence covers startup, recovery, scaling, accessibility, and destructive process operations.
