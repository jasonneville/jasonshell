# Spec: egui bottom-bar prototype

**Author:** Planning/specification subagent for JasonShell
**Date:** 2026-10-07
**Status:** Approved 2026-10-08 for initial safe M0/M1 fixture only; later native/shared-service and I integration gates remain separate. NOT authorization to run live shell smoke.
**Reviewers:** Independent plan review PASS; user approved initial safe fixture, then explicitly approved reuse and validation of pinned egui/eframe 0.32.3 + wgpu 25.0.2 and the clearly simulated showcase Process Manager on 2026-10-08.

**Approval boundary:** Reuse the existing ordinary-window fixture, not a UI rewrite.
No real process/launch actions, AppBar/Explorer changes, hooks, persistence, native
host integration or renderer dependency expansion are approved by this M0/M1 gate.
The broader requirements below remain proposed later-track contracts; simulated
showcase process actions are not connected Process Manager parity.

## Context

The requested experiment is “just the bottom bar (all things functionality within it)” to evaluate look, feel, and performance without a browser-rendered bar. Current JasonShell is Tauri 2/Svelte 5/TypeScript/Rust/Win32; primary-monitor AppBar ownership remains its runtime boundary. No egui prototype, pinned dependency version, benchmark, or measured saving exists. [Earlier feasibility research](../rust-ui-frontend-feasibility-2026-10-07.md) is context, not evidence that migration wins.

Source inspection changes the apparent scope: the rendered bottom-left control is **Quick Launch**, not an inline launcher-icon rail. `BottomBar.svelte` retains launcher reorder/menu handlers and CSS, but its template renders no launcher buttons. Quick Launch currently supplies launch and Run as administrator, not all seven dormant launcher menu entries. The plan inventories both, without quietly making dormant code into a new required UI. Current visible right-edge control is Processes; no bottom-bar desktop, clock, terminal, or Stack Browser button exists. Stack Browser opens from top-bar pinned folders (`TopBar.svelte`, `stack_popup/`); bottom launchers ShellExecute validated Explorer `.lnk` pins (`launchers.rs`), not a direct Stack Browser folder dispatch. This boundary prevents an unnecessary editor/Git/terminal rewrite.

**Recommended boundary:** build native egui bar-owned UI (bar, Quick Launch and its admin menu, single preview, gallery, task/group menus) plus reusable host-independent Win32 services. Deliver **two separately labelled tracks**: S, a standalone Rust executable with no Tauri/WebView dependencies for isolated bar measurements; I, a separately approved integration companion that reuses the existing Process Manager for complete reachable process workflows. The companion has WebViews and cannot be advertised as browser-free. S demonstrates process handoff with a truthful capability-unavailable result until I exists; this is an interim limitation, not “all functionality complete.” Final requested scope requires I's real Process Manager journey as well as S's native bar journeys. A fully native Process Manager is an optional scope expansion requiring another approved spec; it is not a prerequisite for comparing bar renderers.

## Feature inventory and scope treatment

Every row maps to a requirement and acceptance scenario below. N = native bar-owned egui; H = validated host service/external OS or application action; I = existing external panel integration; D = dormant/unrendered code, documented not silently added. Paths are repository-relative. Existing Node/component/browser tests are parity references, not tests of a future Rust frontend.

| ID / source | Current action/contract | Prototype treatment | Acceptance/test |
|---|---|---|---|
| B01 `src/components/BottomBar.svelte`, `BottomBar.css`, `src/lib/settings.ts`, `shellBarResize.ts` | Quick Launch left, evenly shared capped task strip, Processes right; default height 32.4; locked resize handle absent; unlocked upward drag grows, 24–120 logical-pixel clamp, coalesced native resize, merge-safe saved height | N/H; settings/theme imported into sandbox; fixture lock toggle is demo control, not invented bar button | FR-1/2, AC-1/2; `tests/shellBarResize.test.mjs` |
| B02 `src/lib/taskbarWindows.ts`, `src-tauri/src/task_windows/windows.rs`, `actions.rs` | Enumerate/filter open task windows, cached initial list, sequenced native snapshots, explicit refresh; root-owner/modal activation; minimized/active/attention/activity/toast state | H/N; preserve sole snapshot producer, normalization and eligibility; no frontend steady poll | FR-3/4, AC-3/4; `tests/taskbar*.test.mjs` |
| B03 `src/lib/taskbarGroups.ts`, `src/features/bottom-bar/taskbarUxState.ts`, `BottomBar.css` | Process-name grouping with HWND fallback; singles direct, 2+ capsules in current auto policy; order reconciliation; neutral counts, Ribbon marks/Matrix glyph; active/toast/busy/minimized cues, pressure hysteresis 24/48, overflow status | N; same ordered model, 160 logical-pixel capsule/single cap; aggregate max toast count, eligible busy signals only | FR-5, AC-5; `tests/taskbarGroups.test.mjs`, `taskbarUxState.test.mjs` |
| B04 `BottomBar.svelte`, `src/lib/taskbarTilePointer.ts`, `taskbarGroups.ts` | Tile click uses pre-click active intent to minimize or restore/focus; pending pointer release executes once; group reorder at 6px horizontal threshold, displacement placement, click suppression, Escape/cancel/lost-capture rollback | N/H; session-only task order, no extra persistence | FR-4/6, AC-4/6; `tests/taskbarTilePointer.test.mjs` |
| B05 `BottomBar.svelte`, `src/lib/taskbarUi.ts`, `taskbarPreview.ts`, `src/components/TaskPreviewSurface.svelte`, `TaskPreviewSurface.css`, `src-tauri/src/task_preview.rs` | Single hover 180ms, hide 140ms with crossing cancellation; shared native request IDs; 332x228 preview, 8px host-edge gap; icon/title/process; live/captured/unavailable; activation actually calls **maximize**, separate Close, Enter/Space | N/H; keep maximize distinction (not just generic focus), request/generation guards and unavailable actions | FR-7/8, AC-7/8; `tests/bottomBarPreviewRequestId.test.mjs`, `tests/browser/task-preview-inset/` |
| B06 `BottomBar.svelte`, `TaskGallerySurface.svelte`, `src/lib/taskGallery.ts`, `taskGalleryLayout.ts`, `src-tauri/src/task_gallery.rs` | Capsule hover 300ms, click toggles focused gallery; 140ms hover close, transition nonce holds crossing; refreshed same-session ordered layout; Ribbon 2/3 if width fits, Matrix 4+ or constrained | N/H; native-authoritative geometry, clamped placement, session close/focus-loss/refresh | FR-9, AC-9; `tests/taskGalleryLayout.test.mjs` |
| B07 `TaskGallerySurface.svelte`, `task_gallery.rs` | Inline thumbnails only; max 12 fully contained frames; live/captured/unavailable; nonce/revision/request guards; retain unchanged identity/layout, clear old destinations before mutation; local menu awaits matching empty clear acknowledgement | N/H; DWM registrations behind adapter, texture backups retained truthfully; no separate per-card hover preview | FR-10, AC-10; `tests/components/task-gallery-runtime.test.ts`, `tests/browser/task-gallery/`, `tests/taskGalleryNativeRuntimeContract.test.mjs` |
| B08 `TaskGallerySurface.svelte`, `src/features/bottom-bar/taskbarUxState.ts` | Selected activation/Close pair only in Tab order; spatial arrows, Home/End, scroll selection into view; hover not selection; separate duplicate-guarded Close; right-click/Shift+F10 menu Restore/Switch and Minimize; Escape closes menu before gallery | N/H; surviving focus/selection retained across revisions | FR-11/12, AC-11/12; `tests/components/task-gallery-runtime.test.ts` |
| B09 `src/components/ContextMenuOverlaySurface.svelte`, `src/lib/contextMenuOverlay.ts`, `taskbarMenus.ts`, `src-tauri/src/taskbar_menu.rs` | Standalone task menu Restore/Switch, Minimize (disabled if minimized), Process Manager (disabled without PID), Pin, Close; menu opaque token, origin focus on dismissal not selected action | N/H/I; native menus preserve dismissal/action distinction; capability checked before action | FR-13, AC-13; taskbar menu/overlay contracts |
| B10 `BottomBar.svelte`, `taskbar_menu.rs`, `src-tauri/src/task_windows/actions.rs` | Group menu Process Manager, Pin, Close all without opening gallery; captured membership/active representative; surviving identity only; normal request-close WM_CLOSE, exclude newcomers, continue failures, never terminate | N/H/I; report requested/failed/skipped, prompts can remain | FR-14, AC-14; `tests/components/taskbar-group-menu.test.ts`, `tests/taskbarGroupMenuContract.test.mjs` |
| B11 `src/components/OverflowTicker.svelte` | Direct/capsule/gallery titles: clipped ellipsis; hover OR focus 600ms delay, 40px/s out, 2s hold, same-speed return, 3s hold/repeat; fitting/reduced motion static; remeasure resize/title, full accessible title | N; animate local text only, keep icons/counts fixed | FR-15, AC-15; `tests/components/overflow-ticker.test.ts` |
| B12 `BottomBar.svelte`, `QuickLaunchPanelSurface.svelte`, `src/lib/quickLaunchPanel.ts`, `src-tauri/src/quick_launch_panel.rs`, `launchers.rs` | Explorer pins alphabetically sorted, icon/name/detail, nonce+allowed canonical shortcuts; toggle, open-flight guard, open-button pointerdown close suppresses click; arrows wrap/Home/End/Enter/Escape, blur dismiss; success closes, failure stays retryable | N/H; native Quick Launch, current Explorer pins only, generic icon fallback, no resolve preflight removal | FR-16/17, AC-16/17; Quick Launch/launcher contract tests |
| B13 `QuickLaunchPanelSurface.svelte`, `quick_launch_panel.rs`, `launchers.rs` | Right-click/ContextMenu/Shift+F10 Run as administrator; UAC cancel errors; WindowsApps fallback can launch unelevated and must not claim elevation; held focus during action | N/H; preserve validation and report actual outcome | FR-18, AC-18; native launcher tests |
| B14 `BottomBar.svelte`, `src/lib/processManager.ts`, `src/components/ProcessManagerSurface.svelte`, `src-tauri/src/process_manager.rs` | Processes button opens anchored panel; window/group menu supplies PID; live metrics, grouped/filterable/sortable processes, refresh, selection, guarded end process, close/blur | I; existing panel/service behavior retained, full connected journey tested, not a dummy button. S has explicit unavailable state | FR-19/20, AC-19/20; process manager tests |
| B15 `BottomBar.svelte`, `taskbarPins.ts`, `BottomBar.css`, `taskbar_menu.rs` | Dormant launcher rail drag/order localStorage v1 and menu Launch/runas/Properties/reveal/revealTarget/copyPath/unpin; **not rendered** in current template | D/H; inventory only in required scope. Compatibility fixtures/actions/order are OPTIONAL, separately approved; no new inline rail by default. If approved, unpin deletes validated .lnk only | Optional FR-21/AC-21, excluded from required completion evidence; `tests/taskbarLauncherReorder.test.mjs`, launcher tests are reference only |
| B16 `src/lib/shellPreferences.ts`, `themes.ts`, `settings.ts`, global styles; `shell_windows.rs`, `appbar.rs`, `layout.rs` | Cross-surface theme/settings; shell window/toolwindow policy, primary AppBar, Explorer hide/restore identity ownership, work-area rollback, fullscreen release/restore, DPI conversion, cleanup | N/H/I; isolated preferences and exclusive experimental ownership; not toolkit-provided behavior | FR-22/23/24, AC-22/23/24; AppBar/Rust tests |
| B17 `BottomBar.svelte`, `taskbarUxState.ts`, native runtime metrics | Task-strip arrows/Home/End scroll focused item; overflow polite status; loading/empty/unavailable/failure messages; subscriptions/timers removed on disposal | N/H; native accessibility tree and event cleanup; distinguish diagnostics from success | FR-25/26, AC-25/26; UX/runtime contract tests |

## Functional Requirements

RFC 2119 keywords below define the **proposed** prototype contract, not current runtime claims.

- FR-1: The prototype MUST render the current visible control order and token-derived layout described by B01.
- FR-2: The prototype MUST support locked/unlocked bottom-height behavior and coalesced 24–120 logical-pixel resizing into sandbox preferences.
- FR-3: Native mode MUST consume one authoritative sequenced task snapshot stream with current enumeration/filtering semantics.
- FR-4: Task activation MUST preserve pre-click active minimize intent, minimized restore, root-owner/modal resolution, and authoritative refresh reconciliation.
- FR-5: Group rendering MUST preserve the B03 aggregate, order, capsule, pressure, count, state-cue, cap, and overflow rules.
- FR-6: Task-group reorder MUST preserve B04 threshold, displacement, cancellation and exactly-once click suppression semantics.
- FR-7: Single preview lifetime MUST preserve B05 hover delays, crossing cancellation and monotonic native request ownership.
- FR-8: Single previews MUST provide maximize activation and separate Close even when imagery is unavailable.
- FR-9: Galleries MUST preserve B06 nonce-scoped hover/click/focus/close and authoritative Ribbon/Matrix layout behavior.
- FR-10: Gallery thumbnail synchronization MUST preserve B07 containment, frame bound, stale-response and identity-continuity rules.
- FR-11: Gallery keyboard/focus behavior MUST preserve B08 spatial navigation and surviving activation/Close focus without hover selection.
- FR-12: Gallery window actions MUST preserve separate Close, local Restore/Switch and Minimize menu, duplicate guarding and failure diagnostics.
- FR-13: Standalone task menus MUST expose B09 actions with token-scoped targets and current disabled states.
- FR-14: Group menus MUST perform B10 captured-survivor actions, with Close all exclusively normal WM_CLOSE request dispatch.
- FR-15: Text tickers MUST implement B11 timing, clipping, measurement, cancellation and reduced-motion behavior.
- FR-16: Quick Launch MUST render the alphabetically sorted current Explorer pin snapshot with B12 navigation and dismissal behavior.
- FR-17: Quick Launch selection MUST validate its active session/allowed pin and close only after successful launch.
- FR-18: Quick Launch administrator selection MUST preserve B13 UAC/fallback outcome and focus-hold semantics.
- FR-19: Integration track MUST support real Processes/PID-menu entry into the existing Process Manager and return/dismissal behavior.
- FR-20: Integration track MUST retain the Process Manager's guarded termination identity checks and confirmation flow, separate from window close.
- FR-21: The prototype MAY include B15 dormant-action/order compatibility fixtures only after separate scope approval; they are OPTIONAL and excluded from required completion evidence.
- FR-22: Preference import MUST use a sandbox copy; supported theme/font, compact-density, strong-focus-ring, reduced-transparency, height/lock and optional launcher-order changes MUST NOT overwrite existing settings/localStorage.
- FR-23: Native shell mode MUST establish exclusive primary-bottom AppBar ownership with tracked Explorer/work-area restoration and fullscreen/DPI handling.
- FR-24: Fixture mode MUST be the default and MUST NOT install hooks, reserve AppBars, hide Explorer, or perform external actions.
- FR-25: Bar keyboard navigation and accessible names/status MUST cover B17 task-strip and open/close/empty/error states.
- FR-26: Disposal MUST cancel pending work, release thumbnails/windows and unregister owned listeners/hooks/registrations without accepting delayed callbacks.

## Non-Functional Requirements

All numbers here are **proposed budgets**, not observations. M0 freezes hardware and baselines; unmeasurable budgets trigger escalation, not invented passes.

- NFR-1: Visual comparison MUST record 100/125/150/200% DPI, 320/1280/1920 logical-width fixtures, default/min/max heights, all shipped themes and chosen fonts. Hit boxes, bar edges and preview/card geometry MUST match logical reference dimensions within 1 logical pixel after scale rounding; no clipping of Close targets. Raster/text antialiasing differences require annotated designer review, not a cross-renderer pixel-identical score.
- NFR-2: On the frozen 60Hz reference machine, active UI-frame p95 SHOULD be <=16.7ms and input-to-first-visible-response p95 <=50ms (excluding intentional hover delay and OS/app work), with neither >10% worse than measured matched baseline. Report p50/p95/max, missed frames, sample count and uncertainty; failed budgets require explicit go/no-go review.
- NFR-3: Stable idle MUST schedule no continuous redraw loop; animation-free fixture MUST have zero self-scheduled frames over 60 seconds after settle. Native worker wakeups and CPU MUST be reported separately and SHOULD not exceed matched baseline by >10% over five-minute idle runs. Hidden ticker/gallery MUST schedule no animation frames.
- NFR-4: Release-process start-to-input-ready p95 SHOULD be <=1 second warm and <=2 seconds cold on the reference machine, and <=baseline*1.10. “Ready” requires drawn controls plus usable fixture/native snapshot, not just window creation; OS launch latency is separate.
- NFR-5: S-track median process-tree private bytes SHOULD improve by >=20% against matched bar-only browser baseline to justify memory-driven continuation. Post-settle private-byte growth SHOULD be <=5% after 100 gallery/open-close/reorder cycles. I-track total memory MUST be reported independently; no whole-app saving claim follows from S. GPU/shared/working-set reporting MUST follow the benchmark method below.
- NFR-6: Keyboard-only users MUST reach every enabled visible action with no focus trap; selected gallery pair and Quick Launch list semantics MUST appear in native accessibility inspection. Narrator MUST announce names, counts, disabled states and diagnostics in the scripted journeys. Focus/control contrast SHOULD reach 3:1 and normal text 4.5:1; exceptions in imported themes require an explicit issue, never an automatic accessibility pass. Reduced motion MUST eliminate nonessential text/busy motion.
- NFR-7: Host action admission MUST reject every stale/malformed/unauthorized test case with zero external side effects; no arbitrary renderer/companion path, HWND or PID constitutes authority. Cross-process integration MUST use a user-scoped authenticated/versioned channel with allowlisted operations, bounded payloads and instance tokens, not a public generic command listener.
- NFR-8: Fixture stress MUST retain at most 12 thumbnail destinations, use bounded decoded-icon/texture caches with an M0-declared byte/capacity limit, and coalesce queued snapshots to latest state. Tests MUST show no unbounded queue/cache growth for 1000-window synthetic snapshots; this is stress evidence, not a promise to draw all thumbnails live.
- NFR-9: Normal native exit/startup-failure and consented crash/relaunch recovery MUST restore exact owned shell state in all scripted cases; any residual registration, hidden taskbar or work-area mismatch blocks further native runs. Watchdog/recovery detection SHOULD begin within 5 seconds; OS/session failure is explicitly outside that deadline guarantee.
- NFR-10: Evidence MUST be reproducible from revision, lockfile, build flags, OS/GPU/driver, DPI/font/theme/workload/consent and raw timestamped samples, with five repeat runs minimum per performance scenario and zero unsupported “native certified” claims from fixtures.

## Acceptance Criteria

Each criterion is parameterized by the inventory row's enumerated actions/states; the runner expands required criteria into distinct positive and negative tests before implementation. AC-21 traces optional FR-21 only: do not author/build/run that compatibility extension without separate approval, and its absence does not block requested rendered-functionality completion.

### AC-1: Visible composition (FR-1, NFR-1)
Given identical theme/font/task fixtures, When the prototype draws B01/B03 at the comparison sizes, Then control order, caps, padding and hit-box measurements match the declared geometry tolerance and annotated screenshots record raster differences.

### AC-2: Height lifecycle (FR-2)
Given locked or unlocked sandbox settings, When a pointer resize/up/cancel/lost-capture sequence occurs, Then locked state changes nothing and unlocked state coalesces native requests and persists the final clamped height without stale settings overwriting an active drag.

### AC-3: Snapshot ordering (FR-3)
Given current sequence 12 and modal/minimized/internal-window fixtures, When sequences 11 then 13 arrive, Then 11 is ignored and 13 supplies exactly the authoritative filtered tasks without starting a renderer poll.

### AC-4: Window toggle (FR-4)
Given active, inactive, minimized and owned-modal task cases, When each receives one tile activation, Then exactly one corresponding minimize or restore/focus request uses pre-click intent and subsequent snapshots reconcile optimistic state or visible failure.

### AC-5: Group/overflow state (FR-5)
Given 1/2/3/4/7-window groups and busy/attention/toast variants, When width or membership changes, Then current auto grouping, aggregate cues, neutral counts, caps and 24/48 pressure transitions match reference fixtures without stale galleries.

### AC-6: Drag versus click (FR-6)
Given captured group rectangles, When horizontal displacement is below/at/above 6px or capture is canceled, Then below-threshold activation occurs once, completed drag changes only group order and suppresses click, and cancellation restores captured order.

### AC-7: Hover crossing (FR-7)
Given pending preview requests, When hover changes or crosses tile/preview edges under a fake clock, Then 180/140ms lifetimes and newer request ownership hold with no obsolete show hiding a newer preview.

### AC-8: Preview actions (FR-8)
Given live/captured/unavailable previews including failed icons, When activation or separate Close is chosen by pointer/Enter/Space, Then maximize or the explicitly declared close policy runs once, imagery failure does not disable actions, and failures remain truthful.

### AC-9: Gallery lifecycle (FR-9, NFR-1)
Given 2/3/4/7-window groups and constrained monitor geometry, When hover expires at 300ms or capsule click toggles the gallery, Then Ribbon fits only qualifying 2/3 groups, otherwise Matrix has <=3 columns/2 visible rows and a 12px gutter, nonce-current layout wins and crossing cancels 140ms close.

### AC-10: Thumbnail races (FR-10, NFR-8)
Given reordered/scrolled/partially clipped frames and changed HWND/PID identity, When old revisions/requests and menu-clear responses complete out of order, Then only current fully contained <=12 destinations survive, changed identities lose captures, and menu appears only after matching empty acknowledgement; failed capture never removes windows.

### AC-11: Spatial focus (FR-11)
Given an open Matrix with a short final row and selected Close focus, When arrows/Home/End or a same-session refresh occurs, Then spatial stride/clamping and scroll-to-selection hold, only selected activation/Close enter Tab order and hover never changes selection/focus.

### AC-12: Gallery actions/menu (FR-12)
Given a current gallery item, When Close is repeated or right-click/Shift+F10 followed by Restore/Switch/Minimize/Escape occurs, Then duplicate Close is blocked, corresponding validated action runs, and Escape restores item focus before any gallery dismissal.

### AC-13: Standalone menu (FR-13)
Given a task menu token and missing/valid PID or minimized state, When each B09 action or dismissal is selected, Then disabled actions cannot dispatch, stale tokens cannot act, enabled actions route to the declared service, and only dismissal restores origin focus.

### AC-14: Captured group close (FR-14)
Given a captured group with one departed/reused identity, one new member, and one unsaved document, When Close all is selected, Then native identity revalidation excludes departed/reused/new targets, surviving captured targets receive normal WM_CLOSE, unsaved prompts can remain, failures do not stop other requests and no termination/elevation fallback occurs.

### AC-15: Ticker (FR-15)
Given fractional-width overflow, fitting titles and reduced motion, When hover/focus/resize/title-change/time advance occurs, Then 600ms/40px-per-second/2s/3s phases match fake-clock assertions, only text moves, and leaving both engagements or reducing motion restores static accessible full text.

### AC-16: Quick Launch UI (FR-16)
Given sorted pins or an empty list and an active nonce, When toggle/pointerdown-close/arrows/Home/End/Escape/blur occurs, Then B12 navigation and dismissal match reference, trailing clicks do not reopen and obsolete close events do not close a newer session.

### AC-17: Quick Launch selection (FR-17)
Given valid, removed, renamed and outside-pin-folder shortcut cases, When selection is attempted, Then allowed current canonical pin launches once and closes on success; denied/failed launch retains a retryable panel without removing pins.

### AC-18: Administrator outcome (FR-18)
Given menu keyboard/pointer entry and UAC denial or WindowsApps fallback, When Run as administrator is selected, Then consented OS execution reports its actual result, cancel preserves retry state and unelevated fallback is not described as elevated.

### AC-19: Process integration (FR-19)
Given I track with connected panel and S track without it, When Processes or group/task PID entry is used, Then I opens real metrics/PID selection and returns/dismisses correctly; S reports capability unavailable and is labelled incomplete for connected functionality.

### AC-20: Guarded termination (FR-20, NFR-7)
Given a disposable process selected in the real panel, When confirmation is canceled, target identity changes, or consented confirmation succeeds, Then cancel/identity mismatch sends no kill and only the still-current confirmed disposable process is ended; window Close all never enters this route.

### AC-21: Optional dormant compatibility (FR-21)
Given separate approval for a specified B15 compatibility extension, When its approved fixture-only actions/order cases are exercised, Then the simulated ledger matches those validated service contracts without external effects or a new default launcher rail. Without that approval this criterion is not applicable and supplies no required completion evidence.

### AC-22: Preferences (FR-22)
Given copied settings including malformed/stale supported UI fields, When themes, UI preferences or height/lock are changed and the prototype relaunched, Then sandbox reconciliation persists only supported prototype fields and checksums of original settings/storage remain unchanged. Launcher-order cases apply only to separately approved optional B15 compatibility, not required preference acceptance.

### AC-23: Shell lifecycle (FR-23, NFR-9)
Given approved native testing with one owner, When AppBar startup, DPI resize, fullscreen entry/exit, Explorer recreation, startup failure or crash/relaunch recovery is exercised, Then negotiated primary-bottom placement/work area and exact owned restoration match recorded OS state; failures disable reactivation until cleanup succeeds.

### AC-24: Safe default (FR-24, NFR-7)
Given a default launch and fixture interaction script, When every UI action is exercised, Then no hooks/AppBars/Explorer/pin/filesystem-launch/process side effects occur outside sandbox and action-ledger output is clearly simulated.

### AC-25: Bar accessibility (FR-25, NFR-6)
Given keyboard-only, Narrator, forced-colors and reduced-motion cases, When every enabled visible journey is traversed, Then B17 focus/scroll/status plus native names/counts/disabled states are inspectable with no trap; contrast results and exceptions are recorded rather than inferred from AccessKit availability.

### AC-26: Disposal (FR-26)
Given outstanding open/snapshot/thumbnail/menu operations, When the owning surface/session exits, Then registrations/resources are released and late completions cannot show windows, dispatch actions or repaint.

### AC-27: Responsiveness/idle/startup (NFR-2, NFR-3, NFR-4)
Given frozen release scenarios and matched baseline, When five repeat active/idle/cold/warm runs are sampled, Then frame/input/startup percentiles and idle redraw/wakeup counters are reported against each budget, with intentional waits separated.

### AC-28: Memory/reproducibility (NFR-5, NFR-10)
Given matched S and separate I scenarios with five repeats, When process/GPU samples and 100 churn cycles are collected, Then raw artifacts support median savings/retained-growth and repeatability gates without adding shared working sets as unique RAM or attributing I to S.

## Edge Cases

- EC-1: HWND disappearance/reuse, PID reuse, modal/root-owner changes, denied process inspection: native identity (including immutable process identity where available) revalidation rejects unsafe actions. HWND+PID continuity in UI caches alone is not a PID-reuse defense (AC-4/10/14/20).
- EC-2: Empty pins, broken .lnk, missing icon, failed resolution/extraction, removed shortcut, pin-folder access denied: generic icon/status, keep row after launch failure, canonical pin-root validation (AC-16/17). Outside-root unpin rejection applies only to a separately approved optional B15 extension (AC-21).
- EC-3: UAC cancel/policy, ShellExecute failure and WindowsApps fallback: no retry loop or elevation claim; existing narrow fallback distinguished from success as administrator (AC-18).
- EC-4: DWM unavailable/composition changes, minimized/protected windows, capture failure, GPU device loss: fall back to captured/unavailable, release registrations, retain actions; no fake live marker; recovery cannot apply stale images (AC-8/10/26).
- EC-5: Slow/hung app close and unsaved-work prompt: group request-only close does not wait/kill; report counts. **Existing standalone Close is force-capable** through `TaskWindowAction::Close`; preview/gallery paths require auditing that shared helper in M0. Native execution of these paths stays disabled until user approves either exact existing behavior on disposable targets or a documented safer request-only prototype deviation (AC-8/12/13/14).
- EC-6: Snapshot/nonce/revision/request overflow, late replies, mismatched menu tokens, missing positive identity: retire session and recreate safely, never wrap IDs or revive stale selection/menu/image (AC-3/7/10/26).
- EC-7: Pointer capture loss, Escape during drag, membership/width change mid-drag, fractional DPI rounding: rollback/reconcile survivors, suppress accidental trailing actions, single host logical-to-physical conversion (AC-2/6/9).
- EC-8: AppBar conflict, partial registration, Explorer restart, fullscreen placement/restoration failure: retain cleanup ownership and error details; no second registration or blanket reveal of taskbars this prototype did not hide (AC-23).
- EC-9: Sandbox storage read/write failure or incompatible copied settings/theme/font: explicit fallback defaults, preserve source copy, runtime changes usable without claiming persistence; no migration-in-place (AC-22).
- EC-10: Companion absent/disconnected/version mismatch/authentication failure/PID stale: capability error, close pending intent, no public fallback IPC; real panel unavailable means I acceptance incomplete (AC-19/20).
- EC-11: Narrow viewport, 1000 tasks, no space for native gallery, display scale changed during show: measured clamped layout, preserve Close size, bounded previews, offscreen tasks remain reachable; no secondary-monitor ownership implied (AC-5/9/10/25).
- EC-12: Session crash, failed watchdog restore or OS shutdown: durable minimal ownership recovery record and startup refusal if unresolved; consented recovery steps documented before shell activation. Rust Drop alone cannot recover a terminated process (AC-23/26).

## API Contracts

Illustrative **host-independent** TypeScript notation; these are proposed Rust trait/message shapes, not new HTTP endpoints or existing Tauri signatures. HTTP is N/A: adding an HTTP endpoint merely to satisfy a generic spec validator would invent an attack surface. Rust maps IDs to newtypes, operations/errors to enums, optional fields to `Option`, results to `Result`; no stringly typed generic invoke or production `unwrap` on expected OS failure.

```ts
type Id = string; // opaque, host-issued, nonempty; never raw renderer authority
type Generation = number; // positive safe integer on wire; checked u64 internally
interface LogicalRect { left: number; top: number; width: number; height: number }
interface PhysicalRect { leftPx: number; topPx: number; widthPx: number; heightPx: number }
interface WindowRef { id: Id; identityGeneration: Generation }
interface TaskWindow { ref: WindowRef; title: string; processName: string;
  processId: number | null; iconId: Id | null; active: boolean; minimized: boolean;
  attention: 'none' | 'requested'; activity: 'idle' | 'busy'; toastCount: number }
interface Launcher { id: Id; name: string; detail: string; iconId: Id | null }
interface Snapshot { sequence: Generation; windows: TaskWindow[]; launchers: Launcher[] }
interface Session { nonce: Id; revision: Generation; requestId: Generation }
type WindowAction = 'toggle' | 'focus' | 'minimize' | 'maximize' | 'close'
  | 'request-close' | 'pin' | 'process';
// Required Quick Launch actions: launch/runas. Remaining B15 actions are
// optional compatibility capabilities, absent unless separately approved.
type LauncherAction = 'launch' | 'runas' | 'properties' | 'reveal' | 'revealTarget'
  | 'copyPath' | 'unpin';
type Request =
  | { kind: 'snapshot'; refresh: boolean }
  | { kind: 'window'; target: WindowRef; action: WindowAction; preClickActive?: boolean }
  | { kind: 'group'; menuToken: Id; action: 'process' | 'pin' | 'closeAll' }
  | { kind: 'launcher'; launcherId: Id; action: LauncherAction; session?: Session }
  | { kind: 'quickLaunch'; operation: 'open' | 'hide' | 'blur'; session?: Session; anchor?: LogicalRect }
  | { kind: 'preview'; operation: 'show' | 'hide'; requestId: Generation;
      target?: WindowRef; anchor?: LogicalRect }
  | { kind: 'gallery'; operation: 'open' | 'refresh' | 'hide' | 'blur';
      session: Session; groupId: Id; anchor?: LogicalRect; focus: boolean }
  | { kind: 'thumbnails'; session: Session; viewport: LogicalRect;
      frames: { target: WindowRef; bounds: LogicalRect }[] }
  | { kind: 'menu'; operation: 'open' | 'dismiss'; menuToken: Id;
      target: WindowRef | { groupId: Id } | { launcherId: Id }; anchor: LogicalRect }
  | { kind: 'processPanel'; target: WindowRef | null; anchor: LogicalRect }
  | { kind: 'preferences'; patch: Partial<Preferences> }
  | { kind: 'shell'; operation: 'activate' | 'resize' | 'cleanup'; heightLogical?: number };
// launcherOrder stays empty/inactive unless optional B15 compatibility is approved.
interface Preferences { schema: 1; themeId: string; fontId: string; heightLogical: number;
  heightLocked: boolean; launcherOrder: Id[]; reducedMotion: boolean;
  compactDensity: boolean; strongFocusRing: boolean; reducedTransparency: boolean }
interface GalleryLayout { mode: 'ribbon' | 'matrix'; columns: number;
  card: LogicalRect; thumbnail: LogicalRect; viewport: LogicalRect; panel: LogicalRect }
interface Thumbnail { target: WindowRef; source: 'live' | 'captured' | 'unavailable';
  imageId?: Id; diagnostic?: string }
interface HostError { code: 'unavailable' | 'stale' | 'invalid' | 'denied' | 'busy'
  | 'canceled' | 'osFailure' | 'cleanupRequired'; operation: string;
  diagnostic: string; retryable: boolean }
type Result<T> = { ok: true; value: T } | { ok: false; error: HostError };
interface ActionOutcome { status: 'simulated' | 'requested' | 'completed';
  requested?: number; failed?: number; skipped?: number; elevated?: boolean }
type Response = Result<Snapshot | GalleryLayout | { session: Session; thumbnails: Thumbnail[] }
  | Preferences | ActionOutcome>;
interface Host { dispatch(request: Request): Promise<Response> }
```

An empty `thumbnails.frames` response is the matching clear acknowledgement, not an arbitrary successful dispatch. Group menu tokens bind host-captured members and representative active-at-open. Shell activation and destructive/launch actions require explicit mode capabilities; fixture host records simulated outcomes only. The host owns HWND/process-start identity, canonical pin paths, session allowed-pin sets, image handles and DPI/monitor origin; UI receives IDs and logical geometry. Host performs scale conversion once, rounding edges consistently. Minimize and maximize are distinct operations. Success of WM_CLOSE means **requested**, not “window gone.” ProcessPanel never accepts an unvalidated raw PID from a companion. A process termination endpoint is intentionally absent here: the existing Process Manager retains its own confirmed immutable-identity service boundary in I.

## Data Models

| Entity / field | Type | Constraints/lifetime |
|---|---|---|
| TaskWindow / ref, process identity | WindowRef; native HWND/PID/start identity | Native registry authoritative; UI ref opaque; invalidate reused/departed targets |
| TaskWindow / title, processName, iconId, state | string; optional image ID; typed state/count | Title fallback process name; bounded icons; no attention/busy inferred beyond current rules |
| TaskGroup / id, members, order, aggregate | Id; WindowRef[]; integer; B03 state | Process-name normalization with HWND fallback; session task order, max toast count |
| Snapshot / sequence, tasks, pins | Generation; arrays | Strictly advancing; coalesced latest snapshots; 1000-window stress workload |
| Launcher / id, name, detail, icon | Id; strings; optional Id | Native canonical shortcut mapping; current Explorer source; fallback icon |
| QuickLaunchSession / nonce, allowed pins, selection flight | Id; set<Id>; boolean | Commit after native open succeeds; clear on close; canonical allowed set |
| Preview / request, target, source, image, anchor | Generation; WindowRef; enum; optional Id; LogicalRect | Global monotonic request ownership; live/captured/unavailable truthful |
| GallerySession / nonce, revision, request, layout, selected ref | Session; GalleryLayout; optional WindowRef | Compare revisions within nonce only; retain surviving focus and identity |
| ThumbnailDestination / target, frame, registration | WindowRef; LogicalRect; native handle | Fully contained, <=12; retire before mutation, clear before menu overlay |
| CapturedImage/Icon / ID, bytes, generation | Id; bounded bytes; Generation | Cache limits frozen M0, native handle RAII, no changed-identity retention |
| MenuIntent / token, kind, captured targets, representative | Id; enum; refs; optional ref | Stale token rejected; exclude new group members; dismissal focus separate |
| Drag / pointer, captured rects/order, delta, suppression | IDs; rects; arrays; number | Single gesture owner, 6px threshold, cancel rollback, exactly-once action |
| Ticker / measured overflow, phase, deadline, engagement | number; enum; monotonic time; booleans | Local clipping; no frame while settled/hidden/reduced motion |
| Preferences / schema, theme/font, height/lock/order/motion | Preferences | Sandbox version 1; height finite 24–120; launcher order empty/inactive unless optional B15 approved; then reconcile missing/duplicate IDs |
| ShellLease / owner, baseline, registrations, Explorer identities, recovery | native registry + minimal recovery record | Exclusive primary bottom; only restore owned effects; failed cleanup blocks new lease |
| ProcessHandoff / validated target, capability, instance | optional WindowRef; bool; Id | I-only real panel, S truthful unavailable; PM kill stays separate |
| EvidenceRun / environment, samples, artifacts, consent | manifest + timestamped data | Revision/versions/flags, reproducible inputs, sensitive paths redacted |

## Architecture and host gate

### Visual and timing baseline to freeze

Translate resolved current theme tokens, not approximate hardcoded dark colors: `--js-bg-bar`, surface/overlay/control/hover/active/selected colors, text/muted/strong, border/soft/accent, raised shadow/inset highlight, focus ring and `--js-font-sans`. Source references are `BottomBar.css`, `TaskPreviewSurface.css`, `TaskGallerySurface.css`, Quick Launch component styles, `src/lib/themes.ts` and shared global styles. Designer records exact resolved values and font files/fallback before comparison; importing the CSS token names does not mean egui renders CSS.

Current baseline: bar padding 2px vertical/4px horizontal, 3px gaps, 2px task/control rounding, tile icons 0.74rem and neutral count; active tile face mixes bar/accent 76/24 versus idle bar/text 93/7, with theme inset bevel and 140ms control transitions. Busy strip animates a 1.15s cycle only for eligible activity, reduced motion is static. Gallery chrome is 8px rounded/shadow-free at native host, 8px inset/gaps, 24px header, 180x136px cards, thumbnail offset (4,4) and 172x96 size, 32px footer and separate **24x24px Close**; constrained layouts never shrink that target. Preview host 332x228, content inset left/right/bottom 12 and top 48 logical pixels, aspect-fit imagery and 8px edge gap, clamped placement wins. Counts/ticker remain local to their labels; no connector seam. Font rem values require actual baseline root-font resolution, not assuming a 16px root. Source geometry/timing is authoritative; fixed-clock tests validate timer phases, while presentation measurements allow frame-quantized scheduling and record actual latency separately.

M0 records deviations (for example missing equivalent font weight or theme contrast) and user approves purposeful differences before acceptance. Aim for recognizable parity first, not a new visual design presented as renderer comparison.

All files below are **proposed, nonexistent**. No production extraction is approved by this draft.

```text
experiments/egui-bottom-bar/
  Cargo.toml, Cargo.lock          # separate manifest; no Tauri dependency in S
  src/main.rs, lib.rs             # minimal executable and composition
  src/model.rs, reducer.rs        # typed states, pure grouping/focus/timer reducers
  src/host.rs, fixture_host.rs    # trait/capabilities; deterministic safe default
  src/ui/{bar,quick_launch,preview,gallery,menus,ticker,theme}.rs
  src/native/{window_host,shell_lease,tasks,launchers,thumbnails,recovery}.rs
  tests/                         # runner-owned RED-first contracts/integration
  fixtures/                      # deterministic pin/task/theme/scenario data
  tools/                         # consent-gated native E2E/benchmark runner
experiments/egui-bottom-bar-integration/
  Cargo.toml, src/                # optional I companion/Tauri-facing adapter
docs/evidence/egui-bottom-bar/    # future manifests, report/index; raw data external
```

Start drawable fixtures with eframe + wgpu. At M2 choose **eframe with native hooks** only if the pinned version allows reliable HWND acquisition, subclass/message handling, per-window activation policy, transparency/composition, DWM destinations, AppBar callbacks, UI-thread OLE/COM initialization, and recoverable teardown. Otherwise use custom winit + egui-winit + egui-wgpu with explicit Win32 host ownership. A narrow host experiment decides this before implementing all surfaces; native-looking widgets do not decide shell feasibility. Keep one GPU device for multiple native viewports where supported; measure actual surface/texture cost. Glow is a fallback experiment only if wgpu compatibility fails, with separate measurements, not a parallel renderer project.

Official documentation checked through Context7 on 2026-10-07:

- [eframe README](https://github.com/emilk/egui/blob/main/crates/eframe/README.md): native egui-winit; wgpu default and glow option; no WebView requirement.
- [native wgpu integration](https://github.com/emilk/egui/blob/main/crates/eframe/src/native/wgpu_integration.rs), [multiple viewports](https://github.com/emilk/egui/blob/main/examples/multiple_viewports/src/main.rs), [egui-wgpu painter](https://github.com/emilk/egui/blob/main/crates/egui-wgpu/src/winit.rs): separate native windows/surfaces, shared device; not shell ownership.
- [winit WindowAttributes](https://docs.rs/winit/latest/winit/window/struct.WindowAttributes.html), [Windows extension](https://docs.rs/winit/latest/winit/platform/windows/trait.WindowExtWindows.html): transparency, skip-taskbar and undecorated shadow controls/raw-handle access. These do **not** prove AppBar/noactivate/DWM/OLE behavior out of the box. Pin compatible crate versions/lockfile at M0 and recheck exact APIs before use.

Reuse strategy: inventory Tauri coupling per existing module first. `task_windows/` has useful native helpers, but snapshot emission/authorization and `task_preview.rs`/`task_gallery.rs`/`appbar.rs` use Tauri handles/windows. Do not import the whole backend binary or include entire `.rs` modules by path into S. Extract only reviewed host-independent helpers into a small shared library in a separately authorized future patch, with existing adapters unchanged in behavior and differential tests; pure frontend reducers translate into Rust with golden vectors from TS. If extraction requires a shipping-shell contract change, stop for approval. Do not maintain a gratuitous second launch/identity/rollback implementation. Temporary wrappers are acceptable only with a documented ownership/deletion plan and a reuse test.

I requires a reviewed integration seam: the current `show_process_manager` takes an AppHandle and current commands enforce caller surfaces; a standalone executable cannot invoke those functions directly. In addition, `src-tauri/src/process_manager.rs:112–128` looks up the existing bottom-bar **WebviewWindow** for current monitor, outer position and outer size; its subsequent placement combines that physical geometry with logical anchor values and scale. Suppressing normal-bar startup alone therefore does not make panel placement usable. The companion needs a reviewed host geometry adapter that obtains trusted live egui bar/monitor geometry, performs the single logical/physical conversion and preserves clamping/anchoring without a dummy owning WebView bar or renderer-supplied geometry authority. M5 validates button/PID-menu placement, nonzero monitor origin, DPI changes, stale/unavailable bar geometry and failed handoff; this is a future seam prerequisite, not a blocker to this draft. A companion must host the existing panel in a non-owning-shell configuration and admit only authenticated prototype handoff. If startup or geometry cannot be decoupled safely, M5 stops: approve a minimal adapter/startup/geometry seam or native PM scope separately. Never weaken existing caller checks to labels supplied by external IPC. Existing top-bar, terminal, search, Stack Browser/editor/Git and tray remain unchanged.

Activation policy is per surface: current shell bars clear `WS_EX_NOACTIVATE` (`shell_windows.rs`), whereas hover helper previews should not steal foreground. Keyboard-invoked gallery/Quick Launch/menu must intentionally accept focus and restore it on dismissal. Do not indiscriminately make every HWND noactivate. Native transparent preview/gallery plus DWM composition/menu z-order needs observed certification, not a CSS translation promise. OLE outbound drag is a Stack Browser dependency only if that surface is added later; task reorder itself is captured pointer handling, not OLE. Pin launch/properties still need correct Shell/COM/thread ownership.

## Safe execution, isolation and rollback

Default fixture window is an ordinary resizable native app, with demo fixtures and no native hooks, ShellExecute, process/pin modifications or work-area changes. “Simulated” is visible in the demo and action ledger. Optional read-only native snapshots are a separate explicit mode; enabling data reads does not authorize actions/shell mutation.

Before any shell run, obtain explicit human consent for the named scenario and restore procedure. Record foreground, monitor scale/work area, Explorer identities/visibility, existing JasonShell ownership and sandbox checksum. Use a process/user-scoped exclusive lease: **one bottom owner only**. Normal old JasonShell must shut down and verify cleanup before S becomes a bottom owner; it owns top and bottom jointly today, so do not assume a bottom-only handoff exists. S reserves only the primary bottom edge, with Explorer left visible initially; any suppression experiment requires separate consent and exact owned restoration records. This bottom-only setup is not identical to the full old shell and must be labelled in comparisons. I companion owns no AppBars. No simultaneous old/new bar shell owners, no silently duplicated hooks/guards.

Before Explorer suppression, implement/verify a separate recovery watchdog or equivalent recovery process, minimal per-user recovery record, explicit emergency restore command and startup refusal on unresolved residue. Record only identities/baseline necessary for recovery, not arbitrary task histories. Kill-test only the experimental process on consented disposable desktop; watchdog restores what that owner actually changed. Normal cleanup uses RAII plus retryable ownership records; crash recovery cannot rely on destructors. Revalidate Explorer recreation identity and retain partial rollback ownership. Fullscreen release/restore and negotiated AppBar rectangles are authority; WebView parking workaround is not automatically needed by egui. Preserve the old primary-monitor safety contract without copying WebView-specific implementation blindly.

Persist under a distinct prototype profile. Copy supported settings with a documented schema transform and preserve the original hash; optional launcher-order compatibility import and required UI-preference import use explicit export/read-copy, not assumed Rust access to localStorage. Launcher-order import belongs only to separately approved B15 compatibility, not required parity. Map only fields that affect these surfaces; date/time/editor preferences do not create new bar controls. Custom font CSS URLs are not native font data: use verified local font assets or explicit fallback and record the visual deviation, never fetch/execute imported remote CSS. No overwrite/export-back by default; remove sandbox to reverse. Tasks/groups stay session-memory only. Consented Pin actions (and Unpin only in an approved optional extension) alter real Explorer pins, so test on temporary disposable shortcuts with restoration manifests; sandbox preferences alone do not undo OS pins or launched apps.

## Staged implementation plan

Effort = rough person-days for an experienced Rust/Win32 + UI pair, excludes approval waits/hardware setup. Dependencies/host uncertainty can double native phases; not a delivery promise. Runner owns tests/artifacts, designer owns visual/UI code, fixer owns nonvisual/native code, independent reviewer owns read-only review. Leader owns scope/consent and approval gates. No stage starts before spec approval; tests RED-first per stage.

| Milestone / effort | Depends on / owner | Deliverable and validation | Stop condition |
|---|---|---|---|
| M0 Baseline/spec freeze / 1–2d | User approval; Leader + runner/designer | Freeze source inventory, exact close policies, versions/fonts/cache limits, hardware, scripts and workload; map required AC/EC to future tests, mark FR-21/AC-21 optional/not applicable unless separately approved; consent-free baseline fixture plan, consented baseline runs separately | Missing actionable contract, unaudited force-close route, unknown measurement path, or requested shipping change |
| M1 Drawable interactive fixtures / 2–4d | M0; runner RED then designer/fixer | Bar/Quick Launch, themes, states, ticker, focus, geometry, demo ledgers; fake-clock/golden-vector tests; screenshots and accessibility inspect | Extra invented control, simulation presented as live, inaccessible keyboard route |
| M2 Native host capability gate / 2–5d | M1; fixer + runner | Tiny consent-free native window/composition fixtures; host matrix for raw HWND, styles, focus, transparent DWM destination/menu overlay. Consent separately for AppBar/recovery trial. Decide eframe or custom host and extraction seam | Cannot meet cleanup/focus/thumbnail semantics; no hook/subclass ownership proof; fallback needs scope approval |
| M3 Launch/tasks/actions / 3–5d | M2 gate; fixer + runner | Shared native snapshot/identity/launcher adapters, real pin listing and consented rendered launch/admin/task-action scenarios; bounded caches, sandbox import; differential existing backend contract tests. No B15 dormant-action/order implementation required | Stale target allowed, launch/path validation weakened, callbacks poll redundantly, close policy unresolved |
| M4 Preview/gallery/menu/reorder / 4–7d | M3; designer + fixer, runner first | Native owned surfaces and thumbnail adapter, all timing/race/focus/close/menu tests; active/live/captured/unavailable screenshots, stale identity/scroll/menu-clear cases | Native DWM covers menu, changed identity retains image, close forces group termination, target clipped |
| M5 Connected flows / 2–4d | M4 + approved I startup/geometry seam; fixer/designer + runner | Reviewed trusted egui-bar/monitor geometry adapter replaces existing bottom-bar WebviewWindow placement dependency; test DPI/origin/stale geometry and button/PID-menu anchors. Real Process Manager/PID metrics/filter/sort/refresh/close/confirmed disposable termination journeys through existing panel; capability failure tests; I overhead labelled separately | Companion needs dual AppBar ownership, auth weakening, or unapproved startup/geometry refactor; S alone not claimed full parity |
| M6 Native E2E / 2–4d | M3–5 + explicit human consent; runner + user | Shell lifecycle, fullscreen, scale, Explorer recreation, input/focus, launch/UAC and disposable close/kill E2E; watchdog crash/relaunch/restore; Narrator/forced colors and manual UX review | Any unsafe residue/identity/action or incomplete critical journey; refuse further native activation |
| M7 Benchmark/report/go-no-go / 1–3d | M6 valid evidence; runner + designer, reviewer independent | S/I isolated release repeated runs, raw manifests/screenshots/profiles, budgets and uncertainty, manual look/feel rating; review reconciles required AC/EC and evidence gaps, marks optional AC-21 not applicable unless separately approved | No matched baseline, savings attribution invalid, safety failure or unresolved required AC; choose revise/stop, not rewrite shell |

Total indicative 17–34 person-days with existing services reusable; first safe drawable sample after M1, first useful measured native sample after M2–4. Early samples are deliberately partial and labelled, not substitute scope completion. Reviewer at each risky host seam and final M7; no review swarm or duplicate test ownership.

## Verification and runnable evidence path

**Current work:** documentation only. E2E **skipped** because no runtime behavior, surface or test artifact is implemented. Run focused documentation truth/policy plus local links/size and spec structure/manual traceability checks. Do not install crates, launch Tauri, reserve AppBars or manufacture routine tests for this draft.

**Future prototype:** E2E **required** because shell ownership, native window focus, cross-surface Process Manager and destructive/window-close journeys cross boundaries that reducers and static source contracts cannot prove. Runner authors RED-first Rust model/fixture tests, adapter integration tests with controlled Win32 test windows/processes, and consented native E2E. Browser parity fixtures remain baseline evidence only; browser automation cannot certify native egui input or DWM. Use native OS state/readback, frame instrumentation, native accessibility tree, screenshots, disposable test apps and timestamped action/registration ledgers. Test-only fixture control/ledger exists only in experimental binary, bounded and path-redacted; not a public production diagnostic API.

Existing evidence to reuse as reference: `tests/taskbar*.test.mjs`, `tests/bottomBarPreviewRequestId.test.mjs`, gallery layout/runtime contracts, `tests/components/task-gallery-runtime.test.ts`, `taskbar-group-menu.test.ts`, `overflow-ticker.test.ts`, browser gallery/inset-preview fixtures and Rust native tests. Run focused existing checks after any authorized shared-helper extraction, plus `npm run check`, `npm run build`, `npm run cargo:check`, `npm run cargo:test` and broader `npm run validate` when the future adapter touches shipping boundaries. Existing scripts remain exactly as README/package.json; no present egui commands are claimed.

Future command examples — **proposed; manifests, flags and scripts do not exist yet**, not instructions to execute now:

```powershell
cargo test --manifest-path experiments/egui-bottom-bar/Cargo.toml
cargo fmt --manifest-path experiments/egui-bottom-bar/Cargo.toml --check
cargo clippy --manifest-path experiments/egui-bottom-bar/Cargo.toml --all-targets -- -D warnings
cargo run --release --manifest-path experiments/egui-bottom-bar/Cargo.toml -- --mode fixture --scenario gallery-churn
# Only after explicit scenario consent and recorded rollback readiness:
pwsh -File experiments/egui-bottom-bar/tools/native-e2e.ps1 -Scenario ShellLifecycle -ConsentRecord consent.json
pwsh -File experiments/egui-bottom-bar/tools/benchmark.ps1 -Track S -Manifest run.json
```

Verification budget/owners: runner establishes AC-2–20/22–24/26 functional+safety evidence and AC-27/28 instrumentation; optional AC-21 receives no required test/artifact allocation unless separately approved. Designer establishes AC-1/9 visual geometry and AC-25 focus/contrast/tree evidence with user Narrator/UX review; fixer supplies observable native state/capability hooks, not self-certification; reviewer checks traceability and safety attribution independently. User assesses feel with a fixed task sheet: find/launch pin, switch/minimize/maximize, cross hover gap, navigate 7-window gallery, close prompt-bearing document, reorder task groups, inspect PID process, dismiss panels. Record completion/errors plus 1–5 density/readability/predictability/smoothness ratings; proposed go gate no worse median rating than baseline and zero unreachable enabled actions, with preference disagreements documented. Subjective ratings complement, never replace measurable ACs.

## Performance experiment and decision gate

Use sequential controlled A/B (alternate baseline/prototype run order), release builds only, same monitor/DPI/font/theme/tasks, power mode, GPU/driver and tracing overhead. Stop one shell owner and verify rollback before starting another. Normal full Tauri shell vs isolated S is **not** a matched renderer-only comparison. M0 prepares an approved baseline bar-only harness importing production BottomBar/owned surfaces with matched host fixtures and lazy surface policy, or reports that no fair bar-only baseline is obtainable. Full shell baseline can still measure actual system impact but must identify extra top/panel/services and cannot isolate renderer savings. Do not subtract shared WebView processes by guesswork.

Workloads: empty/12 pins + 1/3/12 tasks; 100-task and 1000-task synthetic stress; 2/3 Ribbon; 4/7/20 Matrix scroll; label ticker/focus; 100 rapid gallery/preview/menu open-close cycles; snapshot churn/reused identities; five-minute settled idle; 30-second continuous reorder/resize; cold and warm startup; I panel-open metrics/PID lookup/dismissal with same existing panel state. Consented real action scenarios are separate from performance replay; benchmark fixtures never kill processes or modify pins. Sample >=100 interactions per active latency case, >=300 active frames per repeat, five repeats; cold runs document cache/reboot state rather than falsely clearing OS caches. Record idle warming/lazy panel behavior explicitly.

Collect process-tree **private bytes** (commit attributable to each process), working set and private working set separately for every owned helper/companion and WebView2 descendant; document parent/start identity and shared WebView host attribution uncertainty. Working-set sums can double-count shared pages and are not physical unique RAM. Record GPU dedicated/shared allocation per process plus adapter-level allocation/control workload; allocation is not exact unique VRAM residency. Distinguish native textures/DWM registrations, decoded images and driver overhead where measurable, otherwise mark unknown. Record CPU time, worker wakeups, idle self-repaints, frame CPU/GPU times where available, input event to first presented visual acknowledgement and ready times; timestamps use one monotonic clock/correlation scheme. Median across repeat-run summaries plus p50/p95 distributions/error bars, raw samples and peak retained memory—not one Task Manager screenshot.

Artifacts: future evidence manifest/report under `docs/evidence/egui-bottom-bar/`, raw CSV/JSON/traces/screenshots under approved external artifact directory with links and hashes. Include revision, dependency lockfile hash, workload seed, build flags, renderer, surface count, process tree, sampling interval/tool limitations and consent scope. Retain baseline/prototype both, including failures. Repeat evidence only when relevant code/inputs/environment change; do not rerun unaffected broad suites for presentation edits.

**Go:** all requested native bar and connected I functionality passes required mapped AC/EC (FR-21/AC-21 optional B15 compatibility explicitly excluded), native safety evidence has no unresolved defect, visual/a11y/feel gate accepted and proposed NFR budgets pass or user explicitly accepts a documented tradeoff. **Revise:** visual/interaction or host bottleneck with bounded repair hypothesis. **Stop:** unsafe ownership/actions, missing connected flow without approved seam, or no credible benefit after fair measurement. Missing >=20% S memory improvement is a failed memory-driven hypothesis, not proof egui is unusable; user can elect a look/feel-led continuation explicitly. No automatic whole-shell migration follows any outcome.

## Out of Scope

- OS-1: Full JasonShell migration or replacement of top bar/search/tray/audio/calendar/speech/snipping/workspaces — these are not bottom-bar controls and would invalidate a bounded comparison.
- OS-2: Native rewrite of Stack Browser, Git workbench, text editor or terminal — no direct bottom-bar dispatch was found; OS-launched pins remain external applications. If a new bottom entry is desired, approve new inventory/connected-flow acceptance first (including Stack dirty-exit and OLE dependencies).
- OS-3: Multi-monitor AppBar ownership — current primary-monitor boundary retained; scale fixtures do not establish multi-monitor shell support.
- OS-4: Whole-app browser-free claim — I intentionally retains Process Manager WebView; native Process Manager parity needs separately approved expansion including metrics/group/filter/sort/refresh/guarded termination/accessibility and budgets.
- OS-5: New launcher rail, desktop button, new pin database or persistent task-group order — dormant handlers and historical docs are not current visible controls. B15 dormant launcher actions/order compatibility is excluded from required implementation and completion evidence; optional FR-21/AC-21 needs separate approval, not inferred permission from this inventory.
- OS-6: Shipping-shell refactor, dependency install, runtime launch, benchmarks or live smoke during this documentation request — only this draft and docs index are authorized now.
- OS-7: Automatic workspace restoration/startup commands or automation forwarding — reserved/not wired current boundaries unchanged.

## Risks and approval prerequisites

1. **Approve the recommended S+I scope** versus a fully native Process Manager expansion. Without I, samples stay partial for “all functionality”; this is the only meaningful connected-panel expansion currently identified.
2. **Close policy decision before native actions:** source separates group request-close from force-capable standalone Close. Recommend request-only prototype safety deviation for standalone/preview/gallery Close, clearly labelled against baseline; exact parity alternative requires explicit consent/disposable targets. This draft does not silently change the old shell.
3. **Approve dependency/build and any shared extraction/startup/geometry seam separately** after M0/M2 audit; pin versions then. eframe capability fit, DWM composition, font parity and AccessKit output are technical gates, not assumed user decisions or proven outcomes. B15 compatibility is not a prerequisite; request separate approval only if proposing that optional extension.
4. **Obtain separate live Windows consent** for baseline and prototype shell/launch/UAC/pin/destructive/recovery scenarios. Approval of this plan alone does not grant it. Hardware/driver/native smoke evidence pending is explicitly reported; concrete shell safety failures block further runs.

Draft validation checklist: inventory reviewed against rendered template/direct services, every FR-1–26 and NFR-1–10 referenced by AC-1–28, with optional FR-21/AC-21 distinguished from required completion evidence; native/external dependency failures mapped EC-1–12, contracts/models and explicit exclusions included. Generic validator checks syntax/section presence, not semantic completeness, native feasibility, atomic subcase coverage, or NFR traceability. Its HTTP-only API warning is intentionally inapplicable to local Rust contracts. Independent reviewer and user approval remain distinct gates.
