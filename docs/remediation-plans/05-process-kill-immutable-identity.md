# 05 Process Kill Immutable Identity

## Metadata

- Status: Implementation-ready plan; destructive-operation safety change.
- Order: 05 of 13.
- Owner: Rust process-safety implementer.
- Dependencies: Plans 01-02 MUST be complete.
- Related audit finding: P0-4 Process kill can target a reused PID.

## Objective and evidence

Objective: bind Process Manager kill confirmation and final termination to immutable process identity captured from the target process handle, including creation time and canonical image path where available, so PID reuse cannot terminate a replacement process.

Evidence:

- Audit lines 159-172: `kill_process` validates confirmation metadata, then opens only `PROCESS_TERMINATE` and kills by PID; PID reuse can kill wrong process.
- Source `src-tauri/src/process_manager.rs:181-193`: lists processes, validates guardrail, then calls `windows_impl::kill_process(pid)`.
- Source `process_manager.rs:300-341`: confirmation compares plan fields but not creation time/image identity.
- Source `process_manager.rs:589-597`: `OpenProcess(PROCESS_TERMINATE)` then `TerminateProcess` with no query/revalidation.
- `master_spec.md` says process manager has guarded kill action but does not guarantee immutable identity like task-window close.

## Scope

Exact files/symbols likely in scope:

- `src-tauri/src/process_manager.rs`
- `ProcessInfo`
- `ProcessKillConfirmation`
- `ProcessKillPlan`
- `build_kill_guardrail_plan`
- `validate_kill_guardrail_execution`
- `windows_impl::process_info_from_entry`
- `windows_impl::kill_process`
- Existing helpers: `process_cpu_snapshot`, `process_image_path`, `GetProcessTimes`, `QueryFullProcessImageNameW`
- Frontend Process Manager confirmation flow in `src/components/ProcessManagerSurface.svelte` if payload must include identity fields.
- Tests: `tests/processManagerState.test.mjs`, `tests/processManagerWiring.test.mjs`, Rust tests in `process_manager.rs`.

Explicit out of scope:

- Tree termination execution; current tree kill remains plan-only unless separate approved work.
- Killing protected/system processes that require stronger privileges.
- Enabling `SeDebugPrivilege`, using `taskkill.exe`, or elevating JasonShell.
- Changing Process Manager UI unrelated to confirmation identity fields.
- Persistent top JSON/shell terminal and Stack Browser Git workbench feature work.

## Current-state contract

- Process Manager lists processes, builds guarded kill plans, requires second confirmation, refuses self/protected PIDs, and executes only single-process kill.
- Tree termination is plan-only and non-executable.
- Current defect: confirmation is bound to PID/list metadata but final handle is opened later by PID only and may refer to a reused PID.

## Requirements

### Functional

1. `ProcessInfo` or kill plan MUST expose immutable identity data sufficient for confirmation: PID plus process creation time and canonical image path when queryable.
2. `ProcessKillConfirmation` MUST include the identity captured in the displayed plan.
3. `validate_kill_guardrail_execution` MUST reject stale/missing identity for killable processes when identity was available at plan time.
4. Windows kill implementation MUST open one handle with `PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION` and revalidate identity on that same handle immediately before `TerminateProcess`.
5. If creation time or image path cannot be queried, implementation MUST follow a conservative policy: recommended default is refuse termination unless target is explicitly classified as identity-limited and confirmation includes that limitation. Stop/escalate before allowing PID-only kill.
6. PID reuse or image/creation-time mismatch MUST abort without calling `TerminateProcess`.
7. Error messages MUST clearly report stale identity vs access denied vs unsupported protected process.
8. Tree termination MUST remain plan-only and must not gain execution path.

### Nonfunctional

9. Tests MUST use fakes/injected handles for PID-reuse race; no real destructive process kill in unit tests.
10. No elevated privilege, process tree kill, or external `taskkill.exe` may be introduced.
11. Added fields MUST be backward-compatible for frontend where possible; stale old confirmations should fail closed.
12. Identity comparison MUST normalize image paths consistently and avoid false matches by basename only.

## Decisions and implementation approach

- Recommended identity type: `ProcessIdentity { pid: u32, creation_time_100ns: Option<u64>, image_path: Option<String> }` with normalized/canonical image path comparison when available.
- Better if `creation_time_100ns` is required for executable kill; image path is secondary but should be compared when both sides have it.
- Use `GetProcessTimes` on list/enumeration handle and kill handle.
- Use `QueryFullProcessImageNameW(PROCESS_NAME_WIN32)` for image path and existing normalization helpers.
- Stop/escalate if Windows denies query rights for common killable user processes; do not silently fall back to PID-only.

## Phases with RED-first tests

1. RED Rust test: stale confirmation with changed creation time is rejected before kill. Expected fail now.
2. RED Rust test: fake kill handle identity mismatch does not call terminate. Expected fail now.
3. RED Node/source test: frontend confirmation payload includes immutable identity fields. Expected fail now if UI lacks fields.
4. Add identity structs/fields to `ProcessInfo`, `ProcessKillPlan`, `ProcessKillConfirmation`.
5. Populate identity during process listing using query handle.
6. Update plan/confirmation validation to compare identity exactly/conservatively.
7. Refactor `windows_impl::kill_process` to accept expected identity and revalidate on opened handle before termination.
8. Update frontend payload construction and tests.
9. Run focused/full validation.

## Exact tests and assertions

- Add Rust test in `process_manager.rs`: `kill_guardrail_rejects_confirmation_with_stale_creation_time`.
- Add Rust test: `kill_guardrail_rejects_confirmation_with_stale_image_path`.
- Add Rust test: `windows_kill_revalidates_identity_before_terminate` using injectable opener/terminator or non-Windows fake module.
- Add Rust test: `identity_limited_process_kill_fails_closed_without_explicit_policy`.
- Existing `process_manager` guardrail tests must still assert self/protected/tree-plan refusal.
- Add/extend Node `tests/processManagerState.test.mjs`: `killConfirmationIncludesImmutableIdentity` asserts confirmation payload carries identity from selected row/plan.
- Add/extend Node `tests/processManagerWiring.test.mjs`: assert UI sends identity fields to `kill_process`.

## Edge and failure cases

- Target exits before kill handle open -> return no-longer-visible/stale error, no terminate.
- PID reused with same image path but different creation time -> reject.
- PID reused with same creation time impossible in practice but path differs -> reject if both paths available.
- Query image path denied but creation time available -> compare creation time and include warning only if product accepts.
- Query creation time denied -> fail closed by default and ask for explicit policy if this blocks common use.
- Access denied on terminate after identity matches -> return access denied; do not elevate.

## Data/API/event compatibility impacts

- `ProcessInfo`, `ProcessKillPlan`, and `ProcessKillConfirmation` payloads gain identity fields. This is an internal Tauri API but frontend and tests must update together.
- Old renderer confirmations lacking identity must fail closed.
- No event-name changes expected.

## Validation commands

Focused:

```bash
cargo test --manifest-path src-tauri/Cargo.toml process_manager -- --nocapture
node scripts/clean-dist-tests.mjs && tsc -p tsconfig.test.json && node --test tests/processManagerState.test.mjs tests/processManagerWiring.test.mjs
```

Full:

```bash
npm run cargo:test
npm run cargo:check
npm run test:node
npm run validate
```

Manual destructive smoke only with explicit user-created sacrificial process and consent.

## Acceptance criteria

- Given plan identity from process A, when PID is reused by process B before kill, then kill aborts and `TerminateProcess` is not called. Covers req 1-6.
- Given renderer sends old confirmation without identity, when kill requested, then backend fails closed. Covers req 2-3, 11.
- Given identity matches and terminate rights exist, when kill requested, then backend terminates exactly that handle. Covers req 4.
- Given tree-plan confirmation, when kill requested, then backend still refuses execution. Covers req 8.
- Given access denied, when identity matches, then error reports access denied without elevation/taskkill. Covers req 7, 10.

## Risks and rollback

- Risk: fail-closed policy blocks some legitimate kills. Rollback: add explicit identity-limited UX/policy after product decision, not PID-only default.
- Risk: path normalization mismatch causes false stale errors. Rollback: compare creation time as primary and log path mismatch diagnostics; do not kill on mismatch until reviewed.
- Risk: frontend/backend payload skew. Rollback: version/optional fields fail closed and update UI tests.

## Master spec, changelog, docs updates

- Update `master_spec.md` Process Manager section: kill confirmation and final termination are bound to immutable PID + creation time + image identity on same handle; old/stale confirmations fail closed; no elevation/tree kill.
- Append changelog entries for behavior/tests.
- If conservative identity-limited policy remains, document known limitation.

## Handoff checklist

- [ ] Read docs/source/tests.
- [ ] RED tests added for stale creation time, stale image path, final handle revalidation, frontend payload.
- [ ] Identity fields added and populated.
- [ ] Backend kill revalidates same handle before terminate.
- [ ] Old/stale confirmations fail closed.
- [ ] Focused/full validation run.
- [ ] Durable docs updated.
- [ ] Adversarial QA for PID reuse, access denied, protected process, payload skew.

## Copy/Paste Implementation Prompt

```text
Implement remediation plan docs/remediation-plans/05-process-kill-immutable-identity.md. First read master_spec.md, docs/current-state-technical-audit-2026-08-28.md, CHANGELOG_POLICY.md, package.json, and this plan. Preserve unrelated worktree changes. Use RED-first: add failing tests proving PID reuse/identity mismatch cannot call TerminateProcess and frontend confirmation includes immutable identity before code. Refactor Process Manager kill so confirmation includes PID + creation time + normalized image identity where available, and Windows termination opens one PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION handle, revalidates identity on that same handle immediately before TerminateProcess, and fails closed on mismatch or missing required identity. Do not add tree kill execution, taskkill.exe, SeDebugPrivilege, elevation, or unrelated UI changes. Do not implement persistent top JSON/shell terminal or Stack Browser Git workbench feature work. Run focused process_manager Rust and Process Manager Node tests, then npm run cargo:test/cargo:check/test:node/validate as feasible. Update master_spec.md Process Manager contract and changelog.md per policy. Perform adversarial QA focused on PID reuse, stale renderer payloads, access denied, protected processes, and identity-limited policy. Report files changed, commands run, pass/fail evidence, and unresolved blockers.
```
