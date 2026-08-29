# 10 Process Manager Live-Region and Filter Focus Accessibility

## Metadata

- Status: Ready for implementation
- Owner: validation owner
- Priority: P1
- Audit refs: P1-5, P1-6
- Order: 10 of 13
- Dependencies: Plan 05 MUST be complete to avoid Process Manager component conflicts.
- Allowed implementation scope: `src/components/ProcessManagerSurface.svelte`, `src/components/ProcessManagerSurface.css`, focused Process Manager tests, `docs/smoke-test-windows.md`, `master_spec.md`, `changelog.md`

## Objective and evidence

Fix Process Manager accessibility defects without changing process data contracts.

Evidence:

- Audit P1-5: `ProcessManagerSurface.css:70-78` removes filter input outline with no replacement.
- Audit P1-6: `ProcessManagerSurface.svelte:87-147,287,311` refreshes every second and puts `aria-live="polite"` on whole grid.
- Current spec says Process Manager auto-polls while open and uses guarded kill state.
- Current smoke checklist only says rows refresh and polling stops after close; it does not test live-region noise or visible focus.

## Scope

In scope:

1. Remove live-region semantics from auto-refreshing grid.
2. Keep one small status live region for manual refresh, errors, open focus, and kill messages.
3. Add visible `:focus-visible` styling for filter input and grid/action controls if absent.
4. Pause/avoid refresh announcements for background timer updates.
5. Add focused tests for markup/CSS/source contracts and pure state if helper extraction is needed.
6. Add manual Windows/NVDA/JAWS validation checklist, marked manual evidence only.

Out of scope:

- Process kill PID-reuse fix.
- Process grouping/sorting redesign.
- Backend process enumeration changes.
- Claims that screen readers are fixed without manual AT evidence.
- Persistent top JSON/shell terminal and Stack Browser Git workbench feature work.

## Current contract

- `process-manager` webview opens from bottom bar or task menu.
- `show_process_manager` can send optional `focusPid`; surface filters to PID and sorts by PID.
- Process list refreshes every second while open, stops on close/focus loss.
- Header status uses `role="status" aria-live="polite"`.
- Grid currently has `role="grid" tabindex="0" aria-label="Running processes" aria-live={isOpen ? 'polite' : 'off'}`.
- Filter input currently has `outline: 0`.
- IPC/types/events remain unchanged.

## Functional requirements

FR-1. Implementation MUST remove `aria-live` from `.process-table-scroll`/process grid.

FR-2. Implementation MUST keep status updates in a dedicated `role="status"` live region.

FR-3. Automatic timer refresh MUST NOT update status text on every successful poll unless user-meaningful state changed and announcement is intentional.

FR-4. Manual Refresh MUST update the status live region with a result or error.

FR-5. Open with `focusPid` MUST keep the focused PID status announcement.

FR-6. Kill arm, kill start, kill success, kill failure, and kill cancel MUST keep status announcements.

FR-7. Filter input MUST have a visible `:focus-visible` indicator that is not color-only if feasible; acceptable: outline/box-shadow plus border/background change.

FR-8. Grid container focus MUST have visible focus if `tabindex="0"` remains.

FR-9. Sort/header/group/kill buttons MUST retain visible focus using shared button focus styling if not already covered by Melt wrapper.

FR-10. Close/hidden state MUST stop timer and reject stale in-flight refresh results as today.

## Non-functional requirements

NFR-1. This plan MUST NOT introduce IPC command/event changes or additional `ProcessInfo`/`ProcessKillConfirmation` shape changes beyond identity fields established by dependency Plan 05.

NFR-2. Automatic refresh must remain at current 1000 ms cadence unless explicitly justified in `master_spec.md`.

NFR-3. CSS must use existing design tokens where possible.

NFR-4. Node tests must avoid brittle exact pixel/color assertions unless guarding an intentional token-level contract.

NFR-5. Manual AT claims require recorded human evidence; automated tests may only prove DOM/CSS contracts.

## Implementation decisions

- Split refresh cause: add parameter such as `{ preserveVolatileOrder?: boolean; announce?: boolean; reason?: 'auto' | 'manual' | 'open' | 'kill' }` or a small helper. Auto timer uses `announce: false`; manual/open/kill use `announce: true` where appropriate.
- Keep `statusMessage` as live-region text; optionally add non-live `lastSnapshotSummary` for visual process count if product wants live visible count without announcements.
- Prefer CSS on wrapper: `.process-filter:focus-within` and `.process-filter input:focus-visible`.
- If grid container remains focusable only for scroll, add `.process-table-scroll:focus-visible`.
- Do not add `aria-live="off"` to grid; absence is clearer and avoids future confusion.

## Phased RED-first implementation

### Phase 1 - RED tests

Add/update `tests/processManagerAccessibility.test.mjs`.

Expected failing tests before implementation:

1. `process manager keeps auto-refresh grid silent and uses dedicated status live region`
   - Assert source does not include `aria-live={isOpen ? 'polite' : 'off'}` on grid.
   - Assert grid markup has `role="grid"` and no `aria-live` in same opening tag.
   - Assert status span retains `role="status"` and `aria-live="polite"`.

2. `process manager auto refresh does not announce every successful timer tick`
   - Assert timer calls refresh with `announce: false` or equivalent named reason.
   - Assert manual Refresh calls refresh with `announce: true` or `reason: 'manual'`.

3. `process manager filter and grid expose visible focus styles`
   - Assert CSS has `.process-filter:focus-within` or `.process-filter input:focus-visible` with `outline` or `box-shadow` not `none`.
   - Assert CSS has `.process-table-scroll:focus-visible` if grid remains `tabindex="0"`.

### Phase 2 - GREEN implementation

1. Remove grid `aria-live`.
2. Add refresh announce/reason param.
3. Timer refresh calls silent path.
4. Manual Refresh/open/focusPid/kill paths keep announcements.
5. Add focus-visible CSS.
6. Update `master_spec.md` Process Manager section to say grid refresh is silent and status live region is announcement owner.
7. Update `docs/smoke-test-windows.md` Process Manager section with keyboard focus and manual AT checks.
8. Add `changelog.md` entry per `CHANGELOG_POLICY.md`.

### Phase 3 - Refactor

- Extract helper only if testability improves; no broad Svelte rewrite.
- Replace regex assertions with helper tests only if helper exists.

## Exact test names and assertions

- `process manager keeps auto-refresh grid silent and uses dedicated status live region`
  - `assert.doesNotMatch(gridOpenTag, /aria-live/)`
  - `assert.match(processSurface, /role="status"\s+aria-live="polite"/)`
- `process manager auto refresh does not announce every successful timer tick`
  - `assert.match(timerBody, /announce:\s*false|reason:\s*['"]auto['"]/)`
  - `assert.match(refreshButtonSnippet, /announce:\s*true|reason:\s*['"]manual['"]/)`
- `process manager filter and grid expose visible focus styles`
  - `assert.match(css, /\.process-filter(?::focus-within| input:focus-visible)[\s\S]*(outline|box-shadow)/)`
  - `assert.doesNotMatch(focusRule, /outline:\s*none|box-shadow:\s*none/)`

## Manual/live validation

Safety/consent:

- Starting JasonShell changes AppBar/work area and Explorer taskbar visibility. Human MUST consent before `npm run tauri dev`.
- Do not terminate processes during accessibility validation except a disposable test process with explicit consent.
- Do not claim NVDA/JAWS behavior from automation alone.

Manual checks:

1. Keyboard open Process Manager from bottom bar.
2. Tab to Filter at 100%, 200%, light/dark themes; visible focus clear.
3. Tab to grid; visible focus clear if focusable.
4. Let panel idle 30 seconds with active system; screen reader does not continuously announce row changes.
5. Press Refresh; one bounded status announcement occurs.
6. Close panel; confirm polling stops by log or observable no updates.

## Edge cases

- Empty process list still announces empty/unavailable state.
- Slow refresh result after close must not update state.
- Manual refresh while auto refresh in flight keeps existing `isLoading` guard semantics.
- Filtering to PID must remain announced.
- Status must not spam when only CPU/memory values change.

## API/type/event compatibility

- No changes to Tauri commands: `show_process_manager`, `hide_process_manager`, `list_processes`, `kill_process`.
- No changes to events: `process-manager:open`, `process-manager:closed`.
- No changes to TS/Rust structs.
- CSS class additions allowed; existing class names must remain.

## Validation commands

Focused:

```powershell
node scripts/clean-dist-tests.mjs
tsc -p tsconfig.test.json
node --test tests/processManagerAccessibility.test.mjs
npm run check
```

Project gates:

```powershell
npm run test:node
npm run cargo:test
npm run cargo:check
npm run build
npm run validate
```

## Acceptance criteria

- Given Process Manager auto-refreshes, When rows/metrics change for 30 seconds, Then grid itself is not a live region and repeated row changes are not announced by grid semantics.
- Given user presses Refresh, When refresh completes or fails, Then status live region announces one bounded result/error.
- Given keyboard user tabs to Filter, When focus lands, Then visible focus indicator is present at 100% and 200% zoom.
- Given Process Manager closes, When an in-flight refresh resolves, Then stale result does not mutate closed UI or restart timer.
- Given focused PID open payload arrives, When panel opens, Then PID filter and status behavior remain unchanged.

## Risks and rollback

- Risk: removing grid live region may reduce useful updates for some users. Mitigation: keep manual refresh status announcements.
- Risk: focus style clashes with compact UI. Mitigation: token-based outline/box-shadow and manual visual check.
- Rollback: revert Svelte/CSS/test/doc changes; no persistence/API migration needed.

## Docs updates

- `master_spec.md`: Process Manager accessibility contract.
- `docs/smoke-test-windows.md`: Process Manager keyboard/AT smoke.
- `changelog.md`: concise `[CODE]`/`[TOOL]` entries if implementation changes durable behavior/tests/docs.

## Handoff checklist

- [ ] Read audit, spec, changelog policy, package scripts.
- [ ] Preserve unrelated dirty work.
- [ ] Add RED tests first and confirm fail.
- [ ] Implement minimal Svelte/CSS changes.
- [ ] Update durable docs/changelog.
- [ ] Run focused validation.
- [ ] Run full validation or record exact pre-existing failures.
- [ ] Complete manual AT/live checks only with human consent.
- [ ] Run adversarial QA.

## Copy/Paste Implementation Prompt

```text
You are implementing Plan 10 in C:\dev\jasonshell. Read docs/current-state-technical-audit-2026-08-28.md, master_spec.md, CHANGELOG_POLICY.md, package.json scripts, and docs/remediation-plans/10-process-manager-accessibility.md first. Preserve unrelated dirty work. Do not implement unrelated remediation items, persistent top JSON/shell terminal work, or Stack Browser Git workbench feature work.

Goal: fix Process Manager P1-5/P1-6 accessibility by removing live-region behavior from the auto-refreshing grid, keeping a small status live region for intentional announcements, and adding visible keyboard focus for filter/grid/actions.

Use RED-first: add failing focused tests with exact names from plan, prove they fail, then implement. Avoid brittle visual literals except intentional token contracts. Do not change IPC command names, event names, Rust/TS process types, polling cadence, grouping, sorting, or kill semantics.

After code: update master_spec.md current Process Manager contract, docs/smoke-test-windows.md manual checks, and changelog.md per CHANGELOG_POLICY.md. Run focused validation, then full validation: npm run check, npm run build, npm run test:node, npm run cargo:test, npm run cargo:check, npm run validate. Record exact failures if pre-existing gates remain red. Manual live validation requires explicit human consent before npm run tauri dev because AppBar/work-area/taskbar state changes; do not claim NVDA/JAWS behavior without manual evidence. Run adversarial QA before declaring done.
```
