# 01 Node Validation Baseline

## Metadata

- Status: Ready for implementation planning; do not implement from this document alone until current failures are reproduced.
- Order: 01 of 13.
- Owner: Validation remediation implementer.
- Dependencies: None. This pass establishes the Node baseline for every later pass.
- Related audit finding: P0-3 validation gates red. Node slice only.
- Allowed follow-on code scope: Tests and source required to make `npm run test:node` green, with exclusions below.

## Objective and evidence

Objective: restore official Node validation baseline by triaging and fixing all in-scope `npm run test:node` failures reported by the 2026-08-28 audit, without implementing excluded terminal or Stack Git feature behavior. Minimum repo-gate triage for excluded failures is allowed only to keep official validation meaningful.

Evidence:

- Audit lines 144-157: validation gate is release blocker; each failure must be classified as impl defect, stale test, or stale spec.
- Audit lines 317-340: `npm run test:node` failed with 744 total, 730 pass, 11 fail, 3 todo.
- `package.json`: official command is `npm run test:node` which cleans `dist-tests`, compiles `tsconfig.test.json`, then runs `node --test tests/*.test.mjs`.
- `master_spec.md` line 30 currently records stale Node status from earlier P04 work and must be corrected by implementation if validation status changes.

## Scope

In scope exact tests from audit:

1. `tests/bootstrapWindowsContract.test.mjs` line around 40: README bootstrap and lockfile-safe install contract.
2. `tests/meltMigrationWiring.test.mjs` line around 40: shared Melt primitives.
3. `tests/meltMigrationWiring.test.mjs` line around 189: BottomBar Melt command buttons.
4. `tests/quickIcons.test.mjs` line around 135: Quick Launch scoped/camelCase events.
5. `tests/stackBrowserPhase1Safety.test.mjs` line around 90: authorization before side effects.
6. `tests/stackBrowserTerminal.test.mjs` line around 272: terminal auth ordering. Excluded feature; triage only enough to decide stale gate vs shared auth break.
7. `tests/stackPopupState.test.mjs`: two sorting assertions.
8. `tests/taskPreviewRetention.test.mjs` line around 93: close fallback termination contract.
9. `tests/taskPreviewTextPolish.test.mjs` line around 23: expected `2.1rem`, impl `2.4rem`.
10. `tests/taskbarUxState.test.mjs` line around 64: expected 4 px `#ffd54f`, impl 3 px `#7e610b` plus shared 2 px warning style.

Likely source/doc files to inspect, exact edits only if proven authoritative:

- `README.md`
- `scripts/bootstrap-windows.ps1`
- `src/components/melt/*`
- `src/components/BottomBar.svelte`
- `src/components/BottomBar.css` or taskbar CSS modules if present
- `src/components/QuickLaunchPanelSurface.svelte`
- `src-tauri/src/quick_launch*` capability/contract files if source tests cite them
- `src-tauri/src/stack_popup.rs`, `src-tauri/src/stack_popup/*`
- `src-tauri/src/task_preview.rs`, task-window close code cited by test
- `src/features/stack-browser/*` and `src/lib/stackPopup.ts` only for non-Git/non-terminal shared contracts
- `master_spec.md`, `changelog.md` only as required durable docs after behavior/validation status changes

Explicit out of scope:

- Implementing persistent terminal behavior, terminal UX, terminal feature semantics, or Stack Browser Git workbench behavior.
- Broad test rewrites to make red tests disappear without deciding authoritative contract.
- Running live Tauri shell unless user explicitly consents; startup can reserve AppBars and alter desktop work area.

## Current-state contract

- `npm run check` and `npm run build` pass with one known Svelte a11y warning per audit.
- `npm run test:node` is red and official; direct `node --test` without clean/build is not authoritative.
- Source-contract tests are accepted when they guard deliberate security ordering, capabilities, or event names; brittle visual literal tests may be converted to behavioral/computed-style tests only with clear evidence.
- Excluded terminal/Git areas remain out of product-quality scope, but failing official gates must still be explained or quarantined by an explicit test marker only if repo policy supports it.

## Requirements

### Functional

1. Implementation MUST reproduce current `npm run test:node` failures before code changes and capture exact failing test names.
2. Each failing Node test MUST be classified as implementation defect, stale test, stale spec, or excluded-feature gate.
3. For implementation defects, code MUST be changed to restore the documented contract rather than weakening tests.
4. For stale tests, tests MUST be updated to assert current intended behavior at behavior/API level where practical.
5. For stale specs, `master_spec.md` MUST be updated in the relevant functional section and tests aligned to the new contract.
6. Terminal and Stack Git behavior MUST NOT be implemented or redesigned; `stackBrowserTerminal` failure gets minimum auth-order gate triage only.
7. Quick Launch event naming MUST remain nonce/caller scoped and compatible with existing panel lifecycle contracts.
8. Stack Browser side-effect tests MUST preserve caller authorization before filesystem/native side effects.
9. Sorting behavior MUST be deterministic and match Stack Browser current intended folders-first/downloads rules.
10. Task preview/taskbar visual tests MUST be reconciled with `master_spec.md` current attention/text-polish contract.

### Nonfunctional

11. Node gate MUST be run through `npm run test:node`, not stale `dist-tests`.
12. Fixes SHOULD reduce brittle source literal assertions when a pure helper/component-style assertion is feasible.
13. Changes MUST preserve unrelated dirty worktree edits.
14. No production terminal/Git behavior changes SHOULD be made without escalation.

## Decisions and implementation approach

- Use official gate first, then focused reruns after each cluster.
- Prefer smallest authoritative fix per failure.
- Recommended cluster order: bootstrap docs/scripts, Melt wiring, Quick Launch events, Stack auth/sorting, task preview/taskbar visual contracts.
- Stop/escalate if any failure requires terminal/Git product behavior, live desktop mutation, or incompatible contract choice.

## Phases with RED-first tests

1. Reproduce RED: run `npm run test:node`; save failing names/output.
2. Cluster triage: open each failing test and cited source; write classification in implementation notes.
3. RED focused proof: for each cluster, run focused test before edits and confirm it fails for expected reason.
4. Fix cluster code/test/spec.
5. GREEN focused proof: rerun exact focused test file.
6. Full Node proof: run `npm run test:node`.
7. Cross-gate proof: run `npm run check` and `npm run build` if Svelte/source touched.
8. Durable docs: update `master_spec.md` validation status and `changelog.md` per policy if behavior/tests/status changed.
9. QA: adversarial review checks test weakening, excluded-feature creep, stale `dist-tests`, and spec drift.

## Exact tests and assertions

- Existing `bootstrapWindowsContract.test.mjs`: asserts README/bootstrap contract and lockfile-safe install wording/source agreement.
- Existing `meltMigrationWiring.test.mjs`: asserts shared Melt primitive wrappers and command button migration where contract says Melt applies.
- Existing `quickIcons.test.mjs`: asserts Quick Launch events remain scoped and camelCase-compatible.
- Existing `stackBrowserPhase1Safety.test.mjs`: asserts authorization precedes side effects.
- Existing `stackBrowserTerminal.test.mjs`: assert only shared terminal auth ordering if kept in official gate; otherwise document exclusion handling.
- Existing `stackPopupState.test.mjs`: asserts deterministic sorting with folders first and downloads modified desc/nulls last.
- Existing `taskPreviewRetention.test.mjs`: asserts close fallback termination contract remains retained/protected.
- Existing `taskPreviewTextPolish.test.mjs`: asserts correct preview text sizing contract.
- Existing `taskbarUxState.test.mjs`: asserts task attention CSS/spec contract.
- Add tests only if replacing brittle literals; suggested names: `taskPreviewComputedTextScaleContract`, `taskbarAttentionStyleContractReflectsSpec`.

## Edge and failure cases

- Test passes direct but fails official clean build -> fix generated/source mismatch.
- Test expectation conflicts with `master_spec.md` -> choose spec unless source proves spec stale; update doc if changed.
- Excluded terminal failure is unrelated to shared safety -> escalate before disabling/skipping.
- Visual CSS literals conflict with accessible current styling -> update behavior test and spec together.

## Data/API/event compatibility impacts

- Event names/case are compatibility-sensitive for Quick Launch and Tauri capability allowlists.
- No public API additions expected.
- If command/capability set changes, update Rust contracts/capability docs and Node contract tests.

## Validation commands

Focused:

```bash
node scripts/clean-dist-tests.mjs
tsc -p tsconfig.test.json
node --test tests/bootstrapWindowsContract.test.mjs tests/meltMigrationWiring.test.mjs tests/quickIcons.test.mjs tests/stackBrowserPhase1Safety.test.mjs tests/stackBrowserTerminal.test.mjs tests/stackPopupState.test.mjs tests/taskPreviewRetention.test.mjs tests/taskPreviewTextPolish.test.mjs tests/taskbarUxState.test.mjs
```

`npm run test:node` always remains authoritative because it performs clean compilation and runs full suite. Do not use `npm run test:node -- <files>` as focused mode; wildcard in package script still selects full suite.

Full:

```bash
npm run check
npm run build
npm run test:node
npm run validate
```

`npm run validate` may remain red solely because Plan 02 has not yet repaired known Rust baseline failures. Record those failures unchanged as expected baseline evidence. Plan 01 acceptance depends on Node gate results, not concealing or repairing Plan 02 scope.

## Acceptance criteria

- Given current repo, when `npm run test:node` runs, then all in-scope Node tests pass. Covers req 1, 11.
- Given each original failure, when reviewing implementation notes, then classification and authoritative fix are recorded. Covers req 2-5.
- Given excluded terminal/Git scope, when diff is reviewed, then no terminal/Git product behavior was implemented. Covers req 6, 14.
- Given Quick Launch event consumers, when tests run, then scoped/camelCase event contract remains green. Covers req 7.
- Given Stack Browser safety tests, when source is inspected, then auth precedes side effects. Covers req 8.
- Given durable docs, when behavior/status changes, then `master_spec.md` and `changelog.md` are updated per policy. Covers req 5, 13.

## Risks and rollback

- Risk: weakening source-contract tests masks real safety bugs. Rollback: revert test-only weakening and fix source order.
- Risk: visual spec drift. Rollback: restore last documented CSS contract or update spec with explicit rationale.
- Risk: excluded terminal test needs real behavior. Rollback: leave red with documented blocker and ask owner.

## Master spec, changelog, docs updates

- Update `master_spec.md` line 30 stale validation addendum to current Node status after fixes.
- Do not add per-request ledger to `master_spec.md`.
- Append concise `[CODE]`/`[TOOL]` entries to `changelog.md` if tests/source/docs change.
- If official focused-test invocation differs, document correct validation command in relevant docs.

## Handoff checklist

- [ ] Read master spec, audit, changelog policy, package scripts.
- [ ] Reproduced RED official Node gate.
- [ ] Classified every Node failure.
- [ ] Fixed or escalated each failure without excluded behavior creep.
- [ ] Ran focused tests.
- [ ] Ran `npm run test:node`.
- [ ] Updated durable docs if status/contract changed.
- [ ] Performed adversarial QA.
- [ ] Reported exact evidence and residual red gates.

## Copy/Paste Implementation Prompt

```text
Implement remediation plan docs/remediation-plans/01-node-validation-baseline.md. First read master_spec.md, docs/current-state-technical-audit-2026-08-28.md, CHANGELOG_POLICY.md, package.json, and this plan. Preserve unrelated worktree changes. Do not implement excluded persistent terminal or Stack Browser Git feature behavior; only perform minimum repo-gate triage for excluded terminal/Git tests. Use RED-first: run npm run test:node before edits, capture exact failing tests, classify each failure as implementation defect, stale test, stale spec, or excluded-feature gate, then fix the authoritative side. Run focused validation for every touched test file, then npm run test:node, npm run check/build if Svelte or TS changed, and npm run validate if feasible. Update master_spec.md only for durable current behavior/validation status and changelog.md per CHANGELOG_POLICY.md. Perform adversarial QA focused on test weakening, excluded-scope creep, and spec drift. Report files changed, commands run, pass/fail evidence, classifications, and unresolved blockers.
```
