# Git refresh acceptance evidence

## Scope and contracts

Test/evidence owner only. No live Tauri or Windows shell launch, new dependency, polling policy or persistence.

- `get_stack_git_context({ path })` → `{ repositoryRoot, gitDir, canonicalPath } | null`; all three fields strings, filesystem/Git verified, not lexical ancestry.
- `get_stack_git_summary({ path })` → `StackGitStatus | null`, porcelain counts and badge paths only; no line-stat, remote-URL or ahead/behind enrichment.
- `get_stack_git_metadata({ path })` → `{ repositoryRoot, gitDir, remoteRepositoryUrl, ahead, behind } | null`; fetched after summary, stale identity/revision guarded.
- `get_stack_git_status` remains the full workbench contract.

Native synchronous acceptance functions: `stack_git_context_for_path`, `stack_git_summary_for_path`, `stack_git_metadata_for_path`.

## Pre-production baseline

Measured this checkout through the actual existing `stack_git_status_for_path` implementation in an opt-in cfg(test) module. First measured call **945.862 ms**, then warm calls **927.993, 878.845, 930.149, 871.398, 876.975 ms** (warm median **878.845 ms**). All returned 217 entries. This is developer-debug native collection latency, not IPC/render latency; no cold filesystem-cache claim. Test artifacts already existed during the run.

Git TRACE2 observed **7 direct Git invocations per collection**: `rev-parse --show-toplevel`, `branch --show-current`, porcelain status, cached numstat diff, unstaged numstat diff, origin URL config, upstream rev-list. Four are enrichment subprocesses (two diffs/config/rev-list). Git also launched **2 child status invocations per collection**, giving **9 actual Git processes per call** on this checkout, plus one process-wide `git.exe --exec-path` discovery. The source additionally performs bounded untracked file-line enrichment; no claim that TRACE2 measures filesystem reads.

Trace: `C:/Users/jnev1/AppData/Local/Temp/opencode/stack-git-baseline-trace.jsonl`. Native run output: `C:/Users/jnev1/.local/share/opencode/shell/292752bc557805ae27bf02504f17f794171f367b/sh_1ce4ee36b001ZsxzeMJoRzA9X5.out`.

Reproduce read-only baseline (delete/choose a new trace output before repeating):

```powershell
$env:GIT_TRACE2_EVENT='C:/Users/jnev1/AppData/Local/Temp/opencode/stack-git-baseline-trace.jsonl'
npm run cargo:test -- measure_existing_initial_status -- --ignored --nocapture --test-threads=1
Remove-Item Env:GIT_TRACE2_EVENT
```

## RED evidence before implementation

- Focused component suite initially 13 failing / 1 passing, final expanded suite 26 failing / 2 passing (28 cases). Exact failures include no lightweight-summary IPC, branch DOM replaced during same-repo child navigation, absent context boundary requests, missing explicit Refresh/editor-save/file mutation invalidation. One negative-stage error matcher was subsequently broadened to match the existing prefixed diagnostic. Focused log: approved temp `stack-git-refresh-red-component.log`.
- Focused Node contract suite: 19 passing, 1 expected RED for the new separate IPC contract.
- Native acceptance compile RED: E0425 for the three not-yet-existing native functions, 10 missing-function errors. Existing native baseline compiled and passed beforehand.
- Production-mounted browser fixture ran via agent-browser on port 4187. `#result`: `FAIL: Error: Initial lightweight summary call count 1`; invocation ledger instead recorded `get_stack_git_status({path:'C:\\fixture\\repo'})`. Fixture exposes controlled delayed IPC and the complete ledger at `window.stackGitRefreshEvidence`.
- Replaced old per-folder/full-status source-policy test, retaining the full workbench API assertions.

## Evidence path after production changes

1. Run focused component suite and native `refresh_acceptance`; resolve behavior failures, not by weakening acceptance.
2. Run browser fixture documented in `tests/browser/stack-git-refresh/README.md`; save `#result`, checks, invocation ledger and screenshot under approved temp.
3. Repeat native latency and TRACE2 measurement for the new summary path, separately reporting context/summary/metadata work.
4. Required gates owned by test runner: `npm run test:component`, `npm run test:node`, `npm run cargo:test`, `npm run check`, `npm run build`, `npm run cargo:check`.

Mocked browser/component IPC proves renderer behavior, not canonical filesystem resolution. Native temporary Git repositories, submodule, linked worktree and Windows junction cases own that boundary. Live shell smoke remains unauthorized.

## Full Node baseline comparison (2026-11-12, implementation in progress)

Ran the exact `npm run test:node` script in the changed workspace and in a separately extracted **tracked HEAD `a3d1a22ab9b80a32e2ba5339125bdd571cfb0a11`**. Current result: **1,287 tests, 1,268 pass, 13 fail, 3 skipped, 3 todo**, exit 1. Clean tracked HEAD result: **1,287 tests, 1,269 pass, 12 fail, 3 skipped, 3 todo**, exit 1. Neither run cancelled tests. These are fresh observations; the historical README count of ten unrelated failures does not describe this HEAD.

Baseline preparation used read-only `git archive HEAD` into `C:/Users/jnev1/AppData/Local/Temp/opencode/stack-git-refresh-node-head-a3d1a22ab9b8`, then extracted the tar there. Only `node_modules` was junction-linked to the existing checkout dependencies. No dependency install, commit, checkout, reset, revert, new worktree or production edit was performed. Both runs used Node **v24.14.0**, npm **11.9.0** and the exact package script (`clean-dist-tests`, TypeScript compilation, all `tests/*.test.mjs`). Generated `dist-tests` was local to each source root.

All **12 HEAD failures also failed in the changed workspace**, with matching test declaration, normalized assertion error and assertion location. Exact names and locations:

- `tests/changelogPolicy.test.mjs:104`: **README validation scripts match package scripts without stale copies** — assertion at 114; documented array omits `precargo:check`, `precargo:test`, `pretauri` in both runs.
- `tests/contextMenuSpeechAcceptance.test.mjs:8`: **shared app-owned menu provides rounded glass motion, icons, and disabled/destructive items** — assertion at 19; missing backdrop-filter contract.
- `tests/contextMenuVisualRegression.test.mjs:47`: **shared menu uses a stable translucent theme backplate throughout opening animation** — assertion at 50; equality mismatch.
- `tests/contextMenuVisualRegression.test.mjs:71`: **shared menu uses the established 8px backdrop blur without vendor override** — assertion at 73; missing `backdrop-filter: blur(8px)` contract.
- `tests/frontendUiPolicy.test.mjs:30`: **app and component UI sources do not use active gradients** — assertion at 39; `src/components/BottomBar.css` in both source roots.
- `tests/meltMigrationWiring.test.mjs:200`: **bottom-bar command buttons use Melt-backed action buttons without changing taskbar semantics** — assertion at 236; missing exact task-label span contract.
- `tests/persistentTerminalPanel.test.mjs:227`: **material symbol icon set keeps official 960-grid paths and equal inline sizing** — assertion at 228; deep-equality mismatch.
- `tests/processManagerCloseButton.test.mjs:30`: **process manager has an accessible token-styled close button with header and focus affordances** — assertion at 44; missing `position: relative` contract.
- `tests/sourceContractIntentRegistry.test.mjs:149`: **source-text tests have explicit intent tags in the registry** — assertion at 152; unregistered `taskPreviewBaseline.test.mjs` in both runs, not the Git-refresh artifacts.
- `tests/speechIndicator.test.mjs:49`: **mic shell stays a fixed opaque 40px circle in quiet and speaking states** — assertion at 56; missing exact `width: 40px` contract.
- `tests/taskbarAttentionModel.test.mjs:32`: **nativeHook printing and flush are gated without gating signal processing or reconciliation** — assertion at 34; `handle_event must remain present` in both runs.
- `tests/topBarMicControl.test.mjs:276`: **mic hover matches adjacent control and successful paste check overlays visible mic glyph** — assertion at 281; actual `transparent`, expected `var(--js-color-accent-border)` in both runs.

The **only changed-workspace-only failure** was the intended new contract at `tests/stackPopupGitStatus.test.mjs:217`: **browser summary has separate typed context, summary and deferred metadata IPC while preserving full workbench API**. At this run it failed the `export function getStackGitContext(` assertion while implementation was still underway. This is in-scope RED, not an unrelated baseline exemption.

Artifacts under `C:/Users/jnev1/AppData/Local/Temp/opencode/`:

- `stack-git-refresh-current-node-full.log`: full changed-workspace output.
- `stack-git-refresh-head-node-full.log`: full tracked-HEAD output.
- `stack-git-refresh-node-baseline-comparison.json`: twelve matching failure signatures and the single current-only name; source-root paths normalized for comparison.

Limits: this establishes existing source-contract failures, not that the underlying UI behavior is correct. The current workspace had concurrent implementation/documentation owners; this run is not the final post-implementation gate and must be repeated once their work settles. The archive has no `.git` metadata and uses the existing installed dependencies, not a fresh dependency installation. No unrelated failure was repaired or suppressed. Cargo, component and browser gates were deliberately not run for this bounded comparison. Overall E2E remains required and deferred. Requested runner instructions `C:/Users/jnev1/.config/opencode/agents/e2e-runner.md` were explicitly checked and unavailable.

## Added renderer ownership edges (pending next component gate)

The focused renderer acceptance suite includes three additional delayed-IPC cases: failed typed-path validation must preserve the original pending Git context; destination rows cannot inherit old badges before verified context even when their lexical paths match an existing snapshot entry; a failed same-folder explicit Refresh cannot silently retry when its delayed context resolves. A further case verifies that a successful commit still invalidates the summary when its follow-up push fails. All **32 refresh cases** passed in the final full component gate (**520/520**); the UI owner independently reported the preceding 31/31 focused cases passing.

## Native post-implementation acceptance

`npm run cargo:test -- refresh_acceptance -- --nocapture --test-threads=1` passed **4 acceptance tests**, zero failures, with the opt-in benchmark ignored. The junction fixture previously failed in setup before any production context call because `mklink` rejected mixed path separators. Test-only setup now joins `Temp` and `opencode` separately and normalizes both `mklink` arguments to native Windows separators. The test created a real junction and production context resolved its target; both original canonical root/path equality assertions remain unchanged.

Passing cases verify nested repositories/children/nonrepositories, distinct linked-worktree and submodule Git identity, a junction escaping lexical ancestry, and summary/full/deferred-metadata boundaries. Instrumented summary executes exactly one status scan, no diff/numstat/config/rev-list commands and zero untracked-line-enrichment calls. Full workbench still executes diff and untracked-line enrichment; metadata executes no status/diff/numstat. This is actual native Git/filesystem evidence in temporary fixtures, not mocked IPC. Log: approved temp `stack-git-refresh-native-acceptance.log`.

## Post-implementation latency and actual Git processes

Six debug-native samples per lane, same checkout, separate TRACE2 files and test-only direct-command/untracked-enrichment counters. First sample / five warm samples / warm median in milliseconds:

- **Context only:** 105.805 / 103.669, 103.861, 103.138, 103.778, 102.363 / **103.669**. One direct `rev-parse --show-toplevel --absolute-git-dir`, zero child Git processes per call.
- **Summary collector only:** 421.555 / 413.729, 410.856, 359.510, 360.618, 361.194 / **361.194**. Three direct commands (canonical context, branch, one porcelain status) plus one child Git process per call; **four actual Git processes**, zero diff/numstat/config/rev-list and zero untracked-line-enrichment calls.
- **Initial critical native work, context then summary:** 520.849 / 513.641, 461.164, 512.337, 462.637, 464.142 / **464.142**. Four direct plus one child Git process per call; **five actual Git processes**, zero enrichment.
- **Deferred metadata only:** 419.502 / 410.644, 409.449, 410.677, 410.616, 412.137 / **410.644**. Four direct commands (canonical context, branch, origin URL config, upstream rev-list), zero child Git processes per call. Two are enrichment commands; no status/diff/untracked-line enrichment.
- **Unchanged full-workbench control after implementation:** 995.908 / 977.327, 981.120, 1032.608, 924.945, 981.775 / **981.120**. Seven direct plus two child Git processes per call; **nine actual Git processes**, four direct enrichment commands and eleven untracked-line-enrichment visits.

TRACE2 totals across six calls: context 6 starts, summary 24, initial 30, metadata 24, full 54, matching the instrumented direct counts plus Git children. No discovery commands occurred in these post-run trace files. The earlier baseline trace had one separate `--exec-path` event outside the seven-per-collection direct-command set; its precise originating setup step is not attributed here.

The native context→summary critical path's measured warm median **464.142 ms** is below the pre-production full collector's **878.845 ms**; first measured native work **520.849 ms** versus **945.862 ms**. These figures exclude folder listing, IPC transport, WebView scheduling and paint. Metadata remains separate later work: this is removal of enrichment from the initial critical path and no repeated status/enrichment on same-repo navigation, **not** elimination of all subsequent Git subprocesses. Initial context+summary plus deferred metadata totals nine Git processes on this checkout; ordinary same-repo navigation performs only one context probe, not another status/metadata scan.

Limits: no cold-cache claim, no controlled whole-machine-load experiment, no release/IPC/render-latency guarantee. Post-run summary/full responses contained **237 entries**, versus 217 at the earlier baseline, because the worktree/test/generated-permission state changed during implementation. Warm measurements are repeated local observations, not a universal speedup. The untracked counter records calls to line-enrichment code, not low-level filesystem read counts.

Reproduce one native-only lane without launching the app (choose a fresh trace output):

```powershell
$env:JASONSHELL_GIT_BENCH_LANE='summary' # context, summary, initial, metadata, full
$env:GIT_TRACE2_EVENT='C:/Users/jnev1/AppData/Local/Temp/opencode/stack-git-refresh-post-20261112-summary-trace.jsonl'
npm run cargo:test -- measure_split_checkout_lane -- --ignored --nocapture --test-threads=1
Remove-Item Env:GIT_TRACE2_EVENT
Remove-Item Env:JASONSHELL_GIT_BENCH_LANE
```

Raw lane logs/traces: approved temp `stack-git-refresh-post-20261112-{context,summary,initial,metadata,full}.log` and corresponding `-trace.jsonl`. Parsed samples/counts: `stack-git-refresh-post-measurements.json`.

## Pre-review renderer/browser and automated gate checkpoint

- `npm run test:component`: **27 files, 520 tests passed**, including all 32 refresh cases, existing mouse navigation and workbench tests. Existing jsdom canvas-unimplemented diagnostics remain nonfatal; no dependency added.
- `npm run check`: **zero errors, zero warnings**.
- `npm run build`: **passed**.
- `npm run cargo:check`: **passed**.
- `npm run cargo:test`: **passed after building the existing example frontend assets**, **1,184 passed, zero failed, 32 ignored** across 28 test binaries. The ignored set includes existing opt-in/native/hardware cases and the two runner-owned opt-in benchmarks; ignored cases are not counted as validation passes.
- `npm run test:node`: **1,287 tests; 1,269 pass, 12 fail, 3 skipped, 3 todo**, exit 1. Final failure names exactly equal the twelve clean-HEAD failures documented above; **zero additional failures**. Full comparison artifact: `stack-git-refresh-final-node-baseline-comparison.json`. No unrelated failure was suppressed.

Three source-contract assertions were updated only for intentional equivalent/new behavior: wrappers may be `export async function` rather than plain `export function`; row badge lookup must receive the verified `canonicalPath` fifth argument; inline status hover text must preserve file error → Git error → normal status precedence. These are not exemptions from runtime acceptance: the full rendered suite separately verifies async IPC behavior, unverified-row badge exclusion, canonical-context identity and retained/error snapshots.

The exact `npm run cargo:test` initially failed **before test execution** because existing `stack_text_p03`/`stack_text_probe` example frontend assets were absent. With explicit approval, generated ignored assets only were built from their unchanged scoped configs:

```powershell
node node_modules/vite/bin/vite.js build --config vite.p03-experiment.config.ts --outDir dist-p03-experiment --emptyOutDir
node node_modules/vite/bin/vite.js build --config scripts/stack-text-editor/vite.config.mjs
```

Both builds passed. No `cargo run`, example host, Tauri app, dependency installation or production configuration edit was used. The exact full `npm run cargo:test` rerun then passed (1,184 tests); this was not a substituted `--bins --tests` run. `git check-ignore` confirmed both generated example outputs are ignored.

Production-mounted browser E2E ran through agent-browser at **http://127.0.0.1:4187/** and finished **PASS, 16 assertions**. It uses production popup, global CSS and actual CodeMirror editor, not a copied view model. Initial branch painted while metadata was unresolved; summary invocation preceded metadata. Delayed same-repo context preserved the exact branch DOM node, destination rows stayed unbadged until verified, and Back/Forward retained that same node with summary=1 and metadata=1. Refresh failure kept the known snapshot and delayed context completion did not retry it. Actual browser draft insertion and Save produced exactly one save IPC and one summary invalidation without folder navigation. Nested repository entry accepted its own snapshot; the old metadata completion attached neither an old branch nor an old remote link.

Frozen browser ledger: **47 calls** total, including 6 context probes, 6 folder reads, 4 summary requests (initial, explicit failed Refresh, successful editor save, nested repository), 3 metadata requests (initial, editor save, nested repository), **zero full-workbench status requests during browsing**, and one successful save. The same-repo navigation segment alone kept summary=1/metadata=1; later count increases were explicit invalidations or repository change. Existing popup-request reconciliation/icon calls are recorded rather than hidden. A fixture selector initially selected the path `<form>` instead of its `<input>`; it was corrected without a production change, then the complete strengthened journey passed.

Browser artifacts in approved temp:

- `stack-git-refresh-browser-result.json`: frozen exact checks and IPC argument ledger.
- `stack-git-refresh-browser-final.png`: final production-mounted nested-repository screenshot, visually inspected.
- `stack-git-refresh-browser-accessibility.txt`: final interactive accessibility snapshot.

Reproduce via `node node_modules/vite/bin/vite.js --config tests/browser/stack-git-refresh/vite.config.ts`, then agent-browser open the local URL, click **Run Git refresh journey**, inspect `#result` and `window.stackGitRefreshEvidence`. Core agent-browser guidance was loaded before this run. The test fixture's scoped Vite build also passed into ignored `node_modules/.cache/stack-git-refresh-browser`; its existing large-chunk warning is nonfatal and test-only. Full gate logs use the approved temp prefix `stack-git-refresh-final-{component,node,check,build,cargo-test,cargo-check}.log`; example build logs use `stack-git-refresh-example-{p03,probe}-build.log`; fixture build log is `stack-git-refresh-browser-fixture-build.log`.

Browser evidence verifies production-mounted renderer behavior with controlled mocked IPC; native fixture evidence verifies actual Git/canonical filesystem behavior separately. Neither certifies live WebView2/Tauri event delivery or Windows shell behavior. No live shell launch was authorized or performed. Independent review remains the parent session's gate.

## Independent-review regressions: RED before fixes

The preceding final-gate figures are the **pre-review** checkpoint, not completion of the two subsequently identified P2 defects. Added regressions without editing production behavior:

- **Paste success versus listing failure, toolbar and drop:** native `paste_stack_items` returns a nonempty `pasted: StackItem[]`, then `read_stack_folder` rejects. Acceptance requires a second summary request despite the listing error. Both initial component cases failed at their diagnostic query before reaching that count assertion; the separately executed browser RED directly confirmed missing success invalidation. After the fix, the component cases also navigate within the same repository and require no third summary scan.
- **All-failed paste, toolbar and drop:** native resolves `{ pasted: [], failures: [...] }`. Acceptance requires the original summary count **one**. Both initial component cases failed at their diagnostic query before reaching the count assertion. The earlier runner report claiming an observed count of two was an incorrect inference from source, not executed-assertion evidence, and is retracted. The positive paste fixture was made faithful to the actual native result shape, with a real `StackItem` in `pasted`, rather than a result with no success field.
- **Metadata failure versus absence, native:** a real temporary repository has a known origin URL and a valid local upstream. The established thread-local command hook injects one selected `config` or `rev-list` failure and verifies that production reached/consumed it. Separate cases inject nonzero failure and timeout for each probe. All **four fail**: expected `Err` preserving the diagnostic, observed successful `Some(StackGitMetadata)` with nullable fields. A separate real-repository test confirms legitimate no-remote/no-upstream remains successful nullable metadata; it passes.
- **Metadata error retention, renderer:** establish the known remote link, Refresh, then reject `get_stack_git_metadata`. The component preserves the previous remote-link title and exposes the probe/retry diagnostic; this case **already passes**. It establishes that native must propagate operational failure, rather than convert it to a successful null response that the renderer treats as new metadata.

Focused RED results: **37 component cases: 33 passed, 4 failed**; native `refresh_acceptance`: **5 passed, 4 failed, 1 opt-in benchmark ignored**. The existing command recorder now returns a test-only `Result`, with its existing cfg(test) call site using `?`; this supports thread-local injected `GitCommandError::NonZero`/`Timeout` without changing non-test production execution or global environment. Logs: approved temp `stack-git-refresh-review-red-component.log` and `stack-git-refresh-review-red-native.log`.

**RED-report correction:** the diagnostic queries were present in the initial tests, not added afterward. Archived component output shows all four failures were `Unable to find` the post-listing/all-failed error messages. The runner incorrectly described later unexecuted count assertions as observed failures. Source inspection found the existing operation-status formatter omitted `operation.error`, while its active-operation branch hid the normal error text. Primary count assertions were subsequently moved before the unchanged diagnostic assertions to establish mutation-boundary behavior independently; the all-failed case also waits for the follow-up folder-read boundary before checking its count. No diagnostic assertion was weakened or removed. The parent authorized the bounded error-visibility fix, implemented by the production owner.

Production handoff: invalidate from the successful native paste-result boundary **before** the fallible listing read, only when `pasted` is nonempty; preserve captured repository ownership for both toolbar and drop. The IPC `paste_stack_items` result remains `{ pasted: StackItem[], failures?: StackPasteFailure[] }`; no native payload/schema change is required. An optional success callback on the frontend wrapper is one possible implementation, not an acceptance requirement. `get_stack_git_metadata` must reject operational config/upstream errors/timeouts while accepting legitimate missing remote/upstream as null fields; full-workbench behavior must remain compatible.

**Focused browser rerun required**, because paste success and folder-read failure cross the actual mounted UI/API-wrapper boundary. Added a separate **Run paste review regression** button with a frozen `window.stackGitRefreshReviewEvidence` ledger. Its RED run reported `FAIL: Error: Successful paste followed by listing rejection did not invalidate summary`; ledger confirms the native paste and subsequent folder read, but no second summary. Artifact: approved temp `stack-git-refresh-review-red-browser.json`. The final rerun must cover this paste/listing failure, the all-failed no-invalidation case, and known-metadata retention/diagnostic, **plus rerun the original 16-assertion delayed-navigation journey**. The metadata-error browser case remounts the popup after paste cases (without clearing its ledger), separating it from the existing persistent file-operation status. No live shell, new dependency, polling or persistence was added. At this RED checkpoint, GREEN gates and parent re-review were pending the production fixes.

## Post-review GREEN validation

The production owners completed the paste-success callback, paste-only stored-error visibility, and strict deferred-metadata probe handling. Focused refresh acceptance is **37/37 passing**, now actually executing the primary count assertions **before** the unchanged diagnostic queries: toolbar/drop success plus listing rejection gives **summary=2 with visible failure**, while toolbar/drop all-failed native results keep **summary=1 with visible failure**. The tests then require same-repository navigation not to issue a third scan. Known remote metadata remains visible on rejected Refresh metadata IPC with its retry diagnostic.

The first full component rerun found one obsolete pre-existing assertion requiring paste errors to remain hidden. `tests/components/stack-toolbar-status.test.ts` was adapted to the explicitly authorized paste-only error presentation: it still requires the failed operation's phase/error styling, existing progress/spinner semantics, listing/icon suppression and inline placement; it now requires the stored error plus its own hover title, while retaining the phase hover title separately. The subsequent **full `npm run test:component` passed 27 files / 525 tests**, including all 37 refresh cases. This is an intentional contract update, not a suppressed runtime failure.

- `npm run check`: **zero errors, zero warnings**.
- `npm run build`: **passed**.
- `npm run test:node`: **1,269 passed, the same 12 clean-HEAD failures, 3 skipped, 3 todo** out of 1,287 tests; exit 1. `stack-git-refresh-review-final-node-comparison.json` verifies identical sorted failure names, with no extras.
- Exact `npm run cargo:test`: **passed: 1,189 tests, zero failures, 32 ignored**, across 28 test binaries. All **nine native refresh acceptance cases passed**, including actual canonical boundaries, legitimate absent metadata and all four selected nonzero/timeout probe failures. Ignored tests are not counted as passes.
- `npm run cargo:check`: **passed**, with existing warnings.

Both browser journeys were rerun after the production fixes and finished **PASS**:

- Original delayed-navigation/editor journey: **16 assertions**, frozen **41-call** ledger; six context probes, four summaries, three metadata reads and one editor save. The ordinary same-repo navigation segment still keeps summary=1/metadata=1 and the exact branch DOM node. Scheduling-dependent icon-call counts differ from the earlier 47-entry ledger; both full frozen ledgers are retained.
- Reviewer journey: **6 assertions**, frozen **34-call** ledger. Successful native paste followed by a rejected folder read increments summary to two and displays the listing failure; a second all-failed paste leaves that count at two and displays its failure. After an explicitly recorded remount, Refresh with rejected metadata preserves the known remote link and exposes its diagnostic. The ledger includes both paste invocations, remount reads and metadata failure request rather than hiding them.

Browser artifacts: approved temp `stack-git-refresh-review-green-original-browser.json`, `stack-git-refresh-review-green-browser.json`, `stack-git-refresh-review-green-browser.png` (visually inspected), and `stack-git-refresh-review-green-accessibility.txt`. No native shell was launched. Full post-review gate logs use `stack-git-refresh-review-final-{component,node,check,build,cargo-test,cargo-check}.log`; focused component log is `stack-git-refresh-review-green-component.log`.

Because strict deferred-metadata production code changed after the earlier five-lane measurements, its lane was remeasured separately: first **760.736 ms**, warm **844.947, 856.930, 997.017, 1023.996, 1161.649 ms**, median **997.017 ms**. All six calls made **four direct Git processes each, zero child Git processes, zero untracked-line enrichment** (24 TRACE2 starts total). This checkout's branch has no configured upstream: commands were canonical context, branch, origin-URL config and successful empty `for-each-ref --format=%(upstream)` discovery; no `rev-list` was needed. These timing samples ran concurrently with full JS validation and are **not controlled/comparable latency evidence or a claimed performance regression**. The prior metadata timing belongs to the pre-review helper implementation. Context/summary/initial critical-path implementations did not change in these fixes, so their earlier bounded evidence is retained without claiming a new measurement. Logs/traces/parsed values: `stack-git-refresh-review-metadata.log`, `stack-git-refresh-review-metadata-trace.jsonl`, `stack-git-refresh-review-metadata-measurements.json`.

All six exact project gates ran on the final production sources. Five passed; Node retained exactly its twelve proven HEAD failures and no new failures, so the overall `validate` chain is not globally green. All scoped reviewer regressions and both browser journeys passed. The scoped browser fixture build also passed (`stack-git-refresh-review-browser-build.log`, existing test-only large-chunk warning). Parent independent re-review remains the next completion gate. No live Tauri/Windows shell launch, dependencies, new polling or persistence were used.
