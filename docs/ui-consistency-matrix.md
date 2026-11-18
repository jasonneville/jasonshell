# UI consistency audit matrix

**Status:** source inventory pinned to JasonShell `0194489`; close-control
pilot verified. The broader rendered comparison and native acceptance are
still open.
**Scope:** source audit with renderer follow-up below; no app launch and no
native observation. Source routing is `src/lib/shellSurface.ts:1-141`, loading
is `src/lib/surfaceLoader.ts:13-42`, and `src/App.svelte:21-67` mounts the
selected component. The 20 labels below are the complete loader inventory.

## Evidence and disposition rules

- **Source contract** means the branch and its intent are explicit in the
  component/native adapter. **Existing contract coverage** means a nearby
  test/fixture asserts that contract; it is not live Windows evidence.
- **Renderer evidence** can establish DOM dispatch, focus, sizing, and mocked
  IPC. It cannot establish HWND foreground, WebView2 composition, DWM backdrop,
  native blur timing, or OS-owned menu behavior.
- **Windows-owned** is intentional native ownership (for example Explorer
  menus or an external task HWND). **Blocked** means the claim needs a native
  repro or consent-gated environment; a source mismatch alone is not a defect.
- Unless a row says otherwise, launch is the owning launcher/native show path;
  dismissal is the component hide/close path; retained state is mounted
  renderer state where documented. Focus destinations are expected contracts,
  not observations in this lane.
- Severity applies to findings, not to surfaces: P1 is a reachable keyboard
  or comparable-control mismatch; P2 is a local comparison without a proven
  interaction failure. Intentional and Windows-owned rows are not defects.
  The designer owns verified renderer fixes, the E2E runner owns browser
  evidence, and the maintainer owns consent-gated Windows evidence.

## Complete 20-label matrix

| Loader label / source owner | Peer and nested interaction branches | Launch → dismiss / focus contract | State and visual evidence | Disposition |
|---|---|---|---|---|
| `top-bar` — `TopBar.svelte`; loader `surfaceLoader.ts:16` | Peer: `bottom-bar`. Launcher buttons; switching panels; pointerdown/click arbitration; launcher context actions. | Native/top-bar shell launch → panel hide or native blur. Bar remains shell owner; panel target receives focus when applicable. | Shared launcher state, tooltip/shortcut labels, AppBar geometry. Existing hotkey/launcher contracts; native foreground/blur blocked. | Intentional family peer; compare chrome only. Native-owned/blocked evidence. |
| `bottom-bar` — `BottomBar.svelte`; loader `:17` | Peer: top bar. Quick Launch, task tile, grouped gallery, task/launcher/group context menus, reorder and hover preview. | Bottom-bar launcher → close-only preview/gallery or external task action. Hover does not steal focus; selected gallery controls own keyboard focus. | `BottomBar.svelte:382-412,653-744,822-899,1123-1135`; browser taskbar fixtures cover geometry/routing, not screen-edge delivery. | Intentional shell rail; external-window actions and DWM placement blocked. |
| `task-gallery` — `TaskGallerySurface.svelte`; loader `:18` | Peer: task-preview. Card hover/selection, arrows/Home/End, card Close, context menu, close-all pending/failed states. | Group tile opens gallery → explicit gallery dismissal or focus loss. Selected card's Activate and Close controls are tab stops; card Close requests closure of the external task, not the gallery, and does not promise taskbar focus. | `TaskGallerySurface.svelte:78-96,605`; `master_spec.md:114-126`; component/browser mocked-IPC coverage exists. | Intentional grouped-task contract; native HWND/PID action and mixed-DPI placement blocked. |
| `task-preview` — `TaskPreviewSurface.svelte`; loader `:19` | Peer: task-gallery. Hover-lifetime preview, activation, separate Close button. | Bottom-bar hover opens → close button invokes `closePreviewedTaskWindow` (`TaskPreviewSurface.svelte:72-85,177-180`), closing the **external HWND**, not this preview surface. | Preview CSS and browser inset fixture establish renderer geometry/focus only; native activation/close outcome unobserved. | **Intentional**, not a preview-dismiss label defect; native activation/foreground blocked. |
| `search-panel` — `SearchPanelSurface.svelte`; loader `:20` | Anchored vs centered; query field, result activation, centered scope `MeltSelect`, outside/Tab, Escape. | Search launcher/hotkey → hide (`:328-336`) or result activation. Query owns focus; centered Select intentionally `restoreFocusOnClose={false}` (`:428-454`) so Tab/outside does not steal destination focus; selection/Escape uses Select restoration. | Existing search tests and scope contracts; renderer can prove focus/dispatch. Cross-process hotkey/foreground remains blocked (README:221-223). | Intentional focus exception; native shortcut evidence blocked. |
| `stack-popup` — `StackPopupSurface.svelte`; loader `:21` | Folder rows, breadcrumbs, row/background/open-with menus, Git drawer, editor, confirmation, integrated terminal. | Launcher/native request → close request enters dirty-editor guard; clean/confirmed hide and publish closed. Back/Escape consumes innermost layer first. | `StackPopupSurface.svelte:315-320,357,684`; `master_spec.md:193-195`; source-contract tests cover guards. | Intentional guarded persistence/focus contract; native blur and Explorer menu branches blocked. |
| `process-manager` — `ProcessManagerSurface.svelte`; loader `:22` | Processes/System Info tabs, filter/sort, kill confirmation, explicit PID show/focus, Escape armed-kill then panel. | Launcher/hotkey toggles; explicit PID is show/focus-only. Escape cancels armed kill before close (`:434-439`); close button/hide retires native context. | `:306-320,395,413-469`; System Info cache contract in `master_spec.md:80-92`; tests cover stale generations. | Intentional show-vs-toggle distinction; live HWND/focus and sensor/OS data blocked. |
| `quick-launch-panel` — `QuickLaunchPanelSurface.svelte`; loader `:23` | Launcher rows, keyboard navigation, ContextMenu/Run as administrator, native close event. | Bottom-bar Quick Launch → Escape/hide (`:87-90`) or row launch; row/context action transfers focus to destination, not stale panel. | `:40,87-92,138-158,188-217`; component/source contracts cover navigation/menu; native Explorer launch blocked. | Intentional native launcher; Windows association/admin result blocked. |
| `control-plane` — `ControlPlaneSurface.svelte`; loader `:24` | Section tabs, `MeltSelect` controls, refresh/status/error states. | Native show → existing close/hide route; select/menu owns focus before panel. | `ControlPlaneSurface.svelte:6,132`; only source routing/select ownership was inspected; no native run. | Preserve domain-specific panel; renderer fixture and native focus evidence blocked. |
| `settings-panel` — `SettingsPanelSurface.svelte`; loader `:25` | Appearance/theme editor, tabs/selects, hotkey capture, Save/Cancel/Reset, discard/preview draft. | Launcher/hotkey toggles mounted Settings; hide retains draft, Cancel/dispose restores saved theme; Escape stops capture first (`:473,576-580`), then close. | `:566-592`; `master_spec.md:94-102,197-202`; settings/theme tests cover draft and transaction state. | Intentional retained-draft contract; native registration/focus and translucent contrast blocked. |
| `tray-panel` — `TrayPanelSurface.svelte`; loader `:26` | Relay icons, keyboard/context menu, close button, relay-hold/no-activation case. | Tray launcher → Escape/hide (`:63-77`) or icon relay; relay must not activate target accidentally. | `:117-148`; local header close is a P1 visual peer; native tray ownership, relay and foreground are OS-owned. | Compare local close chrome; native relay blocked and not a shell CSS defect. |
| `terminal-panel` — `TerminalPanelSurface.svelte`; loader `:27` | Persistent tabs/splits, search, quick select, context menu, confirmation, close search vs close panel. | Terminal launcher → panel hide; inner search Escape closes search and refocuses terminal (`:997-1020`); quick-select Escape cancels layer (`:1113-1120`). | Stack search peer says Close while dedicated search says Esc (plan §Source findings); status tests cover renderer. ConPTY/native focus blocked. | Pair terminal-search chrome with Stack only after renderer comparison; keep local Escape semantics. |
| `command-panel` — `CommandPanelSurface.svelte`; loader `:28` | Reorder, row/context menu, delete confirmation, transcript URL menu, pending input, history, editor. | Quick Commands launcher/hotkey → close button (`:609,925-926`); Escape first cancels confirmation/input/context/reorder as applicable. Focus restoration is explicit (`:161-173,327-358,657-661,895`). | Existing component/source tests cover nested focus and mutations. Idle window Escape currently only cancels reorder/context (`:657-661`); no native repro. | **Blocked candidate:** do not claim idle Escape defect/change. Existing nested contract passes source evidence; idle native/DOM behavior requires repro. |
| `audio-panel` — `AudioPanelSurface.svelte`; loader `:29` | Volume sliders, device selection, refresh, close text button. | Audio launcher → `closeAudioPanel`/hide; Escape closes (`:184-186`), then focus returns by native owner contract. | `:94-98,225-232`; close is text `Close` beside `Refresh`, unlike Calendar icon. No rendered fixture/native observation. | P1 visual comparison peer; likely scoped visual fix only after measured renderer evidence; native focus blocked. |
| `speech-history-panel` — `SpeechHistoryPanelSurface.svelte`; loader `:30` | Transcript list/empty state, close button, Escape, focus of dialog/panel. | Speech history launcher → `closePanel`/hide; Escape and click close (`:73-87,126-134`). | Literal `x` (`:134`) differs from Calendar/Settings Material affordance; source contract and empty/list branches known, renderer/native focus unobserved. | P1 visual comparison peer; preserve hide callback and focus contract; native blocked. |
| `calendar-panel` — `CalendarPanelSurface.svelte`; loader `:31` | Month/navigation, local-time details, header close, Escape. | Calendar launcher → hide; Escape closes (`:53-56`); header icon close is `MeltActionButton` with tooltip (`:82`). | Anchored-header reference: icon, name, tooltip and shared action button. No live composited contrast evidence. | P1 reference peer; preserve callback. Renderer measurement pending; native backdrop blocked. |
| `speech-indicator` — `SpeechIndicatorSurface.svelte`; loader `:32` | Noninteractive recording meter; nonce/status hydration, hidden/terminal/failure states. | Speech start shows nonce-scoped indicator; terminal/stale/lost state hides (`:23-30,68-107`). No user dismissal or focus target. | `master_spec.md:32-33`; transparent/nonactivating and reduced-motion contracts documented; live WebGL/native composition blocked. | Intentional noninteractive exception; do not add close/focus for parity. |
| `context-menu-overlay` — `ContextMenuOverlaySurface.svelte`; loader `:33` | Pin/task-window/task-group menu variants, disabled rows, arrows/Home/End, backdrop click, window blur, Escape. | Context action opens overlay → selection dispatch hides without origin restoration; Escape/backdrop/blur `dismiss` restores native origin focus (`:55-75,114-147`). | Shared menu primitives and viewport positioning (`:44,118-125`); component acceptance tests cover focus/escape. Native overlay HWND/focus blocked. | Intentional overlay focus owner; distinguish from inline Stack menus; native placement blocked. |
| `snip-overlay` — `SnipOverlaySurface.svelte`; loader `:14` | Pointer selection, drag/capture, pointer cancel/lost capture, native Escape. | Native snip start → Escape/cancel returns to prior owner; overlay is non-persistent and must not focus on cancellation. | `SnipOverlaySurface.svelte:102`; snipping tests cover source/product contracts, but real native Escape journey is opt-in and not run. | Intentional native capture flow; native Escape/monitor evidence blocked. |
| `snip-preview` — `SnipPreviewSurface.svelte`; loader `:15` | Copy, Save picker, explicit Dismiss, pending/disabled/error state; header Close duplicates Dismiss. | Capture publication → Copy/Save/Dismiss; both Close and Dismiss call `dismissSnip` (`:34-54,79,88`), preserving pending guard. | Existing component contracts cover one dismiss dispatch and disabled state; native picker/DWM/nonactivation blocked. | Intentional redundant explicit dismissal pending comparison; do not remove without evidence. |

## Nested branch ledger and evidence boundary

The rows above include the reachable nested controls required by FR-1. The
following compact ledger makes the distinct interaction branches explicit:

| Branch | Owner / peer | Launch, dismiss, focus and retained state | Evidence / disposition |
|---|---|---|---|
| Quick Commands idle Escape | `CommandPanelSurface.svelte:657-661,923`; peer Audio/Calendar/Tray panels | Window Escape currently cancels reorder/context only; close button uses `hideCommandPanel`. Pending input, delete confirmation and reorder each consume Escape first and restore their trigger where connected. | Existing source/component contract covers nested cancellation. **No actual native/DOM idle repro means no change claim**; blocked candidate. |
| Quick Commands context menu | `CommandPanelSurface.svelte:632-645,923,971+`; peer ContextMenuOverlay | Outside click/Escape dismisses local menu; selected action remains in command owner. | Renderer/source evidence; no native claim. |
| Quick Commands delete confirmation | `:327-358` | Escape cancels confirmation without mutation; Tab cycles enabled buttons; close/confirm restores trigger or panel focus. | Existing source contract; renderer focus fixture owner is tests lane. |
| Quick Commands pending input | `:895-923` | Escape clears draft, Enter submits; panel remains mounted. | Existing source contract; renderer test owner. |
| Terminal search vs Stack terminal search | `TerminalPanelSurface.svelte:1018-1020,1120`; `StackTerminalPane.svelte:609,681`; `TerminalPanelSurface.svelte:1902,2609` | Both clear search/refocus terminal; appearance/name differs (Close vs Esc). | Renderer comparison is actionable; native focus outcome blocked. |
| Task Preview Close | `TaskPreviewSurface.svelte:72-85,177-180`; peer Task Gallery | Close sends external task-window close request; it does not dismiss the preview itself. | **Intentional contract**, not visual drift. External HWND activation/close and save-prompt behavior require native evidence. |
| Snip Preview Close/Dismiss | `SnipPreviewSurface.svelte:34-54,79,88` | Two explicit controls share guarded dismiss; pending disables both; preview remains nonactivating. | Existing component contract; native preview/picker blocked. |
| Context overlay vs inline menus | `ContextMenuOverlaySurface.svelte:55-75,114-147`; peers Quick Launch/Stack/Command menus | Overlay owns DOM focus and restores origin only on dismissal, not selected action. Inline menus own their panel focus. | Existing acceptance/source tests; Windows HWND placement/focus blocked. |
| Settings hotkey capture | `SettingsPanelSurface.svelte:473,576-580`; `master_spec.md:197-205` | Escape stops capture before closing; Save/Cancel/Reset are draft transaction branches; hide preserves draft. | Existing settings contracts; live native registration/focus blocked. |
| Stack dirty exit | `StackPopupSurface.svelte:315-320`; editor/confirm peers | Close/back/Escape routes through dirty guard; cancel preserves draft, confirmed exit hides and publishes closed. | Existing source tests; native blur/foreground blocked. |
| Process Manager armed kill | `ProcessManagerSurface.svelte:318-320,434-439` | First Escape cancels armed kill, later Escape closes; explicit PID show/focus never toggles hidden. | Existing source contract; live process/HWND focus blocked. |
| Search scope select | `SearchPanelSurface.svelte:428-454`; `MeltSelect.svelte` | Selection/Escape can restore trigger; Tab/outside intentionally retain destination because centered search opts out of generic restoration. | Documented current contract (`master_spec.md:74-78`); renderer focus evidence tests lane, native hotkey blocked. |
| Tray relay hold | `TrayPanelSurface.svelte:63-77,121-148` | Escape closes panel; icon relay/context action must not activate external tray target as a side effect. | Windows-owned branch; no live tray relay evidence, blocked. |

## Cross-cutting conclusions

1. Calendar is the anchored-header reference. Speech History's literal `x`,
Audio's text `Close`, and Tray's independently styled button are visual-drift
comparisons, not permission to change lifecycle or native ownership.
2. Task Preview is deliberately different from panel dismissal: its Close
   label targets an external HWND. Treating it as “close preview” would be a
   contract regression. Native activation and save-prompt behavior remain
   blocked without a Windows repro.
3. Quick Commands has existing nested Escape contracts. Its idle window-level
   Escape path does not prove that a native idle Escape should hide the panel;
   no change is justified until the real native/DOM path is reproduced. This is
   a blocked candidate, not a confirmed defect.
4. Nonactivating Speech Indicator/Snip Preview and native Explorer/tray/menu
   surfaces are intentional exceptions. Focus parity must not be “fixed” by
   activating them or by restyling Windows-owned UI.
5. No app launch, HWND observation, mixed-DPI run, or native smoke occurred.
   Renderer screenshots below do not establish native acceptance.

## Renderer follow-up

See [close parity fixture](../tests/browser/close-parity/README.md) for
reproduction. Chromium mounted production Svelte with mocked Tauri IPC.
Theme checks below compare close chrome and focus paint. They do not certify
text/control contrast thresholds, all menu states, or native composition.
Final review reruns are in `test-results/close-parity-anchored-review-final/`
and `test-results/close-parity-terminal-review-final/`.

The remaining visual comparison covers menu rows/submenus and shortcut hints;
typography, borders, radius, shadows, and spacing among comparable surfaces;
and selected, pressed, loading, empty, error, and disabled states at typical
and reduced widths. An inventory row or an existing source test does not
count as a rendered pass. Unavailable branches need an explicit reason.

Broader fixture results are documented in the
[renderer audit README](../tests/browser/ui-audit/README.md). The inventory
mounted all 20 loader labels in three configurations. Populated overlay and
Search fixtures reduced missing IPC mocks from 36 to 13 of 60 snapshots;
blank and fallback frames do not count. The Escape probe confirmed only that
idle renderer Escape sends no hide IPC; it does not establish live Windows
behavior.

The design reference is not a single layout for every panel. Anchored headers
share close chrome but keep their existing content insets. Shell context menus
share `ContextMenu` paint, item density, focus and separator rules; inline
submenus and the native overlay retain their own positioning and focus owners.
`src/app.css` supplies semantic typography, spacing, border, radius, shadow,
and focus tokens. Selected, destructive, thumbnail, and hardware/status colors
are not interchangeable with neutral chrome. The Settings equivalence check
below verifies theme token mapping, not every panel's contrast or clipping.

Rust preflight follow-up: building the real, ignored `dist-p03-experiment/`
and `dist-text-probe/` Vite assets removed the example compilation errors.
With the workspace wrapper disabled for the command only, `cargo:test` then
ran the Rust suite: 1,022 passed, 12 ignored, two Quick Commands nonce tests
failed under parallel execution. The full serial Cargo rerun passed with no
failures (`cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1`).
These were test runs, not a live shell launch.

| Batch | Before → after | Evidence and limits |
|---|---|---|
| Calendar, Speech History, Audio, Tray | Calendar's Material close is now shared. Speech's text glyph, Audio's text Close, and Tray's local styling are replaced; Audio Refresh remains separate. | 24 dark/light/translucent states at 440/280 CSS px passed. Targets: 27.1875 × 27.1875 CSS px, centered vertically and flush with each header's content edge. Focus and tooltips passed; 72 Space/click/Escape checks each sent one original hide command. Reports: `test-results/close-parity-baseline/` and `test-results/close-parity-anchored-confirmed/`. |
| Stack and dedicated terminal search | `Close` / `Esc` text controls → shared Material close with name “Close terminal search.” | Six states passed (15 baseline assertion failures). Both targets are 24 × 24 CSS px; click clears search and refocuses xterm without session IPC. Escape closes from xterm; from the search input, it leaves search open and preserves focus. Reports: `test-results/close-parity-terminal-baseline/` and `test-results/close-parity-terminal-green/`. |
| Dedicated terminal toolbar | At 360 CSS px, the focused actions menu button clipped at the viewport → toolbar wraps at narrow widths. Escape previously left an open toolbar dropdown in place → it closes only that dropdown and restores its trigger. | Six controls fit and paint focus at 860, 480, 360 and 320 CSS px. At 360 CSS px the button's right edge moved from 377.75 to 351.20 px. All four Escape paths (Actions and Recent, from item and trigger) restored focus without hide or session IPC. Reports: `test-results/ui-audit-terminal-toolbar/` and `test-results/ui-audit-terminal-menu-escape-green/`. |
| Quick Commands tabs | Previous runs took keyboard focus while clipped → pane header and tab group wrap as needed. | Three browser states passed: both labels fit, the keyboard focus ring is visible, and Space activates Previous runs. At 460 CSS px dark, its right edge moved from 461.95 to 423.69 px within a 438.80 px clipping boundary. Reports: `test-results/ui-audit-command-layout/` and `test-results/ui-audit-command-layout-green/`. |
| Context menu overlay | At 320 × 145 CSS px, End focused Close window below the scrollport → keyboard focus scrolls the menu without moving the viewport. | Three browser states passed. At 320 × 145, End scrolls 42 px and reveals the row at y=101.56–131.95, inside the y=137 edge. Home returns to zero. Focus paint, Tab, arrows, disabled-row skipping, and one Escape dismissal request passed. Native focus restoration remains unverified. Component tests: 41/41. Reports: `test-results/ui-audit-menu/` and `test-results/ui-audit-menu-green/`. |
| Quick Launch nested menu | Escape on the focused menu action hid the panel and left the menu mounted → first Escape dismisses only the menu and restores the valid row. | Three populated browser states passed. Held Escape does not dispatch hide; the next deliberate press dispatches the original hide once. Detached origins are ignored; Tab and in-panel clicking preserve their existing behavior. Node: 11/11. Reports: `test-results/ui-audit-quick-launch/` and `test-results/ui-audit-quick-launch-green/`. Native click-away, launch and elevation were not exercised. |
| Quick Launch menu lifecycle | Old menu survived accepted close/reopen and consumed the next Escape → open and accepted close clear menu state; stale close still preserves the current menu. | Three browser states passed: fresh Escape sends the original hide IPC once. Reports: `test-results/ui-audit-quick-launch-lifecycle-red/` and `test-results/ui-audit-quick-launch-lifecycle-green/`. |
| Quick Commands nested menu | Escape dropped focus to the document body → returns focus to the live command row without scrolling. | Mocked browser Escape keeps panel open and preserves confirmation and pending-input behavior. Connected origin exercised; detached origin not exercised in this fixture. Reports: `test-results/ui-audit-command-nested/` and `test-results/ui-audit-command-nested-green/`. |
| Stack Open With submenu | No change; nested menu owns loading, populated and error content. | At 440 × 300 CSS px, both menus fit and Escape returned focus to the parent item after both successful and failed loading. Report: `test-results/ui-audit-stack-menu/`. No native application launch. |
| Settings theme editor | No lifecycle change. | Existing isolated browser journey passed 16 checks, including mounted hide/show draft retention, Cancel, Save, reload and recovery. Computed equivalence: 3,876 checks across 34 presets and 38 tokens. Browser Hide/Show is not native window evidence. |
| Task Gallery | No product change. | Existing runner had 32 failures (30 title checks, narrow ArrowDown, transport error). The title check measured the wrapper rather than the ticker: at 396 CSS px the latter was 127/170 px with hidden overflow and ellipsis. Separate trusted Chromium verified two-column ArrowDown and both Space and Enter reaching one pending external close. Enter sent one mock close for the selected external window, no activation request, and preserved busy-button focus. Reports: `test-results/task-gallery-diagnosis/` and `test-results/gallery-enter-final/`. |
| Search scope | No product change. | Trusted Chromium at 320 CSS px confirmed Escape and selection restore the scope trigger; Tab and a hit-tested click-away dismiss the popup and leave focus on the query. Report: `test-results/search-scope-cdp-final/`. Native foreground ownership remains unverified. |
| Launcher and guarded exits | No lifecycle changes. | Existing mocked panel/Quick Commands fixtures passed 17 + 4 checks including launcher routing, mocked blur arbitration, and explicit PID show. Targeted component suites passed 110 tests covering Stack close guard, task close ownership, snip, Commands and Settings hotkeys; targeted Node suites passed 47. These are renderer/contract checks, not native reproduction. |
| Bottom Bar tile chrome | No change. | Existing isolated CSS fixture passed 233 checks: dark/light at three heights, gutters and seams, tile and icon fit, active/disabled/minimized/drag cues, and focused and reduced-motion states. Fixture uses production CSS with static markup; it does not prove runtime drag/hover geometry. |
| Task Preview chrome | No change. | Existing fixture mounted `TaskPreviewSurface` with mocked IPC and passed inset acceptance. This does not exercise a real DWM frame or external-window close. |
| Populated Audio, Tray, and Speech History | Audio's application volume slider clipped at 280 CSS px → slider fits with focus-ring clearance. | Nine browser states passed. At 280 CSS px, the focused Audio slider ends at x=263.09 and its ring at x=265.09, inside the x=268.91 clip edge; no horizontal overflow. Volume changed 42 → 43 and USB output selection used the original mocked IPC. Reports: `test-results/ui-audit-active-panels-confirmed/` and `test-results/ui-audit-active-panels-green/`. See the independent pass below for later Tray changes. |
| Process Manager, Control Plane and snip contracts | No product change. | Focused Process Manager and snip suites passed 31 tests; Audio controls/refresh and Tray retention passed 15 Node checks. Nine Control Plane/README contract checks passed, covering section filtering, keyboard actions and safe routing. These are not real hardware or native focus checks. |
| Independent populated work-panel pass | Audio and Process Manager passed. Tray now keeps keyed icons mounted through keyboard refresh and insets the focus ring from the scrollport. | Six Audio/Process Manager passes across dark, light and narrow translucent: device/slider focus and single mock requests; process filtering, keyboard tab selection and retained filter. Tray went from three failures to three passes: successful mock relay retained the icon's focus, the full ring fit at 210 CSS px, failure kept focus and reported an error, and delayed success did not steal focus after Tab. Duplicate activation sent no extra IPC. Reports: `test-results/ui-audit-panels-red/` and `test-results/ui-audit-panels-tray-green/`. Close/reopen during relay is source-only; Explorer and native foreground were not exercised. |
| Tray refresh races | Older reads could replace a newly opened grid or show a stale error → only the latest read for the current opening may publish results or update loading. | Four mocked browser cases passed: pointer refresh across hide/reopen and overlapping opens, each with stale success and rejection. All preserved the newer grid with no stale alert or loading; three keyboard focus cases also passed again. Reports: `test-results/ui-audit-panels-tray-races-{red,green}/` and `test-results/ui-audit-panels-tray-focus-after-read-fix/`. No native relay was exercised. |
| Snip preview geometry | No product change. The geometry fixture now checks bounds separately from decorative glyph rendering. | All four [browser fixture](../tests/browser/snip-preview-geometry.md) cases passed for normal and wrapped-error states at 320 × 264 and 280 × 220 CSS px. The narrower Dismiss button remained 32 × 32; all four controls had their expected accessible names. No native capture or foreground observation. |

Independent review flagged possible clipping in the open Terminal toolbar
actions dropdown at 320 × 200 CSS px. The scoped browser repro did not find
clipping: Tab reached the final eligible action after 156 px of local scroll,
with its bottom at y=193.03, within the y=198.50 clipping edge. The dropdown
needed no additional geometry change. After dismissal, all four routes return
to pre-open scroll offsets with the trigger fully visible. The deepest Actions
route temporarily scrolled the outer panel 0 → 34 → 0; xterm stayed at zero.
Reports: `test-results/ui-audit-terminal-menu-red/` and
`test-results/ui-audit-terminal-menu-escape-preopen/`.

Stack Git drawer and editor exits keep their distinct targets/guards; no
measured peer regression justified changing them. Snip Preview retains both
explicit dismiss controls and their pending guard. Unsupported-label/load
failure styling in `App.svelte` remains a separate candidate. Idle Quick
Commands Escape is unchanged without a native repro. WebView2/DWM backdrop
contrast, actual foreground ownership, Windows DPI and mixed-monitor behavior
remain unverified. Browser device-scale emulation does not close that gap.

The broad inventory did not exercise every populated/error variant. Keep those
cells unverified rather than treating a clean source trace or default frame as a
pass.

## Validation baseline

The broader `npm run validate` run passed Svelte check, build, `cargo:test`, and
`cargo:check` after building ignored test assets and setting process-local
`RUSTC_WORKSPACE_WRAPPER=''` and `RUST_TEST_THREADS=1`. It exited nonzero for
the same three pinned frontend failures: the Stack path-navigation source
assertion (`frontendUiPolicy.test.mjs:83`) and the two Stack Git remote-link
assertions (`stack-git-push.test.ts:47`, 630/632 passed). Search panel component
coverage passed 15/15. After the final Tray race fix and wiring-test updates,
Svelte check and build passed, and a full Node rerun passed 1,433 tests with
only the pinned Stack assertion failing. No Cargo, test timeout, or permission
configuration changes were made.

## Native ownership trace (source only)

- `src-tauri/src/windows_key_hook.rs:873-875` sends Quick Commands toggle to
  TopBar, not a second toggle to the panel. `src-tauri/src/main.rs:545-578`
  owns delayed focus-loss dismissal; `src-tauri/src/command_panel.rs:137-148`
  invalidates that pending dismissal and publishes closed on explicit hide.
- `main.rs:390-432` routes Stack blur to close-only and Search blur to
  closed/hide. At `:442-473`, Settings and Process Manager arbitrate blur
  after 150 ms with revision, visibility, and focus checks; explicit PID
  launch remains show/focus, not toggle.
- `main.rs:496-535` owns Audio/Calendar/Speech blur and Tray's Explorer
  relay hold. At `:537-542`, terminal native close prevents destroy and hides
  the window; the terminal search control does not use that path.

### Remaining native acceptance

Use the [Windows smoke checklist](smoke-test-windows.md) only after consent
for shell startup, AppBar/work-area changes, Explorer taskbar changes, and
global hooks. Record the build, OS/WebView2 versions, monitor layout, and scale.
Compare 100% with one non-default scale; record mixed-DPI or monitor-edge
checks separately if available. Capture prelaunch and post-shutdown desktop
state. Do not use real relay, capture, command execution, process termination,
or external-window close as substitutes for the safe checks below.

| Route | What to record |
|---|---|
| Top/Bottom Bar launchers and hotkeys | One visibility change and committed expanded state per deliberate activation; real click-away, re-click, switch and rapid blur/reopen; explicit PID show remains visible on repeat. |
| Quick Commands idle Escape | Foreground owner and active DOM target before Escape, then window visibility and expanded state. Run idle separately from nested menu, confirmation, reorder and pending input; do not infer closure from DOM alone. |
| Search scope and guarded panels | Tab/click-away destination, nested Escape origin if still valid, retained unsaved Settings theme, Stack close-only while hidden and dirty-editor preservation using a disposable fixture. |
| Visual and nonactivating surfaces | Keyboard focus paint, menu fit at screen edges, composed translucent legibility, and foreground ownership while approved nonactivating previews are visible. Mark unavailable routes blocked. |

## Link and inventory check

- Verified repository links: `src/lib/surfaceLoader.ts`, `src/lib/shellSurface.ts`,
  `src/App.svelte`, `README.md`, `master_spec.md`, `docs/README.md`, and the
  close parity fixture README exist.
- Loader labels counted: **20** (`snip-overlay`, `snip-preview`, `top-bar`,
  `bottom-bar`, `task-gallery`, `task-preview`, `search-panel`, `stack-popup`,
  `process-manager`, `quick-launch-panel`, `control-plane`, `settings-panel`,
  `tray-panel`, `terminal-panel`, `command-panel`, `audio-panel`,
  `speech-history-panel`, `calendar-panel`, `speech-indicator`,
  `context-menu-overlay`).
- Dispositions used: renderer-confirmed fix, intentional, Windows-owned, and
  blocked; no native claim is promoted from source mismatch alone.
