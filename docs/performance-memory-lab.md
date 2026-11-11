# Runtime memory and performance lab

Living scratchpad for the memory-first optimization campaign started 2026-11-11.

Campaign status: **Complete within the authorized isolated-validation scope.** Fourteen production optimizations (A04–A17) have focused acceptance and independent review. Final check/build, 436 mounted tests, native type checking and isolated native fixtures pass. Repository-wide Node tests retain ten pre-existing failures; no new failure or registry omission remains. Whole-app memory and live native acceptance are not measured.

## Goal and evidence rules

Reduce retained runtime memory first; improve latency and CPU work second without removing features or weakening safety. Record measured results, not predicted percentage claims. Fixture allocation/retention, bundle sizes, and benchmark timings are not whole-application private bytes. Compare identical inputs and commands; retain regressions and unsuccessful experiments here.

No live shell launch is authorized in this campaign: AppBars, hooks, Explorer taskbar changes, native interaction, hardware, and actual WebView2 process-tree memory remain unmeasured. No dependencies, system configuration changes, or automatic feature removal.

## Metrics sheet

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

### Baseline provenance and pre-existing blockers

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

At campaign close, all fourteen grounded runtime candidates A04–A17 are implemented, measured and independently reviewed. Remaining leads below are already optimized, developer-only, unsafe without a product decision, or dependent on consent-gated native/model residency evidence. No justified candidate remains open within the current safety/evidence boundary. Future profiling can reopen this ledger; current fixtures cannot establish the largest remaining whole-app residency costs.

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
| Replace process snapshot deep proxies with raw state | Rejected for inspected surface | `ProcessManagerSurface.svelte` uses legacy assignment reactivity, not deep `$state` proxies; no applicable proxy waste there |
| Remove snipping frame/crop deep clones | Already optimized | Coordinator already shares `Arc<Frame>` / `Arc<CroppedImage>` and borrows pixels outside lock |
| Eliminate PNG raw-pixel copy | Already optimized | Current encoder borrows pixels and preallocates encoded output |
| Encode clipboard bytes directly into HGLOBAL | Deferred higher-risk alternative | Staging overlap offers safer first win; avoid unsafe mutable-memory encoder redesign |
| Remove COM clipboard GetData copies | Rejected | Copies transfer ownership to clipboard consumers; not redundant owned data |
| Reuse speech input across inference calls | Rejected absent ownership evidence | Current inference API consumes the contiguous owned vector |
| Memoize all subtree counts | Deferred | Cycles/duplicates need explicit semantics; reusable count-only traversal is smaller and safer |
| Fix screenshot object-URL lifecycle leak | Rejected after inspection | Overlay/preview revoke on replacement/dismiss/unmount and stale result handling; do not confuse removable byte copy with URL leak |
| Clear gallery snapshots on close | Already optimized | Close clears captured image maps; replacement prunes missing members; visible capture requests bounded to 12 thumbnails |
| Remove editor initial/draft duplicate text | Deferred | Current dirty/conflict semantics need baseline; safe redundant ownership not established |
