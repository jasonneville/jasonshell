# Code Context

## Files Retrieved
1. `src-tauri/src/appbar.rs` (lines 657-674, 948-991, 1009-1028, 1030-1139) - fullscreen hide/restore, HWND placement, and AppBar state transition.
2. `src-tauri/src/shell_windows.rs` (lines 127-153, 418-451) - top/bottom webview construction and the unconditional post-show `SWP_FRAMECHANGED` call.
3. `src/App.svelte` (lines 13-49) - each webview resolves its label and lazy-loads exactly one persistent surface at initial mount.
4. `src/lib/surfaceLoader.ts` (lines 13-35) - `bottom-bar` maps to `BottomBar.svelte`.
5. `src/components/BottomBar.svelte` (lines 772-869) - BottomBar initializes once and retains listeners/polling; there is no fullscreen visibility/remount logic.
6. `src/components/BottomBar.css` (lines 1-13) and `src/app.css` (lines 400-408) - mounted bottom-bar root and document hosts occupy 100% height/width.
7. `scripts/smoke-fullscreen-appbar.ps1` (lines 137-163) - current smoke only asserts restored bars/geometry, not rendered content.
8. `tests/shellBarResize.test.mjs` (lines 98-105) - source wiring test for reservation/resize only; no fullscreen repaint coverage.

## Key Code

### Native lifecycle / likely failure point
Fullscreen entry hides both persistent webviews, then repositions their HWNDs while hidden:

```rust
// src-tauri/src/appbar.rs:986-990
hide_shell_surface_windows(app_handle)?;
move_window_to_rect(HWND(layout.top_hwnd as *mut _), layout.top_rect)?;
move_window_to_rect(HWND(layout.bottom_hwnd as *mut _), layout.bottom_rect)?;
```

On exit, AppBars and the work area are restored first. The native HWND geometry is then set while hidden, followed by Tauri `show()`, followed immediately by a *second* native frame-change operation:

```rust
// src-tauri/src/appbar.rs:1098-1113
move_window_to_rect(top_hwnd, restored_layout.top_rect)?;
move_window_to_rect(bottom_hwnd, restored_layout.bottom_rect)?;
window.show()?;
apply_no_alt_tab_shell_style_to_hwnd(bottom_hwnd, ...)?;
```

`move_window_to_rect` already calls `SetWindowPos(... SWP_FRAMECHANGED ...)` (lines 657-670). `apply_no_alt_tab_shell_style_to_hwnd` calls another `SetWindowPos(... SWP_FRAMECHANGED ...)` **even when the extended style did not change** (lines 422-447). Thus a restore does: hide -> hidden frame change/placement -> show -> visible frame change. This is the only native operation after `show()` that can perturb the WebView2 host/compositor, while geometry remains correct.

### Frontend is not remounting or clearing
The shared app chooses the bottom-bar surface once from the window label and dynamically imports it only during `onMount` (`src/App.svelte:30-43`; `src/lib/surfaceLoader.ts:13-16`). `BottomBar.svelte` then loads settings/data, listener registrations, and its polling interval in a single `onMount` (`src/components/BottomBar.svelte:772-806`), with cleanup only on component destruction (`854-868`). A native `hide()`/`show()` does not destroy this Svelte tree. Its root is a 100%-sized, opaque-background `.bottom-bar.surface` (`BottomBar.css:1-13`), with `html`, `body`, and `#app` also 100% sized (`app.css:400-408`).

**Conclusion:** this is overwhelmingly a native WebView2 repaint/composition problem, not data loss, lazy-load failure, frontend remounting, or CSS geometry. The successful HWND/work-area checks specifically do not prove the embedded webview drew.

## Architecture

1. `main.rs` creates hidden top/bottom Tauri webview windows; `appbar::activate_shell_surfaces` owns their AppBar reservation and first show.
2. A 250ms fullscreen guard in `appbar.rs` detects a foreground fullscreen candidate.
3. On entry it unregisters AppBars, restores the full monitor work area, hides both Tauri webviews, and calls `SetWindowPos` to keep HWND rectangles warm.
4. On exit it registers/reserves AppBars and applies fresh negotiated rectangles. It positions HWNDs, calls Tauri `show()`, then forces a visible `SWP_FRAMECHANGED`, and finally validates only `GetWindowRect` stability.
5. The bottom-bar renderer remains mounted throughout. Its contents go blank only after the native hide/show/frame-change path.

## Minimal native fix

**Recommended smallest fix:** do not issue the redundant visible `SWP_FRAMECHANGED` when the shell extended style is already correct. In `src-tauri/src/shell_windows.rs`, make `apply_no_alt_tab_shell_style_to_hwnd` return after the style comparison when `desired_style == current_style`; only call `SetWindowPos(SWP_FRAMECHANGED ...)` in the branch that actually calls `SetWindowLongPtrW`.

Why this is minimal:
- `build_shell_window` applies the shell style at creation (`shell_windows.rs:151`), so fullscreen hide/show normally has no style bit to change.
- Restore already performs final topmost placement with `SWP_FRAMECHANGED` before `show()` (`appbar.rs:1099-1100`).
- It eliminates the post-show non-client/frame transition most likely to detach/blank the WebView2 visual, without changing AppBar registration, negotiated geometry, work-area restoration, frontend code, or the existing error/retry state machine.

If Alt+Tab style testing proves a post-show frame notification is genuinely required on this Tauri version, keep it but add a native repaint immediately after it for **both** shell HWNDs: `RedrawWindow` with `RDW_INVALIDATE | RDW_ERASE | RDW_FRAME | RDW_ALLCHILDREN | RDW_UPDATENOW` (or the equivalent supported Win32 wrapper). This is the fallback workaround, not the first choice: it treats the compositor symptom, whereas avoiding an unnecessary post-show frame reset removes the trigger.

Do not add a frontend `visibilitychange` reload/remount. It would mask the native lifecycle defect, reload taskbar state, and risks duplicate async listeners/pollers contrary to the persistent-surface lifecycle contract.

## Tests

1. **Rust unit/source regression:** extract or test a small helper that reports whether a frame change is required; assert unchanged `WS_EX_TOOLWINDOW` / no `WS_EX_APPWINDOW` / no `WS_EX_NOACTIVATE` style produces no post-show frame change, while an incorrect style produces one. Update existing `shell_windows.rs` style tests. This is testable without a live HWND.
2. **Existing Rust geometry tests:** retain `fullscreen_restore_layout_uses_newly_negotiated_bottom_rect` in `src-tauri/src/appbar.rs` (currently around lines 1621-1680) and `cargo test --manifest-path src-tauri/Cargo.toml appbar` to ensure the fix does not regress negotiated placement.
3. **Focused live smoke (required):** extend `scripts/smoke-fullscreen-appbar.ps1` BRW-004/005 pass criteria to explicitly require: bottom-bar background, launcher icons/task tiles, and process-manager button are visibly painted and interactive after each of at least three fullscreen exits. Current lines 137-147 only cover bar presence/flush geometry.
4. **Diagnostic confirmation before/after:** use the existing `reportShellSurfaceRuntimeMetrics('bottom-bar')` call (`BottomBar.svelte:848-852`) to log correct native/webview dimensions. A blank bar with all positive metrics confirms the issue is paint/composition; after the native change, repeat browser F11 and borderless-app cycles.

## Start Here

Open `src-tauri/src/shell_windows.rs` at lines 418-451 first. It contains the unconditional post-show `SWP_FRAMECHANGED` that is the narrowest plausible trigger. Then verify its restore call sequence in `src-tauri/src/appbar.rs:1098-1124`.
