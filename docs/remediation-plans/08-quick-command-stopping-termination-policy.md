# 08 Quick Command Stopping State and Safer Termination Policy

## Metadata
- Status: Ready for implementation.
- Order: 08 of 13.
- Audit finding: P1-2.
- Owner: implementation agent.
- Dependencies: Plans 01-02 and 06. Plan 06 lands first because both may touch Command Panel.
- Exclusions: no persistent top JSON/shell terminal work; no Stack Browser Git workbench feature work.

## Objective and Evidence
Make Quick Command stop nonblocking, visibly consistent, and safer by default. Recommend safe root-only async termination unless an existing owned Windows Job Object already makes identity-bounded tree semantics possible. Require escalation before any new persistent schema/API or default tree-kill contract.

Evidence:
- Audit P1-2 cites `src-tauri/src/quick_commands.rs:980-1041`.
- Current code validates root process creation time, removes live state, pushes stopped transcript, then synchronously runs `C:\Windows\System32\taskkill.exe /PID <pid> /T /F`; on failure reinserts state.
- Impact: blocking command path, transient disappearing run, descendant tree kill without child identity binding.

## Scope
In scope exact files/symbols:
- `src-tauri/src/quick_commands.rs`: `stop_running_quick_command`, run state model, transcript/status update emission, termination helper.
- `src/components/CommandPanelSurface.svelte`: only if needed to render stopping state already exposed by history/live event.
- `src/lib/quickCommands.ts`: only if existing types need optional stopping status; avoid new persistent API unless escalated.
- Rust tests in `quick_commands.rs` for stop state/termination helpers.
- Node tests in `tests/quickCommands.test.mjs` and `tests/commandPanelWiring.test.mjs` for source/UX contracts.
- Docs: `master_spec.md`, `changelog.md`.

Out of scope:
- New saved-command schema fields.
- New user-facing stop mode selector.
- Persistent top JSON/shell terminal features.
- Stack Browser Git workbench features.
- Default tree-kill behavior unless explicitly escalated/approved or proven owned Job Object semantics already exist.

## Current Contract
- Quick Commands can run trusted local commands and emit transcript/history events.
- Stop verifies root PID creation time before terminating.
- History entry uses `running: false`, `exit_code: None`, stopped transcript entry.
- Frontend disables stop button with `stoppingId` while stop request in flight.

## Requirements
### Functional Requirements
- FR-1: Stop command MUST move run to a visible `stopping` state before termination and keep it visible until final result.
- FR-2: Termination work MUST run off the Tauri command path via async blocking boundary.
- FR-3: Default termination SHOULD be root-process-only, identity-checked immediately before kill, unless existing owned Job Object support can provide safe tree termination.
- FR-4: Implementation MUST NOT introduce new persistent schema, public API, or default tree-kill contract without escalation and explicit approval.
- FR-5: If termination fails, run MUST return to running/error-visible state without losing buffered stdout/stderr/transcript.
- FR-6: If termination succeeds, history MUST contain one stopped record with bounded sanitized output and stopped transcript entry.
- FR-7: Root PID reuse MUST abort termination.

### Non-Functional Requirements
- NFR-1: Slow termination MUST not block unrelated lightweight Tauri commands.
- NFR-2: State transitions MUST be observable and deterministic: running -> stopping -> stopped or running/error.
- NFR-3: No shelling out to `taskkill.exe` on default path unless explicitly justified as fallback after escalation.
- NFR-4: No descendant process termination unless identity/ownership proof exists.

## Implementation Decisions
- Recommended policy: replace `taskkill /T /F` with Windows root-only `OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION)`, verify creation time on same handle, call `TerminateProcess`, wait briefly if needed. Non-Windows can keep no-op/test-safe behavior if existing pattern requires.
- If repo already owns a Windows Job Object for Quick Command children, tree stop may use closing/terminating that owned job because membership is owned by JasonShell. If no such ownership exists, do not invent new tree kill in this plan.
- Represent stopping either as in-memory flag on live run or transcript pending entry; avoid persisted schema unless approved.
- Emit update when stopping starts so UI remains stable.

## Phased RED-First Implementation
1. RED state tests:
   - Add Rust tests for running -> stopping without removal from registry.
   - Add failure path test preserving run state/output after termination failure.
2. RED termination policy tests:
   - Source test rejects `taskkill.exe`, `/T`, `/F` in default stop path.
   - Rust unit test with injectable termination helper rejects PID creation-time mismatch.
3. RED async boundary tests:
   - Node/source test requires `stop_running_quick_command` or command wrapper to use `spawn_blocking` and async command signature as needed.
4. GREEN implementation:
   - Add internal stopping flag/state.
   - Split stop planning/marking under mutex from blocking termination.
   - Run termination outside mutex and command path.
   - Finalize stopped history/event after success; roll back stopping on failure.
5. Docs:
   - Update `master_spec.md` Quick Commands stop semantics.
   - Append changelog.

## Exact Tests and Assertions
- `src-tauri/src/quick_commands.rs`
  - `stop_marks_run_stopping_before_termination`.
  - `stop_failure_restores_running_state_and_preserves_transcript`.
  - `stop_rejects_reused_root_pid_before_terminate`.
  - `stop_success_appends_single_stopped_history_entry`.
- `tests/quickCommands.test.mjs`
  - `Quick Command stop does not shell out to taskkill tree kill by default`.
  - `Quick Command stop uses async blocking boundary`.
  - `Quick Command live update exposes stopping state or pending stopped transition`.
- `tests/commandPanelWiring.test.mjs`
  - stop button disabled/label reflects stopping without removing run row, if UI type changes.

## Edge Cases
- Process exits naturally while stop is starting.
- PID reused between initial check and final termination handle open.
- Termination helper returns access denied.
- Blocking helper panics or join fails.
- Stop requested twice for same run.
- Output arrives while stopping.
- Settings history update fails after termination succeeds.

## API, Type, Event Compatibility
- Prefer no new IPC command names.
- Existing stop command name and args remain stable.
- Event payload may add optional `stopping?: boolean` only if nonbreaking and needed; do not require consumers to send it.
- No persistent settings/history schema change unless escalated.
- Existing `running: false` stopped history remains.

## Validation
Focused:
- `cargo test --manifest-path src-tauri/Cargo.toml quick_commands`
- `npm run test:node`
- `npm run check`

Full:
- `npm run validate`

Manual:
- Run long command, stop it, verify UI shows stopping until final state and app remains responsive.
- Try already-exited run; verify clear error and no disappeared state.

## Acceptance Criteria
- AC-1 (FR-1,NFR-2): Given running command, when Stop clicked, then run remains visible as stopping until termination succeeds/fails.
- AC-2 (FR-2,NFR-1): Given slow termination helper, when Stop invoked, then async command path remains nonblocking by design/source/test seam.
- AC-3 (FR-3,FR-7): Given root PID creation time mismatch, when stop finalizes, then termination is aborted.
- AC-4 (FR-5): Given termination failure, when helper returns error, then run state/output/transcript remain available and UI can retry.
- AC-5 (FR-4): Given implementation needs new persistent schema/API or default tree kill, when discovered, then work stops and escalates.

## Risks and Rollback
- Risk: root-only stop may leave child processes. This is safer default; document and require explicit future tree design.
- Risk: UI expects removal during stop. Mitigate with tests and optional state mapping.
- Risk: Windows termination handle rights vary. Return explicit error and preserve run.
- Rollback: revert stop helper/state changes; no schema migration if plan followed.

## Master Spec and Changelog Updates
- `master_spec.md`: update Quick Commands stop contract to async visible stopping and root-only safe termination, or document approved Job Object tree semantics if present.
- `changelog.md`: append `[CODE]` with validation and any accepted root-only limitation.

## Handoff Checklist
- [ ] Read `master_spec.md`, audit, plan 08.
- [ ] Preserve dirty worktree.
- [ ] RED tests first.
- [ ] Escalate before new schema/API/default tree kill.
- [ ] Validate focused + full where feasible.
- [ ] Update docs.
- [ ] Adversarial QA.
- [ ] No excluded terminal/Git workbench work.

## Copy/Paste Implementation Prompt

```text
Implement remediation plan 08 in C:\dev\jasonshell. First read master_spec.md, docs/current-state-technical-audit-2026-08-28.md, CHANGELOG_POLICY.md, package.json scripts, and docs/remediation-plans/08-quick-command-stopping-termination-policy.md. Preserve dirty worktree. Use TDD: add RED Rust/Node tests for visible stopping state, async blocking boundary, root PID reuse rejection, failure rollback, and no default taskkill /T /F before code changes. Scope to quick_commands.rs, minimal CommandPanelSurface/quickCommands TS types only if needed, focused tests, master_spec.md, changelog.md. Do not implement persistent top JSON/shell terminal or Stack Browser Git workbench feature work. Recommend and implement safe root-only async termination by default unless existing owned Windows Job Object semantics already exist and are testable. Escalate before adding any new persistent schema/API or making tree kill a default contract. Keep existing command names compatible. Validate with cargo quick_commands tests, npm run test:node, npm run check, and npm run validate if feasible. Update docs per policy. Run adversarial QA before final response.
```
