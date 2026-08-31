# Adaptive Taskbar Group Capsules and Window Gallery

Status: implementation plan, amended 2026-08-30

## Required behavior

- One-window groups always render as direct task buttons.
- Two-window groups render directly under `auto` until measured strip pressure crosses hysteresis.
- Groups with three or more windows render as capsules.
- Capsules open one exact-window gallery on click or after a cancellable 300ms pointer dwell. Gallery order comes from the authoritative taskbar snapshot.
- Latest request overrides earlier search/list thresholds: gallery stays one horizontal native tab strip only, with no search row, list mode, or card grid.
- Gallery actions target the selected HWND for activation, active-window minimize intent, context menu, and preview.
- Arrow keys, Home, End, Enter, Space, ContextMenu, Shift+F10, and Escape remain supported.
- Preview work is demand-driven, one request at a time, and freshness guarded.
- Group drag/reorder and backend task-window safety checks remain authoritative.
- No policy persistence is introduced. Pointer leave or task-group drag cancels pending dwell-open. Once open, leaving both gallery tabs and task preview schedules gallery dismissal with the regular preview hide delay; entering either surface cancels it.

## Native surface amendment

Inline rendering is rejected. `bottom-bar` is a fixed-height native AppBar webview (32.4 logical px by default), and its HWND plus document/task-strip overflow boundaries clip content. A usable gallery cannot extend above that native window.

Implement gallery as dedicated `task-gallery` webview, following existing Quick Launch and Task Preview lifecycle patterns:

1. BottomBar sends anchor geometry, nonce, group metadata, and the ordered authoritative window snapshot.
2. Rust sizes and positions the gallery above BottomBar, stores the current nonce, and allowlists supplied HWNDs.
3. Gallery emits exact actions through nonce-checked commands. Rust rejects stale nonces, unknown HWNDs, and unauthorized caller labels before forwarding to existing task-window/menu safety paths.
4. Only one gallery session exists. New open replaces previous state. Snapshot reconciliation closes gallery when its group disappears or drops below three windows and republishes changed ordered rows, including removal after preview-X close, only into the matching active session. A queued refresh must never recreate a closed session.
5. Escape, focus loss, successful activation, or BottomBar toggle hides the window, clears runtime state, hides preview, and emits a nonce-bearing closed event to both BottomBar and gallery.
6. Native context-menu focus transitions use a bounded focus-loss hold, matching Quick Launch behavior.
7. Gallery preview remains existing `task-preview` surface, requested lazily from hovered/focused exact rows. Hover-open does not activate the gallery; click-open may focus it for keyboard use. BottomBar and gallery allocate request generations from shared native preview state so either surface can safely supersede the other. Preview close routes the gallery nonce and exact HWND back through native authorization, hides only the preview, and retains the gallery until the authoritative snapshot removes the row or the group drops below three windows. Rust captures PID, process creation time, and canonical image path for every allowed HWND, revalidates that identity before each action, and uses the clicked window's captured PID for Process Manager filtering. No polling is added.

## Verification

- Pure display, pressure, filtering, navigation, and reconciliation tests.
- Source/native contract tests for surface registration, caller checks, nonce/HWND allowlisting, exact routing, and lifecycle events.
- Focused TypeScript/Svelte and Rust checks, then full project validation.
- Manual Windows QA recorded only if actually performed.
