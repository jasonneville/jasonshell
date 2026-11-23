# JasonShell takeover audit — summary and triage (2026-11-22)

Author: Claude (Opus 5.5), taking over from the OpenAI-model development era.
Status: **audit only**. No production code changed. Every finding is a candidate, not a commitment.

## How to use this folder

| File | Lane | Findings |
|---|---|---|
| `00-summary.md` | This file: themes, triage tiers, sequencing | — |
| `01-visual-ux.md` | Design system, theming, accessibility, per-surface UX, motion | 62 + 10 big bets (`VIS-*`) |
| `02-frontend-arch.md` | Svelte 5 architecture, bundle, runtime perf, persistence | 54 + 5 big bets (`FE-*`) |
| `03-ipc.md` | Rust ⇄ JS boundary: commands, events, ACL, payloads, types | 35 + 6 big bets (`IPC-*`) |
| `04-rust-perf.md` | Rust threading, locks, hot paths, git/search/terminal/speech, Cargo | 60 + 8 big bets (`RS-*`) |
| `05-native-windows.md` | Win32 correctness, AppBar, Explorer, unsafe, security, shell completeness | 57 + 6 big bets (`WIN-*`, `SEC-*`) |
| `06-product.md` | Feature gaps vs industry, half-built features, creative ideas, roadmap | 62 + 14 big bets (`PRD-*`, `BB-*`) |
| `07-tests-build.md` | Tests, CI, build/release, repo hygiene, DX | 46 + 6 big bets (`TST/CI/BLD/HYG/DX-*`) |

Total: **~376 findings + 55 big bets**. Each finding carries impact, effort (S/M/L), `file:line` evidence, problem, and recommendation. Lanes overlap deliberately at the boundaries; cross-lane duplicates are listed under "Cross-cutting themes" so you can pick one ID and close the others.

### Method and limits

- Seven independent read-only auditors, each scoped to one lane, read source directly. Only `master_spec.md` and `docs/current-state-brief.md` were used as prose context (plus `README.md` for the product lane); the rest of `docs/` and the root plan files were intentionally ignored as stale.
- Evidence is **static**. The Tauri app was never launched (live shell smoke requires consent). Latency, memory, and payload figures are estimates from code, not measurements.
- Commands actually run: `npm run test:node` (1,449 pass / 0 fail / 4 skip / 3 todo, 14 s), `svelte-check` (0 errors), a scratch `vite build` for bundle sizes, `cargo fmt --check`, a PE-header read of the release exe, `git`/`gh` inventory, and a WCAG contrast script over all 34 generated palettes.
- The leader spot-verified the highest-severity claims in source: SEC-01, BLD-01/WIN-05, CI-01, and the WIN-16 terminate fallback all match what the code does.

## The one-paragraph picture

JasonShell has a lot of carefully reasoned behavior: identity-checked moves and kills, nonce/generation guards, a disciplined low-level keyboard hook, handle-based renames, and a truthful spec. That care is uneven, though. It went into local correctness contracts, while the **platform foundations** were left thin. Nearly every command runs synchronously on the one UI thread that all 19 webviews share. One capability grants 88% of commands to 18 windows. The release exe is a console app whose console kills the shell when closed, and a crash leaves Explorer's taskbar hidden for good. CI has never run on `master`. Most tests regex source text rather than exercise behavior, which makes refactors expensive and lets real regressions through. On the visual side the default theme's tokens disagree with each other, there are about 60 font sizes (mostly 9–11px), no shared UI primitives, no Mica or acrylic, and no panel motion. The product is an *Explorer companion* (the tray, pins, Start, and the AppBar manager still come from Explorer), not yet a shell replacement. The biggest wins come from **fixing the foundations first**, which makes every later feature cheaper and safer.

## Cross-cutting themes

These are the patterns that show up in several lanes. Fixing the theme closes the listed IDs together.

### T1 — Everything blocks the UI thread
About 146 of 226 commands are synchronous, so Tauri runs them on the main/UI thread that every webview shares. Some of them do blocking I/O, sleeps, UAC waits, git, or full directory scans. A slow disk or a hung Explorer freezes the whole shell.
**IDs:** IPC-01, IPC-02, IPC-03, IPC-04, RS-01, RS-02, WIN-07, FE (search round-trip FE-29).
**Direction:** make commands `async` by default, with blocking work on `spawn_blocking` or a dedicated worker. Add a source test that fails any new sync command doing I/O. Fix RS-02 first: command-panel `Resized` currently does a settings read/parse/write on every resize event.

### T2 — Security boundary is mostly nominal
One capability grants 199 of 226 commands to 18 windows. About 93 commands take no window parameter and cannot check the caller. `auth.rs` holds a dead, wrong caller-rule table. `core:default` lets any webview emit events that normally come from Rust. Path-taking commands such as `open_shell_path` and `launch_app_path` trust the renderer.
**IDs:** IPC-08, IPC-09, IPC-12, IPC-13, IPC-20, IPC-21, SEC-* (see 05).
**Direction:** generate one command inventory and derive per-window capabilities from it. Delete or wire `auth.rs`. Drop `core:event:allow-emit` from renderer capabilities.

### T3 — Concrete exploitable / data-loss bugs (fix before anything else)
- **SEC-01:** "Open terminal here" falls through to `powershell -Command "Set-Location -LiteralPath $args[0]" <dir>`. PowerShell 5.1 appends trailing arguments to the script text, so a folder named `x;calc` runs `calc`. `wt.exe` lives in WindowsApps, not System32, so this PowerShell branch is the one that actually runs. Verified at `src-tauri/src/stack_popup/file_ops.rs:95-140`.
- **SEC-02:** the Everything SDK DLL is loaded from an env var and from the working directory and its parents before Program Files, with no signature check. That is DLL planting into the process that owns the keyboard hook.
- **SEC-03:** git runs on every browsed folder with no hardening, so a downloaded repo's `core.fsmonitor`, filter, or textconv settings can execute code. Use `-c core.fsmonitor=false`, `safe.directory` checks, `GIT_CONFIG_NOSYSTEM`, and `--no-optional-locks`.
- **WIN-16 / SEC-04:** taskbar Close escalates to `TerminateProcess` after about 1.75 s, including through an elevated helper, so apps showing "Save changes?" lose work. The spec says group Close-all is request-only, but the single-tile path is not.
- **WIN-04 + WIN-05/BLD-01:** there is no `windows_subsystem = "windows"`, so a release launch opens a console and closing it kills the shell. After any crash, Explorer's taskbar stays hidden and is then recorded as "originally hidden", so it is never restored.
- **RS-14:** the 64 KiB git stdout cap silently truncates status output on large changesets.

### T4 — No shell lifecycle robustness
The AppBar callback message is registered but never received, so ABN_* notifications are lost. Nothing handles `TaskbarCreated` (Explorer restart), `WM_DISPLAYCHANGE`/`WM_DPICHANGED`, session lock/unlock, or sleep/resume. Screen geometry is captured once at startup. The fullscreen guard can deadlock against `resize_shell_bar` (WIN-06). There is no panic hook, marker file, watchdog, or single-instance forwarding.
**IDs:** WIN-01..07, WIN-04, PRD-17 (single-instance forwarding).

### T5 — Data over IPC is fat and chatty
- Icons and previews travel as base64 PNG data URLs inside JSON, including in `list_processes`, which is polled every second.
- The taskbar snapshot is rebuilt every second for every top-level window and broadcast to every webview even when unchanged.
- Terminal output is sent twice, as an event per 4 KiB plus a 1 s poll under a global lock.
- The Stack popup polls `getStackPopupRequest` every 250 ms forever, even while hidden.

**IDs:** IPC-16, IPC-17, RS-25, RS-26, RS-31, RS-32, FE-18.
**Direction:** a custom `icon://` URI scheme with JSON carrying keys only; diff-only taskbar snapshots sent with `emit_to`; one `tauri::ipc::Channel` per terminal session carrying coalesced bytes; replace the 250 ms poll with an event.

### T6 — No generated contract between Rust and TS
There are 226 commands and five hand-kept name inventories that already disagree (226 / 216 / 219 / 199 / dead). The TS side has 364 hand-written types and no specta or ts-rs. Errors are `Result<_, String>` 1,031 times, and the frontend sometimes branches on error text. About 97 hand-rolled stale-request counters exist in TS.
**IDs:** IPC-08..11, IPC-25, RS (errors), TST-12, FE (types).
**Direction:** use `tauri-specta` to generate bindings and types, with a typed error enum (`thiserror` plus a serializable `code`). Add one shared `latestOnly()` / generation-guard primitive in TS.

### T7 — Tests guard text, not behavior
- 57% of Node test files only regex source text; about 76% of assertions are `assert.match`/`doesNotMatch`.
- 74 files parse Rust source with copy-pasted brace counters.
- 8 files return `''` when a source file is missing, so their `doesNotMatch` guards silently pass.
- Component tests, the 17 browser/CDP fixtures, and the runtime-ram suite are not in any gate.
- CI triggers on `main`, but the default branch is `master`; CI has run 6 times ever, all failed.

These tests are also the main obstacle to decomposing the god components (FE-44).
**IDs:** TST-01..19, CI-01..08, FE-44.

### T8 — Design system is implicit and inconsistent
- The default `base-dark` accent family contradicts itself: `accent-border` is `rgba(0,0,0,.44)` and is used 106 times.
- Muted and subtle text fail WCAG contrast across most of the 34 themes.
- There are about 60 font sizes, and Google Sans weights are faked.
- There are no shared Button, Menu, Input, or Spinner primitives: four spinners, three context-menu implementations, four close buttons.
- Focus rings disappear in forced-colors mode.
- Both terminals ignore the theme.
- There is no Mica, acrylic, or open/close motion.

**IDs:** VIS-01..14, VIS-20b, FE (shared UI kit BB-3).

### T9 — God components and legacy-mode Svelte
- StackPopupSurface (3.7k lines, 127 top-level `let`s, 203 functions), StackGitPanel (3.3k), TerminalPanelSurface (2.7k), and TopBar (2.3k) run in Svelte 4 legacy mode, with `$:` effects and an `afterUpdate` that forces layout.
- There are three xterm implementations; TerminalPanelSurface carries 25 duplicated `*ForRuntime` twins.
- Opening Stack eagerly loads about 1.2 MB of JS (xterm plus CodeMirror).
- Stack rows build an `Intl.DateTimeFormat` per row on every scroll frame.

**IDs:** FE-01..13, FE-25, FE-34.

### T10 — Duplicated subsystems
- Three "run a thing" systems: Quick Commands, `dev_tools/task_runner.rs`, and settings `task_history`.
- Two git layers, one of which runs git from PATH with no timeout.
- Four icon caches, each doing O(n) work per lookup.
- Old `search_sources/` alongside the new `search/`.
- Four persistence authorities, with localStorage primary for preferences and themes; settings broadcasts go three ways, so listeners fire twice.
- The Control Plane duplicates the Settings theme selector.

**IDs:** PRD-16, RS-13, RS-25, FE-41, FE-42.

### T11 — Observability and release are absent
There is no `tracing` or `log` (48 `eprintln!` calls), no benchmarks, and no `[profile.release]`. The release exe is 49.8 MB because ORT and tokenizers are statically linked. There is no installer, updater, or signing, no lint or format configuration, and `cargo fmt --check` reports 4,234 hunks. The 652 MB speech model loads eagerly at startup and stays resident.
**IDs:** RS (observability, profile), RS-41, RS-42, BLD-*, CI-*.

### T12 — Repo hygiene
- 25 root markdown plan files (about 1 MB) sit at the root, along with several `dist-*` directories.
- Tracked files that shouldn't be: Everything SDK DLLs and `obj/` (under a gitignored path), a Steam webhelper log at `src-tauri/logs/webhelper.txt`, `.code/agents`, and `.openchamber` screenshots.
- Both `bun.lock` and `package-lock.json` exist.
- Local branches hold a 652 MB ONNX model and `fusion.db`, so the pack is 464 MiB.
- `build.rs` rewrites tracked permission files on every build, which explains the many autogenerated permission diffs in the current dirty tree.

**IDs:** HYG-01..10.

## Triage tiers

### Tier 0 — Safety and data-loss (do first; mostly S effort)

| # | ID(s) | Fix | Effort |
|---|---|---|---|
| 1 | SEC-01 | Pass the directory via `-WorkingDirectory`/`current_dir`, or `-Command` with `-EncodedCommand`/`-File`; never append it to script text. Same audit for `cmd /K cd /d <dir>` (`&` in a name). | S |
| 2 | BLD-01 / WIN-05 | `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` in `main.rs`. | S |
| 3 | WIN-04 | Persist a "we hid the taskbar" marker before hiding; restore on next start if the marker is present; panic hook plus `SetConsoleCtrlHandler`/`WM_ENDSESSION` restoration. | S–M |
| 4 | WIN-16 / SEC-04 | Make single-tile Close request-only (`WM_CLOSE`); move terminate to an explicit "End task" with confirmation. | S |
| 5 | SEC-02 | Load the Everything DLL only from a fixed install path with `LOAD_LIBRARY_SEARCH_SYSTEM32`/an absolute path and an Authenticode check; load once. | S |
| 6 | SEC-03 | Harden every git invocation: `-c core.fsmonitor= -c core.hooksPath=NUL`, no textconv/filters for status, `--no-optional-locks`, `GIT_CONFIG_NOSYSTEM=1`, and a `safe.directory` ownership check. | S–M |
| 7 | RS-14 | Respect the truncation flag; stream or raise the cap for `status`; surface "too many changes". | S |
| 8 | CI-01 | Point the workflow at `master` (or rename the branch) and make it green. | S |
| 9 | WIN-06 | Never hold the shell-state lock across cross-thread `SetWindowPos`. | S–M |

### Tier 1 — High-leverage quick wins (S, mostly independent)

- **Perf:** RS-02 (debounce/move resize persistence off the main thread); FE-18 (replace the 250 ms Stack poll with an event); FE-25 (hoist `Intl.DateTimeFormat`, index git entries by path, use a `Set` for selection); FE-34 (dynamic `import()` for terminal/editor/git panes); RS-20/21 (`DirEntry::metadata`, cached sort keys); RS-31 (filter invisible windows before querying title/icon; diff-only snapshot).
- **Visual:** VIS-01 (fix the base-dark accent tokens); VIS-14 (per-theme ANSI palettes for xterm); VIS-10 (use `outline` for focus rings plus `forced-colors` support); VIS-20b (notification banner: bigger dismiss target, `aria-live`, pause on hover); VIS-03 (contrast gate in `generate-theme-palettes.mjs`).
- **Product:** PRD-03 (Quick Commands in search); PRD-01/02 (Calculator/Web/Bookmarks providers, which the contracts already declare).
- **Build/hygiene:** `[profile.release]` (lto, codegen-units=1, strip); untrack the junk files (HYG-02..04); pick one lockfile; stop `build.rs` from rewriting tracked files when unchanged.
- **Tests:** TST-03 (missing source file → fail, not `''`); add component tests to `validate` and CI.

### Tier 2 — Structural foundations (M–L; unlock everything after)

1. **Async-by-default command layer** (T1), with a guard test.
2. **Generated contracts** (T6): `tauri-specta` bindings, a typed error enum, a single command inventory that also generates capabilities (T2).
3. **Least-privilege capabilities** (T2): per-window capability files derived from the inventory; remove renderer `emit`.
4. **Shell lifecycle** (T4): an AppBar callback window, `TaskbarCreated`, display/DPI/session/power handling, a watchdog, single-instance forwarding.
5. **IPC transport upgrade** (T5): `icon://` scheme, terminal Channels, diff snapshots, `emit_to` everywhere.
6. **Observability**: `tracing` with a rolling file appender, spans on commands, and the existing `report_shell_surface_runtime_metrics` used to *measure* before big perf work.
7. **Test strategy reset** (T7): freeze new regex-source tests; convert high-value ones to behavior tests; typed `mockIPC` generated from the bindings; a WebView2 CDP E2E harness on the existing browser fixtures; CI matrix (node, component, cargo test, clippy, fmt).
8. **Design-system kit** (T8): a `src/ui` primitives package (Button, IconButton, Menu, Input, Select, Spinner, Tooltip, VirtualList, XtermView, DiffView), a 4/8/12 radius scale, a ~6-step type ramp (12/13/14/16/20/28), a motion tokens file, Segoe UI Variable with a real variable font.
9. **Svelte 5 runes migration** of the god components, after tests are behavior-based; split StackPopupSurface into listing, selection, drag, clipboard, and routing modules with `.svelte.ts` stores.
10. **Subsystem consolidation** (T10): one run/task system, one git layer, one icon cache, one persistence authority (Rust-owned settings, with the renderer as a cache).

### Tier 3 — Big bets (pick a few)

Drawn from every lane's big-bets section. See the lane files for full write-ups.

- **Fluent-grade visual refresh**: Mica/acrylic backdrops via `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE)` and wry transparency, spring-based panel motion, a real type ramp, and theme-driven terminals (01).
- **Webview consolidation / lazy creation**: 19 eager webviews → lazy creation plus a few multiplexed host webviews. Measure WebView2 private bytes first (02 BB-1).
- **Rust as single source of truth**: versioned Channel snapshots feeding rune stores in every surface (02 BB-2, 03).
- **Search as the shell hub**: a Ctrl+K action panel per result, providers as plugins, clipboard history, calculator, window actions, quick commands, settings deep links (06 PRD-04, PRD-10).
- **Automation/CLI + single-instance forwarding**, enabling voice commands, Stream Deck/AHK, and scripting (06 PRD-17).
- **Context Engine / Workspaces**: switch the whole machine between contexts (windows, folders, terminals, commands) (06 BB-01).
- **True shell replacement path**: own tray host (`Shell_NotifyIcon` broker instead of scraping Explorer), per-monitor bars, virtual desktops, SMTC media, notification center (05 matrix, 06).
- **Native-speed git**: move from 6–8 `git.exe` spawns per refresh to `gix` or one `status --porcelain=v2 --branch -z` call (04).
- **Lazy speech model** with ORT session options and an unload-on-idle policy, saving about 650 MB resident when unused (04 RS-41).
- **Release pipeline**: NSIS/MSI bundle, updater, signing, and dynamic ORT linkage to shrink the 49.8 MB exe (07, 04).

## Suggested sequencing

1. **Week-scale, Tier 0** (all S/M; one fixer plus a reviewer each). Do the SEC items and the console/taskbar restore first.
2. **CI green on master** with node, component, cargo test, fmt (non-blocking at first), and clippy (warn).
3. **Observability + measurement baseline**: tracing, plus a runtime-metrics capture of webview memory, IPC payload sizes, and main-thread stalls. Later perf work then gets proven numbers.
4. **Async command layer + generated bindings**. These are mechanical but wide. Do them before decomposition so the new modules use typed calls.
5. **Capabilities rebuilt from the generated inventory.**
6. **Shell lifecycle robustness** (T4).
7. **Design-system kit + token fixes**, then migrate surfaces one at a time (start with the notification banner, Settings, and Search, the smallest high-visibility ones).
8. **Test strategy reset alongside runes migration** of the god components.
9. **Product big bets** on the new foundation: search hub, then automation, then workspaces.

## Things that are good and should be preserved

Listed so the refactor doesn't regress them (details in 04 RS-60, 03 IPC-35, 05):
- the low-level keyboard hook design (lean callback, atomic gating, bounded drain)
- latest-only search dispatch
- `ipc::Response` raw bytes for snip images
- the quick-command notification `Channel` with a least-privilege capability
- `ReadDirectoryChangesW` folder watching
- handle-based, reparse-aware local-drop rename and undo
- identity-revalidated process kill
- BatBadBut-safe Quick Command launching
- RAII DWM thumbnail handles
- the spec's discipline about "truthful limits"

## Open questions for the human reviewer

- **Scope ambition:** stay an *Explorer companion* (lean on Explorer for tray, Start, and pins), or commit to a *shell replacement* (own the tray host, Start, and multi-monitor)? Many T4/T3 items change shape depending on the answer.
- **Default branch:** rename `master` to `main`, or retarget CI to `master`?
- **Test policy:** may existing regex-source tests be deleted when replaced by behavior tests, or must they be kept? AGENTS.md and the current tests encode wording, so changing this needs your sign-off.
- **Speech:** keep the bundled ORT static (simple, 50 MB exe), or move to dynamic/on-demand loading?
- **Live smoke consent:** several Tier 0/2 items (AppBar callbacks, Explorer restore, DPI) need consent-gated native smoke to certify.
