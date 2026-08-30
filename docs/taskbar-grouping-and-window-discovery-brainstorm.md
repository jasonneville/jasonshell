# Taskbar grouping and window discovery brainstorm

**Status:** Draft research, 2026-08-30  
**Scope:** Future research only. Not an implementation spec. Not code changes.  
**Method:** Ground current repo behavior in source, compare external patterns, then explore design options.

Legend: **Confirmed** = current JasonShell repo behavior. **External** = verified in source docs, used as inspiration only. **Proposal** = future idea, not commitment.

## Executive summary

JasonShell already has a real taskbar grouping core, and Search already indexes open task windows in `src/lib/searchCatalog.ts`; the current gap is specifically bottom-bar dense group discovery. App groups are keyed by normalized `processName`, each group expands into equal inner window tiles, and the strip relies on width caps and overflow rather than richer discovery. That works for small counts, but it gets fragile once a group has many windows, mixed attention states, or ambiguous identity.

The strongest direction is **Adaptive Group Capsule + Window Gallery**: keep direct tiles for tiny groups, collapse dense groups into a compact capsule, and open a stable exact-window gallery on dwell or explicit expand. That preserves speed for simple cases while adding exact-window discovery, policy control, and keyboard parity when groups become dense.

External desktop patterns point the same way: Windows, macOS, GNOME, KDE, FancyZones, and Groupy all show that users need a mix of fast switching, explicit grouping, and a denser detail view for larger sets. Those facts are inspiration, not JasonShell commitments.

## Current JasonShell baseline, grounded in repo

- **Bottom bar height:** currently defaults to **32.4 logical pixels** in `src/components/BottomBar.css` and the shell bar height path in `master_spec.md`.
- **Grouping key:** `src/lib/taskbarGroups.ts` groups windows by `processName.trim().toLocaleLowerCase()`, with `window:${hwnd}` only as a fallback when `processName` is empty.
- **Group rendering:** `src/components/BottomBar.svelte` renders each group as a set of equal-width child `MeltActionButton` tiles, one per window, inside the app group.
- **Width cap:** `src/components/BottomBar.css` gives each group `flex: var(--task-window-count, 1) 1 0` and `max-width: calc(10rem * var(--task-window-count, 1))`, so the strip does not endlessly stretch group width.
- **Label behavior:** `src/components/BottomBar.css` truncates labels with `overflow: hidden` and `text-overflow: ellipsis`.
- **Overflow behavior:** `src/components/BottomBar.css` keeps the task strip horizontally contained and exposes an overflow status node, while `src/features/bottom-bar/taskbarUxState.ts` surfaces keyboard guidance for hidden items.
- **Snapshot authority:** `src-tauri/src/task_windows/windows.rs` is the authoritative snapshot worker path, with sequenced state and `taskbar:windows-snapshot` publication; `src/lib/taskbarUi.ts` names the frontend event constants.
- **Timing:** hover preview delay is **180ms** and hide delay is **140ms** in `src/lib/taskbarUi.ts`.
- **Preview model:** `src/lib/taskbarPreview.ts` carries exact `hwnd`, title, process name, icon, minimized state, and preview source. `src/components/TaskPreviewSurface.svelte` consumes DWM / fallback preview payload. `src-tauri/src/task_preview.rs` owns native preview publication and lifecycle. Frontend task-window commands own activate / close paths.
- **Per-window controls:** each window tile has a native right-click menu in `src/components/BottomBar.svelte`, and left click activates or minimizes via the current window state.
- **Drag and reorder:** `src/lib/taskbarGroups.ts` and `src/components/BottomBar.svelte` support drag reorder with a 6px threshold and live reorder compensation.
- **Persistence:** I found no durable grouping policy or order persistence. `taskGroupOrder` lives in `src/components/BottomBar.svelte` memory and is only reconciled on refresh and drag operations.

## Problem decomposition and design goals

### What problem is actually being solved

1. **Fast recognition:** user should identify the right app or exact window in one glance.
2. **Fast activation:** user should get to the right window with minimal pointer or keyboard work.
3. **Density handling:** one app may have 1 window or 30. The UI must degrade cleanly.
4. **Policy control:** some apps need always-group, some never-group, some auto-group.
5. **Stable trust:** destructive actions must target the exact intended window, not stale identity.
6. **Low friction:** the bar must stay compact and predictable.

### Design goals

- Exact-window discovery stays available, not hidden behind too many taps.
- Small groups stay direct and readable.
- Dense groups become compact without losing control.
- Keyboard and pointer paths stay first-class.
- Native right-click menus remain available on exact windows.
- Group order remains stable unless user explicitly changes it.
- Policy state stays understandable and inspectable.

### Non-goals

- Not a global window manager replacement.
- Not a full virtual desktop system.
- Not a forced auto-reordering experience.
- Not a hard dependency on hover-only interaction.
- Not a commitment to one permanent visual language before validation.

## External pattern research

Linked docs below describe verified facts only. They are inspiration only, not JasonShell commitments.

Source access date: 2026-08-30.

| Source | Verified facts | Transferable lesson | Citation |
|---|---|---|---|
| Windows Alt+Tab / Task View | Alt+Tab shows thumbnails of open apps and cycles selection; Task View shows all open windows and desktops; users can click thumbnails. | Exact-window discovery needs a fast switching path plus a richer overview path. | https://support.microsoft.com/en-us/windows/how-to-multitask-in-windows-b4fa0333-98f8-ef43-e25c-06d4fb1d6960 |
| macOS Spaces / app assignment | Apps can be assigned to All Desktops, This Desktop, Desktop on Display N, or None; switching to an app can auto-switch to a Space with open windows. | Grouping policy and app placement benefit from explicit per-app control, not only implicit heuristics. | https://support.apple.com/guide/mac-help/work-in-multiple-spaces-mh14112/mac |
| GNOME workspaces | Workspaces group windows; the workspace selector shows used workspaces plus an empty one; windows can be dragged between workspaces. | A group overview works best when it includes an obvious place to send or reorganize windows. | https://help.gnome.org/gnome-help/shell-workspaces.html |
| GNOME windows | Activities overview and dash are the main control surface for launching apps and controlling active windows. | Discovery and control should stay close to the overview surface, not buried in settings. | https://help.gnome.org/gnome-help/shell-windows.html |
| KDE Plasma Tasks | Grouped tabs show a count and small arrow; clicking a group opens a list of individual window tabs; right-clicking a group can close all windows in that group; tooltips can show thumbnails. | Group chip plus disclosure list is a proven compact pattern for dense app/window sets. | https://userbase.kde.org/Plasma/Tasks |
| FancyZones | Zones can be snapped by mouse or keyboard; layouts can be custom; windows can cycle within a zone; monitor-aware behavior and layout preview exist. | User-tunable geometry and keyboard parity matter when density is high. | https://learn.microsoft.com/en-us/windows/powertoys/fancyzones |
| Groupy | Apps can be grouped with tabs; same-app instances can be automatically grouped; groupings can launch multiple apps; accents help mark type, task, or purpose. | Tabbed grouping can be extended with semantic labels and launch/group presets. | https://www.stardock.com/products/groupy/ |

## Concept options

Each option below is materially different. Some are mutually compatible, but each can stand alone as a distinct product shape.

| Concept | Mechanism | Bar appearance | Preview / discovery | Per-window controls / right-click | Strengths | Drawbacks / failure modes | Best fit |
|---|---|---|---|---|---|---|---|
| Mini-tabs | Keep 1-3 windows as visible mini-tabs inside the group, then collapse later. | Small segmented inner tabs, active accent, count badge. | Hover or click expands to exact window targets. | Native menu on each mini-tab; group menu on capsule. | Very direct for tiny groups. | Gets crowded fast. | Low-count apps where exact window identity matters. |
| Group drawer | Collapse group into one chip; click opens an in-flow drawer with exact windows. | Single chip plus drawer panel below or above the strip. | Drawer lists exact windows with state and actions. | Each drawer row keeps native menu. | Compact, stable, easy to scan. | Adds extra vertical or overlay step. | General default when the bar is tight. |
| Hover filmstrip | Hover opens a horizontal filmstrip of window thumbnails anchored to the group. | Capsule plus hover strip. | Fast visual browse by thumbnail. | Thumbnail card menu on each window. | Strong for visual recognition. | Hover dependence, pointer jitter risk. | Users who think visually and switch often. |
| Thumbnail grid / gallery | Expand to a grid of thumbnails and titles. | Capsule with gallery popover or flyout. | Best for many windows and mixed states. | Exact card menu plus group menu. | High findability, handles large counts. | Needs space and careful scroll logic. | Dense apps with 4+ windows. |
| Pinned exact windows | Users can pin exact windows inside a group. | Capsule with pinned sub-tile row. | Pinned windows stay first in discovery order. | Pinned row gets native menu and pin/unpin. | Great for recurring windows. | Pin drift and stale pin risk. | Repeated task sets, monitoring windows. |
| Recency stack | Sort exact windows by last focus, newest first. | Capsule with stack ordering. | Most likely window appears at top. | Row menu includes pin or keep stable. | Very fast for active work. | Can feel jumpy and non-deterministic. | Rapid context switching. |
| Stable spatial stack | Preserve a stable visual order based on first seen or saved order. | Capsule with fixed internal order. | Findability via memory, not churn. | Row menu includes move up/down. | Low surprise, good continuity. | New windows may feel buried. | Users who value stable muscle memory. |
| Card deck | Expand into overlapping cards with one active card in front. | Layered cards, active card offset. | Swipe or click through stacked cards. | Front card controls on visible face. | Pleasant, compact, tactile. | Harder exact selection at high counts. | Medium counts with strong visual polish. |
| Density markers | Represent groups with count bars, dots, attention dots, and activity marks. | Very compact capsule language. | Hover or click opens exact-window view. | Right-click on capsule only; row menus in expanded view. | Excellent width efficiency. | Weak exact-window discoverability alone. | Very narrow bars or many groups. |
| Searchable palette | Open a search palette that filters exact windows by title, app, state, or alias. | Capsule plus command-palette trigger. | Type-to-find exact window. | Palette rows can keep native menus. | Best keyboard and high-count access. | Slower for simple click users. | Large counts, power users, accessibility. |
| Semantic / project lanes | Group by project, workspace, or semantic label instead of only app. | Lane chips with project accent. | Browse by task domain, not just process. | Lane menu edits label, alias, policy. | Best for deliberate workflow grouping. | Requires user setup and labeling. | Professional workflows, multi-step projects. |
| Adaptive second row | Keep a first row of direct tiles, then spill a second row when width pressure rises. | Two-row strip, still inline. | Discovery stays on bar, no popover needed. | Native menus on each tile in both rows. | Familiar and low learning cost. | Can compete with other shell surfaces vertically. | Wide groups on roomy displays. |
| Focus spine | Show one active title as the spine, with arrows or steps to cycle exact windows. | Single dominant title, minimal metadata. | Next/prev exact-window stepping. | Per-window menu via menu key or overflow action. | Extremely compact and keyboard-friendly. | Poor browseability for many windows. | Small strips and keyboard-heavy users. |
| Per-window command rail | Expand a rail of actions per exact window: activate, close, pin, move, policy. | Card or row with action strip on the side. | Exact window plus action affordances stay visible. | Native menu supplemented by rail actions. | Powerful and explicit. | Visual noise if always shown. | Power-user mode, dense control workflows. |
| Window map | Draw a miniature map of all exact windows in a group, spatially arranged. | Capsule with mini-map or matrix. | Map helps locate by layout and state. | Each node gets its native menu. | Good for large groups and spatial memory. | Hard to make readable at tiny scale. | Users with many windows on one app. |
| Preview lock / safe corridor | Hover opens preview, which stays alive while pointer moves through a safe corridor or lock state. | Capsule with locked preview state cue. | Prevents accidental preview collapse. | Context menu from locked preview card. | Reduces jitter and accidental hide. | Can feel sticky if tuned badly. | Hover-heavy preview workflows. |
| Adaptive collapse thresholds | Group changes form at thresholds based on count, width, and pressure. | Dynamically shifts from tile to capsule to gallery. | Discovery follows the active density mode. | Menu model matches current form. | Very flexible and space aware. | Harder to explain and debug. | A governing policy layer, not a single shape. |
| Overflow shelf | Excess exact windows move into a secondary shelf or paging strip. | Primary capsule plus overflow shelf edge. | Shelf exposes hidden windows in ordered batches. | Shelf rows keep native menus. | Good for very large groups. | More navigation overhead. | 8+ windows with limited strip width. |
| User-defined app aliases / manual task collections | Let users name and collect windows into custom groups that outlive pure process identity. | Alias chips or named collections. | Discovery via collection names and aliases. | Collection menu edits membership and policy. | Matches how users think about work. | Requires durable model and trust. | Power workflows and cross-app tasks. |
| Workspace / monitor lanes | Organize groups by workspace or monitor lane, then drill down to windows. | Lane headers, monitor badges, group chips. | Discovery starts from location or workspace. | Window menus include move-to-lane actions. | Good for future multi-monitor stories. | Needs stronger identity and geometry. | Future multi-monitor and space-aware UI. |

## Policy dimensions to decide explicitly

| Dimension | Options | Design question |
|---|---|---|
| Auto / Always / Never combine | Auto, Always, Never | Should every app be allowed to auto-group, or should some stay ungrouped or always grouped? |
| Grouping key | AppUserModelID, package identity, executable path, normalized processName, manual collection | What identity should define a group when process names are too weak? |
| Manual vs automatic | Manual only, auto only, hybrid | Can users override grouping, or does policy control everything? |
| Ordering | Stable, recency, hybrid, user-pinned | Does internal order stay fixed, or float with focus history? |
| Collapse threshold | Count only, width only, count + width, count + attention | When does a group become a capsule instead of a visible tile set? |
| Overflow behavior | Horizontal scroll, shelf, second row, gallery, search | What happens when the strip cannot show everything? |
| Hover vs click | Hover only, click only, hybrid | Is expansion a passive preview or explicit action? |
| Density mode | Direct tiles, mini-tabs, capsule, gallery, list | Which representation wins at each count band? |
| Preview mode | Thumbnail, grid, filmstrip, list, locked preview | How should exact windows be discovered once the group is expanded? |

## Decision matrix

This matrix scores shortlisted candidate interaction shapes. Omitted catalog concepts are supporting policy / extension / advanced layers, not accidental gaps. Directional hypotheses only. Scores use 1 to 5, where 5 is best. Risk score uses 5 = low risk.

| Option | Space eff. | Exact findability | Pointer speed | Keyboard / a11y | Visual continuity | Scalability | Risk |
|---|---:|---:|---:|---:|---:|---:|---:|
| Adaptive Group Capsule + Window Gallery | 5 | 5 | 4 | 5 | 4 | 5 | 4 |
| Group drawer | 4 | 4 | 4 | 4 | 4 | 4 | 4 |
| Thumbnail grid / gallery | 3 | 5 | 3 | 4 | 4 | 5 | 3 |
| Hover filmstrip | 4 | 4 | 5 | 2 | 4 | 3 | 3 |
| Recency stack | 4 | 3 | 5 | 3 | 3 | 4 | 4 |
| Stable spatial stack | 4 | 4 | 4 | 4 | 5 | 4 | 4 |
| Searchable palette | 3 | 5 | 3 | 5 | 2 | 5 | 4 |
| Adaptive second row | 3 | 4 | 4 | 4 | 4 | 4 | 3 |
| Overflow shelf | 4 | 4 | 3 | 4 | 3 | 5 | 3 |
| Semantic / project lanes | 3 | 4 | 3 | 4 | 4 | 4 | 3 |
| Pinned exact windows | 4 | 4 | 4 | 4 | 4 | 4 | 3 |
| Mini-tabs | 4 | 4 | 5 | 3 | 4 | 2 | 4 |

## Hybrid packages

### 1. Capsule + gallery + search

- **Pros:** compact by default, exact-window search for high counts, easy to explain.
- **Cons:** three interaction layers if overused.

### 2. Capsule + mini-tabs + adaptive second row

- **Pros:** keeps tiny groups direct and familiar.
- **Cons:** row growth can fight vertical space and create visual clutter.

### 3. Capsule + stable spatial stack + preview lock

- **Pros:** low surprise, strong muscle memory, fewer accidental preview drops.
- **Cons:** weaker freshness signal unless attention markers are strong.

### 4. Capsule + semantic lanes + user aliases

- **Pros:** best fit for project-based work and cross-app collections.
- **Cons:** depends on settings, naming, and user discipline.

## Recommendation

### Recommended direction: Adaptive Group Capsule + Window Gallery

**Exact intended behavior:**

- 1 window: current direct tile.
- 2-3 windows: direct segmented child tiles while width allows.
- 4+ windows, or when strip pressure crosses threshold: compact app capsule with icon, active title, count, and attention / activity state.
- Pointer dwell or explicit expand affordance opens an exact-window gallery anchored to the group.
- Gallery shows thumbnail, full title, and state.
- Stable order by default, optional recent sort.
- Lazy or live preview only for visible or focused cards.
- List fallback and search at high counts.
- Left click on an exact card activates that window.
- Exact-card right-click keeps the native per-window menu.
- Group background right-click opens group policy and group actions.
- Keyboard support: arrows, Home, End, Enter, Esc, Menu, Shift+F10.
- Per-app policy: Auto, Always, Never.
- Temporary expansion is clear and reversible, with no surprise auto-reordering.
- Preview reliability needs visible state, plus safe-corridor or lock behavior so hover does not collapse too easily.

### Why this wins

It is the best balance of width efficiency, exact-window findability, and implementation risk. It keeps the current direct-tile model for small groups, adds one compact collapse step for dense groups, and preserves native per-window actions without forcing a new mental model on every user. It also gives JasonShell a clean place to grow into policy, search, and future multi-monitor concepts later.

### What stays optional later

- Semantic / project lanes.
- Recency stack sorting.
- User-defined aliases and manual task collections.
- Workspace / monitor lanes.
- Full palette-first discovery for power users.

## Technical architecture implications, future requirements to validate

These are not current behavior. They are likely requirements if the design moves forward.

- **Identity chain must grow beyond processName.** Current grouping is too weak for durable policy. Future priority should be: `AppUserModelID > package identity > canonical executable path > normalized processName > HWND fallback`. **UNCONFIRMED:** AppUserModelID / package identity collection reliability must be proven through enumeration / diagnostics before adoption.
- **Raw HWND must never become durable identity.** HWND is a live window handle, not a stable app key.
- **State should be layered.** Separate app identity, policy, group membership, exact window identity, and preview request state.
- **Destructive actions need stale-identity safety.** Close, menu actions, and preview-driven actions should validate the exact live target before firing.
- **Group preview contract should evolve.** Group-level preview should not blur together with exact-card preview. The contract needs an anchor + exact target split.
- **DWM and thumbnail work must stay lazy.** Live preview for every card at once will be too expensive. Visible or focused only is the safer rule.
- **Settings persistence needs a durable schema.** Policy, order, and aliases should be versioned and merge-safe.
- **Future multi-monitor coordinates matter.** Gallery anchoring, lane placement, and preview positioning should not assume a single monitor forever.

## Risk register

| Risk | Why it matters | Mitigation idea |
|---|---|---|
| Identity collision | `processName` groups too much together. | Move toward canonical app identity chain. |
| Stale window handle | A closed or replaced HWND can target the wrong window. | Revalidate exact live target before destructive actions. |
| Hover jitter | Hover-only expansion can flicker or collapse too fast. | Add explicit expand affordance plus safe-corridor lock. |
| Control overload | Too many actions on every card can muddy the bar. | Hide advanced actions behind gallery or context menu. |
| Perf regressions | Too many thumbnails can hurt frame time. | Lazy load, virtualize, and limit live preview. |
| Policy confusion | Auto / Always / Never can feel abstract. | Make defaults simple and expose explainers in settings. |
| Persistence migration | Policy and alias schema will change. | Version settings and keep backward-safe migration. |
| Accessibility drift | Hover-only or drag-only UX can exclude keyboard users. | Keep keyboard parity from the start. |

## Unresolved decision questions

- Default policy per app: Auto, Always, or Never?
- Ordering default: stable first, recent first, or hybrid?
- Collapse threshold: count only, width only, or both?
- Expansion trigger: hover, click, or both?
- Preview mode at high counts: grid, list, filmstrip, or search-first?
- Should user aliases be first-class or advanced-only?
- When does a group earn a second row instead of a capsule?
- How much should background right-click differ from card right-click?

## Prototyping and measurement plan

Conceptual only:

1. Build low-fi mocks for capsule, drawer, gallery, and searchable palette variants.
2. Test small, medium, and large counts, like 1, 3, 8, and 20 windows.
3. Measure time to find exact window, misclicks, hover drops, and keyboard completion.
4. Compare stable order vs recency sort for recognition speed.
5. Measure live preview cost with visible-only loading versus eager loading.
6. Validate native menu access on exact cards and group background.
7. Check readability at reduced width and on a second monitor.

## Staged future exploration

1. **Stage 1:** keep current direct tiles for 1-3 windows, test capsule collapse at 4+.
2. **Stage 2:** add exact-window gallery with stable order and optional recent sort.
3. **Stage 3:** add search fallback and policy controls.
4. **Stage 4:** add aliases, manual collections, and workspace / monitor lanes if needed.
5. **Stage 5:** revisit advanced visual models like window maps or card decks only if data proves they help.

## Recommendation confidence

Medium-high. Confidence is strongest on the capsule + gallery core. Confidence is lower on the exact policy defaults, collapse threshold, and final preview mode until prototypes and timing data exist.

## Sources and references

### Project files

- `master_spec.md`
- `src/components/BottomBar.svelte`
- `src/components/BottomBar.css`
- `src/features/bottom-bar/taskbarUxState.ts`
- `src/lib/taskbarGroups.ts`
- `src/lib/taskbarPreview.ts`
- `src/lib/taskbarUi.ts`
- `src/components/TaskPreviewSurface.svelte`
- `src-tauri/src/task_windows/windows.rs`
- `src-tauri/src/task_preview.rs`
- `src-tauri/src/contracts.rs`

### External URLs

- https://support.microsoft.com/en-us/windows/how-to-multitask-in-windows-b4fa0333-98f8-ef43-e25c-06d4fb1d6960
- https://support.apple.com/guide/mac-help/work-in-multiple-spaces-mh14112/mac
- https://help.gnome.org/gnome-help/shell-workspaces.html
- https://help.gnome.org/gnome-help/shell-windows.html
- https://userbase.kde.org/Plasma/Tasks
- https://learn.microsoft.com/en-us/windows/powertoys/fancyzones
- https://www.stardock.com/products/groupy/
