Inherited decisions:
- Fresh `ABM_QUERYPOS`/`ABM_SETPOS` rectangles are authoritative on fullscreen restore.
- Preserve AppBar reservation and retryable release/restore behavior.
- Avoid redundant WebView/AppBar positioning; native rect success is not render success.

Diagnosis:
- Geometry is correct: work area `[0,18,2048,1124]`, bottom HWND `[0,1124,2048,1152]`.
- The failure is likely WebView2 composition after repeated `window.hide()` / `window.show()`, not AppBar negotiation.
- `BottomBar.svelte` is persistent; no renderer remount path explains this.
- Current stabilization only checks HWND rectangles. Startup-only runtime metrics cannot detect a later blank WebView.

Drift / contradiction check:
- `hide_shell_for_fullscreen()` hides both persistent WebView windows; restore subsequently shows them and reapplies frame changes.
- This conflicts with the established warning that native positioning/visibility churn can race WebView startup/composition.
- The current AppBar fix correctly updates `ShellSurfaceLayout` to negotiated rectangles; do not revert it.

Recommendation:
- Keep both shell WebViews continuously visible to WebView2. During fullscreen:
  1. remove AppBars and restore full-monitor work area as now;
  2. park top and bottom HWNDs fully outside the *virtual desktop* using native `SetWindowPos`, without `SWP_HIDEWINDOW`, `WebviewWindow::hide()`, or frame-change flags;
  3. mark fullscreen released only after both parks succeed.
- On restore, retain the current fresh AppBar negotiation and work-area update, then place the already-visible HWNDs at the newly negotiated rectangles. Do not call `window.show()` or reapply shell styles/frame changes on this normal path; their styles did not change.
- On restore failure, park the HWNDs again, release registrations, and retain retryability.
- Do not add `RedrawWindow`/invalidate as the primary fix; that masks the visibility lifecycle defect.

Risks:
- Parking only outside the primary monitor can leak bars onto another monitor; use virtual-desktop bounds.
- A park failure must leave `fullscreen_hidden == false` so the guard retries rather than treating a visible bar as hidden.
- The WebView2 hypothesis is strongly indicated, but must be confirmed by repeated live testing.

Need from main agent:
- No architectural decision needed. Update the durable spec wording that fullscreen uses offscreen parking rather than native hide/show.

Suggested execution prompt:
- Implement a minimal Windows-only fullscreen lifecycle repair in `src-tauri/src/appbar.rs`: retain the current fresh negotiated AppBar rectangles/work-area restore, replace fullscreen `WebviewWindow::hide/show` with transactional offscreen parking outside virtual desktop bounds, and avoid restore-path frame-change churn. Add pure Rust tests for virtual-desktop parking and failed-park retry state; preserve existing AppBar restore tests. Validate with `cargo test --manifest-path src-tauri/Cargo.toml appbar`, then live Chrome F11 and Alt+Tab cycles (10+) with screenshots and native work-area/HWND rect logs. Require visible bottom-bar content, not merely correct geometry.

I could not write `fullscreen-blank-oracle.md`: this role has no write-capable tool.