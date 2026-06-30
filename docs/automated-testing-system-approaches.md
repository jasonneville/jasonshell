# Automated Testing System Approaches

_Last updated: 2026-06-03_

## Goal

Design a stronger automated regression testing system that can be used across:

- `C:/dev/jasonshell` — Windows/Tauri 2/Svelte 5/TypeScript/Rust shell prototype with native Win32 behavior.
- `C:/dev/automated-harness` — TypeScript pnpm monorepo with server, web dashboard, CLI, core domain package, and engine package.

The system should catch breakage from new updates without turning every change into a slow, flaky, manual validation session.

## Current baseline

### JasonShell

Observed validation commands:

```powershell
npm run check
npm run build
npm run test:node
npm run cargo:test
npm run cargo:check
npm run validate
```

Existing characteristics:

- Uses npm, Svelte check, TypeScript build, Node source tests, Rust cargo tests/checks.
- Has Windows CI in `.github/workflows/windows-ci.yml` that runs check/build/Node tests/Cargo tests/Cargo check.
- Has many source-level regression tests in `tests/*.test.mjs` and Rust module tests.
- Still has live behavior gaps that are hard to prove with source tests only: WebView2 delivery, AppBar geometry, Explorer drag/drop, native menus, focus-loss behavior, and mouse/hotkey behavior.

### Automated Harness

Observed validation commands:

```bash
pnpm typecheck
pnpm test
pnpm build
pnpm check
pnpm smoke:pi
pnpm smoke:pi:sdk
```

Existing characteristics:

- Uses pnpm workspace orchestration and Vitest.
- Has package/app tests under `apps/server`, `apps/web`, `packages/core`, `packages/engine`, and `packages/cli`.
- No observed GitHub Actions workflow in the repo.
- Smoke tests involving Pi are opt-in and should stay out of ordinary hermetic CI unless explicitly enabled.

## Design principles from industry-grade systems

1. **Fast tests first, slow tests last.** The practical test pyramid model favors many small deterministic tests, fewer integration tests, and a small number of full E2E tests.
2. **Classify tests by cost and isolation.** Google-style small/medium/large thinking keeps local gates fast while reserving environment-heavy tests for nightly/release gates.
3. **Make E2E tests realistic but scarce.** E2E catches wiring issues that lower layers miss, but Google’s guidance warns that too many E2E tests become slow, flaky, and hard to diagnose.
4. **Test user-visible behavior.** Playwright and Cypress both emphasize stable selectors/locators, isolated tests, and avoiding brittle implementation-coupled checks.
5. **Prefer contract tests over full-stack E2E for boundaries.** Pact-style consumer-driven contracts catch API/message compatibility breaks faster than booting every dependent system.
6. **Make test runs hermetic and diagnosable.** Bazel’s test model is strict about declared inputs, temp output locations, timeouts, sharding, tags, and reproducibility.
7. **Record artifacts, not just pass/fail.** Every automated gate should produce machine-readable results, logs, screenshots/traces when applicable, durations, selected test rationale, and flake status.

## Approach 1 — Layered deterministic regression pyramid

### Summary

Use a shared test taxonomy and gate model across both repos:

- **Small:** pure unit/source tests with no real network, no GUI, no real filesystem beyond temp directories.
- **Medium:** package/module integration tests with controlled filesystem, local server, local DB/file store, mocked external processes.
- **Large:** full application or OS/browser/native tests.
- **Smoke:** shortest representative checks before merge or before release.
- **Manual/live smoke:** explicit human or local-machine checks for behavior that cannot yet be automated reliably.

For JasonShell, this keeps the current Node/Rust source-test strength and adds clearer labels around what is not covered. For Automated Harness, this fits Vitest package tests and can expand into server/web integration tests without requiring browser E2E immediately.

### What it would look like

Create a shared manifest such as `testing.manifest.json` or `testing.manifest.ts` per repo:

```json
{
  "project": "jasonshell",
  "gates": {
    "local": ["npm:check", "npm:test:node", "cargo:targeted"],
    "prepush": ["npm:validate"],
    "ci": ["npm:check", "npm:build", "npm:test:node", "cargo:test", "cargo:check"],
    "nightly": ["ci", "smoke:windows-live"]
  },
  "commands": {
    "npm:check": { "cmd": "npm run check", "size": "small" },
    "npm:test:node": { "cmd": "npm run test:node", "size": "small" },
    "cargo:test": { "cmd": "npm run cargo:test", "size": "medium" },
    "smoke:windows-live": { "cmd": "docs/smoke-test-windows.md", "size": "large", "manual": true }
  }
}
```

Automated Harness equivalent:

```json
{
  "project": "automated-harness",
  "gates": {
    "local": ["pnpm:typecheck", "pnpm:test"],
    "ci": ["pnpm:typecheck", "pnpm:test", "pnpm:build"],
    "nightly": ["ci", "smoke:pi"]
  },
  "commands": {
    "pnpm:typecheck": { "cmd": "pnpm typecheck", "size": "small" },
    "pnpm:test": { "cmd": "pnpm test", "size": "small" },
    "pnpm:build": { "cmd": "pnpm build", "size": "medium" },
    "smoke:pi": { "cmd": "pnpm smoke:pi", "size": "large", "env": { "RUN_PI_SMOKE": "1" }, "manualOptIn": true }
  }
}
```

### Pros

- Best first step because both repos already have test commands.
- Makes current validation visible and repeatable.
- Fast enough for every change if small/medium gates are kept lean.
- Works for npm, pnpm, cargo, Node tests, Vitest, Svelte check, and build checks.
- Produces a foundation for later E2E, contract, and visual tests.
- Easy to integrate into Automated Harness as a real feature: project-aware test runner, result history, and task gating.

### Cons

- Does not catch real GUI/native regressions by itself.
- Requires discipline to classify tests and keep slow tests out of the local gate.
- Without test impact selection, full validation may still get expensive as both repos grow.
- Does not solve flaky tests automatically; it only gives a structure for identifying them.

### Best fit

Use this as the **baseline system**. Build it first, then plug other approaches into it.

## Approach 2 — User-journey E2E and smoke automation

### Summary

Add a small set of high-value E2E tests that exercise critical user journeys through a running app:

- Automated Harness: browser/API journeys with Playwright or Cypress.
- JasonShell: selective Tauri/WebView/native smoke automation where practical, plus manual smoke checklists for Windows-only shell behaviors until automation is reliable.

The key is to keep this suite small and focused. E2E should verify that the product works from the user’s perspective, not re-test every source-level branch.

### What it would look like

Automated Harness initial Playwright journeys:

- Server health endpoint loads.
- Board page renders tasks from test data.
- Create task through UI, verify API/store update.
- Task detail opens and renders runtime/profile controls.
- Settings page saves and reloads a safe setting.

JasonShell initial smoke journeys:

- Launch app in a controlled Windows test account.
- Top bar and bottom bar appear and reserve expected work area.
- Search panel opens, accepts query, closes on Escape.
- Terminal panel opens, starts a shell, echoes a simple command, closes without leaking session state.
- Stack Browser opens for a known folder and renders rows.

Artifact expectations:

- Failure screenshots.
- Playwright traces or equivalent UI automation traces.
- App logs.
- Test environment metadata: OS, display scale, app version, commit, command, timeout.

### Pros

- Catches wiring and runtime regressions that unit tests miss.
- Gives high confidence in user-critical workflows.
- Playwright has strong locator, auto-waiting, trace, parallel, and CI support.
- Automated Harness is a conventional server/web target, so E2E is straightforward there.
- Can produce artifacts that make failures easier to reproduce.

### Cons

- E2E tests are slower and more fragile than source tests.
- JasonShell native shell behaviors depend on Windows desktop state, display scale, focus, AppBar interactions, WebView2, and foreground windows.
- Requires careful test data setup/teardown.
- Too many E2E tests will slow development and create noisy failures.
- GUI tests can require a dedicated Windows runner rather than a generic CI environment.

### Best fit

Use this for **critical smoke and release confidence**, not as the main regression layer.

## Approach 3 — Contract and boundary testing

### Summary

Use contract tests to lock down API, event, IPC, package, and message boundaries without running the whole system. This is especially useful for Automated Harness and for JasonShell’s Rust/TypeScript command/event contracts.

Examples:

- Automated Harness server API contracts between `apps/server` and `apps/web`/`packages/cli`.
- Engine/runtime contracts for planner/executor/reviewer result payloads.
- JasonShell Tauri command contracts between frontend wrappers and Rust command payloads.
- Event registry contracts: every emitted event must exist in the canonical registry and every frontend shared event constant must match Rust authority.

### What it would look like

Automated Harness:

- Define request/response fixtures or Pact contracts for server endpoints.
- Verify the provider (`apps/server`) against consumer expectations from web/CLI tests.
- Store contract artifacts and fail if a provider breaks an existing consumer.

JasonShell:

- Expand existing source-test style contracts:
  - IPC command names and argument schemas.
  - Tauri event names and payload shapes.
  - Settings JSON schema migration and merge behavior.
  - Terminal event payload sequencing.
  - Search result payload validation.

### Pros

- Much faster than full E2E for compatibility regressions.
- Gives precise failure messages at the boundary that broke.
- Useful for AI-driven development because it protects contracts from accidental drift.
- Works well before a browser/native E2E suite is mature.
- Pact-style consumer-driven contracts test only interactions consumers actually use.

### Cons

- Requires writing and maintaining explicit contracts or fixtures.
- Does not prove the full UI or native desktop behavior works.
- Can give false confidence if contracts are too shallow or not generated from real consumers.
- Versioning/publishing contracts adds process overhead if multiple repos consume them.

### Best fit

Use this for **API/IPC/event/schema stability**. It should become a major layer between source tests and E2E.

## Approach 4 — Harness-centric orchestration, test impact selection, and flake control

### Summary

Build the actual “better automated testing system” as a reusable runner, likely inside `C:/dev/automated-harness`, that can run both repositories’ manifests, collect results, decide which gates to run, and track regression history.

Core capabilities:

- Project registry: knows repo path, package manager, supported gates, OS requirements.
- Test manifest parser: maps gate names to commands, sizes, tags, timeouts, env, artifacts.
- Command runner: npm, pnpm, cargo, Playwright/Vitest, generic shell command support.
- Result model: JSON summary, logs, durations, exit codes, failed test names, artifacts.
- Test impact selection: use git diff and package graph to run affected tests first.
- Flaky test policy: retry only in defined lanes, quarantine with owner/reason, never silently hide failures.
- Historical dashboard: pass/fail trend, slowest tests, flake rate, time-to-signal.

### What it would look like

Example command shape:

```bash
harness-test run --project C:/dev/jasonshell --gate local
harness-test run --project C:/dev/jasonshell --gate ci --json .harness/test-runs/latest.json
harness-test run --project C:/dev/automated-harness --gate affected --base main
harness-test list --project C:/dev/automated-harness
harness-test report --last 20
```

Example result record:

```json
{
  "project": "jasonshell",
  "gate": "local",
  "commit": "abc123",
  "startedAt": "2026-06-03T12:00:00Z",
  "durationMs": 84231,
  "status": "failed",
  "commands": [
    {
      "id": "npm:test:node",
      "cmd": "npm run test:node",
      "size": "small",
      "durationMs": 19200,
      "exitCode": 1,
      "logPath": ".harness/test-runs/abc123/npm-test-node.log"
    }
  ],
  "selection": {
    "mode": "affected",
    "reason": "src/features/terminal changed; selected terminal Node tests and terminal Rust module tests"
  }
}
```

### Pros

- Turns testing into a productized workflow rather than scattered scripts.
- Can support both repos despite npm/pnpm/cargo differences.
- Makes failures auditable and comparable over time.
- Enables fast local gates and comprehensive CI/nightly/release gates.
- Natural fit for Automated Harness because it already models tasks, execution, runtime profiles, and local automation.
- Can grow into a dashboard and AI-assisted failure triage system.

### Cons

- More engineering effort than simply adding tests.
- Must avoid becoming a flaky wrapper around flaky commands.
- Test impact selection can be dangerous if too aggressive; default should be conservative.
- Needs artifact retention and log-size limits to avoid noisy storage growth.
- Windows GUI automation adds runner complexity.

### Best fit

Use this as the **long-term system**. It should orchestrate the other approaches rather than replace them.

## Approach 5 — Golden, snapshot, and visual regression testing

### Summary

Capture stable outputs and compare them across updates:

- TypeScript/Rust pure function snapshots for structured payloads.
- API response snapshots for safe deterministic fixtures.
- DOM/component snapshots only where they are stable and meaningful.
- Visual screenshots for selected UI surfaces and journeys.
- Terminal/output golden files for command formatting and parser behavior.

For JasonShell, this is valuable for geometry, command payloads, terminal parsing, search ranking examples, and event contracts. For Automated Harness, it is valuable for CLI output, API JSON payloads, dashboard components, and runtime result formatting.

### Pros

- Good at catching accidental UI/output drift.
- Helpful for AI-assisted changes because snapshots expose unintended broad changes.
- Visual diffs can catch CSS/layout regressions that unit tests miss.
- Golden text/JSON tests are fast when kept deterministic.

### Cons

- Snapshots can become noisy if overused.
- Visual tests need stable fonts, DPI, viewport, OS theme, animation control, and baseline review.
- Bad snapshots can bless implementation details instead of behavior.
- Requires a review workflow for intentional snapshot updates.

### Best fit

Use sparingly for **high-value stable outputs and critical UI surfaces**. Avoid broad snapshotting of volatile UI.

## Recommended combined architecture

Best overall path is not one approach alone. Use a layered system:

1. **Manifest-driven runner** as the shared interface.
2. **Deterministic pyramid** as the default gate model.
3. **Contract tests** for API/IPC/event/schema boundaries.
4. **Small E2E smoke suite** for real user journeys.
5. **Golden/visual checks** for stable UI/output regression points.
6. **Impact selection and flake tracking** after the first four are reliable.

Recommended gate names:

| Gate | Intended use | JasonShell | Automated Harness |
| --- | --- | --- | --- |
| `local` | Fast pre-commit confidence | `npm run check`, focused Node/Rust tests | `pnpm typecheck`, affected Vitest |
| `prepush` | Strong local confidence | `npm run test:node`, targeted cargo | `pnpm test`, package builds |
| `ci` | Merge protection | current Windows CI validation | `pnpm check` on CI |
| `nightly` | Slow full sweep | full validation + selected Windows smoke | full check + Playwright journeys + optional smoke |
| `release` | Highest confidence | CI + manual/live Windows smoke checklist | CI + E2E + opt-in Pi smoke |

## Phased adoption plan

### Phase 1 — Unify existing validation commands

Deliverables:

- Add a test manifest to both repos.
- Add a runner command in Automated Harness or a small standalone package.
- Support npm, pnpm, cargo, command timeouts, logs, JSON result output.
- Run both repos through the same interface.

Acceptance:

```bash
harness-test run --project C:/dev/jasonshell --gate local
harness-test run --project C:/dev/automated-harness --gate local
```

### Phase 2 — Add CI parity and result history

Deliverables:

- Add CI workflow for Automated Harness.
- Make JasonShell CI and local manifest use the same gate names.
- Persist `.harness/test-runs/*.json` summaries.
- Track slow commands and failed commands.

Acceptance:

- CI uses the same manifest as local runs.
- A failed test run has a JSON summary and log paths.

### Phase 3 — Strengthen contract tests

Deliverables:

- Automated Harness API contracts for server/web/CLI boundaries.
- JasonShell IPC/event/settings contracts where missing.
- Contract test gate included in `ci`.

Acceptance:

- Breaking a command/event/API payload fails before E2E.

### Phase 4 — Add small E2E smoke suites

Deliverables:

- Automated Harness Playwright smoke suite.
- JasonShell first automated Windows smoke only for stable surfaces.
- Store traces/screenshots on failure only.

Acceptance:

- Nightly gate runs smoke tests without blocking local development.

### Phase 5 — Add impact selection and flake policy

Deliverables:

- Diff-based selection for common file changes.
- Package graph selection for Automated Harness pnpm workspace.
- Conservative mapping for JasonShell feature/test modules.
- Flake registry with quarantine reason, owner, expiry, and linked issue/task.

Acceptance:

- A focused change runs a smaller selected gate with recorded reasoning.
- Unknown/high-risk changes fall back to full validation.

## Suggested next implementation target

Build **Approach 4 as the orchestrator**, but start with **Approach 1** inside it.

Why:

- It immediately improves both repos without waiting for fragile E2E automation.
- It preserves JasonShell’s existing strong validation commands.
- It gives Automated Harness a natural product direction: testing orchestration and result history.
- It creates the slots where Playwright, contract tests, visual checks, smoke tests, and impact selection can plug in later.

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| E2E flakiness hides real regressions | Keep E2E small, isolate state, store traces, retry only in nightly/release with explicit flake labels. |
| Test selection misses breakage | Start conservative; run full validation for lockfiles/config/core/shared files or unknown mappings. |
| Logs/artifacts grow without bound | Cap log bytes, compress large artifacts, keep recent local runs only, upload CI artifacts with retention policy. |
| JasonShell native behavior is hard to automate | Separate source contracts, automated smoke, and manual/live Windows smoke. Do not pretend source tests cover AppBar/WebView2/desktop focus completely. |
| Runner becomes another maintenance burden | Keep manifest schema small, wrap existing commands first, avoid replacing native test frameworks. |
| Snapshot tests become noisy | Snapshot only stable contracts/outputs; require explicit review to update baselines. |

## Repo-local evidence checked

- `C:/dev/jasonshell/package.json` — existing npm validation scripts.
- `C:/dev/jasonshell/.github/workflows/windows-ci.yml` — existing Windows CI gate.
- `C:/dev/jasonshell/README.md` — current validation guidance and live Windows smoke caveats.
- `C:/dev/jasonshell/tsconfig.test.json` — source-test TypeScript compilation scope.
- `C:/dev/automated-harness/package.json` — pnpm workspace validation and smoke scripts.
- `C:/dev/automated-harness/pnpm-workspace.yaml` — workspace package layout.
- `C:/dev/automated-harness/README.md` — package/app purpose, run commands, and opt-in Pi smoke context.
- `C:/dev/automated-harness/scripts/check.mjs` — existing check order: typecheck, test, build.

## Sources

- [The Practical Test Pyramid — Martin Fowler](https://martinfowler.com/articles/practical-test-pyramid.html)
- [Just Say No to More End-to-End Tests — Google Testing Blog](https://testing.googleblog.com/2015/04/just-say-no-to-more-end-to-end-tests.html)
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
- [Cypress Best Practices](https://docs.cypress.io/app/core-concepts/best-practices)
- [Pact Documentation](https://docs.pact.io/)
- [Bazel Test Encyclopedia](https://bazel.build/reference/test-encyclopedia)
- [Vitest Guide](https://vitest.dev/guide/)
