# wmux Feature Inventory for Future JasonShell Terminal Improvements

Purpose: technical research notes from `C:/dev/wmux` for later JasonShell planning. This is not an implementation plan; it is a feature inventory with transfer notes for a Tauri/Svelte/Rust JasonShell terminal.

Primary wmux sources inspected:

- `C:/dev/wmux/README.md`
- `C:/dev/wmux/src/shared/types.ts`
- `C:/dev/wmux/src/main/*`
- `C:/dev/wmux/src/renderer/*`
- `C:/dev/wmux/src/shell-integration/*`
- `C:/dev/wmux/resources/cli/*`
- `C:/dev/wmux/resources/wmux-orchestrator/*`
- `C:/dev/wmux/docs/superpowers/specs/*`

## Product shape and architecture

wmux is an Electron + React + TypeScript + xterm.js + node-pty terminal multiplexer for Windows. It is explicitly framed as a visibility layer for AI coding agents: the user sees terminals, browser actions, agent status, shell state, notifications, and workspace metadata in one cockpit.

- Main process responsibilities:
  - PTY spawning and lifecycle via `node-pty`/ConPTY in `src/main/pty-manager.ts`.
  - Named pipe API in `src/main/pipe-server.ts`.
  - JSON-RPC command dispatch in `src/main/index.ts`.
  - CDP browser bridge/proxy in `src/main/cdp-bridge.ts` and `src/main/cdp-proxy.ts`.
  - Claude Code hook/context/plugin installation in `src/main/claude-context.ts`.
  - Passive Claude terminal output observer in `src/main/claude-observer.ts`.
  - Session persistence in `src/main/session-persistence.ts`.
  - Theme/config import in `src/main/theme-loader.ts`, `src/main/config-loader.ts`, and `src/main/user-config.ts`.
  - Git/PR/port polling in `src/main/git-poller.ts`, `src/main/pr-poller.ts`, and `src/main/port-scanner.ts`.
- Renderer responsibilities:
  - React/Zustand workspace and split state.
  - xterm terminal creation and addon management.
  - Recursive split-pane rendering.
  - Sidebar, settings, notification panel, command palette, tutorial, browser panel, diff/markdown surfaces.
- JasonShell transfer note:
  - JasonShell already has Tauri/Svelte/Rust + ConPTY/xterm terminal surfaces. The main lesson is not the Electron stack, but the data model: durable workspaces, binary split trees, stable terminal IDs, visible agent status, and scriptable control API.

## 1. Workspace model as durable terminal desktops

Source anchors:

- `src/shared/types.ts` — `WorkspaceInfo`, `SplitNode`, `SurfaceRef`, `SavedSession`.
- `src/renderer/App.tsx` — startup restore, workspace rendering, autosave, zoom projection.
- `src/renderer/store/workspace-slice.ts` — workspace CRUD.
- `src/renderer/components/Sidebar/*` — workspace list, drag reorder, context menu.

Feature specifics:

- A workspace is not just a tab. It contains:
  - `id`, `title`, `customColor`, `pinned`.
  - A full `splitTree` layout.
  - Terminal defaults such as `shell` and last `cwd`.
  - Operational metadata: `gitBranch`, `gitDirty`, `prNumber`, `prStatus`, `ports`, `shellState`, `notificationText`, `browserUrl`.
  - Unread count for notifications.
- wmux keeps inactive workspaces mounted and hides them with CSS (`visibility: hidden`, pointer events disabled) instead of destroying terminal components.
- Workspace switching therefore preserves xterm instances, scrollback, pane layout, tab selection, and running PTYs.
- Sidebar supports:
  - Drag-reorder workspace rows.
  - Pin/unpin.
  - Rename.
  - Color presets.
  - Close others.
  - Mark read/unread.
  - Status-rich row rendering.

JasonShell improvement ideas:

- Add a first-class terminal workspace concept instead of only one persistent terminal panel session list.
- Represent each workspace as a named terminal desktop with its own pane tree and metadata.
- Keep terminal workspace surfaces mounted/attached where practical to avoid losing xterm state on navigation.
- Make the workspace row an operational dashboard: cwd, branch, dirty marker, current process/agent state, unread notifications, and running port hints.

## 2. Binary split tree for pane layout

Source anchors:

- `src/shared/types.ts` — `SplitNode` type.
- `src/renderer/store/split-utils.ts` — immutable tree operations.
- `src/renderer/components/SplitPane/SplitContainer.tsx` — recursive renderer.
- `src/renderer/components/SplitPane/SplitDivider.tsx` — drag resizing.

Feature specifics:

- Layout is a serializable binary tree:
  - Leaf: `{ type: 'leaf', paneId, surfaces, activeSurfaceIndex }`.
  - Branch: `{ type: 'branch', direction: 'horizontal' | 'vertical', ratio, children: [leftOrTop, rightOrBottom] }`.
- Split operations replace one leaf with a branch containing old and new leaves.
- Close operations collapse the sibling back upward when a leaf is removed.
- Resize operations update only the matching branch ratio.
- Ratio is clamped, so panes never become unusably tiny.
- Divider double-click resets a branch ratio to 50/50.
- A derived grid layout can be generated for external `layout.grid` commands.

JasonShell improvement ideas:

- Use a serializable binary split-tree for any future terminal workbench rather than ad hoc nested flex state.
- Store pane ratios as logical floats, not pixel widths, so sessions restore well after window resize or monitor changes.
- Make split/close/resize operations pure and testable; keep backend terminal lifetime separate from frontend layout mutation.

## 3. Pane zoom without mutating layout

Source anchors:

- `src/renderer/App.tsx` — `zoomedPaneId` logic.
- `src/renderer/store/split-utils.ts` — `findLeaf`.
- `src/renderer/hooks/useKeyboardShortcuts.ts` — `Ctrl+Shift+Enter`.

Feature specifics:

- Zoom state is a separate renderer value, not a layout mutation.
- When a pane is zoomed, wmux renders only that leaf as a projection of the original split tree.
- The original split tree remains unchanged and returns intact when zoom is toggled off.
- Zoom state is cleared if the pane disappears.

JasonShell improvement ideas:

- Implement terminal pane maximize as a view projection rather than rewriting split state.
- Preserve split layout exactly across zoom toggles.
- Add a keyboard shortcut and toolbar affordance for zooming the active terminal pane.

## 4. Surface tabs inside each pane

Source anchors:

- `src/shared/types.ts` — `SurfaceRef` and `SurfaceType`.
- `src/renderer/store/surface-slice.ts` — add/close/move/reorder/rename/split-and-move.
- `src/renderer/components/SplitPane/PaneWrapper.tsx` — tab keep-alive rendering.
- `src/renderer/components/SplitPane/SurfaceTabBar.tsx` — tab bar UI.

Feature specifics:

- A pane can contain multiple surfaces, with one active surface index.
- Surface types include terminal, browser, markdown, and diff.
- Surface metadata can include:
  - Custom title.
  - Shell type.
  - Cwd.
  - Per-surface color scheme.
  - Agent label and agent ID.
- All surfaces in a pane are rendered simultaneously but hidden when inactive, preserving xterm sessions and browser state.
- Tabs are closeable, renamable, reorderable, and movable between panes.
- wmux has a known design caveat: some close paths kill PTY explicitly from UI, while keyboard/pipe paths can mutate store state without central PTY cleanup. JasonShell should centralize terminal close semantics.

JasonShell improvement ideas:

- Let each terminal pane contain multiple terminal tabs rather than only a flat tab strip.
- Add non-terminal surface tabs later: browser preview, logs, markdown, diff, command output.
- Tie every terminal tab to one stable backend session ID and centralize close/kill rules in one controller.

## 5. Drag/drop tabs, move-to-pane, and split-on-drop

Source anchors:

- `src/renderer/components/SplitPane/SurfaceTabBar.tsx`.
- `src/renderer/components/SplitPane/PaneWrapper.tsx`.
- `src/renderer/styles/splitpane.css`.
- `src/renderer/store/surface-slice.ts`.

Feature specifics:

- Drag payload uses a custom MIME type containing `sourcePaneId` and `surfaceId`.
- Dropping onto a tab bar can:
  - Reorder within the same pane.
  - Move to another pane.
- Dropping onto a pane body can:
  - Move the surface into the target pane center.
  - Split left/right/top/bottom when dropped near an edge.
- CSS edge zones are invisible overlays: left, right, top, bottom, and center.
- Split-on-drop creates a new leaf and moves the dragged tab into it.

JasonShell improvement ideas:

- Add browser-like terminal tab dragging inside the terminal panel.
- Use edge drop zones to split terminal tabs into panes naturally.
- Make drag payload generic enough to support terminals, browser previews, markdown/diff surfaces, and future agent dashboards.

## 6. Stable PTY lifecycle keyed by surface ID

Source anchors:

- `src/main/pty-manager.ts`.
- `src/renderer/hooks/useTerminal.ts`.
- `src/shared/types.ts`.

Feature specifics:

- wmux uses the surface ID as the PTY ID.
- Terminal renderer first checks whether a PTY already exists for the surface ID.
- If yes, renderer reattaches to existing PTY.
- If no, renderer creates a PTY with that same ID.
- React component cleanup deliberately does not kill the PTY; explicit close does.
- This makes layout changes, tab moves, hide/show, and remount safe.
- PTY creation injects env vars:
  - `WMUX=1`
  - `WMUX_SURFACE_ID`
  - `WMUX_PIPE`
  - `WMUX_CLI`
  - agent-specific vars for spawned agents.

JasonShell improvement ideas:

- Treat terminal tab ID/session ID as the backend ConPTY key.
- Ensure frontend remount/layout changes cannot accidentally kill a shell.
- Use explicit user actions or lifecycle policy to kill sessions.
- Inject JasonShell-specific environment variables so shell integrations and tools can report back.

## 7. PTY robustness and shell startup

Source anchors:

- `src/main/pty-manager.ts`.
- `src/main/shell-detector.ts`.
- `src/shell-integration/*`.

Feature specifics:

- Shell fallback chain: PowerShell 7 (`pwsh.exe`), Windows PowerShell, then `cmd.exe`.
- Shell availability is validated before spawn.
- Shell integration is loaded at shell startup.
- Long writes are split into ConPTY-friendly chunks and serialized through a write queue to avoid dropped/interleaved bytes.
- Resize calls forward xterm fit dimensions to PTY.
- CMD and PowerShell have different integration startup paths.

JasonShell improvement ideas:

- Keep JasonShell's existing ConPTY startup hardening, but add a write queue/chunking contract for long paste/automation writes if not already complete.
- Make shell integration opt-in/visible, with clear env vars and fallback behavior.
- Consider shell labels from detected process/shell state for tab title generation.

## 8. xterm behavior and addon set

Source anchors:

- `src/renderer/hooks/useTerminal.ts`.
- `src/renderer/components/Terminal/FindBar.tsx`.
- `src/renderer/components/Terminal/CopyMode.tsx`.
- `src/renderer/styles/terminal.css`.

Feature specifics:

- Addons used:
  - Fit addon.
  - WebLinks addon.
  - Search addon.
  - Unicode 11 addon.
  - Image addon.
  - Canvas renderer addon.
- wmux intentionally uses Canvas rather than WebGL due Chromium context limits when many terminals are visible.
- Mouse wheel handling distinguishes normal buffer scrollback from alternate-screen TUI behavior.
- CJK/IME composition behavior is patched for reliability.
- Resize/refit occurs when a tab/workspace becomes visible and focused.
- xterm options such as font, cursor, scrollback, and theme can update live.

JasonShell improvement ideas:

- Review JasonShell WebGL usage against multi-pane/multi-tab context pressure; provide Canvas fallback or policy.
- Add a focused terminal search bar powered by xterm SearchAddon for every terminal pane/tab.
- Ensure visibility changes trigger fit/resize/refresh without duplicating rows or losing cursor state.
- Preserve alternate-screen application behavior while still supporting normal scrollback.

## 9. Copy, paste, OSC, and image paste

Source anchors:

- `src/renderer/hooks/useTerminal.ts`.
- `src/preload/index.ts`.
- `src/main/ipc-handlers.ts`.
- README clipboard image paste feature.

Feature specifics:

- `Ctrl+C` copies xterm selection and clears it; if no selection exists, it passes through as interrupt.
- `Ctrl+V` checks for clipboard image first:
  - Saves the image to a temp file.
  - Injects the temp file path into the terminal.
  - Allows an AI agent to read screenshots copied by Win+Shift+S/Snipping Tool.
- Text paste uses xterm paste semantics to preserve bracketed paste.
- OSC 52 clipboard writes route through Electron clipboard IPC.

JasonShell improvement ideas:

- Add screenshot-to-temp-path paste as a high-value AI-terminal feature.
- Make terminal paste routing explicit for active tab/pane, preserving current JasonShell fixes around native paste suppression.
- Add a user-visible option for image paste behavior: paste path, paste markdown image link, or ask.

## 10. Keyboard shortcut system

Source anchors:

- `src/renderer/hooks/useKeyboardShortcuts.ts`.
- `src/renderer/store/settings-slice.ts`.
- `src/renderer/components/Settings/ShortcutRecorder.tsx`.
- README keyboard shortcut table.

Feature specifics:

- All documented shortcuts are rebindable from settings.
- Shortcut groups include:
  - Workspaces: new, close, rename, next/previous, jump 1-9, toggle sidebar.
  - Surfaces/tabs: new, close, next/previous, jump 1-8.
  - Split panes: split right/down, directional focus, zoom, flash focused pane.
  - Browser: toggle browser panel, devtools, console.
  - Notifications: notification panel, jump latest unread.
  - Find: open, next, previous, close.
  - Terminal: copy, paste, font zoom/reset.
  - Window: new window, settings, command palette.
- Safe interception avoids stealing common bare Ctrl combinations from terminal apps unless explicitly whitelisted.
- Directional pane focus computes candidate rectangles from the split tree.
- Shortcut recorder detects conflicts.

JasonShell improvement ideas:

- Move terminal workbench shortcuts into a user-editable settings model.
- Add directional pane focus once split panes exist.
- Keep a terminal-safe interception layer so shells, TUIs, editors, and readline keep expected key behavior.

## 11. Command palette

Source anchors:

- `src/renderer/components/CommandPalette/CommandPalette.tsx`.
- `src/renderer/App.tsx`.

Feature specifics:

- `Ctrl+Shift+P` opens a palette.
- Palette supports fuzzy matching.
- Shows actions with shortcut labels.
- Includes workspace switching entries.
- Some actions are stubs in wmux, so the feature is conceptually useful but not fully mature.

JasonShell improvement ideas:

- Extend JasonShell Quick Commands or command panel into a universal terminal command palette.
- Include workspace/pane/tab actions, recent commands, terminal profiles, saved sessions, split commands, and agent visibility actions.
- Show current shortcut beside each action.

## 12. Session persistence and named sessions

Source anchors:

- `src/main/session-persistence.ts`.
- `src/renderer/App.tsx`.
- `src/renderer/components/Sidebar/SessionMenu.tsx`.
- README Session Restore section.

Feature specifics:

- Auto-save writes to `%APPDATA%/wmux/sessions/session.json`.
- Named sessions write under `%APPDATA%/wmux/sessions/saved`.
- Writes are atomic: temp file then rename.
- Named sessions are sanitized and sorted by save time.
- Startup restore order:
  - Auto-saved session.
  - Latest named session.
  - Fresh default workspace.
- Saved data includes:
  - Window position and size.
  - Sidebar width.
  - Workspace titles and colors.
  - Split tree.
  - Working directory per terminal.
  - Shell type per terminal.
  - Browser URLs.
  - Terminal preferences.
- wmux does not restore live process state; shells are respawned fresh in saved directories.

JasonShell improvement ideas:

- Add explicit named terminal workspace snapshots.
- Persist terminal pane layout, tabs, cwd, shell profile, active tab, font/theme prefs, and browser/auxiliary surface state.
- Decide early whether JasonShell will restore live processes, respawn shells, or support attaching to long-lived backend sessions.

## 13. Shell integration and metadata reporting

Source anchors:

- `src/shell-integration/wmux-powershell-integration.ps1`.
- `src/shell-integration/wmux-bash-integration.sh`.
- `src/shell-integration/wmux-cmd-integration.cmd`.
- `src/main/pipe-server.ts`.
- `src/renderer/App.tsx`.
- `src/renderer/components/Sidebar/WorkspaceRow.tsx`.

Feature specifics:

- Shell integrations report:
  - Current working directory.
  - Git branch.
  - Git dirty state.
  - Shell state: idle, running, interrupted.
  - PR status through GitHub CLI polling.
  - Port scan triggers.
- V1 named pipe protocol supports simple text commands:
  - `report_pwd <surface_id> <path>`
  - `report_git_branch <surface_id> <branch> [dirty]`
  - `report_shell_state <surface_id> idle|running|interrupted`
  - `notify <surface_id> <text>`
  - `ping`
- Renderer maps surface-level metadata back to workspace rows.
- Long-running commands over a threshold can trigger notifications on completion.

JasonShell improvement ideas:

- Add a JasonShell shell integration protocol independent of UI implementation.
- Surface shell state in terminal tabs and top-bar terminal button state.
- Report cwd/git/dirty/command-start/command-end through a narrow protocol.
- Add durable metadata to terminal sessions so tab titles and workspace rows update without scraping terminal output.

## 14. Git, PR, and port metadata

Source anchors:

- `src/main/git-poller.ts`.
- `src/main/pr-poller.ts`.
- `src/main/port-scanner.ts`.
- `src/shared/types.ts`.
- `src/renderer/components/Sidebar/WorkspaceRow.tsx`.
- `src/renderer/components/Sidebar/PrStatusIcon.tsx`.

Feature specifics:

- Git branch and dirty status appear inline in workspace rows.
- PR metadata appears with open/merged/closed iconography.
- Port scanner uses `netstat -ano` and coalesces burst scans.
- Browser can auto-navigate to new local development ports.
- Workspace row becomes a compact dev-environment dashboard.

JasonShell improvement ideas:

- Reuse JasonShell Stack Browser git knowledge to show terminal workspace git state.
- Add dev-port discovery for active terminal cwd/processes.
- Offer one-click local browser/open-in-stack actions when a new dev server appears.

## 15. Notification center and activity rings

Source anchors:

- `src/renderer/store/notification-slice.ts`.
- `src/main/notification-manager.ts`.
- `src/renderer/components/Titlebar/NotificationBell.tsx`.
- `src/renderer/components/Titlebar/NotificationPanel.tsx`.
- `src/renderer/components/SplitPane/PaneWrapper.tsx`.
- `src/renderer/components/Terminal/NotificationRing.tsx`.

Feature specifics:

- Notifications are stored with surface/workspace/pane IDs.
- Panes can show a blue ring when attention is needed.
- Tabs can light up when inactive surfaces receive events.
- Notification bell shows unread count.
- Notification panel lists pending notifications and jumps to the source surface.
- Native Windows toast and taskbar flash are triggered for alerts.
- Notification sources include:
  - OSC sequences.
  - CLI `wmux notify`.
  - Idle/long-command completion detection.
  - Agent completion/needs-attention state.

JasonShell improvement ideas:

- Add terminal-scoped notifications tied to session IDs and pane IDs.
- Keep top-bar terminal icon completion state, but extend it into a per-tab/per-pane notification center.
- Add jump-to-terminal action for every notification.
- Add OS toast/taskbar flash policy with user settings.

## 16. Passive Claude Code integration

Source anchors:

- `src/main/claude-context.ts`.
- `src/cli/wmux-hook.ts`.
- `src/main/claude-observer.ts`.
- `src/main/index.ts` hook handlers.
- `src/renderer/App.tsx` hook activity handling.
- `src/renderer/components/Sidebar/WorkspaceRow.tsx`.
- `resources/claude-instructions.md`.

Feature specifics:

- wmux auto-manages blocks in `~/.claude/CLAUDE.md` and `~/.claude/settings.json`.
- It injects `PostToolUse` hooks for tools such as Bash, Read, Write, Edit, Grep, Glob, Agent, WebSearch, WebFetch, and Skill.
- Hook helper sends lightweight events over the wmux pipe.
- Renderer stores per-workspace hook activity and displays current tool/agent state.
- PTY output observer also parses Claude TUI output for:
  - Running/finished subagents.
  - Tool uses.
  - Token counts.
  - Active skill.
  - Done/cost markers.
- Edit/Write hooks can auto-open or refresh a diff surface.

JasonShell improvement ideas:

- Add optional AI-harness visibility integration rather than forcing agent behavior changes.
- Prefer passive observation/events over wrapping or replacing existing tools.
- Surface current agent tool, subagent status, token/cost hints, and active skill in terminal workspace UI.
- Be careful with managed edits to user config files; require opt-in, use markers, preserve unrelated settings, and expose undo.

## 17. Visible browser / CDP automation panel

Source anchors:

- `src/renderer/components/Browser/BrowserPane.tsx`.
- `src/main/cdp-bridge.ts`.
- `src/main/cdp-proxy.ts`.
- `src/main/index.ts` browser handlers.
- README Live Browser Visibility feature.
- `docs/superpowers/specs/2026-03-26-wmux-visibility-layer-design.md`.

Feature specifics:

- Browser is an Electron webview surface that can be placed in a pane.
- Main process attaches Electron debugger to the browser webContents.
- CDP bridge supports:
  - Navigate.
  - Accessibility snapshot with stable `@eN` refs.
  - Click by ref.
  - Type/fill by ref.
  - Screenshot.
  - Text extraction.
  - JavaScript eval.
  - Wait.
  - Batch commands.
- CDP proxy exposes a Chrome-compatible endpoint on `localhost:9222`-style ports.
- Claude's `chrome-devtools-mcp` can target the proxy, so native browser actions become visible in wmux.
- If automation is requested and no browser surface exists, wmux can create one automatically.

JasonShell improvement ideas:

- Consider a terminal-adjacent browser/webview surface for AI browser visibility and local dev preview.
- Provide accessibility-snapshot refs to agents/scripts instead of raw DOM only.
- Expose browser actions through JasonShell commands/events so Quick Commands and agents can drive visible UI.
- If implemented in Tauri, map this carefully to WebView2 capabilities and security constraints.

## 18. Named pipe CLI and JSON-RPC automation API

Source anchors:

- `src/main/pipe-server.ts`.
- `resources/cli/wmux.js` and `src/cli/wmux.ts`.
- `src/renderer/pipe-bridge.ts`.
- `src/main/index.ts` V2 handlers.
- README CLI and Socket API sections.

Feature specifics:

- wmux exposes `\\.\pipe\wmux` for external control.
- Two protocols exist:
  - V1 text protocol for shell integration.
  - V2 JSON-RPC-ish protocol for CLI and automation.
- CLI commands include:
  - `ping`, `identify`, `capabilities`.
  - Workspace create/list/select/rename/close.
  - Surface create/list/focus/close.
  - Pane split/focus/close/zoom/list.
  - `send`, `send-key`, `read-screen`.
  - Browser open/snapshot/click/type/fill/screenshot/eval.
  - Agent spawn/spawn-batch/list/status/kill.
  - Notify.
  - Sidebar status/progress/log.
  - `tree` for workspace/pane/surface hierarchy.
- Renderer exposes internal `window.__wmux_*` functions that main calls to mutate UI state.

JasonShell improvement ideas:

- Add a local JasonShell automation API for scripts and AI agents.
- Split the API into:
  - Stable external protocol.
  - Internal Tauri event/command bridge.
  - UI state mutations behind typed commands.
- Expose terminal read/send/split/focus/session-tree operations.
- Keep security boundaries explicit: local-only, optional, per-instance pipe/token if needed.

## 19. Agent spawning and visible multi-agent workflows

Source anchors:

- `src/main/agent-manager.ts`.
- `src/main/index.ts` agent handlers.
- `src/renderer/App.tsx` agent update handling.
- `resources/wmux-orchestrator/*`.

Feature specifics:

- Agent manager creates PTYs for spawned agents.
- Agent env includes ID and label.
- It waits for shell readiness before writing the agent command.
- Spawned agents appear as real terminal surfaces in target panes.
- Batch spawn supports assignment strategies:
  - `distribute` balances by tab count.
  - `stack` stacks into least-loaded pane.
  - `split` is planned/fallback in wmux.
- Agent status/list/kill are exposed through the pipe API.
- Renderer receives agent updates and adds visible terminal surfaces.

JasonShell improvement ideas:

- Add a visible agent spawn API that creates terminal tabs/panes instead of hidden background tasks.
- Show agent label/status directly in tab title and workspace row.
- Provide batch spawn helpers that lay out panes before launching agents.
- Balance agents across panes based on current workload.

## 20. wmux-orchestrator plugin

Source anchors:

- `resources/wmux-orchestrator/README.md`.
- `resources/wmux-orchestrator/commands/orchestrate.md`.
- `resources/wmux-orchestrator/skills/orchestrate/SKILL.md`.
- `resources/wmux-orchestrator/scripts/*`.
- `src/main/orchestration-watcher.ts`.
- `src/renderer/components/Sidebar/OrchestrationPanel.tsx`.

Feature specifics:

- Bundled Claude Code plugin is auto-installed and enabled by wmux.
- Slash command `/wmux:orchestrate` decomposes a complex task into dependency-aware waves.
- It requires explicit user approval before spawning agents.
- It writes state to temp directories such as `wmux-orch-*`.
- Spawn scripts create a balanced grid and launch visible Claude agents in panes.
- Orchestration watcher polls state files every second.
- Sidebar shows one active orchestration with:
  - Wave progress.
  - Agent progress.
  - Reviewer state.
  - Tool counts.
  - Elapsed time.
  - Completion linger.

JasonShell improvement ideas:

- Add an orchestration dashboard surface/side panel for multi-agent runs launched from JasonShell.
- Use file-backed run state or event streams so independent agent processes can report progress.
- Treat visible terminal panes as the default execution substrate for parallel agents.
- Integrate with Fusion/subagents carefully without duplicating existing task-board semantics.

## 21. Settings store and modal

Source anchors:

- `src/main/settings-store.ts`.
- `src/renderer/store/settings-slice.ts`.
- `src/renderer/components/Settings/*`.
- `src/preload/index.ts`.

Feature specifics:

- Settings are stored under `%APPDATA%/wmux/settings.json`.
- Renderer hydrates synchronously from preload to avoid settings flash.
- Store migrates legacy localStorage values.
- Settings window uses category tabs:
  - Sidebar.
  - Workspace.
  - Terminal.
  - Notifications.
  - Browser.
  - Shortcuts.
- Shortcut recorder captures key chords and detects conflicts.
- Some wmux settings are partially wired; JasonShell should avoid shipping no-op settings.

JasonShell improvement ideas:

- Expand JasonShell settings with a terminal-workbench category.
- Add shortcut recorder and conflict detection.
- Keep settings durable in backend-owned JSON with merge-safe saves.
- Mark experimental settings clearly or hide them until wired.

## 22. Theme and color-scheme system

Source anchors:

- `resources/themes/*.theme`.
- `src/main/theme-loader.ts`.
- `src/main/config-loader.ts`.
- `src/main/user-config.ts`.
- `src/renderer/hooks/useTerminal.ts`.
- `src/renderer/components/Settings/TerminalSettings.tsx`.

Feature specifics:

- Bundles hundreds of Ghostty-style `.theme` files plus curated wmux themes.
- Theme loader parses Ghostty key/value syntax.
- Windows Terminal import reads `settings.json`, default profile, font, and color scheme.
- Ghostty import reads `~/.config/ghostty/config`.
- User config supports named color schemes in TOML.
- Per-surface color scheme overrides can be set at split/new-surface time or updated later.
- xterm theme/font/cursor settings can update live without recreating terminal.

JasonShell improvement ideas:

- Add terminal themes as data files rather than hardcoded colors.
- Import Windows Terminal and Ghostty settings to reduce migration friction.
- Support per-tab/pane color schemes for environment coding:
  - production red.
  - staging yellow.
  - dev green.
  - CI purple.
- Expose theme selection through settings, command palette, and CLI/automation.

## 23. User dotfile config

Source anchors:

- `src/main/user-config.ts`.
- `docs/config.md`.
- README Config section.
- CLI `config show|reload|path` commands.

Feature specifics:

- wmux reads `~/.wmux/config.toml`.
- Terminal settings can be expressed as TOML.
- Named color schemes can be defined by users.
- Parser maps kebab-case and camelCase variants.
- Parse failures are non-fatal.
- Config can be reloaded live.

JasonShell improvement ideas:

- Add optional `~/.jasonshell/config.toml` or project-local config for power users.
- Keep GUI settings as source of truth for normal users; dotfile for portable advanced config.
- Provide `show config path`, `reload config`, and diagnostics commands.

## 24. Sidebar polish and workspace status rows

Source anchors:

- `src/renderer/components/Sidebar/Sidebar.tsx`.
- `src/renderer/components/Sidebar/WorkspaceRow.tsx`.
- `src/renderer/components/Sidebar/WorkspaceContextMenu.tsx`.
- `src/renderer/components/Sidebar/UnreadBadge.tsx`.
- `src/renderer/styles/sidebar.css`.

Feature specifics:

- Sidebar can resize and collapse.
- Footer buttons cover save/load/new workspace actions.
- Workspace context menu includes pin, rename, color, move, close, mark read/unread.
- Rows show:
  - Active selection fill.
  - Custom color tint.
  - State dot.
  - Inline rename.
  - Close button.
  - Unread badge.
  - Git/cwd context line.
  - PR line.
  - Active tool/agent activity line.
- Dots communicate state:
  - Orange pulsing = working.
  - Green = done.
  - Red = interrupted.

JasonShell improvement ideas:

- If JasonShell gains a terminal workbench side surface, make rows information-dense rather than only labels.
- Use color and badges consistently across terminal tab strip, workspace row, top-bar terminal icon, and notifications.
- Add context actions that match terminal user workflows: duplicate session, split here, open folder stack, copy cwd, save workspace.

## 25. Diff and markdown surfaces

Source anchors:

- `src/main/diff-provider.ts`.
- `src/renderer/components/Diff/DiffPane.tsx`.
- `src/renderer/components/Markdown/MarkdownPane.tsx`.
- `src/main/index.ts` markdown/diff handlers.
- Hook handling in `src/renderer/App.tsx`.

Feature specifics:

- wmux supports surfaces beyond terminals/browser:
  - Markdown surface.
  - Diff surface.
- Edit/Write Claude hook events can open or refresh diff surface.
- Markdown can load file content or set content through API.
- Diff surface gives immediate visibility into agent file changes.

JasonShell improvement ideas:

- Add a diff/review surface adjacent to terminal sessions for AI coding runs.
- Auto-open diff when terminal/agent reports file edits.
- Add markdown/log surfaces for generated plans, test summaries, and command output snapshots.

## 26. First-launch tutorial and help affordance

Source anchors:

- `src/renderer/components/Tutorial/Tutorial.tsx`.
- `src/renderer/App.tsx`.
- `src/renderer/components/Settings/WorkspaceSettings.tsx`.
- README First-launch tutorial feature.

Feature specifics:

- Seven-step onboarding modal.
- Explains workspaces, splits, tabs, browser panel, and notifications.
- Shows shortcut chips.
- Progress dots indicate current step.
- First-launch state is stored so it only opens once.
- Help button can reopen tutorial.

JasonShell improvement ideas:

- Add a short terminal workbench tutorial after major terminal upgrades.
- Focus tutorial on high-value workflows:
  - Toggle terminal.
  - Split pane.
  - Create tab.
  - Save session.
  - Paste screenshot path.
  - Jump to notification.
  - Open command palette.

## 27. Multi-window lifecycle and app shell

Source anchors:

- `src/main/window-manager.ts`.
- `src/main/index.ts`.
- README Architecture section.

Feature specifics:

- wmux can create/list/focus/minimize/maximize windows.
- Window bounds are included in autosave.
- CLI/socket API includes window operations.
- Multi-window is part of the app model even if most feature work centers on one cockpit.

JasonShell improvement ideas:

- JasonShell already uses multiple Tauri webview windows for top/bottom/panels. A terminal workbench could still expose workspace movement between windows later.
- Consider detached terminal workspace windows once split-pane state is serializable.

## 28. Browser/local-dev port integration

Source anchors:

- `src/main/port-scanner.ts`.
- `src/renderer/App.tsx`.
- `src/renderer/components/Browser/BrowserPane.tsx`.

Feature specifics:

- Shell completion can trigger a port scan.
- Renderer filters dev ports and can navigate browser panel to a newly detected localhost server.
- Browser surface is both a preview tool and an AI visibility surface.

JasonShell improvement ideas:

- When a terminal starts Vite/Next/other dev server, show an unobtrusive prompt: open preview, pin port, or ignore.
- Use Stack Browser or a new preview panel as the target.

## 29. Transfer-priority matrix for JasonShell

High-value, likely synergistic with JasonShell terminal:

- Stable terminal session ID as backend ConPTY key.
- Binary split-pane tree.
- Pane-local terminal tabs.
- Drag tab to split/move.
- Named terminal workspaces/sessions.
- Shell integration metadata: cwd, command state, git state.
- Terminal-scoped notification center.
- Screenshot clipboard paste as temp-file path.
- Per-tab/pane color schemes and theme imports.
- Command palette for terminal workbench actions.

Medium-value, larger scope:

- Visible browser/CDP surface.
- Diff/markdown side surfaces.
- Agent spawn/batch spawn API.
- Orchestration dashboard.
- Dev port browser integration.
- Sidebar workspace cockpit.

Lower priority or caution:

- Auto-mutating user Claude settings and plugins should be opt-in and reversible.
- Shipping settings that are not wired creates trust issues; avoid wmux's partially implemented settings categories.
- Multiple always-mounted terminals can pressure renderer resources; measure Canvas/WebGL, memory, and event listeners.
- External local pipe/API needs clear security and instance isolation.

## 30. Suggested JasonShell planning slices

These are planning candidates, not approved tasks:

1. Terminal workbench data model
   - Define `TerminalWorkspace`, `TerminalPaneTree`, `TerminalPane`, `TerminalSurface`.
   - Keep stable session IDs independent from UI mount state.
   - Add serialization tests.
2. Split-pane terminal UI
   - Render binary tree in Svelte.
   - Add split right/down, resize, collapse-on-close, zoom projection.
   - Add directional focus.
3. Pane-local tabs and drag/drop
   - Move current flat terminal tabs into pane leaves.
   - Add drag reorder/move/split zones.
   - Centralize close/kill semantics.
4. Session persistence
   - Auto-save workspace layouts.
   - Add named save/load/delete.
   - Decide respawn-vs-reattach behavior.
5. Shell metadata protocol
   - Add PowerShell integration for cwd/git/command start/end.
   - Feed metadata into tab labels, top-bar terminal button, notifications.
6. Notification center
   - Terminal session notifications with jump-to-pane/tab.
   - OS toast/taskbar flash user policy.
7. Theme and color scheme upgrade
   - Import Windows Terminal/Ghostty.
   - Per-tab color scheme.
   - Live xterm theme updates.
8. AI visibility layer
   - Optional passive hooks/events for agent activity.
   - Diff surface on file edits.
   - Agent status row and orchestration dashboard.
9. Visible browser/dev-preview surface
   - Local dev port detection.
   - Browser preview surface.
   - Later CDP control if Tauri/WebView2 support is feasible.

## 31. Key source map

Use these wmux files as reference when planning similar JasonShell features:

- Layout/types:
  - `C:/dev/wmux/src/shared/types.ts`
  - `C:/dev/wmux/src/renderer/store/split-utils.ts`
  - `C:/dev/wmux/src/renderer/store/surface-slice.ts`
  - `C:/dev/wmux/src/renderer/store/workspace-slice.ts`
- Split UI:
  - `C:/dev/wmux/src/renderer/components/SplitPane/SplitContainer.tsx`
  - `C:/dev/wmux/src/renderer/components/SplitPane/SplitDivider.tsx`
  - `C:/dev/wmux/src/renderer/components/SplitPane/PaneWrapper.tsx`
  - `C:/dev/wmux/src/renderer/components/SplitPane/SurfaceTabBar.tsx`
- Terminal:
  - `C:/dev/wmux/src/renderer/hooks/useTerminal.ts`
  - `C:/dev/wmux/src/main/pty-manager.ts`
  - `C:/dev/wmux/src/renderer/components/Terminal/FindBar.tsx`
- Persistence/settings/themes:
  - `C:/dev/wmux/src/main/session-persistence.ts`
  - `C:/dev/wmux/src/main/settings-store.ts`
  - `C:/dev/wmux/src/main/theme-loader.ts`
  - `C:/dev/wmux/src/main/config-loader.ts`
  - `C:/dev/wmux/src/main/user-config.ts`
- Shell/metadata:
  - `C:/dev/wmux/src/shell-integration/wmux-powershell-integration.ps1`
  - `C:/dev/wmux/src/shell-integration/wmux-bash-integration.sh`
  - `C:/dev/wmux/src/main/pipe-server.ts`
- CLI/API:
  - `C:/dev/wmux/resources/cli/wmux.js`
  - `C:/dev/wmux/src/cli/wmux.ts`
  - `C:/dev/wmux/src/renderer/pipe-bridge.ts`
  - `C:/dev/wmux/src/main/index.ts`
- AI/browser/orchestration:
  - `C:/dev/wmux/src/main/claude-context.ts`
  - `C:/dev/wmux/src/main/claude-observer.ts`
  - `C:/dev/wmux/src/main/cdp-bridge.ts`
  - `C:/dev/wmux/src/main/cdp-proxy.ts`
  - `C:/dev/wmux/src/main/agent-manager.ts`
  - `C:/dev/wmux/src/main/orchestration-watcher.ts`
  - `C:/dev/wmux/resources/wmux-orchestrator/README.md`
