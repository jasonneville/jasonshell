# 09 Control Plane Semantic Tabs and Visible Filter Focus

## Metadata
- Status: Ready for implementation.
- Order: 09 of 13.
- Audit findings: P1-4 plus relevant P1-5.
- Owner: implementation agent.
- Dependencies: Plans 01-02 MUST be complete.
- Exclusions: no persistent top JSON/shell terminal work; no Stack Browser Git workbench feature work.

## Objective and Evidence
Resolve Control Plane tab semantic mismatch and restore visible focus for filter input.

Evidence:
- Audit P1-4 cites `src/components/ControlPlaneSurface.svelte:130-154`: Melt tabs triggers/content are used but every panel is forced `hidden={false}`.
- Audit P1-5 cites `src/components/ControlPlaneSurface.css:103-112`: filter input removes outline without replacement.
- `master_spec.md` says Control Plane section navigation uses Melt Tabs and remains raw builder trigger buttons; action buttons use `MeltActionButton`.

## Scope
In scope exact files/symbols:
- `src/components/ControlPlaneSurface.svelte`: `sectionTabs`, trigger list, content rendering, `hidden={false}`, `visibleSections`, `activeSectionId`.
- `src/components/ControlPlaneSurface.css`: focus-visible treatment for filter input and select/theme trigger if needed; active/inactive panel styles if true tabs chosen.
- `tests/controlPlaneRouting.test.mjs`, `tests/controlPlaneState.test.mjs`, or new `tests/controlPlaneAccessibility.test.mjs` for source/semantic assertions.
- Docs: `master_spec.md`, `changelog.md`.

Out of scope:
- New Control Plane data sources, settings persistence, secrets exposure, provider fetching, process actions.
- Process Manager focus fixes except if shared CSS token note is needed.
- Quick Launch focus policy.
- Persistent terminal or Stack Browser Git features.

## Current Contract
- Control Plane is authority-light renderer dashboard over existing contracts.
- Sections are filterable.
- Melt Tabs builder provides trigger/content ARIA semantics.
- Current visual layout shows all section cards despite tab state; active card only highlighted.

## Requirements
### Functional Requirements
- FR-1: Control Plane MUST choose one accessible model: real tabs with only active panel visible, or non-tab filter/navigation controls with all matching sections visible.
- FR-2: Because current spec names Melt Tabs, implementation SHOULD use real tabs unless product owner chooses all-sections dashboard semantics.
- FR-3: If real tabs are used, inactive tab panels MUST be hidden from visual layout and accessibility tree by builder/default `hidden` behavior.
- FR-4: If all sections remain visible, tab roles MUST be removed and controls renamed as filters/navigation, not tabs.
- FR-5: Filter input MUST have clear `:focus-visible` styling that meets visible focus expectations without relying only on removed outline.
- FR-6: Filtering MUST keep active section valid and deterministic.
- FR-7: Raw builder trigger buttons MUST remain real `<button type="button">` controls.

### Non-Functional Requirements
- NFR-1: Keyboard users MUST be able to see focus at 100% and 200% zoom.
- NFR-2: Screen-reader semantics MUST not expose selected tab plus unrelated visible panels.
- NFR-3: No new dependency or persistent state.
- NFR-4: Existing dashboard authority-light/no-secrets constraints remain.

## Implementation Decisions
- Recommended: real tabs. Remove `hidden={false}` and render active tab panel according to Melt content attributes. If layout requires cards, show only active section card; counts remain in tab triggers.
- Add focus ring on label/container using `:focus-within` and input using `:focus-visible`; e.g. border/box-shadow token consistent with theme.
- Keep filter hint accurate: arrow keys apply to tabs; text filter narrows tab list.
- When filter removes active tab, existing reactive assignment to first visible section remains.

## Phased RED-First Implementation
1. RED semantic tests:
   - Add Node/source test that fails while `hidden={false}` exists on Control Plane tab content.
   - Assert either real tab content hides inactive panels or no tab roles are used; choose real tabs in test.
2. RED focus tests:
   - Add CSS/source test requiring `.control-plane-toolbar input:focus-visible` or container focus-within replacement with visible box-shadow/border.
3. GREEN implementation:
   - Remove forced `hidden={false}`.
   - Ensure only active/selected panel is visible and labelled/described correctly.
   - Add focus-visible CSS.
4. Refactor:
   - Keep markup simple; avoid new wrapper components.
5. Docs:
   - Update `master_spec.md` Control Plane line with real tab semantics and filter focus.
   - Append changelog.

## Exact Tests and Assertions
- `tests/controlPlaneAccessibility.test.mjs`
  - `Control Plane tab panels are not forced visible`: source does not contain `hidden={false}` in tab content block.
  - `Control Plane uses real tab semantics`: source contains `Tabs<ControlPlaneSectionId>` and `getContent(section.id)` with builder hidden behavior, or test adjusted if non-tab model approved.
  - `Control Plane filter has visible focus style`: CSS contains `.control-plane-toolbar input:focus-visible` or `.control-plane-toolbar label:focus-within` with nonzero outline/border/box-shadow.
- Existing `tests/controlPlaneState.test.mjs`
  - filter results still deterministic; active section fallback remains.

## Edge Cases
- Filter returns zero sections: tablist empty; no panel visible; no stale active panel exposed.
- Filter removes active section: first visible section selected.
- Keyboard focus on active tab then filter changes list.
- High-contrast/theme colors with focus ring.
- 200% zoom/narrow width horizontal tab overflow.

## API, Type, Event Compatibility
- No Tauri command changes.
- No IPC/event changes.
- No exported TS type changes expected.
- No settings persistence or schema changes.
- DOM/accessibility semantics intentionally change for correctness.

## Validation
Focused:
- `npm run check`
- `npm run test:node`

Full:
- `npm run validate`

Manual/accessibility:
- Keyboard through filter, tablist, active panel at 100% and 200% zoom.
- Inspect accessibility tree: only active tabpanel exposed for real-tabs model.
- Verify no secrets/unbounded source lists appear.

## Acceptance Criteria
- AC-1 (FR-1,FR-3,NFR-2): Given Control Plane tabs, when Settings tab selected, then only Settings tabpanel is visible/exposed; other panels are hidden.
- AC-2 (FR-6): Given filter narrows sections excluding current active tab, when visible sections recompute, then active section becomes first visible result.
- AC-3 (FR-5,NFR-1): Given keyboard focus enters filter input, when `:focus-visible` applies, then visible ring/border/shadow appears.
- AC-4 (FR-7): Given tab triggers, when rendered, then they remain real `button type="button"` controls.

## Risks and Rollback
- Risk: product intended dashboard to show all panels. Mitigation: plan defines alternative non-tab model; if chosen, remove tab roles instead.
- Risk: hiding panels reduces at-a-glance info. Mitigate with counts/status in tab triggers.
- Risk: focus style conflicts with theme. Mitigate with theme tokens and manual theme check.
- Rollback: reapply all-sections layout only with roles changed to non-tab semantics; no data migration.

## Master Spec and Changelog Updates
- `master_spec.md`: update Control Plane statement to real tab semantics plus filter visible focus, or non-tab dashboard model if approved.
- `changelog.md`: append `[CODE]` validation entry.

## Handoff Checklist
- [ ] Read `master_spec.md`, audit, plan 09.
- [ ] Preserve dirty worktree.
- [ ] RED tests first.
- [ ] Choose real-tabs model unless explicit approval says all-sections non-tab model.
- [ ] Validate with check/test.
- [ ] Update docs.
- [ ] Adversarial/accessibility QA.
- [ ] Do not touch excluded terminal/Git workbench features.

## Copy/Paste Implementation Prompt

```text
Implement remediation plan 09 in C:\dev\jasonshell. First read master_spec.md, docs/current-state-technical-audit-2026-08-28.md, CHANGELOG_POLICY.md, package.json scripts, and docs/remediation-plans/09-control-plane-accessibility.md. Preserve dirty worktree. Use TDD: add RED tests for Control Plane tab semantics and visible filter focus before code changes. Scope to ControlPlaneSurface.svelte, ControlPlaneSurface.css, focused tests, master_spec.md, changelog.md. Do not implement persistent top JSON/shell terminal or Stack Browser Git workbench feature work. Do not add new data sources, settings schema, or process/provider behavior. Prefer real Melt Tabs: inactive panels hidden, active tab/panel semantics aligned. If keeping all sections visible, remove tab roles instead and update tests/spec accordingly, but do not mix both models. Add clear focus-visible styling for filter input. Validate with npm run check, npm run test:node, and npm run validate if feasible. Update docs per policy. Run adversarial accessibility QA before final response.
```
