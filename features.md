# Cairo Shell Features

This document summarizes the user-facing shell features in `cairoshell-0.4.434-rust-rewrite`. It focuses on what each feature does and how it behaves in normal use, not on the internal implementation.

## Menu Bar

- The menu bar is the shell's top-level command surface and stays docked across the primary monitor.
- It acts as a stable entry point for the rest of the shell, keeping the active workspace, status indicators, and main menus in one place.
- It is designed to stay readable and useful even when the rest of the desktop is busy, so the user can glance at the shell state without opening another window.
- The bar exposes the shell's core menus, including the Cairo menu, Places menu, and Programs menu.
- It supports a more workspace-aware workflow than a traditional launcher bar, so the same surface can be used to launch apps, navigate common locations, and reach project tools.

## Cairo Menu

- The Cairo menu is the shell's command-oriented menu surface.
- It is meant for actions that behave like shell commands rather than simple app launches.
- Menu entries are configured as a list of commands, which makes the menu flexible enough for different workflows without changing the shell layout.
- The menu can be used as a stable home for shell-wide actions that you want always available from the top bar.
- It is especially useful for actions that should feel part of the shell itself, such as opening settings, switching shell behavior, or triggering system-level shortcuts.

## Places Menu

- The Places menu is a quick navigation surface for common locations.
- It provides fast access to well-known folders and shell locations without forcing the user to browse through the file system manually.
- It supports standard location tokens such as home, documents, downloads, computer, and recycle bin.
- It can also include custom paths, so a user can keep project folders or work directories close at hand.
- The menu is meant to behave like a curated location list rather than a raw folder picker, which keeps it fast to scan and easy to use.
- The menu's purpose is to reduce friction when jumping between the places a user visits repeatedly during the day.

## Programs Menu

- The Programs menu is the shell's application launcher surface.
- It groups installed or available programs into a discoverable menu rather than leaving them scattered across the desktop.
- It is meant for quick access to apps that the user launches repeatedly, especially when they want a consistent launch path from the shell.
- The menu pairs naturally with search and quick launch behavior, so the shell can move between curated menu browsing and direct lookup.
- Its role is to make the shell feel like a launcher-first environment instead of a plain desktop wrapper.

## Taskbar

- The taskbar is the bottom docked shell surface for open windows and quick launch items.
- It shows which application windows are currently visible and lets the user switch between them without relying on the native Windows taskbar.
- Windows can be grouped so that related windows stay together as a single logical item when that is the configured behavior.
- The taskbar understands multi-monitor setups and can adapt which windows it shows based on the current monitor and the chosen display mode.
- It is designed to keep the active workspace and the user's running apps visible at a glance.
- The taskbar can coordinate with quick launch entries so launching an app and switching to an already-open window feel like part of the same flow.
- Right-click actions on task buttons and window previews expose shell-owned options such as pinning, unpinning, app properties, running as administrator, and grouped-window actions.
- The taskbar can also expose workspace-aware actions for supported developer tools, so a window can be treated as part of a project workflow rather than only as an isolated app.
- Spotify media controls can appear in the taskbar when Spotify has an active session, giving the user playback controls and album art without leaving the shell.

## Search Panel

- The search panel is the shell's command palette.
- It is built for fast keyboard-driven lookup rather than browsing through several menus.
- Search can surface applications, shell commands, folders, workspaces, settings tabs, and project tasks in one place.
- It also has room for workspace status and task history results, which makes it useful for both navigation and recovery.
- Results are ranked so that more relevant or more frequently used items rise toward the top.
- Recently used entries gain priority, which makes repeat actions easier to reach over time.
- The selected result stays visibly highlighted during keyboard navigation, so the user always knows which item will run if they press Enter.
- The panel refreshes in the background instead of forcing every keystroke to rebuild the entire result set immediately.
- That behavior keeps the search surface responsive even when it is scanning a broad set of apps, commands, and workspaces.
- Search results can launch apps, invoke commands, open settings tabs, switch workspaces, open a workspace in an editor or terminal, or run a project task.

## Settings Panel

- The settings panel is the shell's configuration surface.
- It organizes shell options into user-facing groups instead of forcing the user to edit raw configuration files.
- The panel covers the main shell areas, including the menu bar, desktop behavior, taskbar behavior, and advanced options.
- It is designed to be the central place for changing how the shell looks, how it launches things, and how it behaves across workspaces.
- Workspace-related settings can store aliases, editor and terminal commands, environment variables, quick-launch entries, startup actions, and project tasks.
- The panel is meant to support both everyday customization and deeper project-oriented setup without requiring separate tools.
- Because settings are also searchable, users can often reach a settings page faster through the command palette than by clicking through tabs.

## Developer Dashboard

- The developer dashboard collects workspace and task information into one readable surface.
- It shows the active workspace, all configured workspaces, and recent task runs in one place.
- It is meant to help a user understand what the shell is doing without needing to inspect raw logs or configuration files.
- Workspace entries can show useful badges such as branch, toolchain, dirty state, merge state, or rebase state when that information is available.
- Recent task runs give the user a compact history of what succeeded or failed, which is useful after running project commands from the shell.
- The dashboard is especially helpful in project-heavy workflows where the shell is expected to support development work as well as general desktop use.

## Stack Popup

- The stack popup is the shell's file and folder browser surface.
- It is designed for browsing a folder's contents without leaving the shell or opening a separate file manager first.
- The popup shows items in a details-oriented layout so the user can scan names, types, sizes, and modified times quickly.
- Navigation controls make it possible to move backward, forward, or up through folder history.
- The popup supports mouse and keyboard interaction, so it can be used like a focused browsing panel rather than a passive menu.
- Clipboard actions such as copy, cut, and paste are part of the browsing flow, which makes the popup useful for real file management work.
- Rename happens in place, so the user can edit a file or folder name without being pushed out into a different dialog.
- Large folders are meant to load progressively so the popup stays usable instead of freezing while it gathers every item at once.
- The stack popup keeps common file-browser actions available while the folder view stays open, which makes it suitable for short, repeated file operations.

## Workspace Profiles

- Workspace profiles let the shell treat a project or working environment as a first-class concept.
- A workspace can carry a display name, aliases, a desktop path, editor and terminal commands, environment variables, startup actions, quick-launch paths, and tasks.
- Activating a workspace can switch the shell into the right project context rather than just opening a folder.
- Workspace activation can also restore associated launcher state and run startup actions, which helps the shell feel tuned to the current project.
- Project tasks can be exposed directly from the shell so a workspace is not just a folder reference but a real workflow container.
- The shell can use workspace data to make search results, status cards, and developer tools more useful and more specific to the current project.

## Task History

- Task history keeps a record of recent workspace task runs.
- The shell can use that history to show what was run, whether it succeeded, and when it happened.
- This makes it easier to rerun a task or inspect what happened after a failed command.
- Task history is useful both in the developer dashboard and in search, because it turns recent work into a quickly reachable part of the shell.
- The feature is meant to reduce repetition for project workflows, especially when the same build or test command is run many times.

## Automation And CLI

- The shell can be controlled through a command-line client as well as through the visible surfaces.
- Automation can activate workspaces, open a workspace in an editor or terminal, invoke shell commands, and run project tasks.
- This gives the shell a dual role: it is a desktop environment for interactive use and a control plane for scripted workflow actions.
- The automation path is useful for developers who want the shell to respond to tooling instead of only to mouse input.
- It also helps the shell fit into repeatable project workflows where the same action needs to be triggered from scripts, shortcuts, or other tools.

## Visual System

- The rewrite uses a consistent dark visual language across all shell surfaces.
- The shell relies on shared visual tokens so the menu bar, taskbar, search panel, settings panel, dashboard, and stack popup feel like parts of one system.
- The surface design is meant to be calm and readable rather than loud or decorative.
- Status badges, cards, and labels are used to communicate state quickly without overwhelming the user.
- The visual system is built to support both everyday desktop use and more data-rich developer workflows without changing the shell's overall identity.
- The result is a shell that feels cohesive even as it switches between navigation, configuration, workspace management, and task-oriented views.
