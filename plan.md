# Historical Bottom-Bar Plan

Status: historical implementation note. Current product and validation authority now live in `master_spec.md`, current source/tests, `README.md`, and `action_plan.md`.

This file describes an older bottom-bar task-window slice. It is retained for context only and should not override the current six-surface JasonShell specification.

## Original Scope

1. Keep the shell split intact: the bottom bar owns pinned launchers on the left and open-window tabs to the right, with icon-only launchers and labeled task buttons for live windows.
2. Add backend window commands for the bottom bar: `list_open_task_windows` returns stable HWND-backed window rows, and `activate_task_window` toggles inactive, minimized, and active windows with taskbar-like behavior.
3. Preserve Explorer-compatible launcher behavior: read pinned shortcuts from the taskbar pin directory, execute the `.lnk` itself when launching, and improve icon fidelity by resolving explicit shortcut icons or target icons before falling back to the shortcut file.
4. Keep the v1 window model simple: one tab per eligible top-level window on the primary monitor, no grouping, previews, or multi-monitor expansion.

## Current Status

- Bottom-bar behavior has moved beyond this original v1 plan and now includes grouped open-window tiles, previews, menus, reorder gestures, and a process-manager entry point.
- Multi-monitor behavior remains future work.
- Use `docs/smoke-test-windows.md` for live validation coverage.
