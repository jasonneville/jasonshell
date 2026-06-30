Reasoning Level: medium

# Compact Top Header Bar Implementation Plan

## 1. Requirements & Constraints

- **REQ-001**: Reduce the default top bar height from `23.4` logical pixels to `20.0` logical pixels.
- **REQ-002**: Preserve user resizing support for the top bar, including the existing minimum clamp of `18` logical pixels and maximum clamp of `120` logical pixels.
- **REQ-003**: Make visible top-bar controls fit the compact height without clipping.
- **REQ-004**: Do not change bottom bar height, bottom bar resize behavior, or bottom bar defaults.
- **REQ-005**: Update durable documentation because top-bar default behavior changes.
- **CON-001**: Do not remove existing top-bar controls: settings, pins, terminal, command, tray, sound, time, search, and resize handle.
- **CON-002**: Preserve `MeltActionButton` usage and real button semantics.
- **PAT-001**: Follow existing shell bar resize pattern in `src/lib/shellBarResize.ts`.
- **PAT-002**: Follow existing tests in `tests/shellBarResize.test.mjs`.

## 2. Implementation Steps

### Implementation Phase 1

- **GOAL-001**: Discover and confirm every top-bar height/default source before editing.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-001 | Inspect `src/components/TopBar.svelte`, `src/components/TopBar.css`, `src/lib/settings.ts`, `src/lib/shellBarResize.ts`, `tests/shellBarResize.test.mjs`, `master_spec.md`, and `changelog.md`. | None | All hardcoded `23.4` top-bar defaults and compact-control CSS values are identified. |
| TASK-002 | Confirm no backend Rust default separately defines top-bar height by searching `src-tauri/src` for `topBarHeightLogical`, `23.4`, and top AppBar default constants. | None | Any Rust-side height defaults are documented for update, or confirmed absent. |

### Implementation Phase 2

- **GOAL-002**: Apply compact top-bar sizing while preserving resize behavior.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-003 | In `src/lib/settings.ts`, change `defaultShellSettings().ui.topBarHeightLogical` from `23.4` to `20.0`. | TASK-001 | New default settings return `topBarHeightLogical: 20.0`; bottom default remains `32.4`. |
| TASK-004 | In `src/components/TopBar.svelte`, change initial `topBarHeightLogical` and `topBarResizeStartHeight` from `23.4` to `20.0`. | TASK-001 | Top bar initializes at compact height before settings load. |
| TASK-005 | In `src/components/TopBar.css`, change `.top-bar.surface` fallback `min-height: var(--top-bar-height-logical, 23.4px)` to `20px`. | TASK-001 | CSS fallback matches new default. |
| TASK-006 | In `src/components/TopBar.css`, reduce top-bar vertical/control sizing so controls fit within `20px`: set shell home, rail scroll, stack pin buttons, search control, terminal button, command button, tray button, and sound button heights from `1.4rem` to `1.2rem`; reduce `.top-bar.surface` horizontal padding from `var(--js-space-4)` to `var(--js-space-3)` if needed for compact density. | TASK-005 | No top-bar control has a declared height greater than `1.2rem` except the `5px` resize handle. |
| TASK-007 | In `src/components/TopBar.css`, adjust font/padding values only where needed to prevent clipping: reduce `.time-pill` padding to `0 0.38rem`, `.search-control input` font size to `0.64rem`, and `.terminal-button/.command-button/.tray-button` font sizes by at most `0.04rem`. | TASK-006 | Search text, time pill, glyph buttons, and pin labels remain visually contained within the compact bar. |

### Implementation Phase 3

- **GOAL-003**: Update tests and durable docs.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-008 | Update `tests/shellBarResize.test.mjs` expected default `topBarHeightLogical` from `23.4` to `20.0`. | TASK-003 | Default settings test matches the compact top-bar default. |
| TASK-009 | Add or update a source-inspection assertion in `tests/shellBarResize.test.mjs` verifying `TopBar.css` contains fallback `20px` for `--top-bar-height-logical`. | TASK-005 | Test fails if CSS fallback regresses to `23.4px`. |
| TASK-010 | Update `master_spec.md` top-bar section to state: “Top bar defaults to 20.0 logical pixels high and clamps user resizing to 18-120 logical pixels.” | TASK-003 | Master spec accurately reflects new behavior. |
| TASK-011 | Update `changelog.md` according to `CHANGELOG_POLICY.md` with one concise entry describing compact top-bar default and CSS control-size reduction. | TASK-010 | Changelog records user-visible top-bar compactness change. |

### Implementation Phase 4

- **GOAL-004**: Validate compact top-bar behavior.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-012 | Run `pnpm build`. | TASK-003, TASK-004, TASK-005, TASK-006, TASK-007, TASK-008, TASK-009 | Build exits with code `0`. |
| TASK-013 | Run focused test command `node --test tests/shellBarResize.test.mjs` after build output exists. | TASK-012 | Focused shell-bar tests exit with code `0`. |
| TASK-014 | Manually smoke check the top bar in app runtime: verify the top header appears shorter, no controls are clipped, search remains usable, pin buttons remain clickable, popup buttons still open panels, and unlocked resize still works from `18` to `120` logical pixels. | TASK-012 | Manual check confirms compact bar has no visual clipping or broken controls. |

## 3. Alternatives

- **ALT-001**: Only reduce CSS padding and control heights without changing `topBarHeightLogical`. Rejected because the native AppBar reservation would remain tall.
- **ALT-002**: Lower the top-bar minimum clamp below `18`. Rejected because this risks unusable controls and breaks existing resize expectations.
- **ALT-003**: Force all existing users’ persisted top-bar height to `20.0`. Rejected because it would overwrite user settings unexpectedly.
- **ALT-004**: Change both top and bottom bars for consistent density. Rejected because the task only requests the top header bar.

## 4. Dependencies

- **DEP-001**: Svelte top-bar component: `src/components/TopBar.svelte`.
- **DEP-002**: Top-bar styling: `src/components/TopBar.css`.
- **DEP-003**: Settings defaults: `src/lib/settings.ts`.
- **DEP-004**: Shell bar resize helpers: `src/lib/shellBarResize.ts`.
- **DEP-005**: Existing build command: `pnpm build`.

## 5. Files

- **FILE-001**: `src/components/TopBar.svelte` — top-bar runtime default height initialization.
- **FILE-002**: `src/components/TopBar.css` — compact top-bar control heights, padding, fonts, and fallback height.
- **FILE-003**: `src/lib/settings.ts` — persisted settings default for `ui.topBarHeightLogical`.
- **FILE-004**: `src/lib/shellBarResize.ts` — inspect only unless discovery finds default coupling.
- **FILE-005**: `tests/shellBarResize.test.mjs` — focused tests for default top-bar height and CSS fallback.
- **FILE-006**: `master_spec.md` — durable top-bar behavior specification.
- **FILE-007**: `changelog.md` — per-change history entry.

## 6. Testing

- **TEST-001**: Update `tests/shellBarResize.test.mjs` default settings assertion to expect `topBarHeightLogical: 20.0`.
- **TEST-002**: Add/update source assertion verifying `src/components/TopBar.css` uses `var(--top-bar-height-logical, 20px)`.
- **TEST-003**: Run `pnpm build`.
- **TEST-004**: Run `node --test tests/shellBarResize.test.mjs`.
- **TEST-005**: Manual runtime smoke check for no clipping and preserved resize behavior.

## 7. Risks & Assumptions

- **RISK-001**: Existing persisted user settings may keep the top bar at the old height. Mitigation: preserve user settings by design; document that the new height applies to defaults and new/reset settings.
- **RISK-002**: Reducing control heights may clip text or icons. Mitigation: adjust CSS font sizes and padding, then perform manual smoke check.
- **RISK-003**: Native AppBar reservation may depend on backend constants not identified initially. Mitigation: complete TASK-002 before implementation.
- **ASSUMPTION-001**: The “top header bar” means the JasonShell top bar described in `master_spec.md`. Verify by inspecting `src/components/TopBar.svelte`.
- **ASSUMPTION-002**: `20.0` logical pixels is the deterministic compact target because it is lower than current `23.4` while remaining above the existing `18` minimum.

## 8. Related Specifications / Further Reading

- **REF-001**: `master_spec.md` — canonical top-bar behavior and shell surface specification.
- **REF-002**: `CHANGELOG_POLICY.md` — changelog update rules.
- **REF-003**: `src/components/TopBar.svelte` — top-bar component behavior.
- **REF-004**: `src/components/TopBar.css` — top-bar visual density.
- **REF-005**: `tests/shellBarResize.test.mjs` — existing shell-bar height and resize coverage.

---
The plan file is located at c:\dev\jasonshell\.harness\tasks\task_3b1ab7ec-abda-45be-8fe1-5c08618bd267.md