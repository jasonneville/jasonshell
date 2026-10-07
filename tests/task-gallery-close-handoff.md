# Gallery tile close acceptance handoff

Scope: RED-first then GREEN test ownership; no production-logic edits, native actions, ACL changes, or shell launch.

## GREEN continuation (test owner)

Production owners completed the route/UI; runner edited tests only, including native module-local `#[cfg(test)]` sections. Retry now waits for `aria-busy=false` rather than native `disabled=false` (native disabling intentionally avoided to preserve focus). Updated obsolete listbox/option source assertion to labelled group/sibling controls, and boundary-anchored legacy-close regex so `request_close_task_window_with_identity` is not a false forbidden match.

Focused results:

- `npm run test:component -- tests/components/task-gallery-close.test.ts`: **17/17 pass**; final retry assertions include busy/disabled accessibility state, retained focus, failure diagnostic and retry.
- `node --test tests/taskbarGalleryContract.test.mjs tests/taskbarGalleryUxState.test.mjs tests/taskbarGalleryCloseContract.test.mjs`: **28/28 pass**.
- `cargo test --manifest-path src-tauri/Cargo.toml request_only_close`: **4/4 pass**.
- `cargo test --manifest-path src-tauri/Cargo.toml gallery_close`: **3/3 pass**.
- `npx vite build --config tests/browser/task-gallery/vite.config.ts`: pass, 124 modules, isolated cache output.
- `npx tsc --noEmit --target ES2022 --module ESNext --moduleResolution Bundler --strict --skipLibCheck tests/browser/task-gallery/main.ts tests/browser/task-gallery/bridge.ts tests/browser/task-gallery/vite.config.ts src/vite-env.d.ts`: pass; fixture-specific type validation.
- `npm run cargo:check`: pass (existing compiler warnings).
- `npm run cargo:test`: completed successfully; **1016 passed, 0 failed, 26 ignored** across all test binaries (main binary: 870 passed / 7 ignored). Existing ignored/environment-dependent suites were not enabled.

New native behavior tests exercise actual production seams `request_gallery_close_with`, `snapshot_window_from_runtime`, `request_close_with`, and `revalidate_close_target_with`. Wrong callers cannot snapshot/dispatch; stale/missing sessions and nonmembers cannot dispatch. Dead targets cannot inspect/dispatch; PID/time/path mismatches and identity inspection errors cannot dispatch. Valid requests dispatch once, ordinary/access-denied errors propagate without retries, and gallery authorization survives success/failure/retry. Malformed input tests the production parser without dispatch. Shell case combines existing exact current-executable rejection predicate with injected admission failure; it is bounded seam evidence, not a concrete Tauri caller/Win32 journey.

Full `npm run validate` **failed at Node**, after check (0 errors/warnings), build, and **282 component tests passing**. Node: **1230 total, 1218 pass, 7 fail, 2 skipped, 3 todo**. Seven failures are outside gallery ownership: `contextMenuSpeechAcceptance.test.mjs` (1), `contextMenuVisualRegression.test.mjs` (2), `frontendUiPolicy.test.mjs` (1: SpeechIndicator gradient), `persistentTerminalPanel.test.mjs` (1: existing crop icon), `speechIndicator.test.mjs` (1: 35px versus expected 40px), `topBarMicControl.test.mjs` (1: transparent versus accent hover border). No unrelated production/test changes made to conceal these failures. Cargo full tests/check were invoked separately because validate short-circuited before native gates.

Artifacts under `C:\Users\jnev1\AppData\Local\Temp\opencode\`: `gallery-close-component-green.log`, `gallery-close-contract-green.log`, `gallery-close-native-actions.log`, `gallery-close-native-gallery.log`, `gallery-close-fixture-build.log`, `gallery-close-validate.log`, `gallery-close-cargo-full.log`, `gallery-close-cargo-check.log`.

Fixture ready/running: **http://127.0.0.1:4179/**. Setup: `npx vite --config tests/browser/task-gallery/vite.config.ts`. `tests/browser/task-gallery/README.md` documents controls and JS inspection API. `wmux` absent PATH; used approved OpenChamber visible browser panel, not invisible automation. Browser snapshot measured each close slot **24×24px** at simulated **32.4px height / 480px width** (three 160px tiles) and **24px height / 240px width / light colors** (three 80px tiles); Gamma title truncated in both. Idle X opacity 0 and selected/nonselected tab indices 0/-1 observed. This establishes baseline browser geometry only, not actual keyboard/hover/native focus delivery. OpenChamber click dispatch does not demonstrate trusted hardware focus; navigation behavior is covered by component tests and fixture remains ready for parent/manual visible interaction.

Captures: `.openchamber/screenshots/gallery-close-32px-baseline-2026-10-06T05-19-58-839.jpg`, `.openchamber/screenshots/gallery-close-24px-light-2026-10-06T05-20-26-074.jpg` (OpenChamber-managed paths). Native destructive E2E remains **pending / UNAUTHORIZED**, unchanged.

## Final test/fixture tightening

Independent reviewer reported CLEAN, no blockers; optional stale-rejection coverage strengthened to assert replacement **role=status empty** and **aria-busy=false**, not merely absent alert/native disabled. Component rerun: **17/17 pass**. Added native contract membership/stability assertion for `commands::CLOSE_TASK_GALLERY_WINDOW` in `commands::ALL`; `cargo test --manifest-path src-tauri/Cargo.toml new_command_contracts_are_unique_and_stable` passes (1 test). Log: `gallery-close-membership.log`.

Full native completion collected: **1016 pass, 0 fail, 26 ignored**. No full-suite rerun; previously described seven unrelated Node failures remain untouched.

Fixture now applies real production base-dark/base-light themes, so minimized/active/accent/muted colors are coherent. New controls: **Focus activation**, **Focus X**, **Snapshot 30 windows**. Focus controls call actual DOM `.focus()`; summary/full geometry report active element, focus-within, settled reveal opacity, pointer-events, bounds, strip overflow, title widths, color, clipping. Approved visible OpenChamber verified activation/X focus-within with opacity 1 and pointer-events auto; no hardware keyboard/hover/eval APIs used. First automatic measurements can precede compositor animation; Capture followed by Measure recorded settled state without CSS overrides.

**Concrete DOM stress limit exposed, not concealed:** 3 tiles at width144/height24 have zero-width titles but intact icons/X and three fully visible 24×24px X targets. With 30 windows at width144 or240/height24, production horizontal scrollbar reduces clientHeight to14px (tiles13.6px); 24px X boxes vertically clip, zero fully visible X targets. At width240/height32.4, clientHeight22px, X boxes clip by1px above/below tiles. ScrollWidth1440px; all titles zero-width. No scrolling/target/CSS workaround added. These narrow-width stress samples are not proof of native monitor geometry/user reachability, but refute a blanket claim of intact 24px targets during overflow. Parent/UI owner should assess the overflow limit.

Additional captures: `.openchamber/screenshots/gallery-close-focus-x-144-light-2026-10-06T05-29-05-865.jpg`, `.openchamber/screenshots/gallery-close-30-window-24px-144-limit-2026-10-06T05-30-02-114.jpg`, `.openchamber/screenshots/gallery-close-production-light-focus-activation-2026-10-06T05-30-41-712.jpg`. Fixture URL unchanged: http://127.0.0.1:4179/; reproducible settings `/?height=24&width=144&theme=light`, then Snapshot 30 windows. Native destructive E2E remains unauthorized.

## Acceptance interfaces for implementation owners

- `closeTaskGalleryWindow(hwnd: string, nonce: string): Promise<void>` invokes `IPC_COMMANDS.closeTaskGalleryWindow` / `close_task_gallery_window` with `{ args: { hwnd, nonce } }`.
- New native `close_task_gallery_window` admits the gallery caller, calls `snapshot_window(&args.nonce, &args.hwnd)`, and delegates to `request_close_task_window_with_identity(authorized.row.hwnd, authorized.identity)`.
- Request-only primitive reuses shell/exact immutable identity validation but not legacy close policy. One fallible `WM_CLOSE`, no termination, retry ladder, wait, elevation, or foreground activation. Success means request accepted, not target destroyed.
- Keep preview X and `close_task_gallery_previewed_window` unchanged.
- Tile activation and `button[type=button][aria-label="Close <window title>"]` are siblings. Selected tile alone supplies the activation/X tab pair. X has a permanently reserved 24px slot, revealed on hover/focus-within. Never nest buttons.
- Success emits existing `TASKBAR_REFRESH_WINDOWS_EVENT` with no argument payload; rows and preview remain until authoritative `task-gallery:open` reconciliation. Pending state is per session/target; settlement checks lifecycle/session validity.

Names above are proposed test interfaces, not shipped behavior. If owners deliberately select alternate equivalent names, coordinate tests; do not relax behavioral requirements.

## RED evidence

Commands executed without native shell startup:

```powershell
npm run test:component -- tests/components/task-gallery-close.test.ts
node --test tests/taskbarGalleryCloseContract.test.mjs
```

Observed baseline: component **17 total, 14 fail / 3 pass**. Most fail for missing accessible Close buttons; surviving-target reconciliation also exposes a stale context menu. Activation Enter/Space and context/minimize/Escape regressions pass. Node **5 total, 4 fail / 1 pass**: missing dedicated wrapper/command, native command, request-only primitive, and 24px reveal contract; unchanged preview policy passes.

Logs: `C:\Users\jnev1\AppData\Local\Temp\opencode\gallery-close-component-red.log` and `gallery-close-contract-red.log` in the same directory. These are diagnostic artifacts, not native dispatch evidence.

Independent review attempt blocked by subagent depth limit. Parent must own final review. No coverage percentage claimed from this RED suite.

Existing gallery regression baseline: `node --test tests/taskbarGalleryContract.test.mjs tests/taskbarGalleryUxState.test.mjs` passed **23/23**, using existing `dist-tests` output (not freshly recompiled). Log: `gallery-close-existing-baseline.log` in the same artifact directory. Svelte autofixer analysis was read-only and flags existing legacy `$:` as invalid in its default runes analysis; component tests successfully compile the legacy component. Log: `gallery-close-svelte-analysis.log`.

## Initial missing native evidence seam (supplied during implementation)

At RED baseline, native gallery commands required concrete Tauri windows/AppHandle, used singleton session state, and directly called Win32 actions. No injected request-only dispatcher existed. Adding direct command tests then would either fail compilation or exercise unsafe live HWND behavior; neither was done. Production owners subsequently supplied seams tested in the GREEN continuation above.

Native owner should add a narrowly scoped pure/injected admission + dispatch seam and module-local tests in `task_gallery.rs` / `task_windows/actions.rs`:

1. Wrong caller, stale/missing nonce, absent membership, malformed/dead HWND, shell/current-exe target, PID/creation-time/image-path mismatch: error, dispatcher count zero.
2. Valid captured identity: one `WM_CLOSE`, no activation/force/elevation calls.
3. Access denied/ordinary dispatch failure: truthful error, no retries or privilege changes.
4. Ignored close/unsaved dialog surviving HWND: authorization remains; revalidated retry and activation still usable until authoritative snapshot removal.
5. Successful or failed close must not remove membership or reset session; stale session settlement must not affect replacement session.

Source regex guards only check wiring and forbidden fallback names; they cannot establish the above runtime security outcomes.

## Visual browser evidence route

Existing isolated precedent: `tests/browser/stack-mouse/vite.config.ts` aliases Tauri core/event/window to a test bridge and serves production components with global CSS, avoiding shell startup. It is not a gallery fixture. Designer can use the same pattern in a gallery-only test fixture, mount `TaskGallerySurface`, publish three titled windows, control deferred close completion and same-nonce removal, and keep a visible IPC ledger. Launch fixture-only Vite (never `tauri dev`). Use visible `wmux browser open <fixture-url>` / `snapshot` / `screenshot` / `eval` for hover/focus reveal, 24px geometry, title truncation, Tab order, and no layout shift. No visible browser evidence was collected in this RED task; jsdom cannot prove CSS geometry or native focus behavior.

## Required destructive cross-native journey — pending / UNAUTHORIZED

Actual live Tauri/Windows smoke needs explicit human consent, not supplied. Do not launch shell or imply mock DOM tests certify Win32.

After consent: use disposable fixture-owned same-process windows (three titles) with logged WM_CLOSE receipt, including one ignoring close and one showing a save prompt. Open real gallery, exercise click/Enter/Space X, assert exactly one target WM_CLOSE per request, no activation/force/UAC; ignored-close/prompt windows remain authorized and usable. Verify successful target destruction reaches an authoritative native snapshot before its row disappears; surviving gallery focus/preview remain, stale menu disappears, no foreground steal. Repeat denied dispatch, duplicate pending requests, replacement/hide/unmount races, and unchanged preview X semantics. Capture message counts, native snapshot/session ledger, process survival, foreground HWND, and screenshots. Clean up only fixture-owned windows and restore shell state through existing rollback paths.
