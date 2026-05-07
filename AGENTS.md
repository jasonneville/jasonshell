# JasonShell Agent Instructions

## Core Loop

- Read agents.md in C:/Users/jnev1/.codex directory.
- Read `master_spec.md` first before engineering work in `C:\dev\jasonshell`.
- Inspect available Codex skills, load every relevant skill, and skip irrelevant skills.
- Use Context7 MCP when current framework/library documentation is needed.
- Delegate specialist implementation, testing, documentation, and QA work to suitable subagents when available.
- Keep user replies in caveman ultra mode unless the user asks for normal mode.
- Preserve unrelated dirty worktree changes.

## Durable Docs

- `master_spec.md` is the canonical current architecture and behavior spec.
- `changelog.md` is the canonical per-change history.
- Use `CHANGELOG_POLICY.md` for changelog rules.
- Do not append per-request change-log entries to `master_spec.md`.
- When behavior, commands, events, persistence, validation coverage, or known risks change, update the relevant functional section in `master_spec.md`.

## Validation

- Use RED-first tests for implementation work when practical.
- Run focused validation for touched surfaces.
- Run QA/adversarial review before declaring substantial changes complete.
