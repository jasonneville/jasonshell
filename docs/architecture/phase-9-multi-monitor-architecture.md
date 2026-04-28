# Phase 9 Multi-Monitor Architecture Decision

Status: Draft foundation for design/test-first Phase 9 work
Date: 2026-04-27
Owner: Worker A

## Context

JasonShell currently ships a stable single-monitor shell model: one primary top bar, one primary bottom bar, and hidden auxiliary windows (`task-preview`, `search-panel`, `stack-popup`, `process-manager`) that anchor from the current owning bar. `action_plan.md` Phase 9 requires design and monitor-mapping tests before broad multi-monitor implementation because the behavior crosses AppBar reservation, shell window placement, task grouping, previews, search/stack popups, and Explorer taskbar coexistence.

This document defines the intended architecture without turning on live multi-monitor duplication. The implementation foundation is a pure Rust planning layer in `src-tauri/src/layout.rs`; runtime window creation and AppBar activation remain single-monitor until a later accepted implementation wave.

## Decision

JasonShell will use a primary-shell plus secondary-task-strip model, not full duplicated bars per monitor.

The primary monitor owns the complete shell: top bar, bottom strip, AppBar reservation, search input, pinned stack rail, process-manager launcher, and auxiliary-window orchestration. Secondary monitors may receive bottom task strips in the later live implementation wave, but they do not receive duplicated top bars, search inputs, stack pin rails, settings/process buttons, or independent AppBar top reservations.

Rationale:

- Duplicating full top and bottom bars per monitor would multiply event routing, AppBar ownership, focus-loss handling, and hidden auxiliary webview targeting before the single-monitor shell contracts have live multi-monitor smoke coverage.
- Secondary task strips solve the main ergonomic need: windows on a secondary monitor can have a local task grouping/activation affordance without duplicating global shell controls.
- A single primary top bar preserves one canonical search and stack-popup entry point, reducing persistence and command-surface ambiguity.

## Monitor Ownership

- Exactly one monitor is selected as `PrimaryShell`. It is the OS primary monitor when known; otherwise the first enumerated monitor is used as a deterministic fallback.
- Every non-primary monitor is selected as `SecondaryTaskStrip`.
- The primary shell owns global commands and auxiliary surface routing.
- Secondary task strips own only monitor-local task grouping and task preview anchoring in the later runtime wave.
- No monitor may own both `PrimaryShell` and `SecondaryTaskStrip` at the same time.

## Bars And AppBars

- Primary shell: top bar and bottom strip reserve primary-monitor work area.
- Secondary monitor: bottom task strip planning is allowed, but live AppBar registration and work-area mutation are out of scope for this foundation.
- Full duplicated bars are explicitly rejected for Phase 9 wave 1.
- Live secondary strip creation must be gated behind accepted tests and mixed-DPI hardware smoke.

## Popup Anchoring

- Search and Stack Browser popups anchor to the primary top bar unless a future accepted spec adds a secondary top affordance.
- Task preview and process-manager-style bottom popups anchor to the owning bottom strip.
- Popup geometry must be computed in the source monitor's scale factor, then clamped to that source monitor's physical bounds.
- Hidden auxiliary webviews remain singleton windows in this foundation. Future live secondary strips must define whether singleton popups move between monitors or whether per-monitor auxiliary windows are introduced.

## Task Grouping And Strip Assignment

- A task window with a known monitor belongs to that monitor's task strip.
- If a window's current monitor cannot be resolved, JasonShell should keep the previous strip assignment when that monitor still exists.
- If neither current nor previous assignment is valid, the task falls back to the primary shell strip.
- Group identity remains application/process based; strip assignment is monitor-local so the same application can appear on multiple strips when its windows are spread across monitors.

## DPI Scaling

- All monitor shell and popup calculations use per-monitor scale factors.
- Logical bar heights convert independently per monitor and are rounded to physical pixels.
- Popup widths/heights convert using the source monitor scale factor and clamp against that monitor's physical bounds with edge padding.
- Cross-monitor logical coordinates must not be reused without conversion through the owning monitor.

## Explorer Taskbar Coexistence

- Current Explorer taskbar hiding/restoration is primary-taskbar-specific and must remain so until secondary taskbar behavior is explicitly modeled.
- Phase 9 wave 1 does not hide secondary Explorer taskbars or register secondary AppBars.
- A later live wave must detect `Shell_SecondaryTrayWnd` placement, decide coexistence vs hide/restore, and add rollback tests before changing secondary taskbar visibility.

## Out Of Scope

- Live duplicated top/bottom bars on every monitor.
- Runtime creation of secondary strip webviews.
- Secondary AppBar registration or secondary work-area mutation.
- Hiding or restoring `Shell_SecondaryTrayWnd`.
- Per-monitor auxiliary popup windows.
- Provider, automation, workspace, or control-plane changes.

## Acceptance Coverage

- `layout::plan_monitor_shell_layout` covers primary vs secondary ownership and mixed-DPI strip sizing.
- `layout::plan_popup_anchor` covers monitor-local popup anchoring and clamping.
- `layout::assign_task_strip_monitor` covers stable task-strip assignment.
- Focused validation should run `cargo test --manifest-path src-tauri/Cargo.toml layout`.
