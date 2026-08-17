# Stack Browser Improvement Audit

Status: source-only audit. No runtime observation completed.

Scope: Stack Browser popup, related frontend docs, Rust/Tauri commands, file operations, Git workbench, ZIP/archive behavior, drag/drop, path autocomplete, resize/persistence, smoke coverage, accessibility, UX, performance, safety, maintainability.

## Evidence label rules

- Verified source finding: confirmed from repository source/docs/spec supplied or inspected.
- Runtime-observed finding: confirmed by running app/browser/native UI. None in this audit.
- Risk/inference: plausible issue from source shape or missing guard, needs runtime/manual validation.
- Product idea: optional feature direction, not required unless product strategy chooses it.
- Manual-validation gap: behavior not covered by existing manual smoke checklist or automated evidence.

## Methodology and limits

- `master_spec.md` treated as canonical current behavior.
- `docs/smoke-test-windows.md` checked for current Stack smoke scope.
- `src-tauri/capabilities/stack-popup.json` verified present and scoped to `stack-popup` with `core:default` and `core:window:default`.
- Source facts supplied for Stack popup frontend/backend used as audit input, with high-priority source claims tied to exact paths/lines where inspected.
- Vite server unavailable and native Tauri not run, so this doc intentionally contains zero `Runtime-observed finding` items beyond label taxonomy.
- This audit does not edit `master_spec.md`, `changelog.md`, or product behavior.

## Current strengths

- Verified source finding: Stack Browser is already feature-rich: Svelte/Tauri popup, virtual rows, paging, icon hydration, file ops, drag/drop, Git workbench, ZIP browsing, path autocomplete, context menus, persistent resize.
- Verified source finding: `StackPopupSurface.svelte` is large and centralized, but it also means many workflows share one consistent state machine.
- Verified source finding: `master_spec.md` describes many mature behaviors: paged folder reads, navigation history, editable path textbox, clickable breadcrumbs, sorting, range/toggle selection, blank-area marquee, delete confirmation, focus-loss suppression, inline rename/new folder, row/background context menus, native Properties handling, Git views, and async icon hydration.
- Verified source finding: `src-tauri/capabilities/stack-popup.json:1-11` exists, narrows window scope to `stack-popup`, and grants only core/window defaults, which is stronger than a broad all-window capability.
- Verified source finding: icon hydration is designed with visible-row prioritization, stale guards, capped batch size, capped concurrency, cache reuse, and progress counters.
- Verified source finding: Git command argv is fixed, repo root resolution exists, and branch/tree/path validation is described in `master_spec.md`.

## Priority model

- P0: proven or strongly evidenced data loss/security boundary failure, command injection, or unbounded app-wide hang.
- P1: common workflow broken/degraded, severe accessibility failure, major perf cliff, stale/misleading docs.
- P2: high UX friction, responsive/layout weakness, missing cancellation/progress, test coverage gap.
- P3: polish, discoverability, power-user productivity, maintainability improvements.
- P4: optional product expansion, nice-to-have customization, future differentiation.

## Ranked summary table

| Rank | Priority | Label | Area | Recommendation | Dependency | Impact | Effort |
|---:|---|---|---|---|---|---|---|
| 1 | P1 | Risk/inference | File ops | Add verified copy/move journal, recovery, conflict handling | unified job model | Prevent ambiguous crash recovery and split results | High |
| 2 | P1 | Verified source finding + Risk/inference | Git | Add timeouts, `GIT_TERMINAL_PROMPT=0`, typed nonzero errors | git wrapper | Avoid hangs, expose auth failures | Medium |
| 3 | P1 | Verified source finding + Risk/inference | External tools | Add timeout/cancel for PowerShell/7z/archive extraction | unified job model | Avoid stuck process impact | Medium |
| 4 | P1 | Verified source finding | Docs | Fix stale `stack_browser.md` claims, esp nonvirtualized rows | docs pass | Prevent wrong future decisions | Low |
| 5 | P1 | Verified source finding | A11y | Implement combobox/listbox path autocomplete semantics | frontend component split | Keyboard/screen reader usability | Medium |
| 6 | P1 | Risk/inference | A11y | Full menu arrow/Home/End/typeahead/focus-return model | menu controller | Standard desktop semantics | Medium |
| 7 | P1 | Verified source finding | Perf | Make folder reads/path suggestions async/nonblocking | backend refactor | Avoid UI stalls | Medium |
| 8 | P1 | Verified source finding | ZIP | Avoid full ZIP scan each open; add index/cache/progress | archive service | Large archive usability | High |
| 9 | P1 | Risk/inference | UI | Responsive compact/narrow layout, overloaded toolbar redesign | design pass | Smaller popup usable | Medium |
| 10 | P2 | Verified source finding | Maintainability | Split 2924-line Svelte and 1587-line CSS into modules | tests first | Lower regression risk | High |
| 11 | P2 | Manual-validation gap | QA | Expand native smoke matrix for file/Git/archive/perf/a11y | QA docs | Real confidence | Medium |
| 12 | P2 | Risk/inference | Jobs | Canonical unified operation center/job model with progress/cancel/retry/history | job model | Trust during long ops | High |
| 13 | P2 | Verified source finding + Risk/inference | Resources | Add byte budgets for bounded caches/output | state budgets | Predictable memory | Medium |
| 14 | P3 | Product idea | UX | Action palette, recent/favorites, tabs/split/dual-pane | command registry | Power-user speed | Medium/High |
| 15 | P4 | Product idea | Features | Search/preview/bulk rename/custom columns/localization | product choice | Differentiation | Varies |

## Detailed findings and recommendations

### 1. File move/copy safety journal

- Label: Risk/inference.
- Priority: P1.
- Evidence/current state: Risk/inference only. Source shape indicates cross-volume move uses copy then delete behavior; this does not prove original loss after partial copy. Real gap: crash/recovery ambiguity, no cited metadata/integrity verification, conflict/duplicate reconciliation risk, and no durable operation journal found in supplied source facts.
- User scenario/problem: user drags large folder from one drive to another; copy succeeds for some items, delete later begins, then crash/network/path error occurs. User may see ambiguous state, split duplicates, or no guided repair path.
- Recommendation: implement operation journal for copy/move/delete/extract. For cross-volume move: copy to temp/final, verify size/hash or directory manifest where practical, flush/close, then delete originals only after verified success. Persist journal enough to offer repair/resume after crash.
- Why worthwhile: file manager trust depends on never losing data silently.
- Why industry-standard apps include it: Explorer, Finder, Directory Opus, Total Commander-like tools show conflict/progress states and avoid destructive steps until copy side succeeds.
- Impact: highest trust/safety improvement; reduces catastrophic failure risk.
- Effort/complexity: high; needs backend job abstraction, journaling, conflict policy, tests with temp dirs and injected failures.
- Acceptance criteria: cross-volume move failure before verification never deletes unverified originals; crash mid-operation shows recoverable journal; tests simulate copy failure/delete failure/crash-after-copy; UI shows copied/deleted/remaining state and repair option.

### 2. Git command noninteractive timeouts

- Label: Verified source finding + Risk/inference.
- Priority: P1.
- Evidence/current state: Verified nuance: `src-tauri/src/stack_popup/git_status.rs:445-455` `git_stdout_bytes` runs `Command::new("git").output()` and swallows nonzero status as `Ok(None)`. `src-tauri/src/stack_popup/git_status.rs:458-475` mutating `run_git` captures stderr and returns it on nonzero status. `src-tauri/src/stack_popup/git_status.rs:478-495` `run_git_with_stdin` waits without visible timeout. None of these helpers set a timeout or noninteractive env such as `GIT_TERMINAL_PROMPT=0`. Hang/auth impact is Risk/inference until runtime-tested.
- User scenario/problem: user opens Git workbench in repo requiring credentials or stuck remote; fetch/pull/push can hang or fail with no actionable error.
- Recommendation: wrap all Git processes with timeout, kill tree on timeout, set `GIT_TERMINAL_PROMPT=0`, capture stderr/stdout separately, return typed errors for auth required, network failure, merge conflict, non-fast-forward, timeout.
- Why worthwhile: prevents hidden native process hangs and gives user next action.
- Why industry-standard apps include it: VS Code, Git clients, and IDEs surface auth/conflict/non-fast-forward states explicitly instead of swallowing nonzero status.
- Impact: prevents dead workflows and stale Git UI.
- Effort/complexity: medium.
- Acceptance criteria: fake Git command timeout test; auth-needed stderr maps to visible auth message; nonzero exit preserves stderr; fetch/pull/push cannot run indefinitely.

### 3. External archive/tool process controls

- Label: Verified source finding + Risk/inference.
- Priority: P1.
- Evidence/current state: Verified: source/docs identify external PowerShell/7z archive extraction and no cited timeout/cancel path. Inferred: corrupt/password/huge archive can become stuck or require cancel; runtime impact not proven by this audit.
- User scenario/problem: user extracts corrupt huge archive or 7z waits unexpectedly; Stack popup may appear stuck or leave user without cancel/retry.
- Recommendation: route extraction through job runner with timeout, cancellation, bounded output, stderr capture, progress where possible, and process-tree kill.
- Why worthwhile: archive workflows can be slow/untrusted and must not monopolize app.
- Why industry-standard apps include it: Explorer and archive tools show progress/cancel and return clear errors for corrupt archives.
- Impact: better reliability for ZIP/archive feature.
- Effort/complexity: medium.
- Acceptance criteria: corrupt archive returns bounded visible error; cancel kills child process; timeout test passes; no unbounded stdout/stderr retention.

### 4. Blocking filesystem reads and suggestions

- Label: Verified source finding + Risk/inference.
- Priority: P1.
- Evidence/current state: Verified shape: `src-tauri/src/stack_popup/paging.rs:156` uses `fs::read_dir(...)` synchronously inside folder-page read; source facts also identify synchronous path suggestion scans. Runtime UI stall/freeze is Risk/inference, because native app was not run.
- User scenario/problem: user opens slow network folder or directory with many entries; UI may stall or feel frozen if sync work occupies an async command path.
- Recommendation: move directory reads and suggestion scans fully behind async/spawn-blocking jobs with cancellation/stale request IDs; expose progress states for slow reads.
- Why worthwhile: browsing must stay responsive under slow disks/network paths.
- Why industry-standard apps include it: Explorer/Finder/VS Code avoid blocking UI while enumerating folders.
- Impact: major perceived performance improvement.
- Effort/complexity: medium.
- Acceptance criteria: stale folder read result cannot replace newer path; slow mock read does not block UI state update; smoke test covers network/large folder.

### 5. ZIP full scan on each open

- Label: Verified source finding.
- Priority: P1.
- Evidence/current state: ZIP browsing performs full scan each open: `src-tauri/src/stack_popup/paging.rs:202-207` opens `ZipArchive` and iterates `0..archive.len()` for folder collection.
- User scenario/problem: user repeatedly navigates large ZIP; every open feels slow and battery/CPU heavy.
- Recommendation: cache archive index by canonical path + mtime + size; lazy-load directory children; add progress and cancel for index build; bound cache memory.
- Why worthwhile: makes ZIP browsing usable beyond small archives.
- Why industry-standard apps include it: archive managers keep central-directory indexes and avoid repeated full scans.
- Impact: high for archives; lowers CPU.
- Effort/complexity: high if nested paths/encoding/security edge cases included.
- Acceptance criteria: second open avoids full scan; cache invalidates on mtime/size change; large ZIP open has progress/cancel; zip-slip paths rejected in tests.

### 6. Paging snapshots and resource budgets

- Label: Verified source finding + Risk/inference.
- Priority: P2.
- Evidence/current state: Verified: sessions are capped, not unbounded. `src-tauri/src/stack_popup/paging.rs:12-13` defines `DEFAULT_PAGE_LIMIT = 80` and `MAX_STACK_FOLDER_SESSIONS = 32`; `src-tauri/src/stack_popup/paging.rs:443-454` trims session count. Exact risk: 32 full snapshots can still exceed desired memory if entries are huge; session count is not a byte budget. Terminal replay may already be bounded by spec to 256 KiB, so do not call terminal replay unbounded without fresh source proof.
- User scenario/problem: user browses many huge dirs/ZIPs; memory rises unpredictably.
- Recommendation: define resource budgets for Stack listing snapshots, archive indexes, Git/archive outputs, status text, clipboard metadata, and diagnostics. Store compact page windows or shared entry index with LRU by byte budget; expose perf counters.
- Why worthwhile: count limits do not equal memory limits.
- Why industry-standard apps include it: virtualized file explorers maintain bounded caches.
- Impact: protects long sessions.
- Effort/complexity: medium.
- Acceptance criteria: cache max bytes enforced; memory benchmark for 100k-entry dirs remains under baseline budget; evicted pages reload correctly; output truncation carries explicit `truncated` flags.

### 7. Short close-button hit height

- Label: Verified source finding.
- Priority: P2.
- Evidence/current state: `src/components/StackPopupSurface.css:27-39` defines `.stack-browser-close-button` with `height: 1.35rem` and `min-width: 2.1rem`. Width is at least 2.1rem; concern is height/hit area, not width.
- User scenario/problem: user misses target on touchpad/high-DPI/fast use; closing feels fiddly.
- Recommendation: increase hit target to at least 32x32 CSS px visual or 24x24 minimum with larger invisible hit area; keep red rectangular visual if product wants shell consistency.
- Why worthwhile: reduces click errors on common action.
- Why industry-standard apps include it: Windows titlebar and Fluent controls use comfortable target sizes.
- Impact: small but frequent UX gain.
- Effort/complexity: low.
- Acceptance criteria: hit target measured >= 32px or documented exception; focus ring visible; keyboard accessible name present.

### 8. Extremely small typography

- Label: Risk/inference.
- Priority: P2.
- Evidence/current state: `src/components/StackPopupSurface.css:157` uses breadcrumb font `.58rem`; `src/components/StackPopupSurface.css:191` uses Git summary font `.55rem`.
- User scenario/problem: user cannot read paths/status at normal scale or on dense displays.
- Recommendation: set minimum practical font scale, e.g. 11-12px equivalent for secondary text, 13px for core labels; support compact mode separately.
- Why worthwhile: readability is baseline accessibility.
- Why industry-standard apps include it: Explorer, VS Code, Finder use smaller text sparingly and retain legibility.
- Impact: medium UX/a11y improvement.
- Effort/complexity: low/medium due layout ripple.
- Acceptance criteria: no essential text below agreed min; 125% Windows scaling remains usable; truncation has tooltip/title where needed.

### 9. Overloaded toolbar and dense Git workbench

- Label: Risk/inference.
- Priority: P1.
- Evidence/current state: Source-grounded UX assessment/inference from `src/components/StackPopupSurface.svelte`, `src/components/StackPopupSurface.css`, and Git workbench breadth in `src-tauri/src/stack_popup/git_status.rs`. This is not a verified defect without runtime/usability testing.
- User scenario/problem: casual user cannot find safe primary action; expert user loses speed due visual clutter.
- Recommendation: group actions into primary nav/path/search lane, view/action lane, overflow menu, and contextual selection bar. For Git, use tabs with clear status summary, staged/unstaged sections, branch sync status, and progressive disclosure.
- Why worthwhile: reduces cognitive load without removing power.
- Why industry-standard apps include it: VS Code, Git clients, Explorer command bar prioritize common actions and hide advanced actions in overflow/context menus.
- Impact: high discoverability and perceived polish.
- Effort/complexity: medium.
- Acceptance criteria: default width shows top 5 actions without overlap; overflow is keyboard reachable; Git common flow add/commit/fetch visible and advanced actions grouped.

### 10. Narrow/compact fixed columns

- Label: Risk/inference.
- Priority: P1.
- Evidence/current state: Source-grounded UX assessment/inference from Stack popup CSS/layout; no runtime resize test completed, so not a verified defect.
- User scenario/problem: user resizes popup narrow; columns clip, controls overlap, horizontal scan becomes hard.
- Recommendation: implement responsive column presets: compact cards/list for narrow width, details grid for medium/wide, hide low-value columns behind column chooser.
- Why worthwhile: popup is resizable; layout must adapt.
- Why industry-standard apps include it: Files, Finder, VS Code sidebars adapt density/views to available width.
- Impact: high because persistent resize invites custom sizes.
- Effort/complexity: medium.
- Acceptance criteria: at min width, filename, icon, selection, primary actions remain usable; no essential control overflow; columns can be toggled/restored.

### 11. Path autocomplete semantics

- Label: Verified source finding.
- Priority: P1.
- Evidence/current state: path input keyboard handler exists at `src/components/StackPopupSurface.svelte:2438`, but grep found no `role="combobox"`, `role="listbox"`, or `aria-activedescendant` in `StackPopupSurface.svelte`.
- User scenario/problem: screen reader user or keyboard-only user cannot understand suggestion count/active option; browser autocomplete patterns not announced.
- Recommendation: implement WAI-ARIA combobox pattern: input role/attributes, listbox popup, option ids, `aria-activedescendant`, expanded state, keyboard model, status announcement.
- Why worthwhile: path box is core navigation.
- Why industry-standard apps include it: mature apps expose predictable keyboard/autocomplete semantics for assistive tech.
- Impact: high accessibility improvement.
- Effort/complexity: medium.
- Acceptance criteria: arrow keys move active suggestion; Enter/Tab accept; Escape closes; screen reader announces active option; tests cover ARIA attributes.

### 12. Context menu keyboard model

- Label: Verified source finding.
- Priority: P1.
- Evidence/current state: context menus use `role="menu"` at `src/components/StackPopupSurface.svelte:2833`, `2844`, and `2873`, but visible keydown handling at `2838` and `2878` only handles Escape; no central roving tabindex/Home/End/typeahead model was found in supplied source facts.
- User scenario/problem: keyboard user opens menu and cannot navigate like desktop menus or reliably return focus to origin.
- Recommendation: centralize menu controller with roving tabindex, ArrowUp/Down, Home/End, Escape close, Enter/Space activate, disabled item skip, focus restore to invoker/row.
- Why worthwhile: context menus hold file operations; keyboard access is essential.
- Why industry-standard apps include it: Windows menus, VS Code menus, web ARIA menu patterns include this behavior.
- Impact: high for accessibility and power use.
- Effort/complexity: medium.
- Acceptance criteria: automated keyboard tests; Escape returns focus; menu cannot trap focus after close; disabled items skipped.

### 13. Modal focus trap and return

- Label: Risk/inference.
- Priority: P1.
- Evidence/current state: Existing partial focus management is visible in `src/components/StackPopupSurface.svelte` (for example delete cancel focus and grid focus returns around lines 961-966 and 994). No reusable full trap/inert modal primitive evidence was found in supplied source facts.
- User scenario/problem: delete/rename/Git dialogs open; Tab escapes behind modal or close returns focus nowhere.
- Recommendation: add reusable modal primitive with initial focus, trap, inert background or equivalent, Escape/close semantics, and focus return.
- Why worthwhile: destructive file ops require safe focus behavior.
- Why industry-standard apps include it: modal dialogs in desktop/web apps trap focus and restore origin.
- Impact: high a11y/safety.
- Effort/complexity: medium.
- Acceptance criteria: Tab cycles within modal; Shift+Tab works; Escape behavior documented; focus returns to selected row/action after close.

### 14. Unified status, error, and progress feedback

- Label: Verified source finding.
- Priority: P2.
- Evidence/current state: Existing loading/error/status/icon progress is present (`src/components/StackPopupSurface.svelte:136-137`, `380-415`, `549-587`, `597-611`, and icon hydration state/progress per `master_spec.md`). Gap is unified long-operation byte/file progress, cancel, retry, details, and history across file ops/archive/Git.
- User scenario/problem: user starts delete/copy/extract/fetch and cannot tell if it is running, stuck, succeeded, or failed.
- Recommendation: introduce canonical unified operation center/job model with queued jobs, file/byte progress, cancel, retry, details, copy error, and final history. Other progress/cancel recommendations should reference this model instead of inventing parallel mechanisms.
- Why worthwhile: confidence during dangerous/long operations.
- Why industry-standard apps include it: Explorer copy dialog, Finder progress, VS Code SCM/output channels all provide operation state.
- Impact: high trust improvement.
- Effort/complexity: high if unified across ops; medium for first minimal status lane.
- Acceptance criteria: each long op has pending/running/succeeded/failed/canceled state; byte/file progress appears when knowable; cancel/retry paths deterministic; errors actionable; `aria-live` announces completions/failures.

### 15. Hidden shortcuts and command discoverability

- Label: Risk/inference.
- Priority: P3.
- Evidence/current state: Source/docs describe many actions and shortcuts, but discoverability pain was not runtime-tested. Treat as Risk/inference.
- User scenario/problem: user never learns XButton nav, Tab autocomplete, marquee selection, Git commands, open-with, resize persistence.
- Recommendation: add command palette/help overlay/shortcut reference and optional first-run hints.
- Why worthwhile: existing features become usable without new backend.
- Why industry-standard apps include it: VS Code, Directory Opus, Total Commander-like tools expose commands/shortcuts centrally.
- Impact: medium; high ROI.
- Effort/complexity: low/medium if backed by action registry.
- Acceptance criteria: `?` or menu opens shortcuts; every action has label, shortcut, enabled reason; no hidden-only critical action.

### 16. Frontend monolith maintainability

- Label: Verified source finding.
- Priority: P2.
- Evidence/current state: `src/components/StackPopupSurface.svelte` is 2924 lines and `src/components/StackPopupSurface.css` is 1587 lines.
- User scenario/problem: future fixes risk regressions because unrelated file nav, Git, drag, menu, autocomplete, resize, selection, and archive states share one giant component.
- Recommendation: split into focused modules: path bar/autocomplete, virtual file list, selection/marquee, context menu, file ops dialogs, Git workbench, archive view, resize shell, status/operation center. Move pure state reducers to `src/features/stack-browser` with tests.
- Why worthwhile: lowers regression risk and makes a11y/perf work tractable.
- Why industry-standard apps include it: complex surfaces use component/state boundaries and testable reducers.
- Impact: high dev velocity/quality.
- Effort/complexity: high; should be phased behind tests.
- Acceptance criteria: no single component > about 800-1000 lines without documented reason; pure reducers covered; UI behavior parity tests pass.

### 17. Stale Git views

- Label: Risk/inference.
- Priority: P2.
- Evidence/current state: stale Git views possible; Git workbench has Changes/Log/Tree/Branches and remote operations.
- User scenario/problem: user commits/fetches/checkouts externally or in terminal; Stack Git view shows old state and user acts on stale info.
- Recommendation: add repo watcher or refresh-on-focus/operation complete, with generation IDs and stale badge. Avoid aggressive polling on huge repos.
- Why worthwhile: SCM UI must be accurate before destructive/branch actions.
- Why industry-standard apps include it: VS Code/Git clients auto-refresh and show sync state.
- Impact: medium/high for dev workflows.
- Effort/complexity: medium.
- Acceptance criteria: external file change triggers refresh or stale badge; operation completion refreshes relevant views; stale responses cannot overwrite new view.

### 18. Diagnostics log full paths

- Label: Verified source finding.
- Priority: P3 unless exported/shared logs are included in support flow.
- Evidence/current state: diagnostics log full path.
- User scenario/problem: exported/shared logs may disclose sensitive user/project path names during bug reports.
- Recommendation: redact or hash home/project roots by default; provide opt-in verbose diagnostics mode.
- Why worthwhile: privacy and safer support logs.
- Why industry-standard apps include it: mature apps scrub PII/secrets from diagnostics unless user opts in.
- Impact: medium privacy improvement.
- Effort/complexity: low/medium.
- Acceptance criteria: logs replace home path with `%USERPROFILE%` or token; tests cover redaction; verbose mode documented.

### 19. IPC permissions and invoke exposure assurance audit

- Label: Risk/inference assurance gap.
- Priority: P1.
- Evidence/current state: Verified good baseline: `src-tauri/capabilities/stack-popup.json:1-11` exists, scopes local window to `stack-popup`, and grants `core:default` plus `core:window:default`. This audit did not prove a vulnerability. Gap is assurance: no complete command/window/validation matrix cited for powerful Stack file/Git/native invokes.
- User scenario/problem: future change could accidentally broaden renderer authority or wrong-window invoke surface if custom command exposure is not documented/tested.
- Recommendation: inventory every Stack IPC command, allowed window labels, path validation, canonicalization, and capability binding. Confirm custom commands cannot be invoked from unrelated windows or untrusted contexts; add explicit tests/contracts for exposure. Phrase output as assurance posture, not proven exploit.
- Why worthwhile: Stack Browser has powerful filesystem/native operations.
- Why industry-standard apps include it: desktop apps isolate privileged file operations and narrow renderer authority.
- Impact: high security boundary confidence, not proven vulnerability remediation.
- Effort/complexity: medium.
- Acceptance criteria: command matrix lists each command, window, validation, side effects; tests reject wrong window where feasible; docs do not claim missing capability.

### 20. Native drag/drop blocking

- Label: Verified source finding.
- Priority: P1.
- Evidence/current state: `src-tauri/src/stack_popup/native_drag.rs:56` imports `SHDoDragDrop`; `src-tauri/src/stack_popup/native_drag.rs:82` calls it directly. Runtime stall impact remains inference until manual drag testing.
- User scenario/problem: user drags files to Explorer; Stack UI may freeze until native drag completes or target stalls.
- Recommendation: isolate blocking native drag on dedicated thread/job; guard reentrancy; show drag state; ensure cleanup after cancel/drop.
- Why worthwhile: drag/drop is core file-manager muscle memory.
- Why industry-standard apps include it: native file managers keep UI responsive around shell drag loops where possible.
- Impact: medium/high.
- Effort/complexity: medium/high due COM/apartment/threading details.
- Acceptance criteria: drag cancel returns UI to normal; no stuck gesture state; repeated drags safe; manual test includes drag to Explorer and invalid target.

### 21. Delete serial frontend commands

- Label: Verified source finding.
- Priority: P2.
- Evidence/current state: delete uses serial frontend commands.
- User scenario/problem: deleting many selected files is slow; partial failures hard to summarize.
- Recommendation: backend batch delete command with job id, preflight, per-item result, cancel, undo/journal where possible.
- Why worthwhile: batch operations need atomic-ish UX and summary.
- Why industry-standard apps include it: Explorer groups deletes and reports per-item failures/conflicts.
- Impact: medium/high.
- Effort/complexity: medium.
- Acceptance criteria: 100-item delete uses one job; partial failures summarized; focus-loss suppression remains correct; cancel works before next item.

### 22. Shared terminal backend stop sync wait

- Label: Adjacent/shared risk.
- Priority: P3.
- Evidence/current state: `src-tauri/src/stack_popup/terminal.rs:554` and `624` show synchronous `session.child.wait()` in shared terminal backend. Master spec says current visible terminal UI is persistent `terminal-panel`, not Stack Browser embedded CLI. This is adjacent shared backend risk, not Stack Browser visible UI defect.
- User scenario/problem: stopping shared terminal sessions could block if process tree is slow to exit.
- Recommendation: handle wait in blocking worker with timeout and forced cleanup; return immediate stopping state for terminal-panel UI.
- Why worthwhile: stop should not freeze UI.
- Why industry-standard apps include it: terminal apps detach UI responsiveness from process teardown.
- Impact: medium for terminal backend; adjacent to Stack popup only through shared module history.
- Effort/complexity: low/medium.
- Acceptance criteria: stop returns quickly; timeout logged; zombie sessions cleaned; terminal-panel UI shows stopping state.

### 23. Resource budget audit

- Label: Verified source finding + Risk/inference.
- Priority: P2.
- Evidence/current state: Do not call Stack folder sessions unbounded: `src-tauri/src/stack_popup/paging.rs:13` caps sessions at 32 and `443-454` trims them. Risk is budget ambiguity: count caps do not prove byte caps; Git/archive/status/clipboard diagnostics need explicit max bytes. Terminal replay may already be bounded per `master_spec.md`, so include it only if fresh source audit finds an uncovered path.
- User scenario/problem: long sessions, large clipboard paths, repeated operations, huge Git/archive output, or large folder snapshots grow memory/log output beyond acceptable budgets.
- Recommendation: define byte/count budgets for clipboard metadata, status text, folder listing snapshots, Git output, archive output, and diagnostics. Enforce truncation with explicit `truncated` flags.
- Why worthwhile: predictable resource use.
- Why industry-standard apps include it: robust desktop apps bound logs, output panes, and in-memory caches.
- Impact: medium reliability.
- Effort/complexity: medium.
- Acceptance criteria: each buffer/cache has named owner and constant; tests exceed limit and verify truncation/eviction; UI shows truncated marker; baseline memory run records max MB.

### 24. Persistence temp collisions

- Label: Risk/inference.
- Priority: P2.
- Evidence/current state: persistence tmp collisions identified as risk.
- User scenario/problem: simultaneous saves from multiple surfaces collide, corrupt settings, or lose resize/pin state.
- Recommendation: use atomic writes with unique temp names, fsync where practical, merge-safe update API, and recovery from temp leftovers.
- Why worthwhile: state persistence should be boring and safe.
- Why industry-standard apps include it: settings files are commonly written atomically with temp+rename.
- Impact: medium.
- Effort/complexity: low/medium.
- Acceptance criteria: concurrent save test; crash before rename leaves previous file valid; orphan temp cleanup safe.

### 25. Native clipboard RAII

- Label: Risk/inference.
- Priority: P1.
- Evidence/current state: native clipboard RAII risk identified.
- User scenario/problem: clipboard opened and not closed on error/panic; paste/copy fails in other apps until process exits.
- Recommendation: wrap clipboard open/close and global memory handles in RAII types with tests for early-return cleanup.
- Why worthwhile: native resource leaks are user-visible across desktop.
- Why industry-standard apps include it: Win32 wrappers use RAII to guarantee cleanup.
- Impact: high if leak exists; medium otherwise.
- Effort/complexity: medium.
- Acceptance criteria: all clipboard paths use guard type; tests/instrumentation prove close on error; no raw open/close pairs outside guard.

### 26. Git path canonicalization

- Label: Risk/inference.
- Priority: P1.
- Evidence/current state: path canonicalization for Git is listed as risk; Git commands accept folder/path inputs after validation.
- User scenario/problem: symlink/junction/case/`..` path lets command act outside intended repo or mis-stage file.
- Recommendation: canonicalize repo root and candidate paths, preserve display path separately, reject paths outside root after resolving symlinks/junctions where possible.
- Why worthwhile: prevents wrong-file operations.
- Why industry-standard apps include it: Git clients validate repo-relative paths before staging/checkout operations.
- Impact: high safety/security.
- Effort/complexity: medium.
- Acceptance criteria: tests for `..`, symlink/junction if feasible, case variants, UNC paths; rejected paths show clear error.

### 27. Operation cancellation and progress model

- Label: Risk/inference.
- Priority: P2.
- Evidence/current state: cancellation/progress risk identified across file ops, Git, ZIP, native drag, delete, external extraction.
- User scenario/problem: user starts long copy/extract/fetch then wants to stop or switch folders.
- Recommendation: use canonical unified operation center/job model from finding 14: job id, cancel token, progress events, stale UI guards, job history, and cleanup.
- Why worthwhile: one solution covers many weaknesses.
- Why industry-standard apps include it: file managers and IDEs treat long-running work as cancelable jobs.
- Impact: high cross-cutting improvement.
- Effort/complexity: high.
- Acceptance criteria: at least copy/delete/extract/Git fetch use shared job state; cancel result deterministic; UI survives navigation during job.

### 28. Smoke test coverage gap

- Label: Manual-validation gap.
- Priority: P1.
- Evidence/current state: existing `docs/smoke-test-windows.md` Stack section only checks opening, nav, context-menu fit, top-bar drops, and XButton back/forward.
- User scenario/problem: regressions in native properties, open-with, popup resize/focus, marquee, drag to Explorer, Git remote/auth, large folder/ZIP/network performance can ship unnoticed.
- Recommendation: expand smoke matrix below and require evidence capture for release signoff.
- Why worthwhile: Stack Browser mixes webview, Win32, filesystem, shell, Git, archive, and network path behavior; unit tests cannot cover all.
- Why industry-standard apps include it: native desktop apps maintain manual smoke matrices for shell integration.
- Impact: high confidence.
- Effort/complexity: medium.
- Acceptance criteria: checklist has owner/status/evidence columns; failed or blocked checks recorded; release notes distinguish manual gaps.

### 29. Stale docs

- Label: Verified source finding.
- Priority: P1.
- Evidence/current state: `C:\dev\jasonshell\stack_browser.md` exists at repo root. Lines 46-50 contain stale `Known Limits`; line 48 claims very large folders render accumulated rows directly rather than virtualizing them, conflicting with current `master_spec.md` and source behavior.
- User scenario/problem: future agent/engineer optimizes wrong problem or removes virtualization because docs lie.
- Recommendation: update or replace `C:\dev\jasonshell\stack_browser.md` to match `master_spec.md`; mark historical docs clearly; add doc test grep for false nonvirtualized claim.
- Why worthwhile: docs are part of architecture control.
- Why industry-standard apps include it: mature repos prevent stale design docs from contradicting current implementation.
- Impact: medium/high for future work quality.
- Effort/complexity: low.
- Acceptance criteria: no doc claims Stack rows are nonvirtualized; docs link to canonical spec; changelog policy followed when behavior changes.

### 30. Large folder and network path performance budgets

- Label: Manual-validation gap.
- Priority: P1.
- Evidence/current state: large folder/ZIP/network performance not covered by current smoke tests.
- User scenario/problem: Stack Browser works on dev folders but stalls on Downloads, node_modules, network shares, or huge archives.
- Recommendation: create synthetic/manual perf scenarios with baseline budgets to validate, then adjust from measured release-mode data: small folder first usable paint <= 500 ms; 10k local folder <= 1500 ms; 100k synthetic first page <= 3000 ms; network first visible result <= 5000 ms or explicit loading state by 500 ms; ZIP first index progress <= 1000 ms; path input p95 latency <= 50 ms while scans run; scroll >= 45 FPS on 10k entries; visible icon hydration p95 <= 1500 ms; cancel p95 <= 1000 ms; Stack popup memory delta <= 250 MB for 32 listing sessions until owner revises. Owners/TBD gate: QA owns manual evidence, frontend owns input/scroll/icon metrics, backend owns folder/ZIP/Git/file-op timings and memory.
- Why worthwhile: performance regressions otherwise invisible.
- Why industry-standard apps include it: Explorer/VS Code-like apps optimize large directory browsing explicitly.
- Impact: high.
- Effort/complexity: medium.
- Acceptance criteria: budgets recorded as initial baselines-to-validate or owner-approved TBD gates; blocked cases explicit; release-mode/manual evidence captured; dev diagnostics not used as release acceptance.

## Product ideas and feature opportunities

These are not automatic ship recommendations. Evaluate against JasonShell product fit: lightweight shell popup vs full Explorer replacement.

### Watcher/live refresh

- Label: Product idea.
- Priority: P2.
- Evidence/current state: stale Git/file views possible; no runtime validation.
- User scenario/problem: external tool changes current folder; Stack list stale.
- Recommendation: add file watcher with debounce, pause on active edit/selection, stale badge, manual refresh.
- Why worthwhile: current folder stays trustworthy.
- Why industry-standard apps include it: Explorer, Finder, VS Code refresh when filesystem changes.
- Impact: high for active dev folders.
- Effort/complexity: medium/high.
- Acceptance criteria: external create/delete appears or stale badge shows within budget; no selection loss during rename/delete.

### Fast search and content search

- Label: Product idea.
- Priority: P3.
- Evidence/current state: Stack has folder browsing and path autocomplete; content search listed as idea.
- User scenario/problem: user wants find file/text inside current tree without opening global search.
- Recommendation: add current-folder name search first, optional content search via ripgrep-like backend with cancel/progress/excludes.
- Why worthwhile: common file manager/developer workflow.
- Why industry-standard apps include it: VS Code, Finder, Explorer, Directory Opus include local search.
- Impact: high if Stack targets dev workflows.
- Effort/complexity: medium for name search, high for content search.
- Acceptance criteria: search cancel works; ignores `.git/node_modules` by default or setting; results open/reveal path.

### Preview/Quick Look pane

- Label: Product idea.
- Priority: P3.
- Evidence/current state: preview not listed as current Stack feature.
- User scenario/problem: user wants inspect image/text/PDF quickly without opening app.
- Recommendation: add optional preview pane for text/images/basic metadata; defer complex formats.
- Why worthwhile: reduces app switching.
- Why industry-standard apps include it: Finder Quick Look, Explorer preview pane, Files preview features.
- Impact: medium/high.
- Effort/complexity: medium/high depending formats.
- Acceptance criteria: preview respects file size limits; binary unknowns show metadata only; no UI freeze on large files.

### Action palette and command registry

- Label: Product idea.
- Priority: P3.
- Evidence/current state: many hidden actions/shortcuts.
- User scenario/problem: user cannot find command; developer duplicates enable/disable logic.
- Recommendation: implement typed command registry powering toolbar, context menus, shortcut help, and palette.
- Why worthwhile: improves discoverability and maintainability.
- Why industry-standard apps include it: VS Code and IDEs centralize commands.
- Impact: high leverage.
- Effort/complexity: medium.
- Acceptance criteria: every action has id, label, shortcut, enabled predicate, telemetry tag; palette filters actions.

### Recent, favorites, tabs, split, dual pane

- Label: Product idea.
- Priority: P3/P4.
- Evidence/current state: pinned folders and nav history exist; split/dual-pane not current.
- User scenario/problem: power user copies between locations repeatedly.
- Recommendation: add recents/favorites first; evaluate tabs/split/dual pane only if Stack becomes file-manager replacement.
- Why worthwhile: faster navigation and two-location workflows.
- Why industry-standard apps include it: Finder tabs, Explorer tabs, Directory Opus/Total Commander dual panes.
- Impact: medium to high for power users.
- Effort/complexity: low/medium for recents, high for split/dual pane.
- Acceptance criteria: recents bounded and privacy-aware; split panes have independent selection/cwd; drag between panes tested.

### Conflict resolution and undo journal

- Label: Product idea.
- Priority: P1/P2.
- Evidence/current state: conflict/rollback gaps identified.
- User scenario/problem: paste/extract overwrites existing files or deletes wrong item.
- Recommendation: add Explorer-style conflict dialog, apply-to-all choices, rename/skip/replace, and undo journal for recent file ops where feasible.
- Why worthwhile: prevents accidental overwrite/delete.
- Why industry-standard apps include it: every mature file manager handles conflicts and undo.
- Impact: very high safety.
- Effort/complexity: high.
- Acceptance criteria: conflict tests for file/folder collisions; undo restores deleted/moved items when possible; irreversible cases clearly labeled.

### Customizable columns/views

- Label: Product idea.
- Priority: P4.
- Evidence/current state: fixed columns/responsive weakness.
- User scenario/problem: user wants image dimensions, Git status, modified date, size, extension, owner, or compact cards.
- Recommendation: add view presets and column chooser after responsive foundation.
- Why worthwhile: adapts to different workflows.
- Why industry-standard apps include it: Explorer/Finder/Directory Opus support view customization.
- Impact: medium.
- Effort/complexity: medium.
- Acceptance criteria: column prefs persist; keyboard resize/sort works; hidden columns do not break accessibility.

### Bulk rename

- Label: Product idea.
- Priority: P4.
- Evidence/current state: inline rename exists; bulk rename not listed.
- User scenario/problem: user needs rename many files with pattern.
- Recommendation: add preview-first bulk rename with regex/template/date counters and undo.
- Why worthwhile: high power-user value.
- Why industry-standard apps include it: advanced file managers include bulk rename; VS Code-like tools support multi-file rename workflows.
- Impact: medium for power users.
- Effort/complexity: medium/high.
- Acceptance criteria: preview required before apply; invalid names highlighted; undo/journal support.

### Archive improvements

- Label: Product idea.
- Priority: P3.
- Evidence/current state: ZIP browsing/extraction exists; full scan each open and external extraction issues noted.
- User scenario/problem: user wants browse/extract selected files, preview archive, handle password/corrupt files safely.
- Recommendation: cached index, selected extraction, progress, zip-slip hardening, password error handling, archive create support only if product fit.
- Why worthwhile: makes archive browsing production-grade.
- Why industry-standard apps include it: Explorer and archive tools expose targeted extract/progress/errors.
- Impact: medium/high.
- Effort/complexity: high.
- Acceptance criteria: selected extract works; path traversal rejected; corrupt/password archives show clear state.

### Git workbench expansion

- Label: Product idea.
- Priority: P2/P3.
- Evidence/current state: Git Changes/Log/Tree/Branches plus fetch/pull/push/checkout/create branch exist; dense/stale/auth gaps noted.
- User scenario/problem: developer wants commit review without opening separate Git client.
- Recommendation: add ahead/behind badge, auth/error states, diff/hunks, stash, conflict view, branch compare, remote status, amend only later.
- Why worthwhile: Stack Browser becomes useful dev surface.
- Why industry-standard apps include it: VS Code and Git clients include diff/hunks/stash/conflicts because commit quality depends on review.
- Impact: high for developer target.
- Effort/complexity: high.
- Acceptance criteria: diff view handles binary/large files; hunk stage/unstage tested; conflict files clearly marked; push/pull auth failures actionable.

### Accessibility, localization, settings

- Label: Product idea.
- Priority: P2/P4.
- Evidence/current state: a11y gaps verified/risked; settings/localization not described for Stack-specific behavior.
- User scenario/problem: user needs larger text, reduced motion, high contrast, language support, or custom defaults.
- Recommendation: prioritize accessibility settings (density/font/reduced motion/high contrast) before full localization. Externalize strings when UI stabilizes.
- Why worthwhile: broadens usability and reduces hardcoded UX assumptions.
- Why industry-standard apps include it: OS-integrated desktop apps respect accessibility and locale expectations.
- Impact: medium/high.
- Effort/complexity: medium for settings, high for full localization.
- Acceptance criteria: density/font setting persists; reduced motion respected; string extraction plan exists.

## Measurement budgets to establish

Initial numeric targets below are baselines to validate, not proven release guarantees. If a target is not yet technically measurable, assign owner and TBD gate before release signoff.

- P1 first-open current folder: small folder <= 500 ms first usable paint; 10k local folder <= 1500 ms; 100k synthetic first page <= 3000 ms; network first visible result <= 5000 ms or loading state by 500 ms; ZIP first index progress <= 1000 ms.
- Input latency: path typing/autocomplete p95 <= 50 ms while folder scan/icon hydration runs.
- Scroll: virtual list maintains >= 45 FPS in 10k-entry synthetic folder; 100k-entry target TBD after release-mode measurement.
- Icon hydration: visible rows hydrate first; visible-row p95 <= 1500 ms; backlog bounded; no duplicate unbounded requests.
- Memory: initial Stack popup delta <= 250 MB for 32 paging sessions plus large ZIP cache until backend/frontend owners revise with evidence.
- Git: status/log/branch/fetch timeout baseline <= 30 s; max captured stdout/stderr bytes TBD by Git wrapper owner; auth/non-fast-forward/offline cases return typed errors.
- File ops: cancel p95 <= 1000 ms after cancel request for job-backed ops; progress events 2-10 Hz; stale UI result rejection tested.
- Accessibility: keyboard-only completion time for nav/open/rename/delete/context menu/Git commit recorded as manual baseline; pass gate is no keyboard trap and documented focus return.

## Comprehensive native smoke matrix

Add to Windows smoke checklist or linked Stack-specific doc.

| Area | Manual check | Evidence needed |
|---|---|---|
| Open/close | open pinned folder, close via red X, reopen same folder | pass/fail, screenshot optional |
| Resize | resize popup narrow/wide/tall, restart, verify persisted geometry | dimensions before/after |
| Focus | popup focus loss hides only when expected; native dialogs do not bury behind topmost popup | notes |
| Navigation | back/forward/up/XButton/path segments/path textbox | pass/fail |
| Autocomplete | RightArrow accept, Tab cycle, Escape, keyboard-only | pass/fail plus issues |
| Selection | click, Ctrl, Shift range, marquee blank-area selection | pass/fail |
| Context menu | row/background/menu fit, keyboard arrows/Home/End/Escape/focus return | pass/fail |
| File ops | new folder, rename, copy, cut, paste, delete, multi-delete partial failure | temp dir evidence |
| Native Properties | row Properties opens Explorer properties above popup and returns focus safely | pass/fail |
| Open With | default open, open-with menu/action, missing app/error path | pass/fail |
| Drag/drop in | Explorer folder drop to top bar/popup valid/invalid paths | pass/fail |
| Drag/drop out | drag Stack selection to Explorer/Desktop, cancel drag, repeat | pass/fail |
| Cross-volume move | temp drives/USB/VHD if available, failure simulation if possible | exact result |
| ZIP browse | small ZIP, large ZIP, corrupt ZIP, nested dirs, extract selected/all | timings/errors |
| Network path | UNC/share folder open, slow share behavior, disconnect mid-read | timings/errors |
| Large folder | 10k/100k files synthetic, sort/filter/scroll/icon hydration | timings/memory |
| Git local | status, stage, commit, log, tree, branch checkout/create | pass/fail |
| Git remote | fetch/pull/push success, auth required, offline, non-fast-forward | errors visible |
| Git conflicts | conflict repo state, stale view after external commit | pass/fail |
| A11y | keyboard-only file open/rename/delete/Git commit; screen reader spot-check | notes |
| High DPI | 125/150/200 percent scaling, min-width readability | screenshots optional |
| High contrast | Windows high contrast theme, focus rings, icons/text | pass/fail |
| Reduced motion | animations/scrolling do not cause issue | pass/fail |
| Multi-monitor | popup anchoring near screen edges, context menu clamping | pass/fail |

## Docs, tests, and observability recommendations

- Verified source finding: `master_spec.md` is canonical; keep behavior changes there, not per-request logs.
- Verified source finding: `docs/smoke-test-windows.md` Stack coverage is too shallow for current feature set.
- Recommendation: create Stack-specific smoke doc or expand section with matrix above.
- Recommendation: add source-contract tests for stale docs, IPC command matrix, ARIA attributes, menu keyboard reducer, cache byte budgets, Git timeout/error mapping, ZIP zip-slip rejection, path canonicalization.
- Recommendation: add diagnostic counters: folder read duration, entry count, icon queue/backlog/in-flight, ZIP index duration/cache hit, Git command duration/status, file-op job progress, memory cache bytes.
- Recommendation: redact sensitive paths by default in diagnostics.

## Phased roadmap

### Phase 1: stabilize and safety

- P1: Git timeouts/noninteractive errors.
- P1: file operation job journal and cross-volume verification design.
- P1: IPC invoke exposure/capability assurance audit.
- P1: clipboard RAII and Git path canonicalization.
- P1: external extraction timeout/cancel.
- P1: stale docs fix.

### Phase 2: responsive UI and accessibility

- P1: path autocomplete combobox semantics.
- P1: context menu keyboard model and focus return.
- P1: modal focus trap/return.
- P1/P2: toolbar redesign, responsive columns, readable typography, larger close hit target.
- P2: visible status/error/progress region with `aria-live`.

### Phase 3: performance and jobs

- P1: async folder reads/path suggestions.
- P1: ZIP index/cache/lazy children.
- P2: byte-budgeted paging/session caches.
- P2: canonical unified operation center with cancel/retry/history.
- P2: measurement harness and native smoke perf budgets.

### Phase 4: major features, only if product fit

- Watcher/live refresh.
- Current-folder search/content search.
- Preview/Quick Look.
- Action palette and shortcut help.
- Recents/favorites/tabs/split/dual-pane.
- Conflict resolution/undo journal.
- Custom columns/views.
- Bulk rename.
- Git diff/hunks/stash/conflicts/ahead-behind.
- Accessibility settings/localization.

## Dependency notes

- Canonical unified operation center depends on backend job ids/cancel tokens/progress events.
- Cross-volume safety depends on operation journal and conflict policy.
- Responsive toolbar benefits from command registry/action model.
- Menu/a11y work benefits from splitting monolith and centralizing focus/menu primitives.
- ZIP performance depends on archive index cache and byte budgets.
- Git workbench expansion depends on timeout/error model and stale-view refresh.
- Native smoke confidence depends on app actually running under Tauri on Windows; no runtime observation currently exists for this audit.

## Final recommendation

Do not treat Stack Browser as broken. Current source indicates strong foundation and many advanced capabilities. Main risk is breadth without enough safety assurance, accessibility, runtime validation, and maintainability boundaries. Best next move: fix P1 safety/docs/a11y/perf foundations before shipping optional Explorer-replacement features.
