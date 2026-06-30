# Make Task Preview Close Button Rectangular

## 1. Requirements & Constraints

- **REQ-001**: Change the Task Preview close button from a circular/pill-shaped red `×` button to a rectangular close button.
- **REQ-002**: Preserve current close behavior: prevent preview activation, close the previewed task window, refresh taskbar windows, and hide the preview.
- **REQ-003**: Preserve accessibility: the close button must keep `ariaLabel="Close previewed window"` and remain keyboard/focus reachable through `MeltActionButton`.
- **REQ-004**: Preserve hover-retention behavior owned by `.preview-interaction-root`.
- **CON-001**: Do not replace `MeltActionButton` with a raw `<button>`.
- **CON-002**: Do not modify unrelated task preview backend, DWM thumbnail, taskbar, or Tauri IPC behavior.
- **CON-003**: Follow repository validation command hint: `pnpm build`.
- **PAT-001**: Existing Task Preview source files are `src/components/TaskPreviewSurface.svelte` and `src/components/TaskPreviewSurface.css`.
- **PAT-002**: Existing source-level tests read component/CSS text from `tests/taskPreviewRetention.test.mjs`, `tests/taskPreviewTextPolish.test.mjs`, and `tests/taskbarPreviewContract.test.mjs`.

## 2. Implementation Steps

### Implementation Phase 1

- **GOAL-001**: Add test coverage that defines the rectangular close-button shape contract before implementation.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-001 | Update `tests/taskPreviewRetention.test.mjs` test `preview close button is accessible red X and does not activate preview` to assert `.preview-close-button` is not circular. Add regex assertions requiring `border-radius` to be a rectangular value such as `0`, `2px`, `3px`, `4px`, `var(--js-radius-xs)`, or `var(--js-radius-sm)`, and requiring width greater than height or explicit horizontal padding. | None | Test fails against current CSS because `.preview-close-button` currently uses `border-radius: 999px`, equal `width`, and equal `height`. |
| TASK-002 | Update `tests/taskPreviewTextPolish.test.mjs` test `preview close button stays out of text flow with reserved header space` to assert the button remains `position: absolute`, `z-index: 4`, and has rectangular dimensions/padding. | None | Test documents shape while preserving existing layout expectations. |
| TASK-003 | Run focused RED validation command: `pnpm test:node -- tests/taskPreviewRetention.test.mjs tests/taskPreviewTextPolish.test.mjs` if supported by scripts; if not supported, run `node scripts/clean-dist-tests.mjs && npx tsc -p tsconfig.test.json && node --test tests/taskPreviewRetention.test.mjs tests/taskPreviewTextPolish.test.mjs`. | TASK-001, TASK-002 | Validation fails only on the new rectangular close-button assertions. |

### Implementation Phase 2

- **GOAL-002**: Implement the rectangular close-button CSS without changing behavior.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-004 | Modify `.preview-close-button` in `src/components/TaskPreviewSurface.css`: replace `border-radius: 999px;` with a rectangular radius, preferably `border-radius: var(--js-radius-xs);` if defined globally, otherwise `border-radius: 3px;`. | TASK-003 | CSS no longer contains `border-radius: 999px` in `.preview-close-button`. |
| TASK-005 | Modify `.preview-close-button` dimensions in `src/components/TaskPreviewSurface.css`: replace equal square sizing with rectangular sizing. Recommended values: `height: 1.35rem;`, `min-width: 2.1rem;`, `padding: 0 0.42rem;`, and remove `width: 1.35rem;`. | TASK-004 | Button has a horizontal rectangular footprint while retaining absolute placement at `top: 0.42rem` and `right: 0.42rem`. |
| TASK-006 | Confirm `src/components/TaskPreviewSurface.svelte` remains unchanged unless needed for test stability. Do not change `handlePreviewClose`, `ariaLabel`, `class="preview-close-button"`, or button text `×`. | TASK-004 | Close behavior and accessibility source assertions still pass. |

### Implementation Phase 3

- **GOAL-003**: Update durable documentation only if required by repository policy.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-007 | Inspect the Task Preview section in `master_spec.md`. If it describes close-button shape, update that sentence to state the close button is rectangular. If it does not describe shape, do not add a new durable spec entry for this cosmetic-only change. | TASK-006 | `master_spec.md` is either unchanged with a clear reason or updated only in the relevant Task Preview section. |
| TASK-008 | Append a concise entry to `changelog.md` under `## Change Ledger` only if tests, durable docs, or visual behavior are changed. Use `CHANGELOG_POLICY.md` format with `[CODE]` and `[TOOL]` bullets. | TASK-007 | Changelog entry is concise, dated, and contains no secrets or raw logs. |

### Implementation Phase 4

- **GOAL-004**: Validate the change with focused and required project checks.

| Task | Description | Dependencies | Acceptance Criteria |
| ---- | ----------- | ------------ | ------------------- |
| TASK-009 | Run focused tests: `node scripts/clean-dist-tests.mjs && npx tsc -p tsconfig.test.json && node --test tests/taskPreviewRetention.test.mjs tests/taskPreviewTextPolish.test.mjs tests/taskbarPreviewContract.test.mjs tests/meltMigrationWiring.test.mjs`. | TASK-008 | All focused tests pass. |
| TASK-010 | Run required build command: `pnpm build`. | TASK-009 | Build exits with code 0. |
| TASK-011 | Optional manual smoke check in the running app: open a task preview, verify close button is rectangular, verify clicking `×` closes the previewed window instead of activating the preview, and verify pointer hover retention still works. | TASK-010 | Visual shape and close behavior match requirements. |

## 3. Alternatives

- **ALT-001**: Change only `border-radius` and keep equal width/height. Rejected because a square button with slightly rounded corners may not satisfy “rectangular” if the expected outcome is visibly wider than tall.
- **ALT-002**: Replace the `×` button with a text button labeled `Close`. Rejected because it changes compact preview chrome and may affect reserved header spacing.
- **ALT-003**: Use a raw native `<button>`. Rejected because current repository guidance uses `MeltActionButton` for safe task-preview action buttons.

## 4. Dependencies

- **DEP-001**: Svelte component `src/components/TaskPreviewSurface.svelte`.
- **DEP-002**: CSS module `src/components/TaskPreviewSurface.css`.
- **DEP-003**: Local `MeltActionButton` wrapper at `src/components/melt/MeltActionButton.svelte`.
- **DEP-004**: Node source-level test runner used by `tests/*.test.mjs`.
- **DEP-005**: Project package manager command `pnpm build`.

## 5. Files

- **FILE-001**: `src/components/TaskPreviewSurface.css` — primary CSS change for rectangular close-button shape.
- **FILE-002**: `src/components/TaskPreviewSurface.svelte` — inspect only; preserve close-button markup and handlers.
- **FILE-003**: `tests/taskPreviewRetention.test.mjs` — update shape contract assertions.
- **FILE-004**: `tests/taskPreviewTextPolish.test.mjs` — update layout/shape assertions.
- **FILE-005**: `tests/taskbarPreviewContract.test.mjs` — run to ensure native preview contracts remain intact.
- **FILE-006**: `tests/meltMigrationWiring.test.mjs` — run to ensure `MeltActionButton` usage remains intact.
- **FILE-007**: `master_spec.md` — inspect and update only if it currently specifies close-button shape.
- **FILE-008**: `changelog.md` — append concise change/validation notes if repository policy requires it.

## 6. Testing

- **TEST-001**: Add/modify source test asserting `.preview-close-button` does not use `border-radius: 999px`.
- **TEST-002**: Add/modify source test asserting `.preview-close-button` has rectangular sizing via `min-width`/`padding` or width greater than height.
- **TEST-003**: Run `node scripts/clean-dist-tests.mjs && npx tsc -p tsconfig.test.json && node --test tests/taskPreviewRetention.test.mjs tests/taskPreviewTextPolish.test.mjs tests/taskbarPreviewContract.test.mjs tests/meltMigrationWiring.test.mjs`.
- **TEST-004**: Run `pnpm build`.
- **TEST-005**: Manual smoke check: close button is rectangular and clicking it closes the previewed task window without activating/maximizing it.

## 7. Risks & Assumptions

- **RISK-001**: Reducing circular radius could conflict with theme variables if `--js-radius-xs` is undefined. Mitigation: verify global CSS variables or use literal `3px`.
- **RISK-002**: Increasing button width could overlap long preview title text. Mitigation: keep `.preview-header padding-right: 2.1rem` or increase it only if the final button width exceeds reserved space.
- **RISK-003**: Test command filtering may not be supported by package scripts. Mitigation: use direct `node --test` command listed in TASK-009.
- **ASSUMPTION-001**: “Rectangular” means visibly wider than tall with small corner radius, not a pill/circle. Verify by checking final CSS has `min-width` or horizontal padding and no `border-radius: 999px`.
- **ASSUMPTION-002**: No backend changes are required because the request is visual-only.

## 8. Related Specifications / Further Reading

- **REF-001**: `master_spec.md` — Task Preview runtime boundary and UI primitive baseline.
- **REF-002**: `CHANGELOG_POLICY.md` — changelog update rules.
- **REF-003**: `src/components/TaskPreviewSurface.svelte` — Task Preview component behavior.
- **REF-004**: `src/components/TaskPreviewSurface.css` — Task Preview visual styling.
- **REF-005**: `tests/taskPreviewRetention.test.mjs` — close-button behavior/source contract.
- **REF-006**: `tests/taskPreviewTextPolish.test.mjs` — close-button layout/source contract.
- **REF-007**: `tests/meltMigrationWiring.test.mjs` — Melt action button usage contract.