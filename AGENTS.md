# JasonShell agent instructions

## Intake

1. Read this file and `docs/current-state-brief.md` for every task.
2. Inspect available skills and load every relevant skill.
3. Use `README.md` for active product truth, setup, caveats, and exact scripts.
4. Read task-owned source, nearby tests, and module docs before editing.
5. Use keyword search plus relevant `master_spec.md`, `docs/`, and historical excerpts. Do not read full history by default.
6. Read full current `master_spec.md` plus targeted historical material for architecture, security, persistence, native Windows, or cross-cutting work.
7. Use Context7 MCP when current framework/library docs are needed.
8. Delegate specialist implementation, testing, documentation, and QA work to suitable subagents when available.
9. Keep user replies in caveman ultra mode unless the user asks for normal mode.
10. Preserve unrelated dirty worktree changes.

## Authority order

Runtime source, tests, and observed behavior are final authority. `README.md` is active product truth for users. `master_spec.md` is the current architecture/behavior reference. `docs/current-state-brief.md` is onboarding and lookup. `docs/README.md`, plans, audits, changelog, and archived specs guide search, but they do not override current source/tests.

When prose conflicts with source/tests, follow source/tests and update the stale prose if the task owns docs.

## Durable docs

- Update `master_spec.md` only for durable current behavior, architecture, contracts, validation coverage, limits, or known risks.
- Keep `master_spec.md` out of per-change ledger format.
- Use `CHANGELOG_POLICY.md` for `changelog.md` history rules. Changelog lookup is targeted and is not mandatory for every request.
- Use `docs/README.md` to choose docs by task type.

## Validation

- Use RED-first tests for implementation work.
- Fully cover edge cases and functionality.
- Run focused validation for touched surfaces.
- Run QA/adversarial review before declaring substantial changes complete.
- Live Tauri/Windows shell smoke needs explicit human consent.
- Report `Complete` when requested scope, required automated validation, and review gates pass. Manual, hardware, or environment validation still pending belongs under `Further info` or `Next Steps`; it does not downgrade status to `Needs changes` unless it blocks requested scope or exposes a concrete defect.
