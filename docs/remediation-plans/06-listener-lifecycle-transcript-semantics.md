# 06 Command Panel + Tray Listener Lifecycle and Command Transcript Semantics

## Metadata
- Status: Ready for implementation.
- Order: 06 of 13.
- Audit findings: P1-3 and P2-1.
- Owner: implementation agent; validation owner: current planning request.
- Dependencies: Plans 01-02 MUST be complete. This pass MUST land before Plan 11 because both touch Tray Panel.
- Exclusions: no persistent top JSON/shell terminal work; no Stack Browser Git workbench feature work.

## Objective and Evidence
Fix two long-lived async listener lifecycle gaps and the Command Panel transcript semantic warning.

Evidence:
- `docs/current-state-technical-audit-2026-08-28.md` P1-3 cites `src/components/CommandPanelSurface.svelte:391-406` and `src/components/TrayPanelSurface.svelte:56-67`.
- `master_spec.md` persistent surface lifecycle contract says long-lived surfaces with async Tauri listeners must use synchronous `onMount` cleanup plus disposed guard; late resolved listeners must unlisten immediately.
- `CommandPanelSurface.svelte:451` attaches `keydown` and `contextmenu` listeners to noninteractive transcript section, producing Svelte a11y warning while `npm run check` still exits 0.

## Scope
In scope exact files/symbols:
- `src/components/CommandPanelSurface.svelte`: `onMount`, `listen(IPC_EVENTS.quickCommandRunUpdated, ...)`, `handleTranscriptKeydown`, `handleTranscriptContextMenu`, `.command-transcript-shell` markup.
- `src/components/TrayPanelSurface.svelte`: `onMount`, `listen(TRAY_PANEL_OPEN_EVENT, ...)`.
- `tests/persistentSurfaceLifecycle.test.mjs`: add Command Panel and Tray delayed-listen/disposed assertions.
- `tests/commandPanelWiring.test.mjs` or new focused `tests/commandPanelTranscriptSemantics.test.mjs`: transcript semantic/source assertions.
- Docs after behavior change: `master_spec.md`, `changelog.md` per `CHANGELOG_POLICY.md`.

Out of scope:
- Quick Command execution, decoding, history persistence, prompt/input protocols.
- Tray keyboard action parity and close control from P1-7.
- Persistent terminal or Stack Browser Git features.

## Current Contract
- Async listener registration returns a promise for unlisten.
- Destroy cleanup must be synchronous from Svelte `onMount` return.
- If listener promise resolves after component destroy, returned unlisten must be called exactly once immediately.
- Transcript remains selectable; Ctrl+C copies selected transcript text only when selection exists; context menu on selected text must not be suppressed by app-level handlers.

## Requirements
### Functional Requirements
- FR-1: Command Panel MUST guard async listener registration with `disposed` and unlisten late registrations exactly once.
- FR-2: Tray Panel MUST guard async listener registration with `disposed` and unlisten late registrations exactly once.
- FR-3: Registered listener callbacks MUST NOT mutate component state after disposal.
- FR-4: Command transcript MUST use valid interactive semantics for keyboard/contextmenu behavior, eliminating Svelte a11y warning.
- FR-5: Transcript text selection, Ctrl+C behavior, and selected-text context menu behavior MUST remain unchanged.
- FR-6: Existing IPC event names and payloads MUST remain unchanged.

### Non-Functional Requirements
- NFR-1: No listener leak under rapid mount/destroy or HMR.
- NFR-2: Cleanup MUST be idempotent and tolerate `listen()` rejection.
- NFR-3: `npm run check` MUST emit zero warnings for Command transcript after fix.
- NFR-4: No new runtime dependency.

## Implementation Decisions
- Prefer local helper pattern mirroring compliant surfaces or existing `StackPopupSurface.svelte`; do not introduce shared helper unless already present.
- For transcript semantics, prefer focusable region with explicit role only if interaction is genuinely keyboard-operable; otherwise move keyboard handling to a focusable inner element. Recommended: `role="region"`, `tabindex="0"`, accessible label, and keep `aria-label="Merged transcript"` if key handling remains on transcript container.
- Keep native text selection CSS and pointer behavior unchanged.

## Phased RED-First Implementation
1. RED lifecycle tests:
   - Extend `tests/persistentSurfaceLifecycle.test.mjs` to include `CommandPanelSurface.svelte` and `TrayPanelSurface.svelte`.
   - Assert source contains disposed guard and late `unlisten()` path for both.
   - Add negative assertion that cleanup does not only `then(unlisten)` after destroy without disposed branch.
2. RED transcript tests:
   - Add `tests/commandPanelTranscriptSemantics.test.mjs` or extend `tests/commandPanelWiring.test.mjs`.
   - Assert transcript host has `tabindex="0"` and semantic role/label, or equivalent valid interactive element.
   - Assert keydown/contextmenu handlers remain present on semantic/focusable host.
3. GREEN implementation:
   - Add `let disposed = false; const unlisteners: Array<() => void> = [];` inside each `onMount`.
   - On listener resolution: if disposed, call returned unlisten immediately; else push/store it.
   - Cleanup: set disposed true, remove DOM listeners/timers/frames, drain unlisteners exactly once.
   - In callbacks, early return if disposed before state mutation.
   - Update transcript markup semantics.
4. Refactor:
   - Remove duplication only if tiny helper stays local and obvious.
5. Docs:
   - Update `master_spec.md` lifecycle line to include Command Panel and Tray as covered.
   - Append concise `changelog.md` entry.

## Exact Tests and Assertions
- `tests/persistentSurfaceLifecycle.test.mjs`
  - `it('Command Panel disposes delayed quick-command listener resolution')`: expects disposed guard, late unlisten call path, callback guard.
  - `it('Tray Panel disposes delayed open listener resolution')`: expects same.
  - Existing surfaces still pass.
- `tests/commandPanelTranscriptSemantics.test.mjs`
  - `transcript host is keyboard-focusable when it owns keydown/contextmenu`.
  - `transcript host keeps merged transcript label`.
  - `transcript handlers remain wired for copy/context menu behavior`.

## Edge Cases
- `listen()` resolves after destroy.
- `listen()` rejects after destroy.
- Cleanup called after listener already unlistened.
- HMR remount creates one active listener, not two.
- No transcript selection: Ctrl+C falls through.
- Selected transcript text: context menu remains native/WebView menu.

## API, Type, Event Compatibility
- No IPC command changes.
- No event name changes: `quick-command:run-updated` via `IPC_EVENTS.quickCommandRunUpdated`, `TRAY_PANEL_OPEN_EVENT` unchanged.
- No payload type changes.
- No settings schema changes.

## Validation
Focused:
- `npm run check`
- `npm run test:node -- persistentSurfaceLifecycle` is not a package script; use official `npm run test:node` or direct only after rebuild if needed.
- `npm run test:node`

Full:
- `npm run validate`

Manual/browser:
- Open Command Panel, run command, select transcript text, Ctrl+C, right-click selection.
- Open/close Tray rapidly and verify no duplicate reloads or console errors.

## Acceptance Criteria
- AC-1 (FR-1,NFR-1): Given Command Panel is destroyed before `listen()` resolves, when promise resolves, then returned unlisten is called exactly once and callback cannot mutate destroyed state.
- AC-2 (FR-2,NFR-1): Given Tray is destroyed before `listen()` resolves, when promise resolves, then returned unlisten is called exactly once.
- AC-3 (FR-4,NFR-3): Given `npm run check`, when Svelte analyzes Command Panel, then no transcript noninteractive warning is emitted.
- AC-4 (FR-5): Given selected transcript text, when user presses Ctrl+C or right-clicks, then existing copy/context behavior still works.

## Risks and Rollback
- Risk: adding `tabindex` changes tab order. Mitigate with accessible label and keyboard test.
- Risk: callback guard suppresses valid late event during normal mount. Mitigate only guard after cleanup.
- Rollback: revert component/test/doc changes; no persistence migration.

## Master Spec and Changelog Updates
- `master_spec.md`: update persistent surface lifecycle paragraph to include `CommandPanelSurface.svelte` and `TrayPanelSurface.svelte`; update Quick Commands transcript semantics if changed.
- `changelog.md`: append dated `[CODE]` entry with tests run.

## Handoff Checklist
- [ ] Read `master_spec.md`, audit, this plan.
- [ ] Preserve dirty worktree.
- [ ] Write RED tests first.
- [ ] Implement only scoped files.
- [ ] Run focused validation.
- [ ] Update docs.
- [ ] Run adversarial QA.
- [ ] Do not touch excluded terminal/Git workbench features.

## Copy/Paste Implementation Prompt

```text
You are implementing remediation plan 06 in C:\dev\jasonshell. First read master_spec.md, docs/current-state-technical-audit-2026-08-28.md, CHANGELOG_POLICY.md, package.json scripts, and docs/remediation-plans/06-listener-lifecycle-transcript-semantics.md. Preserve unrelated dirty worktree changes. Use TDD: add RED tests for CommandPanelSurface and TrayPanelSurface delayed async listener cleanup and Command Panel transcript semantics before code changes. Scope code to CommandPanelSurface.svelte, TrayPanelSurface.svelte, focused tests, master_spec.md, and changelog.md. Do not implement persistent top JSON/shell terminal work or Stack Browser Git workbench features. Do not change IPC event names, payloads, settings schema, Quick Command execution semantics, or Tray keyboard parity. Implement disposed-guard async unlistener pattern and semantic/focusable transcript host so npm run check has no transcript warning while selection/Ctrl+C/context menu behavior remains. Validate with npm run check and npm run test:node; run npm run validate if feasible and report pre-existing unrelated failures separately. Update master_spec.md current behavior and changelog.md per policy. Run adversarial QA before final response.
```
