# 07 Open With Validation Parity and Stack Open With Async Boundary

## Metadata
- Status: Ready for implementation.
- Order: 07 of 13.
- Audit findings: P1-1 and P1-8.
- Owner: implementation agent.
- Dependencies: Plans 01-02 MUST be complete; coordinate with active Stack file-operation work.
- Exclusions: no Stack Browser Git workbench feature work; no persistent top JSON/shell terminal work.

## Objective and Evidence
Make Open With validation consistent with normal shell-path classification and move Stack Open With slow filesystem/application work off Tauri command path.

Evidence:
- Audit P1-1: `src-tauri/src/shell_paths.rs:196-203,321-350` only trims/nonempty before `ShellExecuteW(openas)`.
- Audit P1-8: `src-tauri/src/stack_popup.rs:682-720` and `src-tauri/src/stack_popup/open_with.rs:56-75,131-149` do existence/canonicalization/resolver/spawn on sync command path.
- Normal `open_shell_path` rejects protocols, missing targets, executable/script extensions.

## Scope
In scope exact files/symbols:
- `src-tauri/src/shell_paths.rs`: `open_shell_path_with_picker`, `classify_shell_open_target`, possible new `classify_open_with_picker_target`.
- `src-tauri/src/stack_popup.rs`: `open_stack_item_with_picker`, `list_stack_open_with_candidates`, `open_stack_item_with_app` command async signatures/bodies.
- `src-tauri/src/stack_popup/open_with.rs`: `open_with_candidates_for_path`, `open_with_app`, resolver helpers; add async wrappers using `tauri::async_runtime::spawn_blocking` if suitable.
- `src-tauri/src/contracts.rs` and capability files only if async command signature requires no name change; command names must remain stable.
- `src/lib/stackPopup.ts` only if TS wrapper awaits same command names and generated type expectations need no behavior change.
- Rust tests in `shell_paths.rs` and `stack_popup::open_with` tests; Node source contract tests such as `tests/stackBrowserContextActions.test.mjs` or new `tests/stackOpenWithAsyncBoundary.test.mjs`.

Out of scope:
- Adding arbitrary executable picker or custom app persistence.
- Changing Open With candidate list semantics beyond validation parity.
- Git workbench features.

## Current Contract
- Stack Open With commands are authorized for `stack-popup` caller.
- `open_stack_item_with_picker` only accepts existing normalized file path and uses Windows native Open With picker.
- `open_stack_item_with_app` launches allowlisted candidate selected by app id.
- Command names and frontend invoke names are stable.

## Requirements
### Functional Requirements
- FR-1: Open With picker MUST reject protocol/file URI inputs, nonexistent paths, directories, and executable/script extensions using same effective policy as normal shell open unless explicitly narrower for files.
- FR-2: Stack Open With commands MUST preserve caller authorization before side effects.
- FR-3: Stack Open With candidate resolution and app launch MUST run behind `tauri::async_runtime::spawn_blocking` or equivalent blocking boundary.
- FR-4: Command names, capability identifiers, TS wrapper function names, response shapes, and user-visible error text SHOULD remain compatible except for newly rejected unsafe picker inputs.
- FR-5: App-id allowlist MUST remain authoritative; renderer MUST NOT provide executable paths for launch.
- FR-6: Directory targets MUST continue returning `Open with is only available for files`.

### Non-Functional Requirements
- NFR-1: Slow disk, antivirus, network path, or process spawn MUST NOT block async command worker threads.
- NFR-2: Validation must be deterministic and covered by unit tests.
- NFR-3: No new persistent schema, settings, or capability broadening.

## Implementation Decisions
- Add a picker-specific classifier that reuses `classify_shell_open_target` local path rules but disallows `ms-settings:` and requires file, not directory.
- Convert Stack Open With Tauri commands to `async fn`; perform lightweight caller auth first on command path, then move path normalization plus resolver/launch to `spawn_blocking` unless auth needs window only.
- Keep `normalize_existing_path` inside blocking section if canonicalization may touch slow filesystem; but ensure no side effect before auth.
- For `open_stack_item_with_app`, move candidate resolution and `Command::spawn` together into blocking closure.

## Phased RED-First Implementation
1. RED Rust validation tests:
   - In `shell_paths.rs` tests, add picker rejects URL/protocol, `file:`, missing file, directory, `.exe`, `.cmd`, `.ps1`, `.lnk`, `.url`; accepts temp `.txt`.
2. RED async-boundary tests:
   - Add source-contract test verifying `open_stack_item_with_picker`, `list_stack_open_with_candidates`, `open_stack_item_with_app` are `async fn` and use `tauri::async_runtime::spawn_blocking` after authorization.
   - Add assertion command constants unchanged.
3. GREEN validation:
   - Implement classifier and route picker through it.
4. GREEN async:
   - Convert three Stack commands to async; wrap blocking work; preserve errors.
5. Refactor:
   - Extract common file validation helper only if it reduces duplication without widening policy.
6. Docs:
   - Update `master_spec.md` Stack Browser/Open With behavior; append changelog.

## Exact Tests and Assertions
- `src-tauri/src/shell_paths.rs`
  - `open_with_picker_rejects_protocol_targets`.
  - `open_with_picker_rejects_missing_targets`.
  - `open_with_picker_rejects_executable_or_script_targets`.
  - `open_with_picker_accepts_existing_document_file`.
- `src-tauri/src/stack_popup/open_with.rs`
  - `open_with_app_uses_candidate_allowlist` existing/added; renderer app id only.
- `tests/stackOpenWithAsyncBoundary.test.mjs`
  - source order: `authorize_stack_command` appears before `spawn_blocking`.
  - three Stack Open With commands are async and contain/route to `spawn_blocking`.
  - no new command names added for same behavior.

## Edge Cases
- UNC/network path with slow canonicalize.
- Existing directory passed to picker/app.
- Windows drive path `C:\file.txt` not misclassified as protocol.
- `ms-settings:` allowed for normal shell open but rejected for picker.
- Candidate executable disappears between list and launch: return existing unavailable/launch error.
- Spawn returns before child app initializes; command should not wait for app exit.

## API, Type, Event Compatibility
- IPC command names remain `open_stack_item_with_picker`, `list_stack_open_with_candidates`, `open_stack_item_with_app`.
- Rust function signatures may become async but Tauri invoke contract remains Promise-based.
- `StackOpenWithCandidate` unchanged.
- No event changes.
- No settings/schema migration.

## Validation
Focused:
- `npm run cargo:test -- shell_paths` is not a package script; use `cargo test --manifest-path src-tauri/Cargo.toml shell_paths` if local focused run desired.
- `cargo test --manifest-path src-tauri/Cargo.toml stack_popup::open_with`
- `npm run test:node`
- `npm run check`

Full:
- `npm run validate`

Manual:
- Open With picker on `.txt`; reject `.exe`; list candidates and open with Notepad/VS Code where installed.

## Acceptance Criteria
- AC-1 (FR-1): Given URL/protocol/missing/executable input, when Open With picker is invoked, then command rejects before `ShellExecuteW`.
- AC-2 (FR-2): Given unauthorized caller, when Stack Open With command is invoked, then auth rejects before filesystem or launch work.
- AC-3 (FR-3,NFR-1): Given slow resolver/launcher test seam or source contract, when command runs, then blocking work occurs in `spawn_blocking`.
- AC-4 (FR-5): Given renderer-supplied `app_id`, when app launch runs, then executable comes only from backend candidate allowlist.

## Risks and Rollback
- Risk: rejecting `.lnk` in picker changes previous permissive behavior. This is intended parity; document in spec.
- Risk: async conversion may affect Tauri macro imports or generated command handler. Mitigate with cargo check/test.
- Rollback: revert to sync commands and old picker classifier; no data migration.

## Master Spec and Changelog Updates
- `master_spec.md`: update Stack Browser Open With to mention picker validation parity and async blocking boundary.
- `changelog.md`: append concise `[CODE]` with validation.

## Handoff Checklist
- [ ] Read `master_spec.md`, audit, plan 07.
- [ ] Preserve dirty worktree.
- [ ] RED tests first.
- [ ] Keep command names stable.
- [ ] No arbitrary app persistence.
- [ ] Validate Rust + Node + check.
- [ ] Update docs.
- [ ] Adversarial QA.

## Copy/Paste Implementation Prompt

```text
Implement remediation plan 07 in C:\dev\jasonshell. First read master_spec.md, docs/current-state-technical-audit-2026-08-28.md, CHANGELOG_POLICY.md, package.json scripts, and docs/remediation-plans/07-open-with-validation-async-boundary.md. Preserve dirty worktree. Use TDD: add RED Rust tests for Open With picker validation parity and RED Node/source tests for Stack Open With async blocking boundary before implementation. Scope to shell_paths.rs, stack_popup.rs Open With commands, stack_popup/open_with.rs, focused tests, master_spec.md, changelog.md, and only minimal TS wrapper/capability updates if required without renaming commands. Do not implement persistent top JSON/shell terminal or Stack Browser Git workbench feature work. Do not add arbitrary executable persistence or broaden capabilities. Keep existing IPC command names, payloads, and StackOpenWithCandidate shape. Require auth before side effects, reject unsafe picker targets, move candidate resolution/canonicalization/launch off command path via spawn_blocking. Validate with cargo focused tests, npm run test:node, npm run check, and npm run validate if feasible. Update docs per policy. Run adversarial QA before final response.
```
