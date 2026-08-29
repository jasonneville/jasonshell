# JasonShell Audit Remediation Plan Index

## Purpose

Execution map for findings in `docs/current-state-technical-audit-2026-08-28.md`. Each linked plan is a self-contained implementation session with evidence, requirements, phased RED-first work, tests, acceptance criteria, rollback guidance, durable-documentation duties, and a copy/paste implementation prompt.

Default execution is sequential, 01 through 13. Do not start a plan until its required dependencies pass their acceptance criteria or their remaining failures are explicitly documented as unrelated baseline failures.

## Global Scope

- Preserve unrelated dirty worktree changes.
- Read `master_spec.md` before engineering work.
- Follow `CHANGELOG_POLICY.md` for implemented changes.
- Do not implement persistent top JSON/shell terminal features.
- Do not implement Stack Browser Git workbench features.
- Plans 01-02 may make minimum test/contract corrections in excluded areas only when required to restore repository-wide validation.
- Use RED-first tests where practical. Never weaken safety/security tests merely to turn gates green.
- Treat `npm run test:node` as authoritative Node evidence; focused direct Node tests require clean `dist-tests` plus `tsc -p tsconfig.test.json` first.
- Do not run live Tauri shell smoke without explicit human consent. Startup can reserve AppBars, mutate work area, hide Explorer taskbars, and install global hooks.
- Update `master_spec.md` when behavior, commands, events, persistence, validation coverage, or known risks change. Update `changelog.md` for each implemented pass.

## Ordered Plans

| Order | Plan | Priority | Primary outcome | Required dependencies |
|---|---|---|---|---|
| 01 | [Node Validation Baseline](01-node-validation-baseline.md) | P0 gate | Classify and resolve 11 authoritative Node failures without excluded feature expansion | None |
| 02 | [Rust Validation Baseline and Stale Status](02-rust-validation-baseline-stale-validation-status.md) | P0 gate | Resolve 3 Rust failures; make validation status truthful | 01 |
| 03 | [AppBar Activation Lock Boundary](03-appbar-activation-lock-boundary.md) | P0 safety | Keep slow Win32/Tauri side effects outside global shell mutex; preserve rollback | 01-02 |
| 04 | [Stack Paging Resource Bounds](04-stack-paging-resource-bounds.md) | P0 resource | Bound page size, discovery, archive scanning, session storage, and continuation memory | 01-02 |
| 05 | [Process Kill Immutable Identity](05-process-kill-immutable-identity.md) | P0 safety | Prevent PID-reuse wrong-process termination using same-handle identity validation | 01-02 |
| 06 | [Listener Lifecycle and Transcript Semantics](06-listener-lifecycle-transcript-semantics.md) | P1 lifecycle/a11y | Dispose delayed Command/Tray listeners exactly once; clear transcript compiler warning | 01-02 |
| 07 | [Open With Validation and Async Boundary](07-open-with-validation-async-boundary.md) | P1 security/perf | Enforce picker target policy parity; move Stack Open With blocking work off command path | 01-02 |
| 08 | [Quick Command Stopping and Termination](08-quick-command-stopping-termination-policy.md) | P1 safety/perf | Keep stopping run visible; replace synchronous forced tree kill with safer async policy | 01-02, 06 |
| 09 | [Control Plane Accessibility](09-control-plane-accessibility.md) | P1 a11y | Align visual/ARIA tab state; restore filter focus visibility | 01-02 |
| 10 | [Process Manager Accessibility](10-process-manager-accessibility.md) | P1 a11y | Silence auto-refresh grid announcements; preserve intentional status; restore focus | 05 |
| 11 | [Quick Launch and Tray Keyboard Parity](11-quick-launch-tray-keyboard-parity.md) | P1 a11y | Add keyboard secondary actions/dismissal and visible Quick Launch focus | 06; 10 optional style precedent |
| 12 | [Product Truth and Dependency Audit](12-product-truth-docs-dependency-audit.md) | P2 docs/hygiene | Document planning-only limits; remove suspected deps only with evidence | 01-11 recommended |
| 13 | [Runtime Smoke and Test Modernization](13-runtime-smoke-and-test-modernization.md) | P2 evidence | Add dry-run/consent-gated runtime evidence path; replace selected brittle tests | 01-12 recommended |

## Dependency Graph

```text
01 -> 02
02 -> 03 -> 13
02 -> 04 -> 13
02 -> 05 -> 10 -> 13
02 -> 06 -> 08 -> 13
06 -> 11 -> 13
02 -> 07 -> 13
02 -> 09 -> 13
01..11 -> 12 -> 13
```

After Plan 02, Plans 03, 04, 05, 06, 07, and 09 are logically independent. Parallel work is safe only in isolated worktrees with later conflict reconciliation. Default remains ordered execution because shared `master_spec.md`, `changelog.md`, test registries, and UI components create merge risk.

## Audit Coverage

| Audit issue | Owning plan |
|---|---|
| Red Node suite; stale/brittle source contracts | 01, then 13 |
| Red Rust suite; stale validation snapshot | 02 |
| AppBar global mutex spans slow side effects | 03 |
| Stack response paging performs unbounded discovery/storage | 04 |
| Process kill PID reuse race | 05 |
| Command/Tray delayed-listener teardown gaps | 06 |
| Command transcript noninteractive-handler warning | 06 |
| Shell Open With picker validation mismatch | 07 |
| Stack Open With synchronous filesystem/process work | 07 |
| Quick Command run disappears during synchronous forced stop | 08 |
| Control Plane invalid mixed tab/dashboard semantics | 09 |
| Control Plane missing filter focus indicator | 09 |
| Process Manager grid live-region chatter | 10 |
| Process Manager missing filter focus indicator | 10 |
| Quick Launch mouse-only admin menu and suppressed focus ring | 11 |
| Tray missing keyboard secondary action and explicit dismissal | 11 |
| Workspace/automation/multi-monitor capability overstatement | 12 |
| Suspected unused platform/tool dependencies | 12 |
| Runtime Tauri/Win32 evidence gap | 13 |
| Brittle CSS/source-literal test overuse | 01 for current gates; 13 for durable modernization |
| Top-bar DPI/crowding static hypothesis | 13 manual, consent-gated evidence only; no defect claim before proof |

## Per-Plan Execution Contract

1. Open linked plan and use its final `Copy/Paste Implementation Prompt` in a fresh implementation session.
2. Read required repo/spec/audit files and inspect current git state before edits.
3. Reproduce baseline or add focused RED tests before implementation.
4. Make smallest contract-correct change. Respect plan out-of-scope list.
5. Run focused checks listed in plan.
6. Run full checks required by plan. Record exact unrelated failures; do not conceal them.
7. Run adversarial review/QA for touched risk boundary.
8. Update durable spec and changelog per repo policy.
9. Report changed files, evidence, remaining uncertainty, and whether next plan is unblocked.

## Global Completion Gate

Remediation sequence is complete only when:

- `npm run check` passes with no newly accepted accessibility warnings.
- `npm run build` passes.
- `npm run test:node` passes.
- `npm run cargo:test` passes, excluding documented intentional ignores.
- `npm run cargo:check` passes.
- `npm run validate` passes.
- Runtime smoke harness passes dry-run mode.
- Any live AppBar/taskbar/work-area/process-termination smoke has explicit consent and retained evidence, or is clearly marked not run.
- Audit exclusions remain unexpanded.
- `master_spec.md`, `README.md`, and `changelog.md` match shipped behavior and known limitations.
