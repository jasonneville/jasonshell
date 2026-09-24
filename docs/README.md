# JasonShell docs index

Status: current index. Use it to choose what to read. Do not treat every file in `docs/` as current product truth.

## Always read first

- `AGENTS.md`: agent intake and validation rules.
- `docs/current-state-brief.md`: concise current-state guide and lookup map.
- `README.md`: active product, setup, caveats, and exact package scripts.
- `master_spec.md`: current architecture and behavior reference. Read targeted sections unless the task is architecture, security, persistence, native Windows, or cross-cutting.

## Current references

- `smoke-test-windows.md`: Windows smoke guidance. Use only when live smoke is approved.
- `adding-svg-icons.md`: icon addition workflow.
- `terminal-panel-prewarm-idle-policy.md`: terminal prewarm/idle policy.
- `architecture/phase-9-multi-monitor-architecture.md`: planning reference for multi-monitor work. Current runtime remains primary-monitor unless source/tests show otherwise.
- `plans/performance-regression/`: active plan set for performance hardening when the task touches those specific areas.
- `remediation-plans/`: ordered audit remediation plans. Read a plan only when doing that plan or touching its named issue.

## Current evidence and audits

These docs capture evidence at a point in time. Use them for leads, then confirm against source/tests.

- `current-state-technical-audit-2026-08-28.md`
- `dependency-usage-audit-2026-08-30.md`
- `stack-text-editor-*-evidence.md`

Scheduled reports are point-in-time evidence, not current product truth. Use them for leads, then confirm against source/tests.

- `docs/scheduled/09212026_WEEKLY_HEALTH_AUDIT.md`: latest weekly health audit in this checkout. Use for current audit leads and validation caveats.
- `docs/scheduled/09142026_WEEKLY_HEALTH_AUDIT.md`: older weekly health audit. Use for historical comparison only.
- `docs/scheduled/09142026_session_learn.md`: session learning report. Use for workflow/process observations, not product behavior.

## Plans and research

These are not current behavior. Read only when the task names the feature or issue.

- `stack-browser-*research*.md`
- `stack-browser-*plan*.md`
- `stack-text-editor-p*.md`
- `search-functionality-investigation.md`
- `search-functionality-implementation-plan.md`
- `taskbar-*-brainstorm.md`
- `wmux-feature-inspiration-for-jasonshell.md`

## Historical archive

- `archive/master_spec_full_legacy_2026-09-23.md`: previous full master spec. Historical snapshot from 2026-09-23. Use targeted keyword search for legacy detail after checking current `master_spec.md`, nearby source, and tests.

## Blocked or superseded docs

No root index can safely mark every older plan as blocked or superseded without re-auditing source. When status is not explicit in the file or this index, treat it as unverified planning/history, not current truth.

## Read by task type

- Small bug/doc/test task: read `AGENTS.md`, current-state brief, README section if relevant, nearby source/tests/module docs, and targeted `master_spec.md` excerpts.
- Feature behavior change: add targeted `master_spec.md`, current tests, and any named plan/evidence doc.
- Architecture/security/persistence/native Windows/cross-cutting change: read full current `master_spec.md`, relevant legacy/history excerpts, and focused plans/audits.
- Changelog/history question: read `CHANGELOG_POLICY.md`, then keyword-search `changelog.md` or the archive for the exact topic.
