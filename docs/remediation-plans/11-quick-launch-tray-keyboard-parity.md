# 11 Quick Launch and Tray Keyboard Parity and Visible Focus

## Metadata

- Status: Ready for implementation
- Owner: validation owner
- Priority: P1
- Audit refs: P1-7 and Quick Launch P1-5
- Order: 11 of 13
- Dependencies: Plan 06 MUST be complete because both touch Tray Panel. Plan 10 provides optional focus-style precedent.
- Allowed implementation scope: `QuickLaunchPanelSurface.svelte`, `TrayPanelSurface.svelte`, related CSS/lib/tests/Rust only where keyboard/context-menu/close/focus parity requires it, durable docs

## Objective and evidence

Provide keyboard parity for Quick Launch admin action and tray secondary actions, plus visible focus for Quick Launch rows.

Evidence:

- Audit P1-7: Quick Launch exposes `Run as administrator` only via right-click; tray right action lacks Menu-key equivalent; tray lacks in-panel Escape and visible close.
- Audit P1-5: Quick Launch intentionally suppresses focus ring and relies on subtle background.
- `master_spec.md` currently says Quick Launch selected row uses background-only styling and native/browser focus outline is suppressed.
- `tests/quickLaunchReliability.test.mjs` currently asserts background-only focus. This test must be revised if behavior changes.

## Scope

In scope:

1. Quick Launch: support Context Menu key and Shift+F10 for selected/focused row.
2. Quick Launch: add visible focus indicator that meets project accessibility intent.
3. Quick Launch: revise current no-focus-ring spec/test contract if focus behavior changes.
4. Tray: support Context Menu key and Shift+F10 for focused tray icon, invoking right action.
5. Tray: support Escape close inside panel.
6. Tray: add visible close button with accessible label.
7. Tests and docs for keyboard flows.

Out of scope:

- Changing Quick Launch nonce/allowed-path authorization.
- Changing Run as administrator native menu payload security.
- Changing tray icon discovery/click backend semantics.
- Automating real native tray menu assertions without live evidence.
- Persistent top JSON/shell terminal and Stack Browser Git workbench feature work.

## Current contract

- Quick Launch panel opens with nonce and allowed rows; selection invokes `select_quick_launch_panel`.
- Quick Launch native context menu uses `show_quick_launch_panel_context_menu` from quick-launch panel only.
- Quick Launch right-click opens single `Run as administrator` action.
- Quick Launch Escape hides panel; Arrow/Home/End/Enter navigate/activate.
- Quick Launch focus-visible currently sets background selected and `outline: none`, `box-shadow: none`, transparent border.
- Tray icon buttons support click -> left action and contextmenu -> right action.
- Tray panel stays open on invoke failure and shows inline error.
- Tray panel closes on focus loss through native window handler, not local Escape.

## Functional requirements

FR-1. Quick Launch MUST open same native admin menu from keyboard Context Menu key.

FR-2. Quick Launch MUST open same native admin menu from Shift+F10.

FR-3. Quick Launch keyboard menu invocation MUST use current focused row and current nonce.

FR-4. Quick Launch keyboard menu invocation MUST not trigger primary launch.

FR-5. Quick Launch row focus MUST have visible focus indicator beyond hover-equivalent background.

FR-6. If Quick Launch focus ring/outline is added, `master_spec.md` MUST revise the current no-focus-ring contract and tests MUST stop asserting background-only focus.

FR-7. Tray icon focused button MUST invoke right action on Context Menu key.

FR-8. Tray icon focused button MUST invoke right action on Shift+F10.

FR-9. Tray panel MUST close on Escape from inside panel.

FR-10. Tray panel MUST expose visible close button with accessible label, calling existing hide command/wrapper.

FR-11. Existing mouse click/right-click behavior MUST remain.

FR-12. Error/loading/empty states MUST remain accessible.

## Non-functional requirements

NFR-1. No weakening of Quick Launch nonce/caller/allowed-path validation.

NFR-2. No new renderer-provided executable/admin target.

NFR-3. Keyboard support must not depend on browser-specific deprecated `keyCode`.

NFR-4. Focus styling must use existing tokens where possible and avoid low-contrast color-only state.

NFR-5. Manual UAC/admin validation requires human consent.

## Implementation decisions

- Reuse `openQuickLaunchNativeMenu` by widening event input or adding a keyboard helper that computes coordinates from `getBoundingClientRect()` of focused row.
- For keyboard menu coords, use row center or left-middle; native menu must be visible and anchored to row.
- Add `on:keydown` to Quick Launch row button or handle at panel level using `focusedIndex`.
- Tray can add `on:keydown` to icon buttons; Shift+F10 and `event.key === 'ContextMenu'` call `triggerTrayIcon(icon, 'right')`.
- Add `hideTrayPanel` import/wrapper if not present; visible close calls it.
- Use `MeltActionButton` only if it preserves native button semantics and does not disrupt surface simplicity; raw button acceptable with explicit type.

## Phased RED-first implementation

### Phase 1 - RED tests

Update/add tests before code.

Tests expected to fail:

1. In `tests/quickLaunchReliability.test.mjs`, replace `quick launch selected row and focus-visible stay background-only` with `quick launch selected row keeps visible focus indicator distinct from hover selection`.
2. Add `quick launch opens admin context menu from keyboard menu keys`.
3. Add `tray panel supports keyboard secondary action and Escape close`.
4. Add `tray panel exposes visible close control`.

### Phase 2 - GREEN implementation

1. Quick Launch key handler detects `ContextMenu` and Shift+F10.
2. It prevents default/stops propagation and opens native menu for focused row.
3. Quick Launch focus-visible CSS adds outline/box-shadow/border token.
4. Tray imports hide command/wrapper, adds Escape handler and close button.
5. Tray icon keydown maps menu keys to right action.
6. Update smoke docs for keyboard-only primary/secondary/dismiss flows.
7. Update `master_spec.md` Quick Launch current contract, especially no-focus-ring revision.
8. Update changelog.

### Phase 3 - QA/refactor

- Verify keyboard actions do not double-fire click/contextmenu.
- Verify focus-loss suppression for Quick Launch native menu still protects UAC/native menu flow.
- Keep tests source-contract until component harness exists.

## Exact test names and assertions

- `quick launch selected row keeps visible focus indicator distinct from hover selection`
  - Assert focus-visible rule does not contain `outline: none`, `box-shadow: none`, and `border-color: transparent` together.
  - Assert focus-visible has `outline` or `box-shadow` or non-transparent `border-color`.
  - Assert old test name is removed or rewritten.

- `quick launch opens admin context menu from keyboard menu keys`
  - Assert key handling includes `event.key === 'ContextMenu'`.
  - Assert key handling includes `event.shiftKey && event.key === 'F10'`.
  - Assert handler calls `showQuickLaunchPanelContextMenu`/keyboard helper with nonce and focused launcher shortcut.
  - Assert it calls `preventDefault()`.

- `tray panel supports keyboard secondary action and Escape close`
  - Assert tray source handles `ContextMenu` and Shift+F10.
  - Assert keyboard path calls `triggerTrayIcon(icon, 'right')`.
  - Assert Escape calls `hideTrayPanel()` or wrapper.

- `tray panel exposes visible close control`
  - Assert `aria-label="Close notification area icons"` or equivalent.
  - Assert close button has `type="button"`.
  - Assert click calls hide wrapper.

## Manual/live validation

Safety/consent:

- Starting JasonShell changes AppBar/work area/taskbar state; human MUST consent.
- Run as administrator may show UAC; human MUST consent before triggering admin flow.
- Tray icon right actions can open app menus or mutate app state; test on safe icons only with user-approved targets.

Manual checks:

1. Open Quick Launch by keyboard.
2. Arrow between rows; focus indicator visible at 100% and 200% zoom.
3. Press Enter; primary launch still works on safe target.
4. Press Context Menu; native Run as administrator menu opens for focused row.
5. Press Shift+F10; same menu opens.
6. Escape closes Quick Launch.
7. Open tray panel; Tab to icon; Enter/Space left action works where safe.
8. Context Menu and Shift+F10 invoke right action on safe icon.
9. Escape and visible close button close tray.

## Edge cases

- No Quick Launch rows: menu keys do nothing and do not throw.
- Focused index stale after rows change: clamp before menu open.
- Native menu/UAC focus loss: existing hold must prevent premature close.
- Tray invoke in flight: keyboard/mouse actions respect shared guard.
- Disabled tray buttons: keydown must not invoke.
- Close button present during loading/error/empty states.

## API/type/event compatibility

- No changes to `select_quick_launch_panel`, `show_quick_launch_panel_context_menu`, `hide_quick_launch_panel`, `hide_quick_launch_panel_on_focus_loss` contracts.
- No changes to tray icon id snapshot type.
- No changes to tray backend right/left action semantics.
- New CSS allowed; existing classes retained.

## Validation commands

Focused:

```powershell
node scripts/clean-dist-tests.mjs
tsc -p tsconfig.test.json
node --test tests/quickLaunchReliability.test.mjs tests/trayPanelWiring.test.mjs tests/trayPanelRetention.test.mjs
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

- Given Quick Launch row has keyboard focus, When user presses Context Menu, Then admin native menu opens for that row and primary launch does not run.
- Given Quick Launch row has keyboard focus, When user presses Shift+F10, Then same secondary action is reachable.
- Given Quick Launch row has focus, When viewed at 200% zoom, Then focus indicator is visibly distinct from hover/selection.
- Given tray icon has keyboard focus, When user presses Context Menu or Shift+F10, Then right action is invoked using same guard as mouse right-click.
- Given tray panel is open, When user presses Escape or activates close button, Then panel hides and top-bar state clears through existing close event path.

## Risks and rollback

- Risk: Quick Launch focus ring changes intended minimal visual design. Mitigation: spec/test update required and manual visual review.
- Risk: keyboard native menu coordinates poor. Mitigation: anchor to focused row rect center.
- Risk: tray right action on unsafe icon mutates state. Mitigation: manual validation uses safe icons only.
- Rollback: revert Svelte/CSS/tests/docs. No persistence migration.

## Docs updates

- `master_spec.md`: revise Quick Launch no-focus-ring statement if behavior changes; add keyboard secondary-action parity and tray close/Escape contract.
- `docs/smoke-test-windows.md`: add Quick Launch/tray keyboard-only paths.
- `changelog.md`: concise history entries.

## Handoff checklist

- [ ] Read audit/spec/changelog policy/package scripts/plan.
- [ ] Preserve unrelated dirty work.
- [ ] Add RED tests; confirm old no-ring assertion fails/removed.
- [ ] Implement keyboard parity/focus/close minimal changes.
- [ ] Update spec explicitly if focus contract changed.
- [ ] Run focused and full validation.
- [ ] Manual UAC/tray validation only with consent.
- [ ] Run adversarial QA.

## Copy/Paste Implementation Prompt

```text
You are implementing Plan 11 in C:\dev\jasonshell. Read docs/current-state-technical-audit-2026-08-28.md, master_spec.md, CHANGELOG_POLICY.md, package.json scripts, cited Quick Launch/tray source/tests, and docs/remediation-plans/11-quick-launch-tray-keyboard-parity.md first. Preserve unrelated dirty work. Do not implement unrelated findings, persistent top JSON/shell terminal work, or Stack Browser Git workbench feature work.

Goal: fix P1-7 and Quick Launch P1-5 by adding keyboard parity for Quick Launch Run as administrator and tray right actions, adding tray Escape/visible close, and making Quick Launch focus visibly distinct. If focus behavior changes, revise master_spec.md current no-focus-ring Quick Launch contract and update tests that assert background-only focus.

Use RED-first: write/update failing focused tests with exact plan names, confirm fail, then implement. Preserve Quick Launch nonce/caller/allowed-path validation, tray id/action contracts, mouse behavior, focus-loss/UAC holds, and existing IPC/event names. Do not add renderer-provided executable/admin paths.

After code: update master_spec.md, docs/smoke-test-windows.md, and changelog.md per CHANGELOG_POLICY.md. Run focused validation, then npm run check, npm run build, npm run test:node, npm run cargo:test, npm run cargo:check, npm run validate. Record exact pre-existing failures if any. Manual live validation requires explicit human consent before npm run tauri dev; Run as administrator/UAC and tray right actions require explicit consent and safe targets. Do not automate assistive-tech claims without manual evidence. Run adversarial QA before done.
```
