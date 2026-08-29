# 12 Product Truth Docs and Dependency Usage Audit

## Metadata

- Status: Ready for implementation
- Owner: validation owner
- Priority: P2
- Audit refs: P2-3, P2-4
- Order: 12 of 13
- Dependencies: Plans 01-11 SHOULD be complete so documentation describes final remediated behavior and dependency evidence uses a stable build.
- Allowed implementation scope: docs/UI truth labels/tests for planning-only behavior; dependency audit scripts/tests only after evidence; package removal only after proof and maintainer acceptance

## Objective and evidence

Align product docs/UI with actual incomplete capabilities and audit suspected unused dependencies without speculative removal.

Evidence:

- Audit P2-3: workspace restoration hardcoded `reserved-not-implemented`; startup plans never execute; automation forwarding planned not wired; multi-monitor runtime remains single-monitor; README can overstate limits.
- Audit P2-4: `package.json` contains `afplay`, `bun`, `bunx`, `osascript`; preliminary app-source search found no imports, but this is low-confidence.
- `CHANGELOG_POLICY.md` says durable behavior/spec changes update `master_spec.md`; per-change history goes to `changelog.md`.

## Scope

In scope:

1. Audit user-facing docs and UI copy for workspace restoration, startup command execution, automation forwarding, and multi-monitor ownership.
2. Add truthful labels: planning-only, reserved, not executed, single-monitor runtime where applicable.
3. Add tests that guard against overstated docs/copy if practical.
4. Build dependency graph/source/build evidence for suspected unused packages.
5. Decide keep/remove with evidence; removal only if disuse is proven and accepted.

Out of scope:

- Implementing workspace restoration/startup/automation forwarding/multi-monitor support.
- Removing dependencies based only on string search.
- Changing lockfile/package without proof.
- Marketing rewrite unrelated to truth gaps.
- Persistent top JSON/shell terminal and Stack Browser Git workbench feature implementation.

## Current contract

- Workspaces persist metadata/plans, but restoration/startup execution are not active.
- Automation parsing/planning exists; forwarding is not wired.
- Multi-monitor architecture/planning exists; runtime is single-monitor.
- No automatic workspace command execution is a safety feature.
- Package scripts use npm, Node, Tauri, Rust; no package script currently invokes `afplay`, `bun`, `bunx`, or `osascript`.

## Functional requirements

FR-1. Docs MUST clearly state workspace restoration is planning/reserved unless implementation exists with runtime evidence.

FR-2. Docs MUST clearly state startup commands are not executed automatically.

FR-3. Docs MUST clearly state automation forwarding is planned/not wired.

FR-4. Docs MUST clearly state multi-monitor runtime is single-monitor until implemented/live-tested.

FR-5. UI copy that displays these capabilities MUST label planning-only/reserved behavior.

FR-6. Dependency audit MUST inspect package scripts, TS/Svelte source, Rust source, config files, tests, build output/import graph, and lockfile reason.

FR-7. Dependency removal MUST require graph evidence, source evidence, build/test evidence, and maintainer acceptance.

FR-8. If evidence is inconclusive, dependencies MUST remain and docs MUST record uncertainty.

FR-9. No speculative dependency removal is allowed.

## Non-functional requirements

NFR-1. Docs must be ASCII Markdown.

NFR-2. Truth labels must not imply defect if capability intentionally reserved for safety.

NFR-3. Audit output must be reproducible with commands or script checked into repo if durable.

NFR-4. Package-lock changes must be reviewed carefully and only include intended dependency changes.

NFR-5. No runtime behavior change except text/labels unless separately approved.

## Implementation decisions

- Split into two workstreams: product truth docs/UI and dependency audit.
- For product truth, update `README.md`, `master_spec.md`, and any in-app copy in workspace/control-plane/automation/multi-monitor surfaces only if present.
- For dependency audit, prefer a script/report under docs or tests only if implementation owner needs repeatable evidence.
- Use `npm ls <pkg>`, lockfile inspection, ripgrep imports/requires/dynamic strings, `tsc --traceResolution` or bundler metafile if added, and production build.
- Removal gate: all four packages individually proven unused by app, tests, scripts, and transitive graph before removal candidate.

## Phased RED-first implementation

### Phase 1 - Product truth RED tests

Add tests such as `tests/productTruthDocs.test.mjs`.

Failing tests:

1. `README labels workspace restoration and startup execution as not implemented`
2. `README labels automation forwarding and multi-monitor runtime as planned only`
3. `master spec preserves planning-only product truth`

Assertions should search for explicit phrases, not exact paragraphs.

### Phase 2 - Product truth docs/UI

1. Update README capability matrix/limitations.
2. Update `master_spec.md` current system snapshot or relevant functional sections.
3. Update UI labels if existing UI currently implies active behavior.
4. Update smoke docs only if manual checks need truth labels.
5. Add changelog entry.

### Phase 3 - Dependency audit RED/evidence

Before any package changes, create evidence report, e.g. `docs/dependency-usage-audit-2026-08-28.md`, or test-only guard if preferred.

Evidence MUST include:

- `npm ls afplay bun bunx osascript`
- package scripts review
- `rg "from ['\"](afplay|bun|bunx|osascript)|require\(['\"](afplay|bun|bunx|osascript)|\b(afplay|bunx|osascript)\b"` over source/tests/config with lockfiles excluded and then included separately
- production build pass with deps present
- if removing, production build/test pass after removal
- lockfile/package diff review

### Phase 4 - Removal only if proven

If and only if graph/source/build evidence proves a dependency unused:

1. Ask/record maintainer acceptance if plan execution policy requires.
2. Remove one package or coherent set.
3. Run focused and full validation.
4. Document why removal safe and how to rollback.

If not proven, keep deps and document why.

## Exact test names and assertions

- `README labels workspace restoration and startup execution as not implemented`
  - Assert README includes `workspace restoration` near `not implemented`, `reserved`, or `planning-only`.
  - Assert README includes `startup` near `not executed` or `not run automatically`.
- `README labels automation forwarding and multi-monitor runtime as planned only`
  - Assert README includes `automation forwarding` near `planned`/`not wired`.
  - Assert README includes `multi-monitor` near `single-monitor runtime` or `planning-only`.
- `master spec preserves planning-only product truth`
  - Assert `master_spec.md` includes no automatic workspace command execution and single-monitor runtime limit.
- Optional `dependency audit report records evidence before removal`
  - Assert report contains sections `Graph evidence`, `Source evidence`, `Build evidence`, `Decision`.

## Manual/live validation

Safety/consent:

- No live Tauri run required for docs-only truth updates.
- If UI labels are checked live, human MUST consent before `npm run tauri dev` because AppBar/work-area/taskbar changes.
- Do not execute startup commands as part of this plan.

Manual checks:

1. Read README as new user; limitations are clear before use.
2. If UI changed, open relevant panel and verify labels do not imply active restoration/forwarding/multi-monitor ownership.
3. Verify no docs claim runtime support without evidence.

## Edge cases

- Docs should not erase planned architecture; they should distinguish planned vs active.
- Tests should not require exact marketing words that block copy improvements.
- Dependencies may be used by postinstall/bin side effects; inspect package metadata before removal.
- `bun` package name may be a runtime binary shim; do not assume unused because JS imports absent.
- Lockfile may include transitive deps; distinguish direct vs transitive.

## API/type/event compatibility

- Docs/UI-label changes should not alter APIs/events/types.
- Dependency removal must not alter package scripts unless explicitly needed and tested.
- If package-lock changes, only intended dependency graph changes allowed.

## Validation commands

Focused:

```powershell
node scripts/clean-dist-tests.mjs
tsc -p tsconfig.test.json
node --test tests/productTruthDocs.test.mjs
npm run check
npm run build
npm ls afplay bun bunx osascript
```

Search/audit commands:

```powershell
rg "from ['\"](afplay|bun|bunx|osascript)|require\(['\"](afplay|bun|bunx|osascript)|\b(afplay|bunx|osascript)\b" -g "!package-lock.json" -g "!node_modules" .
rg "afplay|bunx|osascript|\bbun\b" package.json package-lock.json .npmrc .github scripts src src-tauri tests docs
```

Project gates:

```powershell
npm run test:node
npm run cargo:test
npm run cargo:check
npm run build
npm run validate
```

## Acceptance criteria

- Given new user reads README, When they inspect workspaces, Then they learn restoration/startup execution are not active.
- Given user reads automation docs/UI, When they inspect forwarding, Then they learn forwarding is planned/not wired.
- Given user reads multi-monitor docs/UI, When they inspect runtime support, Then they learn runtime is single-monitor until implemented/live-tested.
- Given dependency removal is proposed, When reviewer asks for proof, Then graph/source/build evidence exists before removal.
- Given evidence is inconclusive, When plan completes, Then dependencies remain and uncertainty is documented.

## Risks and rollback

- Risk: truth labels sound too negative. Mitigation: describe as intentional safety/reserved capability.
- Risk: dependency removal breaks hidden platform tooling. Mitigation: no removal without graph/source/build proof and full validation.
- Rollback docs: revert README/spec/UI copy.
- Rollback dependency: restore package/package-lock from git, run `npm install`, rerun validation.

## Docs updates

- `README.md`: product truth/limitations.
- `master_spec.md`: current behavior/risk truth.
- Optional `docs/dependency-usage-audit-2026-08-28.md`: evidence/decision.
- `changelog.md`: entries for durable docs/dependency changes.

## Handoff checklist

- [ ] Read audit/spec/changelog policy/package scripts.
- [ ] Preserve unrelated dirty work.
- [ ] Add RED docs truth tests.
- [ ] Update README/spec/UI labels.
- [ ] Run dependency audit evidence before any package change.
- [ ] Do not remove dependencies without proof and acceptance.
- [ ] Run focused/full validation.
- [ ] Run adversarial QA.

## Copy/Paste Implementation Prompt

```text
You are implementing Plan 12 in C:\dev\jasonshell. Read docs/current-state-technical-audit-2026-08-28.md, master_spec.md, CHANGELOG_POLICY.md, package.json scripts, README.md, cited workspace/automation/multi-monitor source/tests, and docs/remediation-plans/12-product-truth-docs-dependency-audit.md first. Preserve unrelated dirty work. Do not implement workspace restoration, startup execution, automation forwarding, multi-monitor support, persistent top JSON/shell terminal work, or Stack Browser Git workbench feature work.

Goal: fix P2-3/P2-4 by making product docs/UI truthful about planning-only capabilities and by auditing suspected unused deps (`afplay`, `bun`, `bunx`, `osascript`) without speculative removal.

Use RED-first for docs truth tests. Update README/master_spec/UI labels so workspace restoration/startup execution/automation forwarding/multi-monitor runtime limits are clear. For dependencies, prove disuse with graph evidence, source evidence, package script/config evidence, lockfile reason, and build/test evidence before any removal. If evidence is inconclusive, keep deps and document uncertainty. Do not remove based only on grep.

After changes: update changelog.md per CHANGELOG_POLICY.md. Run focused tests/audit commands, then npm run check, npm run build, npm run test:node, npm run cargo:test, npm run cargo:check, npm run validate. Record exact failures if pre-existing gates remain red. Live UI checks require explicit human consent before npm run tauri dev. Run adversarial QA before done.
```
