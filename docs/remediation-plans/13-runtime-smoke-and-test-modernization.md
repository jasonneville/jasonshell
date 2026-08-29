# 13 Runtime Smoke Evidence Harness and Brittle-Test Modernization

## Metadata

- Status: Ready for implementation
- Owner: validation owner
- Priority: P2
- Audit refs: P2-2 and runtime evidence gaps
- Order: 13 of 13
- Dependencies: Plans 01-12 SHOULD be complete. This final pass measures stabilized behavior and MUST NOT mask remaining red gates.
- Allowed implementation scope: test scripts/harness docs/tests, brittle test replacements, smoke docs, durable spec/changelog

## Objective and evidence

Create repeatable runtime smoke evidence path and modernize brittle tests without pretending source regex equals runtime proof.

Evidence:

- Audit P2-2: source regex/CSS literal tests overused; direct `node --test` can consume stale compiled helpers; official entry is `npm run test:node`.
- Audit runtime gaps: AppBar startup/rollback/taskbar restore/fullscreen/cleanup, DPI/zoom, multi-monitor, AT navigation, keyboard secondary actions, large data perf, PID reuse, per-surface smoke, search live timing, visual computed styles.
- Current script: `smoke:fullscreen` only.
- Current `docs/smoke-test-windows.md` is manual and limited.
- Audit says no live Tauri shell launched because startup modifies global desktop state.

## Scope

In scope:

1. Add non-destructive/dry-run-first runtime smoke harness design and implementation.
2. Add evidence artifact format under ignored `test-results/`.
3. Add opt-in script(s) for safe surface smoke and manual checklist integration.
4. Modernize brittle tests by moving visual literal/source regex checks to behavioral/helper/computed-style checks where practical.
5. Document official test entry points and stale `dist-tests` risk.

Out of scope:

- Forcing live smoke in default `npm run validate` without maintainer decision.
- Automating destructive AppBar/taskbar/work-area/global hook/process termination changes without explicit human consent.
- Claiming assistive tech support without manual NVDA/JAWS evidence.
- Fixing all current failing tests unless directly part of modernization target.
- Persistent top JSON/shell terminal and Stack Browser Git workbench feature implementation.

## Current contract

- `npm run test:node` cleans `dist-tests`, compiles `tsconfig.test.json`, runs `node --test tests/*.test.mjs`.
- `npm run validate` runs check, build, node tests, cargo tests, cargo check.
- `npm run smoke:fullscreen` runs `scripts/smoke-fullscreen-appbar.ps1`.
- Manual smoke requires `npm run tauri dev`, which can reserve AppBars, alter work area, hide/restore Explorer taskbar, install hooks, and touch process/window state.

## Functional requirements

FR-1. Smoke harness MUST default to non-destructive/dry-run mode.

FR-2. Harness MUST require explicit human consent before AppBar, taskbar, work-area, global hook, or process termination changes.

FR-3. Harness MUST write timestamped evidence artifacts under ignored `test-results/runtime-smoke/<timestamp>/`.

FR-4. Evidence MUST include command, env, consent flags, skipped checks, pass/fail/blocked status, and notes.

FR-5. Harness MUST distinguish automated evidence, manual evidence, and blocked/not-run evidence.

FR-6. Harness MUST not claim NVDA/JAWS/assistive-tech behavior unless a human records manual evidence.

FR-7. Harness MUST provide safe surface smoke checks first: build/static preflight, app launch preflight, IPC/event console-error capture where possible, open/close safe panels only after consent.

FR-8. Brittle visual tests SHOULD be replaced with computed-style/layout assertions or helper behavior tests.

FR-9. Source-regex tests MAY remain only for intentional invariants: command names, authorization ordering, event names, capability boundaries.

FR-10. Docs MUST state `npm run test:node` is official Node test entry and direct `node --test` is unsafe unless compiled artifacts are known fresh.

## Non-functional requirements

NFR-1. Harness must be safe for local developer machine by default.

NFR-2. Scripts must be PowerShell-compatible on Windows.

NFR-3. No secret/env dumping; redact sensitive env if captured.

NFR-4. Artifacts must be ignored if not already covered.

NFR-5. Test modernization must not lower behavioral coverage.

NFR-6. Full validation remains authoritative; smoke augments, not replaces, tests.

## Implementation decisions

- Add script like `scripts/runtime-smoke.ps1` with parameters: `-DryRun` default true, `-ConsentDesktopMutation`, `-ConsentProcessTermination`, `-ConsentGlobalHooks`, `-ManualEvidenceFile`.
- Add npm script `smoke:runtime` only if script is non-destructive by default.
- Store artifact JSON plus `summary.md` and `manual-evidence-template.md`.
- For app launch, dry-run should print planned commands and safety gates, not start Tauri.
- For live mode, require explicit flags and display normal-language warning before launch.
- Modernize tests incrementally: select known brittle failures like CSS literal attention/task-preview tests and convert to helper/computed-style where feasible.

## Phased RED-first implementation

### Phase 1 - Harness contract RED tests

Add `tests/runtimeSmokeHarnessContract.test.mjs`.

Failing tests:

1. `runtime smoke script defaults to dry run and records evidence path`
2. `runtime smoke script requires explicit consent for desktop mutation and process termination`
3. `runtime smoke docs forbid automated assistive technology claims without manual evidence`
4. `package scripts expose non-destructive runtime smoke entrypoint`

### Phase 2 - Harness GREEN

1. Implement script dry-run and consent gates.
2. Add npm script if desired: `smoke:runtime`: `powershell -ExecutionPolicy Bypass -File scripts/runtime-smoke.ps1 -DryRun`.
3. Write docs in `docs/smoke-test-windows.md` or new linked doc.
4. Ensure artifacts path ignored; update `.gitignore` only if necessary.
5. Update `master_spec.md` validation section with harness contract.
6. Add changelog.

### Phase 3 - Brittle test inventory RED

Add or update `tests/testModernizationPolicy.test.mjs`.

Failing tests should guard policy, not freeze every current bad pattern:

- Official Node entrypoint documented.
- Visual literal tests have owner comments or are converted.
- Source regex tests justify invariant category.

### Phase 4 - Modernize selected brittle tests

Targets from audit:

- `tests/taskPreviewTextPolish.test.mjs` expected `2.1rem` vs impl `2.4rem`: replace exact CSS size check with computed/layout intent if helper available, or update to semantic class/overflow behavior with owner comment.
- `tests/taskbarUxState.test.mjs` expected 4 px `#ffd54f` vs impl 3 px `#7e610b`: replace exact literal with class/state helper assertion or token-level contract if visual design intentionally changed.
- Keep source tests for authorization/capability/event invariants.

## Exact test names and assertions

- `runtime smoke script defaults to dry run and records evidence path`
  - Assert script declares default dry-run true or param default.
  - Assert script writes/mentions `test-results/runtime-smoke`.
  - Assert dry-run path does not invoke `npm run tauri dev` without consent.

- `runtime smoke script requires explicit consent for desktop mutation and process termination`
  - Assert script has consent params for desktop mutation and process termination.
  - Assert AppBar/taskbar/work-area/global hook/process termination steps are gated by those params.

- `runtime smoke docs forbid automated assistive technology claims without manual evidence`
  - Assert docs include `Do not claim NVDA/JAWS` or equivalent manual-evidence requirement.

- `package scripts expose non-destructive runtime smoke entrypoint`
  - Assert package script exists only if it includes `-DryRun` or no live consent flags.

- `official node tests clean dist-tests before execution`
  - Assert `package.json` `test:node` includes `scripts/clean-dist-tests.mjs`, `tsc -p tsconfig.test.json`, and `node --test tests/*.test.mjs`.

## Manual/live validation

Safety/consent rules:

- Before any live run, present warning: JasonShell may reserve AppBars, alter work area, hide/restore Explorer taskbar, install global hooks, open/close windows, and interact with process/window state.
- Require explicit human consent for:
  - AppBar reservation or work-area mutation.
  - Explorer taskbar hide/restore checks.
  - Global hook checks.
  - Process termination checks.
  - UAC/admin flows.
- Default run must be dry-run and non-destructive.
- Do not automate assistive-tech claims. Human must record AT, screen reader, DPI, multi-monitor observations.

Manual checks:

1. Run dry-run harness; verify no desktop mutation.
2. Review planned live checks and consent prompts.
3. With consent, run safe surface smoke: open/close search, tray, command, process manager, quick launch; capture console/IPC errors.
4. With separate consent, run AppBar/taskbar restore smoke.
5. With human AT tester, record NVDA/JAWS evidence in manual template.

## Edge cases

- Dev ports occupied: harness records blocked with exact error.
- Tauri app fails launch: artifact status blocked/fail, no false pass.
- Consent missing: checks skipped, not failed unless required mode requested.
- Crash before cleanup: docs include recovery steps and artifact partial status.
- Existing red tests: modernization must not hide by skipping; record exact status.

## API/type/event compatibility

- Harness adds scripts/docs/tests; no app API changes required.
- If smoke opens panels through existing UI/IPC, must use existing commands/events.
- Test modernization must preserve command/event/capability invariant checks.

## Validation commands

Focused:

```powershell
node scripts/clean-dist-tests.mjs
tsc -p tsconfig.test.json
node --test tests/runtimeSmokeHarnessContract.test.mjs tests/testModernizationPolicy.test.mjs
npm run smoke:runtime
npm run check
```

Full:

```powershell
npm run test:node
npm run cargo:test
npm run cargo:check
npm run build
npm run validate
```

Live, only with consent:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/runtime-smoke.ps1 -DryRun:$false -ConsentDesktopMutation
```

Add additional consent flags only for explicitly approved checks.

## Acceptance criteria

- Given developer runs default smoke command, When no consent flags are passed, Then no AppBar/taskbar/work-area/global-hook/process-termination mutation occurs.
- Given live smoke is requested without consent, When harness reaches a risky check, Then it records skipped/blocked and explains required consent.
- Given smoke completes or blocks, When artifacts are inspected, Then summary includes evidence type, status, command, flags, skipped checks, and notes.
- Given docs mention AT checks, When no manual evidence exists, Then docs do not claim NVDA/JAWS pass.
- Given brittle visual tests are modernized, When style implementation changes without behavior change, Then tests do not fail solely due to arbitrary pixel/color literals.
- Given source tests guard security ordering/events/capabilities, When modernization runs, Then those intentional source invariants remain covered.

## Risks and rollback

- Risk: harness accidentally mutates desktop. Mitigation: dry-run default plus explicit consent gates and tests.
- Risk: too much smoke flakiness. Mitigation: classify blocked/skipped/manual separately.
- Risk: test modernization weakens coverage. Mitigation: require replacement behavioral assertion before deleting literal assertion.
- Rollback: remove smoke script/npm script/tests/docs; no persistence migration.

## Docs updates

- `master_spec.md`: validation/smoke harness contract and official Node test entry.
- `docs/smoke-test-windows.md` or new runtime smoke doc: dry-run/consent/manual evidence policy.
- `changelog.md`: durable workflow/test docs entries.
- Maybe `.gitignore`: ensure `test-results/` artifacts ignored.

## Handoff checklist

- [ ] Read audit/spec/changelog policy/package scripts/current smoke docs/cited brittle tests.
- [ ] Preserve unrelated dirty work.
- [ ] Add RED harness contract tests.
- [ ] Implement dry-run default and consent gates.
- [ ] Add evidence artifact format.
- [ ] Modernize selected brittle tests with replacement behavior checks.
- [ ] Update durable docs/changelog.
- [ ] Run focused/full validation.
- [ ] Do not perform live risky smoke without explicit consent.
- [ ] Run adversarial QA.

## Copy/Paste Implementation Prompt

```text
You are implementing Plan 13 in C:\dev\jasonshell. Read docs/current-state-technical-audit-2026-08-28.md, master_spec.md, CHANGELOG_POLICY.md, package.json scripts, docs/smoke-test-windows.md, cited brittle tests, and docs/remediation-plans/13-runtime-smoke-and-test-modernization.md first. Preserve unrelated dirty work. Do not mask existing red gates, implement unrelated product fixes, implement persistent top JSON/shell terminal work, or implement Stack Browser Git workbench feature work.

Goal: address P2-2 and runtime evidence gaps by adding a runtime smoke evidence harness that defaults non-destructive/dry-run, requires explicit human consent before AppBar/taskbar/work-area/global hook/process termination changes, writes evidence artifacts, and distinguishes automated/manual/blocked evidence. Also modernize selected brittle visual/source-literal tests into behavioral/helper/computed-style evidence where practical.

Use RED-first: add failing harness/policy tests with exact plan names, confirm fail, then implement. Keep source tests for intentional security/event/capability invariants. Do not automate assistive-tech claims; NVDA/JAWS/DPI/multi-monitor claims require manual evidence. Default smoke command must not start live desktop mutation without consent.

After changes: update master_spec.md, smoke docs, changelog.md per CHANGELOG_POLICY.md, and .gitignore only if needed for artifacts. Run focused validation, dry-run smoke, then npm run check, npm run build, npm run test:node, npm run cargo:test, npm run cargo:check, npm run validate. Record exact pre-existing failures. Before any live smoke, obtain explicit human consent and list risks in normal clear language. Run adversarial QA before done.
```
