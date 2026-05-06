# Shell Open Close Event Correctness P2

Status: Draft, implementation not started  
Source: `findings.md`, Suggested Fix Order item 2  
Priority: P2, run after `shell_surface_contract_parity_p1.md`

## Goal

Make close/open events target the webviews that own state. This fixes stale TopBar search state, audio hidden polling after close, and tray panel stale snapshots.

## Issue

Current findings show cross-window event delivery drift:

- Search native focus-loss emits `search-panel:closed` on `search-panel`, while `TopBar.svelte` owns `searchOpen` reset.
- Audio close is emitted to TopBar, but the `audio-panel` webview does not hear its own close event.
- Tray panel loads icons on mount only; `show_tray_panel` does not emit an open/refresh event to the tray webview.

Context7 confirms Tauri 2 supports targeted `emit_to`, and webview-specific listeners only receive targeted events for their target.

## Phase 1: RED Event Target Tests

Acceptance criteria:

- Tests fail because search close is not emitted to `top-bar`.
- Tests fail because audio panel close is not also delivered to `audio-panel`.
- Tests fail because tray open does not target `tray-panel` for refresh.

Tests to write:

- Source contract for `src-tauri/src/main.rs` requiring search focus-loss close to call `emit_to(shell_windows::TOP_BAR_LABEL, search_panel::SEARCH_PANEL_CLOSED_EVENT, ...)`.
- Source or Rust contract requiring explicit hide/close paths to share the same event helper.
- Node/source test requiring `TrayPanelSurface.svelte` to listen for a tray open event and reload icons on every open.
- Node/source test requiring `AudioPanelSurface.svelte` to listen for audio close and stop polling.

Implementation tasks:

- No production behavior change in this phase.
- Name event constants before writing shape-specific assertions where possible.

Subagents to run:

- `tauri-event-contract-test-worker`: use `rust-skills`, `tdd-guide`.
- `frontend-state-contract-test-worker`: use `senior-frontend`, `tdd-guide`.

Validation gate:

- Focused Node event tests.
- Focused Rust/source contract tests.

## Phase 2: Search Close State Repair

Acceptance criteria:

- Native search focus-loss and explicit hide notify TopBar.
- `TopBar.svelte` routes every native close through `resetActiveSearchState()`.
- Reopening centered search after outside click starts blank/fresh and does not stale-route show/publish behavior.

Tests to write:

- Extend `tests/searchCloseReset.test.mjs` so native close event drives the shared reset helper.
- Add source contract that explicit hide in `search_panel.rs` notifies TopBar or calls a shared notifier.

Implementation tasks:

- Add a Rust helper if needed, for example `emit_search_panel_closed_to_top_bar(app_handle)`.
- Use `emit_to(TOP_BAR_LABEL, SEARCH_PANEL_CLOSED_EVENT, ())`.
- Keep same-surface search-panel cleanup only if the search panel has real local cleanup work.

Subagents to run:

- `search-close-event-worker`: use `senior-frontend`, `rust-skills`, `tdd-guide`.

Validation gate:

- `node --test tests/searchCloseReset.test.mjs tests/searchPanelState.test.mjs`
- Rust tests covering `search_panel` if touched.

## Phase 3: Audio Close And Tray Open Refresh

Acceptance criteria:

- Audio panel receives an own-window close event and stops internal timers.
- TopBar still receives audio close and updates popup state.
- Tray panel receives an open event on every show and reloads icon snapshot.
- Hidden persistent webviews do not rely on mount-only state for runtime freshness.

Tests to write:

- Node/source test for `AudioPanelSurface.svelte` listening to `AUDIO_PANEL_CLOSED_EVENT`.
- Rust/source test requiring audio close emission to both `TOP_BAR_LABEL` and `AUDIO_PANEL_LABEL`, or a documented shared broadcast target if chosen.
- Node/source test for tray open listener and `loadTrayIcons()` on open.
- Rust/source test requiring `show_tray_panel` to `emit_to(TRAY_PANEL_LABEL, TRAY_PANEL_OPEN_EVENT, ...)`.

Implementation tasks:

- Add missing event constants in the local feature wrappers or central event registry, based on current repository convention.
- Emit tray open after show/focus succeeds enough for the webview to receive it.
- Preserve TopBar popup exclusivity behavior.

Subagents to run:

- `audio-close-state-worker`: use `senior-frontend`, `rust-skills`, `tdd-guide`.
- `tray-open-refresh-worker`: use `senior-frontend`, `rust-skills`, `tdd-guide`.

Validation gate:

- `node --test tests/trayPanelWiring.test.mjs tests/contractsSettings.test.mjs`
- `cargo test --manifest-path src-tauri/Cargo.toml tray_panel audio_panel`
- `npm run check`

## Phase 4: Live Smoke And QA

Acceptance criteria:

- Centered search: open, click desktop, press Windows key again, panel opens with blank fresh state.
- Audio: open sound, click away, wait longer than 2 seconds, no `get_audio_state` polling continues.
- Tray: change or mock tray icon list, open panel twice, second open shows fresh data.

Tests to write:

- No extra automated tests unless smoke exposes a deterministic gap.

Implementation tasks:

- Run smoke only after focused tests pass.
- Record any smoke limitation if `npm run tauri dev` is not safe in the environment.

Subagents to run:

- `event-delivery-adversarial-reviewer`: use `adversarial-reviewer`.
- `runtime-smoke-worker`: use `agent-browser` only if browser/Tauri smoke is explicitly safe; otherwise document manual smoke.

Validation gate:

- `npm run validate` when feasible.

