# JasonShell Action Plan

Source: [future_enhancements.md](C:/dev/jasonshell/future_enhancements.md)  
Generated: 2026-04-27

This plan turns the full `future_enhancements.md` roadmap into an execution sequence that is biased toward early correctness, validation trust, and maintainable foundations. The first phases intentionally avoid piling on new product surface area until the current shell is safer to change. Lower-priority features are still included, but they are pushed behind the foundational work they depend on.

## Planning Rules

- Phase order is dependency-first, not novelty-first.
- Validation trust, state correctness, and architectural seams come before new workflow features.
- Documentation/spec work is attached to the implementation phase that makes it accurate, so docs do not drift away from code.
- Every numbered item from `future_enhancements.md` is mapped in the coverage matrix at the end of this file.

## Item Prefixes

- `MS#` = `Master Spec Readability Improvements` items 1-10.
- `R#` = `Priority Roadmap` items 1-35.

## Phase 0: Stabilize Validation, Top-Bar State, and Product Truth

Priority: highest  
Goal: restore trust in local validation, remove the most immediate state bug, and force the docs to describe the product that actually exists.

Worker B status, 2026-04-27:
- `R2`, `R3`, `R5`, `R11`, `MS2`, `MS3`, `MS6`, `MS7`, `MS8`, and `MS10` are addressed for the non-frontend validation/CI/documentation scope by the Node harness repair, Windows CI workflow, current-product README/features refresh, historical doc banners, and live smoke checklist.
- `R4` is documented as parked/experimental from this worker scope unless another worker confirms shipped system-tray integration.
- `R1` and `MS1` are outside Worker B's assigned code/doc ownership unless handled by another worker.

Includes:
- `R2` Repair `tsconfig.test.json` and Node test source coverage
- `R1` Fix TopBar pin hydration recursion
- `R3` Resolve `taskbarGroups.test.mjs` drift
- `R4` Decide the system tray slice: integrate or park
- `R5` Add CI before major refactors
- `R11` Update README and docs to reflect current reality
- `MS1` New agent quick start
- `MS2` Documentation authority order
- `MS3` Separate current behavior from future direction
- `MS6` Test harness caveats
- `MS7` Clarify system tray status
- `MS8` Mark historical docs clearly
- `MS10` Add a live smoke checklist

Implementation steps:
1. Fix the Node test compilation model so every `dist-tests/*.js` import is backed by a current TypeScript source entry.
2. Rename or clarify `test:search` so validation names match actual scope.
3. Resolve the TopBar pin recursion and add focused source-level coverage.
4. Make an explicit product decision on `system_tray`: integrate now or mark as parked/experimental.
5. Add Windows CI for `check`, `build`, Node tests, Cargo test, and Cargo check.
6. Refresh `README.md`, `master_spec.md`, `features.md`, `plan.md`, `wave_*.md`, and add `docs/smoke-test-windows.md`.

Exit criteria:
- `npm run check`
- `npm run build`
- `node --test tests/*.test.mjs`
- `npm run cargo:test`
- `npm run cargo:check`
- CI passes on a clean Windows checkout
- Live smoke checklist exists and covers all currently shipped surfaces

Why first:
- Later phases are not credible if validation can pass against stale artifacts.
- Workspace and search improvements depend on stable TopBar pin state.
- The repo currently has documentation ambiguity around what is active, parked, or aspirational.

## Phase 1: Visual System and Accessibility Baseline

Priority: very high  
Goal: make the current shell surfaces consistent, readable, accessible, and resilient before deeper feature work.

Includes:
- `R12` Introduce shared visual tokens
- `R13` Improve accessibility semantics
- `R14` Add responsive and constrained-width layouts
- `R15` Add reduced-motion and contrast preferences
- `R16` Improve status hierarchy
- `R17` Extract inline styles into CSS files

Implementation steps:
1. Create the global design-token layer in `src/app.css`.
2. Migrate each surface to tokens instead of hardcoded colors, radii, spacing, and shadows.
3. Move inline CSS from Search, Stack Browser, and Task Preview into dedicated CSS files.
4. Fix ARIA and keyboard semantics for Process Manager, Search Panel, Stack Browser menus, and TopBar rail controls.
5. Add constrained-width behavior for narrow layouts and high DPI scenarios.
6. Add reduced-motion behavior and reserve high-contrast support in the token system.
7. Standardize loading, info, warning, and error states across surfaces.

Exit criteria:
- `npm run check`
- `npm run build`
- Keyboard-only pass for all current surfaces
- Narrow-width pass at 320, 420, 720, 980, and 1440 logical widths
- Manual reduced-motion pass
- Visual screenshot pass for all six surfaces

Why here:
- This improves daily usability without increasing architectural risk.
- Accessibility and responsive behavior are easier to fix before large component extractions.

## Phase 2: Break Up High-Risk UI and Rust Monoliths

Priority: high  
Goal: reduce change risk in the largest modules before adding more behavior into them.

Includes:
- `R6` Split large Svelte surfaces into feature components
- `R7` Split `stack_popup.rs` by responsibility
- `R20` Improve Stack Browser scalability and polish

Implementation steps:
1. Create `src/features/top-bar`, `bottom-bar`, `search`, `stack-browser`, and `process-manager`.
2. Extract Stack Browser subcomponents first: toolbar, breadcrumb, details grid, context menu, inline editor, DnD controller, keyboard controller, folder loader.
3. Keep state transitions pure in `stackPopupState.ts` or move them into a dedicated feature state module.
4. Split `src-tauri/src/stack_popup.rs` into submodules while preserving command names and payload shapes.
5. Use the extraction to enable virtualization, current-folder watcher support, stronger delete UX, breadcrumb overflow treatment, and clearer toolbar hierarchy.

Exit criteria:
- `npm run check`
- Focused Node tests around extracted controllers/state
- `cargo test --manifest-path src-tauri/Cargo.toml stack_popup`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Live smoke for Stack Browser keyboarding, menus, drag/drop, and large folders

Why before more shell UX:
- Stack Browser is central to the Explorer-replacement direction and is already oversized.
- Deferring this refactor would make later workspace/file actions more expensive and riskier.

## Phase 3: Contract, Capability, and Diagnostics Hardening

Priority: high  
Goal: centralize IPC contracts, narrow authority per surface, and add first-class diagnostics before product breadth increases.

Includes:
- `R8` Create a single IPC/event/surface contract layer
- `R9` Harden Tauri capabilities and CSP
- `R10` Introduce a diagnostics layer
- `MS4` Add a surface capability matrix
- `MS5` Add a contract index for events and commands

Implementation steps:
1. Create frontend contract modules in `src/ipc/` and backend contract definitions in `src-tauri/src/contracts.rs`.
2. Replace inline event names, command names, and surface labels with shared constants.
3. Add tests for wrapper targets, command names, and payload types where useful.
4. Update Tauri capabilities so each surface has only the authority it needs.
5. Define a production-vs-development CSP policy instead of leaving CSP null.
6. Add frontend/backend diagnostics ring buffers with redaction and export support.
7. Reflect the resulting contracts and authorities in `master_spec.md`.

Exit criteria:
- `npm run check`
- `npm run build`
- Node tests for IPC wrappers/contracts
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Live smoke across all surfaces after capability tightening

Why here:
- Later settings, workspaces, tasks, CLI, and extensions will multiply IPC traffic and permissions.
- Hardening this layer early reduces accidental contract drift and over-privileged surfaces.

## Phase 4: Shell UX Upgrade Without Workspace Coupling

Priority: medium-high  
Goal: raise the quality of the current shell interaction model before introducing workspace-driven workflows.

Includes:
- `R18` Improve bottom bar overflow and group behavior
- `R19` Make Process Manager a serious developer tool
- `R21` Make Search a premium command palette
- `R22` Give Top Bar a stronger shell/menu identity

Implementation steps:
1. Add bottom-bar overflow affordances, stronger group state indicators, and keyboard focus handling.
2. Upgrade Process Manager with filtering, mini bars, safer kill UX, and tree-aware presentation.
3. Rework Search into grouped keyboard-first results with recent/frequent behavior and better default/secondary actions.
4. Add TopBar shell identity elements: JasonShell/Home menu, Places, Programs/apps flyout, indexing indicators, and room for a future workspace pill.

Exit criteria:
- `npm run check`
- Focused TS tests for search ranking/state and process filtering/sorting
- Rust guardrail tests for process actions where applicable
- Live smoke for keyboard-only task switching, search workflows, and process actions

Why before workspace features:
- These improvements strengthen the shell itself and create clearer UI anchors for workspace-driven behavior later.
- Search should already behave like a command surface before workspaces inject tasks and commands into it.

## Phase 5: Durable Settings and Persistence Foundation

Priority: high  
Goal: establish versioned, recoverable, secure persistence before adding workspaces, tasks, tools, or automation.

Includes:
- `R23` Add versioned settings architecture before settings UI
- `MS9` Document durable config ownership before adding features

Implementation steps:
1. Create `src-tauri/src/settings.rs` and `src/lib/settings.ts`.
2. Define the initial versioned JSON files and migration rules for settings, workspaces, and task history.
3. Define corrupt-file backup behavior, secret-handling rules, and clear Rust-vs-frontend persistence ownership.
4. Move current persistence assumptions into the new config model without breaking existing pin/index behavior.
5. Document the config contract in `master_spec.md`.

Exit criteria:
- Rust load/save/migrate tests
- Corrupt-backup tests
- TS wrapper tests
- Restart persistence smoke

Why now:
- Workspaces, tools, tasks, saved searches, and automation all require a durable config contract.
- Delaying this would force repeated persistence rewrites in later phases.

## Phase 6: Workspace Core and Shell Activation Model

Priority: high  
Goal: make workspace profiles the central product concept and connect them to the shell layout safely.

Includes:
- `R24` Add workspace profiles as the central developer concept
- `R25` Make workspace activation drive shell layout

Implementation steps:
1. Implement workspace CRUD and activation APIs in Rust and TypeScript.
2. Define workspace schema, validation, aliasing, pins, tool defaults, and task declarations.
3. Make activation update the TopBar, search biasing, workspace pins, and task exposure.
4. Add safety rules around startup commands and environment handling.
5. Reserve but do not yet fully implement later window/app restoration behavior.

Exit criteria:
- Rust schema and path validation tests
- TS tests for workspace search results and activation plans
- Live smoke for activate/update/delete workspace and shell layout changes

Why before tool integrations:
- Terminal/editor/git/task features should attach to a workspace model, not an ad hoc path picker.

## Phase 7: Tooling and Execution Workflows

Priority: medium-high  
Goal: turn workspace profiles into operational developer workflows.

Includes:
- `R26` Add terminal and editor integration
- `R27` Add git-aware workspace status
- `R28` Add workspace task runner and task history

Implementation steps:
1. Add safe command-template expansion for terminal and editor launches.
2. Expose editor/terminal actions from workspace state and file interactions.
3. Add git status, ahead/behind, and merge/rebase state to the workspace pill and search.
4. Implement task execution, output streaming, cancellation, and task history persistence.
5. Associate JasonShell-launched tasks/processes with process-manager views.

Exit criteria:
- Rust quoting/injection tests
- Rust git parsing/temp-repo tests
- Rust spawn/cancel/history tests
- TS task rendering/ranking tests
- Live smoke with VS Code, Windows Terminal, `npm run validate`, and dirty/clean git repos

Why here:
- Once settings and workspaces exist, these features deliver the first strong developer-control-plane value.

## Phase 8: Developer-Aware File, Search, and Process Depth

Priority: medium  
Goal: deepen the shell’s power-user workflows after workspace/task primitives are stable.

Includes:
- `R29` Add developer Stack Browser actions
- `R30` Add saved searches and search providers
- `R31` Add developer-aware process details

Implementation steps:
1. Expand Stack Browser context actions around paths, templates, editor/terminal launches, and git-aware file operations.
2. Add bounded providers for workspace files, recent files, git changes, task history, commands, settings, and processes.
3. Add saved-search persistence and workspace scoping.
4. Extend Process Manager with command line, ports, process-tree context, workspace ownership, and kill-tree guardrails.

Exit criteria:
- Rust file-operation tests
- Provider unit and bounded-performance tests
- Rust kill-tree guardrail tests
- TS provider/filter/group tests
- Live smoke across monorepos, developer processes, clipboard actions, and workspace-owned tasks

Why after Phase 7:
- These features depend on workspace context, task history, tool integration, and a trusted persistence model.

## Phase 9: Multi-Monitor Architecture and Shell Expansion

Priority: lower than single-monitor workflow maturity  
Goal: design and then implement multi-monitor support only after the single-monitor shell model is stable.

Includes:
- `R32` Design multi-monitor before implementing it

Implementation steps:
1. Produce the monitor ownership and AppBar architecture decision.
2. Decide whether bars are duplicated per monitor or centered on a primary shell with secondary task strips.
3. Define popup anchoring, task grouping, DPI scaling, and Explorer taskbar coexistence rules.
4. Add monitor-mapping tests before broad implementation.
5. Implement in a second wave only after the design and tests are accepted.

Exit criteria:
- Written architecture decision
- Rust monitor-mapping tests
- Mixed-DPI live smoke on multi-monitor hardware

Why late:
- It cuts across layout, AppBar, shell window placement, previews, search popups, and process UI.
- Doing this before single-monitor workflows settle would multiply rework.

## Phase 10: Automation, Providers, and Control-Plane Surfaces

Priority: lowest until the local shell contracts are stable  
Goal: expose JasonShell outward through automation and then add the control panels that sit on top of the stable foundations.

Includes:
- `R33` Add `jasonshell` CLI automation
- `R34` Add extension/provider model after contracts stabilize
- `R35` Add settings panel and developer dashboard

Implementation steps:
1. Add opt-in local automation transport and single-instance forwarding for safe CLI control.
2. Enforce explicit security boundaries for destructive and unauthenticated actions.
3. Introduce config-driven providers first, not arbitrary plugin execution.
4. Build the Settings panel on top of the already-versioned config architecture.
5. Build the Developer Dashboard on top of existing workspace, git, task, and process data.

Exit criteria:
- Rust CLI parser and permission tests
- Provider contract and performance-budget tests
- Settings persistence tests
- Keyboard/a11y pass for dashboard/settings
- Live workflow smoke against a running app

Why last:
- CLI automation and extension points freeze contracts; they should not be added while core surface/state contracts are still moving.
- Dashboard/settings UI should consume stable data systems, not invent them.

## Cross-Phase Dependency Notes

- Do not start `R23` before `R2`, `R5`, and `R8` are complete enough to make persistence changes safe and testable.
- Do not start `R24`-`R28` until `R1`, `R12`-`R16`, and `R23` are stable.
- Do not start `R33`-`R35` until `R8`, `R9`, `R10`, and `R23`-`R28` have settled.
- `R20` should begin with refactor-enabling work in Phase 2; deeper polish can continue in Phase 8 only if the architecture remains stable.

## Recommended First Backlog Slice

If execution starts immediately, the first queue should be:

1. `R2` Test harness repair
2. `R1` TopBar pin recursion fix
3. `R3` Taskbar group test drift resolution
4. `R4` System tray integrate-or-park decision
5. `R5` Windows CI
6. `R11` + `MS1` + `MS2` + `MS3` + `MS6` + `MS7` + `MS8` + `MS10` documentation alignment
7. `R12` token system bootstrap
8. `R13` accessibility fixes

## Coverage Matrix

### Master Spec Readability Improvements

- `MS1` -> Phase 0
- `MS2` -> Phase 0
- `MS3` -> Phase 0
- `MS4` -> Phase 3
- `MS5` -> Phase 3
- `MS6` -> Phase 0
- `MS7` -> Phase 0
- `MS8` -> Phase 0
- `MS9` -> Phase 5
- `MS10` -> Phase 0

### Priority Roadmap

- `R1` -> Phase 0
- `R2` -> Phase 0
- `R3` -> Phase 0
- `R4` -> Phase 0
- `R5` -> Phase 0
- `R6` -> Phase 2
- `R7` -> Phase 2
- `R8` -> Phase 3
- `R9` -> Phase 3
- `R10` -> Phase 3
- `R11` -> Phase 0
- `R12` -> Phase 1
- `R13` -> Phase 1
- `R14` -> Phase 1
- `R15` -> Phase 1
- `R16` -> Phase 1
- `R17` -> Phase 1
- `R18` -> Phase 4
- `R19` -> Phase 4
- `R20` -> Phase 2
- `R21` -> Phase 4
- `R22` -> Phase 4
- `R23` -> Phase 5
- `R24` -> Phase 6
- `R25` -> Phase 6
- `R26` -> Phase 7
- `R27` -> Phase 7
- `R28` -> Phase 7
- `R29` -> Phase 8
- `R30` -> Phase 8
- `R31` -> Phase 8
- `R32` -> Phase 9
- `R33` -> Phase 10
- `R34` -> Phase 10
- `R35` -> Phase 10
