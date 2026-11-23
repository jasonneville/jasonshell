# Audit 06 — Product, feature-gap & opportunity audit

Scope: feature completeness, UX flows, product opportunities, creative ideas. Read-only. Grounded in `README.md` and a source skim of `src/components/*`, `src/lib/*`, `src/features/*`, `src-tauri/src/*`. Nothing under `docs/` or other root `*.md` was read. Effort: S = days, M = 1–3 weeks, L = a month or more. Value is for a single power user running JasonShell as a daily shell.

## Product, feature-gap & opportunity audit

### Executive summary

JasonShell already ships a lot of engineering depth: guarded native IPC, ConPTY terminal, a real Git workbench, a recovery-staged file browser with undo, a phased search engine with an Everything provider, per-app audio, DWM previews, and local speech-to-text. The weak spot is the product layer that connects these parts. Most surfaces are separate islands opened by their own hotkey, and the code has several "contract-only" scaffolds that look finished but are not:

- **Search** declares `Calculator`, `Web` and `Bookmarks` provider IDs and result kinds (`src-tauri/src/search/contracts.rs:219-243`, `src/lib/searchEngine.ts:12-43`) and gives them scoring weights (`src-tauri/src/search/scoring.rs:263,276,336`). No provider module implements them: `src-tauri/src/search/providers/` contains only `apps.rs, everything.rs, local.rs, open_windows.rs, settings.rs`.
- **Workspaces** have a full data model: pins, tasks, env with `Prompt` sources, startup commands, tool defaults (`src-tauri/src/workspaces.rs:12-216`). Activation only returns a plan, and restoration is hard-coded `"reserved-not-implemented"` (`workspaces.rs:9`).
- **Automation** parses a CLI grammar with security levels (`automation.rs:41-47,70-74`), but `FORWARDING_STATUS = "planned-not-wired"` (`automation.rs:7`), and only five verbs exist.
- **Providers registry** (`providers.rs:36-43`) defines `WorkspaceFiles, GitChanges, TaskHistory, Commands, Settings, Processes` with budgets, but `arbitrary_plugin_execution_allowed` is a contract flag, not an extension system.
- **Dev tools task runner** (`dev_tools/task_runner.rs`) spawns workspace tasks with history and events (`task:started/output/completed`). It overlaps heavily with Quick Commands (`quick_commands.rs`), which is the surface users actually see.
- **Control Plane** ("Settings & Developer Dashboard", `ControlPlaneSurface.svelte:92`) is reachable only through a search command (`searchCatalog.ts:17-23`) and the top bar (`TopBar.svelte:1827`). It overlaps with Settings.

Highest-leverage direction: **one keyboard-first command surface (search) as the hub**. Every capability (Quick Commands, workspaces, window actions, Git actions, settings, snips, speech history, clipboard history) becomes a search result or action, plus a small set of always-on status widgets on the top bar.

---

### (1) Per-surface gap table

| Surface | Has (grounded) | Missing vs industry | Suggested next |
|---|---|---|---|
| **Top bar** | Reorderable controls `['terminal','command','tray','mic','sound','snip']` (`src/lib/topBarControls.ts:1`); clock with settings section (`SettingsPanelSurface.svelte:725`); search entry; Control Plane entry | No status widgets (battery, network, CPU/RAM sparkline, media now-playing, weather, focus/DND, VPN, Bluetooth); no notification center; no per-widget click actions; no "workspace" indicator. YASB/Zebar/Rainmeter and macOS menu bar are the benchmarks | Widget framework over existing data: System Info cache (`process_manager/system_info.rs`) for CPU/RAM/net, `audio.rs` for volume, plus new SMTC and battery providers |
| **Bottom taskbar** | Explorer pins as launcher source (`launchers.rs`, `explorer.rs`); task tiles; grouping with Ribbon/Matrix galleries; live/captured DWM previews; overflow ticker; attention flashing (`task_windows/attention.rs`); per-app toast badge counts via `UserNotificationListener` (`task_windows/notifications.rs:6-27`); right-click group actions (Process Manager, Pin, Close all); run-as-admin in menus (`taskbarMenus.ts`, `launchers.rs`) | Jump lists / recent files per app; progress overlays (ITaskbarList3 progress); thumbnail toolbar buttons (media prev/play/next); middle-click new instance / close; drag-to-reorder tiles; Win+1..9 activation; window snapping/tiling from tiles; virtual desktop filtering; multi-monitor taskbars | Win+N activation, middle-click, then jump lists and progress. Window-management actions in tile menus |
| **Quick Launch** | Native panel over Explorer pins (`QuickLaunchPanelSurface.svelte`, `quick_launch_panel.rs`) | Folders/categories, recents/frequents, type-to-filter, launch with args, custom (non-Explorer) pins | Type-to-filter, plus merge into search as a "Pinned" provider |
| **Stack Browser** | Virtualized listing; search in current folder; breadcrumbs; new file/folder, rename, extract; cut/copy/paste; drag-move; undo move/delete with recovery storage (`stack_popup/recovery_journal.rs`); Open With (`open_with.rs`); native drag-out; live folder watching (`folder_watch.rs`); CodeMirror editor (`StackTextEditor.svelte`) with save-conflict handling; Markdown preview (`StackMarkdownPreview.svelte`); embedded terminal pane (`StackTerminalPane.svelte`); Git panel (Changes/History/Stashes/Branches, commit, fetch/pull/push, checkout, create branch) | Tabs; dual pane; preview pane for images/PDF/media/hex (Peek/QuickLook); bookmarks/favorites sidebar beyond pins; bulk rename; compress (extract exists, compress does not); file tags/colors; size calculation per folder; content search (Everything exists in search but not in Stack); "recent locations"; Recycle Bin integration (deletes go to JasonShell recovery storage, README §Stack Browser); Git blame, hunk staging, merge-conflict resolution, rebase/cherry-pick, amend (grep of `StackGitPanel.svelte` finds 0 hits for blame/cherry/rebase/amend) | Space-to-preview (Peek), tabs, then hunk staging + blame |
| **Terminal panel** | ConPTY + xterm; tabs/splits; profiles (Windows Terminal, Git Bash, PowerShell); shell integration; command records with exit codes (`terminalHistory.ts`); quick fixes (`terminalQuickFixes.ts`); quick-select hint labels (`terminalQuickSelect.ts`); 20 actions incl. rerun, copy output, open cwd in VS Code/Git workbench (`terminalActions.ts:50-70`) | Session restore across restarts (scrollback + layout); command palette; broadcast input to panes; AI command generation/explain (Warp); blocks UI (Warp); per-profile themes / terminal palette editor (README: "no terminal palette editor"); quake drop-down animation; SSH profile manager; "save command as Quick Command" | "Save as Quick Command" from a command record; layout/session restore; palette editor tied to themes |
| **Quick Commands** | Saved entries `{label, mode, targetPath, args, commands, cwd, artifactLocation}` (`settings.rs:223-235`); live transcript; stdin input (`SendQuickCommandInputRequest`, `quick_commands.rs:267`); run history; job-object-tracked launch; completion banner with chime (`quick_command_notification.rs`) | Parameters/placeholders prompted at run; per-command hotkeys; scheduling/triggers (on login, interval, on file change); not in search (no `quick_command` reference in `src-tauri/src/search*` or `src/lib/search*.ts`); groups/folders; import/export; chaining/pipelines; failure-only notifications; Windows toast output | Expose in search (S), parameters (M), per-command hotkey (M) |
| **Search** | Phased engine with progress events, provider timings/health (`searchEngine.ts`); apps, open windows, settings (rich alias table, `search/providers/settings.rs`), local important folders, Everything (with install helper `search_sources/everything_install.rs`); `copyText`, `runControlPanel` actions | Calculator, unit/currency conversion, web search fallback, bookmarks (all declared, none implemented); clipboard history; emoji picker; window actions (close/move/snap from result); file actions (reveal, copy path, open with); Quick Commands; Git repos; system commands (lock, sleep, empty recycle bin); action panel per result (Raycast ⌘K); prefix scopes (`>` commands, `@` windows, `/` files); frecency learning; extensions | Implement Calculator + Web (S each), action panel (M), prefix scopes (S) |
| **Process Manager + System Info** | Grouped processes (Applications/Background/Windows, `processManagerState.ts:34-44`); sortable metrics; guarded end process; System Info Overview/Storage/Sensors/Network/Hardware; quota-aware storage | History charts (README: "no battery/history/charts/actions"); battery; startup apps manager; services tab; open file location / search online; suspend/resume; priority/affinity; per-process network; "what's using my file" (Restart Manager); top-bar mini graphs | Sparkline history buffer (S/M), Startup apps tab (M), Open file location (S) |
| **Audio** | Master volume/mute; per-app session volume/mute; device list and default device switching (`audio.rs:68-123`) | Media controls / now playing (SMTC: no `GlobalSystemMediaTransportControls` reference anywhere in source); input device/mic level; per-app output routing; volume OSD; scroll-on-icon volume; spatial/EQ presets | SMTC now-playing card + media keys in top bar (M) |
| **Calendar** | Month grid with wheel navigation (`CalendarPanelSurface.svelte`, 118 lines) | Events (Outlook/ICS/Google), week numbers, world clocks, timers/stopwatch/alarms, countdowns, "time until next meeting" in top bar | ICS feed + world clocks (M) |
| **Tray** | Enumerate and invoke tray icons (`system_tray.rs:108-136`) | Overflow pinning/hiding rules, icon reorder, tooltip/badge refresh. README warns "tray behavior needs caution" | Hide/pin rules persisted in settings (S/M) |
| **Settings** | Appearance with custom theme editor; Clock; JSON settings; Speech model import; 8 hotkeys with draft/save; Shell bars; Power (sleep/restart/shutdown with confirm, `system_power.rs`); Exit (`SettingsPanelSurface.svelte:660-890`) | Settings search; import/export/backup; profiles; theme import/export (README: none); wallpaper/accent sync; autostart toggle; per-surface feature toggles; "what's new"; hotkeys for more than 8 fixed actions (`StandardHotkeySettings`, `settings.rs:41-60`); lock screen action | Settings export/import (S), settings-in-search deep links (S) |
| **Control Plane** | "Settings & Developer Dashboard" with sections for settings, workspaces, Git statuses, task history, processes, providers (`ControlPlaneSurface.svelte:1-209`, `controlPlaneState.ts`) | Clear purpose vs Settings; editing workspaces (it only displays them); task launching UI | Turn it into the Workspaces hub, or fold it into Settings + search |
| **Speech (Ctrl+D)** | Local Parakeet ONNX; streaming; orb indicator (WebGL); safe paste to the captured foreground target with identity checks (`speech_target.rs`); in-session history panel (`speech_history_panel.rs`, `speech.rs:99`) | Voice commands ("open terminal", "run build"); punctuation/format modes; custom vocabulary; push-to-talk vs toggle; persisted history (opt-in); LLM cleanup/rewrite; dictation into the terminal | Voice-command grammar routed through automation intents (M) |
| **Snipping** | Native freeze overlay across monitors; Copy/Save/Dismiss preview (`snipping/*`) | Annotations, OCR (Text Extractor), color picker, pin-to-screen, window/fullscreen capture modes, delay timer, capture history, recording (GIF/MP4). README: "no annotations, spanning mode, autosave, capture history, upload" | OCR via `Windows.Media.Ocr` (M), pin-to-screen (S/M), annotations (M/L) |
| **Notifications** | Quick Command completion banner; per-app toast counts on tiles | Notification center/history; DND/focus modes; mirroring Windows toasts into a shell surface; JasonShell-internal event feed (Git push done, snip saved, speech pasted) | Unified in-shell notification feed (M) |
| **Window management** | Focus/maximize/minimize/close only (`task_windows/actions.rs:64-71`) | Snap layouts, FancyZones, tiling (GlazeWM/komorebi), always-on-top, move to monitor, window switcher (Alt+Tab replacement), opacity, "hide all / peek desktop" | Snap actions in tile menu and search (M), then zones (L) |
| **Workspaces** | Data model, CRUD, activation plan (`workspaces.rs:219-293`) | Any UI to create/edit; restoration; startup tasks; context switching | See PRD-05/BB-01 |
| **Multi-monitor / virtual desktops** | Snipping captures all monitors; AppBar is primary-only (README caveats); no `IVirtualDesktopManager` reference anywhere | Per-monitor bars, per-monitor taskbar filtering, desktop switcher | Phase 3 big item |
| **Onboarding** | Bootstrap script; README; speech model import | No first-run surface (only grep hit for "tour"/"welcome" is unrelated `folderDrag.ts`); no hotkey cheat sheet; no "what's new" | First-run tour + `?` cheat sheet (S/M) |

---

### (2) Findings

#### Search and command hub

- **PRD-01 — Implement the Calculator provider that the contracts already promise** (High value, S). Evidence: `SearchProviderId::Calculator` and `SearchResultKind::Calculator` in `src-tauri/src/search/contracts.rs:226,239`; scoring weights at `search/scoring.rs:263,276,336`; `copyText` action kind exists in `src/lib/searchEngine.ts:39`; no `providers/calculator.rs`. Recommendation: add a pure-Rust expression evaluator (e.g. `meval`/hand-written Pratt parser) behind the existing provider timing model. Start with arithmetic, %, ^, hex/bin (`0xff`, `0b1010`), and bit ops for devs. Enter copies the result through the existing `copyText` action. Phase 2: units (`5 km in mi`), dates (`now + 90d`), time zones (`3pm PST in London`). This is the cheapest "feels like Raycast" win.

- **PRD-02 — Web search fallback and bookmarks** (Med, S). Evidence: `Web`/`Bookmarks` provider IDs and `openWebUrl`/`openBookmark` action kinds (`searchEngine.ts:40-41`, `contracts.rs:170,202`) have no provider. Recommendation: (a) when the query is a URL, offer "Open URL"; (b) always offer a last-place "Search the web for …" row with a configurable engine; (c) read Chrome/Edge `Bookmarks` JSON (read-only, local) for the Bookmarks provider. Keep provider budgets from `providers.rs` (150 ms default).

- **PRD-03 — Quick Commands as first-class search results** (High, S). Evidence: no `quick_command` or `quickCommand` reference in `src-tauri/src/search/`, `src-tauri/src/search_sources/` or `src/lib/search*.ts`; the only "commands" in the catalog are three static entries (`searchCatalog.ts:15-40`). Recommendation: add a `QuickCommands` provider reading `settings.quick_commands.entries`. Enter runs the command (reusing the existing job-tracked launch). Shift+Enter opens it in Quick Commands, and a secondary action opens the last run. This turns Ctrl+Space into a launcher for the user's scripts.

- **PRD-04 — Per-result action panel (Raycast ⌘K / PowerToys Run context actions)** (High, M). Evidence: `SearchResult` carries a single `action` (`searchEngine.ts:66-95`); app tile menus already implement run-as-admin and pin actions (`taskbarMenus.ts`, `launchers.rs`); Stack menus already have copy path, open terminal here, properties (`stackPopupViewModel.ts:512-586`). Recommendation: add an `actions[]` list per result and a Ctrl+K / Tab panel. Apps get Run as admin, Open file location, Pin, Uninstall settings. Windows get Close, Minimize, Move to monitor, Snap left/right. Files and folders get Reveal in Stack, Copy path, Open with, Open terminal here. Reuse the existing native commands so there is no new authority surface.

- **PRD-05 — Search prefix scopes and mode chips** (Med, S). Evidence: providers are combined in a single ranking (`search/scoring.rs`); there is no scope parsing in `search/mod.rs`. Recommendation: `>` commands and Quick Commands, `@` windows, `/` or `~` files (Everything), `=` calculator, `?` web, `:` emoji, `git ` repos. Show chips under the input for discoverability, and narrow the provider set when a prefix is present so results come back faster.

- **PRD-06 — Frecency learning across all providers** (Med, M). Evidence: scoring is static by kind/provider weights (`scoring.rs:263-336`); `ACTIVATION_SEARCH_BOOST` exists only for workspaces (`workspaces.rs:8`). Recommendation: persist a small (query-prefix → result-id, count, last-used) table with time decay. Boost picks for the same prefix, as Spotlight and Raycast do. It must be bounded and clearable from Settings.

- **PRD-07 — Window actions and window switcher from search** (High, M). Evidence: open-window provider exists (`search/providers/open_windows.rs`); window verbs limited to focus/max/min/close (`task_windows/actions.rs:64-71`). Recommendation: an Alt+Tab-like "switcher" mode of the centered search that shows live DWM thumbnails (already produced for galleries, `task_windows/previews.rs`). It should support fuzzy filtering and Ctrl+W to close.

- **PRD-08 — Settings deep-links inside JasonShell** (Med, S). Evidence: settings provider covers Windows `ms-settings:` targets (`search/providers/settings.rs:131-140`) but JasonShell's own Settings sections (Appearance, Clock, Speech, Keyboard shortcuts, Shell bars, Power) are not individually searchable. The catalog has one "Open developer dashboard" command (`searchCatalog.ts:17`). Recommendation: index every JasonShell setting heading and important toggle as a `setting` result that opens Settings scrolled to and focused on that control. Typing "theme", "hotkey" or "speech model" should land directly.

- **PRD-09 — System verbs in search** (Med, S). Evidence: `system_power.rs` implements Sleep/Restart/Shutdown, Settings-caller-only (`system_power.rs:26-36`); there is no lock or sign-out action. Recommendation: expose Lock (`LockWorkStation`), Sleep, Restart, Shutdown, Empty recycle bin, Toggle dark mode, Restart Explorer and Exit JasonShell as search commands. Keep the existing confirm dialog for destructive verbs, which means the caller policy has to be extended deliberately rather than loosened.

#### Clipboard, snippets, text tools

- **PRD-10 — Clipboard history** (High, M). Evidence: no `AddClipboardFormatListener` or clipboard-history store anywhere; the clipboard is written only by speech (`speech_clipboard.rs`), Stack (`stack_popup/clipboard.rs`) and snipping (`snipping/clipboard*.rs`). Recommendation: an opt-in listener that stores text, images (from snips) and file lists in a bounded encrypted-at-rest store (DPAPI). It should honour `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory` formats so password managers are skipped. Surface it as a search scope (`;` or Ctrl+Shift+V hotkey) with pinning, paste-as-plain-text, and reuse of the safe paste-target logic in `speech_target.rs`.

- **PRD-11 — Text snippets / expansions** (Med, M). Evidence: none exist. Quick Commands are process launchers only (`settings.rs:223-241`). Recommendation: snippets with placeholders (`{date}`, `{clipboard}`, `{cursor}`), searchable and pasteable through the same target-safe paste path as speech. A keyword-expansion hook is a later, riskier phase on the existing keyboard-hook infrastructure (`windows_key_hook.rs` uses `WH_KEYBOARD_LL`).

- **PRD-12 — Emoji / symbol picker** (Low, S). Evidence: none. Recommendation: a static dataset provider under the `:` prefix that pastes through the speech paste path.

#### Quick Commands and automation

- **PRD-13 — Parameterized Quick Commands** (High, M). Evidence: `QuickCommandEntry` has fixed `args`/`commands`/`cwd` (`settings.rs:223-235`); `WorkspaceEnvValueSource::Prompt` already exists in the workspace model (`workspaces.rs:84-90`), so the prompt concept is designed but unused. Recommendation: `{{name}}`, `{{choice:a|b}}`, `{{file}}`, `{{clipboard}}` and `{{cwd}}` placeholders, rendered as a small form before the run. Remember the last values per command.

- **PRD-14 — Per-command hotkeys** (Med, M). Evidence: hotkeys are a fixed 8-field struct (`settings.rs:41-60`); the README describes a strict eight-field Save. Recommendation: generalize `StandardHotkeySettings` into fixed core slots plus a bounded user list (`{actionRef, chord}`) that covers Quick Commands, workspaces, snippets and window actions. Reuse the conflict detection and capture lease already built.

- **PRD-15 — Triggers and schedules for Quick Commands** (Med, M). Evidence: there is no scheduler. Workspace startup modes are only `ManualOnly | SuggestOnly` (`workspaces.rs:106-111`). Recommendation: triggers on JasonShell start, every N minutes, at a time of day, on file change (reuse `stack_popup/folder_watch.rs`), on network connect, and on Git push. Default to "notify, don't run" for the first occurrence, then require an explicit opt-in.

- **PRD-16 — Unify `dev_tools::task_runner` with Quick Commands** (Med, M). Evidence: `task_runner.rs` has its own history file `jasonshell-task-history-v1.json`, a 50-entry cap, and events `task:started/output/completed` (`task_runner.rs:16-24`). Quick Commands has a separate history in settings (`settings.rs:168-177`) and job-object launch (`quick_commands/windows_launch.rs`). `ShellSettings.task_history: Vec<Value>` is a third untyped list (`settings.rs:33`). Recommendation: make workspace tasks a kind of Quick Command scoped to a workspace. Drop or alias the separate runner so there is a single history, a single notification path and a single process-tracking guarantee.

- **PRD-17 — Wire the automation CLI (`jasonshell.exe --action …`)** (High, M). Evidence: parse/validate exists with ReadOnly/Mutating/Destructive levels and opt-in, authenticated, user-present and confirm flags (`automation.rs:1-208`). Forwarding status is `"planned-not-wired"` (`automation.rs:7`). Verbs are only `Help, ShowSearch, ListProviders, ActivateWorkspace, DeleteWorkspace` (`automation.rs:41-47`). Recommendation: implement single-instance argv forwarding (`tauri-plugin-single-instance`), keep the validator as the gate, and grow the verb set. Candidates: `toggle terminal`, `run-quick-command <id>`, `snip`, `search "<q>"`, `open-stack <path>`, `notify "<text>"`. This unlocks Stream Deck, AutoHotkey, Task Scheduler, and scripts that end with `jasonshell notify "build done"`.

- **PRD-18 — Completion notifications for terminal commands** (Med, S/M). Evidence: shell integration records exit codes and timing (`terminalHistory.ts:3-20`); Quick Commands already has a banner and chime (`QuickCommandNotificationSurface.svelte`). Recommendation: when a terminal command longer than N seconds finishes while the terminal panel is hidden, reuse the Quick Command banner (showing exit code and duration). Clicking it opens the terminal at that command block.

- **PRD-19 — "Save as Quick Command" from terminal history** (Med, S). Evidence: `terminalActions.ts:52-54` has copy command, copy output and rerun, but no save action. Recommendation: add a "Save as Quick Command" action that prefills command, cwd and shell profile.

#### Workspaces

- **PRD-20 — Workspace editor UI** (High, M). Evidence: CRUD commands exist (`workspaces.rs:219-290`) and the Control Plane only displays workspaces (`ControlPlaneSurface.svelte:29-30`). There is no create/edit form. Recommendation: a Workspaces section with name, root, aliases, pins, tasks and startup. Offer "Create workspace from this folder" in Stack Browser and terminal cwd menus.

- **PRD-21 — Workspace activation that does something** (High, L). Evidence: `activate_workspace` returns a `WorkspaceActivationPlan` with layout/search/pins/tasks/startup plans (`workspaces.rs:142-216`); restoration is reserved. Recommendation, staged: (1) scope search and Stack pins to the workspace and show it in the top bar; (2) open a terminal tab at the root with tool defaults; (3) offer startup tasks as one-click suggestions (`SuggestOnly` already exists); (4) restore window layout (needs PRD-27). See BB-01.

#### Window management, desktops, monitors

- **PRD-22 — Snap/arrange actions** (High, M). Evidence: there are only four window actions (`task_windows/actions.rs:64-71`). Recommendation: Snap left/right/top/bottom, quarters, center, "almost maximize", move to next monitor and always-on-top (`SetWindowPos HWND_TOPMOST`), exposed in tile menus, search actions (PRD-04) and assignable hotkeys (PRD-14).

- **PRD-23 — Zones / tiling layouts (FancyZones-lite)** (Med, L). Evidence: there is no layout engine. `layout.rs` is about shell preview rects (`layout.rs:1-30`). Recommendation: named zone templates per monitor, with Shift-drag snapping into zones and optional auto-tiling for selected apps. This is a prerequisite for workspace layout restore.

- **PRD-24 — Virtual desktop awareness** (Med, M). Evidence: no `IVirtualDesktopManager` usage in `src-tauri/src`. Recommendation: filter taskbar tiles to the current desktop with `IVirtualDesktopManager::IsWindowOnCurrentVirtualDesktop` (a public API), and show a desktop indicator on the top bar. Desktop switching and moving windows rely on undocumented interfaces, so treat that as an optional experiment.

- **PRD-25 — Multi-monitor bars** (High for multi-screen users, L). Evidence: README says "multi-monitor support is planning-only"; AppBar is primary-only. Recommendation: per-monitor bottom bar that shows only that monitor's windows, plus a primary top bar with an optional mirrored minimal bar on secondary monitors. Gate it behind a setting until it is live-tested.

- **PRD-26 — Show desktop / peek** (Low, S). Evidence: none found. Recommendation: a bottom-bar corner affordance plus hotkey that minimizes everything and restores it (store the HWND list).

- **PRD-27 — Window layout snapshot and restore** (Med, L). Evidence: depends on PRD-22/23; workspace restoration is reserved. Recommendation: capture process path, args when available, window rect, monitor and state. Restore by matching existing windows first and launching missing apps second.

#### Taskbar

- **PRD-28 — Win+1..9 / Alt+1..9 positional activation** (High, S). Evidence: `Alt+1` is already taken by Stack Browser (README §Keyboard shortcuts). Pins and tiles are ordered (`taskbarPins.ts`, `taskbarWindows.ts`). Recommendation: `Win+N` behaviour (launch or cycle the Nth pinned/tile app) through the existing hook. It needs a deliberate decision about Win-key ownership, since `windows_key_hook.rs` already uses `WH_KEYBOARD_LL`.

- **PRD-29 — Jump lists and recent files per app** (Med, M). Evidence: no jumplist/`IApplicationDocumentLists` usage (the grep for "Recent" only matches `speech_runtime.rs`). Recommendation: right-click a tile or pin to show the app's recent documents through `IApplicationDocumentLists` / `AutomaticDestinations` (read-only), with pin-to-list support.

- **PRD-30 — Taskbar progress and media thumbnail buttons** (Med, M). Evidence: there is no `ITaskbarList3` interception. A replacement shell does not receive `SetProgressValue` calls automatically. Recommendation: progress is hard to get without Explorer, so treat it as research. Media buttons are achievable through SMTC (PRD-33): show prev/play/next on the preview of the app that owns the media session.

- **PRD-31 — Middle-click and drag-reorder tiles** (Med, S). Evidence: tile pointer handling lives in `taskbarTilePointer.ts`. Recommendation: middle-click opens a new instance (shift-launch semantics), and drag reorders pinned and running tiles with persisted order.

- **PRD-32 — Notification center and DND** (High, M/L). Evidence: `UserNotificationListener` is already accessed, but only for counts (`task_windows/notifications.rs:6-27`). Recommendation: since the listener already has access, render a history panel (app icon, title, body, time) with dismiss, clear and "open app". Add a DND/Focus toggle that suppresses JasonShell banners and chime, and optionally set Windows Focus Assist via its settings deep link. Merge JasonShell's internal events into the same feed (PRD-18, snip saved, push complete).

#### Status widgets and media

- **PRD-33 — Media now-playing and SMTC controls** (High, M). Evidence: there is no `GlobalSystemMediaTransportControlsSessionManager` usage anywhere. Audio is volume-only (`audio.rs:68-123`). Recommendation: a top-bar media widget with title, artist, artwork and prev/play/next that expands into the Audio panel. Provide hotkeys, plus a "pause media when dictating" integration with speech (Ctrl+D).

- **PRD-34 — Top-bar status widgets (battery, network, CPU/RAM, disk)** (High, M). Evidence: System Info already maintains native cache snapshots on 2 s/15 s/60 s cadences (README §Process manager), but they are admitted only for the visible, focused Process Manager (`process_manager/system_info.rs`). There is no battery reading (README: "no battery"). Recommendation: a lightweight always-on telemetry tier (CPU %, RAM %, net up/down, battery %/charging via `GetSystemPowerStatus`) feeding compact widgets. Clicking a widget opens Process Manager on the right section. Respect the existing "unavailable stays unavailable" honesty rule.

- **PRD-35 — Metrics history and sparklines** (Med, S/M). Evidence: README says "System Info has no battery/history/charts". Recommendation: a ring buffer (e.g. 5 min at 2 s) for CPU/RAM/GPU/net shown as sparklines in System Info and widgets. Per-process CPU history in a row tooltip helps answer "what spiked?".

- **PRD-36 — Weather / calendar events / world clock widgets** (Med, M). Evidence: the calendar is a 118-line month grid (`CalendarPanelSurface.svelte`); the clock has settings (`SettingsPanelSurface.svelte:725`). Recommendation: add world clocks and timers first (offline and private), then ICS URL or Outlook local calendar events with a "next meeting in 12 min" top-bar chip. Weather comes last, as opt-in with an explicit provider, since it needs network access.

#### Stack Browser

- **PRD-37 — Quick Look / Peek preview pane** (High, M). Evidence: Markdown preview and text editor exist (`StackMarkdownPreview.svelte`, `StackTextEditor.svelte`); there is no image, PDF or media preview in `StackPopupSurface.svelte`. Recommendation: Space toggles a preview pane for images (`<img>` via asset protocol, bounded size), PDF (WebView2 renders PDF natively), audio/video (`<video>`), code (read-only CodeMirror), archives (listing via the existing extract code path), and a hex view for binaries. Shell `IPreviewHandler` is a later phase.

- **PRD-38 — Tabs and dual-pane** (Med, M/L). Evidence: the surface is single-location (`StackPopupSurface.svelte` is 3739 lines with one path state). Recommendation: tabs first (Ctrl+T/W, middle-click a folder to open it in a tab), then an optional split pane with drag-between panes that reuses the existing local-drop move and undo.

- **PRD-39 — Git workbench depth: hunk staging, blame, conflicts, amend** (High for dev workflows, M/L). Evidence: grep in `StackGitPanel.svelte` returns 0 hits for blame, cherry, rebase and amend; "conflict" appears 5 times. Commit/push/fetch/pull/stash/branches exist. The context-menu entries are named "Show git diff plan", "Stage file plan" and "Restore file plan" (`stackPopupViewModel.ts:663-687`), which suggests plan-only actions. Recommendation: (1) stage and unstage hunks/lines from the diff; (2) commit amend and "undo last commit (soft)"; (3) a merge-conflict view with ours/theirs/both per hunk; (4) blame gutter in the editor; (5) per-file history. Also verify and rename the "plan" menu items if they really execute.

- **PRD-40 — Compress, bulk rename, folder sizes** (Med, S/M). Evidence: Extract and Extract-here labels exist, but there is no "Compress" label (label grep). Recommendation: Compress to zip (Rust `zip` crate); bulk rename with a pattern and live preview (PowerRename-style, regex plus counters, undoable through the recovery journal model); on-demand folder size calculation with cancel.

- **PRD-41 — Recycle Bin choice and recovery browser** (Med, S/M). Evidence: deletes go to `%LOCALAPPDATA%\com.jnev1.jasonshell\stack-deleted` with manual recovery via `manifest.json` (README §Stack Browser). There is no UI beyond single-step undo, and undo tracks only the latest action. Recommendation: (a) a "Deleted items" view listing journal manifests with restore and purge actions; (b) a retention policy (size or age), because that folder currently grows unbounded unless purge exists elsewhere; (c) an optional "Use Windows Recycle Bin" setting (`IFileOperation` with `FOFX_RECYCLEONDELETE`).

- **PRD-42 — Favorites sidebar and recent locations** (Med, S). Evidence: there are Stack pins (`stack_popup/pins.rs`) and breadcrumbs; I found no recents list. Recommendation: a collapsible left rail with pins, recents, drives and workspace roots. Recents feed search via frecency (PRD-06).

- **PRD-43 — "Open in Stack" from everywhere** (Med, S). Evidence: Stack opens "from a pinned folder" (README §Practical workflows). The terminal has "Reveal cwd in Files" (`terminalActions.ts:57`). Recommendation: make Stack the reveal target for search file/folder results (action panel), snip saves (reveal saved PNG), Quick Command artifacts (`artifactLocation` exists in `settings.rs:234`) and Process Manager "open file location".

#### Terminal

- **PRD-44 — Session and layout restore** (High, M). Evidence: `persistentTerminal.ts` starts a session; there is no serialization of tabs, splits or scrollback. Recommendation: persist tab and split layout, cwd, profile and title, plus optional scrollback (bounded and opt-in, because of secrets). On restart, recreate the panes and print a "restored from <time>" marker.

- **PRD-45 — Terminal palette tied to themes** (Med, S/M). Evidence: README says there is "no terminal palette editor". Themes are a token system (`themePalette.ts`, `builtinThemePalettes.generated.ts`), and the built-in themes are editor themes (Monokai, Nord, Dracula, Solarized; `themes.ts:34-41`) that have canonical 16-color ANSI palettes. Recommendation: derive xterm ANSI colors from the theme, with an override section in the theme editor.

- **PRD-46 — Command palette for terminal and shell actions** (Med, S). Evidence: 20 terminal actions are already declared with labels and shortcuts (`terminalActions.ts:50-70`). Recommendation: Ctrl+Shift+P inside the terminal lists `enabledTerminalActions()`. Better still, contribute the same actions to global search when the terminal is focused (PRD-04).

- **PRD-47 — Broadcast input and SSH profiles** (Low/Med, M). Evidence: there are three profiles (README §Persistent terminal panel). Recommendation: an SSH host list parsed from `~/.ssh/config` as profiles and search results, plus "type into all panes" for split sessions.

#### Speech and AI

- **PRD-48 — Voice commands** (High, M). Evidence: speech always pastes the transcript into the captured target (`speech_target.rs:1-20`). The automation intent model already defines safe verbs with security levels (`automation.rs:41-74`). Recommendation: a command mode (hold a modifier with Ctrl+D, or a wake prefix like "shell, …") that routes the transcript to a small grammar. Examples: "open terminal", "search for X", "run quick command build", "snip", "volume 40". Mutating and destructive levels require confirmation through the existing validator.

- **PRD-49 — Speech history persistence and quality controls** (Med, S/M). Evidence: history is per process session (`speech.rs:10` comment "per process-session history entry"); README says "Audio and transcripts are not newly persisted". Recommendation: opt-in persisted history (searchable), a custom vocabulary and replacement list (e.g. "jason shell" → "JasonShell"), and formatting modes (raw, sentence case, code identifier, bullet list).

- **PRD-50 — Local LLM integration (optional, local-first)** (Med, L). Evidence: there is no LLM integration (no openai/ollama/anthropic references beyond false-positive substrings). An ONNX runtime is already shipped for Parakeet (`speech_runtime.rs`, `speech_model*.rs`). Recommendation: a pluggable "assistant" provider that talks to a user-configured local endpoint (Ollama/llama.cpp OpenAI-compatible) first. Uses: (a) natural-language to command in terminal and search ("find files over 1 GB modified this week" → Everything query or PowerShell); (b) explain the last terminal error, building on `terminalQuickFixes.ts`; (c) a commit-message draft from the staged diff in the Git panel; (d) dictation cleanup. It must never auto-execute generated commands, and should show them as an editable Quick Command draft.

#### Snipping

- **PRD-51 — OCR (Text Extractor)** (High, M). Evidence: the snip pipeline already produces a frozen bitmap and binary per-monitor image responses (`snipping/capture.rs`, `session.rs`); README says "no annotations". Recommendation: "Copy text" in the preview, using `Windows.Media.Ocr.OcrEngine` (built-in, offline, through the `windows` crate already in use). Add a dedicated "snip-to-text" hotkey that skips the preview.

- **PRD-52 — Pin snip to screen and color picker** (Med, S/M). Evidence: the preview is already a non-activating window. Recommendation: "Pin" keeps the snip as an always-on-top, draggable, resizable, click-through-toggle window (ShareX/Snipaste). Add a color picker mode on the frozen overlay (hover shows hex/RGB with a magnifier, click copies it), which is essentially free because the frozen pixels are already there.

- **PRD-53 — Annotations, capture modes, history** (Med, M/L). Evidence: README lists no annotations, autosave or capture history. Recommendation: arrow, box, highlight, blur/pixelate, text and numbered steps on a canvas in the preview. Add window and fullscreen modes (click a window to capture its rect), a delay timer, and an opt-in history folder with a "recent snips" search scope.

- **PRD-54 — Screen recording (GIF/MP4)** (Low/Med, L). Evidence: none. Recommendation: Windows.Graphics.Capture with Media Foundation encoding for region recordings. Treat it as a later bet.

#### Settings, onboarding, theming

- **PRD-55 — First-run tour and hotkey cheat sheet** (High, S/M). Evidence: there is no onboarding surface; the only entry points are the bootstrap script and README; eight global hotkeys are undiscoverable without reading Settings. Recommendation: a first-run card sequence (bars, Ctrl+Space, Alt+\`, Alt+1, Ctrl+D with speech model import, Alt+S), plus a persistent "?" overlay listing current bindings from `settings.hotkeys`. Add a "what's new" card driven by changelog version.

- **PRD-56 — Settings export/import and backup** (High, S). Evidence: settings are one JSON document `ShellSettings` (`settings.rs:20-37`), and there is a "JSON shell settings" section (`SettingsPanelSurface.svelte:765`). Themes live in `jasonshell.themes.v1` (README §Custom themes). Recommendation: one-click export (settings, themes, Quick Commands, workspaces) to a `.jasonshell.json` bundle, import with diff preview and conflict choices, and automatic rolling backups (last N) before each strict Save.

- **PRD-57 — Theme import/export and accent sync** (Med, S). Evidence: README says "There is no terminal palette editor or theme import/export surface". There is no wallpaper or accent reading (grep for wallpaper/accent/DwmGetColorizationColor/Mica returns nothing). Recommendation: export and import a single theme as JSON (validated by `validateCustomTheme` in `themePalette.ts`), plus a "Follow Windows accent color" and "Generate theme from wallpaper" option (k-means on a downscaled wallpaper).

- **PRD-58 — Clarify Control Plane vs Settings** (Med, S). Evidence: Control Plane is titled "Settings & Developer Dashboard" (`ControlPlaneSurface.svelte:92`) and duplicates theme selection (`:120-140`). Settings is the separate main configuration surface. Recommendation: rename it "Workspaces & Dev Dashboard", drop the duplicate theme selector, and make it the home of PRD-20, PRD-16 and provider health.

- **PRD-59 — Autostart and "replace Explorer shell" mode toggles** (Med, M). Evidence: launch is `npm run tauri dev` or bootstrap (README §Later runs); I found no autostart setting. Recommendation: a "Start with Windows" toggle (Run key or Task Scheduler) and a documented "hide Windows taskbar while JasonShell runs" option. Shell replacement (Winlogon `Shell`) should stay a manual, documented power-user path behind explicit consent.

- **PRD-60 — Packaging and updates** (Med, M). Evidence: the run path is a dev server. Recommendation: a signed MSI/NSIS build through Tauri bundler, plus the Tauri updater pointed at a private GitHub release feed. This is what actually turns the prototype into a daily driver.

#### Process manager

- **PRD-61 — Startup apps and services tabs** (Med, M). Evidence: there are two tabs, Processes and System Info (`ProcessManagerSurface.svelte:476`). Recommendation: a Startup tab (Run keys, Startup folder, scheduled logon tasks, with enable/disable that mirrors Task Manager's `StartupApproved` semantics) and a read-only Services tab with start/stop behind elevation prompts.

- **PRD-62 — Process row actions beyond end** (Med, S). Evidence: the only row action is guarded end (`processManagerUxState.ts:267-283`). Recommendation: open file location (in Stack), copy path/command line, search online, show windows (jump to tile), and "find what's locking a file" (Restart Manager API) launched from a Stack file menu.

---

### (3) Creative big bets

**BB-01 — Context Engine: "workspaces that actually switch your computer".**
Workspaces become the spine of the shell. Activating "jasonshell-dev" scopes search to the repo and its tasks, sets Stack pins, opens the terminal layout (tabs and splits at the right cwds), offers startup tasks as one-click chips, restores window layout (VS Code left two-thirds, browser right third), sets the audio device and focus mode, and shows the workspace name and Git branch on the top bar. Deactivating parks those windows (minimizes them, or moves them to a virtual desktop) and swaps in the next context. Path: PRD-20 (editor) → PRD-21 stages 1–3 (no window control needed) → PRD-22/27 (snap and layout snapshot) → optional virtual desktop binding (PRD-24). Most of the data model already exists in `workspaces.rs`; the missing pieces are execution and UI.

**BB-02 — Omnibar: Raycast-class launcher with an extension SDK.**
Turn the centered search into the product's front door, with providers, prefixes, per-result action panels, inline "detail" views (calculator, clipboard preview, file preview, process sparkline), and a constrained extension model. Extensions are declarative manifests plus sandboxed scripts (e.g. Deno or QuickJS, or out-of-process executables speaking JSON over stdio with the existing budget fields `max_results`/`timeout_ms` from `providers.rs:48-62`). They return result lists and declare actions from a whitelist (open URL, copy text, run Quick Command, open path). The `arbitrary_plugin_execution_allowed: false` contract field becomes a real capability system: each extension declares network, file-read and process capabilities, and the user approves them once. Path: PRD-01..09 first → stdio provider protocol → manifest and permissions → a local "extensions" folder (no marketplace initially).

**BB-03 — Shell-wide timeline ("what was I doing?").**
A privacy-first local activity journal recording which windows were focused (process, title hash or title), files opened through Stack, terminal commands with cwd and exit code, Git commits, snips and dictations, all with timestamps. Search scope `when:` ("yesterday afternoon", "last Tuesday repo X") and a scrubbable timeline panel let you reopen files, rerun commands, or restore the workspace as it was. Path: bounded, opt-in SQLite store; event sources already exist (`terminalHistory.ts`, Quick Command history, Stack operations, task windows foreground tracking); retention and per-source toggles in Settings.

**BB-04 — Local AI copilot with "never auto-run" ergonomics.**
A local-model-first assistant (Ollama / llama.cpp endpoint, optional cloud by explicit config) integrated at the points where it helps. In the terminal, "explain this error" (extending `terminalQuickFixes.ts`) and "# describe what you want" producing a command draft. In Git, commit message drafts and PR summaries. In search, natural-language file queries compiled to Everything syntax. For speech, "dictate then rewrite" (formal, bullet list, code comment). For snips, OCR then summarize or translate. Every generated command lands as an editable Quick Command draft with a risk label from the automation security levels (`automation.rs:70-74`), and nothing executes without Enter. Path: provider abstraction → three integration points (terminal, Git, search) → speech/snip later.

**BB-05 — Voice-first shell control.**
Combine the existing on-device Parakeet ASR with the automation verb set into a hands-free mode: hold Ctrl+D plus Shift for commands, or say "Hey shell". It understands app launches, window snapping ("put Chrome on the right"), media ("pause", "next"), Quick Commands ("run deploy staging" with parameter filling by voice), and Stack navigation ("open downloads"). Confirmation prompts can be spoken ("yes"/"cancel"), and the orb shows the recognized intent before execution. Path: PRD-17 verbs → PRD-48 grammar → PRD-22/33 actions → optional intent classification via local LLM (BB-04).

**BB-06 — Tiling-aware taskbar (GlazeWM-lite built into the shell).**
An opt-in tiling mode per workspace or monitor, with master/stack and columns layouts and gaps. Focus moves with Alt+H/J/K/L. The bottom bar shows a mini-map of the tile arrangement, and dragging tiles on the mini-map rearranges windows. Floating exceptions are configured per app. Because JasonShell already owns AppBar work areas and window enumeration (`task_windows/windows.rs`), it can tile within the reserved work area precisely. Path: PRD-22 → zone engine (PRD-23) → event-driven re-layout on window create/destroy (native hooks exist in `task_windows/native_hooks.rs`) → per-app rules.

**BB-07 — Live "Peek" everywhere.**
One universal preview service: Space on any file in Stack or any file result in search, hover over a snip in history, a Quick Command artifact, or a Git diff file all open the same preview window (images, PDF, media, code with syntax highlight, Markdown, archives, CSV as a table, JSON tree, fonts, hex). Pinned peeks float as reference windows. Path: PRD-37 implementation extracted into a shared surface; register `IPreviewHandler` fallback for Office formats later.

**BB-08 — Status bar widget platform (YASB/Zebar inside JasonShell).**
A widget API for the top bar: declarative JSON widgets bound to built-in data sources (CPU, RAM, GPU, net, disk, battery, media, calendar next event, Git branch of the active workspace, Quick Command last status, clock and world clocks, weather opt-in), each with click, scroll and right-click actions mapped to automation verbs. Users drag widgets into the existing reorderable control strip (`topBarControls.ts`). Advanced users write small Svelte or HTML widgets loaded from a folder in a sandboxed iframe that gets data only through a postMessage bridge. Path: PRD-34 data tier → built-in widgets → JSON widget schema → custom widget sandbox.

**BB-09 — Unified notification and activity center.**
A right-edge slide-out combining Windows toasts (through the `UserNotificationListener` access already granted, `task_windows/notifications.rs`), JasonShell internal events (Quick Command finished/failed, long terminal command done, Git push result, snip saved, model installed), and media and quick toggles (DND, mic mute, dark mode, audio output device). It works like the Win11/macOS Control Center but is keyboard-addressable and searchable, with rules ("mute Slack while workspace = focus", "only show failures for builds"). Path: PRD-32 → toggles → rules engine reusing PRD-15 triggers.

**BB-10 — "Shell as code": declarative config, Git-synced.**
Everything (settings, themes, Quick Commands, workspaces, hotkeys, widgets, search providers) is round-trippable to a human-readable `~/.jasonshell/` folder (TOML/JSON) with schema validation and hot reload. Users can version it in Git, sync it between PCs, and share theme and workspace packs. Settings UI and files stay two views of one truth, with the existing revision-conflict rejection (README §Custom themes) generalized. Path: PRD-56 export/import → file-backed store with a watcher (reuse `folder_watch`) → JSON Schema published for editor autocompletion → optional Git sync command in the Control Plane.

**BB-11 — Theme marketplace-lite and dynamic themes.**
Theme packs (shell tokens plus terminal ANSI palette plus CodeMirror theme plus orb colors) importable from a URL or a file, with a gallery and live preview using the existing preview sync (`themePreview.ts`). Dynamic themes switch light/dark at sunset or follow the Windows app mode. "Wallpaper-derived" themes generate a palette from the current wallpaper and validate contrast with the existing guidance. Path: PRD-45 and PRD-57 → pack format → gallery (a curated GitHub repo index, no server).

**BB-12 — Smart clipboard and "transform" actions.**
Clipboard history (PRD-10) plus transforms on any item: JSON pretty or minify, base64 encode/decode, URL decode, case conversions, strip formatting, CSV to Markdown table, OCR an image item, translate (local LLM), and "paste as file" (save the clipboard image to the current Stack folder). Transforms are also search actions on any selected text ("transform selection" hotkey grabs the selection via Ctrl+C simulation into the target-safe path). Path: PRD-10 → transform registry (pure functions, easy to test) → selection capture hotkey.

**BB-13 — Developer cockpit: repos, PRs and CI at a glance.**
Building on the Stack Git workbench and the Control Plane `gitStatuses` (`ControlPlaneSurface.svelte:31`), a cockpit listing all workspace repos with branch, dirty count, ahead/behind, last CI status and open PRs (via the user's `gh` CLI, without storing tokens). Top-bar chips flag failing CI on the active workspace, and one key opens the PR in the browser or the diff in Stack. Path: workspace repo list → batched `git status` (dev_tools/git_status.rs exists) → optional `gh` provider with explicit opt-in.

**BB-14 — Focus sessions.**
A Pomodoro and deep-work mode that hides the taskbar's non-workspace tiles, enables DND, sets a top-bar countdown, blocks chosen apps from stealing focus (attention flashing suppressed via `task_windows/attention.rs`), and records the session to the timeline (BB-03). Path: timer widget → DND (PRD-32) → tile filtering → stats.

---

### (4) Suggested roadmap phases

**Phase 0 — Ship-quality foundation (1–2 weeks).** These unblock daily use and cost little.
- PRD-55 first-run tour and hotkey cheat sheet
- PRD-56 settings export/import and rolling backups
- PRD-41 recovery browser and retention for `stack-deleted` (avoids unbounded growth)
- PRD-58 Control Plane rename and dedupe
- PRD-60 packaging and autostart (with PRD-59)

**Phase 1 — Make search the hub (2–4 weeks).** All S/M items, mostly additive providers inside existing contracts.
- PRD-01 calculator, PRD-02 web and bookmarks, PRD-03 Quick Commands in search, PRD-05 prefixes, PRD-08 JasonShell settings deep links, PRD-09 system verbs
- PRD-04 action panel (enables PRD-07, PRD-22 surfacing, PRD-43)
- PRD-06 frecency
- PRD-19 save terminal command as Quick Command, PRD-18 long-command notifications

Dependency: PRD-04 should land before PRD-22/43 so the new verbs have a home.

**Phase 2 — Daily-driver polish (4–6 weeks).**
- PRD-33 SMTC media, PRD-34 top-bar status widgets, PRD-35 sparklines
- PRD-10 clipboard history (then PRD-11 snippets, PRD-12 emoji)
- PRD-37 Peek preview, PRD-42 favorites and recents, PRD-40 compress and bulk rename
- PRD-51 OCR, PRD-52 pin snip and color picker
- PRD-28 Win+N activation, PRD-31 middle-click and reorder
- PRD-44 terminal session restore, PRD-45 terminal palette from theme

Dependency: PRD-34 needs a new always-on telemetry tier separate from the Process Manager admission rule.

**Phase 3 — Power-user automation (6–10 weeks).**
- PRD-17 wire automation CLI and single-instance forwarding (prerequisite for BB-05, BB-08 actions, and external integrations)
- PRD-13 parameters, PRD-14 user hotkey slots, PRD-15 triggers, PRD-16 unify task runner
- PRD-20 workspace editor, PRD-21 activation stages 1–3
- PRD-32 notification center and DND
- PRD-22 snap actions, PRD-26 show desktop, PRD-24 virtual desktop filtering
- PRD-39 Git hunk staging, amend, conflicts, blame
- PRD-48 voice commands (needs PRD-17 verbs)

**Phase 4 — Big bets (quarter+).**
- BB-01 Context Engine (needs PRD-20/21/22/27)
- BB-02 Omnibar extension SDK (needs Phase 1 complete)
- BB-06 tiling (needs PRD-22/23), PRD-25 multi-monitor bars
- BB-04 local AI copilot, BB-05 voice-first control
- BB-03 timeline, BB-09 activity center, BB-10 shell-as-code, BB-08 widget platform, BB-11 theme packs, BB-12 clipboard transforms, BB-13 developer cockpit, BB-14 focus sessions
- PRD-53 annotations, PRD-54 recording, PRD-38 tabs and dual pane, PRD-61 startup and services tabs

#### Key dependency graph (condensed)

```
PRD-04 action panel ──► PRD-07 window switcher, PRD-22 snap (surfacing), PRD-43 reveal-in-Stack
PRD-17 automation CLI ──► PRD-48 voice cmds ──► BB-05
                      └─► BB-08 widget actions, external tools (Stream Deck/AHK)
PRD-14 user hotkeys ──► per-QuickCommand / per-workspace / per-window-action keys
PRD-22 snap ──► PRD-23 zones ──► PRD-27 layout restore ──► BB-01 Context Engine
                              └─► BB-06 tiling
PRD-20 workspace editor ──► PRD-21 activation ──► BB-01
PRD-16 unify runners ──► PRD-15 triggers ──► BB-09 rules
PRD-10 clipboard ──► PRD-11 snippets, BB-12 transforms
PRD-34 telemetry tier ──► PRD-35 sparklines, BB-08 widgets
PRD-32 notif center ──► BB-09, BB-14
PRD-56 export/import ──► BB-10 shell-as-code ──► BB-11 theme packs
```

#### Quick wins vs big bets summary

Quick wins (S, high or medium value): PRD-01, 02, 03, 05, 08, 09, 12, 19, 26, 28, 31, 42, 43, 46, 55 (S/M), 56, 57, 58, 62.

Big bets (L): PRD-21, 23, 25, 27, 38, 50, 54; BB-01..BB-14.

#### Observations on product coherence (non-feature)

- **Three overlapping "run a thing" systems**: Quick Commands (`quick_commands.rs`), the workspace task runner (`dev_tools/task_runner.rs`), and an untyped `task_history: Vec<Value>` in settings (`settings.rs:33`). Users see one, while maintainers carry three. Unify them (PRD-16).
- **Two overlapping configuration surfaces**: Settings and the Control Plane both select themes. Pick one owner (PRD-58).
- **Contract-first scaffolds look finished in code but deliver nothing to the user**: Calculator, Web, Bookmarks, workspace restoration, automation forwarding, and the provider registry. Either implement them (Phase 1/3) or mark them clearly in code as reserved so future agents don't assume they work.
- **Hotkeys are a fixed 8-slot struct**, which will not scale to the features above. Generalizing it (PRD-14) is a structural prerequisite for most power-user features.
- **The honesty discipline is a product strength.** "Unavailable stays unavailable" and the explicit live-verification gates in the README make the shell trustworthy. New features (widgets, AI, notifications) should keep the same explicit-state UX.
