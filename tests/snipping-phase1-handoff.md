# Snipping Phase 1 test handoff

Remaining full approved feature now has its own RED contract/tests in
`tests/snipping-product-handoff.md`. User waived interactive gate as implementation
prerequisite only; manual desktop/mouse/AppBar/hardware evidence remains unverified.
Phase 1 history below is not full-feature acceptance evidence.

Status: current native13 GREEN after P2 fix; original14 previously GREEN and unchanged.
Earlier P2 RED evidence is retained below as history, superseded by final rerun.
Synthetic clipboard
was explicitly replaced; no shell launch/control, private desktop capture, save or
upload. Standalone clipboard E2E is established, not full feature/native shell E2E.

## Runnable RED evidence

Original preparation evidence (before implementation):

- `npm run test:node`: TypeScript compilation completed; four snip cases fail with
  `ERR_MODULE_NOT_FOUND` for `dist-tests/lib/snipSelection.js`. Full run: 1195 pass,
  5 fail, 1 skip, 3 todo. Fifth failure is unrelated existing
  `contextMenuVisualRegression.test.mjs:87` (missing combined icon CSS selector).
- `npm run cargo:test -- --test snipping_geometry --test snipping_session snipping`:
  exit 1, missing `src/snipping/geometry.rs` and `src/snipping/session.rs`.
  First attempt timed out at 120 seconds; second completed at 240-second timeout.
  This is compile-time RED, not executed Rust assertion evidence.
- After implementation, rerun these commands with fresh `dist-tests`; then run
  `npm run test:component` for the rendered acceptance additions listed below.
  No rendered snipping test exists yet, so existing component tests cannot prove it.

## Proposed pure TypeScript contract

`src/lib/snipSelection.ts`, emitted by existing `tsconfig.test.json`:

```ts
type Point = { x: number; y: number };
type Bounds = { width: number; height: number };
type Rect = Point & { width: number; height: number };
type Pointer = Point & { pointerId: number; button: number; isPrimary: boolean };
// State shape private; callers pass it back, never inspect its fields.
normalizeSelection(start: Point, end: Point, bounds: Bounds): Rect | null;
beginSelection(pointer: Pointer, bounds: Bounds): State | null;
moveSelection(state: State | null, pointer: Pointer, bounds: Bounds): State | null;
finishSelection(state: State | null, pointer: Pointer, bounds: Bounds):
  { state: State | null; selection: Rect | null };
cancelSelection(state: State | null, pointerId?: number): State | null;
```

Coordinates are monitor-local CSS pixels. Preserve fractions, normalize direction,
clamp to [0,width]/[0,height], reject nonfinite/empty bounds and zero-area results.
Foreign pointer move/up/cancel cannot complete or clear the original gesture.
Undefined cancel pointer means unconditional Escape/lost-capture cancellation.
Renderer must not replace an active state on a second pointerdown; rendered test pending.

## Proposed Rust geometry contract

Standalone pure module `src-tauri/src/snipping/geometry.rs`; no Tauri globals or Win32 calls.
`PhysicalRect { left, top, right, bottom: i32 }` and
`LogicalRect { x, y, width, height: f64 }` derive Copy/Clone/Debug; PhysicalRect also PartialEq.
All functions return `Result<_, E>` where E: Debug:

```rust
checked_rgba_len(width: u32, height: u32, max_bytes: usize) -> Result<usize, E>;
physical_rect_from_logical(monitor: PhysicalRect, scale: f64, local: LogicalRect)
    -> Result<PhysicalRect, E>;
crop_rgba(source: PhysicalRect, crop: PhysicalRect, pixels: &[u8], max_bytes: usize)
    -> Result<CroppedImage, E>;
// CroppedImage: width/height u32, pixels Vec<u8>, Debug.
```

Half-open physical edges. Logical conversion floors left/top, ceils right/bottom,
then adds the native monitor origin exactly once. Native caller supplies trusted
monitor metadata/scale; never accept renderer authority over display topology.
Reject outside/empty/overflow rectangles, invalid scale, truncated source and
allocation budget violations before allocation/indexing. Limit in tests is 128 MiB;
product may impose a stricter budget in addition. No downsampling.
Mixed-DPI vectors test arithmetic only, not actual OS DPI behavior.

Exact negative-origin oracle: source (-4,-2)..(0,1), crop (-3,-1)..(-1,1),
4x3 RGBA pixel IDs 0..11 -> 2x2 IDs [5,6,9,10] without source mutation.

## Proposed Rust session/authorization contract

Standalone pure module `src-tauri/src/snipping/session.rs`:

```rust
Caller { label: String, window_id: u64 } // Clone; concrete native instance ID
enum Action { Select, Cancel, Copy, Save, Close } // Copy
SessionManager<R>: Default // no R: Default/Debug requirement
begin(&mut self, resource: R, overlay: Caller) -> Result<u64, E>;
show_preview(&mut self, generation: u64, preview: Caller) -> Result<(), E>;
authorize(&self, generation: u64, caller: &Caller, action: Action) -> Result<(), E>;
cancel(&mut self, generation: u64) -> Result<(), E>;
finish(&mut self, generation: u64) -> Result<(), E>;
```

E: Debug. Nonreused generations; one session. Duplicate start fails without replacing
active session and drops incoming resource. Stale operations cannot affect new session.
Selecting: exact bound overlay can Select/Cancel, not Copy/Save/Close.
Preview: exact bound preview can Copy/Save/Close; overlay loses all authority.
Finish requires preview; cancel supports either phase. Terminal/drop paths release
owned resource once. Drop spy proves Rust ownership only, not native handles or images
held by windows/workers. `window_id` must be derived from Tauri-injected concrete caller,
not IPC payload; HWND reuse additionally requires fresh generation/instance binding.
Actual command injection/capability guards, owner start authorization, and revalidation
immediately before clipboard/file publication still require integration tests.

## Native feasibility handoff (mandatory E2E, not executed)

Owner: native feasibility implementer/leader. Separate consent already covers live
shell/AppBar, focus changes and synthetic clipboard replacement. Do not run until
leader releases the Phase 1 native gate. No autosave/upload or new dependencies.

Preferred affordance: opt-in bounded native harness using the same capture/window/
clipboard implementation as production, plus synthetic desktop target and clipboard
reader. Keep it out of normal startup; ask leader before introducing persistent
diagnostic surfaces. A browser mock or reading PNG export alone is weaker evidence.

1. Preflight: record build/revision, consent, monitor native rectangles, per-monitor
   scale, foreground HWND class/instance, work-area rectangles and Explorer taskbar
   visibility. Use synthetic grid/color target only; cover private desktop content.
   Restrict artifacts to dimensions, sampled synthetic pixel IDs, operation status,
   handle counts and timings. No real screenshots/clipboard content in logs.
   Set synthetic clipboard sentinel only after user removes valuable content; do not
   promise arbitrary clipboard restoration. Arrange graceful app exit and recovery.
2. Launch approved shell; compare AppBar/work area against baseline. Start snip from
   actual owning window. Verify overlay native bounds equal full monitor rectangle
   (not AppBar work area), receives focus, and is not present in frozen captured pixels.
   Move/change synthetic target after capture: selection still uses frozen frame.
3. Drag known rectangle four directions and across overlay boundaries. Read native
   captured dimensions and pixel IDs, compare to independent physical grid oracle.
   Exercise primary monitor at its actual DPI. Negative-origin and mixed-DPI native
   cases require corresponding real display topology; mark blocked if unavailable,
   never substitute simulated vectors for native evidence. Do not alter display
   configuration without further consent. Multi-monitor shell ownership is out of scope.
4. Escape, pointercancel/lost capture, overlay close and duplicate start: no preview,
   clipboard sentinel unchanged, no retained windows/image resources. Cancel/restart
   with delayed old completion; old generation cannot publish/close the newer session.
   Foreground restoration must target original still-valid window, never steal focus
   from a newly chosen target. Record actual foreground window, not UI inference.
5. Commit: overlay closes and preview appears without activating or stealing focus.
   Try Copy/Save/Close from wrong surface,
   stale overlay instance and old generation through actual invoke path; reject before
   side effects. Current bound preview Copy publishes native image once. Independent
   Win32 clipboard reader decodes image dimensions/pixel IDs; paste into an approved
   Windows image consumer if available. A mocked invoke success is not clipboard E2E.
6. Induce capture/preview-window creation/clipboard busy failures where harness can
   control them. Verify typed failure, unchanged sentinel before publication, released
   GDI DC/bitmap/global-memory handles and retryable session. Clipboard failure after
   EmptyClipboard is not generally transactional: record exact preservation boundary;
   do not assert arbitrary rollback unless implementation can establish it.
7. Repeat 20 start/cancel/commit/close cycles; warm up then compare process GDI/USER
   handle counts and retained image/session count, with bounded measurement tolerance
   declared beforehand. Counts cannot prove every release; pair with failure injection
   and native resource ownership review. Exit gracefully, verify restored work area/
   taskbar visibility and absence of orphan overlay/preview windows. Stop on mismatch.

Explicit Save only: once picker/save contract settles, cancel picker must write
nothing; approved temporary synthetic PNG path must round-trip exact pixels. No
autosave, persistent capture cache, upload or image-content logs. Do not exercise
arbitrary files/overwrite paths without leader approval.

Record every check as automated/manual/blocked with command, observation and reason;
required native boundary cannot be reported Complete based on Node/Rust unit tests.

## Remaining Phase 1 acceptance work

- Rendered overlay: real pointer capture; active second-pointerdown cannot reset;
  all drag directions/clamping; Escape/pointercancel/lostcapture; zero-area no invoke;
  exactly one generation-tagged selection invoke; stale events and unmount cleanup.
- Rendered preview: authorized generation only; image dimensions; Copy/Save/Close
  accessible controls, pending/duplicate-action gating, typed errors, picker cancellation,
  focus/close event lifecycle, no effects from stale settlements.
- Routing: actual `snip-overlay`/`snip-preview` resolution/loading tests when paths land.
- Native integration: real injected caller identity/capabilities, start-owner rejection,
  stage/worker generation rechecks, window creation failure unwind, native RAII failpoints,
  clipboard formats/ownership/failure boundary, save picker/write errors/no autosave.
- Mandatory native E2E above. Missing harness/product APIs block execution; available
  display topology may block DPI cases. No new dependency or product mutation authorized
  by this test handoff alone.

Review: self-reviewed numeric oracles and test-only scope. Independent code-reviewer
delegation attempted but blocked by subagent depth limit; leader owns independent review.

## Earlier resumed Phase 1: pure GREEN and native clipboard RED

Historical pre-driver evidence; superseded for clipboard readiness by latest section.

Fresh `npm run test:node` rebuilt `dist-tests`: all four snip tests pass; full suite
1199 pass, 1 fail, 1 skipped, 3 todo (1204 tests). Unrelated existing failure remains
`contextMenuVisualRegression.test.mjs:87`, missing combined CSS icon selector.
`npm run cargo:test -- --test snipping_geometry --test snipping_session snipping`:
five geometry plus five session cases passed, exit 0. No native assertion implied.
`npm run cargo:test -- --test snipping_clipboard_native`: compilation succeeds;
0 passed, six ignored, exit 0 (readiness/build evidence only).
With `JASONSHELL_SNIP_CLIPBOARD_TESTS=1`, the exact readiness command below ran:
0 passed, one failed, five filtered out, exit 1. Failure is
`bounded JSON native report missing; clipboard driver API unavailable: Disconnected`.
Current binary rejects missing synthetic-native consent before native initialization;
readiness invocation supplied only `--clipboard-test-protocol`. No sentinel was set,
no clipboard/image/window case ran, and temporary consent env was restored afterward.

Native evidence supplied by leader/fixer, not independently rerun by test owner:
one 2560x1440 monitor at 125% scale; physical synthetic capture/freeze,
nonactivation and prior-preview exclusion passed. Initial reserved work-area
(0,23)..(2560,1404) changed to full monitor: failure remains, cause unproved.
Later full-monitor baseline stayed full: cannot prove reserved AppBar preservation.
Already-running shell PID 6724 may release AppBars for external fullscreen targets.
Do not bypass the invariant, terminate/control that shell, or launch another shell.
Leader must orchestrate reserved-baseline observation before clearing that gate.

### Opt-in clipboard test driver contract (implementation-owner negotiation)

`src-tauri/tests/snipping_clipboard_native.rs` is test-owned. It calls the actual
standalone `snip-feasibility` binary and independently reads Windows CF_DIB/optional
PNG. All six cases are ignored by default because they can replace clipboard;
ignored is not passed. No producer module/API mocks. No full-monitor windows.
Readiness protocol is verified BEFORE even setting the synthetic sentinel.

Required minimal CLI affordance in the native binary (production owner only):

- `--clipboard-test-protocol`: no consent argument required because it must have
  zero effects; print exactly one JSON line
  `{"protocol":"snipping-clipboard-test-v1","sideEffects":false}`, exit 0.
  Do not initialize windows/OLE, publish clipboard, capture desktop or launch shell.
- `--consent-synthetic-clipboard-test --clipboard-test-case CASE`: isolated STA,
  no visible/full-monitor window, no capture/shell action. Report metadata as one
  bounded JSON line (<=4096 bytes), exit 0 for expected injected outcomes. Unknown
  cases fail closed. Synthetic input is 2x2 opaque RGBA, top-left row
  `[11,22,33,255]`, `[44,55,66,255]`; next row `[77,88,99,255]`, `[111,122,133,255]`.
  Required CF_DIB BI_RGB 24/32bpp, bottom-up or top-down; optional PNG must agree.
- Cases `invalid-image`, `budget`, `allocation`, `ole-set-rejected`: exercise actual
  publication rejection paths (native dependency failpoint for allocation/commit),
  not fabricated return objects. JSON `case`, `status:"rejected"`, `committed:false`,
  `durable:false`. Parent sets sentinel beforehand and reads it after child exit.
  For `ole-set-rejected`, injected OleSetClipboard failure must occur before native
  mutation; real busy rejection is separate and not substituted by injection.
- `real-busy`: parent holds OpenClipboard throughout child attempt. Actual publication
  must return `code:"clipboard-busy"`, rejected/committed:false without clearing.
- `success`: use actual IDataObject/OleSetClipboard/OleFlushClipboard; report
  `status:"committed"`, `committed:true`, `durable:true`; exit and release local STA
  resources. Independent test reader reads image only AFTER producer has exited.
- `flush-after-commit --hold-until-reader`: commit actual OleSetClipboard, inject
  subsequent OleFlushClipboard failure. Report `status:"committed-warning"`,
  `code:"clipboard-committed-not-durable"`, committed:true, durable:false. Keep STA
  owner/message pump alive while parent exercises actual delayed GetData. Await
  stdin `release-after-observation` for at most five seconds; retry real flush,
  report committed/durable:true, then exit. Timeout must not masquerade as durable.
- `repeat-cleanup-20`: warm native clipboard/OLE path once, then twenty actual
  successful commit+flush/release and twenty precommit allocation/commit-failure
  attempts. Record `iterations:20`, `successfulPublications:20`,
  `failedPublications:20`, `before`/`after` native `gdi`, `user`, `processHandles`.
  After each local owner retires, owned `liveObjects`, `liveGlobalAllocations`,
  `liveStagedBytes` must return to zero; report final after values. Counters must
  observe actual RAII acquisition/release, not estimates. OS clipboard-owned durable
  storage is not a local leak. Predeclared warmed native handle tolerance: no growth.
  If a platform retains additional handles, report blocker before changing tolerance.

These are a bounded test-facing proposal, not authority to introduce permanent product
diagnostic surfaces. Production owner may choose an equivalent test-only affordance;
settle protocol before executing mutation cases. Existing EmptyClipboard primitive
cannot satisfy this final publication contract. Do not adopt it for product Copy.

Commands (run serially, valuable clipboard removed, explicit synthetic consent):

```powershell
npm run cargo:test -- --test snipping_clipboard_native
# Compiles tests; six ignored. NOT native evidence.
$env:JASONSHELL_SNIP_CLIPBOARD_TESTS = '1'
npm run cargo:test -- --test snipping_clipboard_native snipping_clipboard_protocol_is_available -- --ignored --test-threads=1
# RED readiness only; handshake missing => failure BEFORE sentinel/desktop effects.
npm run cargo:test -- --test snipping_clipboard_native -- --ignored --test-threads=1
# Run only after owner implements protocol. Replaces synthetic clipboard; no restoration promise.
Remove-Item Env:JASONSHELL_SNIP_CLIPBOARD_TESTS
```

Reader compares pixels privately, never logs unexpected text/image content. Test
driver subprocesses and reports have five-second deadlines; cleanup may terminate
only the test's own standalone child, never JasonShell or another process. Test owner
does not run mutation cases while protocol is absent. A postcommit warning is not
rejection; prior sentinel must not be claimed restored after actual commitment.

### Hardware/unattended boundary

Available observation: one 2560x1440 monitor, 125% DPI, no demonstrated negative
origin or mixed-DPI pair. Negative-origin/mixed-DPI native capture requires real
matching topology, including at least two unequal-scale monitors for mixed DPI;
currently unavailable/unestablished. Synthetic geometry tests are not substitute.
Do not change OS display layout/DPI without orchestration/consent.

Unattended standalone harness can establish synthetic pixels, freeze/exclusion,
window-show nonactivation, independent native clipboard formats/persistence and
bounded cleanup after driver exists. It cannot establish actual mouse boundary
capture, release outside monitor, Escape delivery or clickable-preview usability
without a person during `--interactive`. Native synthetic SendInput is not human
pointer evidence. Existing harness merely prints overlay observations; missing
down/up/moves/Escape/out-of-bounds evidence must block those manual claims.
Approved external image consumer paste requires consumer availability and an
orchestrated actual paste; independent Win32 read does not establish Teams/Explorer/
other consumer interoperability. No consumer availability demonstrated in this pass.
Real Tauri injected-caller guards and reserved AppBar coexistence remain outside
standalone clipboard tests and require leader-controlled native shell E2E.

## Latest native clipboard E2E: parent message pumping and six GREEN cases

Scoped source reread: clipboard.rs now stages STA IDataObject CF_DIB+PNG, commits via
OleSetClipboard, materializes via OleFlushClipboard, and retains/pumps a committed
owner on flush warning. clipboard_driver.rs implements the negotiated subprocess
protocol. No production edits made by test owner.

Harness defect: parent creates the sentinel-owning hidden HWND on the test thread,
then originally blocks that thread in `recv_timeout` while the child calls
OleSetClipboard. Native clipboard owner notification can require that thread to
service messages. Prior producer-exit attempts timed out at five seconds despite
direct producer success. Test now polls the report channel and child exit while
calling PeekMessage/TranslateMessage/DispatchMessage on the HWND-owning test thread.
Deadlines remain five seconds, with five-millisecond polling; no timeout extension
or ignored invariant. Exact blocking Windows message was not traced, so do not
claim a specific WM_DESTROYCLIPBOARD/OLE callback as an independently observed cause.

First complete serial mutation run after pumping change: all six cases passed in
0.42 seconds, exit 0. Independent reader still accesses clipboard only after the
success producer has exited. Warning case reads while owner lives, requests real
flush recovery, waits for exit, then reads again. Pixel oracles are unchanged;
PNG is now REQUIRED (current producer contract), with 2x2 dimensions/RGB pixel order
and opaque alpha checked independently alongside native DIB.
Final rerun with mandatory PNG/alpha assertions: six passed, zero failed/ignored,
0.34 seconds, exit 0. Independent review delegation was retried and blocked by
subagent depth limit; leader review remains required.

Exact command (granted synthetic consent; env restored to its prior value afterward):

```powershell
$previous = $env:JASONSHELL_SNIP_CLIPBOARD_TESTS
try {
  $env:JASONSHELL_SNIP_CLIPBOARD_TESTS = '1'
  npm run cargo:test -- --test snipping_clipboard_native -- --ignored --test-threads=1
} finally {
  if ($null -eq $previous) {
    Remove-Item Env:JASONSHELL_SNIP_CLIPBOARD_TESTS -ErrorAction SilentlyContinue
  } else { $env:JASONSHELL_SNIP_CLIPBOARD_TESTS = $previous }
}
```

Established boundary claims:
- Actual independent process publication/exit and Win32 CF_DIB/PNG image persistence.
- Sentinel preservation for invalid image, budget, injected allocation/set rejection
  and actual OpenClipboard busy rejection (not arbitrary prior-format restoration).
- Committed-not-durable warning is not rejection; live owner serves image; successful
  subsequent real flush yields image readable after producer exit.
- Twenty real publish/flush cycles plus twenty injected precommit failure attempts;
  reported actual RAII-owned object/global/staged-byte counters return to zero;
  warmed native GDI/USER/process-handle counts do not grow.

Limits: these do not prove arbitrary image sizes/alpha, every native OOM mode,
permanently failing real flush/shutdown recovery, every consumer's paste semantics,
actual mouse/keyboard interactions, Tauri caller injection/capabilities, reserved
AppBar coexistence or unavailable mixed-DPI/negative-origin hardware. No shell process
was controlled. Native report/child waits retain deadlines; synchronous Win32 reader
calls themselves rely on Windows servicing the producer, not a separate watchdog.
Original 14 tests were not rerun: only native test harness/handoff changed.
Leader owns independent review; no production defect demonstrated by this run.

## Current process-isolation patch: 11 native regressions and focused gates

Reread actual clipboard.rs, clipboard_process.rs, clipboard_driver.rs, native windows,
both helper/harness binaries, FEASIBILITY.md and package scripts. New clipboard owner
is process isolated: binary stdin, metadata-only stdout, 2s operation deadline,
500ms shutdown plus bounded cleanup. Only private retained child/job handles may be
terminated. Normal nondurable owner is retained for application lifetime; uncertain
commit is not rejection. No product integration/packaging proof inferred.

Added five executable opt-in acceptance cases (all remain ignored without consent):
- Nonaligned 2x2 PNG: independently parse IEND, assert exact stored-deflate logical
  length 86, independently decode original pixel oracle after producer/helper exit,
  inspect ALL GlobalSize bytes and require every exposed byte after IEND to be zero.
  Live delayed-owner reader also checks full HGLOBAL bytes. DIB trailing bytes are
  inspected too. No source-string padding assertion or producer decoder reused.
- Permanent helper pre-set stall: must complete in bounded time, report
  publication-unknown with PRESENT committed:null/durable:null, never precommit
  rejection or claimed old-content preservation; helper exit/writer drainage required.
- Permanent acknowledged postcommit/preflush stall: committed-warning,
  clipboard-durability-lost, committed:true/durable:false; bounded private cleanup.
- Permanent acknowledged nondurable shutdown stall: same truthful loss distinction;
  500ms shutdown policy exercised; bounded cleanup, no indefinite owner teardown.
- Real hidden native WM_CLOSE invalidates authority for stale operations. Replacement
  survives old wrapper drop. Report actual handle reuse, never fabricate forced reuse.

Timeout assertions preserve published driver tolerances: operation <=3000ms,
shutdown <=1000ms; actual operation stalls >=1800ms, shutdown stall >=450ms;
whole subprocess <=4500ms. Existing outer report/exit waits remain five seconds and
pump only the test's owning thread. Failpoint declares a permanent helper stall at
native boundary, NOT an actual OS COM hang. Private child exit and writerDrained are
reported by actual ProcessOwner; tests do not inspect unrelated processes by PID.

Important test-premise correction, not a source defect: first new run was 10 pass/
1 fail because it required allocator rounding for an 86-byte logical PNG. Windows
GlobalSize exposed exactly 86 bytes both after durable producer exit and from live
delayed GetData. Being nonaligned does not guarantee observable allocation padding.
Removed that invalid platform requirement, not zero-byte/pixel/deadline assertions.
Final test always checks every exposed byte and reports logical/global/padding lengths.
Final observations: logicalBytes=86, globalBytes=86, paddingBytes=0,
roundedPaddingCoverage=false. Therefore actual positive-padding disclosure branch is
NOT established on this host. Do not claim zeroed rounded storage was behaviorally
proved. If leader requires positive-padding evidence, implementation owner must
provide a narrow opt-in allocation failpoint that deliberately obtains a larger
HGLOBAL than logical content while exercising the SAME Global::copy/GetData path;
test owner cannot introduce that production affordance. Reader must assert exact
logical PNG length, compare pixels and inspect the entire independently read HGLOBAL.

Exact current native command (same prior-env-restoring wrapper as above):

```powershell
$env:JASONSHELL_SNIP_CLIPBOARD_TESTS = '1'
npm run cargo:test -- --test snipping_clipboard_native -- --ignored --test-threads=1 --nocapture
# Restore the prior env value afterward, as in wrapper above.
```

Final native result: 11 passed, 0 failed, 0 ignored; 5.70s; exit 0. Supplied six
clipboard boundary cases are current-patch GREEN, not reused pre-patch evidence.
Metadata-only observations:
- helper-hang-operation: publication-unknown, operation 2007ms, shutdown 0ms.
- helper-hang-flush: committed-warning/durability-lost, operation 2006ms, shutdown 0ms.
- helper-hang-shutdown: committed-warning/durability-lost, operation 11ms, shutdown 510ms.
- All three: forcedShutdown=true, helperExited=true, writerDrained=true.
- Hidden native window check: handleReused=false. Destruction invalidation and
  replacement survival passed; real reused-HWND scenario remains unestablished.

Original14/focused automated validation rerun:
- `npm run test:node`: fresh compile, core4 pass; full 1204 tests: 1199 pass,
  1 existing unrelated CSS-selector fail, 1 skipped, 3 todo. Existing failure remains
  contextMenuVisualRegression.test.mjs:87; no unrelated repair performed.
- `npm run cargo:test -- --test snipping_geometry --test snipping_session snipping`:
  core10 pass (5 geometry/5 session), exit 0.
- `npm run check`: zero errors/warnings, exit 0.
- `npm run build`: passes; existing XML mixed-import and chunk-size warnings.
- `npm run test:component`: 7 files/34 tests pass; existing jsdom canvas diagnostics.
- `npm run cargo:check`: passes, exit 0. Full repository cargo:test was not run;
  touched native evidence is the focused integration/core commands above.

No production defect demonstrated. Remaining evidence limits: positive-padding
allocation branch, real HWND reuse, actual permanently stuck OS COM call (injected
helper stalls only), packaging/bundled helper selection, Tauri authorization and
UI routing, reserved-AppBar coexistence, real pointer/consumer paste, negative-origin/
mixed-DPI hardware. No interactive flows, shell launch/control, private image logs,
files, save or upload performed. Clipboard retains synthetic content, not restored
arbitrary prior user formats. Independent review remains leader-owned due known
subagent depth limit; source-author self-review is not substituted for that gate.

## P2 reviewer durability regression: runnable RED for implementation owner

Historical RED preparation; corrected source/driver now pass final rerun below.

Reviewer identified clipboard_process.rs flush stopped path (83-88) and timeout
mapping (117-119) discard known durability. Test owner reread those actual APIs.
Only native tests/handoff changed; no production edit or helper/shell bypass.

New native API regression directly includes the real snipping modules and invokes
public ProcessOwner with the actual separate `snip-feasibility` private helper:
successful synthetic 2x2 publish -> committed/durable true -> successful shutdown
with helper exited/writer drained -> redundant flush. Independent Win32 DIB+PNG pixel
reader executes AFTER helper exit and BEFORE failing metadata comparison, establishing
persisted pixels remain correct. Expected post-shutdown flush metadata is exactly
`{"status":"committed","committed":true,"durable":true}`.

Exact RED command, inside prior-env-restoring consent wrapper:

```powershell
$env:JASONSHELL_SNIP_CLIPBOARD_TESTS = '1'
npm run cargo:test -- --test snipping_clipboard_native snipping_clipboard_known_durable -- --ignored --test-threads=1 --nocapture
```

Observed exit 1: 0 pass, 2 fail, 11 filtered; 0.08s:
- `snipping_clipboard_known_durable_shutdown_then_flush_never_reports_loss`:
  actual `{"code":"clipboard-durability-lost","committed":true,"durable":false,
  "status":"committed-warning"}` versus expected committed/durable:true.
  Assertion at native test line 444: "flush after successful durable shutdown must
  preserve acknowledged commitment/durability". This is an executed source defect
  reproducer, NOT a clipboard persistence failure; independent pixels already passed.
- `snipping_clipboard_known_durable_redundant_timeout_never_reports_loss`:
  "native report stream closed before report". Driver does not recognize the requested
  case; this is missing test affordance RED, NOT evidence of an executed native timeout.

Required exact negotiated driver hook (implementation owner only):
`--consent-synthetic-clipboard-test --clipboard-test-case durable-redundant-flush-timeout`.
Run actual native successful publication/flush first, preserving confirmed durability.
Then arm helper response unavailability for a redundant flush (or a narrow test-only
transport failure at its send/receive dependency), call NORMAL ProcessOwner::flush.
If corrected API short-circuits known durable state without another IPC, the armed
fault may remain unused: report that explicitly rather than fabricate an observed
timeout. Otherwise inject missing request-2 reply and enforce existing 2s operation/
bounded cleanup policy. Do not fake native publication, weaken binary opcode/flag
validation or introduce failure into the INITIAL successful materialization.

One metadata-only report must contain case, initialCommitted:true,
initialDurable:true, status:"committed", committed:true, durable:true,
writerDrained:true, operationMs<=3000, and
`failpoint:"redundant-flush-response-unavailable-after-confirmed-durable-publication"`.
Flush preservation must not depend on helper still being reachable. Shutdown/exit
before driver exits; test independently reads matching DIB+PNG pixels afterward.
Any transport/timeout result must preserve known durable acknowledgement, not label
it loss/unknown. Previously nondurable/unknown cases must retain their distinctions.

Only these two focused new cases rerun; previous11/core14 untouched. All cases remain
opt-in, serial and synthetic. No shell control, private images, file/save/upload.
Leader/fixer owns source correction and driver hook; rerun both new RED cases plus
native11 after fix. Existing reserved-AppBar/manual/hardware/padding/reuse review
limitations remain unchanged. Independent review gate remains leader-owned.

## Final current native13 rerun after P2 correction

Reread current ProcessOwner::flush/timeout, driver hook and FEASIBILITY.md. Known
durability short-circuits before stopped/helper checks; timeout mapping preserves
acknowledged durable state. Test now explicitly requires faultUsed=false,
shortCircuited=true, helperExited=true and writerDrained=true for the redundant case.
Removed stale missing-driver comments, not behavior/pixel/deadline assertions.
No production edit.

Exact current command, inside the prior-env-restoring wrapper above:

```powershell
$env:JASONSHELL_SNIP_CLIPBOARD_TESTS = '1'
npm run cargo:test -- --test snipping_clipboard_native -- --ignored --test-threads=1 --nocapture
```

Observed exit 0: **13 passed, 0 failed, 0 ignored; 5.82 seconds**. Consent env restored.
Independent DIB+PNG pixel oracles still run after producer/helper exit for both P2
cases. Native publish -> durable shutdown -> later flush retains committed/durable=true.
Armed redundant fault: faultUsed=false, shortCircuited=true, operationMs=0; do not
claim an actual redundant OLE/IPC timeout occurred. Avoidance of unnecessary transport
is the behavior established, not fabricated timeout execution.

Other current-patch metadata: permanent pre-set unknown 2008ms; acknowledged
postcommit loss 2008ms; nondurable shutdown loss 511ms (publish 11ms). All report
helperExited/writerDrained true. PNG logical/global bytes remain 86/86, positive
padding branch unestablished. Hidden replacement handleReused=false, not real forced
reuse evidence. Existing six clipboard boundary cases and five source-defect
regressions all pass; no source failures observed.

Original14/focused checks were not rerun this request: only native test assertions,
comments and handoff changed; prior results recorded above. Review remains
leader-owned. Reserved-AppBar, manual pointer/consumer paste, unavailable mixed-DPI/
negative-origin hardware, positive padding, real HWND reuse and product integration
gates remain open. No shell control, private desktop/image logs, save or upload.
