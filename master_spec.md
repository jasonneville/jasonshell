# JasonShell master spec

Status: current architecture and behavior reference. Rewritten 2026-09-23 after the previous full version became too large for default intake. The prior full text is archived at `docs/archive/master_spec_full_legacy_2026-09-23.md` as historical detail. Use targeted search there when this file points to a legacy detail gap.

Source, tests, and observed runtime behavior are final authority when prose conflicts.

## Purpose

This spec records current architecture, behavior, limits, safety contracts, validation gaps, and lookup pointers. It is not a changelog and does not store per-request history. Use `CHANGELOG_POLICY.md` for history rules.

## Current product boundary

JasonShell is a Windows shell prototype built with Tauri 2, Svelte 5, TypeScript, Rust, and Win32. It runs as shell-adjacent native surfaces, not as a normal app window.

Current runtime target:

- Windows
- primary-monitor AppBar work-area reservation for top and bottom bars
- Tauri frontend windows/webviews backed by Rust/Win32 integration
- Node contract tests plus Rust tests as automated evidence

Current shipped surfaces:

- top and bottom bars
- Quick Launch
- task tiles, grouping, galleries, and previews
- Stack Browser and Stack text editor experiments
- persistent terminal panel
- Quick Commands and command panel
- centered search
- process manager
- audio, calendar, settings, tray, and related panels
- speech capture uses a dedicated transparent, noninteractive `speech-indicator` surface; its nonce-tagged `speech:voice-level` event carries only a normalized scalar meter, never audio or transcript data

Intentional limits:

- multi-monitor support is planning-only until source/tests and live smoke prove implementation
- workspace restoration is reserved and not implemented
- workspace startup commands are not executed automatically
- automation forwarding is planned and not wired
- docs must not turn a plan, audit, or archived statement into current behavior

## Architecture map

### Frontend

`src/App.svelte` hosts surface selection and window-level composition. `src/components/` owns rendered Svelte surfaces. `src/lib/` owns state, view models, IPC wrappers, feature contracts, and pure logic. `src/ipc/` owns command/event names and runtime bridge helpers.

Feature clusters:

- top bar: `src/components/TopBar.svelte`, `src/lib/topBarPins.ts`, `src/features/top-bar/`
- top-bar controls: terminal, Quick Commands, tray, speech/mic, and audio reorder horizontally by captured pointer drag after a 4px threshold; order persists in renderer `localStorage`, safely reconciles malformed/stale/duplicate IDs to the five known controls, and excludes time/calendar and search. Clicks below threshold retain existing panel/mic behavior; completed drags suppress the generated click. `src/lib/topBarControls.ts` owns pure ordering/reconciliation logic and `tests/topBarControlReorder.test.mjs` covers it plus Svelte wiring.
- taskbar and previews: `src/lib/taskbar*.ts`, `TaskPreviewSurface.svelte`, `TaskGallerySurface.svelte`
- Stack Browser/editor: `src/lib/stack*.ts`, `src/components/Stack*.svelte`
- terminal: `src/lib/terminalPanel.ts`, `persistentTerminal.ts`, terminal components
- search: `src/lib/search*.ts`, `SearchPanelSurface.svelte`
- settings/tray/audio/calendar/process manager: matching `src/lib/*` and `src/components/*Surface.svelte`

### Backend and native

`src-tauri/src/main.rs` registers commands, windows, setup, and plugins. Rust modules own native integration:

- shell windows and work area: `shell_windows.rs`, `appbar.rs`, `layout.rs`
- task windows: `task_windows/`, `task_preview.rs`, `task_gallery.rs`, `taskbar_menu.rs`, `launchers.rs`
- Stack Browser: `stack_popup.rs`, `stack_popup/`
- search: `search/`, `search_sources/`, `search_panel.rs`
- terminal and commands: `terminal_panel.rs`, `quick_commands.rs`, `command_panel.rs`
- process, audio, tray, settings, workspaces: matching Rust modules

### Tests

Node tests live in `tests/*.test.mjs` and usually assert source contracts, view-model behavior, generated snippets, or static runtime contracts. Rust tests live beside Rust modules. Use nearby tests for behavior claims.

### Speech model installation

Speech model provisioning is a runtime import workflow, not a repository or bootstrap prerequisite. Settings exposes **Import speech model**; the backend opens the picker, accepts only trusted Parakeet TDT 0.6b v2 int8 `.tar`, `.tar.gz`, or `.tgz` archives, validates them in a secure staging area, and requires a successful ONNX model load before replacing the installed model. The three required model files are `encoder-model.int8.onnx`, `decoder_joint-model.int8.onnx`, and `vocab.txt`.

The validated model is installed under the per-user `app_local_data_dir`, becomes available to the current session, and the startup resolver selects the newest valid installed generation after restart or app update without requiring the original archive. Audio and transcripts are not newly persisted by model installation. Installed files may be removed with the app's local data; documentation must not promise survival across uninstall or profile deletion. Archive provenance is guidance only: no pinned hash, license clearance, or independent compatibility claim is established.

Each replacement publishes a unique generation and retains older generations. Repeated imports therefore consume additional disk space, and an interrupted import may leave a staging directory; no power-loss durability guarantee is made.

Contract pointers: archive installation lives in `src-tauri/src/speech_model_install.rs`; ONNX layout validation/loading remains in `src-tauri/src/speech_model.rs`. Acceptance pointers are `src-tauri/tests/speech_model_install.rs` and `tests/components/settings-speech-import.test.ts`. The Settings contract uses the authorized, path-free commands `import_speech_model` and `get_speech_model_status`.

## Safety and no-persistence contracts

- AppBar and native-window changes must preserve rollback paths and avoid holding global locks across slow or fallible side effects.
- Process termination must validate immutable identity where applicable. A PID alone is not enough for safety-sensitive claims.
- Startup commands are not auto-run by workspace activation.
- Workspace restoration persists reserved metadata/status only. It does not restore windows unless current source/tests prove otherwise.
- Automation parsing and validation may exist for first-party intents. Forwarded automation payloads are not executed until wiring is implemented and tested.
- Live shell smoke can alter Windows shell state. Get human consent before running it.
- File/process/native-picker actions need backend validation, not only frontend hiding.
- Top-bar and native Stack Browser toggles emit `stack-popup:close-requested` to the mounted popup instead of hiding it directly. The popup routes requests through the dirty-editor exit guard: cancel leaves the draft and popup open; a clean or confirmed exit hides the popup and emits `stack-popup:closed` to update top-bar state. Live Windows shell verification remains consent-gated.
- Persistence claims need current source path, data shape, migration/default behavior, and tests or explicit validation gap.
- Global hotkeys expose only standard search, terminal, Stack Browser, and speech transcription actions. Settings persist validated canonical chords (defaults `Ctrl+Space`, `Alt+Backquote`, `Alt+1`, `Ctrl+D`); empty, control-character, bare, unknown, duplicate, and Ctrl+Alt/AltGr-conflicting bindings are rejected before persistence or registration.
- `windows_key_hook.rs` owns four no-repeat OS `RegisterHotKey` registrations on a dedicated Windows message thread and dispatches `WM_HOTKEY` to top-bar contract events; it does not intercept or suppress raw key releases. Startup loads persisted settings before registering. Settings saves validate and replace registrations before writing to disk; failed registration attempts restore prior bindings, and failed writes attempt to restore prior bindings. Windows does not provide atomic replacement: if another process claims an old chord during rollback, the save fails with explicit restoration details and retains verified active registrations; the claimed chord may remain unavailable until a later successful update. `Backquote` maps to the layout-sensitive `VK_OEM_3`. Live Windows hotkey and Alt+Tab behavior remains consent-gated manual-smoke pending.

## Subsystem pointers

Use `docs/current-state-brief.md` for the concise feature-to-code/tests/docs table. Use this section for broader ownership.

| Subsystem | Primary files | Validation |
|---|---|---|
| Top bar/AppBar | `TopBar.svelte`, `topBarPins.ts`, `shell_windows.rs`, `appbar.rs` | `tests/topBar*.test.mjs`, bootstrap/AppBar tests, Rust tests |
| Bottom bar/task windows | `taskbar*.ts`, `task_windows/`, `task_preview.rs` | `tests/taskbar*.test.mjs`, Rust task-window tests |
| Stack Browser/editor | `stack*.ts`, `Stack*.svelte`, `stack_popup/` | `tests/stack*.test.mjs`, Rust Stack tests |
| Terminal | `terminalPanel.ts`, `persistentTerminal.ts`, `terminal_panel.rs` | `tests/terminal*.test.mjs` |
| Search | `search*.ts`, `search/`, `search_sources/` | `tests/*search*.test.mjs`, Rust search tests |
| Quick Commands | `quickCommands.ts`, `commandPanel.ts`, `quick_commands.rs` | command and Quick Command tests |
| Workspaces/automation | `workspaces.ts`, `automation.ts`, `workspaces.rs`, `automation.rs` | `tests/workspaces.test.mjs`, `tests/automationProviders.test.mjs` |
| Process manager | `processManager*.ts`, `process_manager.rs` | process manager tests and Rust tests |
| Panels | matching `src/lib`, Svelte surface, Rust module | matching focused tests |

## Validation commands

Use `package.json` as source of truth. Current configured scripts include:

- `npm run test:node`
- `npm run check`
- `npm run build`
- `npm run cargo:test`
- `npm run cargo:check`
- `npm run smoke:runtime`
- `npm run smoke:fullscreen`
- `npm run validate`

Docs-only changes can use focused docs-policy tests plus link/size checks. Implementation changes need the focused tests for touched source, and larger gates when contracts or runtime boundaries cross subsystems.

## Active known validation gaps

- Stack Browser outbound drag is exclusively native Shell/OLE via `start_stack_file_drag`: pointer motion on the same held primary pointer crosses a 6px threshold, then a UI-thread Win32 button check gates `SHDoDragDrop` with the popup HWND and copy-only effect. Same-parent selections use an owned absolute parent PIDL and borrowed child-relative PIDLs; mixed-parent selections return typed `unsupported` (`mixed-parent`) until a standards-compliant `CF_HDROP` data object is implemented. Typed outcomes contain request ID, item count, status/effect, duration, mechanism, and failure stage, without paths. During drag, popup focus loss is held and topmost demoted; cleanup never steals focus. `tests/stackPopupNativeDrag.test.mjs` tests the pointer intent and absence of HTML outbound wiring, not external transfer.
- Stack native drag still needs consent-gated real Windows Explorer/Teams smoke, especially folder/multi-select, Unicode/space paths, cancellation, linked entries, window z-order and successful external copy. Static and compile checks cannot establish the external OLE transfer.
- Native Windows behavior still needs manual smoke for high-risk shell changes.
- Multi-monitor behavior remains planning-only.
- Plans and audits may describe desired fixes or stale failures. Confirm against current source/tests before using them as a claim.
- Some Node tests assert static source contracts. Passing them does not prove live Win32/Tauri behavior.
- Live smoke is consent-gated because it can mutate shell state.

## Reference pointers

- `docs/current-state-brief.md`: default current-state onboarding and lookup map.
- `docs/README.md`: docs index and task-based reading guide.
- `README.md`: active product truth, setup, caveats, validation script list.
- `CHANGELOG_POLICY.md`: append-only history policy and targeted lookup rules.
- `changelog.md`: historical ledger. Search only when history is needed.
- `docs/archive/master_spec_full_legacy_2026-09-23.md`: historical full spec. Search for detail only after checking current source/tests and this spec.
- `docs/remediation-plans/README.md`: audit remediation map. Use when doing or touching a named plan.
- `docs/smoke-test-windows.md`: Windows smoke guidance.

## Maintenance rules

Update this spec when durable current behavior, architecture, event/command contracts, persistence, validation coverage, intentional limits, or known risks change. Keep details concise and point to source/tests for implementation mechanics.

Do not append change ledger entries here. Do not duplicate source-level implementation detail. Do not claim planned behavior as current behavior.
