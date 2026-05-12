# JasonShell Changelog Policy

## Purpose

`changelog.md` is the append-only repository change history for JasonShell. It keeps per-change progress, implementation, validation, and workflow notes out of `master_spec.md` so the master spec can stay focused on current behavior, architecture, contracts, risks, and validation coverage.

`master_spec.md` remains canonical for behavior and architecture. When behavior, APIs, events, commands, persistence, tests, or known risks change, update the relevant functional section in `master_spec.md`. Do not use `master_spec.md` as the running ledger.

## Entry Rules

- Preserve existing `changelog.md` history.
- Write change-history entries to `changelog.md`, not `master_spec.md`.
- Add future change-history entries to `changelog.md` under `## Change Ledger`.
- Use concise factual bullets with date or ISO timestamp and one provenance tag: `[USER]`, `[CODE]`, `[TOOL]`, or `[ASSUMPTION]`.
- Do not require a `changelog.md` entry before every request. Add entries when a task changes repository behavior, architecture, workflow, durable docs, tests, or validation state.
- Do not add entries for purely conversational turns, failed local exploration with no durable consequence, raw logs, secrets, tokens, or unrelated machine-local data.
- Keep long implementation details in `master_spec.md` functional sections or focused plan/spec files when they are future-operational behavior, not in changelog bullets.

## Update Order

1. Read `master_spec.md` first for repo context.
2. Use this policy to decide whether `changelog.md` needs an entry.
3. Make the behavior/doc/test change.
4. Update `master_spec.md` only for durable current behavior, architecture, contracts, tests, risks, or maintenance rules.
5. Append concise `[CODE]` and `[TOOL]` entries to `changelog.md` when durable changes or validation evidence need history.

## Source-test docs contract

`tests/changelogPolicyHygiene.test.mjs` guards this split:

- `master_spec.md` must not reintroduce mandatory first-step ledger rules or a `## Change Ledger` section.
- `CHANGELOG_POLICY.md` owns future changelog protocol.
- `AGENTS.md` must route changelog workflow through this policy.
- `changelog.md` must retain existing history under `## Change Ledger`.
