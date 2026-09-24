# JasonShell changelog policy

## Purpose

`changelog.md` is the append-only repository change history for JasonShell. It keeps per-change progress, implementation, validation, and workflow notes out of `master_spec.md`.

`master_spec.md` is the current architecture and behavior reference. Source, tests, and observed runtime behavior remain final authority when prose conflicts.

## Entry rules

- Preserve existing `changelog.md` history.
- Write change-history entries to `changelog.md`, not `master_spec.md`.
- Add future change-history entries under `## Change Ledger`.
- Use concise factual bullets with date or ISO timestamp and one provenance tag: `[USER]`, `[CODE]`, `[TOOL]`, or `[ASSUMPTION]`.
- Do not require a `changelog.md` entry before every request.
- Add entries when a task changes repository behavior, architecture, workflow, durable docs, tests, or validation state.
- Do not add entries for purely conversational turns, failed local exploration with no durable consequence, raw logs, secrets, tokens, or unrelated machine-local data.

## Lookup rules

- Changelog reading is targeted, not mandatory.
- Search `changelog.md` only when a task needs history, provenance, validation lineage, or prior decision context.
- Prefer keyword search for the relevant feature, command, event, file, or date. Do not read the full changelog by default.
- Historical entries are evidence of what changed, not current runtime authority. Confirm current behavior against source/tests and `master_spec.md`.

## Update order

1. Read `AGENTS.md` and `docs/current-state-brief.md`.
2. Use this policy only when history or durable-documentation impact is relevant.
3. Make the behavior, doc, or test change.
4. Update `master_spec.md` only for durable current behavior, architecture, contracts, tests, risks, or maintenance rules.
5. Append concise `[CODE]` or `[TOOL]` entries to `changelog.md` only when durable changes or validation evidence need history.

## Source-test-docs contract

`tests/changelogPolicy.test.mjs` guards this split:

- `master_spec.md` must not reintroduce mandatory first-step ledger rules or a `## Change Ledger` section.
- `CHANGELOG_POLICY.md` owns future changelog protocol.
- `AGENTS.md` must route changelog workflow through this policy.
- `changelog.md` must retain existing history under `## Change Ledger`.
