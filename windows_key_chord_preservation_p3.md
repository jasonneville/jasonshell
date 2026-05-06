# Windows Key Chord Preservation P3

Status: Completed 2026-05-06; automated validation passed; live smoke checklist recorded in `shell_event_windows_key_smoke_p2_p3.md` and `Win+L` requires user consent  
Source: `findings.md`, Suggested Fix Order item 3  
Priority: P3, run after `shell_open_close_event_correctness_p2.md`

## Goal

Preserve native Windows-key chords while bare Windows-key tap opens JasonShell centered search and does not open Start.

## Issue

`windows_key_hook.rs` suppresses Windows-key down/up events. Findings warn that `Win+R`, `Win+D`, `Win+E`, and similar chords may break because Windows might never see the Win modifier even if non-Win chord keys pass through.

This is user-visible and OS-level. It needs live smoke, not source tests only.

## Phase 1: Product Contract And RED Tests

Acceptance criteria:

- Product contract explicitly states:
  - Bare Left/Right Windows-key tap opens centered JasonShell search.
  - Bare Windows-key tap must not open Start.
  - Native chords such as `Win+R`, `Win+D`, `Win+E`, and `Win+L` must continue to reach Windows unless proven impossible.
- Tests fail against current suppression behavior or fail with a documented "contract impossible with current hook design" finding.

Tests to write:

- Rust classifier tests for bare tap, duplicate release, stray release, left/right overlap, and chord sequences.
- Test that potential chord paths do not suppress modifier state unless the implementation has another proven OS delivery path.
- Existing `tests/windowsKeyOverride.test.mjs` source contracts extended to capture the chosen contract.

Implementation tasks:

- Review current `windows_key_hook.rs` state machine before changing it.
- Separate the classifier contract from the Win32 hook callback where possible.
- Document any hard OS limitation before implementation.


Validation gate:

- `cargo test --manifest-path src-tauri/Cargo.toml windows_key_hook`
- `node --test tests/windowsKeyOverride.test.mjs`

## Phase 2: Hook Behavior Adjustment

Acceptance criteria:

- Bare Windows-key still opens centered search once on final bare release.
- Bare Windows-key still suppresses Start.
- Native chords pass through in live smoke.
- Hook state cannot leave Windows key "stuck" after a chord or after app exit.
- Hook install failure behavior remains fail-closed according to current spec.

Tests to write:

- Regression for chord final keyup not opening search.
- Regression for left/right Windows-key overlap.
- Regression for app shutdown/uninstall cleanup if testable.

Implementation tasks:

- Rework keydown/keyup handling so potential chords are not broken.
- Avoid logging or persisting keystrokes.
- Keep event emission limited to `search:open-centered` targeted to `top-bar`.
- Preserve existing setup failure behavior in `src-tauri/src/main.rs`.


Validation gate:

- `cargo test --manifest-path src-tauri/Cargo.toml windows_key_hook`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `node --test tests/windowsKeyOverride.test.mjs`
- `npm run check`

## Phase 3: Live Smoke

Acceptance criteria:

- Bare Win opens centered search and Start does not flash/focus.
- `Win+R` opens Run.
- `Win+D` shows desktop.
- `Win+E` opens File Explorer.
- `Win+L` locks the session or is explicitly skipped with user consent because it disrupts the session.
- Exiting JasonShell restores normal Windows-key behavior.

Tests to write:

- Manual smoke checklist in the implementation notes or a durable smoke doc if none exists.

Implementation tasks:

- Run smoke on real Windows shell only after automated tests pass.
- If smoke cannot run safely, record exact reason and leave risk open.

Validation gate:

- Manual smoke result plus full `npm run validate` when feasible.
