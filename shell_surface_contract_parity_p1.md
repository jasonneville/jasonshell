# Shell Surface Contract Parity P1

Status: Draft, implementation not started  
Source: `findings.md`, Suggested Fix Order item 1  
Priority: P1, run first

## Goal

Fix the foundation drift where `audio-panel` is a shipped Tauri surface but is missing from capability and registry contracts. This must land before event/lifecycle/layout work so later workers can trust surface labels and permissions.

## Issue

`audio-panel` exists in `shell_windows.rs`, routes through `App.svelte`, and is recognized by `src/lib/shellSurface.ts`, but it is absent from:

- `src-tauri/capabilities/audio-panel.json`
- `src/ipc/surfaces.ts`
- `src-tauri/src/contracts.rs`
- current capability parity expectations

Tauri 2 capability docs require permissions to be associated with window labels, and Context7 confirms targeted events should use explicit targets such as `emit_to`.

## Phase 1: RED Surface Inventory Tests

Acceptance criteria:

- A Node parity test fails when any surface created by `src-tauri/src/shell_windows.rs` lacks a matching frontend registry, route, and capability file.
- A Rust contract test fails when any shipped shell surface label is missing from `contracts::surfaces::ALL`.
- The tests fail against the current `audio-panel` omissions before implementation.

Tests to write:

- Add or extend `tests/contractsSettings.test.mjs` to compare labels across:
  - `src-tauri/src/shell_windows.rs`
  - `src/lib/shellSurface.ts`
  - `src/ipc/surfaces.ts`
  - `src/App.svelte`
  - `src-tauri/capabilities/*.json`
- Add Rust coverage in `src-tauri/src/contracts.rs` or nearby tests requiring all shell window labels to appear in `contracts::surfaces::ALL`.

Implementation tasks:

- Do not change production wiring in this phase.
- Make the failing test output list missing labels and missing files.
- Avoid brittle manual arrays when a source file can be parsed or normalized in one helper.

Subagents to run:

- `surface-contract-test-worker`: use `tdd-guide`, `senior-frontend`, `senior-backend`.
- `rust-contract-test-worker`: use `rust-skills`, `tdd-guide`.

Validation gate:

- `node --test tests/contractsSettings.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml contracts`

## Phase 2: Registry And Capability Repair

Acceptance criteria:

- `audio-panel` has a Tauri capability JSON with the same scoped baseline permissions used by comparable auxiliary surfaces.
- `src/ipc/surfaces.ts` exports `audio-panel` with a stable descriptive title.
- `src-tauri/src/contracts.rs` includes `audio-panel` in `surfaces::ALL`.
- Existing `App.svelte` and `src/lib/shellSurface.ts` routing remain unchanged except where parity tests require normalization.

Tests to write:

- Extend the Phase 1 parity tests so they pass only when `audio-panel` is represented everywhere.
- Add a negative fixture/assertion that a future missing capability file fails the test.

Implementation tasks:

- Add `src-tauri/capabilities/audio-panel.json`.
- Add `audioPanel: 'audio-panel'` or equivalent canonical entry to `src/ipc/surfaces.ts`.
- Add `AUDIO_PANEL` to the Rust surface contract ledger.
- Keep labels exactly aligned with `shell_windows::AUDIO_PANEL_LABEL`.

Subagents to run:

- `surface-parity-impl-worker`: use `senior-frontend`, `senior-backend`, `rust-skills`, `tdd-guide`.
- `contract-ledger-worker`: use `rust-skills`, `spec-driven-workflow`.

Validation gate:

- `npx tsc -p tsconfig.test.json`
- `node --test tests/contractsSettings.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml contracts`
- `npm run check`

## Phase 3: Adversarial QA

Acceptance criteria:

- No shipped surface remains represented in only some registries.
- The parity tests guard future surfaces, not just `audio-panel`.
- The plan result is recorded in `master_spec.md` when implemented.

Tests to write:

- No new tests unless QA finds a missed registry.

Implementation tasks:

- Run adversarial review against the diff with focus on stale manual arrays and capability over-permission.
- Update `master_spec.md` functional sections only if behavior/contract surface changes.

Subagents to run:

- `surface-contract-adversarial-reviewer`: use `adversarial-reviewer`, `rust-skills`, `senior-frontend`.

Validation gate:

- Full gate when feasible: `npm run validate`.
- If full gate fails for unrelated dirty-tree state, record exact focused gates and blocker.

