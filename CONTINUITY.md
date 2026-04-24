# Continuity Ledger

## Snapshot
- 2026-04-24 [CODE] Search Panel is implemented and validated: top bar owns search, `search-panel` renders results, Rust stores latest payload, and Windows Search/SystemIndex rows merge with local indexed snapshots.
- 2026-04-24 [USER] Goal: implement Stack Popup / Stack Browser as a persistent single `stack-popup` webview with top-bar pinned folders, pin-from-search, persistent navigation history, details table, copy/cut/paste/rename commands, app-local pin persistence, tests, and `stack_browser.md`.
- 2026-04-24 [CODE] Now: Stack Browser vertical slice is implemented and validated; top-bar folder pins open the persistent popup, search folder results expose a pin action, popup state keeps history across hide/show and folder switching, and backend commands cover folder reads, persisted pins, native clipboard copy/cut, paste, rename, and item open.
- 2026-04-24 [TOOL] QA: blocking Stack Popup recursive folder paste issue was found, then fixed with backend self/descendant paste guards and a Rust regression test.
- 2026-04-24 [CODE] Now: Stack Browser follow-up fixes are implemented and validated; folder switches/history navigation clear stale rows, and recursive self-copy/self-move into descendants is rejected.
- 2026-04-24 [CODE] Now: Search panel mouse behavior fixed; single-click selects a result, double-click launches it, one-click folder `Pin` pins to top bar, and Explorer folder drops on the top bar pin folders.
- 2026-04-24 [CODE] Now: folder pinning from search is fixed by resolving `shell:` aliases (Profile/Desktop/Personal/Downloads) to real filesystem paths before backend pin validation.
- 2026-04-24 [CODE] Now: search folder rows are draggable with internal folder path payloads, top-bar drop pins and opens the dropped folder, and first pin-load seeds default `Desktop` + `Downloads` pins.
- 2026-04-24 [CODE] Next: runtime coverage still depends on real filesystem permissions, Windows drag/drop delivery, and Windows clipboard/shell behavior.
- 2026-04-23 [USER] Open questions: none.

## Decisions
- 2026-04-23 [CODE] D001 ACTIVE: workspace-aware shell inventory docs should favor user-visible behavior over crate/module structure when summarizing the rewrite.
- 2026-04-23 [CODE] D002 ACTIVE: `features.md` will document the six shell surfaces from the rewrite docs and add supporting sections for search, workspace, automation, and visual behavior where those capabilities are user-facing.
- 2026-04-23 [CODE] D003 ACTIVE: search results render in a dedicated `search-panel` webview because the top bar is only 26 logical pixels tall; the actual search field remains in the top bar to the right of date/time.
- 2026-04-24 [CODE] D004 SUPERSEDED: the first JasonShell search catalog used existing local shell data sources only: pinned apps, open windows, common folders, and built-in commands.
- 2026-04-24 [CODE] D005 SUPERSEDED: broad live search uses bounded native scans of Start Menu/program locations and common user folders, then merges those backend results into the existing Svelte search panel payload.
- 2026-04-24 [CODE] D006 ACTIVE: system search queries an in-memory persistent index populated from app/file roots and persisted to `search-index-v1.json`; refresh is asynchronous and emits an index-ready event so typing does not trigger recursive scans.
- 2026-04-24 [CODE] D007 SUPERSEDED: Windows Search integration was a guarded COM query-helper probe/fallback boundary, not a SystemIndex row provider.
- 2026-04-24 [CODE] D008 ACTIVE: Windows Search integration should prefer real SystemIndex OLE DB rows when available and use the warmed app-managed persistent index only as the unavailable/empty fallback.

## Done (recent)
- 2026-04-24 [CODE] Completed Search Panel through Windows Search/SystemIndex parity and cached provider refresh behavior.
- 2026-04-24 [CODE] Added persistent `stack-popup` webview creation, surface routing, TS invoke wrapper, StackPopup Svelte surface, top-bar pinned-folder strip, and search-panel folder pin action.
- 2026-04-24 [CODE] Added stack backend helpers for app-local pin persistence, folder details, latest popup request, item open, copy/cut/paste, rename, and native Windows file clipboard integration.
- 2026-04-24 [CODE] Added `stackPopupState` reducer tests for history persistence, stale payload ignoring, selection, size formatting, and path name extraction.
- 2026-04-24 [CODE] Fixed QA findings: stale stack rows are cleared on folder/history changes, and folder paste rejects self/descendant destinations to avoid recursive copies.
- 2026-04-24 [CODE] Fixed Search Panel mouse interactions by replacing row `mousedown` launch with click-to-select/double-click-to-launch and a real one-click pin button; added top-bar native drag/drop folder pinning from Explorer paths.
- 2026-04-24 [CODE] Fixed Stack pin persistence path validation for curated search folders by resolving supported `shell:` aliases before canonical existence checks.
- 2026-04-24 [CODE] Added internal drag-and-drop from Search Panel folder rows to Top Bar pin rail; drop path pins via existing backend command and opens stack popup for the dropped folder.
- 2026-04-24 [CODE] Added default pin seeding logic so first-load pin state includes Desktop and Downloads when those folders exist.
- 2026-04-24 [CODE] Added `stack_browser.md` describing behavior, commands, files, and keyboard shortcuts.

## Working set
- 2026-04-23 [CODE] `CONTINUITY.md`
- 2026-04-24 [CODE] `src-tauri/src/main.rs`
- 2026-04-24 [CODE] `src-tauri/src/shell_windows.rs`
- 2026-04-24 [CODE] `src-tauri/src/stack_popup.rs`
- 2026-04-24 [CODE] `src/App.svelte`
- 2026-04-24 [CODE] `src/lib/shellSurface.ts`
- 2026-04-23 [CODE] `src/components/TopBar.svelte`
- 2026-04-24 [CODE] `src/components/TopBar.css`
- 2026-04-24 [CODE] `src/components/SearchPanelSurface.svelte`
- 2026-04-24 [CODE] `src/components/StackPopupSurface.svelte`
- 2026-04-24 [CODE] `src/lib/stackPopup.ts`
- 2026-04-24 [CODE] `src/lib/stackPopupState.ts`

## Receipts
- 2026-04-24 [TOOL] Search Panel milestone validations passed across targeted panel payloads, broad search, persistent index, realtime refresh, and Windows Search/SystemIndex row retrieval; latest pre-stack validation had `npm run validate` passing with 7 Node tests and 43 Rust tests.
- 2026-04-24 [TOOL] Strict Windows Search implementation validation: `cargo fmt --manifest-path src-tauri/Cargo.toml` completed; `npm run cargo:test` passed 41 Rust tests.
- 2026-04-24 [TOOL] `npm run test:search` first hit sandbox `spawn EPERM`; approved rerun passed 7 Node tests. `npm run validate` passed with `svelte-check` 0 errors/0 warnings, Vite build, 7 Node tests, 41 Rust tests, and `cargo check`.
- 2026-04-24 [TOOL] Final QA closure reran `npm run validate`; it passed with `svelte-check` 0 errors/0 warnings, Vite build, 7 Node tests, 41 Rust tests, and `cargo check`.
- 2026-04-24 [TOOL] Cairo parity search pass validation: `cargo fmt --manifest-path src-tauri/Cargo.toml` completed; `npm run test:search` passed 7 tests after sandbox `spawn EPERM` approved rerun; `npm run cargo:test` passed 43 tests; `npm run validate` passed fully.
- 2026-04-24 [TOOL] Stack Browser validation: first sandboxed `npm run test:search` hit known Node `spawn EPERM`; approved rerun passed 13 Node tests. `cargo test --manifest-path src-tauri/Cargo.toml` passed 47 Rust tests.
- 2026-04-24 [TOOL] Final Stack Browser `npm run validate` passed: `svelte-check` 0 errors/0 warnings, Vite build passed, 13 Node tests passed, 47 Rust tests passed, and `cargo check` passed. `cargo fmt --manifest-path src-tauri/Cargo.toml --check` also passed.
- 2026-04-24 [TOOL] Stack Popup QA review reran targeted validation: sandboxed `npm run test:search` hit `spawn EPERM`; approved rerun passed 13 Node tests. `npm run cargo:test` passed 47 Rust tests.
- 2026-04-24 [TOOL] Post-QA Stack Browser validation passed: `npm run test:search` approved rerun passed 15 Node tests after sandbox `spawn EPERM`; `npm run cargo:test` passed 48 Rust tests; final `npm run validate` passed with 15 Node tests, 48 Rust tests, Vite build, `svelte-check`, and `cargo check`; `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passed.
- 2026-04-24 [TOOL] Search mouse/drop fix validation passed: `npm run validate` completed with `svelte-check` 0 errors/0 warnings, Vite build, 15 Node tests, 48 Rust tests, and `cargo check`.
- 2026-04-24 [TOOL] Pin fix validation passed: `npm run cargo:test` passed 49 Rust tests after adding shell-alias resolution coverage; `npm run validate` passed with `svelte-check` 0 errors/0 warnings, Vite build, 15 Node tests, 49 Rust tests, and `cargo check`.
- 2026-04-24 [TOOL] Drag/default-pin validation passed: final `npm run validate` succeeded with `svelte-check` 0 errors/0 warnings, Vite build, 15 Node tests, 49 Rust tests, and `cargo check`.
