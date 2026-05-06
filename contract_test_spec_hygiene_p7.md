# Contract Test Spec Hygiene P7

Status: Draft, implementation not started  
Source: `findings.md`, Suggested Fix Order item 7 plus P2/P3 hygiene findings  
Priority: P7, run after `backend_blocking_lock_boundaries_p6.md`

## Goal

Clean up contract authority, dead event text, stale spec guidance, brittle source tests, and stale `dist-tests` risk after higher-priority behavioral fixes land.

## Issue

Findings show:

- `open_shell_path` is a broad native execution boundary.
- Event constants are split between central IPC files and feature wrappers.
- `audio:refresh` appears frontend-only, with no Rust emitter.
- `master_spec.md` has contradictory current search scheduling text.
- TopBar rail scroll timeout is untracked.
- Source-text tests are useful but overused/brittle.
- Ignored `dist-tests` artifacts can go stale and be imported accidentally.

This plan is last because it can create broad churn and should not block fixes for real user-visible runtime issues.

## Phase 1: Native Open Boundary Audit

Acceptance criteria:

- `open_shell_path` is classified by intent, not treated as a generic execution pipe.
- Executables, scripts, remote URLs, `file://`, and unknown protocols are rejected unless routed through a specific audited command.
- Allowed local file/folder and approved settings/control-panel intents remain working.

Tests to write:

- Rust negative tests for `cmd.exe`, `.ps1`, `.bat`, `.cmd`, `http://`, `file://`, and unknown protocols.
- Rust positive tests for existing local folder/file.
- Rust positive tests for allowed settings/control-panel commands if they remain in this boundary.

Implementation tasks:

- Split helpers by intent: open existing file/folder, reveal path, allowed `ms-settings:`, vetted control-panel action, explicitly allowed URL if needed.
- Keep quick-command and settings action execution paths separate from shell-path opening.

Subagents to run:

- `native-open-boundary-test-worker`: use `rust-skills`, `tdd-guide`, `senior-backend`.
- `native-open-boundary-impl-worker`: use `rust-skills`, `senior-backend`.
- `native-open-security-reviewer`: use `adversarial-reviewer`.

Validation gate:

- `cargo test --manifest-path src-tauri/Cargo.toml shell_paths`
- `npm run check`

## Phase 2: Event Registry Authority And `audio:refresh`

Acceptance criteria:

- Repository has one documented rule: `src/ipc/events.ts` is exhaustive for cross-window events, or it is explicitly a convenience subset.
- Cross-window event strings have named constants in the chosen authoritative layer.
- Rust event contracts align with frontend constants.
- `audio:refresh` is either implemented by a Rust emitter or removed/de-emphasized as dead contract.

Tests to write:

- Event parity test between `src/ipc/events.ts`, Rust `contracts::events::ALL`, and feature wrappers.
- Source scan banning raw cross-window event literals outside approved files if exhaustive authority is chosen.
- If `audio:refresh` is kept: Rust source test requiring `emit_to(AUDIO_PANEL_LABEL, AUDIO_REFRESH_EVENT, ...)`.
- If removed: Node test requiring no frontend listener/import remains.

Implementation tasks:

- Decide registry authority before edits.
- Move or document constants.
- Prefer removing/de-emphasizing `audio:refresh` unless a native watcher exists nearby.
- Update `master_spec.md` with the event authority rule.

Subagents to run:

- `event-registry-test-worker`: use `senior-frontend`, `rust-skills`, `tdd-guide`.
- `audio-refresh-contract-worker`: use `senior-frontend`, `rust-skills`.
- `event-registry-adversarial-reviewer`: use `adversarial-reviewer`.

Validation gate:

- `node --test tests/contractsSettings.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml contracts`
- `npm run check`

## Phase 3: TopBar Timeout Cleanup

Acceptance criteria:

- `setTimeout(updateRailScrollButtons, 160)` stores its timer id.
- Previous pending timeout is cleared before scheduling a replacement.
- Component cleanup clears the timeout.
- HMR/destroy cannot mutate stale component state.

Tests to write:

- Source test requiring tracked timeout variable and cleanup path.
- Existing TopBar/search tests remain green.

Implementation tasks:

- Add a `railScrollUpdateTimeout` or equivalent variable.
- Clear on reschedule and destroy.

Subagents to run:

- `topbar-timeout-worker`: use `senior-frontend`, `tdd-guide`.
- `topbar-timeout-reviewer`: use `adversarial-reviewer`.

Validation gate:

- `node --test tests/searchCloseReset.test.mjs tests/searchPanelState.test.mjs`
- `npm run check`

## Phase 4: Master Spec Search Text Cleanup

Acceptance criteria:

- Current Search section has one authoritative no-debounce/no-coalescing visible typed-search contract.
- Stale zero-delay/latest-only scheduling language is marked historical, superseded, or removed from the current-behavior section.
- Future agents cannot read current spec and reintroduce the old one-character-behind bug.

Tests to write:

- Spec consistency test banning stale current-section phrases such as `queueSearchQueryProcessing` and zero-delay latest-only provider scheduling.
- Existing search typing tests remain green.

Implementation tasks:

- Edit current spec sections only enough to remove contradiction.
- Do not rewrite historical ledger entries.
- Add dated `[CODE]` and `[TOOL]` ledger entries.

Subagents to run:

- `search-spec-hygiene-worker`: use `spec-driven-workflow`, `senior-frontend`.
- `search-spec-adversarial-reviewer`: use `adversarial-reviewer`.

Validation gate:

- `node --test tests/searchTypingFreezePhase1.test.mjs tests/searchPanelState.test.mjs`
- Spec consistency test.

## Phase 5: Source-Test Hygiene

Acceptance criteria:

- Source-text tests are tagged by intent: architecture contract, registry parity, security boundary, or temporary legacy guard.
- At least one brittle behavior-style source test is moved to a helper/Rust/UI behavior test as an exemplar.
- Source tests still protect command/event/security boundaries.

Tests to write:

- Meta-test requiring source-contract tests to declare an intent tag.
- Converted behavior test for the chosen exemplar.

Implementation tasks:

- Establish naming convention or `tests/source-contracts/` folder.
- Move only low-risk tests first.
- Do not weaken coverage for command/event/security contracts.

Subagents to run:

- `source-test-hygiene-worker`: use `tdd-guide`, `senior-frontend`, `rust-skills`.
- `test-hygiene-adversarial-reviewer`: use `adversarial-reviewer`.

Validation gate:

- `npm run test:node`
- `npm run check`

## Phase 6: `dist-tests` Cleanup

Acceptance criteria:

- Test compile cleans `dist-tests` before emitting compiled test helpers.
- Tests cannot import root-level stale `dist-tests/*.js`.
- Generated output remains ignored.

Tests to write:

- Node scan test rejecting `../dist-tests/*.js` root imports.
- Source/script test proving test script cleans `dist-tests` before `tsc -p tsconfig.test.json`.

Implementation tasks:

- Add safe cleanup to test script or a small cross-platform helper.
- Avoid destructive broad deletes; target only the repo-local `dist-tests` path.
- Keep import paths pointed at current compiled locations.

Subagents to run:

- `dist-tests-cleanup-worker`: use `tdd-guide`, `senior-frontend`.
- `dist-tests-adversarial-reviewer`: use `adversarial-reviewer`.

Validation gate:

- Delete `dist-tests`, run `npm run test:node`.
- `npm run validate`.

