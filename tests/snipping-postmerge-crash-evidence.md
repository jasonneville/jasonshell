# Post-merge capture/cancel crash: bounded automated evidence

User report: whole application exits after first capture, or sometimes cancellation.
No confirmed root cause or native reproduction was established by this test work.

## Coordinated read-only native investigation update

The investigation owner subsequently reported matching native crash artifacts;
the test owner did not execute the application or independently read a dump:

- Application Error 1000, local 2026-11-14 12:27:41.624: `jason-shell.exe`,
  module `jason-shell.exe`, exception `c0000005`, RVA `0x10ef004`.
  Follow-up 12:27:42.620: `c000041d`, same RVA. Another pair was recorded at
  12:27:14.980 / 12:27:15.962. The initial no-events report was corrected because
  the records use unnamed Data nodes rather than the parser's named fields.
- Existing release artifact `jason-shell.exe` (machine-local location omitted)
  matches event PE timestamp `6af89d2d`; CodeView PDB identity is
  `53838C6C-D1AF-42C8-BE7F-40F36D5191C6`, age 1. Investigation owner reports DIA
  `loadAndValidateDataFromPdb` succeeded using already-installed tooling.
- Fault RVA is the first `mov (%rcx),%rcx` of a shared COM wrapper beginning
  `0x10ef000`, followed by vtable slot `0x60`. Identical-code folding gives it a
  public `ClientCertificateRequestedEventArgs::Handled` alias; that alias does
  **not** identify a certificate event as the actual operation.
- PDB maps potential callers `0x108f6f0` and `0x108f878` to
  `wry::webview2::InnerWebView::parent_subclass_proc`. Matching callback binary
  routes `WM_ENTERSIZEMOVE` (`0x231`) and focus handling to raw refdata use without
  a null guard. Wry 0.54.4 source `webview2/mod.rs:1219–1277` reportedly has
  unguarded SIZE/FOCUS/MOVE dereferences, while DESTROY releases refdata and sets
  subclass refdata null. Installed versions: Tauri 2.10.3, runtime-wry 2.10.1,
  Wry 0.54.4, Tao 0.34.8.

This is strong evidence for a dependency callback-lifetime defect compatible
with the user crash. Without actual crash stack/register evidence it does not
prove the faulting callback caller, RCX value or exact message sequence. The
safe Coordinator tests below cannot exercise this raw COM/WM lifetime edge.

The smallest relevant native regression would use production NativeWindowPort
helper creation/focus, complete or cancel/destruction, then repeated helpers,
with independent native UI and renderer drains plus observed DESTROY and later
focus/size/move delivery. It must distinguish process survival from worker
completion. Such a fixture remains opt-in and **unexecuted/unauthorized**; no
claimed native pass or artificial pure-model RED substitutes for it. Dependency
patching requires separate scope approval and was not performed by test owner.

## Scope and new checks

### Actual vendored callback RED now executable

The proposal below was superseded by a narrower test-owned **verbatim-source
facade**, requiring no production testability edits. The approved unmodified
published crate is now at `src-tauri/vendor/wry`. The test reads its real
`parent_subclass_proc`, extracts the entire function verbatim (no body rewrite),
and compiles it inside a small standalone Rust test executable. Only Win32/COM
names and the surrounding `parent_bounds` boundary are test facade symbols. The
vendored source remains intact. This is actual-source-backed execution under a
shim, **not** execution of the linked Wry crate against Windows/COM.

Artifacts:

- `src-tauri/tests/snipping_wry_retirement.rs`: unique actual-callback extraction,
  bounded compile/run child, requires both case bodies execute, preserves logs.
- `src-tauri/tests/fixtures/wry_parent_subclass_facade.rs`: no-OS/no-COM facade,
  injected synchronous controller-release reentry, exact retirement assertions.

During fake controller Drop, the facade dispatches WM_NULL through the currently
registered **actual callback**. WM_NULL is deliberate: it exercises reentry into
the actual default arm without dereferencing a controller while its Drop has an
exclusive reference. Using focus/move on that in-Drop object would itself be
undefined behavior; this harness does not do that. Callback authority during
release is the ordering oracle. After removal, facade-delivered size/focus/move
and repeated destroy messages must not reach controller/bounds/position operations.

Command: `npm run cargo:test -- --test snipping_wry_retirement -- --nocapture`.

**Observed RED before behavioral patch:** wrapper 0 passed / 1 failed; child
**0 passed / 2 failed / 0 ignored**. Both WM_DESTROY and private destroy fail:
`COM release synchronously reentered still-authorized ACTUAL callback` with
observed reentry **1**, expected **0**; trace
`["release", "default", "set", "default"]`. The initial harness run also exposed
test mutex poisoning between failures; assertions were moved outside the lock,
then the rerun independently reproduced the same ordering failure in both cases.
No assertion was weakened and no behavioral dependency edit was made by test owner.

Log: `C:/Users/jnev1/AppData/Local/Temp/opencode/snipping-wry-retirement-red-isolated.log`.
Generated source/compile/run artifacts:
`C:/Users/jnev1/AppData/Local/Temp/opencode/wry-callback-retirement-20668/`.

The requested fix is the official upstream removal-before-drop backport from
Wry PR1795, commit `3fbf592feab29ffb269778a6f5573746a2f25a4a`, not null guarding
alone. Test expects callback removal before release, no reentrant delivery,
exactly one controller release, and no installed null-data callback afterward.
The facade models successful removal; real RemoveWindowSubclass failure, native
COM scheduling and app crash resolution remain outside its evidence boundary.
Native E2E remains required, unauthorized and unexecuted.

### Official backport GREEN

After the native owner applied only upstream removal-before-drop in the vendored
callback, test owner ran:

`npm run cargo:test -- --test snipping_wry_retirement --test snipping_coordinator --test snipping_native_registry -- --test-threads=1 --nocapture`

All passed: **59 coordinator**, **20 registry**, **1 actual-callback wrapper**.
The wrapper asserts both child cases actually execute: **2 passed, 0 failed,
0 ignored**. Thus the identical callback-source harness went RED before the
behavioral edit and GREEN after it. No test assertions were changed between the
independent RED run and this GREEN run. The facade observes removal before
release, no synchronous reentrant callback delivery and exactly one release.

Log: `C:/Users/jnev1/AppData/Local/Temp/opencode/snipping-wry-retirement-green.log`.
This closes the bounded dependency ordering regression only; it does not establish
that the reported live app crash is resolved. Real capture/cancel native E2E is
still required and was not executed. Production patch/integrity/cargo-check
ownership remains with the native implementation owner.

### Final backport validation gate

Test owner independently repeated the original RED command against the applied
backport: `npm run cargo:test -- --test snipping_wry_retirement -- --nocapture`.
PASS wrapper **1/1**, requiring child **2/2**, with unchanged assertions.

Additional current gates:

| Command | Observed result |
| --- | --- |
| `npm run cargo:test -- --tests --bins` | PASS **1,297 passed, 0 failed, 30 ignored**, 29 top-level summaries |
| Runtime include/inventory inside that gate | All **23** actual-runtime bodies run; inventory wrapper verifies executed summary, not listing alone |
| Coordinator / registry / save / geometry / codec inside that gate | **59 / 20 / 4 / 5 / 11** passes respectively; codec has one encode-only timing test ignored |
| `npm run test:component -- tests/components/snipping-barrier.test.ts tests/components/snipping-product.test.ts` | PASS **58/58**, 2 files |
| `npm run cargo:check -- --offline` | PASS |
| `npm run check` | PASS, 0 errors/warnings |
| `npm run build` | PASS |
| `npm run cargo:test` | FAIL before execution: `stack_text_probe`/`stack_text_p03` proc-macro compilation needs missing `dist-text-probe`/`dist-p03-experiment` frontend bundles |

No ignored native opt-in fixture was enabled. The totals include shared source
include bridges and are test-body counts, not independent behavior coverage.
Full Node/components were not repeated for this native dependency-only backport;
the focused production snipping components were repeated as shown. No current
Node baseline claim is inferred from unrun suites.

Dependency selection was checked read-only with
`cargo tree --manifest-path src-tauri/Cargo.toml --offline -i wry`: local
`src-tauri/vendor/wry` **0.54.4** → runtime-wry **2.10.1** → Tauri **2.10.3**.
Manifest/lock diff contains the local patch declaration and removal of Wry's
registry source/checksum only; no package-version changes were present.
Production owner separately owns archive integrity and exact upstream patch
provenance. Crash artifact/PDB metadata above was verified by the native
investigation owner, not independently reconstructed by this test harness.

Logs in the approved temporary evidence directory:
`snipping-backport-final-{facade,rust-tests-bins,components,cargo-check,check,build,standard-cargo}.log`.

**Residual gap:** actual native capture/cancel/process-survival E2E remains
required, unauthorized and unexecuted. GREEN facades establish the upstream
retirement-order repair under stubbed OS/COM symbols; they do not certify the
live crash is eliminated or prove the user crash's actual register/caller state.

### Earlier callback seam proposal (superseded, not implemented/executed)

Test owner directly read installed Wry 0.54.4 `src/webview2/mod.rs:1210–1297`
and `Drop:70–77`. The unchecked controller dereferences and destroy-then-null
ordering are present. Raw callback invocation with HWND zero is **not** currently
a valid safe harness: its normal return calls `DefSubclassProc`, whose documented
input is the window being subclassed, and WM_SIZE calls `GetClientRect` before
controller dereference. A child process contains a fault but does not make these
OS API preconditions valid. No such raw invocation was executed.

Smallest proposed production-owned test seam in the approved local dependency
repair (names illustrative, not an extra application API):

1. Keep the real `extern "system" parent_subclass_proc` signature unchanged.
2. Have it call a private `parent_subclass_dispatch` containing the **one actual
   callback implementation**, with two injected edges: `next: FnOnce() -> LRESULT`
   and `bounds: FnOnce(HWND) -> Result<PhysicalSize<i32>>`. Production supplies
   closures calling `DefSubclassProc` and `Self::parent_bounds`. Every existing
   default-return path must use `next`; WM_SIZE must use `bounds`. Avoid a second
   model, renderer-facing entry point, feature-permission change or global hook.
3. Put test-owner inline tests inside that same patched dependency module, so
   they can call the private production dispatcher without exposing internals.
   Null refdata calls use HWND zero only as an inert typed token; supplied edges
   return a sentinel or fail if touched. No WebView/controller/COM object is
   constructed and no real native default/bounds routine runs.

Required null-entry cases: non-minimized WM_SIZE, WM_SETFOCUS, WM_ENTERSIZEMOVE,
WM_MOVE, WM_MOVING, repeated WM_DESTROY and the private PARENT_DESTROY_MESSAGE.
Assert normal sentinel return, exactly one next-dispatch, zero bounds calls and
no controller access. Repeat the message sequence to check post-retirement
idempotence. Use exact case inventory and one bounded child per message because
the unguarded focus/move cases can abort rather than unwind. Tests must compile
the exact locally patched Wry crate selected by Cargo, not a hand-written copy
of its callback or a separate application policy helper.

RED feasibility: first apply only the seam (callback logic unchanged) in an
isolated test build; focus/move null-entry children should fail/abort, and SIZE
should fail a bounds-edge sentinel assertion. Then apply the null guard and
require all cases to return normally. This is a proposed evidence path, **not an
observed RED**. Staging the seam/patch belongs to the native implementation owner;
test owner did not change the installed registry source, manifest or dependency.
If official upgrade is selected instead of a local patch and provides no such
seam, pure actual-callback execution is unavailable without further scoped
dependency testability work; do not substitute a mirrored model.

Limitation: null-entry tests prove safe behavior *after data is null*, not that
actual controller release publishes retirement before COM Drop reentry. Testing
that ordering needs an additional shared production release helper with injected
refdata-update/detach and drop edges, or authorized native execution. A test-only
fake retirement model would not establish it. The native owner should choose
this only if the approved fix changes that ordering; null-entry coverage alone
must not be presented as a complete teardown-race or real crash reproduction.

API documentation consulted: Microsoft Learn `DefSubclassProc` and
`SetWindowSubclass` (both require the subclassed window; helper callbacks are
not reference-counted, and cross-thread subclassing is prohibited). Context7
lookup was attempted but returned monthly quota exhaustion; official Microsoft
documentation was fetched read-only instead.

<https://learn.microsoft.com/en-us/windows/win32/api/commctrl/nf-commctrl-defsubclassproc>

<https://learn.microsoft.com/en-us/windows/win32/api/commctrl/nf-commctrl-setwindowsubclass>

`src-tauri/tests/snipping_coordinator.rs` uses the actual production Coordinator,
tiny synthetic monitor pixels and injected WindowPort/clipboard adapters only.
Four new tests add evidence beyond the existing single-replacement cases:

- `lifecycle_repeated_complete_replace_complete_survives_synchronous_close_callbacks`:
  eight captures/replacements, one live latest preview, one automatic publication
  per completion, retired generation calls have zero effects, exact-once cleanup.
- `lifecycle_repeated_complete_replace_cancel_restores_preview_through_reentrant_cleanup`:
  eight replacement/cancel cycles retain the exact original preview and do not
  republish clipboard; closed overlay calls cannot affect the restored generation.
- `lifecycle_repeated_cancel_retry_then_complete_cleans_each_resource_once`:
  eight cancel/retry cycles followed by successful completion/dismissal.
- `lifecycle_pending_create_late_return_cannot_publish_or_dispose_newer_generation`:
  pause WindowPort creation, expire old generation, start/arm newer generation,
  release old creation; late result closes without publishing old context or
  changing the newer window set, token or resources.

The first three execute under a five-second bound. The close observer runs
synchronously after adapter resource removal and reenters production inspection,
context lookup, cancellation and expiry; it checks closed callers remain denied.
Callbacks are injected timing probes, not a simulated explanation of the crash.
No coordinator or native lifecycle implementation was duplicated. Resource drop
and close records must each occur once per disposed resource.

## Result

Command:

`npm run cargo:test -- --test snipping_coordinator --test snipping_native_registry -- --test-threads=1`

Result: coordinator integration **59 passed, 0 failed, 0 ignored**; registry
integration **20 passed, 0 failed, 0 ignored**. The former includes the existing
23 actual-runtime cases and 10 image-concurrency cases; the latter includes 10
image-concurrency cases too. These overlapping include-bridge bodies are not
additional independent native acceptance evidence.

All four new lifecycle tests passed on their first completed run. **No RED crash
regression was established.** Existing pending-construction admission/retirement,
reused registration epoch, context wakeup and late publication cases passed too.

Log: `C:/Users/jnev1/AppData/Local/Temp/opencode/snipping-postmerge-lifecycle.log`.

## Native owner handoff / remaining boundary

The production native close path still requires concrete `WebviewWindow`/HWND
objects and queues `window.destroy()` onto the UI thread. These safe fixtures do
not run that callback, Tao/WebView2 destruction, real focus restoration, native
clipboard publication, or `main.rs` window event dispatch. Passing reentrant
Coordinator tests therefore does not prove whole-app survival or eliminate any
of those boundaries as the cause.

A narrow production-owned injectable queue/destroy/event-dispatch adapter around
the actual NativeWindowPort close/Entry retirement path would permit deterministic
ordering and late-callback tests without constructing HWNDs. Such a seam would
test ordering/authority, not reproduce an OS/WebView2 crash by itself. Native
process-exit/crash evidence and real capture/cancel E2E remain necessary to identify
the reported crash. No shell, HWND, hotkey, screen capture, native clipboard or
live E2E was executed: those operations remain unauthorized.
