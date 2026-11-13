# Runtime memory and performance lab

Living scratchpad for the memory-first optimization campaign started 2026-11-11.

Campaign status: **2026-11-13 authorized source/fixture campaign complete: four further production optimizations independently validated and reviewed.** All fifteen follow-up candidates have explicit dispositions below. Native/lifecycle experiments remain evidence- or consent-gated, not exhausted in principle. The earlier campaign completed fourteen production optimizations (A04–A17); its historical validation and measurements remain below and are not fresh results for this checkout. Whole-app memory, speed and live native acceptance remain unmeasured.

### Native measurement follow-up (blocked; no accepted comparison)

After reviewing the fixture results and explicit native-consent limitation, the user requested **“Measure it”**. This authorizes controlled JasonShell launches for actual baseline/current runtime measurement. The prior no-launch statements below describe the completed source/fixture campaign, not this newly authorized follow-up. It does not authorize unrelated system changes, feature-policy changes, destructive workflows or modifying user data.

Measurement owner completed fresh release builds from baseline `3203040e` and current six frontend source files in isolated temporary staging, preserving the dirty checkout. Both use unique measurement identifier `com.jnev1.jasonshell.rammeasurement20261113`, fresh native settings without an imported model, and recorded source/binary SHA-256 hashes. Three counterbalanced idle pairs were planned with normal hooks/prewarm and per-attempt WebView user-data folders. Sampling tracks root/descendant PID plus creation identity; shutdown uses topbar `WM_CLOSE` with workarea/taskbar equality and survivor checks. Independent safety/methodology review is in progress. Raw artifacts: `test-results/runtime-ram-20261113/native-ram-native-20261113-080859/`.

**Safety stop after first pair:** baseline/current roots exited normally with code zero and no surviving descendants, but current post-exit taskbar visibility was false versus true before launch. The sampler stopped before pair two; no blanket cleanup or further native launch was performed. Reported logical workarea remained `2048×1152`; both runs logged expected physical `2560×1438` versus observed `2560×1440`. Read-only diagnosis is distinguishing auto-hide/transient visibility and DPI observer effects from a concrete restoration defect. Preserve both raw attempt series; they are not an accepted repeated comparison or proof of savings. Do not relax the restoration guard to finish the campaign.

Read-only follow-up found taskbar visibility returned to true without any restoration/input/system action. DPI-aware inspection (temporary thread PMv2 context restored in `finally`) reports physical SPI workarea `0,0,2560,1440`, taskbar rectangle `0,1438,2560,1488`, auto-hide state `1`, and unchanged taskbar HWND/Explorer PID plus creation time predating this campaign. Fifty-one 200 ms samples remained stable over ten seconds, with no JasonShell process alive. This supports a transient visibility observation, **not retrospective proof of exact physical prelaunch restoration**. Evidence: `post-stop-desktop-physical.json` and `post-stop-desktop-physical-series.json` under the native artifact root. Independent reviewer is assessing inspector safety and prospective physical/stable restoration invariants before any further launch.

Review requires checked cursor reads, style/exstyle capture, end-of-read taskbar/Explorer identity revalidation, checked DPI-context restoration and bounded input validation. Prospective collection must capture a stable physical prelaunch baseline and, after normal exit plus zero owned survivors, require the exact visibility/workarea/identity/auto-hide/style/geometry state to return continuously under unchanged pointer conditions within a bounded settling window. Owner is implementing the stricter ten-second stable interval within thirty seconds, with pure regression tests and independent re-review before launch. Auto-hide does not waive `WS_VISIBLE` equality; persistent mismatch or input/observer interference stops collection without automatic restoration. Optional UIA process-manager/terminal workflow must prove populated process rows and exclude its fixed dwell from action latency; it cannot establish icon/diff/output-churn savings.

Revised shared physical observer and integrated sampler are ready for re-review; no relaunch has occurred. Owner reports 21 restoration cases, 11 invariant checks, six process-identity cases, observer/DPI/consent checks and JS summary checks passing. Evidence: `review-guard-fix-tests.log` and `review-fix-readonly-observer-final.json`. Original incomplete pair warm-idle process-tree private-byte medians were **943.7 MiB baseline / 968.9 MiB current**; this single unaccepted pair is diagnostic only, not an optimization result or a repeated before/after comparison. Reviewer approval and explicit leader clearance remain required before fresh collection.

**Prospective gate review passed; leader cleared fresh collection.** Independent pure tests, parsing and read-only desktop observation passed; original findings closed. Exact leftover local profile ownership was verified against launch timing and the original absent-profile guard. Only that directory may be archived to the absent, non-reparse staging destination after fresh ownership/identity/survivor checks, using literal paths without force/deletion/wildcards. Three fresh counterbalanced pairs with the reviewed physical restoration gate and nondestructive process/terminal UIA workflow are now authorized; the original pair remains excluded. Final native E2E results and independent results review remain pending.

**Final outcome: blocked.** Fresh campaign `test-results/runtime-ram-20261113/native-accepted-20261113-091408/` stopped with exit 1 after only `pair1-baseline`; current and subsequent pairs were not launched. Accepted attempts/pairs: **0/0**. Fifty identical physical prelaunch observations spanned 10.180 seconds. Root PID 4376 exited normally (code 0), with zero tracked owned survivors. First post-exit observation returned `Mixed desktop state during observation`; the changed field is unknown, so exact restoration is unverified, not evidence of persistent taskbar/work-area damage. No repair or relaunch followed. The original stopped pair remains excluded, and the fresh campaign-owned baseline profile remains intact.

The actual native workflow completed three Process Manager open/close cycles, each with 472 populated production rows. Invoke-to-loaded times were **3,870.56 / 1,872.94 / 1,849.29 ms**, excluding the fixed dwell. First terminal invocation failed with `Unsupported Pattern`; no fallback input, terminal success or post-workflow quiet phase occurred. Native E2E was attempted, **not passed**. Existing summarizer rejected the invalid attempt; no accepted summary or paired delta exists.

Excluded baseline-only diagnostics: warm-idle tree private bytes **960.81 MiB** median (956.13–964.99), root **23.10 MiB**, descendants **937.71 MiB**; summed working sets **1,765.37 MiB**, potentially double-counting shared pages. Warm-window CPU was **1,281.25 ms over 15.29647 s**. These are observations, **not before/after savings or a speed comparison**. Full phase ranges, 92 samples and zero metric errors are in `excluded-baseline-diagnostics.json`; commands/provenance are in `final-handoff.json`, rejection in `summary-rejection.log`, hashes in `final-evidence-sha256.json`. All 13 measurement-artifact and six production-source hashes were unchanged. For exact creation-time identities use raw/string evidence: the derived diagnostic JSON rounds Explorer creation ticks.

Independent final results review confirmed the rejected comparison, raw metrics, restoration uncertainty and eight evidence hashes. Further collection requires narrow diagnosis of the observer's changed end-read field and the terminal's supported nondestructive accessibility pattern, followed by fresh clearance. No whole-app improvement has been established.

Required records: exact source/binary hashes, release/build mode, fixed scenarios/warmup/sample windows, root and descendant private bytes, separate working sets, CPU/threads/handles and variation. Churn-specific fixes require exercised workflows; idle-only differences cannot establish those savings. Deterministic nondestructive UI Automation follow-up is being prepared without CDP flags. No completed native comparison is claimed yet; this section will record results or concrete blockers when execution finishes.

## Follow-up campaign: 2026-11-13

Priority: runtime RAM, then speed. Explore remaining techniques autonomously, retain only evidence-backed improvements, and preserve features, security, cancellation, replay and shell safety. No native shell launch or consent-dependent smoke is authorized. No whole-app savings will be inferred from fixture allocation counts.

Observed host/toolchain: Windows 10 Home `10.0.19045` x64; PowerShell `7.6.10`; Node `v24.14.0`; npm `11.9.0`; rustc `1.97.1 (8bab26f4f 2026-07-14)`; Cargo `1.97.1 (c980f4866 2026-06-30)`. These identify this follow-up environment, not a native memory sample.

| Work item | State | Evidence / next action |
|---|---|---|
| Fresh shared validation baseline | Captured; Rust prerequisites subsequently repaired | Check/build/Cargo check pass; components 531 pass; Node 1,376 pass / 7 inherited failures. Initial Cargo tests lacked frontend example artifacts; frontend-only builds subsequently enabled 1,204 Rust tests to pass / 30 ignored. Raw logs: `test-results/runtime-ram-20261113/` |
| Native remaining-opportunity audit | Complete | Existing caps/ownership confirmed; queued terminal output and model/WebView residency remain measurement-dependent, not claimed wins |
| Renderer remaining-opportunity audit | Complete | Four grounded candidates B01/B12–B14 have mounted mocked-IPC acceptance coverage and before measurements |
| New production changes | Complete; independently verified and reviewed | 43/43 RAM acceptance, 531/531 components, 1,204 Rust tests pass / 30 ignored. Review-found delayed-listing disposal race closed: no new icon IPC and zero cache entries/bytes after unmount. Two obsolete Node contracts adapted and independently reviewed: 68/68 focused pass; full Node 1,376 pass / seven inherited failures, all normalized failure payloads match baseline. Final build/check and docs policy pass |
| Whole-app private bytes / working set / peak | Unmeasured | Native launch remains consent-gated; fixture evidence is not a replacement |

This section is the current follow-along sheet. Append measured before/after rows and candidate dispositions as investigations finish. “Exhausted” means no remaining justified safe candidate under the available evidence and authorization, not that no future optimization exists.

### Follow-up candidate register

| ID | Mechanism / source | Disposition | Evidence required / constraint |
|---|---|---|---|
| B01 | Unused terminal title output overlaps replay: `TerminalPanelSurface.svelte::rememberTerminalTitleOutput` | Implemented; independently verified and reviewed | 20,000 → zero unused characters in runtime-less session, separately from replay. Mounted acceptance preserves titles, reattach, display/replay and cleanup. Character counts are not heap/RSS evidence |
| B02 | Stack terminal poll queue: `stack_popup/terminal.rs`, 1,024 messages × 4,096-byte reads | Measure first | Approximately 4 MiB queued text per session before decoding expansion/overhead; actual occupancy unknown. Do not lower capacity without push/poll/recovery/overflow evidence or silently lose ordered output |
| B03 | Full process-tree measurement, not root-only metrics | Measurement gap | Existing `measure-performance.ps1` samples root metrics; WebView/child attribution needs identity-safe process records. Summed working sets double-count shared pages; private bytes, working set and reserve are different metrics |
| B04 | Large values behind entry-count bounds: icons, images, drafts, active sessions, fragmented dedupe intervals | Scoped lead B12; other quotas deferred | Renderer icon map is actionable. No silent active-session/replay/draft eviction; bounded native caches do not establish a redundant owned payload |
| B05 | Snapshot IPC serialization and stale work coalescing | No additional justified change | Prior shared-index/rank-before-hydration wins remain. No new production duplicate ownership established; only superseded snapshots may coalesce, never ordered terminal/auth transitions |
| B06 | Hidden renderer DOM, xterm/WebGL and image URL lifetime | Already optimized / lifecycle deferred | Lazy per-window routing, xterm disposal and object-URL replacement/dismiss/stale-result cleanup confirmed. Additional teardown needs state/focus/native residency evidence |
| B07 | Release LTO/codegen/stripping, image compression, allocators | Experiment deferred | Binary size or encoded payload size does not establish private-byte improvement. No dependencies/global heap knobs without traces and measured non-regression |
| B08 | Thread stacks and inference pools | Measurement deferred | Distinguish reserved address space from committed stack pages; tune only owned idle pools with concurrency/cancellation evidence |
| B09 | Speech model mmap/unload and WebView pooling/teardown | Lifecycle deferred | Likely consequential, not proven. Native model/process-tree attribution plus API/security/latency evidence required; no consent to launch shell |
| B10 | Opt-in low-memory mode disabling idle terminal prewarm | Product proposal deferred | RAM priority does not authorize changing intentional readiness policy; needs explicit product/persistence/latency decision and native evidence |
| B11 | Working-set trimming, forced GC, smaller replay/history quotas, unsafe clipboard copy removal | Rejected | Trimming/GC is not retained-memory proof; arbitrary quotas remove semantics; clipboard requires consumer ownership |
| B12 | Stack renderer `StackPopupSurface.svelte::iconCache` retains every resolved path across navigation/hide | Implemented; independently verified and reviewed | LRU bounds estimated key/value strings to 2 MiB and 256 entries, including misses. Oversized values still display; stale-batch and post-disposal acceptance passes |
| B13 | `ProcessManagerSurface.svelte::closeSurface` retains `processes` snapshot after hide | Implemented; independently verified and reviewed | Both accepted hide routes release process/PID snapshots while preserving reopen/refetch, request-generation invalidation and pending-kill protection |
| B14 | `StackGitPanel.svelte::getDiffLines` splits full diff before displaying 4,000 lines | Implemented; independently verified and reviewed | Prefix scanner counts all lines without splitting all text; CRLF/count/truncation acceptance passes. This is peak materialization, not steady RSS |
| B15 | WebView2 inactive memory target / renderer suspension / shared environments | Vendor-supported; evidence deferred | Microsoft documents low target (scripts continue; pages may swap), suspension (timers pause) and environment reuse. None establishes released private allocation; wrapper/API support, current sharing, hidden event/replay contracts, resume latency and process-tree evidence must be verified before adoption |

Native audit dispositions: speech history is bounded to five × 16 KiB transcripts; Quick Commands keeps 20 history entries with 16 KiB streams / 256 transcript rows; gallery close clears thumbnail state; Stack paging has 32-session / 50,000-entry / 32 MiB limits; Git reads have 256-file / 8 MiB aggregate / 1 MiB per-file caps and 64 KiB process streams. These are bounds, not measured application residency. Existing A04–A17 work is not recounted as new savings.

Read-only host check found no `jason-shell.exe` process; no application was launched. This is not process-tree memory measurement.

B01 follow-up: the title helper `src/features/terminal/terminalTabTitle.ts` does not read `recentOutputText`. Removing this unused title metadata needs no replacement parser; terminal display/replay are separate and must stay unchanged. Dedicated RED tests precede production edits; existing title/replay/reattach checks already pass before implementation.

Implementation preparation corrected a stale audit assumption: current Process Manager snapshots already use `$state.raw`, including `processes`, taskbar PIDs and system-info snapshots. No proxy migration is needed. Hidden-snapshot release must cover accepted reconciliation as well as explicit close, while preserving stale-generation guards and pending-kill duplicate protection. Terminal runtime output history is **not** the unused title metadata and remains required.

### Follow-up comparison sheet

| Candidate | Before, source-established | Before, fixture | After, fixture | Required non-regression |
|---|---|---|---|---|
| B01 title metadata | Up to 20,000 unused characters per session, separate from required runtime/replay state | 20,000 retained characters in runtime-less session | Zero title-output characters; reattached runtime output remains separate | Same title selection, replay, reattach and cleanup |
| B12 icon ownership | Renderer map has no entry/byte budget across folder churn | 13 folders: 624 entries / 10,287,676 estimated string bytes; negative-only churn: 624 entries; stale response pollutes cache | Same positive churn: 127 entries / 2,094,128 charged bytes; 48 visible icons still display. Default direct 10,000-negative-key churn: 256 entries / 9,728 charged bytes | Visible icons still display, cached misses bounded, stale batches rejected |
| B13 hidden process snapshot | Accepted hide can retain previous process rows | Both close routes retain 40 rows / 665,910 serialized characters | Both routes: zero rows / payload characters; pending kill still exactly one destructive mocked invocation across close/reopen | Reopen refetches; stale requests, generation and kill safety preserved |
| B14 diff parsing | Whole diff split before 4,000-line display limit | 30,000-line fixture materializes 30,000 split strings, returns 4,001; mounted viewer materializes 10,000 | Same fixture returns 4,001 prefix/notice strings; zero whole-input split strings | Exact line count/truncation, CRLF/trailing-line and hunk parity |

Source-established bounds describe ownership opportunities, not measured heap bytes. Fixture counts, string lengths, accounted string bytes and allocation probes must remain labelled as such; none is whole-app private bytes or a guarantee about JavaScript string representation.

Acceptance reproduction: `node tests/runtime-ram/run.mjs --reporter=default --reporter=json --outputFile=test-results/runtime-ram-20261113/acceptance-rerun.json` (choose a fresh filename for each run). Test contracts and observation limits: `tests/runtime-ram/README.md`; baseline metrics/source hashes: `test-results/runtime-ram-20261113/metrics-before.json`; staged RED evidence: `red-handoff.{json,log}` in the same ignored results directory. No native shell was launched.

Independent final production evidence: `test-results/runtime-ram-20261113/final-20261113-011756294/`, `metrics-after.json` and `validated-production-sha256.json`. All six validated production source hashes stayed unchanged. Cargo prerequisite remediation built only the two frontend example artifacts, never their native launchers. All 209 generated permission files were restored to baseline SHA-256 bytes; no permission changes remain. `git diff --check` passed. Runtime speed and whole-app heap/private bytes remain unmeasured; these are logical retention/materialization improvements.

Fresh baseline Node failures (excluded from this optimization): two centered-search combobox/listbox contracts in `centeredSearchSurface.test.mjs`; agent-instruction routing in `changelogPolicyHygiene.test.mjs`; centered-search clear button in `searchClearButtons.test.mjs`; two flat-visibleRows/keyboard-ARIA contracts in `searchPanelState.test.mjs`; source-text intent registry in `sourceContractIntentRegistry.test.mjs`. Their full assertion payloads are preserved with baseline evidence. An intermediate final run also failed obsolete title-output accumulation and inline diff truncation assertions; both task-owned contracts were adapted without production changes. Final Node evidence and reproducible normalized baseline comparison: `test-results/runtime-ram-20261113/node-contract-adaptation-20261113-012454121/` and `compare-node-contract-adaptation.mjs`. All seven remaining assertion payloads match baseline; all six validated production source hashes remain unchanged.

### Remaining whole-app evidence path (not executed)

The current root-only sampler cannot answer whether WebView2/model/terminal ownership dominates application RAM. A future authorized release-build run should record:

- Build/source identity, settings/model/profile, monitor topology, scenario inputs and process creation times. Match processes by PID **and creation time**; do not attribute a reused PID to the shell. Capture descendant changes throughout the scenario, including short-lived and surviving children.
- Root and descendant private bytes separately, working sets separately, peak private bytes, CPU time, thread count and handle count. Summed working sets can double-count shared pages; reserved stack address space is not committed RAM. Do not present either as unique retained bytes.
- Repeated cold idle, warm idle, first-open, repeated open/close, folder/icon churn, terminal output/reattach and post-close quiet-state samples with identical inputs. Include all raw samples rather than choosing the best run. Distinguish root exit, descendant exit, allocator retention and a merely lower working set.
- Paired first-open/warm-action latency and CPU alongside RAM. Revert an experiment if apparent savings remove required replay, hide stale state, weaken authorization/cancellation or regress the accepted interaction contract.

Model unload/mmap, native window teardown/pooling, terminal prewarm policy, thread pools/stacks, release profiles and allocator experiments stay deferred until this evidence can support their actual ownership and latency trade-offs. No automatic working-set trim or forced GC substitutes for it.

Vendor research: [WebView2 performance best practices](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance), [Win32 memory target API](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_19) and [Win32 suspension API](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_3). A low target is best-effort and can swap cached pages rather than remove ownership; restore normal on activation. Suspension pauses scripts/timers and requires an invisible controller, so it cannot be blindly applied to hidden event consumers. Do not mix suspension and manual target policies. Environment sharing must not weaken profile/capability isolation; no current sharing change or saving is claimed.

## Goal and evidence rules

Reduce retained runtime memory first; improve latency and CPU work second without removing features or weakening safety. Record measured results, not predicted percentage claims. Fixture allocation/retention, bundle sizes, and benchmark timings are not whole-application private bytes. Compare identical inputs and commands; retain regressions and unsuccessful experiments here.

No live shell launch is authorized in this campaign: AppBars, hooks, Explorer taskbar changes, native interaction, hardware, and actual WebView2 process-tree memory remain unmeasured. No dependencies, system configuration changes, or automatic feature removal.

## Historical metrics sheet (2026-11-11)

The B01–B05 rows in this historical table are validation-run labels from the earlier campaign, not the B01–B15 follow-up candidate IDs above. Follow-up measurements must record their own inputs, commands and source identity before comparison.

| ID | Scenario / metric | Before | After | Evidence / limitation |
|---|---|---|---|---|
| B01 | Automated validation baseline | `validate` fails at Node gate | — | Before production changes: check/build pass; component 395/395 pass; Node 1238 pass, 10 fail, 2 skipped, 3 TODO |
| B02 | Native validation baseline | Cargo check passes; tests link-blocked | — | Offline check 29.77 s / 256 existing warnings; test link fails `OrtGetApiBase`, so runtime Rust tests did not execute |
| B03 | First final shared-gate rerun | B01 baseline | Check/build pass; component 419 pass / 15 fail; Node 1,252 pass / 11 fail / 2 skipped / 3 TODO | Historical failed rerun: 15 obsolete terminal IPC mocks and 1 stale command-panel Set assertion; repaired and superseded by B04/B05 |
| B04 | Repaired final shared-gate rerun | B03 | Check/build pass; component 436/436 pass; Node 1,253 pass / 10 fail / 2 skipped / 3 TODO | New runtime/mock/static regressions resolved; additional registry metadata omission repaired under B05 |
| B05 | Final affected Node gate after intent registration | B04 | 1,253 pass / 10 baseline fail / 2 skipped / 3 TODO | 0 new failures / 0 new registry omissions; failure payloads match original baseline after timing/stack-location normalization |
| M01 | Whole-app idle private bytes / working set | Not measured | Not measured | Live shell consent unavailable; do not infer from isolated tests |
| M02 | Whole-app peak / post-use private bytes | Not measured | Not measured | Same boundary as M01 |
| T01 | Native cold/warm panel latency | Not measured | Not measured | No native shell launch |
| E01 | Existing running JasonShell process observation | No matching process | — | Read-only `Get-Process` name filter; no process launched, no whole-app memory sample available |
| F01 | Process sort, 512-row fixture, Set constructions/invocation | 7,524 | 1 | Three fresh Node processes; deterministic 128-active-PID fixture; constructor count, NOT allocator bytes |
| F02 | Process refresh, same fixture, Set constructions/invocation | 13,600 | 1 | Preserve-order refresh; same limitation as F01 |
| F03 | Process sort median time / p95 | 20.0736 / 20.6215 ms | 1.0550 / 1.4103 ms | Across-run median of 3 process medians / per-run p95s, 5 warmups/30 paired samples each |
| F04 | Process refresh median time / p95 | 35.3380 / 37.5488 ms | 2.0380 / 3.5393 ms | Same setup as F03; fixture-only, not native UI latency |
| F05 | Clipboard 4K peak instrumented staged bytes | 132,719,996 B | 99,542,356 B | Actual clipboard module + geometry, Win32 allocation primitives; 33,177,640 B staging overlap removed; no clipboard publication |
| F06 | App-index 4,000-entry / 100-snapshot allocation requests | 2,400,100 allocations / 114,000,000 B | 0 / 0 B | Isolated operation-only allocator probe; shared snapshots, not retained allocator/RSS |
| F07 | App ranking icon hydration calls, 4,000 matches / limit 30 | 4,000 | 30 | Winner-only hydration; limit-zero acceptance also passes |
| F08 | Terminal 16 MiB / 4,096-byte reads, allocation requests | 34,013,710 B / 12,304 allocations | 17,236,494 B / 8,208 allocations | Reader fixture; actual ConPTY and shell not launched |
| F09 | Speech contiguous-input allocation requests | 7 allocations / 6 reallocations / 1,300,480 B | 1 allocation / 0 reallocations / 655,360 B | Actual-source independent allocator probe; bitwise input equality; no model loaded |
| F10 | Quick Commands 40 completed runs + 1 active prompt | 41 runs / 24,041 rows / 240,410 body characters | 21 / 5,121 / 51,210 | Deterministic renderer merge fixture; not heap bytes |
| F11 | Mounted Quick Commands 24 completed runs | 24 runs / 14,424 rows / 98,232 body characters | 20 / 5,120 / 35,720 | Mocked Tauri fixture; no desktop launch |
| F12 | Snip explicit typed-array staging slices | 1 per payload | 0 | 10-byte full / 6-byte offset ordinary views reuse backing; shared subview requires 6-byte compatibility copy; Blob snapshot remains |
| F13 | Persist 4,000 app-index entries, allocation requests | 24,015 allocations / 3,237,024 B | 14 / 2,097,024 B | Same 792,050 serialized JSON bytes; borrowed view, not a schema change |
| F14 | Rank 4,000 app matches / retain 30, allocation requests | 300,018 allocations / 18,065,853 B | 161,219 / 7,057,773 B | Actual-source operation probe; ranking parity checked |
| F15 | PNG CRC 4K isolated median, five repetitions | 168.087 ms bitwise | 78.323 ms table | Production implementation adds 1 KiB static table; full PNG byte parity passes; fixture timing only |
| F16 | Terminal stop churn, 10,000 unique sessions | 10,000 obsolete retained IDs | 0 | Extracted registry/auth methods; 0 active sessions / 10,000 drop-observable ownership releases; not ConPTY termination proof |
| F17 | Visible terminal, 10,000 distinct output chunks | 10,000 Set copies / 50,005,000 copied keys | 0 / 0 | Actual-source panel declaration fixture; one canonical tracker; contiguous history retains 1 interval / 0 fallback keys; not heap bytes |
| F18 | Hidden terminal, 10,000 chunks plus duplicate delivery | 20,000 Set copies / 149,995,000 copied keys | 0 / 0 | Canonical shared tracker; no membership loss for sparse/out-of-order sequences |
| F19 | Native process descendant counts, 1,000-process chain | 21,181 allocations / 13,967,918 B | 3,021 / 271,446 B | Operation-only allocator probe; median 23.007 → 13.133 ms; kill traversal unchanged |
| F20 | Serialized terminal polling fixture response | 26,885 B legacy | 16,857 B chunks-only | 10,028 duplicated aggregate-output bytes removed for this fixture; not universal IPC-size saving |

## Experiment ledger

| ID | Hypothesis / target | State | Result / next evidence |
|---|---|---|---|
| A01 | Inventory retained buffers, caches, queues, models, eager surfaces and idle work | First pass complete | Most obvious caches/transcripts already bounded; aggregate and copy-cost investigation follows |
| A02 | Establish safe repeatable isolated benchmark path and regression baseline | Baseline captured | Existing Node failures/native linker blocker separated from intended RED; safe fixture benchmarks available |
| A03 | Brainstorm redundant large allocations/copies and avoid unnecessary retention | Native pass complete | Eight grounded native candidates; all improvements require isolated evidence before claims |
| A04 | Stop rebuilding taskbar PID membership inside process-sort comparisons | Complete: tests and independent review pass | 32/32 focused pass after 2 initial RED; membership shared across refresh passes; redundant owned-array copy removed; F01–F04 measured |
| A05 | Release clipboard DIB staging before allocating PNG HGLOBAL | Complete: tests and independent review pass | F05 measured; failure cleanup, ownership and byte parity pass; conservative admission unchanged |
| A06 | Share immutable app-index snapshots | Complete: tests and independent review pass | F06 measured; shared-generation identity/lifetime fixtures pass |
| A07 | Hydrate app results/icons only after ranking/truncation | Complete: tests and independent review pass | F07/F14 measured; exact ordering and limit-zero fixtures pass |
| A08 | Move terminal reader text through event then queue | Complete: tests and independent review pass | F08 measured; UTF-8/EOF/channel and safe push/poll dedupe fixtures pass |
| A09 | Borrow app-cache entries for persisted serialization | Complete: tests and independent review pass | F13 measured; identical JSON/schema fixtures pass |
| A10 | Count process descendants with reusable traversal scratch | Complete: tests and independent review pass | F19 measured; cycle/duplicate/origin semantics preserved; kill planning untouched |
| A11 | Pre-size contiguous speech inference input | Complete: tests and independent review pass | F09 measured; no inference/model lifecycle change |
| A12 | Table-based PNG CRC | Complete: tests and independent review pass | F15 measured; same encoded bytes, 1 KiB static table tradeoff |
| A13 | Remove screenshot typed-array staging copy before Blob | Complete: tests and independent review pass | F12 measured; ordinary subview avoids copy, shared backing retains compatibility copy; Blob snapshot remains |
| A14 | Bound renderer Quick Commands retained history/transcripts | Complete: tests and independent review pass | F10/F11 measured; expanded 65/65 component and 31/31 Node pass, including exact Stop targeting and 100-run chronology |
| A15 | Remove unused production terminal stop tombstones | Complete: tests and independent review pass | 10,000 stops → zero retained IDs; actual ownership/target-isolation assertions replace obsolete cfg(test) tombstone checks |
| A16 | Canonical terminal dedupe ownership and exact interval compaction | Complete: tests and independent review pass | Expanded mounted 49/49 and Node 7/7 pass; F17/F18 measured; fragmented histories may still grow |
| A17 | Use existing chunks-only terminal poll route | Complete: tests and independent review pass | F20 measured; both renderer consumers migrated; legacy native read contract and authorization preserved |

## Decisions and safeguards

- Initial worktree clean; preserve unrelated changes introduced during this session.
- Production changes follow failing regression evidence where feasible, then focused reruns and independent review.
- Preserve terminal replay, output ordering, identity/authorization checks, cancellation and bounded resource contracts.
- Keep raw measurements and command provenance; noisy timings are observations, not guaranteed speedups.
- Ordinary ArrayBuffer-backed snip views should avoid preparatory copies. SharedArrayBuffer-backed views may require an ordinary-buffer compatibility snapshot for Blob; zero `slice()` calls alone would not establish zero copied bytes. Test this boundary rather than casting away the backing type.
- Reject a measured optimization if it weakens ownership, ordering, cancellation, authorization or active controls. Passing a source-pattern check is not sufficient memory evidence; retain rejected experiments and their reason.

### Historical baseline provenance and pre-existing blockers (2026-11-11)

Evidence directory: `C:\Users\jnev1\AppData\Local\Temp\opencode\memory-speed-baseline-20261111\` (environment JSON, command logs/results, `measurement-inventory.md`, `process-sort-handoff.md`, and three `process-sort-before-run*.json` files). Windows 19045 x64; Node 24.14.0; npm 11.9.0; Rust/Cargo 1.97.1; external cached Cargo target `C:\dev\jasonshell\src-tauri\target`.

A04 after-evidence: `process-sort-after-validation.json`, three `process-sort-after-run*.json` files and `process-sort-comparison.json` in that directory. Independent rerun compiled in 2.115 s and passed all 32 focused tests without skips. Fixture/oracle hashes, Node and CPU matched baseline. After each invocation constructs one Set, enumerates membership once and yields 128 active PIDs. Median fixture time fell 94.74% for sort and 94.23% for refresh; this is not an app-wide speedup or retained-memory percentage. Freshness, frozen caller input, row identity and all columns/directions were checked against the legacy oracle. Independent source/benchmark review passed: the refresh array is owned, constructor instrumentation is restored, counts run separately from timing, and no new process-action/security boundary exists. Shared classification oracle and Node/GC timing variability remain limits.

`npm run validate` baseline stopped after 43.42 s at Node tests. Vite reported 5.43 s; component runner 16.34 s; Node runner 3.901 s (these are individual tool phases, not comparable benchmark totals). Ten baseline failing contracts: `contextMenuSpeechAcceptance` (1), `contextMenuVisualRegression` (2), `frontendUiPolicy` (1), `meltMigrationWiring` (1), `persistentTerminalPanel` (1), `sourceContractIntentRegistry` (1), `speechIndicator` (1), `taskbarAttentionModel` (1), `topBarMicControl` (1). Exact names/errors are retained in the inventory/log; unrelated visual/source-contract changes are excluded.

Offline Cargo tests failed at link in 92.11 s because ONNX Runtime `OrtGetApiBase` was unavailable. No Rust runtime test pass is claimed. Isolated harnesses must exercise actual scoped production source and disclose mocked/native boundary limitations rather than silently substituting a copied algorithm.

A05–A12 final independent fixtures: `node scripts/benchmark-native-memory.mjs` reports 32 pass; `node scripts/benchmark-native-clipboard.mjs` reports 6 pass. `node tests/fixtures/native-memory/integration.mjs` passes actual-source search conversion → panel progress/final/stale-query fixtures and terminal producer → extracted renderer consumer push/poll in both orders, adapted to the interval tracker. Offline `cargo check --tests` passes after four Arc test-constructor and two terminal ownership-test adaptations; linking the full suite remains blocked. Initial RED was 14 pass / 4 intended failures plus clipboard 2 pass / 1 intended failure. Clipboard staging instrumentation lives only in a generated translation unit; no publication occurs. Durable harnesses and fixtures: `scripts/benchmark-native-{memory,clipboard}.mjs`, `tests/fixtures/native-memory/`. Immutable before evidence: `test-results/native-memory/before/`; source identities now use SHA-256. Independent native source/evidence review passes (A05–A12 and A15).

Final native type-check provenance: `tests/fixtures/terminalRegistryRetention.evidence.json` records PASS, exit 0 and durable checkout-relative output `test-results/native-memory/final-cargo-check-tests.log`, with command/result metadata in `final-cargo-check-tests.meta.json`. An earlier tool-managed capture expired; the check was repeated to preserve accessible evidence (7.220 s, existing warnings). The older `test-results/native-memory/cargo-check-tests.txt` contains intermediate removed-helper errors and is not the final check result. Registry evidence discloses that its original RED source hash was not captured; final production-declaration SHA-256 is recorded. No linked test execution is inferred from type checking.

First final shared rerun: `final-validate-20261111-011135421.log` and `final-node-20261111-011253238.log` in the baseline evidence directory. Check reports zero errors/warnings; build passes (Vite 5.57 s). Components failed 15 status cases with `Unexpected terminal test IPC: poll_stack_terminal_session`, short-circuiting `validate` before Node/Cargo. Separate Node run preserved all ten original failures and added one stale wiring assertion. `final-node-comparison-20261111-011253238.json` records exact baseline/new classification. Dedicated owners repaired test contracts without production changes: status/expanded terminal 49/49, wiring 20/20, renderer memory 65/65 pass. Wiring assertions verify synchronous terminal removal, history-derived active commands, retirement replay rejection, stopping cleanup and pending reconciliation rather than obsolete Set syntax. Evidence: `memory-wiring-before.txt`, `memory-wiring-after.txt`, `memory-wiring-mounted-after.txt` in the approved temp directory. Subsequent shared rerun and independent review close these regressions; focused passes alone were not treated as sufficient.

Renderer RED handoff: `tests/memory-retention-handoff.md`, with local output `C:\Users\jnev1\AppData\Local\Temp\opencode\memory-red-final.txt`. Five focused component suites collected correctly: 53 pass / 8 intended RED, including 40 existing product/lifecycle checks passing. Besides retention, the mounted fixture exposes stale pending input after stop. The acceptance preserves live-known pending request metadata after its transcript row ages out; cold mounting after native request-row eviction is not certified because native snapshots have no independent pending-request field.

Renderer independent GREEN: original five suites 61/61, then expanded safety assertions 65/65; test TypeScript compilation and Quick Commands Node 31/31 pass. Evidence in the approved temp directory: `memory-green-independent.txt`, `memory-green-final-independent.txt`, `memory-quickcommands-independent.txt`. Tests check 100-run chronology replay, monotonic terminal state, pending-input resolution after eviction, exact submit/Stop targeting after native omission, and ordinary/shared Blob backing distinctions. Independent source review passes: one weakly owned chronology watermark per current array and input control per current run, not a growing strong tombstone collection. GC/RSS and cold-mount recovery after native prompt eviction remain unmeasured.

Terminal retention original RED: `node --test tests/terminalSequenceRetention.test.mjs tests/terminalRegistryRetention.test.mjs tests/terminalPollingContract.test.mjs` reported 2 pass / 6 intended failures; mounted `tests/components/terminal-retention.test.ts` reported 1 pass / 7 intended failures without unhandled errors. Causes: retained stop IDs, missing exact sequence tracker, missing chunks-only polling adapter. `tests/fixtures/terminalRetentionBaseline.mjs` captures F16–F18; the registry fixture uses a drop-observable session double, not native child processes. Payload comparison was deferred until the adapter existed; F20 now records it.

Terminal independent GREEN: expanded retention (16 cases) plus the existing status suite pass 49/49 mounted checks; sequence/polling Node checks pass 7/7 and TypeScript test compilation passes. Both surfaces cover push-first and poll-first, sparse/duplicate membership, hidden/reopen, stale/replacement delivery, cwd and exit/final-output order. Stack pre-start push is recovered through polling, not certified as pre-session event buffering. Existing affected Node suites pass 45 checks with the unchanged crop-icon baseline failure. `tests/fixtures/terminalRetention.evidence.json` records measured counts/payload bytes and current source hashes; original RED hash was not captured and is explicitly not invented. The 15 status-suite mock failures from B03 are resolved by realistic `running/chunks/cwd` responses, preserving all status assertions. Terminal source/test review and final shared validation reconciliation pass within the disclosed baseline-red limits.

Final terminal production/test review passes, including the repaired status mock and Command Panel wiring assertions. Shared rerun `repaired-final-validate-20261111-011605620.log` passes check (zero errors/warnings), build (Vite 5.37 s) and all 436 mounted tests in 24 files. Node ran within `validate`: 1,268 total, 1,253 pass, 10 fail, 2 skipped, 3 TODO. Although failure titles matched baseline, the diagnostic comparison caught a new missing intent for `terminalRegistryRetention.test.mjs`. Its accurate `architectureContract` registration was added and independently reviewed without weakening compliance checks. Final affected `npm run test:node` took 6.064 s: same totals, zero new failures, zero new omissions; all ten payloads match original baseline after timing/stack-location normalization. Missing list is again exactly `['taskPreviewBaseline.test.mjs']`. Final evidence: `metadata-final-node-20261111-011908594.log`, `metadata-final-node-20261111-011908594-result.json`, `metadata-final-node-comparison-20261111-011908594.json`. Check/build/components were not repeated for a metadata-only registry edit; their relevant source remained unchanged. The full repository suite is still baseline-red, not claimed green.

The existing `measure-performance.ps1` launches/stops shell processes and samples only the root, not the full WebView2 tree. It also assumes checkout-local target paths. It is not run under current authorization and cannot establish whole-app memory numbers here.

### Experiment acceptance and measurement interpretation

| Lane | Decisive evidence | Preserve / refuse |
|---|---|---|
| Process sorting | One invocation-scoped PID membership preparation; three paired timing runs and full ordering oracle parity | No cross-call option cache; no caller mutation; constructor counts are not retained bytes |
| Clipboard staging | Actual-source buffer-lifetime/allocation probe; identical DIB/PNG payloads | Keep conservative admission charge and native publication ownership; no real clipboard write |
| Search cache and ranking | Shared-generation identity/lifetime, operation-only allocator counts, JSON/ranking parity and icon hydration count | Preserve cache age/state, exact score/title/key tie order, stale-query suppression and serialization schema |
| Terminal reader | Actual reader fixture, split/invalid UTF-8 and EOF; allocation delta plus push/poll sequence parity | Preserve channel backpressure, disconnect handling, replay and deduplication; no real ConPTY launch |
| Process descendants | Differential count tests over chains/stars/cycles/orphans/duplicates; allocator probe | Do not alter process termination planning or authorization |
| Speech input | Contiguous input equality and allocator probe before deciding | Reject speculative improvement; no model loading or lifecycle changes |
| PNG CRC | Known vectors and complete PNG byte parity; separate release timing | Account for static table bytes; reject unexplained memory/latency regression |
| Screenshot Blob | Full/subview binary fixtures and explicit staging-copy count | Respect view offset/length, response shapes and URL cleanup; Blob snapshot allocation remains |
| Quick Commands history | Mounted mocked-runtime churn beyond native limits, active input and delayed/stale messages | Preserve interactive requests and terminal statuses; no unbounded tombstone workaround |
| Terminal retention/polling | Mounted push/poll, hidden/replacement/cleanup/stale fixtures; Set-oracle parity; tracker counts and response bytes | Preserve startup output and exact out-of-order membership; no session quota, replay removal or legacy IPC break |

Each lane has a dedicated test owner and separate production owner. Fixture integration can establish deterministic event/retention contracts but cannot verify desktop rendering, Win32/ConPTY integration, WebView2 memory or hardware behavior. A lower allocation count is useful evidence, not proof of lower steady-state application memory. Timing and allocation instrumentation must run separately; raw samples and toolchain/source identity accompany measurements.

Campaign completion gates: each implemented lane must pass focused acceptance and independent source review; shared frontend/build and native type-check gates must be rerun after relevant production changes. Compare broad-suite failures with the recorded baseline, distinguishing new failures from existing ones. No global coverage percentage is asserted without instrumentation. Live shell/model/hardware checks remain explicit unrun follow-ups rather than silently passed gates.

### Safe fixture reproduction

Use the existing installed toolchain and cached Rust dependencies; these commands do not launch the shell. Native harnesses require the cached serde artifacts/toolchain described in their source, so they are environment-dependent rather than a replacement for linked Cargo tests.

```powershell
node node_modules/typescript/bin/tsc -p tsconfig.test.json
node scripts/benchmark-process-manager-sort.mjs --output test-results/process-sort-new.json
node scripts/benchmark-native-memory.mjs
node scripts/benchmark-native-clipboard.mjs
node tests/fixtures/native-memory/integration.mjs
node --test tests/terminalSequenceRetention.test.mjs tests/terminalRegistryRetention.test.mjs tests/terminalPollingContract.test.mjs
```

Choose a new process-sort output path for each run; the script refuses overwriting prior evidence. Native harnesses regenerate current raw outputs under ignored `test-results/native-memory/`; preserve any run needed for comparison separately. Original campaign before-evidence is archived under `before/`. Production-source hashes and fixture/toolchain context belong with every recorded comparison.

## Idea exhaustion / disposition

Each concrete candidate receives an implemented, rejected, already-bounded, or blocked disposition with reason. Exhaustion means no remaining justified safe candidate supported by available evidence, not a claim that no future optimization exists.

At the **2026-11-11 campaign close**, all fourteen grounded runtime candidates A04–A17 were implemented, measured and independently reviewed. The historical dispositions below describe that campaign, not the reopened investigation above. Fresh audits found additional renderer ownership candidates; current fixtures still cannot establish the largest remaining whole-app residency costs.

| Candidate | Disposition | Reason / evidence needed |
|---|---|---|
| Remove permanent taskbar 1 Hz polling | Already optimized | Current task snapshot pipeline and focused tests already cover removal |
| Bound icon entry counts and TTL | Already optimized | Task cache 256 entries; process/search/Stack caches 128; encoded strings, not retained native icon handles |
| Bound Quick Commands transcript / renderer history | Implemented and reviewed under A14 | Baseline renderer merge helpers retained rows/completed runs indefinitely despite native caps. Renderer now retains newest 256 rows per run and 20 completed runs plus known active runs; pending controls survive row eviction |
| Unload ready speech model automatically | Deferred pending evidence | Potentially large saving, but changes availability and warm latency; no real-model residency measurements |
| Aggregate terminal replay quota | Rejected without product decision | Per-session replay cap 262,144 code units; active session count intentionally permits more than four; no justified aggregate quota or silent session removal |
| Shared text-document page-store quota | Deferred; developer-only subsystem | No production commands/callers; feasibility session configured cache capacity 212,992 B excluding held references, despite constructor ceiling 32 MiB per store |
| Skip unused zero-cache PageStore append copy | Deferred; developer-only | Real redundant allocation in feasibility/recovery kernel, not shipped-runtime memory; avoid expanding campaign into developer subsystems |
| Bound diagnostic count retention | Already bounded | Native/frontend 200-entry rings; task-window known/unresolved ID sets 128 each, unresolved sample 8; entry count is not a byte bound |
| Replace terminal dedupe with only largest sequence seen | Rejected | Drops unseen out-of-order output; exact per-stream intervals are lossless alternative under A16 |
| Lazy native window creation / WebView teardown | Deferred pending native measurement | Changes runtime lifecycle; existing performance plan explicitly requires residency measurement first |
| Disable terminal's 5-second idle prewarm | Deferred pending current native measurement | Existing policy intentionally trades hidden shell residency for first-open latency; `terminal-panel-prewarm-idle-policy.md` labels historical 75–80 MiB / 1.5–1.7 s observations approximate surrogates, not fresh measurements. Do not report those as campaign savings or silently rewrite policy |
| Replace process snapshot deep proxies with raw state | Already optimized in current source | Current `ProcessManagerSurface.svelte` uses `$state.raw` for snapshots and taskbar PIDs; no applicable deep-proxy waste there |
| Remove snipping frame/crop deep clones | Already optimized | Coordinator already shares `Arc<Frame>` / `Arc<CroppedImage>` and borrows pixels outside lock |
| Eliminate PNG raw-pixel copy | Already optimized | Current encoder borrows pixels and preallocates encoded output |
| Encode clipboard bytes directly into HGLOBAL | Deferred higher-risk alternative | Staging overlap offers safer first win; avoid unsafe mutable-memory encoder redesign |
| Remove COM clipboard GetData copies | Rejected | Copies transfer ownership to clipboard consumers; not redundant owned data |
| Reuse speech input across inference calls | Rejected absent ownership evidence | Current inference API consumes the contiguous owned vector |
| Memoize all subtree counts | Deferred | Cycles/duplicates need explicit semantics; reusable count-only traversal is smaller and safer |
| Fix screenshot object-URL lifecycle leak | Rejected after inspection | Overlay/preview revoke on replacement/dismiss/unmount and stale result handling; do not confuse removable byte copy with URL leak |
| Clear gallery snapshots on close | Already optimized | Close clears captured image maps; replacement prunes missing members; visible capture requests bounded to 12 thumbnails |
| Remove editor initial/draft duplicate text | Deferred | Current dirty/conflict semantics need baseline; safe redundant ownership not established |
