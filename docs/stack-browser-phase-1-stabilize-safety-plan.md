# Stack Browser Phase 1 Stabilize Safety Plan

## 1. Metadata

- Status: Draft.
- Readiness: Implementation-ready after approval gate.
- Approval gate: Human maintainer must approve this plan before code changes.
- Date: 2026-08-16.
- Scope: Phase 1 safety/stabilization only.
- Canonical behavior source: `master_spec.md` only.
- Canonical policy source: `CHANGELOG_POLICY.md` for changelog rules only.
- Supporting current-source refs for this plan: `src/ipc/commands.ts`, `src-tauri/src/main.rs`, `src-tauri/src/contracts.rs`, `src/lib/stackPopup.ts`, current Svelte callsites, `src-tauri/src/stack_popup.rs`, `src-tauri/src/stack_popup/git_status.rs`, `src-tauri/src/stack_popup/clipboard.rs`, `src-tauri/src/stack_popup/file_ops.rs`, `src-tauri/src/stack_popup/paging.rs`, `src-tauri/capabilities/stack-popup.json`, `src-tauri/Cargo.toml`, and focused tests.
- Stale supporting doc: root `stack_browser.md` is not canonical; use only as stale-doc cleanup target after implementation.
- Intended readers: implementation agents.
- Intended readers: QA agents.
- Intended readers: security reviewers.
- Intended readers: documentation agents.
- Intended readers: maintainer approving Phase 1.
- ASCII only: yes.
- No code implementation in this file: yes.
- No changes outside this file in this request: yes.

## 2. Executive summary

- Phase 1 hardens Stack Browser trust boundaries, subprocess behavior, Git path safety, clipboard ownership, and paste recovery.
- Phase 1 must keep existing IPC shapes stable: external command results remain `Result<_, String>`.
- Phase 1 must not build Phase 3 unified job/operation-center UI, events, or history.
- Phase 1 must not add public archive cancel IPC.
- Phase 1 must preserve command-thread blocking boundaries already documented in `master_spec.md:49`.
- Main risk 1: verified assurance gap / privileged risk: current source lacks backend caller-label guards for Stack/pin/terminal commands; no wrong-window runtime exploit test has been completed.
- Main risk 2: Git and archive child processes can stall, flood output, or outlive direct children.
- Main risk 3: Git path validation is string-prefix based in places and needs canonical root/candidate handling for case, UNC, junction, reparse, and deleted leaf scenarios.
- Main risk 4: Windows clipboard write can leak/leave clipboard open on `?` after successful `OpenClipboard`, and can publish CF_HDROP before Preferred DropEffect.
- Main risk 5: cut/move fallback copies then deletes source, but lacks journal/verification, so crash can leave ambiguous split state.
- Main risk 6: stale docs after implementation would mislead future agents unless fixed per policy.

## 3. Current baseline with evidence refs

- Evidence B-001: `master_spec.md:45` says Stack Browser is hidden persistent `stack-popup` opened from top-bar pinned folders.
- Evidence B-002: `master_spec.md:45` lists file operations, delete confirmation, rename/new folder, drag/drop, context menus, Open With, archive/properties behavior.
- Evidence B-003: `master_spec.md:49` says `paste_stack_items` is async and uses `spawn_blocking`; preserve this.
- Evidence B-004: `master_spec.md:49` says `delete_stack_item` validates before spawn and preserves focus-loss hold behavior.
- Evidence B-005: `master_spec.md:49` says `extract_stack_archive` validates/builds `ArchiveExtractionPlan`, then runs `run_archive_extraction_plan` behind `spawn_blocking`.
- Evidence B-006: `master_spec.md:49` says archive process error text remains `Failed to extract archive: {error}` or `Archive extraction failed with status {status}`.
- Evidence B-007: `master_spec.md:25` says `npm run test:node` and `npm run validate` fail only unrelated `tests/stackPopupState.test.mjs` sort assertions.
- Evidence B-008: exact unrelated failing test name: `sorts stack entries deterministically while preserving folders first`.
- Evidence B-009: exact unrelated failing test name: `sorts modified desc with folders/files grouped by timestamp and nulls last`.
- Evidence B-010: root `stack_browser.md:48` has stale nonvirtualized claim: `Very large folders are fully enumerated through backend pages, but the current UI still renders the accumulated rows directly rather than virtualizing them.`
- Evidence B-011: `src-tauri/capabilities/stack-popup.json:4-6` scopes capability to window `stack-popup`.
- Evidence B-012: `src-tauri/capabilities/stack-popup.json:7-10` grants only `core:default` and `core:window:default`; no custom command authorization there.
- Evidence B-013: `src-tauri/Cargo.toml:11-20` dependencies include base64, libloading, png, portable-pty, raw-window-handle, serde, serde_json, tauri, zip.
- Evidence B-014: `src-tauri/Cargo.toml:11-20` has no wait-timeout crate.
- Evidence B-015: `src-tauri/src/stack_popup.rs:272-277` registers `show_stack_popup` without caller window arg.
- Evidence B-016: `src-tauri/src/stack_popup.rs:353-354` `get_stack_git_status(path)` delegates to `git_status::stack_git_status_for_path_async`.
- Evidence B-017: `src-tauri/src/stack_popup.rs:419-429` Git add/commit commands preserve `Result<StackGitOperationResult, String>`.
- Evidence B-018: `src-tauri/src/stack_popup.rs:792-815` `extract_stack_archive` is async and `spawn_blocking`.
- Evidence B-019: `src-tauri/src/stack_popup.rs:817-829` archive uses `Command::new(...).status()` with no timeout.
- Evidence B-020: `src-tauri/src/stack_popup/git_status.rs:14-98` Git async wrappers use `spawn_blocking`.
- Evidence B-021: `src-tauri/src/stack_popup/git_status.rs:445-455` `git_stdout_bytes` maps any nonzero git status to `Ok(None)`.
- Evidence B-022: `src-tauri/src/stack_popup/git_status.rs:458-505` `run_git` and `run_git_with_stdin` preserve stderr/status but no timeout/caps.
- Evidence B-023: `src-tauri/src/stack_popup/git_status.rs:403-428` pathspec validation requires absolute path and `starts_with(repo_root)`, then NUL pathspec stdin later.
- Evidence B-024: `src-tauri/src/stack_popup/git_status.rs:431-437` `nul_joined_pathspecs` retains NUL pathspec stdin behavior.
- Evidence B-025: `src-tauri/src/stack_popup/file_ops.rs:251-276` move fallback copies successfully, then deletes source; do not claim partial-copy source deletion.
- Evidence B-026: `src-tauri/src/stack_popup/clipboard.rs:153-229` clipboard read opens, runs closure, then closes; close error can override read result.
- Evidence B-027: `src-tauri/src/stack_popup/clipboard.rs:301-311` clipboard write opens, empties, sets CF_HDROP, registers Preferred DropEffect, sets effect, closes; `?` before close can skip `CloseClipboard`.
- Evidence B-028: `src-tauri/src/stack_popup/clipboard.rs:304-309` write currently sets CF_HDROP before Preferred DropEffect.
- Evidence B-029: `src-tauri/src/stack_popup/clipboard.rs:40-63` paste snapshots destination/clipboard and updates cut clipboard after join.
- Evidence B-030: `src-tauri/src/stack_popup/clipboard.rs:104-136` paste processes each source and returns pasted/failures.

## 4. Goals

- G-001: Add caller-label authorization to privileged backend commands.
- G-002: Keep capabilities JSON as defense-in-depth, not sole authorization.
- G-003: Add central authorization matrix and helper.
- G-004: Add std-only bounded subprocess runner.
- G-005: Apply runner to Stack Git and archive extraction.
- G-006: Preserve external IPC result/error shape.
- G-007: Use internal typed errors for subprocess/auth/path/Git failure classification.
- G-008: Canonicalize Git repository roots and request paths safely.
- G-009: Preserve NUL pathspec stdin.
- G-010: Add RAII wrappers for Windows clipboard write/read safety.
- G-011: Publish Preferred DropEffect before CF_HDROP.
- G-012: Add backend-only per-operation recovery journal for paste copy/cut fallback paths.
- G-013: Keep logs/export privacy-safe by redacting full paths outside private local journal.
- G-014: Update stale docs only after implementation.
- G-015: Provide deterministic tests and smoke path before runtime claims.

## 5. Non-goals

- NG-001: Do not build Phase 3 unified operation center.
- NG-002: Do not build Phase 3 job UI.
- NG-003: Do not build Phase 3 event stream for operation progress.
- NG-004: Do not build Phase 3 persistent user-facing history.
- NG-005: Do not add public archive cancel command.
- NG-006: Do not introduce object-shaped IPC errors.
- NG-007: Do not add new crate unless std path fails and maintainer approves escalation.
- NG-008: Do not journal Recycle Bin delete.
- NG-009: Do not journal archive extraction.
- NG-010: Do not implement automatic rollback.
- NG-011: Do not implement automatic source deletion on recovery.
- NG-012: Do not implement recovery UI.
- NG-013: Do not claim metadata fidelity for copy/move.
- NG-014: Do not hash file contents in journal.
- NG-015: Do not add auth kill switch.

## 6. Invariants

- INV-001: Existing IPC command names remain stable.
- INV-002: Existing frontend wrappers continue to receive `Result<_, String>` failure strings.
- INV-003: Stack Browser file operations keep spawn_blocking boundaries where already present.
- INV-004: Archive extraction remains spawn_blocking but gains subprocess timeout/resource safety.
- INV-005: Git argv remains fixed; no shell interpolation.
- INV-006: Git pathspecs remain NUL-delimited through stdin.
- INV-007: Caller auth fails closed for unclassified commands.
- INV-008: Capabilities stay present and window-scoped, but backend guards are authoritative for custom commands.
- INV-009: Journals are private app-local backend artifacts, not user-facing operation history.
- INV-010: Disabling emergency journal never deletes existing artifacts.
- INV-011: Logs never include full source/destination paths from journals.
- INV-012: Recovery classification never performs speculative repair.

## 7. RFC2119 requirements

- REQ-AUTH-001: Backend commands MUST inject `tauri::WebviewWindow` where caller label is authorization input.
- REQ-AUTH-002: Central helper MUST compare `window.label()` against command allowlist.
- REQ-AUTH-003: Helper MUST return stable string `Unauthorized caller for command {command}` at IPC boundary.
- REQ-AUTH-004: Helper MUST log rejected command without full path args.
- REQ-AUTH-005: Helper MUST fail closed for missing matrix entry.
- REQ-AUTH-006: There MUST NOT be an auth kill switch.
- REQ-AUTH-007: Capability JSON MAY remain unchanged except comments/docs elsewhere; backend guard is required.
- REQ-PROC-001: Subprocess runner MUST use std only initially.
- REQ-PROC-002: Runner MUST drain stdout/stderr concurrently.
- REQ-PROC-003: Runner MUST retain max 64 KiB stdout and 64 KiB stderr.
- REQ-PROC-004: Runner MUST drain excess bytes without unbounded memory growth.
- REQ-PROC-005: Runner MUST set deadline and poll child completion.
- REQ-PROC-006: Runner MUST kill direct child on timeout.
- REQ-PROC-007: Windows fallback MUST resolve trusted `taskkill.exe` dynamically with Win32 `GetSystemDirectoryW`, or validated canonical `%SystemRoot%\System32` fallback; never use PATH lookup or hardcoded `C:\Windows`.
- REQ-PROC-008: Runner MUST use bounded post-kill wait.
- REQ-PROC-009: Git commands MUST set `GIT_TERMINAL_PROMPT=0`.
- REQ-PROC-010: Git commands MUST set `GCM_INTERACTIVE=never`.
- REQ-PROC-011: Git read probes default timeout MUST be 10s.
- REQ-PROC-012: Git local mutation default timeout MUST be 30s.
- REQ-PROC-013: Git remote default timeout MUST be 90s.
- REQ-PROC-014: Archive default timeout MUST be 10min.
- REQ-PROC-015: Optional Git timeout env overrides MUST clamp 1s to 10min.
- REQ-PROC-016: Optional archive timeout env override MUST clamp 30s to 1h.
- REQ-GIT-001: Internal Git errors MUST include kinds spawn, stdin, timeout, canceled/internal, nonzero, authRequired, conflict, nonFastForward, notRepository.
- REQ-GIT-002: `git_stdout_bytes` replacement MUST return None only for explicitly designated optional probe behavior.
- REQ-GIT-003: Operational Git errors MUST preserve bounded stderr and exit status internally.
- REQ-GIT-004: Repo root MUST be canonicalized.
- REQ-GIT-005: Existing candidate paths MUST canonicalize and verify under canonical root.
- REQ-GIT-006: Missing/deleted leaf staging paths MUST resolve nearest existing ancestor.
- REQ-GIT-007: Missing/deleted leaf staging paths MUST exactly match fresh backend Git status path set before stage/commit.
- REQ-GIT-008: Case, UNC, junction, and reparse behavior MUST be covered by tests where platform permits.
- REQ-CLIP-001: Clipboard access MUST use `ClipboardSession` RAII.
- REQ-CLIP-002: Global locks MUST use `GlobalLockGuard` RAII.
- REQ-CLIP-003: Owned movable memory MUST use `OwnedGlobalMem` until transfer.
- REQ-CLIP-004: Preferred DropEffect MUST be set before CF_HDROP.
- REQ-CLIP-005: Owned memory MUST disarm only after successful `SetClipboardData` transfer.
- REQ-CLIP-006: Drop MUST close/free/unlock as appropriate.
- REQ-JRN-001: Journal scope MUST be only copy and cut/move fallback paths used by paste.
- REQ-JRN-002: Journal files MUST be per-operation atomic JSON under app-local `stack-browser-recovery/`.
- REQ-JRN-003: Journal schema MUST be versioned.
- REQ-JRN-004: Full paths MAY exist only in private local journal.
- REQ-JRN-005: Logs/export MUST redact full paths.
- REQ-JRN-006: Journal MUST start before mutation.
- REQ-JRN-007: Journal MUST record selected collision destination.
- REQ-JRN-008: Journal MUST record planned, copiedVerified, deleteStarted, sourceRemoved, failed, completed states as applicable.
- REQ-JRN-009: File copy verification MUST compare `fs::copy` byte count to source length.
- REQ-JRN-010: Directory copy verification MUST record recursive manifest counts and bytes.
- REQ-JRN-011: Stale running record MUST classify `interrupted`.
- REQ-JRN-012: Stale running handling MUST preserve artifact.
- REQ-JRN-013: Cleanup MUST respect retention and never delete non-journal artifacts.
- REQ-DOC-001: After implementation, update root `stack_browser.md`.
- REQ-DOC-002: After implementation, update `docs/smoke-test-windows.md`.
- REQ-DOC-003: After implementation, update `master_spec.md` relevant current behavior.
- REQ-DOC-004: After implementation, update `changelog.md` per `CHANGELOG_POLICY.md`.

## 8. NFRs with measurable values

- NFR-001: Caller auth overhead <= 1 ms per command in normal path.
- NFR-002: Process output retained memory <= 128 KiB per child plus small metadata.
- NFR-003: Process drain threads terminate within post-kill wait plus 2s under timeout tests.
- NFR-004: Git probe timeout default 10s +/- 500ms plus process cleanup overhead in tests.
- NFR-005: Git local mutation timeout default 30s; test may use clamped env override.
- NFR-006: Git remote timeout default 90s; test may use clamped env override.
- NFR-007: Archive timeout default 600s; test may use clamped env override.
- NFR-008: Journal atomic write uses unique same-dir temp file, file flush, Windows replace-capable atomic operation (`MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH` or approved equivalent), and best-effort parent-dir durability; no torn valid JSON on crash simulation.
- NFR-009: Journal write overhead for single file paste <= 100 ms on normal local disk excluding copy time.
- NFR-010: Journal cleanup scans at most configured journal dir and ignores unknown extensions.
- NFR-011: Private journal retention default 14 days for completed/failed/interrupted records.
- NFR-012: Running records older than 24h classify interrupted on startup or first paste maintenance.
- NFR-013: RED-first tests must fail before implementation for auth matrix, process timeout, Git path deletion, clipboard RAII source contract, and journal states.

## 9. Acceptance criteria and traceability

- AC-001 Given caller label `command-panel`, When it invokes `stack_git_push`, Then backend rejects with `Unauthorized caller for command stack_git_push`. Trace: REQ-AUTH-001..005.
- AC-002 Given caller label `stack-popup`, When it invokes `stack_git_push`, Then auth passes and command reaches Git layer. Trace: REQ-AUTH-001..005.
- AC-003 Given caller label `top-bar`, When it invokes pin list/pin/unpin/reorder/show/hide/focus holds/open VS Code commands, Then auth passes. Trace: matrix M-TOP.
- AC-004 Given caller label `terminal-panel`, When it uses terminal session commands with target `terminal-panel`, Then auth passes. Trace: matrix M-TERM.
- AC-005 Given caller label `terminal-panel`, When target label is `stack-popup`, Then auth rejects. Trace: REQ-AUTH-005.
- AC-006 Given unknown command lacks matrix row, When invoked from any label, Then auth helper fails closed in source-contract test. Trace: REQ-AUTH-005.
- AC-007 Given Git child writes > 64 KiB stderr, When command exits nonzero, Then returned string is bounded and process cannot OOM. Trace: REQ-PROC-002..004.
- AC-008 Given Git child hangs on credential prompt, When timeout elapses, Then runner kills child/tree and returns stable timeout string. Trace: REQ-PROC-006..010.
- AC-009 Given `git status` optional probe outside repo, When classified notRepository, Then status command returns `Ok(None)` only for that optional probe. Trace: REQ-GIT-001..003.
- AC-010 Given `git add` fails auth, conflict, non-fast-forward, or generic nonzero, When command returns, Then stable actionable string includes bounded stderr/status. Trace: REQ-GIT-001..003.
- AC-011 Given deleted Git file selected from fresh status, When staging, Then missing leaf path passes only if exact status path match exists. Trace: REQ-GIT-006..007.
- AC-012 Given deleted path not in fresh status, When staging, Then backend rejects. Trace: REQ-GIT-006..007.
- AC-013 Given path under junction escaping repo, When staging, Then canonical verification rejects. Trace: REQ-GIT-004..008.
- AC-014 Given clipboard write fails after `OpenClipboard`, When error returns, Then `CloseClipboard` still runs through RAII. Trace: REQ-CLIP-001.
- AC-015 Given Preferred DropEffect publish succeeds and CF_HDROP fails, When clipboard session remains open, Then implementation best-effort calls `EmptyClipboard` to clear partial publish, returns primary error plus cleanup failure if cleanup also fails, and does not free transferred DropEffect handle. Manual Windows smoke required. Trace: REQ-CLIP-004..006.
- AC-016 Given CF_HDROP publish succeeds, When ownership transfers, Then `OwnedGlobalMem` disarms. Trace: REQ-CLIP-005.
- AC-017 Given copy paste starts, When crash simulated after planned, Then journal reopens as interrupted and no auto-repair occurs. Trace: REQ-JRN-001..012.
- AC-018 Given file copy returns byte count != source length, When verification runs, Then journal marks failed and paste reports failure. Trace: REQ-JRN-009.
- AC-019 Given directory copy completes, When manifest counts/bytes recorded, Then completed journal has expected counts/bytes and no hashes. Trace: REQ-JRN-010.
- AC-020 Given emergency journal disable env is set, When paste runs, Then no journal is created and existing artifacts are not deleted. Trace: REQ-JRN-013.
- AC-021 Given implementation complete, When docs pass, Then stale root `stack_browser.md:48` claim is fixed. Trace: REQ-DOC-001.

## 10. Fixed architecture decisions

- AD-001: Phase 1 does not build unified operation-center/job UI/events/history.
- AD-002: Backend caller-label guards are required via injected `tauri::WebviewWindow`.
- AD-003: Central auth helper owns command-to-label matrix.
- AD-004: Tauri window-scoped capability JSON does not currently restrict custom invoke commands.
- AD-005: Capabilities remain defense-in-depth.
- AD-006: No auth kill switch.
- AD-007: External IPC compatibility remains `Result<_, String>`.
- AD-008: Internal typed enums model process/auth/path failures.
- AD-009: Command boundary converts typed errors to stable actionable strings.
- AD-010: Phase 1 does not introduce object-shaped IPC errors.
- AD-011: Subprocess runner is std-only unless implementation proves insufficiency and escalates.
- AD-012: Runner uses concurrent pipe drain, retained byte caps, deadline polling, child kill, trusted taskkill fallback, bounded post-kill wait.
- AD-013: Timeout defaults are fixed: Git probes/read 10s, local mutation 30s, remote 90s, archive 10min.
- AD-014: Env overrides are optional and clamped: Git 1s-10min, archive 30s-1h.
- AD-015: Git env must disable prompts with `GIT_TERMINAL_PROMPT=0` and `GCM_INTERACTIVE=never`.
- AD-016: Output cap is 64KiB stdout plus 64KiB stderr retained while draining excess.
- AD-017: Git optional-probe `None` is explicit behavior, not blanket nonzero suppression.
- AD-018: Git canonicalization uses canonical repo root and verified canonical candidate/ancestor.
- AD-019: Clipboard RAII wraps session, locks, and owned global memory.
- AD-020: Preferred DropEffect is set before CF_HDROP.
- AD-021: Recovery journal scope is paste copy and cut/move fallback only.
- AD-022: Journal is backend-only, app-local, per-operation atomic JSON, versioned.
- AD-023: No hashes, metadata fidelity claim, auto rollback, auto source deletion on recovery, recovery UI, or user-facing job history.

## 11. Caller authorization matrix

- Matrix rule M-000: WP0 MUST reconcile command inventories from `src/ipc/commands.ts`, `src-tauri/src/main.rs`, `src-tauri/src/contracts.rs`, `src/lib/stackPopup.ts`, and Svelte callsites before WP1.
- Matrix rule M-001: `open_stack_folder_in_vscode` is registered in `main.rs` and wrapped in `src/lib/stackPopup.ts`, but `contracts.rs` lacks both its command constant and its `commands::ALL` entry; WP0 blocker: add both and reconcile Rust/TS parity before WP1.
- Matrix rule M-002: Every Stack/pin/Git/terminal command below is in Phase 1 auth scope; non-Stack commands in `IPC_COMMANDS` are outside Phase 1.
- M-TOP-001: `list_pinned_stack_folders` allowed labels: `top-bar`, `stack-popup`.
- M-TOP-002: `pin_stack_folder` allowed labels: `top-bar`, `stack-popup`, `search-panel` only if current callsite still pins from search event; otherwise reject.
- M-TOP-003: `unpin_stack_folder` allowed labels: `top-bar`, `stack-popup`.
- M-TOP-004: `reorder_pinned_stack_folders` allowed labels: `top-bar`.
- M-TOP-005: `show_stack_popup` allowed labels: `top-bar`.
- M-TOP-006: `hide_stack_popup` allowed labels: `top-bar`, `stack-popup`.
- M-TOP-007: `begin_stack_popup_focus_loss_hold` allowed labels: `top-bar`, `stack-popup`.
- M-TOP-008: `end_stack_popup_focus_loss_hold` allowed labels: `top-bar`, `stack-popup`.
- M-STACK-001: `get_stack_popup_request` allowed labels: `stack-popup`.
- M-STACK-002: `resize_stack_popup` allowed labels: `stack-popup`.
- M-STACK-003: `read_stack_folder` allowed labels: `stack-popup`.
- M-STACK-004: `suggest_stack_paths` allowed labels: `stack-popup`.
- M-STACK-005: `resolve_stack_item_icons` allowed labels: `stack-popup`.
- M-STACK-006: `open_stack_item` allowed labels: `stack-popup`, `terminal-panel` only for proven terminal quick-select callsite.
- M-STACK-007: `open_stack_item_with_picker` allowed labels: `stack-popup`.
- M-STACK-008: `list_stack_open_with_candidates` allowed labels: `stack-popup`.
- M-STACK-009: `open_stack_item_with_app` allowed labels: `stack-popup`.
- M-STACK-010: `rename_stack_item` allowed labels: `stack-popup`.
- M-STACK-011: `copy_stack_items` allowed labels: `stack-popup`.
- M-STACK-012: `prepare_stack_file_drag` allowed labels: `stack-popup`.
- M-STACK-013: `cut_stack_items` allowed labels: `stack-popup`.
- M-STACK-014: `paste_stack_items` allowed labels: `stack-popup`.
- M-STACK-015: `delete_stack_item` allowed labels: `stack-popup`.
- M-STACK-016: `new_stack_folder` allowed labels: `stack-popup`.
- M-STACK-017: `new_stack_text_file` allowed labels: `stack-popup`.
- M-STACK-018: `reveal_stack_item` allowed labels: `stack-popup`, `terminal-panel` only for proven terminal quick-select/cwd callsite.
- M-STACK-019: `extract_stack_archive` allowed labels: `stack-popup`.
- M-STACK-020: `show_stack_item_properties` allowed labels: `stack-popup`.
- M-STACK-021: `open_stack_folder_in_vscode` allowed labels: `top-bar`, `stack-popup`, `terminal-panel`; requires contracts parity fix first.
- M-GIT-001: `get_stack_git_status` allowed labels: `stack-popup`.
- M-GIT-002: `open_stack_git_remote_url` allowed labels: `stack-popup`.
- M-GIT-003: `stack_git_add_paths` allowed labels: `stack-popup`.
- M-GIT-004: `stack_git_commit` allowed labels: `stack-popup`.
- M-GIT-005: `stack_git_log` allowed labels: `stack-popup`.
- M-GIT-006: `stack_git_tree` allowed labels: `stack-popup`.
- M-GIT-007: `stack_git_branches` allowed labels: `stack-popup`.
- M-GIT-008: `stack_git_fetch` allowed labels: `stack-popup`.
- M-GIT-009: `stack_git_pull` allowed labels: `stack-popup`.
- M-GIT-010: `stack_git_push` allowed labels: `stack-popup`.
- M-GIT-011: `stack_git_checkout_branch` allowed labels: `stack-popup`.
- M-GIT-012: `stack_git_create_branch` allowed labels: `stack-popup`.
- M-TERM-001: `start_persistent_terminal` allowed labels: `terminal-panel`; outside Stack Browser UI but Phase 1 terminal auth scope because same backend session registry.
- M-TERM-002: `start_stack_terminal` allowed labels: `terminal-panel` when request target is `terminal-panel`; `stack-popup` only for legacy `StackTerminalPane.svelte` callsite if retained.
- M-TERM-003: `read_stack_terminal` allowed labels: caller must match stored session target; session-derived target auth, never trust request target alone.
- M-TERM-004: `write_stack_terminal` allowed labels: caller must match stored session target.
- M-TERM-005: `resize_stack_terminal` allowed labels: caller must match stored session target.
- M-TERM-006: `stop_stack_terminal` allowed labels: caller must match stored session target.
- M-TERM-007: `poll_stack_terminal_session` allowed labels: caller must match stored session target; session-derived target auth, never trust request target alone.
- M-TERM-008: `list_stack_terminals` allowed labels: caller must equal requested `targetLabel`; absent target defaults to caller label.
- M-TERM-009: `rename_stack_terminal` allowed labels: caller must match stored session target.
- M-TERM-010: `stop_terminal_panel_sessions` allowed labels: `terminal-panel`.
- M-TERM-011: `get_stack_terminal_cwd` allowed labels: caller must match stored session target.
- M-TERM-012: `open_stack_terminal_here` allowed labels: `stack-popup`, `terminal-panel` only for proven terminal quick-select/cwd callsite.
- M-OUT-001: `show_terminal_panel` and `hide_terminal_panel` are terminal-panel lifecycle commands, not Stack session/file/Git commands, and are outside this Phase 1 Stack authorization helper. WP0 MUST record their `top-bar`/`terminal-panel` callsites and open a separate terminal-panel authorization follow-up if backend caller guards are absent; this exclusion does not classify them as globally safe.
- M-FAIL-001: Any unclassified current/future Stack, pin, Git, file, archive, Open With, properties, drag, terminal command fails closed.

## 12. Dependency graph

- DG-001: WP0 baseline/contracts blocks all implementation WPs.
- DG-002: WP1 auth depends on WP0.
- DG-003: WP2 process runner depends on WP0.
- DG-004: WP3 Git timeout/error depends on WP2.
- DG-005: WP4 Git paths depends on WP3 for shared typed errors, but path tests can be drafted after WP0.
- DG-006: WP5 archive depends on WP2.
- DG-007: WP6 clipboard RAII depends on WP0 only.
- DG-008: WP7 Phase 1b copy/move journal depends on WP0 and touches clipboard/file_ops only after WP6 clipboard ownership is merged.
- DG-009: WP8a privacy/docs/smoke verification for an explicitly approved Phase 1a release depends on WP1-WP6. WP8b full Phase 1 verification depends on WP1-WP7.
- DG-010: WP1 signature/auth pass lands before WP3/WP5 start editing command signatures.
- DG-011: WP2 owns runner API exclusively until frozen gate; WP3/WP5 consume frozen runner API and MUST NOT edit runner unless returned to WP2.
- DG-012: WP6 owns `clipboard.rs` first; WP7 may create/test journal module in parallel but cannot integrate into `clipboard.rs` or `file_ops.rs` until WP6 merge.
- DG-013: WP4 should integrate after WP3 to avoid Git churn conflicts.
- DG-014: Serialize writer scopes: no parallel writers on same file; explicit handoff required for `stack_popup.rs`, `git_status.rs`, `clipboard.rs`, `file_ops.rs`, runner module, and contracts.
- DG-015: Merge order: WP0 inventory/parity blocker, WP1 auth signatures/matrix, WP2 runner frozen gate, WP3 Git runner, WP4 Git paths, WP5 archive runner, WP6 clipboard RAII, WP7 Phase 1b journal core then integration, WP8 docs/validation.

## 13. Work packages

### WP0 Baseline contracts and inventory

- Owner agent type: Rust/Node source-contract tester.
- Allowed file scope: tests only plus this plan if correcting source refs.
- Source symbols/files: `src-tauri/src/contracts.rs`, `src-tauri/src/main.rs`, `src/lib/stackPopup.ts`, `src/components/StackPopupSurface.svelte`, `src/components/TopBar.svelte`, `src/components/TerminalPanelSurface.svelte`.
- Change step 1: Inventory all Stack Browser, pin, terminal, file, Git, archive, Open With, properties commands.
- Change step 2: Inventory current frontend callsites and source webview labels.
- Change step 3: Block until `open_stack_folder_in_vscode` command parity is reconciled between `main.rs`, TS wrappers, and `contracts.rs`.
- Change step 4: Write RED source-contract tests for auth matrix existence and fail-closed behavior.
- RED tests: add cases in `tests/stackBrowserPhase1Safety.test.mjs` expecting helper/matrix not yet present.
- RED tests: update `tests/backendBlockingLockBoundariesP6.test.mjs` to assert archive remains spawn_blocking and issue is timeout/resource safety.
- GREEN work: none in WP0 except tests can be committed later by implementation agent.
- Refactor: none.
- Acceptance gate: command inventory table reviewed and no command unclassified.
- Dependencies: none.

### WP1 Backend caller authorization

- Owner agent type: Rust IPC/security implementer.
- Allowed file scope: `src-tauri/src/stack_popup.rs`, new `src-tauri/src/stack_popup/auth.rs`, `src-tauri/src/contracts.rs` if tests require constants, focused tests.
- Change step 1: Add internal `CallerAuthError` typed enum.
- Change step 2: Add central helper `authorize_stack_command(window: &tauri::WebviewWindow, command: StackCommandAuth)`.
- Change step 3: Inject `tauri::WebviewWindow` into guarded Tauri command signatures.
- Change step 4: Call helper at command start before path/process side effects.
- Change step 5: For terminal commands, resolve target/session target and enforce equality with caller label.
- RED tests: source-contract test fails until command signatures include `WebviewWindow`.
- RED tests: Rust unit tests for matrix allow/deny.
- GREEN work: implement helper/matrix and boundary string conversion.
- Refactor: avoid per-command ad hoc string comparisons.
- Acceptance gate: all classified commands have tests; unclassified command path fails closed.
- Dependencies: WP0.

### WP2 Bounded subprocess runner

- Owner agent type: Rust systems implementer.
- Allowed file scope: new `src-tauri/src/process_runner.rs` or `src-tauri/src/stack_popup/process_runner.rs`, module registration, focused Rust tests.
- Change step 1: Define `ProcessRunSpec`, `ProcessTimeoutKind`, `ProcessRunOutput`, `ProcessRunError`.
- Change step 2: Spawn child with piped stdout/stderr as configured.
- Change step 3: Drain stdout/stderr concurrently into capped buffers while counting total bytes.
- Change step 4: Poll deadline using `try_wait` and sleep interval.
- Change step 5: Use reader threads plus completion channels; normal path calls `JoinHandle::join` only after completion signal proves reader finished.
- Change step 6: On timeout, kill direct child, then trusted tree kill, then bounded wait for process completion and reader completion signals.
- Change step 7: MUST NOT call blocking `JoinHandle::join` on timeout without a reader completion signal; Rust std has no timed thread join and fake bounded join is forbidden.
- Change step 8: If trusted tree kill fails and reader does not signal, return cleanup-incomplete internal error, detach handles, and record bounded thread leak risk as escalation/blocker, not successful acceptance.
- Change step 9: Start with feasibility spike; if deterministic cleanup cannot meet NFR std-only, STOP for maintainer-approved Win32 handle approach or dependency.
- Change step 10: Resolve trusted `taskkill.exe` with `GetSystemDirectoryW` or validated canonical `%SystemRoot%\System32` fallback; avoid PATH lookup and hardcoded `C:\Windows`.
- RED tests: command writes > cap; retained bytes capped and total count > cap.
- RED tests: command sleeps; timeout error returned and child exits.
- RED tests: inherited-pipe grandchild keeps stdout/stderr handle open; timeout path must kill tree or return cleanup-incomplete, never hang.
- RED tests: source contract rejects legacy literal System32 `taskkill.exe` paths and PATH lookup for taskkill.
- GREEN work: implement std-only runner.
- Refactor: keep API generic enough for Git and archive, not Phase 3 jobs.
- Acceptance gate: no wait-timeout crate added to `Cargo.toml`; no blocking join without completion signal; deterministic cleanup evidence or STOP escalation recorded.
- Dependencies: WP0.

### WP3 Git timeout/error classification

- Owner agent type: Rust Git backend implementer.
- Allowed file scope: `src-tauri/src/stack_popup/git_status.rs`, runner module, tests.
- Change step 1: Add `GitRunMode`: optional probe, read, local mutation, remote.
- Change step 2: Replace `Command::output`, `Command::spawn`/`wait_with_output` with runner.
- Change step 3: Set `GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never`.
- Change step 4: Map runner/process errors to internal `GitCommandError` kinds.
- Change step 5: Classify stderr/status patterns for authRequired, conflict, nonFastForward, notRepository.
- Change step 6: Convert at IPC boundary to stable strings.
- RED tests: `git_stdout_bytes` blanket nonzero to None source-contract fails.
- RED tests: prompt env vars source-contract fails before impl.
- GREEN work: implement typed internal runner adapters.
- Refactor: keep parsers unchanged.
- Acceptance gate: optional probes only return None for notRepository/explicit optional cases.
- Dependencies: WP2.

### WP4 Git path canonicalization

- Owner agent type: Rust path safety implementer.
- Allowed file scope: `src-tauri/src/stack_popup/git_status.rs`, optional `paths.rs` helper, tests.
- Change step 1: Canonicalize repo root from `rev-parse --show-toplevel`.
- Change step 2: Define canonical repo-relative comparison: canonical root from Git, existing candidate canonicalized, then normalized repo-relative slash path compared exactly to fresh `git status --porcelain=v1 -z` repo-relative set.
- Change step 3: For existing paths, canonicalize candidate and verify under canonical root; reject reparse escape.
- Change step 4: For missing/deleted leaf, reject NUL, ADS (`:` in Windows path component beyond drive prefix), `.`, `..`, prefix tricks, alternate prefix/device namespace tricks, and absolute/relative ambiguity before ancestor resolution.
- Change step 5: For missing/deleted leaf, resolve/canonicalize nearest existing ancestor, reject reparse escape, reconstruct normalized repo-relative slash path from ancestor plus validated missing segments.
- Change step 6: Compare reconstructed path against fresh `git status --porcelain=v1 -z` repo-relative set; parse rename records and include both old and new paths where porcelain emits old/new path.
- Change step 7: Windows case normalization policy: normalize drive/root and repo-relative comparisons with platform case-folding only where Windows filesystem/Git status semantics require it; preserve original-safe repo-relative bytes/OsString for pathspec stdin.
- Change step 8: Feed original-safe repo-relative bytes/OsString through NUL pathspec stdin; never shell-quote/interpolate.
- Change step 9: Document TOCTOU bounded by fresh status plus canonical ancestor checks, not eliminated.
- RED tests: deleted path not in status rejected.
- RED tests: exact deleted path in status accepted.
- RED tests: case/UNC/junction/reparse cases where feasible on Windows.
- RED tests: deleted rename status handles old/new path records correctly.
- RED tests: ADS, NUL, `.`/`..`, alternate prefix, and prefix-trick missing leaves rejected.
- GREEN work: implement canonical helpers and status-set verification.
- Refactor: keep tree path validation separate from stage path validation.
- Acceptance gate: no string-only `starts_with(repo_root)` remains for staging absolute paths.
- Dependencies: WP3.

### WP5 Archive subprocess timeout safety

- Owner agent type: Rust archive/process implementer.
- Allowed file scope: `src-tauri/src/stack_popup.rs`, runner module, tests.
- Change step 1: Keep `extract_stack_archive` validation/plan build on command path.
- Change step 2: Keep `spawn_blocking` around archive execution.
- Change step 3: Replace `Command::new(...).status()` with runner.
- Change step 4: Default archive timeout 10min; env override clamp 30s-1h.
- Change step 5: Preserve public archive error string family: timeout maps to `Failed to extract archive: timed out after {seconds}s`; this new stable behavior requires `master_spec.md` and `changelog.md` update during implementation.
- RED tests: source-contract detects raw `.status()` use in archive plan.
- GREEN work: use runner and map timeout to `Failed to extract archive: timed out after {seconds}s`.
- Refactor: no public cancel API.
- Acceptance gate: corrupt/password/nonzero archive returns bounded error.
- Dependencies: WP2.

### WP6 Clipboard RAII

- Owner agent type: Rust Win32 safety implementer.
- Allowed file scope: `src-tauri/src/stack_popup/clipboard.rs`, optional `clipboard_win.rs`, tests.
- Change step 1: Add `ClipboardSession` that opens in constructor and closes in Drop.
- Change step 2: Add `GlobalLockGuard` that unlocks in Drop.
- Change step 3: Add `OwnedGlobalMem` that frees unless disarmed.
- Change step 4: Allocate and fill DropEffect memory and HDROP memory via RAII.
- Change step 5: Set Preferred DropEffect first.
- Change step 6: Set CF_HDROP last.
- Change step 7: Disarm each memory handle only after successful `SetClipboardData`.
- Change step 8: If Preferred DropEffect succeeds and CF_HDROP fails while clipboard session remains open, call `EmptyClipboard` best-effort to clear partial publish; return primary error plus cleanup failure if cleanup fails; do not free transferred DropEffect handle.
- RED tests: source-contract catches `OpenClipboard(...)?` path without RAII close.
- RED tests: source-contract catches CF_HDROP before Preferred DropEffect.
- GREEN work: implement RAII and partial-publish contract.
- Refactor: keep non-Windows stubs unchanged.
- Acceptance gate: manual Windows clipboard smoke passes, including DropEffect success then CF_HDROP failure/cleanup behavior where test hook permits.
- Dependencies: WP0.

### WP7 Phase 1b copy/move recovery journal safety slice

- Owner agent type: Rust filesystem/recovery implementer.
- Phase note: This is deliberately substantial Phase 1b safety work inside overall Phase 1. It may be approved/released separately and must not silently cut required journal functionality.
- Product decision: Scope remains copy plus cut/move fallback used by paste; no Phase 3 UI/history/events.
- Release gate: Phase 1a auth/process/Git/archive/clipboard may ship without WP7 only by explicit maintainer approval that Phase 1b journal is deferred.
- Allowed file scope stage 1: new `recovery_journal.rs`, app path helper, focused tests only.
- Allowed file scope stage 2 after WP6 merge/exclusive handoff: `src-tauri/src/stack_popup/clipboard.rs`, `file_ops.rs`, journal integration tests.
- Change step 1: Feasibility/approval gate for journal perf and atomic persistence design.
- Change step 2: Add journal model and atomic writer in new module first.
- Change step 3: Create journal before each paste item mutation.
- Change step 4: Record selected collision destination.
- Change step 5: On copy file, verify `fs::copy` bytes equals source length.
- Change step 6: On copy dir, record recursive source manifest counts/bytes before copy and copied manifest counts/bytes after copy.
- Change step 7: On cut fallback, record deleteStarted before source removal and sourceRemoved after success.
- Change step 8: Mark failed/completed terminal state.
- Change step 9: Startup/maintenance classifies stale running as interrupted and logs only op id/count.
- RED tests: journal absent before mutation fails.
- RED tests: crash-transition fixtures classify interrupted.
- GREEN work: implement backend-only journal.
- Refactor: keep paste result shape unchanged.
- Acceptance gate: separate Phase 1b approval/perf gate passes; disabled env creates no new journal and deletes no old artifacts.
- Dependencies: WP0, WP6 coordination.

### WP8 Privacy/docs/smoke/final verification

- Owner agent type: QA/docs/security reviewer.
- Allowed file scope: docs, `master_spec.md`, `changelog.md`, tests if fixing source-contract names.
- Change step 1: Run focused tests and collect evidence.
- Change step 2: Run manual Windows smoke before any runtime safety claim.
- Change step 3: Update root `stack_browser.md` stale virtualization/nonvirtualized claim if implementation changes behavior or wording.
- Change step 4: Update `docs/smoke-test-windows.md` with Phase 1 smoke cases.
- Change step 5: Update `master_spec.md` current behavior sections.
- Change step 6: Append concise `changelog.md` entries per `CHANGELOG_POLICY.md`.
- RED tests: docs checklist test can fail before docs updated.
- GREEN work: docs and final validation.
- Refactor: remove stale plan references if contradicted by actual implementation.
- Acceptance gate: Definition of Done complete.
- Dependencies: WP1-WP6 for an explicitly approved Phase 1a verification pass; WP1-WP7 for full Phase 1 completion.

## 14. API/type pseudocode

- Auth pseudocode: `enum StackCommandAuth { StackOnly(&'static str), TopBarPin(&'static str), TerminalTargeted(&'static str), Shared(&'static str, &'static [&'static str]) }`.
- Auth pseudocode: `fn authorize(window: &WebviewWindow, command: StackCommandAuth) -> Result<(), AuthError>`.
- Auth error: `AuthError::Unauthorized { command, caller }`.
- Stable auth message: `Unauthorized caller for command {command}`.
- Process pseudocode: `ProcessRunSpec { program, args, cwd, envs, stdin, timeout, stdout_cap, stderr_cap, kill_tree }`.
- Process pseudocode: `ProcessRunOutput { status, stdout, stderr, stdout_truncated, stderr_truncated, stdout_total, stderr_total, duration }`.
- Process error kinds: `Spawn`, `Stdin`, `Timeout`, `Wait`, `Kill`, `Internal`.
- Git error pseudocode: `GitError { kind: GitErrorKind, status, stderr, stdout, command_class }`.
- Git stable message spawn: `Failed to run git: {error}`.
- Git stable message stdin: `Failed to write git input: {error}`.
- Git stable message timeout: `Git operation timed out after {seconds}s`.
- Git stable message auth: `Git authentication required; configure credentials and retry`.
- Git stable message conflict: use bounded stderr when actionable, else `Git conflict blocks operation`.
- Git stable message nonFastForward: use bounded stderr when actionable, else `Git operation rejected because branch is not fast-forward`.
- Git stable message notRepository: `Git repository unavailable` for operational commands; `None` for optional status probe.
- Git stable message generic nonzero: bounded stderr if non-empty, else `Git failed with status {status}`.
- Archive stable message spawn: `Failed to extract archive: {error}`.
- Archive stable message nonzero: `Archive extraction failed with status {status}`.
- Archive stable message timeout: `Failed to extract archive: timed out after {seconds}s` (new stable behavior; implementation requires `master_spec.md` and `changelog.md` update).
- Clipboard partial publish contract: if Preferred DropEffect publishes and CF_HDROP fails while clipboard session remains open, call `EmptyClipboard` best-effort to clear partial app publish; report CF_HDROP primary error plus cleanup error if any; transferred DropEffect handle ownership belongs to clipboard and must not be freed by `OwnedGlobalMem`.
- Clipboard partial publish contract: if CF_HDROP publishes, ownership transferred; subsequent close failure reports close error but must not free transferred memory.

## 15. Journal schema/model/table

- Directory: app-local data `stack-browser-recovery/`.
- File name: `{op_id}.json` where op_id is random or monotonic collision-resistant local identifier.
- Atomic temp name: unique per write, same dir, e.g. `{op_id}.{nonce}.json.tmp`; never reuse one fixed temp name across writes.
- Version: integer `1`.
- Record field: `operationId` string.
- Record field: `createdAtMs` integer.
- Record field: `updatedAtMs` integer.
- Record field: `phase` string.
- Record field: `mode` enum `copy` or `cut`.
- Record field: `sourcePath` string private full path.
- Record field: `destinationPath` string private full path.
- Record field: `selectedCollisionDestinationPath` string private full path.
- Record field: `sourceKind` enum `file` or `directory`.
- Record field: `sourceFileLength` integer optional.
- Record field: `copiedFileBytes` integer optional.
- Record field: `sourceManifest` object optional.
- Record field: `copiedManifest` object optional.
- Manifest field: `fileCount` integer.
- Manifest field: `dirCount` integer.
- Manifest field: `totalBytes` integer.
- Manifest field: `skippedUnsupportedCount` integer.
- Record field: `deleteStartedAtMs` optional integer.
- Record field: `sourceRemovedAtMs` optional integer.
- Record field: `failureKind` optional string.
- Record field: `failureMessage` optional redacted/actionable string.
- Record field: `completedAtMs` optional integer.
- Record field: `interruptedAtMs` optional integer.
- State `planned`: journal created before mutation; destination selected.
- State `copyStarted`: copy began.
- State `copiedVerified`: file byte count or directory manifest verified.
- State `deleteStarted`: cut fallback source removal began.
- State `sourceRemoved`: source removal completed.
- State `failed`: operation failed; artifact preserved.
- State `completed`: operation completed; no recovery action needed.
- State `interrupted`: stale running record discovered after crash/interruption.
- State transition: planned -> copyStarted.
- State transition: copyStarted -> copiedVerified.
- State transition: copyStarted -> failed.
- State transition: copiedVerified -> completed for copy.
- State transition: copiedVerified -> deleteStarted for cut fallback.
- State transition: deleteStarted -> sourceRemoved.
- State transition: deleteStarted -> failed.
- State transition: sourceRemoved -> completed.
- State transition: any nonterminal stale running -> interrupted.
- Idempotency: re-reading terminal records has no side effects.
- Idempotency: classifying stale running as interrupted writes only journal state, not file mutations.
- Idempotency: cleanup deletes only terminal journal files older than retention.
- Corruption handling: invalid JSON is renamed to `.corrupt` or left untouched with `.corrupt` marker; no repair.
- Corruption handling: log only op id/file count, not paths.
- Atomic write algorithm step 1: serialize full JSON to bytes.
- Atomic write algorithm step 2: create unique temp file in same directory with exclusive create.
- Atomic write algorithm step 3: write bytes.
- Atomic write algorithm step 4: flush file data/metadata.
- Atomic write algorithm step 5: replace final with Windows replace-capable atomic operation (`MoveFileExW` `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH` or approved equivalent); do not claim plain `std::fs::rename` replaces final on Windows.
- Atomic write algorithm step 6: best-effort parent dir durability documented; do not block core behavior if unsupported.
- Atomic write algorithm step 7: orphan temp cleanup deletes only matching same-dir temp names older than safe threshold, never follows reparse points, never deletes final journals.
- Cleanup default: completed/failed/interrupted older than 14 days eligible.
- Cleanup safety: never delete unknown extension.
- Cleanup safety: never follow symlink/reparse point in journal dir.
- Cleanup safety: never delete destination/source artifacts.

## 16. Edge cases and threat model

- Threat T-001: malicious `command-panel` invokes Stack Git mutation.
- Threat T-002: compromised `top-bar` invokes archive extraction.
- Threat T-003: terminal-panel tries target label `stack-popup`.
- Threat T-004: unclassified new command accidentally exposed.
- Threat T-005: OS clipboard contents are untrusted and may hold malformed HDROP.
- Threat T-006: Clipboard open succeeds then set data fails.
- Threat T-007: Clipboard global memory lock fails.
- Threat T-008: Git path uses different case on case-insensitive filesystem.
- Threat T-009: Git path uses UNC prefix.
- Threat T-010: Git path crosses junction out of repo.
- Threat T-011: Git path is symlink/reparse.
- Threat T-012: Git deleted file no longer exists but is staged/unstaged in status.
- Threat T-013: Git credential prompt hangs forever without prompt env.
- Threat T-014: Git hook stalls local mutation.
- Threat T-015: Git remote network stalls.
- Threat T-016: Git outputs hundreds of MiB.
- Threat T-017: Child spawns grandchild; direct kill insufficient.
- Threat T-018: `taskkill.exe` unavailable or blocked.
- Threat T-019: Archive corrupt/password-protected returns nonzero with huge stderr.
- Threat T-020: Archive extractor hangs.
- Threat T-021: Crash after journal planned.
- Threat T-022: Crash after copy start.
- Threat T-023: Crash after copiedVerified before source deletion.
- Threat T-024: Crash after deleteStarted.
- Threat T-025: Crash after sourceRemoved before completed.
- Threat T-026: Concurrent paste chooses same destination.
- Threat T-027: Disk full while writing journal.
- Threat T-028: App data readonly.
- Threat T-029: Journal JSON corrupt.
- Threat T-030: Emergency journal disable hides audit trail but must not delete artifacts.
- Threat response: reject wrong webview before side effects.
- Threat response: bounded process runner for stall/flood/tree kill.
- Threat response: canonical path verification and fresh status matching.
- Threat response: RAII close/free/unlock.
- Threat response: journal preserves evidence and classifies interruption without repair.

## 17. Verification evidence path

- Test command 1: `npm run check`.
- Test command 2: `node --test tests/stackPopupGitStatus.test.mjs`.
- Test command 3: `node --test tests/backendBlockingLockBoundariesP6.test.mjs`.
- Test command 4 after WP0 creates the RED contract file: `node --test tests/stackBrowserPhase1Safety.test.mjs`. Before WP0, absence is expected and this command is not a baseline check.
- Test command 5: `cargo test --manifest-path src-tauri/Cargo.toml stack_popup:: -- --nocapture`.
- Test command 6: `cargo test --manifest-path src-tauri/Cargo.toml process_runner -- --nocapture`.
- Test command 7: `cargo check --manifest-path src-tauri/Cargo.toml`.
- Test command 8: `npm run validate` only after focused pass, with caveat for unrelated Node sort failures.
- Known unrelated failure file: `tests/stackPopupState.test.mjs`.
- Known unrelated failing test: `sorts stack entries deterministically while preserving folders first`.
- Known unrelated failing test: `sorts modified desc with folders/files grouped by timestamp and nulls last`.
- Rust unit tests: auth matrix allow/deny.
- Rust unit tests: process runner output cap.
- Rust unit tests: process runner timeout kill.
- Rust unit tests: Git error classifier.
- Rust unit tests: Git path canonicalization.
- Rust unit tests: journal transitions.
- Rust unit tests: journal corruption handling.
- Rust source-contract tests: archive remains spawn_blocking.
- Rust source-contract tests: no wait-timeout crate added.
- Node source-contract tests: all guarded commands inject `WebviewWindow`.
- Node source-contract tests: no object-shaped IPC errors introduced.
- Clarification: source-contract tests complement but do not replace behavioral Rust/native tests and manual Windows smoke.
- Manual Windows smoke 1: wrong webview invoke rejected.
- Manual Windows smoke 2: stack-popup Git status/add/commit still works.
- Manual Windows smoke 3: remote Git auth prompt does not hang beyond timeout.
- Manual Windows smoke 4: archive corrupt/password/nonzero returns bounded error.
- Manual Windows smoke 5: large stdout/stderr child does not freeze app.
- Manual Windows smoke 6: copy paste creates journal and completes.
- Manual Windows smoke 7: cut fallback across volumes records deleteStarted/sourceRemoved.
- Manual Windows smoke 8: clipboard copy/cut works with Explorer paste and DropEffect semantics.
- Manual Windows smoke 9: top-bar pin show/hide still works.
- Manual Windows smoke 10: terminal-panel target isolation still works.
- Claim owner auth: WP1 owner provides test evidence.
- Claim owner process: WP2 owner provides timeout/flood evidence.
- Claim owner Git: WP3/WP4 owners provide focused Git evidence.
- Claim owner archive: WP5 owner provides timeout/nonzero evidence.
- Claim owner clipboard: WP6 owner provides manual Windows evidence.
- Claim owner journal: WP7 owner provides crash-transition fixture evidence.
- Claim owner docs: WP8 owner provides docs/changelog evidence.
- Rule: no runtime safety claim before native Windows smoke.

## 18. Rollout

- Rollout step 1: Land RED contract tests.
- Rollout step 2: Land auth helper/matrix behind no kill switch.
- Rollout step 3: Land runner and Git read/local/remote adapters.
- Rollout step 4: Land Git path canonicalization.
- Rollout step 5: Land archive runner integration.
- Rollout step 6: Land clipboard RAII.
- Rollout step 7: Land journal.
- Rollout step 8: Run focused validation.
- Rollout step 9: Run manual Windows smoke.
- Rollout step 10: Update docs and changelog.
- Rollout step 11: Run final validation with known caveat if still present.

## 19. Rollback

- Rollback principle: revert smallest failing work package.
- Rollback auth: if false rejects break valid callsites, update matrix from inventory; do not disable all auth.
- Rollback runner: if runner has process leak, revert Git/archive integrations first, keep tests documenting risk.
- Rollback Git paths: if canonicalization rejects valid Windows paths, add targeted compatibility fix; do not return to string-prefix validation.
- Rollback clipboard: if RAII regresses clipboard publish, revert within clipboard module only.
- Rollback journal: if journal write blocks paste due app-data readonly, fail safe by reporting journal unavailable unless emergency disable explicitly set; do not delete artifacts.
- Rollback docs: update changelog with rollback facts per policy.

## 20. Migration

- No IPC migration.
- No frontend payload migration.
- No user settings migration required.
- New private journal dir created lazily.
- Existing absent journals treated as no prior recovery records.
- Corrupt journal files preserved and classified without repair.
- Completed records cleaned by retention only.

## 21. Observability and privacy

- Log auth rejection: command, caller label, decision, no args.
- Log process timeout: command class, duration, exit/kill outcome, no full paths unless already public and safe.
- Log Git classification: kind/status/truncated flags, no full pathspec list.
- Log journal maintenance: op id count, states counts, no paths.
- Log corrupt journal: file id/name only, no embedded paths.
- Export behavior: no private journal full paths.
- Full paths allowed only in private local journal.
- Redaction required before adding any docs/test snapshots.

## 22. Performance budgets

- Auth matrix lookup: O(1) or tiny static match.
- Runner memory: <= 128 KiB retained pipe bytes per child.
- Runner thread count: <= 2 pipe drain threads per child plus caller wait.
- Git status normal local repo: target no noticeable regression beyond process timeout/env overhead.
- Journal file paste overhead: <= 100 ms excluding copy.
- Journal dir cleanup: bounded to app journal dir; avoid recursive app-data scan.
- Archive timeout polling interval: small enough for responsive timeout, large enough not to burn CPU; target 50-250 ms.
- Post-kill wait: bounded, target <= 5s unless implementation documents reason.

## 23. Docs update checklist after implementation

- Update `master_spec.md` Stack Browser current behavior with auth guards, runner, Git errors, path canonicalization, clipboard RAII, journal scope.
- Preserve `master_spec.md:49` spawn_blocking fact.
- Update root `stack_browser.md` known limits line 48 stale nonvirtualized claim if changed or clarify current limitation.
- Update `docs/smoke-test-windows.md` with Phase 1 manual smoke.
- Append `changelog.md` under `## Change Ledger` per `CHANGELOG_POLICY.md`.
- Do not append per-request ledger to `master_spec.md`.
- Mention known `npm run validate` caveat if unrelated sort failures persist.

## 24. Definition of Done

- Phase 1a interim release rule: maintainer may explicitly defer WP7 only after DoD-001 through DoD-009, DoD-011 through DoD-017, and WP8a evidence pass. Release notes and changelog MUST state Phase 1b journal remains open; DoD-010 remains unmet.
- Full Phase 1 rule: all DoD items, including DoD-010 and WP8b evidence, are mandatory.
- DoD-001: Approval gate passed.
- DoD-002: WP0 inventory complete.
- DoD-003: Auth matrix implemented and tested.
- DoD-004: Wrong webview invokes rejected.
- DoD-005: Runner enforces timeouts/caps/kill fallback.
- DoD-006: Git prompts disabled and errors classified.
- DoD-007: Git paths canonicalized and deleted-leaf status-matched.
- DoD-008: Archive extraction uses runner and keeps spawn_blocking.
- DoD-009: Clipboard write/read uses RAII and correct publish order.
- DoD-010: Paste copy/cut fallback journal implemented in scope only.
- DoD-011: Privacy redaction verified.
- DoD-012: Focused Node tests pass.
- DoD-013: Rust tests/check pass.
- DoD-014: Manual Windows smoke pass before runtime claims.
- DoD-015: Docs updated per checklist.
- DoD-016: Changelog updated per policy.
- DoD-017: `npm run validate` result documented, including unrelated failures if still present.

## 25. Per-WP handoff template

- Handoff field: WP id.
- Handoff field: owner agent type.
- Handoff field: allowed file scope.
- Handoff field: dependencies satisfied.
- Handoff field: exact source refs read.
- Handoff field: RED tests added and observed failing.
- Handoff field: GREEN implementation summary.
- Handoff field: refactor summary.
- Handoff field: focused validation commands.
- Handoff field: evidence paths/log snippets.
- Handoff field: known risks.
- Handoff field: docs needed.
- Handoff field: conflicts/dirty files touched.
- Handoff field: stop/escalation events.

## 26. Stop and escalation conditions

- Stop if implementation requires new crate for runner; escalate with reason and alternatives.
- Stop if Tauri command injection of `WebviewWindow` breaks macro compatibility; escalate with minimal repro.
- Stop if auth matrix inventory finds active callsite not covered by required matrix; update plan before coding.
- Stop if terminal `stack-popup` legacy compatibility is actually required; document source proof before allowing.
- Stop if Git canonicalization cannot safely support UNC/junction case; escalate with failing test.
- Stop if journal write failure would force source deletion ambiguity; prefer failing paste before mutation.
- Stop if clipboard RAII cannot preserve Explorer interoperability; escalate with native smoke evidence.
- Stop if docs require changing Phase 3 scope; keep Phase 3 deferred.

## 27. Explicit exclusions

- Exclusion: unified operation center.
- Exclusion: user-facing operation list.
- Exclusion: operation progress event stream.
- Exclusion: archive cancel IPC.
- Exclusion: recycle-bin delete journal.
- Exclusion: archive extraction journal.
- Exclusion: automatic recovery repair.
- Exclusion: automatic rollback.
- Exclusion: automatic source deletion during recovery.
- Exclusion: file hashes.
- Exclusion: metadata fidelity claims.
- Exclusion: public object-shaped IPC errors.
- Exclusion: auth kill switch.

## 28. Deferred Phase 3 seams

- Seam: journal op id can later map to user-facing job id, but Phase 1 does not expose it.
- Seam: process runner can later emit progress to operation center, but Phase 1 returns only command result.
- Seam: journal states can later feed recovery UI, but Phase 1 only logs redacted maintenance counts.
- Seam: archive extraction can later add cancel/progress, but Phase 1 only adds timeout/resource safety.
- Seam: Recycle Bin delete can later adopt journal/operation model, but Phase 1 excludes it.
- Seam: docs should mark these as deferred, not partially implemented.

## 29. Plan review checklist

- Review item: Does plan preserve existing IPC `Result<_, String>`?
- Review item: Does plan avoid Phase 3 UI/events/history?
- Review item: Does plan state capabilities JSON exists and is defense-in-depth?
- Review item: Does plan avoid saying archive blocks command thread?
- Review item: Does plan preserve `spawn_blocking` for paste/delete/archive work?
- Review item: Does plan correctly state no wait-timeout crate exists?
- Review item: Does plan correctly state move fallback copies then deletes after successful copy?
- Review item: Does plan precisely identify clipboard write close-skip risk?
- Review item: Does plan include fixed timeout/cap values?
- Review item: Does plan include Git nonzero optional-probe nuance?
- Review item: Does plan include exact caller auth matrix and inventory step?
- Review item: Does plan include stale docs fix targets?
- Review item: Does plan include known unrelated Node sort failures exactly?

## 30. Implementation agent quick-start

- Step 1: Read `master_spec.md`, `CHANGELOG_POLICY.md`, and this plan.
- Step 2: Run WP0 inventory before editing.
- Step 3: Add RED tests for your WP only.
- Step 4: Confirm RED failure.
- Step 5: Implement only within allowed file scope.
- Step 6: Run focused validation.
- Step 7: Record evidence and handoff using template.
- Step 8: Do not update docs until implementation behavior is final.
- Step 9: Do not claim runtime safety until manual Windows smoke passes.
- Step 10: Preserve unrelated dirty worktree changes.
