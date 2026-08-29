# 02 Rust Validation Baseline and Stale Validation Status

## Metadata

- Status: Ready for implementation planning; requires fresh RED reproduction.
- Order: 02 of 13.
- Owner: Rust validation remediation implementer.
- Dependencies: Plan 01. Reconcile shared validation docs after both baseline gates are current.
- Related audit finding: P0-3 Rust gate failures and stale canonical validation status.

## Objective and evidence

Objective: make `npm run cargo:test` green for remaining P0-3 Rust failures and correct stale validation status in durable docs after Node/Rust evidence is current.

Evidence:

- Audit lines 144-157: Rust gate red blocks release.
- Audit lines 342-347 lists three Rust failures.
- `package.json`: `npm run cargo:test` runs `cargo test --manifest-path src-tauri/Cargo.toml`; `npm run validate` requires Rust tests and `cargo:check`.
- `master_spec.md` line 30 stale validation text no longer matches audit.

## Scope

In-scope failing Rust tests:

1. `contracts::tests::new_command_contracts_are_unique_and_stable`: missing `hide_quick_launch_panel_on_focus_loss` from expected command set.
2. `search::providers::apps::tests::stale_app_cache_returns_existing_rows_while_refresh_is_deferred`: `snapshot.refresh_needed` unexpectedly false.
3. `shell_paths::tests::vscode_resolver_uses_standard_candidate_order`: got `None`, expected `Some("C:\\Tools\\code.cmd")`.

Likely files/symbols:

- `src-tauri/src/contracts.rs`, command registry constants/tests.
- Quick Launch command registration/capability files if contract truly missing.
- `src-tauri/src/search/providers/apps.rs`, app cache snapshot/refresh-needed logic and tests.
- `src-tauri/src/shell_paths.rs`, VS Code candidate resolver and test fakes.
- `src-tauri/src/main.rs` or command registration modules if contract drift is real.
- `master_spec.md`, `changelog.md` after status/contract change.

Explicit out of scope:

- New Search provider redesign beyond stale-cache test contract.
- New VS Code discovery sources unless current contract demands them.
- Terminal/Git behavior.
- Live Tauri shell launch without user consent.

## Current-state contract

- `contracts::events::ALL` is authoritative for event names; command contract tests guard stable command exposure.
- Search app cache should serve existing rows when refresh is deferred/stale rather than blanking UI.
- VS Code resolver should use standard candidate order and deterministic fakes in tests.
- `master_spec.md` should describe current validation state and known red gates without stale earlier-only failures.

## Requirements

### Functional

1. Implementation MUST reproduce `npm run cargo:test` RED before edits.
2. Each Rust failure MUST be classified as code defect, stale test, stale spec, or environment-sensitive test bug.
3. Command contracts MUST include registered public Tauri commands exactly once and preserve stability tests.
4. If `hide_quick_launch_panel_on_focus_loss` is a live command, contract expectations and capability docs MUST include it; if obsolete, registration or test MUST be corrected consistently.
5. Search stale app cache behavior MUST return cached rows while marking refresh-needed when refresh is deferred by policy.
6. Search stale-cache tests MUST assert deterministic cache/refresh policy without wall-clock flake.
7. VS Code candidate resolution MUST be deterministic under test fakes and preserve documented candidate order.
8. Durable validation status MUST be updated after fresh Node/Rust evidence, not guessed.

### Nonfunctional

9. Rust fixes MUST avoid broad sleeps/time dependence in unit tests.
10. Command registry changes MUST not widen renderer authority without capability review.
11. Validation docs MUST avoid per-request logs in `master_spec.md`.

## Decisions and implementation approach

- Start with focused Rust test names using `cargo test --manifest-path src-tauri/Cargo.toml <test_name> -- --nocapture`.
- Fix command-contract drift first because it may be pure registry/test mismatch.
- Fix app-cache test with controllable time or explicit stale fixture; recommended default: preserve cached rows and `refresh_needed=true` whenever cache age exceeds stale threshold but refresh is not executed in current call.
- Fix VS Code resolver by aligning fake filesystem/env candidate search with production order; stop if real machine-specific install paths are needed.

## Phases with RED-first tests

1. Reproduce full RED: `npm run cargo:test`.
2. Reproduce focused RED for each failing Rust test.
3. Inspect symbol under test and `master_spec.md` contract.
4. Patch smallest authoritative side.
5. Rerun focused test immediately.
6. Rerun `npm run cargo:test` and `npm run cargo:check`.
7. If validation status changes, update `master_spec.md` and append `changelog.md` entry.
8. QA: adversarial review for capability widening, time flake, machine-local assumptions.

## Exact tests and assertions

- Existing `contracts::tests::new_command_contracts_are_unique_and_stable`: asserts command registry expected set matches source registration and has no duplicate/drift.
- Existing `search::providers::apps::tests::stale_app_cache_returns_existing_rows_while_refresh_is_deferred`: asserts cached app rows survive deferred refresh and snapshot reports refresh needed.
- Existing `shell_paths::tests::vscode_resolver_uses_standard_candidate_order`: asserts deterministic `code.cmd` candidate chosen from standard order.
- Add if needed: `contracts::tests::quick_launch_focus_loss_command_is_registered_and_capability_scoped`.
- Add if needed: `search::providers::apps::tests::fresh_app_cache_does_not_request_refresh`.
- Add if needed: `shell_paths::tests::vscode_resolver_ignores_missing_candidates_before_later_match`.

## Edge and failure cases

- Command listed in capabilities but missing Rust handler -> fix registration, not test only.
- Command registered but intentionally private/internal -> remove public expectation and document rationale.
- Stale cache has zero rows -> ensure refresh-needed still true but no fake rows fabricated.
- VS Code candidate exists as `.exe` vs `.cmd` -> test should reflect documented order, not local machine.

## Data/API/event compatibility impacts

- Command registry/capability changes are API compatibility-sensitive.
- Search snapshot `refresh_needed` is observable diagnostic/state contract.
- VS Code resolver affects native shell path handling but should not change public payloads.

## Validation commands

Focused:

```bash
cargo test --manifest-path src-tauri/Cargo.toml contracts::tests::new_command_contracts_are_unique_and_stable -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml search::providers::apps::tests::stale_app_cache_returns_existing_rows_while_refresh_is_deferred -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml shell_paths::tests::vscode_resolver_uses_standard_candidate_order -- --nocapture
```

Full:

```bash
npm run cargo:test
npm run cargo:check
npm run validate
```

## Acceptance criteria

- Given current repo, when `npm run cargo:test` runs, then all Rust tests pass. Covers req 1.
- Given command contract, when registry test runs, then command set is stable and scoped. Covers req 3-4, 10.
- Given stale app cache with deferred refresh, when search apps snapshot is requested, then cached rows return and `refresh_needed` is true. Covers req 5-6.
- Given fake VS Code candidates, when resolver runs, then first standard existing candidate is selected. Covers req 7.
- Given fresh validation evidence, when docs are inspected, then stale validation line is corrected. Covers req 8, 11.

## Risks and rollback

- Risk: command contract fix exposes new command to wrong window. Rollback: revert registry/capability and fix caller routing.
- Risk: stale-cache logic causes repeated refresh churn. Rollback: restore previous policy and add explicit throttle state.
- Risk: VS Code resolver becomes machine-specific. Rollback: isolate fake resolver from environment.

## Master spec, changelog, docs updates

- Update validation status in `master_spec.md` only after actual current Node/Rust evidence.
- If command set changes, update backend command map/capability section.
- Append changelog entries for source/test/doc changes per policy.

## Handoff checklist

- [ ] Read required docs and cited Rust source/tests.
- [ ] Reproduced full and focused RED.
- [ ] Classified each failure.
- [ ] Fixed authoritative side.
- [ ] Ran focused tests, `npm run cargo:test`, `npm run cargo:check`.
- [ ] Coordinated validation-status docs with Plan 01.
- [ ] Adversarial QA complete.
- [ ] Reported evidence.

## Copy/Paste Implementation Prompt

```text
Implement remediation plan docs/remediation-plans/02-rust-validation-baseline-stale-validation-status.md. First read master_spec.md, docs/current-state-technical-audit-2026-08-28.md, CHANGELOG_POLICY.md, package.json, and this plan. Preserve unrelated worktree changes. Use RED-first: run npm run cargo:test and focused failing Rust tests before edits. Classify each failure, then fix the authoritative side without widening capabilities or adding machine-local assumptions. Do not implement terminal or Stack Git behavior. Run focused cargo tests, npm run cargo:test, npm run cargo:check, and npm run validate if feasible. Update master_spec.md only for durable current validation/command/search behavior and changelog.md per policy. Perform adversarial QA focused on capability drift, time flakes, stale docs, and environment assumptions. Report files changed, commands run, pass/fail evidence, classifications, and unresolved blockers.
```
