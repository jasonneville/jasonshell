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
- speech capture uses a dedicated transparent, noninteractive `speech-indicator` surface visible only during recording: a centered, fixed 40×40px opaque circle with a local native-WebGL Rare UI Fluid Orb adaptation and the original 24px mic SVG alpha mask. Cool alloy paint progresses from navy above to silver below; the normalized meter level moves the silver boundary upward as voice grows louder and downward as it softens. The surface maps raw meter level to visual fill with `min(1, level / 0.5)` before smoothing: raw 0.25 targets half fill and raw 0.5 or higher targets full fill. This doubles presentation sensitivity without changing speech detection or the generic smoother. Presentation-only quadratic ease-out interpolates from displayed fill with a 100ms rise and gentler 180ms fall, reaching the exact bounded target; identical targets, including saturated targets, do not restart easing. Meter snapshots, the 0.035 threshold and 300ms word-gap hold remain unchanged: quiet/invalid packets retain the last audible target, then expiry starts the eased release to zero. Terminal status and new nonces reset both meter and displayed fill immediately; stale nonces are ignored. Reduced motion snaps to the current target, hidden documents stop interpolation and resume from displayed fill, and settled fill schedules no frames; presentation listeners/frames are cleaned up on unmount. Mid-level matches the selected navy/silver palette; an internal 0.5px SVG keyline derived from the same artwork preserves full-fill readability without shadows or a backing disk. There is no volume-detected outer ring, glow, waveform, pulse, or geometry change. The exterior remains transparent. Ambient selected-theme-accent/white drift is independent of voice; the renderer resolves the root `--js-color-accent` initially and observes applied theme changes, redrawing paused and reduced-motion stills without starting extra animation chains. Unavailable, failed, or lost WebGL retains an opaque solid fallback using the same theme accent and original alloy ink/internal keyline. Existing shell theme synchronization supplies cross-surface updates; no new theme IPC is used. Animation pauses outside recording and when the document is hidden; reduced-motion changes select a still frame. Forced colors hide decoration and render a CanvasText mic with a static system-color accessibility outline, never a volume highlight. Renderer listeners, theme observer, animation frames, shaders, program, and buffer are cleaned up on unmount. Its nonce-tagged `speech:voice-level` event carries only a normalized scalar meter, never audio or transcript data
- screen snipping uses scoped native overlay and preview surfaces; its native Windows journey remains pending/unverified

Intentional limits:

- multi-monitor support is planning-only until source/tests and live smoke prove implementation
- workspace restoration is reserved and not implemented
- workspace startup commands are not executed automatically
- automation forwarding is planned and not wired
- screen snipping does not provide annotations, spanning mode, autosave, capture history, or upload
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
- snipping: `src/lib/snipping.ts`, `src/lib/shellSurface.ts`, `src/lib/surfaceLoader.ts`, `src/lib/topBarControls.ts`, `SnipOverlaySurface.svelte`, `SnipPreviewSurface.svelte`

### Task preview presentation

The single-window hover preview uses the inset-lens treatment: a theme-token beveled raised frame, softer corners, a recessed bordered thumbnail, and a decorative app icon beside the title/process caption. Blank or failed icons are omitted without affecting the caption or window actions. The existing activation and separate close controls, hover lifetime, gallery nonce handling, captured-image fallback, and unavailable state remain unchanged.

The preview floats 8 logical pixels away from its host edge (scaled and rounded for DPI), above the taskbar or below its host on fallback placement; monitor-edge clamping still takes priority when space is insufficient. Opening a preview no longer paints a seam connector or removes the source tile's top border/bevel. Existing delayed hide and pointer-entry cancellation remain the hover-crossing mechanism.

The native host remains 332×228 logical pixels. `TaskPreviewSurface.css` and `task_preview.rs` share the thumbnail content geometry: 12 logical pixels on the sides and bottom, and 48 on top; the source is aspect-fit inside that frame. The native viewport remains transparent while its inset perimeter stays visible. Browser fixture coverage in `tests/browser/task-preview-inset/` uses the production component/CSS with mocked IPC; it establishes DOM/style geometry, not live DWM compositing. Native shell smoke remains consent-gated.

### Grouped taskbar context menu

Bottom-bar gallery capsules (two or more windows) use the same external context-menu overlay and shared menu primitives as standalone task tiles. Their right-click menu exposes Open in Process Manager, Pin to taskbar, and Close all windows; gallery hover/click and drag behavior remain separate. The origin retains an opaque-token-scoped member snapshot; the overlay receives presentation state, not HWNDs or paths. Process and pin prefer the member that was active when the menu opened if it still survives, otherwise the first captured survivor (process lookup requires a valid PID). Selections recheck group membership and HWND/PID against current taskbar state and exclude newly joined windows. Close all rechecks each target, continues after individual failures, and uses the authorized `run_task_window_action` request-only `request-close` route: native identity/PID validation precedes WM_CLOSE dispatch, without termination or elevation fallback. App save prompts can therefore keep windows open. Existing standalone close behavior is unchanged. Rendered mocked-IPC coverage lives in `tests/components/taskbar-group-menu.test.ts`; native source safety contracts live in `tests/taskbarGroupMenuContract.test.mjs`. Live WebView2/native shell verification remains consent-gated.

### Terminal panel startup and status presentation

The persistent terminal panel keeps hidden mount lazy: it does not eagerly create xterm/ConPTY, and schedules the unchanged bounded `5000 ms` idle prewarm. A panel-open event or terminal-panel focus cancels the pending prewarm and starts or joins the shared startup promise. Existing backend sessions are reattached rather than duplicated; idle prewarm may start/list-attach the backend session without creating xterm. Visible startup still preserves attach/replay, focus, resize-before-input, tab/split, reuse, and cleanup behavior.

Normal startup and output waiting never cover the terminal with a status notice. Genuine failure, exit, and explicit stopped-state diagnostics remain visible and accessible before a pane exists and after output, including startup rejection, PTY/read failure, exit-before-output, later exit, and stopping the final tab. Existing output remains visible alongside a later diagnostic, and retry clears the prior failure without duplicating notices. Rendered coverage is in `tests/components/terminal-panel-status.test.ts`; its mocked IPC/xterm environment is not native ConPTY or desktop Tauri certification. The focused policy and tradeoffs are documented in `docs/terminal-panel-prewarm-idle-policy.md`.

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

Settings also exposes **Import speech model folder** through path-free, Settings-only `import_speech_model_folder`. Its native folder chooser shares archive import admission, busy reservation, epoch invalidation, cancellation, and readiness handling. Folder import copies only the three required root files into the same installed generations, ignoring extras and rejecting empty/nonregular/reparse root/files. Windows opened source handles are checked for reparse attributes and deny write/delete sharing; bounded reads verify actual sizes before staged loading and atomic publication. This does not establish protection against ancestor path replacement. Native picker acceptance and real-model inference remain consent-dependent validation gaps.

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
- Stack Browser mouse Back/Forward owns the top visible in-webview layer before folder history: Back closes the editor, Git drawer/detail, menu, inline folder input, or confirmation layer one level at a time; Forward is consumed while any such layer is visible. Dirty editor Back opens the existing discard confirmation, whose Back cancels safely and whose busy state consumes navigation; confirmation Cancel preserves the draft. Bare-folder Back/Forward alone moves the existing folder history, while toolbar buttons and keyboard navigation remain unchanged. Terminal mode excludes folder-history dispatch, but the global mouse side-button handler prevents default and stops propagation before reaching that terminal guard; the terminal branch is not demonstrated as a reachable live integration and adds no terminal UI or behavior. Compatibility `mousedown`/`auxclick` events are suppressed after mouse `pointerdown`; non-mouse pointers and ordinary buttons remain unowned. Ownership is implemented in `src/components/StackPopupSurface.svelte`, `StackTextEditor.svelte`, `StackGitPanel.svelte`, and `StackConfirmDialog.svelte`.
- Returning from the Stack text editor or Git workbench preserves the current folder viewport only when the same `currentPath` and folder-search query remain active: the popup captures the in-memory details scroll offset and viewport height before replacing the folder grid, restores virtual-row state after the grid remounts, and refocuses the grid with `preventScroll`. A path or search mismatch discards a stale checkpoint, and repeated editor/Git opens replace it with the latest offset. This is renderer-memory behavior for the mounted popup only; it is not cross-folder history, session, or persistence behavior.
- Persistence claims need current source path, data shape, migration/default behavior, and tests or explicit validation gap.
- Global hotkeys expose only standard search, terminal, Stack Browser, and speech transcription actions. Settings persist validated canonical chords (defaults `Ctrl+Space`, `Alt+Backquote`, `Alt+1`, `Ctrl+D`); empty, control-character, bare, unknown, duplicate, and Ctrl+Alt/AltGr-conflicting bindings are rejected before persistence or registration.
- Snipping adds the fifth configurable hotkey with default `Alt+S` and appends its scissors control after Sound without changing the existing v1 order/key. When an old settings file omitted the fifth field and an existing old binding already owns `Alt+S`, the old binding is preserved, ID5 remains vacant, and Settings reports an actionable conflict; no silent remap or duplicate registration occurs. Explicit five-way duplicates remain invalid.
- `windows_key_hook.rs` owns up to five no-repeat OS `RegisterHotKey` registrations on a dedicated Windows message thread and dispatches `WM_HOTKEY` to top-bar contract events; it does not intercept or suppress raw key releases. Startup loads persisted settings before registering. A legacy missing-field `Alt+S` conflict leaves the fifth slot vacant while preserving the original four registrations. Settings saves validate and replace registrations before writing to disk; failed registration attempts restore prior bindings, and failed writes attempt to restore prior bindings. Windows does not provide atomic replacement: if another process claims an old chord during rollback, the save fails with explicit restoration details and retains verified active registrations; the claimed chord may remain unavailable until a later successful update. `Backquote` maps to the layout-sensitive `VK_OEM_3`. Live Windows hotkey and Alt+Tab behavior remains consent-gated manual-smoke pending.

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
| Screen snipping | `src/lib/snipping.ts`, `Snip*Surface.svelte`, `src-tauri/src/snipping/`, `settings.rs`, `windows_key_hook.rs` | snipping component/Node/Rust suites; `tests/snipping-product-handoff.md` |

### Screen snipping contract

Snipping starts only through the exact registered top-bar instance or the trusted native hotkey dispatcher. Native authorization is concrete-instance based for the caller, each overlay, and the preview; matching labels alone are never authority. The capture freezes all enumerated monitors before creating overlays. Ordinary JasonShell bars, panels, and tooltips remain included in the pixels; snipping overlays and the retained previous preview are excluded from capture. The first backend-accepted monitor pointer-down binds the generation, and completion crops only that monitor using trusted native geometry/scale.

Completion creates a hidden non-activating preview, then attempts the existing automatic native Copy immediately before showing it. Clipboard outcomes remain distinct and truthful: committed, committed-warning, rejected, and publication-unknown. The visible preview persists without activation and places right-aligned icon controls below the image: **Copy**, **Save**, and **Dismiss**. A separate header **Close** control uses **Dismiss** behavior. Save is path-free at the renderer boundary, uses the native PNG picker, pins the generation/image while the picker is open, and publishes through a same-directory sibling/atomic PNG path; cancellation has no file effect.

The renderer receives bounded binary PNG data and no HWND, path, origin, scale, or display authority. Snipping capabilities are scoped separately to the top bar, overlay, and preview. The native helper and runtime use bounded deadlines (including the five-second preparation/readiness bound), monitor/frame/staging budgets, exact token checks, and fail-closed cleanup. These limits do not certify live OS behavior.

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
- Stack mouse Back/Forward has focused coverage in `tests/components/stack-mouse-navigation.test.ts` and `tests/components/stack-git-mouse-navigation.test.ts` (expanded 32/32 focused tests). The test-only `tests/browser/stack-mouse/` fixture was exercised in visible browsers: the Git journey returned `#result` `git: PASS`, and the dirty-editor journey returned `editor: PASS` after real CodeMirror insertion plus Cancel/Discard handling; the event ledger showed pointerdown prevention with inert compatibility `mousedown`/`auxclick` events. A separately consented headed Chrome run of the large-directory journey returned `viewport: PASS`: the 400-file folder restored `scrollTop` 10500 through clean Markdown Preview and dirty Cancel→Discard returns, then restored a latest 3000 offset; all samples had finite 425.7875px body height, 31 rendered rows, and the selected row visible, with two initial folder reads only and no hide/delete/save/revert/checkout/create calls. The browser fixture imports production global CSS and uses real animation frames; this establishes DOM/layout/remount/focus behavior, not trusted hardware delivery, WebView2 XButton delivery, or native browser-history suppression. `npm run validate` passed with 75 component tests, 1196 Node tests, 880 Rust tests (10 ignored), plus `check` and `build`; the scoped fixture changes also passed build and 24 scoped checks. WebView2 XButton 3/4 delivery and native shell smoke remain consent-gated and unperformed.
- Native Windows behavior still needs manual smoke for high-risk shell changes.
- Snipping's automated coordinator, save, hotkey, native-registry, component, and actual-runtime fixtures are bounded contract evidence. The standalone native synthetic-region journey is still required; live HWND discovery, GDI/compositor capture, real mouse/AppBar coexistence, mixed-DPI hardware, and consumer paste remain unverified. Do not describe these fixtures as full native end-to-end coverage or as a coverage percentage.
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
