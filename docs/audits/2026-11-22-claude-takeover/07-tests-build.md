## Testing, build, CI & repo hygiene audit

Scope: test strategy/quality, CI, build/release pipeline, tooling, repo hygiene, DX, doc sprawl (structure only). Read-only. Audit date 2026-11-22, repo `C:\dev\jasonshell` at `master` (af3953e, dirty worktree untouched). Commands run: `git ls-files`, `git count-objects`, `git rev-list --objects --all`, `gh run list`, `cargo fmt --check` (check-only), `npm run test:node` (pass), file counting/grep. No cargo test, smoke scripts, component tests, or app launch were run.

### Inventory

| Layer | Location | Files | Cases / size | Wired into `npm run validate`? | Wired into CI? |
|---|---|---|---|---|---|
| Node "contract" tests | `tests/*.test.mjs` | 188 | 1,449 tests (1,442 pass, 4 skip, 3 todo), 26,445 LOC, 6,180 `assert.match/doesNotMatch` vs 1,991 `assert.equal/deepEqual/...` | yes (`test:node`) | yes (but CI never green, see CI-01) |
| Node test compile step | `tsconfig.test.json` -> `dist-tests/` | src/lib, features, ipc, icons | `tsc` emit | yes | yes |
| Component tests (vitest + jsdom + @testing-library/svelte) | `tests/components/*.test.ts` | 36 test files | ~373 `it/test` cases, 8,883 LOC | yes (`test:component`) | **no** |
| Runtime-RAM vitest suite | `tests/runtime-ram/*.test.ts` + own `vitest.config.ts` | 6 `.test.ts`, plus ps1/cs observers | n/a | **no** (no npm script references it) | no |
| Browser/CDP fixtures | `tests/browser/*` | 17 fixture dirs, 138 files, 54 `.mjs` runners | headless Chrome + Vite + `mockIPC` | **no** | no |
| Speech/theme e2e fixture | `tests/speech-theme-e2e` | 5 | runner.mjs | no | no |
| Native Rust test sources outside cargo | `tests/native/*.rs`, `tests/native/stack-local-drop`, `wry-drop-negotiation` | 12 | compiled by Node tests / `#[path]` | partially | partially |
| PowerShell native e2e | `tests/*.ps1`, `tests/runtime-ram/*.ps1`, `scripts/runtime-smoke.ps1`, `scripts/smoke-*.ps1` | ~15 | manual, consent-gated | `smoke:runtime` is `-DryRun` only | no |
| Rust unit tests | inline in `src-tauri/src/**` | 91 files with `#[cfg(test)]` (153 `.rs` files, 89,521 LOC) | 876 `#[test]` | yes (`cargo:test`) | yes |
| Rust integration tests | `src-tauri/tests/*.rs` | 30 test crates + fixtures | 354 `#[test]`, 7,107 LOC; 24 `#[ignore]` | yes | yes |
| Fixtures/evidence | `tests/fixtures`, `tests/*.md` (20 evidence/handoff markdown files) | 20 + 20 | | n/a | n/a |
| CI | `.github/workflows/windows-ci.yml` | 1 | 6 runs total ever, all failed, last 2026-05-13 | | |
| Lint/format configs | eslint/prettier/rustfmt/clippy/deny/editorconfig | **0** | `cargo fmt --check`: 4,234 diff hunks in 1,292 files-locations | no | no |
| Tracked files | `git ls-files` | 1,494 | pack 464 MiB (history) | | |

Node test classification (heuristic via grep over all 188 files, `/scratchpad/cls.txt` method: readFileSync count, `dist-tests` import, match count):

| Class | Files | Share |
|---|---|---|
| Pure source-text (reads `src/`/`src-tauri/` files, no import of compiled code) | 108 | 57% |
| Mixed (imports `dist-tests` behavior AND regexes source) | 43 | 23% |
| Pure behavior (imports compiled code, zero `assert.match`) | 26 | 14% |
| Other (spawn/native/doc-only) | 11 | 6% |

Further: 142 files read source files; 74 read Rust `.rs` source; 98 read `.svelte`; 44 read `.css`; 13 read markdown docs (README, master_spec, AGENTS.md, changelog, CHANGELOG_POLICY, docs/current-state-brief). Assertion ratio: ~76% of assertions in Node tests are regex matches against text.

Sampled files (read or classified): `stackGitPanelRedContract.test.mjs` (540 regex), `commandPanelWiring.test.mjs` (354), `meltMigrationWiring.test.mjs`, `backendBlockingLockBoundaries.test.mjs`, `productTruthDocs.test.mjs`, `changelogPolicy*.test.mjs`, `searchPanelState.test.mjs`, `stackPopupViewModel.test.mjs`, `workspaces.test.mjs`, `overlayDismissalWiring.test.mjs`, `taskbarGalleryContract.test.mjs`, `speechIndicator.test.mjs`, `performanceBaselineContract.test.mjs`, `rustcBuildScriptWrapper.test.mjs`, `snippingNativeJourney.test.mjs`, `distTestsHygiene.test.mjs`, `audioControls.test.mjs`.

---

### Testing findings

- **TST-01 — Majority of Node tests assert source text with regex, not behavior** (High, L).
  Evidence: 108/188 files are pure source-text; 6,180 `assert.match/doesNotMatch` vs 1,991 equality assertions. `tests/commandPanelWiring.test.mjs:5-25` reads 20 files (App.svelte, TopBar.svelte/.css, CommandPanelSurface.svelte/.css, shell_windows.rs, main.rs, command_panel.rs, contracts.rs, capabilities JSON) then e.g. `assert.match(shellWindowsSource, /COMMAND_PANEL_WIDTH_LOGICAL: f64 = 460\.0/)` and `/fn build_command_panel_window\(app: &App\)[\s\S]*?\.resizable\(true\)/`. `tests/meltMigrationWiring.test.mjs:43-48` asserts `import \{ Select \} from 'melt\/builders'`, `new Toggle`.
  Problem: these tests pin implementation spelling (constant names, import lines, method-chain order, CSS text). Any rename, rustfmt run, extraction into a helper, or equivalent refactor fails them, while a real behavior break (wrong value wired, logic inverted in another function) passes. They are change detectors, not regression detectors, and they impose a per-change "update the regexes" tax, which also explains why `cargo fmt` has never been applied (BLD-07): formatting would break hundreds of regexes.
  Recommendation: freeze new source-regex tests (policy + reviewer checklist). Classify existing tests into (a) keep: true architecture invariants (e.g. "no `unsafe` in module X", capability JSON allowlists, command name registry parity) — convert to structured parsing (JSON parse, `syn`-based Rust check in a cargo test, TS AST); (b) replace with behavior tests in vitest/Rust; (c) delete. Track ratio as a metric.

- **TST-02 — Node tests reach into Rust source with hand-written brace-matching parsers** (High, M).
  Evidence: 74 test files read `src-tauri/src/*.rs`. `tests/backendBlockingLockBoundaries.test.mjs:11-27` defines `extractFunction` (regex + `{`/`}` depth counter); `extractFunction` is copy-pasted in 9 files, with variants `extractFunctionBody`, `extractRustFunction`, `extractSpawnBlockingClosure`, `extractListenHandler`, `extractFunctionByPattern`, `extractCssRuleBody` (16 files total). `tests/stackGitPanelRedContract.test.mjs:32-36` has `stripRustTestBlocks` regex that depends on rustfmt's unindented `}`.
  Problem: brace counting ignores braces in strings/chars/comments/raw strings/`format!` args; behavior-relevant Rust facts ("runs behind spawn_blocking", "lock not held across await") are checked lexically, not semantically. False confidence plus brittleness.
  Recommendation: move Rust invariants into Rust: unit tests that exercise the function, or a `syn`-based lint test in `src-tauri/tests/`, or clippy lints (`clippy::await_holding_lock`, `await_holding_refcell_ref`) which cover several "lock boundary" intents natively.

- **TST-03 — Silent empty-file fallback makes negative assertions vacuous** (High, S).
  Evidence: 8 files use `existsSync(url) ? readFileSync(url) : ''` (e.g. `tests/stackGitPanelRedContract.test.mjs:6-9` `readRepoFile`). Those files contain many `doesNotMatch` assertions: `stackBrowserTerminal.test.mjs` 50, `stackGitPanelRedContract.test.mjs` 48, `stackPopupGitStatus.test.mjs` 17, `terminalActions.test.mjs` 7. Repo-wide there are 799 `assert.doesNotMatch`.
  Problem: if a file is renamed/moved, `''` satisfies every `doesNotMatch` and the test silently stays green for the negative half. A renamed `git_status.rs` would pass all "must not contain X" guards.
  Recommendation: make the reader throw on missing paths (one shared helper), and add a "file list exists" assertion up front.

- **TST-04 — Tests assert prose in README / master_spec / AGENTS.md / changelog** (Medium, S).
  Evidence: 13 files read markdown. `tests/productTruthDocs.test.mjs:13-26` regexes README and master_spec (`/workspace restoration[^.;]*(?:is )?reserved[^.;]*not implemented/i`); `changelogPolicy.test.mjs:7` and `changelogPolicyHygiene.test.mjs:6-8` read `AGENTS.md` and `changelog.md` (392 KB); `stackBrowserTerminal.test.mjs:28` reads changelog; `searchOverhaulPhase0.test.mjs` references removed `search_overhaul.md` (3 `todo`s at 279/308/499 citing it was "superseded ... and removed").
  Problem: documentation edits break CI; doc tests encode wording, not truth. Agent-instruction file (AGENTS.md) is effectively locked by tests.
  Recommendation: keep at most a tiny link-checker / "required sections present" doc lint as a separate non-blocking `docs:check` script; delete wording regexes and stale `todo`s.

- **TST-05 — Test files are named after plan phases, not product behavior** (Medium, M).
  Evidence: 22 files with Phase/P#/Red/Hygiene/Spec names, e.g. `stackPopupPagingPhase1Wiring` … `Phase6Responsiveness` (6 files for one feature), `searchOverhaulPhase0/Phase6`, `backendBlockingLockBoundariesP6`, `multiFixPhase6SpecValidation`, `stackTextEditorP02RunnerContract`, `stackGitPanelRedContract`. Root has matching plan docs (`backend_blocking_lock_boundaries_p6.md`, `contract_test_spec_hygiene_p7.md`).
  Problem: tests accrete per work-package and are never consolidated; locating "the tests for paging" requires knowing history; overlapping/contradicting assertions across phases are likely.
  Recommendation: consolidate per feature (`stackPopupPaging.test.mjs`), delete superseded phase tests during the TST-01 triage.

- **TST-06 — No shared test helper module; heavy duplication** (Medium, S).
  Evidence: no `tests/helpers/` or `tests/*.mjs` non-test modules; `readRepoFile`/`read`/`extractFunction` re-implemented per file; 7 files use cwd-relative `readFileSync('src-tauri/...')` (e.g. `backendBlockingLockBoundaries.test.mjs:5`) while others use `new URL('../..', import.meta.url)`.
  Problem: cwd-relative reads fail when tests are run from another directory or IDE runner; inconsistency across 188 files.
  Recommendation: `tests/support/repo.mjs` (throwing `readRepo`, `repoPath`), `tests/support/rust.mjs` if any parser survives; codemod to it.

- **TST-07 — Component tests are not in CI** (High, S).
  Evidence: `.github/workflows/windows-ci.yml` steps: check, build, test:node, cargo test, cargo check. `test:component` absent, though `scripts/run-validation.mjs:7` includes it locally.
  Problem: the highest-value behavior layer (36 files, ~373 cases, real Svelte rendering) is not gated remotely.
  Recommendation: add `npm run test:component` step (and see CI-01, which matters first).

- **TST-08 — Runtime-RAM vitest suite is orphaned and uses source string-injection** (Medium, S).
  Evidence: `tests/runtime-ram/vitest.config.ts` includes `tests/runtime-ram/**/*.test.ts`; no npm script or `run-validation.mjs` stage references it (grep for `runtime-ram` in package.json/scripts/.github: none). Its plugin does `source.replace('</script>', ... export function __ramProbe() ...)` keyed on component file names and internal variable names (`processes`, `taskbarProcessPids`, `iconCache.retainedBytes`, `paneRuntimes`).
  Problem: suite rots silently; probe expressions reference private state names, breaking on any rename with a cryptic compile error, or silently on first `</script>` mismatch (module scripts).
  Recommendation: add `test:ram` script, run it in nightly/optional CI; expose observability via a deliberate test hook instead of textual injection.

- **TST-09 — Browser/CDP fixtures are ad hoc, duplicated, and not runnable as a suite** (Medium, M).
  Evidence: 17 fixture dirs under `tests/browser/`, 54 runner `.mjs` files, 5 files each spawning Chrome with `--remote-debugging-port` (e.g. `tests/browser/stack-drag/run.mjs:11-21`: `CHROME_PATH` or hard-coded `C:/Program Files/Google/Chrome/Application/chrome.exe`), 6 files with hand-rolled CDP WebSocket/`Runtime.evaluate`. Mocks via `@tauri-apps/api/mocks` `mockIPC` in `tests/browser/ui-audit/main.ts:2,25`. No `test:browser` script; each has its own README (17 READMEs).
  Problem: valuable real-layout/real-pointer coverage exists but is invisible to validation; each fixture reinvents server+browser+CDP plumbing; Chrome path assumption differs from WebView2 runtime.
  Recommendation: adopt Playwright test (`@playwright/test`) as the single runner for these fixtures (it already does headless Chromium, CDP, screenshots, traces), one `playwright.config.ts` with a project per fixture, `npm run test:browser`, run in CI (Chromium on windows-latest). Keep `mockIPC` but type it (TST-12).

- **TST-10 — Permanently skipped "RED-ready" tests and stale todos** (Low, S).
  Evidence: `tests/speechContracts.test.mjs:69` `t.skip('RED-ready acceptance: enable when local TDT fallback IPC field is implemented')`; `tests/topBarMicControl.test.mjs:305` same pattern; `tests/topBarMicControl.rendered.test.mjs:4` `t.skip('Known limitation: agent-browser open times out on this host...')` (unconditional); 3 `todo`s in `searchOverhaulPhase0.test.mjs`. Run output: skipped 4, todo 3.
  Problem: dead tests that report as green-ish; unconditional skip hides the whole rendered test file.
  Recommendation: convert to issues/backlog items and delete, or `test.todo` with an owner; remove the unconditional rendered skip file.

- **TST-11 — No coverage measurement anywhere** (Medium, S).
  Evidence: `vitest.config.ts` has no `coverage` block; no `@vitest/coverage-v8` dependency; Node `--test` run without `--experimental-test-coverage`; no `cargo llvm-cov`. `.gitignore` lists `coverage/` but nothing produces it.
  Problem: no way to tell that, e.g., Stack popup Rust (155 KB `stack_popup.rs`, 151 KB `git_status.rs`) or front-end surfaces lack behavior coverage; regex tests inflate perceived coverage.
  Recommendation: add `vitest --coverage` (v8) and `cargo llvm-cov --summary-only` as informational CI artifacts; no thresholds initially.

- **TST-12 — IPC mocks are untyped and disconnected from Rust command signatures** (High, M).
  Evidence: `tests/components/setup.ts:3-13` mocks `invoke` as bare `vi.fn()`; 76 ad-hoc `invoke.mockImplementation/mockResolvedValue` across component tests; browser fixtures use `mockIPC((command, args) => ...)` switch statements. Cargo deps contain no `specta`/`tauri-specta`/`ts-rs`; command-name parity is checked by regex (`workspace IPC wrapper uses centralized command names registered by Rust` in test output).
  Problem: a renamed field in a Rust `#[derive(Serialize)]` struct or a changed arg name passes every frontend test (mocks return whatever the test author typed) and every regex test that does not literally match that field. This is the most likely real-world regression class for a Tauri app and is essentially uncovered.
  Recommendation: generate TS bindings from Rust (`tauri-specta` or `ts-rs`) into `src/ipc/generated.ts`, commit them, CI-check `git diff --exit-code` after generation; build a typed mock factory (`mockCommand<'stack_list'>(...)`) so tests fail to compile on contract drift.

- **TST-13 — Component tests are not type-checked** (Medium, S).
  Evidence: `tsconfig.json` include is `src/**` only; `svelte-check --tsconfig ./tsconfig.json`; vitest transpiles without type-checking. `tests/components/*.ts` (8,883 LOC) and `tests/browser/*.ts` never pass through `tsc`.
  Problem: test code can call component props/IPC shapes that no longer exist and still run (esp. with loose mocks).
  Recommendation: `tsconfig.tests.json` extending main with `tests/**/*.ts`, add to `npm run check`.

- **TST-14 — Flakiness risks in component tests: real timers and broad waitFor** (Low, M).
  Evidence: only 9/36 component files use `vi.useFakeTimers`; 9 instances of `new Promise(... setTimeout ...)` sleeps; 397 `waitFor(` calls; `settings-hotkeys.test.ts:416` `waitFor(..., { timeout: 1500 })`. 12 Node test files use `setTimeout`, 36 spawn child processes.
  Problem: timing-dependent tests become flaky on loaded CI runners (Windows runners are slow).
  Recommendation: prefer fake timers + `vi.advanceTimersByTimeAsync`; replace sleeps with awaited state; run vitest with `--retry=0` and track failures; for spawn tests set explicit timeouts.

- **TST-15 — Surface components without direct component tests** (Medium, M).
  Evidence: only 19 of 45 `.svelte` files are imported directly by `tests/components` or `tests/runtime-ram`. Not directly mounted: `AudioPanelSurface`, `CalendarPanelSurface`, `ControlPlaneSurface`, `QuickLaunchPanelSurface`, `SnipOverlaySurface`, `SnipPreviewSurface`, `SpeechHistoryPanelSurface`, `TrayPanelSurface`, `TopBarMicControl`, `ContextMenu`/`ContextMenuItem`/`ContextMenuSeparator`, `StackMarkdownPreview`, `ThemeColorField`, `ThemeEditor` (covered transitively via `SettingsPanelSurface` in `theme-editor.test.ts`), `TerminalSearchCloseButton`, `AnchoredPanelCloseButton`, `SpeechIndicatorOrb` (lib tested, component not), `TextEditorProjectionExperiment`.
  Problem: these surfaces are covered only by regex tests (if at all). Context menu and snip overlay are high-interaction UI.
  Recommendation: add smoke render + key interaction tests for each surface (mount, open/close, keyboard nav, IPC call made with expected args).

- **TST-16 — Rust integration tests compile production modules via `#[path]` and spawn nested cargo** (Medium, M).
  Evidence: no `src-tauri/src/lib.rs`; tests use `#[path = "../src/snipping/mod.rs"] mod snipping;` (3x), `../src/task_gallery/layout.rs`, `../src/speech_model.rs`, `../src/process_manager/system_info.rs`, etc. `snipping_hotkey_settings.rs:10`, `snipping_runtime_acceptance.rs:48`, `speech_model_folder_runtime.rs:21` run `Command::new(env!("CARGO"))` inside a test. `settings_owner_hotkey_save_contract.rs:9,118,124`, `shell_app_acl.rs:16`, `snipping_wry_retirement.rs:16` read Rust source text with `read_to_string` (same anti-pattern as TST-02, in Rust).
  Problem: `#[path]` re-compiles modules per test crate (slow, and modules compile in a different crate context with different `cfg`/deps), nested cargo inside `cargo test` contends for the target-dir lock and multiplies build time; text-reading tests are brittle.
  Recommendation: split a `jason_shell_core` lib crate (or `src/lib.rs` + thin `main.rs`) so integration tests `use` real APIs; replace nested cargo with `CARGO_BIN_EXE_*` binaries or `[[test]] harness` fixtures.

- **TST-17 — Native/consent-gated E2E exists but is env-flag scattered** (Low, S).
  Evidence: env gates `JASONSHELL_SNIP_JOURNEY_TESTS`, `JASONSHELL_SNIP_CLIPBOARD_TESTS`, `JASONSHELL_ACL_NATIVE_TESTS`, `JASONSHELL_PHYSICAL_CLIPBOARD_TEST`, `JASONSHELL_SNIP_TOPBAR_POINTER_TESTS`, `JASONSHELL_TEST_EXE` …; 24 `#[ignore]` in `src-tauri/tests` (13 in `snipping_clipboard_native.rs`).
  Problem: no single "run the native lane" command; nobody knows which flags exist; ignored tests never run.
  Recommendation: `npm run test:native` that sets the flags and runs `cargo test -- --ignored` + gated Node tests, documented in README; optionally a self-hosted runner.

- **TST-18 — Node test suite is fast and green (positive)** (Info).
  Evidence: `npm run test:node` real 14.0 s, `duration_ms 10294`, 1,449 tests, 0 fail. Tests run in parallel via `node --test`.
  Note: speed is a strength to preserve while migrating behavior tests into this lane (pure-TS view-model tests compiled into `dist-tests` are fast and robust — e.g. `stackPopupViewModel.test.mjs` imports compiled code with 13 behavior tests and zero regex).

- **TST-19 — Test artifacts and evidence docs live inside `tests/`** (Low, S).
  Evidence: 20 markdown files at `tests/` root (`snipping-product-handoff.md`, `graceful-exit-native-evidence.md`, `seven-hotkeys-evidence.md`, …), `tests/fixtures/terminalRetention.evidence.json`, `tests/browser/snip-preview-geometry.md`, per-fixture READMEs.
  Problem: tests directory mixes executable tests with session handoff notes; evidence becomes stale and is asserted by nothing.
  Recommendation: move evidence to `docs/evidence/` (or archive), keep `tests/` executable-only plus one `tests/README.md` index.

### CI findings

- **CI-01 — CI never runs on the real default branch and has never passed** (Critical, S).
  Evidence: `.github/workflows/windows-ci.yml` `on: push: branches: [main]`, `pull_request`. Remote default is `origin/HEAD -> origin/master`; local `main` is stale (`9e6e82d 2026-05-07`, 161 commits behind `master`). `gh run list`: 6 runs total, all `failure`, all `pull_request` on `fusion/fn-00x`, last `2026-05-13`. Direct pushes to `master` (all recent work: af3953e etc.) trigger nothing.
  Problem: there is effectively no CI. Six months of commits are unvalidated remotely.
  Recommendation: change trigger to `master` (or rename default to main and delete stale `main`), fix to green, enable branch protection requiring the check.

- **CI-02 — CI omits component tests and themes/format/lint; duplicates cargo work** (Medium, S).
  Evidence: steps: check, build, test:node, `cargo:test`, then `cargo:check` (check after test is redundant: test already compiles everything). No `test:component` (TST-07), no clippy/fmt (BLD-07/08).
  Recommendation: order: install -> `themes:check` -> `check` -> `test:node` -> `test:component` -> `cargo fmt --check` -> `cargo clippy -D warnings` (after baseline) -> `cargo test` -> `build`. Drop `cargo check`.

- **CI-03 — No Rust/npm caching; full Windows Rust build each run** (Medium, S).
  Evidence: only `actions/setup-node cache: npm`; no `Swatinem/rust-cache`. Dependency tree includes tauri, windows (40+ features), parakeet-rs, cpal, portable-pty; release exe 49.8 MB.
  Problem: cold Windows Rust builds of this tree take tens of minutes, discouraging CI use.
  Recommendation: add `Swatinem/rust-cache@v2` with `workspaces: src-tauri`, consider `sccache`.

- **CI-04 — Toolchain not pinned; Node versions diverge** (Medium, S).
  Evidence: CI `node-version: 20`; local `node v24.14.0`; `@types/node ^24`; no `engines`, `.nvmrc`, `packageManager`. Rust `dtolnay/rust-toolchain@stable`; no `rust-toolchain.toml`; local rustc 1.97.1.
  Problem: CI and local differ (Node 20 vs 24 APIs, e.g. `fs` globs/`--test` features); new stable Rust may break builds (new lints with `#![deny]`).
  Recommendation: add `.nvmrc`/`engines` (24), `rust-toolchain.toml` pinning a version + components (rustfmt, clippy).

- **CI-05 — The repo-local rustc wrapper is mandatory for any cargo invocation** (Medium, S).
  Evidence: `.cargo/config.toml` sets `build.rustc-wrapper = ".local/rustc-build-script-wrapper.exe"`; exe is produced only by `scripts/prepare-rustc-wrapper.mjs` via `pre*` npm hooks. README line 305 documents the workaround (Blaze/ra-jit stub + cmd 8,191 char limit).
  Problem: bare `cargo test`, `cargo clippy`, `cargo build`, rust-analyzer in a fresh clone, or a CI step that calls cargo directly fails with a missing wrapper. Workaround is specific to one developer's Blaze/ra-jit setup but imposed on everyone, including CI.
  Recommendation: make the wrapper opt-in (env var or user-level `~/.cargo/config.toml`), or have the wrapper fall through when absent (e.g. `RUSTC_WRAPPER` set by `npm run` only). At minimum CI should not depend on it.

- **CI-06 — No dependency/security scanning** (Medium, S).
  Evidence: no `.github/dependabot.yml`, no renovate config at root (only vendored `src-tauri/vendor/wry/renovate.json`), no `cargo deny`/`cargo audit`, no `npm audit` step. Pinned-exact versions (`tauri-plugin-dialog = "=2.4.0"`, `@tauri-apps/api 2.10`, `vitest 4.1.11`, `jsdom 27.4.0`).
  Recommendation: Dependabot for npm + cargo + github-actions (weekly, grouped); `cargo deny check advisories licenses bans` (license check matters given vendored wry and onnx/parakeet deps).

- **CI-07 — Workflow hardening gaps** (Low, S).
  Evidence: no `permissions:` block, no `concurrency:` group, no `timeout-minutes`, actions pinned by tag (`actions/checkout@v4`), no artifact upload of test reports.
  Recommendation: `permissions: contents: read`, `concurrency: ci-${{ github.ref }}` with cancel-in-progress, `timeout-minutes: 60`, upload node/vitest JUnit + screenshots on failure.

- **CI-08 — No pre-commit hooks or local fast gate** (Low, S).
  Evidence: `.git/hooks` contains only samples; no husky/lint-staged/lefthook.
  Recommendation: lefthook (Windows-friendly) running `cargo fmt --check` on staged `.rs`, `prettier --check`, `themes:check`; keep it < 10 s.

### Build / release findings

- **BLD-01 — Release binary is a console-subsystem executable** (High, S).
  Evidence: PE header of `src-tauri/target/release/jason-shell.exe` read via Node: `subsystem 3 (2=GUI,3=console)`. `src-tauri/src/main.rs:1-2` has `#![cfg_attr(not(target_os = "windows"), allow(dead_code))]` and `#![deny(clashing_extern_declarations)]` but no `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` (grep for `subsystem` in main.rs/build.rs: none).
  Problem: launching the release shell from Explorer/startup opens a console window behind the shell; closing that console kills the shell. Unusual for a desktop shell, unless intentional (no comment says so).
  Recommendation: add the standard Tauri `windows_subsystem` attribute for release; route diagnostics to a log file (BLD-09).

- **BLD-02 — No installer/bundle configuration, updater, or signing** (Medium, M).
  Evidence: `src-tauri/tauri.conf.json` `bundle` contains only `"resources": ["resources/quick-command-complete.wav"]` — no `active`, `targets`, `icon`, `windows.nsis/wix`, `certificateThumbprint`, `publisher`. No `tauri-plugin-updater`. `.gitignore` ignores `*.cer`/`*.pfx`, implying signing was considered but never configured. No release workflow.
  Problem: there is no reproducible artifact a user can install; Tauri 2 bundling defaults off unless `bundle.active` is true, and icons are not referenced explicitly. Unsigned executables that hook keyboard (Windows key hook), register AppBars, and inject into taskbars trip SmartScreen/AV.
  Recommendation: decide distribution (NSIS per-user is simplest); configure `bundle.active/targets/icon/publisher`, add a tagged `release.yml` building NSIS with signing via a secret, optionally updater with signed manifests.

- **BLD-03 — No Cargo release profile; 49.8 MB release binary** (Medium, S).
  Evidence: `src-tauri/Cargo.toml` has no `[profile.release]`; release exe 49,806,336 bytes; debug 75 MB.
  Recommendation: `[profile.release] lto = "thin"`, `codegen-units = 1`, `strip = "symbols"` (keep PDB separately for crash triage), `panic` left as unwind if catch_unwind is used. Measure size/startup before/after. Consider a `[profile.dev.package."*"] opt-level = 2` for faster debug runtime of heavy deps (onnx/parakeet).

- **BLD-04 — build.rs writes into the source tree during every build** (Medium, S).
  Evidence: `src-tauri/build.rs:36-48` builds `permissions/legacy-shell.toml` from `registered_commands()` and `fs::write(path, ...)` when content differs; tauri-build regenerates `src-tauri/permissions/autogenerated/*.toml` (225 tracked permission files). Current dirty tree shows 5 modified + 2 untracked autogenerated toml files.
  Problem: builds mutate tracked files (dirty trees after a plain build, merge conflicts in generated files, CI cannot detect drift because nobody checks `git diff`). `rerun-if-changed=src/main.rs` only.
  Recommendation: either stop tracking generated permission files, or keep them tracked and add a CI step `git diff --exit-code src-tauri/permissions` after build so drift is caught deliberately.

- **BLD-05 — Vendored, patched wry pins the whole Tauri stack** (Medium, M).
  Evidence: `[patch.crates-io] wry = { path = "vendor/wry" }` (79 tracked files, version 0.54.4, `vendor/wry/JASONSHELL_PATCH.md` documents backport of upstream PR 1795 / commit 3fbf592f); README:249 "released Wry 0.56.1 is incompatible with the current `tauri-runtime-wry ^0.54.0` line". Lock: tauri 2.10.3.
  Problem: good provenance documentation, but no automated signal for when the patch can be dropped; any tauri minor bump that moves `tauri-runtime-wry` off 0.54 silently stops using the patch (patch only applies to matching semver) or fails resolution. `tests/snipping_wry_retirement.rs:16` checks vendored source text, not behavior.
  Recommendation: add a CI assertion that `cargo tree -i wry` resolves to the path source; create a tracked issue "drop vendored wry when tauri-runtime-wry >= 0.56"; Dependabot will surface the bump.

- **BLD-06 — `npm run build` mixes gates with bundling; `tauri build` re-runs them** (Low, S).
  Evidence: `"build": "npm run themes:check && tsc --noEmit && vite build"`; `tauri.conf.json beforeBuildCommand: "npm run build"`; `"check": "npm run themes:check && svelte-check ..."`. `themes:check` runs twice in validate (check + build).
  Recommendation: `build` = `vite build` only; gates live in `check`/CI. Faster packaging, single responsibility.

- **BLD-07 — Rust code has never been rustfmt-formatted; no rustfmt/clippy config** (Medium, M).
  Evidence: `cargo fmt --manifest-path src-tauri/Cargo.toml --check` -> 4,234 `Diff in` hunks across 1,292 distinct file/line locations; integration tests visibly unformatted (`let mut child=Command::new(env!("CARGO"))`). No `rustfmt.toml` / `clippy.toml` in repo (only inside vendored wry). `.github/instructions/rust.instructions.md:62-65,135` says to use rustfmt/clippy — not enforced.
  Problem: large Rust files (stack_popup.rs 155 KB, git_status.rs 151 KB, appbar.rs 120 KB) are hard to review; formatting cannot be applied without breaking TST-01/02 regex tests (e.g. `stripRustTestBlocks` depends on brace layout), so the two problems are coupled.
  Recommendation: after TST triage, one-shot `cargo fmt` commit (add to `.git-blame-ignore-revs`), then `cargo fmt --check` in CI; add `[lints]` table in Cargo.toml and run clippy with a baseline (`-W clippy::all`, deny `await_holding_lock`, `unwrap_used` in selected modules).

- **BLD-08 — No JS/TS/Svelte linting or formatting** (Medium, S).
  Evidence: no eslint/prettier/editorconfig files; svelte-check is the only static gate. 155 `console.*` calls in `src/`.
  Recommendation: `eslint` flat config with `eslint-plugin-svelte` + `typescript-eslint` (rules: no-floating-promises — important with IPC `invoke`, no-unused-vars, no-console except warn/error), prettier with `prettier-plugin-svelte`. Start warn-only.

- **BLD-09 — No structured logging; diagnostics go to stdout/stderr** (Medium, M).
  Evidence: Cargo deps include no `log`, `tracing`, or `tauri-plugin-log`; `src-tauri/src` has 55 `eprintln!` and 29 `println!`; front end has 155 `console.*`, forwarded only in dev by `vite-plugin-terminal` (`vite.config.ts:11-17`, serve only). Ad-hoc env knobs (`JASONSHELL_TASKBAR_NATIVE_HOOK_LOG`, `JASONSHELL_SNIP_PROFILE`, ~25 `JASONSHELL_*` vars).
  Problem: once BLD-01 is fixed (GUI subsystem), all `eprintln!` output disappears in release; field debugging of a shell (hooks, AppBar, WebView2 crashes) has no persistent log.
  Recommendation: `tracing` + `tracing-appender` rolling file in `%LOCALAPPDATA%\JasonShell\logs`, `RUST_LOG`-style filter via one env var, forward front-end errors through `tauri-plugin-log`; document env vars in one place.

- **BLD-10 — Experiments ship in the main build graph** (Low, S).
  Evidence: `src/p03-experiment.ts`, `p03-experiment.html`, `vite.p03-experiment.config.ts`, `dist-p03-experiment/`, `dist-text-probe/`, `src/components/TextEditorProjectionExperiment.svelte`, Cargo feature `text-document-feasibility`, `src-tauri/examples/{p03-experiment,text-probe,stack_text_p03.rs,stack_text_probe.rs}`, bins `snip-feasibility.rs`, `snip-clipboard-owner.rs`, script `p03:experiment`.
  Problem: experiment code is compiled/type-checked with production (tsconfig includes `src/**`), increases surface for regex tests and maintenance.
  Recommendation: move to `experiments/` (top-level, own tsconfig) or delete if concluded; feature-gate bins.

- **BLD-11 — Front-end bundle: large chunks shared across all windows** (Low, S).
  Evidence: `dist/` 2.5 MB; largest chunks `stack-editor-core` 367 KB, `TerminalSearchCloseButton-*.js` 366 KB (xterm likely hoisted into a chunk named after a tiny button), `StackPopupSurface` 228 KB, `index` 169 KB. `vite.config.ts` manualChunks only for codemirror.
  Problem: chunk naming obscures what each surface loads; every WebView window pays parse cost of shared chunks (RAM matters for a multi-window shell, see runtime-ram effort).
  Recommendation: add `rollup-plugin-visualizer` report in CI artifacts; name an explicit `xterm` chunk; track per-surface JS budget.

### Repo hygiene findings

- **HYG-01 — 652 MB ONNX model and other large blobs in git history (local branches)** (High, S/M).
  Evidence: `git count-objects -vH` -> `size-pack: 463.82 MiB`. Largest historical blobs: `src-tauri/resources/speech-models/parakeet-tdt-0.6b-v2-int8/encoder-model.int8.onnx` 652,184,014 B and `decoder_joint-model.int8.onnx` 8,998,286 B (commit d6b76c6 "a lot of various changes", reachable only from local branches `bright-otter`, `lively-iguana`, `lunar-otter`; not in master/origin/master); `.fusion/fusion.db` 5.5 MB (on branches `bright-otter`, `disco-narwhal`, `docs/ui-consistency-audit`, ...), `crates/shell-native/tests/fixtures/settings_store_cutover_observed.json` 2.6 MB, `experiments/egui-bottom-bar/evidence/*.png` 2.1 MB.
  Problem: pushing any of those branches would exceed GitHub's 100 MB file limit / bloat the remote; local clone is ~464 MB packed for a ~20 MB working source tree.
  Recommendation: delete or rewrite those branches (check none are needed), `git gc --prune`; add a pre-push/CI check rejecting blobs > 5 MB; keep models out of repo (already gitignored at `/src-tauri/resources/speech-models/`).

- **HYG-02 — Third-party SDK with build outputs tracked under an ignored path** (Medium, S).
  Evidence: `.local/` is in `.gitignore` but 73 files are tracked: `.local/Everything-SDK/Everything-SDK.zip` (236 KB), `dll/Everything32/64/ARM/ARM64.dll`, C#/VB example projects incl. `obj/x86/Debug/*.resources`, `WindowsApplication1.suo`. Grep for `Everything-SDK` in src-tauri/scripts/README: no references (runtime uses `C:\Everything64.dll`-style detection, `search/providers/everything.rs`).
  Problem: unused vendored binaries (DLLs) of unverified provenance, IDE junk (`.suo`, `obj/`), plus confusing "ignored but tracked" state; `.local/` also hosts the generated rustc wrapper.
  Recommendation: `git rm -r --cached .local/Everything-SDK`; if the SDK header is needed, vendor only `include/Everything.h` + license under `third_party/` with provenance.

- **HYG-03 — Personal/tool junk tracked** (Medium, S).
  Evidence: `src-tauri/logs/webhelper.txt` (3.9 KB Steam `steamwebhelper` log with local Program Files paths — unrelated to the project); `result.txt` (0 bytes, tracked though gitignored); `smoke-results/*` (4 files, not ignored); `.code/agents/*/error.txt` (27 agent run error files); `.openchamber/screenshots/*.jpg` (16 files, ~1.3 MB); `.codex` (0-byte); `.zed/settings.json` (empty `{}` template).
  Recommendation: `git rm --cached` all of these; add `.code/`, `.openchamber/`, `smoke-results/`, `src-tauri/logs/`, `.zed/`, `.codex` to `.gitignore`.

- **HYG-04 — Two lockfiles; bun.lock is stale** (Medium, S).
  Evidence: `bun.lock` (34 KB, last commit 2026-05-12) and `package-lock.json` (135 KB, 2026-11-16). `bun.lock` contains 0 matches for `vitest|jsdom|codemirror`; package-lock 60. README:81 instructs `npm ci` via bootstrap.
  Problem: anyone using bun gets a materially different dependency set (no test deps); Dependabot/tools may pick the wrong lockfile.
  Recommendation: delete `bun.lock`, add `"packageManager": "npm@x"` and `engines`.

- **HYG-05 — Root markdown sprawl: 25 tracked .md files, ~1.0 MB** (Medium, S).
  Evidence (name, bytes, last commit): `AGENTS.md` 2,319 (2026-11-12); `CHANGELOG_POLICY.md` 2,345; `CONTINUITY.md` 2,846 (2026-05-12); `README.md` 31,115; `TEST_CASE_AUDIT.md` 281,719 (2026-09-13); `TEST_COVERAGE_GAP_AUDIT.md` 25,993; `backend_blocking_lock_boundaries_p6.md` 5,266 (2026-05-12); `changelog.md` 392,547; `context.md` 7,205 (2026-08-11); `contract_test_spec_hygiene_p7.md` 7,414 (05-12); `findings.md` 26,703 (05-12); `fullscreen-blank-oracle.md` 3,219; `fullscreen-blank-scout.md` 7,375; `master_spec.md` 79,996; `persistent_surface_lifecycle_cleanup_p4.md`; `plan.md` 26,534 (06-30); `shell_event_windows_key_smoke_p2_p3.md`; `shell_open_close_event_correctness_p2.md`; `shell_popup_layout_scroll_p5.md`; `shell_surface_contract_parity_p1.md`; `stack_browser.md`; `stack_browser_terminal_overhaul_plan.md` 29,770; `terminal_fix_plan.md` 16,505; `terminal_plan.md` 49,067; `windows_key_chord_preservation_p3.md`. Plus `docs/` 100 files / 2.7 MB (incl. `docs/archive/master_spec_full_legacy_2026-09-23.md` 391 KB, `stack-browser-quick-view-editor-implementation-plan.md` 220 KB) and 20 md in `tests/`.
  Problem: 19 of 25 root md files are historical plans/audits (owner considers them outdated). Agents and humans must guess which doc is authoritative; search noise; changelog at 392 KB is unreadable.
  Recommendation: keep root to README, AGENTS.md, CHANGELOG(.md), CHANGELOG_POLICY, master_spec (or move spec to docs/). `git mv` the rest into `docs/archive/YYYY-MM/`. Split changelog by year (`docs/changelog/2026.md`). Note TST-04: doc-asserting tests must be relaxed first.

- **HYG-06 — Agent-instruction duplication and generic instruction bloat** (Medium, S).
  Evidence: instruction sources: `AGENTS.md`; `.github/instructions/*.md` — 11 files + `.github/prompts/unit-test-generation.prompt.md`, 169,748 bytes total, 9 with `applyTo: '**'`/`'*'`, 0 mentioning JasonShell or Tauri (generic copies: a11y, OWASP, memory-bank, spec-driven-workflow, github-actions best practices, ...); `skills/imported/clawteam-engineering/clawteam/SKILL.md`; `.rust-skills` git submodule (actionbook/rust-skills, ~1.1 MB incl. `unsafe-checker` rules); `codex-install.sh` at root; `.code/agents/`, `.agent-checkpoints/`, `.fusion/`, `.serena/`, `.slim/` tool state dirs.
  Problem: ~170 KB of generic always-applied instructions inject into every Copilot request; conflicting guidance vs AGENTS.md (e.g. memory-bank workflow vs AGENTS intake order); the submodule requires `git submodule update` for nothing the build uses.
  Recommendation: one canonical AGENTS.md (+ CLAUDE.md symlink/pointer if desired); remove generic `.github/instructions` or narrow `applyTo`; drop the `.rust-skills` submodule (install per-user instead); move tool state dirs to `.gitignore`.

- **HYG-07 — `codex-install.sh` is an unrelated, mis-pathed script** (Low, S).
  Evidence: root `codex-install.sh` header: "Codex Installation Script for Claude Skills Library ... Usage: ./scripts/codex-install.sh [--all | --category <name>]", examples `--category marketing`, `--skill content-creator`; installs into `$HOME/.codex/skills`. It lives at repo root, not `scripts/`.
  Problem: copied from a different repo; not related to building or running JasonShell; bash-only in a Windows-first repo.
  Recommendation: delete (or move to a personal dotfiles repo).

- **HYG-08 — Branch and worktree sprawl** (Low, S).
  Evidence: 70 local branches incl. `test`, `test-barnch`, `testtttttt`, random names (`agile-otter`, `groovy-ferret`, ...), many `*-checkpoint`; stale local `main` (161 behind master; CI targets it, CI-01); 8 worktrees (`git worktree list`); 14 stashes. Remote has only `master`, `git-upgrade`, `fusion/fn-002`.
  Recommendation: prune merged branches (`git branch --merged master`), remove stale worktrees, delete `main` or make it the real default.

- **HYG-09 — `.gitignore` gaps and contradictions** (Low, S).
  Evidence: missing: `smoke-results/`, `src-tauri/logs/`, `src-tauri/test-results/` (exists locally), `.code/`, `.openchamber/`, `.serena/` (listed at top of file without comment), `.zed/`, `/.local/` already listed but tracked; odd entries `NUL`, `target-pi-check/*`, `.harness/**`. `.ignore` re-includes `.slim/deepwork/` for ripgrep.
  Recommendation: reorganize by section, add missing dirs, then `git rm --cached` tracked-but-ignored files (`git ls-files -ci --exclude-standard`).

- **HYG-10 — Dirty-worktree churn from generated files** (Low, S).
  Evidence: current `git status` lists 5 modified `src-tauri/permissions/autogenerated/*.toml` and 2 untracked new ones alongside real changes. Same root cause as BLD-04.
  Recommendation: see BLD-04.

### Developer experience findings

- **DX-01 — Bootstrap is good; but onboarding depends on a personal toolchain quirk** (Medium, S).
  Evidence: `scripts/bootstrap-windows.ps1` checks commands, finds VS Build Tools via vswhere/VsDevCmd, runs `npm ci`, launches dev (`-SkipInstall`, `-SkipDevLaunch` switches) — solid. But README:305 explains Blaze/ra-jit wrapper; CI-05.
  Recommendation: keep bootstrap; add `-Check` mode that only verifies prerequisites (WebView2 runtime, MSVC, Rust target, Node version), and make the wrapper opt-in.

- **DX-02 — npm script surface is confusing** (Low, S).
  Evidence: `test:search` is an alias of `test:node`; `precargo:check`/`precargo:test`/`pretauri` listed as user commands in README:325-332; `smoke:runtime` uses `pwsh` while `smoke:fullscreen` uses `powershell`; `smoke:runtime` hard-codes `-DryRun`; `p03:experiment` in main scripts; no `test` script (so `npm test` fails), no `test:browser`, `test:ram`, `test:native`, `lint`, `fmt`.
  Recommendation: add `npm test` = fast lanes (node + component); remove aliases; namespace experiments; document lanes in a single table.

- **DX-03 — `validate` is serial and slow by construction** (Low, S).
  Evidence: `scripts/run-validation.mjs:7` runs check, build, test:component, test:node, cargo:test, cargo:check sequentially, no fail-fast option, no timing output.
  Recommendation: run JS lanes in parallel with cargo; print per-stage duration; `--fast` flag (skip build/cargo check). The script already accepts injectable `spawn/stages`, so this is cheap.

- **DX-04 — Dev loop: single Vite server for many WebView windows is fine, but no per-surface dev entry** (Low, M).
  Evidence: `vite.config.ts` serves `localhost:1420` with HMR on 1421; `tauri.conf.json` `"windows": []` (windows built in Rust, `shell_windows.rs`), surfaces loaded via `src/lib/surfaceLoader.ts`. Browser fixtures already render surfaces standalone with `mockIPC`.
  Problem: iterating on one surface requires launching the full shell (AppBar reservation, Windows key hook, taskbar replacement) — disruptive to the developer's own desktop.
  Recommendation: promote the `tests/browser` fixture pattern into a `npm run dev:surface -- stack-popup` playground (Vite + typed mocks), or Storybook-lite. This also becomes the base of the Playwright suite (TST-09).

- **DX-05 — No documented debugging story for Rust + WebView2** (Low, S).
  Evidence: no `.vscode/launch.json` (`.vscode/` ignored), no README section on attaching a debugger, enabling WebView2 devtools in release, or reading logs (BLD-09).
  Recommendation: short `docs/dev/debugging.md`: CodeLLDB/cppvsdbg launch config, `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` for inspecting live windows, log locations, the `JASONSHELL_*` env var table.

- **DX-06 — Env-var feature flags are undiscoverable** (Low, S).
  Evidence: ~25 distinct `JASONSHELL_*` variables across src-tauri/src, tests, scripts (e.g. `JASONSHELL_TASKBAR_NATIVE_HOOKS`, `JASONSHELL_EXPLORER_SUPPRESSION_V*`, `JASONSHELL_POWERSHELL_STARTUP`, `JASONSHELL_ARCHIVE_EXTRACTION_TIMEOUT_SECS`, `JASONSHELL_FOLDER_DRAG_TYPE`).
  Recommendation: central `env.rs` with documented constants; generate the table into README.

- **DX-07 — Huge single files impede review and test design** (Medium, L; overlaps other auditors).
  Evidence (tracked bytes): `src/components/StackPopupSurface.svelte` 155,348; `src-tauri/src/stack_popup.rs` 155,262; `stack_popup/git_status.rs` 151,553; `StackGitPanel.svelte` 133,370; `appbar.rs` 120,403; `TerminalPanelSurface.svelte` 115,343; `quick_commands.rs` 114,144; `speech_runtime.rs` 110,076.
  Problem: from the test angle, these files are the main reason tests regex source: logic embedded in 150 KB components is not importable, so authors grep instead.
  Recommendation: extract pure logic into `src/lib/*.ts` (already the pattern for `stackPopupViewModel.ts`) and Rust submodules, then unit-test those directly.

---

### Big bets

1. **Real E2E harness against the running app over WebView2 CDP.** WebView2 honors `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<n>` (already used for proxy args in `scripts/stack-text-editor/run-native.ps1:36-40`). Launch a debug build with a throwaway profile/settings dir and a "no desktop takeover" mode (skip AppBar reservation, Windows key hook, Explorer suppression behind an env flag), then drive each window with Playwright `chromium.connectOverCDP` — every Tauri window is a CDP target addressable by URL/label. This tests real IPC, real Rust, real serialization — the class of bug TST-12 shows is uncovered. Alternative: `tauri-driver` + `msedgedriver` (official Tauri WebDriver path on Windows) if WebDriver semantics are preferred. Start with 5 golden journeys (open Stack Browser and navigate, run a quick command, search, open terminal, settings save/reload). Run locally on demand and on a self-hosted or `windows-latest` runner (desktop session available on hosted Windows runners) nightly.

2. **Contract codegen instead of contract regex.** Adopt `tauri-specta` (commands + events) or `ts-rs` (types) to generate `src/ipc/generated.ts`; CI fails on uncommitted diff. Replace dozens of "command name registered in Rust and TS" regex tests with the compiler. Build typed mock helpers for vitest/Playwright from the same bindings.

3. **Test rewrite strategy (strangler, not big-bang).** (a) Week 1: freeze new source-regex tests; shared helpers with throwing reads (TST-03/06). (b) Tag every existing Node test file `invariant | behavior-candidate | obsolete` (a spreadsheet of 188 rows; `TEST_CASE_AUDIT.md` may already hold a partial inventory). (c) Delete obsolete and doc-wording tests (TST-04/05/10). (d) For behavior-candidates, extract logic to `src/lib` / Rust modules and write behavior tests, deleting the regex test in the same PR. (e) Convert remaining invariants to structured checks (JSON parse of capabilities, `syn` for Rust, TS compiler API for TS). Track "regex assertions remaining" as a CI-printed metric aiming for < 10% in a quarter. Then apply rustfmt + prettier once.

4. **A working CI pipeline (and matrix).** Fix trigger to `master`, make it green, then two workflows: `ci.yml` (PR + master: fmt, lint, typecheck, node, component, browser/Playwright with mocks, cargo test with rust-cache; ~15 min target) and `nightly.yml` (cargo test `--ignored` lanes that do not need real input, runtime-ram vitest, bundle size report, coverage, `cargo deny`, real-app CDP smoke). Matrix: windows-latest only for runtime; Rust stable pinned + `beta` allowed-to-fail to preview lint breakage. Add `release.yml` on tags building a signed NSIS installer (BLD-02).

5. **Split a library crate.** `src-tauri/src/lib.rs` (core logic: stack ops, git status, search providers, settings, snipping geometry) + thin `main.rs` (Tauri wiring). Integration tests `use jason_shell::...` instead of `#[path]` includes and nested cargo; enables benches, `cargo llvm-cov`, faster incremental test builds.

6. **Repo diet.** One cleanup PR: remove tracked junk (HYG-02/03/04/07), archive root plans (HYG-05), drop generic instructions/submodule (HYG-06), purge large-blob local branches (HYG-01), add size guard. Expected: root from ~45 entries to ~15, clone from ~464 MB to the size of real history.

### Positive observations

- `npm run test:node`: 1,449 tests in ~14 s, 0 failures — a fast lane worth keeping.
- Component tests use real Svelte rendering with Testing Library and a centralized Tauri mock in `tests/components/setup.ts`; jsdom gaps (ResizeObserver, matchMedia) explicitly stubbed.
- `scripts/run-validation.mjs` is small, testable (injectable spawn) and runs all stages without aborting on first failure.
- `scripts/run-component-tests.mjs` documents and fixes a real Windows drive-letter ESM cache duplication issue.
- Vendored wry has exemplary provenance (`vendor/wry/JASONSHELL_PATCH.md`: base hash, upstream commit, license notes).
- Native/consent-gated tests are explicitly opt-in via env flags rather than silently disrupting the desktop.
- `bootstrap-windows.ps1` handles VS Build Tools discovery and uses `npm ci`.
