# JasonShell Features

Status: mixed current and roadmap inventory. `master_spec.md` and current source/tests are authoritative for shipped behavior.

## Current Behavior

- Top bar: primary-monitor AppBar with pinned folder stacks, date/time, search input, folder pin menus, drag/drop pinning, and Stack Browser launch.
- Bottom bar: primary-monitor AppBar with Explorer taskbar `.lnk` launchers, grouped open-window tiles, preview/menu support, activation/minimize behavior, reorder gestures, and a process-manager button.
- Search panel: auxiliary webview for command/search results from pinned apps, open windows, static commands, and indexed system app/file/folder sources.
- Stack Browser: auxiliary folder browser with paged folder reads, history navigation, sorting, selection, file operations, inline rename/new-folder, context menus, drag/drop, and pin persistence updates.
- Task preview: auxiliary webview for hover previews of open task windows.
- Process manager: auxiliary process table with CPU/memory/process metadata, sortable columns, focus-loss close behavior, and guarded kill actions.
- Validation harness: `npm run validate` runs Svelte check, TypeScript/Vite build, Node helper tests, Rust tests, and Cargo check.

## Parked Or Experimental

- System tray support is parked/experimental. Repo evidence shows a test-only Windows backend module, `src/lib/systemTray.ts`, and `tests/systemTray.test.mjs`; `src-tauri/src/main.rs` intentionally does not register shipped tray commands until a later phase adds a visible surface, capabilities, product docs, and live-smoke coverage.

## Roadmap Direction

- Visual system and accessibility hardening across all six surfaces.
- IPC/event/surface contract centralization and Tauri capability tightening.
- Durable settings/config ownership before workspaces, tasks, automation, or extension/provider models.
- Workspace profiles, terminal/editor/git/task workflows, saved searches, process-depth improvements, CLI automation, and developer dashboard surfaces.
- Multi-monitor architecture after the single-monitor shell model is stable.

## Historical Notes

Older Cairo-derived feature descriptions for settings panels, workspace profiles, dashboards, Spotify controls, and broad multi-monitor taskbar behavior are aspirational references unless and until they are reintroduced through `action_plan.md` phases and reflected in `master_spec.md`.
