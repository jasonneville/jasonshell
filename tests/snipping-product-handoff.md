# Remaining approved snipping feature: acceptance contract

## CURRENT native test lifecycle correction: success + forced-timeout cleanup GREEN

Independent reviewer ses_f025b2c60ffeEJaRSFIy2mkivp approved production ACL repair and
native4-marker/7-denial evidence, but found a TEST defect: prior parent timeout branch
killed/panicked BEFORE discovering/guarding child-created profile path; forced exit
bypassed child's OwnedDirectory Drop and could leak owned profiles. This is a concrete
review/source finding, not an executed pre-fix timeout result. Production build.rs,
legacy permissions/capability, handlers and application code remain unchanged here.

Fixed ONLY `src-tauri/tests/shell_acl_native_ipc.rs`:
- Parent generates exclusive numeric parent-PID/nonce fixture ID and creates approved
  Temp/opencode directory plus OwnedDirectory cleanup guard BEFORE child spawn. Parent
  writes own owner marker and passes only the bounded ID, never an arbitrary path.
- Child derives path from approved root, validates numeric ID shape, rejects symlink
  root and checks parent's owner marker. Normal native profile uses this SAME pre-owned
  directory. Child stdout is consistency metadata, NOT authority needed for cleanup.
- Parent timeout terminates only its exact std::process::Child; verifies exit via bounded
  two-second wait. AFTER child exit, parent removes exact owned directory and ASSERTS
  absence BEFORE parsing metadata/reporting native failure/returning timeout error.
  Parent guard also exists for spawn/unwind failures. No process-name or directory scan.
- New ignored-by-default `native_ipc_forced_timeout_cleans_exact_parent_owned_profile`
  drives narrow test-only fault: child writes owned fault-ready file then hangs BEFORE
  Tauri/window/server initialization. Parent waits for that initialization evidence,
  triggers50ms deadline, proves non-success child exit and profile removal BEFORE
  returning exact timeoutErr, which the test independently checks. Child termination
  bypasses child Drop as in review defect. This fault does NOT force an OS WebView hang
  or claim native-cache lock coverage; normal native run still verifies actual WebView
  profile cleanup after process exit. No production flag/seam/dependency added.

Executed focused validation (no full-suite duplication):
- `JASONSHELL_ACL_NATIVE_TESTS=1 npm run cargo:test -- --test shell_acl_native_ipc -- --ignored --test-threads=1 --nocapture`
  (PowerShell process-scoped env restored afterward): **2 passed,0 failed,0 ignored**,
  **5.30s**,19.04s compile. First body retains actual native5 hidden surfaces/4 authorized
  markers/7 ACL denials/snip handlerCalls0 with post-exit real profile cleanup. Second
  body prints `forced timeout: exact owned child exit verified; parent-owned profile
  removed BEFORE timeout error`. No zero-stage/ignored collection counted as pass.
- `npm run cargo:test -- --test shell_app_acl`: **4 passed,0 failed/ignored**,0.07s.
- `node --test tests/snippingCapabilities.test.mjs`: **2 passed**,80ms; snip grants intact.

Independent reviewer was notified for read-only re-review in its existing session;
nested code-reviewer delegation remains depth-blocked, so re-review verdict is pending.
No production shell/handler, activation/menu/file/capture action, clipboard, display
change, paused snip journey or new dependency executed. Harmless native ACL E2E remains
actual transport proof; real application activation is STILL manual pending user's
controlled restart. Requested lifecycle defect/fault validation complete; do not upgrade
that transport evidence into Windows focus/activation success.

## CURRENT repaired ACL gate: resolver4 + actual native IPC GREEN

Supersedes urgent RED below. Backend owner repaired ONLY build.rs and explicit
legacy-shell permission/capability: inventory202 registered commands, legacy191 grouped
across18 existing local static labels, snip11 still separate. This restores previous
legacy IPC reachability; it is NOT a new per-command owner isolation guarantee. Existing
backend caller/native identity/destructive-action guards remain necessary and unchanged.
Test owner did not edit production or relax snipping restrictions.

Fresh affected validation:
- `npm run cargo:test -- --test shell_app_acl`: **4 passed,0 failed,0 ignored**,0.08s.
  Real generated Manifest/Capability -> Tauri Resolved/RuntimeAuthority resolution;
  all202 inventoried, required legacy callers allowed, exact snip partition and remote/
  unknown denial intact. Previous191 missing-grant defect no longer reproduced.
- `node --test tests/contractsSettings.test.mjs tests/snippingCapabilities.test.mjs tests/hotkeySystem.test.mjs tests/windowsKeyOverride.test.mjs`:
  first36 passed/1 failed due expected capability window-array list missing legitimate
  new legacy group. Test owner added EXACT18-label group expectation while retaining
  exhaustive list equality. **Final37 passed,0 failed/skipped/todo**,119ms. No broad
  match/skip or production change. These remain contract evidence, not native transport.
- `npm run check`: **GREEN**,0 errors/0 warnings.
- `npm run build`: **GREEN**,5.44s; existing large-chunk advisory remains.

### Actual native cross-surface WebView IPC executed

New test-owned `src-tauri/tests/shell_acl_native_ipc.rs`, ignored by default with explicit
`JASONSHELL_ACL_NATIVE_TESTS=1`, compiles SAME production generated context/capabilities
into an isolated Tauri Wry/WebView2 process. No production main/modules/AppBar/hooks or
real application handlers loaded. NO add_capability/allow_command/mocked RuntimeAuthority,
no synthetic report command bypass. Registered names match actual production handlers,
but SIX test-owned terminal implementations can ONLY append marker/caller metadata and
return a string. They cannot activate HWND, list private windows, open menus, access
files/settings, capture pixels, publish clipboard or mutate display/focus.

Command with process-scoped opt-in restored afterward:
`npm run cargo:test -- --test shell_acl_native_ipc -- --ignored --test-threads=1 --nocapture`
**FINAL1 passed,0 failed,0 ignored**,5.23s (18.62s build). Test explicitly waits for ALL
five hidden/nonactivating WebViews and ALL11 IPC outcomes, not zero-stage GREEN:
- Local `top-bar`: list_pinned_taskbar_apps and show_top_bar_pin_context_menu -> actual
  renderer invoke -> Tauri ACL -> expected harmless marker/caller.
- Local `bottom-bar`: list_open_task_windows and activate_task_window -> markers;
  start_snip -> actual ACL rejection, terminal never entered.
- Local `acl-unknown`: list_open_task_windows/start_snip -> actual ACL rejections.
- REMOTE `quick-launch-panel` (known legacy-enabled label) on owned loopback-only HTTP
  static document: list_open_task_windows/start_snip -> actual ACL rejections.
- REMOTE `snip-preview-7` (copy enabled LOCALLY by production capability): copy_snip/
  start_snip -> actual ACL rejections.

Observed exact totals: **4 permitted harmless marker calls,7 ACL denials,
start_snip/copy_snip handlerCalls=0**. Host-side oracle checks label+command independently
of renderer's expected flag, exact marker values and counts; every frame must report
actual JS IPC bridge present. Denials must classify as Tauri permission/ACL rejection,
not missing JS bridge/network/general script failure. Actual custom-protocol local
frames and isolated loopback remote frames exercise real native IPC origin handling.
No external site is contacted; server serves only its own minimal static test document.
Window visibility=false/focused=false/skip_taskbar=true; no ShowWindow/focus API.

Fixture lifecycle correction/evidence: initial native traffic assertions passed, but
new strict cleanup assertion correctly went RED because WebView2 retains profile
handles until hosting PROCESS exits. Changed test infrastructure ONLY to bounded
isolated same-test-binary child;25s native watchdog/40s parent bound. Parent validates
newly owned fixture identifier against exact child PID+numeric nonce, waits child exit,
then retries ONLY that exact approved-temp directory and ASSERTS it is removed. Final
native run verifies post-exit cleanup; no unrelated caches/profiles/processes touched.
This was test lifecycle RED, not a reproduced repaired ACL authorization failure.

Evidence scope: actual native renderer -> Tauri IPC ACL transport is NOW established
for11 representative cases plus resolver coverage across complete202 inventory. Four
positive command NAMES reach harmless terminals, not real production behavior. Does
NOT prove taskbar icon clicks activate an application, real menu/pin handlers work,
ownership/destructive handler guards, OS global hotkeys or Windows foreground changes.
Actual activation remains manual pending user's controlled application restart. Do not
automatically restart shell or relabel this proof as real application-focus success.

Paused `tests/snippingNativeJourney.test.mjs` / native synthetic snip journey NOT resumed.
Its earlier exit4 remains unverified, not pass evidence. No Vite server, native clipboard,
capture/window snapshot, production shell, display mutation or new dependency used in
this repaired ACL transport dispatch. Full snipping native workflow remains separate.

Review delegation attempted for native test and exact capability expectation; blocked by
subagent depth limit. Leader independent review remains required. Requested repaired-ACL
checks and harmless native transport evidence complete; actual application activation
requires user-controlled restart/manual observation, not further agent focus mutation.

## Prior urgent regression gate: legacy shell app ACL RED

Current user report: taskbar activation/list and most top-bar menus fail while snipping
still works. Investigator ses_f026abc28ffeQ2SNHZtRQ0KQPc identified build.rs AppManifest
commands restricted to snip11. Test owner independently established REAL Tauri ACL denial,
not just source/config inference. Native affordance execution is STOPPED pending owner
repair; no production AppBar shell, focus action, desktop/private pixel read or display
change is permitted by this regression dispatch.

Artifact `src-tauri/tests/shell_app_acl.rs` uses installed
`tauri::utils::acl::resolved::Resolved::resolve` and
`tauri::ipc::RuntimeAuthority::resolve_access`, loading ACTUAL build-generated
`gen/schemas/acl-manifests.json` and `capabilities.json` into Tauri's Manifest/Capability
types. It creates NO Builder/AppHandle/window, dispatches NO handler and calls NO native
activation/list API. Main invoke-handler source is parsed ONLY to inventory registered
command names (coverage floor), never to decide authorization. Authorization decisions
come from actual library code and generated permissions, not a copied validator or
source regex. All202 registered handlers covered, not merely snip-only JSON expectations.

Command `npm run cargo:test -- --test shell_app_acl`: **EXECUTED RED**,4 tests:
**2 passed,2 failed,0 ignored**,0.03s (15.98s compile). Exact findings:
- `existing_surface_ipc_resolves_before_handlers_without_broadening_snipping` FAIL:
  bottom-bar -> list_open_task_windows, activate_task_window, launcher list/launch,
  preview show/hide and task context menu all DENIED by RuntimeAuthority before handler.
  top-bar -> pins list/launch/context menu, terminal/command/tray/audio/calendar menus,
  centered search and load_shell_settings all DENIED. Settings load/save, process list,
  audio state/devices/sessions and Stack folder read also DENIED:24 representative
  existing caller/command pairs across6 surfaces.
- `every_registered_legacy_command_has_explicit_existing_surface_acl_coverage` FAIL:
  handler inventory202 = snip11 + **191 legacy handlers with NO resolved grant on ANY
  of18 known legacy surfaces**. Failure enumerates all missing commands, not only taskbar.
- `actual_resolver_preserves_snip_surface_partition_and_remote_denial` PASS:
  exact top-bar start/bar-ready; overlays context/image/ready/begin/complete/cancel;
  previews context/image/copy/save/dismiss. Other existing/base-alias/unknown windows
  denied snip11; remote untrusted URL denied every registered command on each surface.
- `resolved_app_command_grants_never_use_global_window_or_webview_wildcard` PASS:
  no global app wildcard. Extended afterward with actual resolution asserting all
  registered commands denied for unknown/future/non-shell surface labels as well.

Installed library proof inspected:
- tauri-build2.5.6/src/acl.rs app_manifest_permissions and build inserts `__app-acl__`
  once application permissions exist; AppManifest::commands autogenerates command grants.
- tauri-utils2.10.1/src/acl/resolved.rs resolves capabilities and sets has_app_acl.
- tauri2.10.3/src/webview/mod.rs1776–1805 resolves access then checks ALL custom commands
  when has_app_acl_manifest is true, not only names passed to AppManifest.commands.
- tauri2.10.3/src/ipc/authority.rs439–469 contains actual window/webview/origin matcher.
Current generated app manifest has only snip permissions. bottom-bar/default top-bar
capabilities have only core plugin permissions, which do NOT grant app custom commands.
Thus app ACL activation silently removes legacy IPC before backend handlers can run.
Context7 current Tauri official security/capabilities docs confirms registered custom
commands are globally allowed by default UNTIL AppManifest::commands constrains them:
https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/security/capabilities.mdx

### Exact acceptance policy for production owner's repair

1. Keep application ACL enabled and snip11 exact partition; unrelated surfaces must
   still resolve start_snip/snipping actions to None. Do NOT restore functionality by
   deleting app ACL, permitting global `*`, adding snip to core/default/all windows,
   disabling resolver or bypassing native concrete-instance/owner checks.
2. Cover FULL actual registered handler inventory in application permission definitions
   and appropriate EXISTING static caller capabilities. Caller policy must come from
   current frontend/native owner contracts, not guessed new broad permission lists.
   Every legacy registered command must retain at least one legitimate existing caller;
   representative24 pairs in artifact MUST resolve locally. The any-existing-surface
   coverage floor catches forgotten commands, but is NOT a full191-command caller matrix
   or proof that sensitive commands are safe from every other surface. Production owner
   must preserve/derive backend owner restrictions and review assignments explicitly.
3. Keep required backend authorization even after ACL allow: renderer paths/handles,
   destructive commands, process identity and owner merges remain validated by handlers.
   Tests do not invoke or weaken those handlers. Unknown/future/remote surfaces deny.
4. Rebuild production generated manifests/capabilities, rerun same artifact until4 GREEN;
   changing test expectation to 'legacy denied' or dropping registration inventory is
   forbidden. Then rerun full Node/rendered/compile and existing snipping suites to
   ensure fix does not broaden snip or regress prior behavior.

What previous focused validation MISSED: snippingCapabilities.test.mjs asserted exact
snip grants and absence in legacy files, but did not evaluate application-wide ACL
activation or require legacy command positive grants. Coordinator/runtime/settings tests
injected adapters without Webview IPC gate. Rendered fake-native mocks resolved IPC
directly. Build/check established compile only. Passing these did NOT establish that
actual taskbar/menu custom IPC could reach its handler. Current artifact fills that
actual resolver boundary gap while keeping old evidence limits intact.

RED/handoff sent to investigator session for original backend owner. Independent
security reviewer delegation blocked by subagent depth limit; leader review required.
No production permission changes/dependencies made by test owner.

### Smallest native cross-surface transport E2E alternative

Actual ACL resolver DOES execute standalone here. Remaining E2E boundary can use a
tiny test-owned Tauri entry (never production main) loading SAME generated capabilities
and a few hidden/nonactivating windows with actual top-bar/bottom-bar/settings/snipping
labels. Invoke selected registered command NAMES via real Webview IPC; harmless
test-only terminal handlers return synthetic values/count calls instead of enumerating
private task windows, activating HWNDs, opening menus or modifying persistence. Assert
bottom-bar list/activation names and top-bar menu/pin names reach harmless terminals;
settings -> start_snip denied BEFORE terminal. This proves native IPC ACL transport,
NOT actual activation/menu handler behavior or real pointer input. It needs no private
pixels/clipboard/shell/display/focus action and no human shell smoke gate. Source owner
may adapt existing standalone fixture; do NOT launch current native journey during
urgent repair. Wider real-shell activation remains separately safety-scoped/unverified.

### Previous synthetic native journey attempt (not pass evidence)

Before urgent STOP, owner-handed example rebuilt GREEN and test owner authored
`tests/snippingNativeJourney.test.mjs`, opt-in only (`JASONSHELL_SNIP_JOURNEY_TESTS=1`);
independent Node/zlib PNG oracle expects all9450 pixels at126x75 and one publication,
actual preview nonactivation and wrong-window start denial. Initial execution ended
exit4 with NO captured stage/assertion report; cause not established. No E2E pass or
concrete native behavioral failure is claimed. Owned journey process check now reports0;
no further launch/cleanup/focus mutation attempted in urgent dispatch. Artifact remains
disabled by default. Synthetic clipboard consent cannot authorize production shell or
private capture. Native journey must await repaired ACL and source-owner handoff.

## Current consolidated validation checkpoint

Fresh requested consolidated runs completed. This section is the latest evidence;
the checkpoint narratives below are historical/superseded where counts or readiness
differ. Actual native Tauri journey remains owned by ses_f04fb272dffepk7LGxikb4mTTD:
test owner did NOT duplicate, build a new affordance or execute that journey here.

### Frontend / Node

| Exact command | Current result |
| --- | --- |
| `npm run test:node` (first fresh run) | 1211 total;1203 passed,4 failed,1 skipped,3 todo,0 cancelled;4.26s |
| `npm run test:node` (final after legitimate fixture corrections) | **1211 total;1206 passed,1 failed,1 skipped,3 todo,0 cancelled**;2.54s |
| `npm run test:component` | **68 passed**,9 files;9.81s; includes snipping rendered34 |
| `npm run check` | **GREEN**,0 errors/0 warnings |
| `npm run build` | **GREEN**,5.54s Vite build; advisory >500kB chunk warning |

The sole FINAL Node failure is the established unrelated baseline:
`tests/contextMenuVisualRegression.test.mjs:87:1`, test
`icon and placeholder share a centered 16px cell for every row`; assertion at91:20,
helper at17:10: missing combined CSS rule
`:global(.js-context-menu .context-menu-icon), :global(.js-context-menu .context-menu-icon-placeholder)`.
No context-menu production/test change, skip or assertion weakening was made.

Three initially NEW failures were legitimate stale test expectations, corrected in
test artifacts ONLY, then full Node rerun demonstrated their removal:
- `tests/surfaceCodeSplitting.test.mjs:50:1`: exhaustive surface-import fixture omitted
  `snip-overlay`/`snip-preview`.
  Added exact imports SnipOverlaySurface.svelte/SnipPreviewSurface.svelte. Existing
  exhaustive union equality, dynamic-import-per-surface, no eager App imports and
  unknown-surface null assertions remain intact.
- `tests/windowsKeyOverride.test.mjs:121:1` and `:161:1`: expected obsolete
  install_windows_key_hook signature. Updated to actual legacy-aware startup settings
  loader and install_loaded_windows_key_hook with original hotkeys+missing_snipping;
  retained uninstall and required/fatal map_err assertions, not optional hook semantics.

All snipping Node contracts now pass within full suite; no new final snipping failure.
Component jsdom printed three HTMLCanvasElement.getContext not-implemented notices;
no dependency installed. Real rendered fake-native journey remains labelled as such,
NOT actual native end-to-end. No Svelte component was edited in this checkpoint.

### Rust / independent synthetic clipboard

| Exact command (all prefixed `npm run cargo:test --`) | Current executed result |
| --- | --- |
| `--test snipping_coordinator --test snipping_save --test snipping_native_registry --test settings_owner_hotkey_save_contract` | All GREEN; individual results below |
| coordinator in that command | **33 passed**,0 failed/ignored,0.48s: original coordinator22 PLUS runtime11 collected through actual runtime cfg(test) include bridge; not33 coordinator-only claims |
| save in that command | **4 passed**,0 failed/ignored,0.02s; independent synthetic PNG/readback and sentinel preservation |
| native_registry in that command | **6 passed**,0 failed/ignored,5.01s; actual registry/composition deadline functions, not live HWND/Tauri authorization |
| owner save contract in that command | **2 passed**,0 failed/ignored,0.00s |
| `--bin jason-shell snip_runtime_ -- --test-threads=1` | **11 passed**,0 failed/ignored,0.01s; actual Runtime with injected edges |
| `--bin jason-shell snip_hotkey_ -- --test-threads=1` | **7 passed**,0 failed/ignored,0.01s; actual settings/registry acceptance bodies |
| `--bin jason-shell settings::tests:: -- --test-threads=1` | **53 passed**,0 failed/ignored,0.58s; filter also matches search-provider settings tests as previously recorded |
| `--bin jason-shell windows_key_hook::tests:: -- --test-threads=1` | **4 passed**,0 failed/ignored,0.00s |
| `--test snipping_clipboard_native -- --ignored --test-threads=1 --nocapture`, with `JASONSHELL_SNIP_CLIPBOARD_TESTS=1` | **13 passed**,0 failed/ignored,11 filtered out,5.83s; current ACTUAL OS clipboard integration |

Native clipboard13 was executed serially under existing synthetic consent, not counted
from ignored-by-default collection. Process-scoped opt-in restored after command.
Only synthetic sentinel/grid PNG data and hidden tiny clipboard windows/helper were
used: no desktop capture, private clipboard readback, shell/AppBar/global hotkey launch,
resolution/DPI/display change or private screenshot. Native13 independently validates
CF_DIBV5/PNG pixels and persistence after producer exit, busy/precommit sentinel
preservation, truthful durable/warning/unknown outcomes and owned helper cleanup.
Latest allocation evidence is STILL logical86/global86/padding0,
roundedPaddingCoverage=false; actual handleReused=false. Positive allocator padding
and real HWND recycling remain UNVERIFIED, not invented by13 passing assertions.
Native stall evidence: acknowledged2010ms, pre-set2008ms, shutdown508ms, helperExited/
writerDrained true; no private contents were logged.

No additional geometry/session Phase1 rerun or unrestricted full Rust suite was started.
The requested coordinator test binary happens to collect runtime11 through the approved
include; recorded transparently rather than discarding cases or silently counting them
as native OS evidence. Builds may share Cargo locks with native owner; no owner process
was stopped or controlled. Rust warnings remain warnings, not suppressed/fixed here.

Review: attempted independent code-reviewer delegation for both small fixture changes;
blocked by subagent depth limit. Leader independent review remains required. Current
consolidated request is complete with the known unrelated Node baseline reported; no
claim that full repository Node is GREEN. Next gate: await native owner's scoped
synthetic Tauri affordance handoff BEFORE running required actual native journey.
Injected runtime11, fake-native rendered34 and clipboard13 each establish their stated
boundaries, not that remaining E2E. Manual mouse/AppBar/mixed-hardware checks remain
waived/unverified, not passed.

## Prior actual Runtime fixture: missing-bridge RED -> eleven bodies GREEN

Supersedes previous runtime seam request, registry-loader RED and post-composition
source risk below. Native owner reported registry6 executed GREEN after CommonControls
v6 test manifest; coordinator22/save4 GREEN. Those unchanged suites were not repeated
here. Actual runtime now exposes RuntimeEdges plus verification construct/Ports/start/
ack/pending/wake/restore; foreground Tracker uses SAME observer apply/proves transitions.
Actual ID5 source now reserves native intent synchronously before spawning its worker,
then consumes run_native_start(ticket); its ticket Drop releases suppression.

New test-owned artifacts:
- `src-tauri/tests/fixtures/snipping_runtime_acceptance.rs`: eleven actual Runtime and
  shared foreground transition cases; six coordinator ports and native edges injected.
  No fake Runtime/Coordinator or copied foreground validator. Publication/picker/file
  boundaries panic if reached, so these barrier tests cannot silently publish anything.
- `src-tauri/tests/snipping_runtime_acceptance.rs`: bounded binary-module runner;
  lists actual binary tests, requires all11 exact names, then executes only snip_runtime_
  bodies serially. Zero collected bodies cannot pass. Owned compiler child deadline90s,
  synthetic compiler output only in approved opencode temp, removed after use.

First command `npm run cargo:test -- --test snipping_runtime_acceptance`:
**executed RED**,0 passed/1 failed,0.97s, exact missing
`snip_runtime_forged_ack_has_zero_composition_and_capture` include bridge/case. No
behavioral bodies were executed by that binary attempt. Exact owner handoff SENT to
native owner ses_f04fb272dffepk7LGxikb4mTTD: at module scope in runtime.rs add
`#[cfg(test)] include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/snipping_runtime_acceptance.rs"));`
Native owner installed this bridge during execution; test owner did not edit production.

Temporary standalone actual-source harness initially executed11 GREEN, then while owner
bridge landed collected22 duplicated bodies. Its later extra cancellation assertion exposed
a TEST adapter mismatch: WindowPort.still_same required exact monitor equality even when
actual Coordinator.live queries concrete caller with monitor None. Corrected boundary
adapter to exact caller ID/label, plus monitor equality IF requested (same native registry
contract). Retained all authorization assertions. Removed redundant temporary harness;
final durable entry is binary runner only, no doubled counts or future zero-test harness.

Validation:
- `npm run cargo:test -- --test snipping_runtime_acceptance`: **wrapper1 passed**,
  all11 actual binary bodies required/executed,15.57s; followed by extra focus-race and
  composition reused-identity assertions in same artifact.
- Final `npm run cargo:test -- --bin jason-shell snip_runtime_ -- --test-threads=1`:
  **11 passed,0 failed,0 ignored**,0.01s (13.82s compilation).
  Visible body evidence includes ALL latest assertions. No native OS capture performed.
- Final wrapper rerun after latest assertions: **1 passed**,1.70s; lists/requires and
  executes all11 current bodies. Final evidence/limits handoff sent to native owner.

Cases/effect evidence:
1. Wrong start caller/window instance rejected before intent snapshot. Wrong ack label,
   reused top-bar ID, forged generation/captureId rejected; pending ack stays false;
   **compositionCalls=0,captureCalls=0**, then bounded timeout releases exact token.
2. Exact bar+token returns accepted plus same token; duplicate rejected while composition
   paused. Intent -> worker-launch -> prepare/ack -> composition -> capture -> overlay
   create; exactly one release. Cancel closes owned overlay; newer generation rejects old
   ack without advancing or composing/capturing that newer attempt.
3. Pending deadline exactly5s; injected clock reaches boundary, late ack rejected;
   wake returns readiness-timeout, clears pending/frame/window state, releases once.
4. Actual reserve_native_start snapshots BEFORE worker launch; repeat Busy; external
   foreground change before run_native_start invalidates original ticket rather than
   recaching. Ticket Drop, cross-runtime rejection, expired ticket and failed bar check
   release reservation; shutdown before dispatch rejects worker. No renderer click path.
5. Composition callback loses observer continuity, changes external foreground, or
   recreates SAME HWND/PID/thread with NEW native epoch: actual Runtime AFTER-flush
   validation rejects before capture, composition1/capture0, truthful cleanup/release.
6. Post-composition bar loss/deadline prevents capture; prepare emit rejection and
   shutdown while awaiting ack both unwind with zero composition/capture.
7. Restore requires stored generation+captureId and still-owned foreground; wrong
   token/external focus/continuity loss suppress restore. User focus change DURING intent
   validation wins second foreground check; no activation callback runs for it.
8. Actual Tracker rejects PID/thread mismatch, missing identity, destroy/create of
   candidate, unknown foreground, other external foreground, same-HWND new epoch and
   continuity loss. Unrelated window lifecycle and proven OWN activation preserve intent.
9. Capture error leaves no resources and allows fresh reservation; dropping actual Runtime
   after successful synthetic capture closes one owned overlay exactly once; Weak cannot
   upgrade afterward. No strong Runtime self-cycle in constructed fixture.

Evidence limits: clock/visible-bar/emit/compositor/foreground-handle/current OS identity
and native worker launch are INJECTED boundaries. Shared Runtime and Tracker logic is
executed, not actual WinEvent queue/PID discovery/GetProp HWND resolver/DwmFlush/GDI.
The controlled launch calls same Runtime reservation/consume APIs as inspected ID5
source, but raw WM_HOTKEY -> AppHandle -> managed Runtime -> actual thread scheduling
is NOT executed. No fabricated ID5 OS dispatch/native Tauri IPC evidence. Renderer
tooltip/decode/component34 and coordinator22 remain separate earlier evidence. No claim
of full-runtime native end-to-end or coverage percentage. Forged ACK oracle is Runtime
core evidence, NOT native WebviewWindow resolve/capability enforcement.

### Minimum remaining synthetic native E2E affordance (proposal only)

Prepare standalone test-owned Tauri harness using existing dependencies, actual commands,
NativeRegistry/NativeWindowPort, Runtime native edges and real mounted bar/overlay/preview
components. Do NOT enter production main, AppBar setup, Explorer or OS global hotkey hooks.
Use an owned external child-window synthetic grid as foreground target and a strictly
bounded test-region inventory: a CapturePort delegates to actual capture::freeze_monitors
ONLY for the grid's proven fully-covered physical client rectangle, never the desktop.
Verify exact concrete ownership/geometry/opaque full coverage and abort BEFORE GDI if
uncertain; ordinary desktop monitor inventory must not be substituted silently. This
reduced-area journey proves that declared boundary only, not full all-monitor production
NativeCapturePort or reserved-AppBar coexistence. Stronger all-monitor capture requires
explicit covered-grid setup on all monitor rectangles and separate agreed validation.

Run current DOM tooltip prepare/flat ack -> actual native composition -> bounded-region
GDI -> renderer decode/armed/selection -> hidden preview/context -> one automatic native
Copy -> nonactivating show -> synthetic PNG save/readback; independent grid pixel oracle
and publication count/outcome metadata. Wrong concrete window command invocations must
deny (including malformed/reused labels) BEFORE capture/composition/publication. Record
safe metadata only; no private screenshots, clipboard contents, settings, paths or logs.
Use existing authorized isolated synthetic clipboard/native13 opt-in for publication
checks, do not broaden that consent to production shell/full-desktop capture. Harness
windows/capture setup not executed during this dispatch. Cleanup closes only owned test
windows/processes and removes only synthetic output; no resolution/DPI/taskbar changes.
Native owner should provide this scoped harness entry/capture-region safety gate; no new
dependencies, permanent production test flags, hidden displays or private desktop capture.

Independent review delegation again blocked by subagent depth limit; leader review
required. This bounded artifact/evidence dispatch is complete. Native E2E REQUIRED and
still pending; manual mouse/AppBar/hardware/HWND recycling stays unverified as recorded.

## Current continuation: seven bodies executed; native registry loader RED

This supersedes the bounded RED checkpoint below. Backend owner landed production
fifth field, legacy startup registry, direct runtime dispatch and test include bridges.
Test owner changed ONLY old `#[cfg(test)]` fixture bodies in settings.rs and
windows_key_hook.rs: added default snipping field, unwrapped optional occupied slots,
removed obsolete double `map(Some)`, kept original assertions and added fifth action/key
and rollback-slot assertions. No production behavior was edited by test owner.

Executed evidence:
- `npm run cargo:test -- --test snipping_hotkey_settings`: wrapper1 passed; rejects
  missing/zero cases and executes all seven actual binary-module bodies serially.
- `npm run cargo:test -- --bin jason-shell snip_hotkey_ -- --test-threads=1`:
  **7 passed,0 failed**,0.01s. Separate visible-body run confirms listing is not evidence alone.
- `npm run cargo:test -- --bin jason-shell settings::tests::`: **53 passed,0 failed**,
  0.24s (filter also matches nearby search-provider settings tests). Includes existing
  owner transaction registration rejection, persistence failure and rollback tests.
- `npm run cargo:test -- --bin jason-shell windows_key_hook::tests::`: **4 passed,0 failed**.
- `npm run cargo:test -- --test settings_owner_hotkey_save_contract`: **2 passed**;
  stale source wiring expectation updated to transaction_hotkey_configurer and explicit
  strict configure plus legacy rollback helper assertions. Actual behavioral evidence
  comes from seven bodies and existing transaction tests, not source string checks.
- `npm run test:component -- tests/components/snipping-product.test.ts tests/components/snipping-barrier.test.ts`:
  **34 passed**, two real rendered suites,4.67s. Fake-native component journey is not OS proof.
- `node --test tests/contractsSettings.test.mjs tests/hotkeySystem.test.mjs tests/snippingCapabilities.test.mjs`:
  **22 passed**. Expected exhaustive Rust events now include all four snip events;
  capability list adds ONLY top-bar/overlay/preview snip scopes; startup wiring assertions
  follow legacy-aware loader/installer, preserving register-before-persist/rollback checks.
  New parsed JSON/generated permission-output cases assert EXACT per-surface command
  grants, listen/unlisten only, no wrong-window/global/default grants; all11 generated
  allow/deny files each bind exactly their own command. This verifies configuration,
  NOT real Tauri invocation denial or native concrete-instance authorization.

`src-tauri/tests/snipping_native_registry.rs` now owns SIX smallest actual native-module
cases, importing production native_registry/composition functions, not parallel mocks:
forged instance/monitor/token rejection; reused label rejects prior epoch/context;
buffered context and registry-lock release; invalidation wake; real five-second
initialization timeout; expired composition deadline rejected before child launch.
`npm run cargo:test -- --test snipping_native_registry` **compiles then fails BEFORE
test harness startup**, exit **0xc0000139 / STATUS_ENTRYPOINT_NOT_FOUND**. **Zero bodies
executed**; this is environment/loader RED, NOT six demonstrated behavioral failures.
No native capture, compositor child, HWND/window, clipboard, foreground observer or
production AppBar was launched by this test. DLL/import root cause not established.
Next owner can resolve loader or include same test bodies in actual binary-module test
bridge (which executes on this machine), without copying registry logic or fail-open stubs.

### Narrow runtime/dispatch verification affordance request

Observed source path (not executed behavior): dispatch_hotkey matches occupied ID5
Snipping and calls managed SnipRuntime.start_native on a worker, not a JS click;
runtime start reserves `starting`, checks active/save state, snapshots native observer
before coordinator.start; prepare emits exact pending token, waits, validates bar/native
intent, flushes composition, and rechecks bar/token/deadline. ack/release are private;
runtime installation creates actual native observers and needs AppHandle/WebviewWindow.
Do not construct a second fake runtime to turn this into GREEN.

Request native/backend owner to expose cfg(test) construction/inspection of the SAME
SnipRuntime.start/prepare/ack/release and SAME hotkey dispatcher. Minimal injected edges:
native-intent snapshot/validate; concrete bar identity+visible check; event emission;
composition flush; worker launch; monotonic deadline. Keep production coordinator,
pending token/bar/ack state, start reservation and callbacks in use; no successful
default and no renderer-supplied HWND. Owner can choose aliases; test owner accepts
equivalent private test-include seam, not a production flag/dependency/API redesign.

Next bounded acceptance table (authored plan, NOT executed or collected tests):
1. ID5 occupied invokes native start once; None/unknown ID does not launch; old1–4
   routes unchanged. Controlled worker records native snapshot BEFORE prepare/UI;
   outstanding start/repeat cannot launch second capture. Never synthesize click/event.
2. Exact pending token plus exact concrete visible bar accepts once. Wrong surface,
   reused bar ID, forged/stale token, duplicate and late ack reject; effect oracle shows
   ZERO composition/freeze for each unauthorized or timed-out preparation.
3. Five-second missing-ack bound releases matching token and drops capture resources;
   cancel/shutdown wakes waiter, no late ack can revive generation or freeze.
4. Valid ack -> native foreground continuity -> composition -> freeze; callback-driven
   epoch/PID/thread change (including same HWND integer) fails closed before freeze;
   changed foreground before restore suppresses activation, token history never relabels.
5. Post-flush bar loss/deadline/topology mismatch unwinds and release matches original
   token; no bar hide/exclude/recolor calls. Runtime registry shutdown/retirement owns only
   exact concrete snip instances. Native tests must assert effects, not source regex.

Specific source-inspection risk for owner: runtime.prepare currently validates
observer.valid(original) immediately BEFORE composition::flush, but its AFTER-flush
checks only bar visibility, deadline and coordinator token. A foreground epoch loss
DURING deferred composition is therefore not visibly rechecked at that boundary.
This is a source finding to drive case4, NOT an executed OS/behavioral failure.
Owner should expose deferred composition and native identity oracle so assertion can
establish whether freeze remains fail-closed; do not claim source inspection proves it.

### Required synthetic native E2E setup, still pending

Repeatable stronger affordance: standalone test-owned Tauri/native harness with NO
production main/AppBar/hooks/Explorer path. Existing dependencies only. Register actual
11 runtime commands/capabilities; create test-only concrete bar, overlay, preview and
external synthetic intent window; cover the captured desktop regions entirely with
deterministic test grids BEFORE actual GDI capture. Abort before capture if complete
coverage/monitor geometry cannot be established; do not read uncovered desktop pixels.
Run actual tooltip ack/composition/freeze/selection/automatic Copy/preview and synthetic
safe-PNG readback with independent pixel oracle; controlled clipboard opt-in and cleanup
restore policy required. No private clipboard restore/capture can be improvised.
Needs native owner seam and explicit agreed harness/clipboard consent before execution;
test preparation adds no permanent production flag/dependency. Existing native13 later
remains required, not counted from ignored tests or historical run. Hardware mouse,
reserved-AppBar coexistence/mixed-DPI/HWND reuse remain waived or unverified as recorded.

Review delegation attempted: code-reviewer blocked by subagent depth limit. Leader
independent review required. Runtime source risks/OS semantics not certified by this
bounded checkpoint. Requested settings validation is GREEN; next runtime RED artifact
and precise affordance handoff are ready, not all-native completion evidence.

## Latest bounded settings/hotkey RED + postcommit observer GREEN

Test-owned settings/registry cases now live in
`src-tauri/tests/fixtures/snipping_settings_acceptance.rs` (five cases) and
`src-tauri/tests/fixtures/snipping_registry_acceptance.rs` (two cases). These are
acceptance children of ACTUAL production modules, not copied validators/state machines:
old four defaults preserved plus Alt+S; legacy missing-field load/canonicalization without
rewrite; explicit five-way canonical duplicate rejection; unrelated save still strict;
repair/register-before-write and register/persist rejection rollback with existing
`save_settings_transaction` callbacks; IDs1–4 preserved/ID5 no-repeat Alt+S; legacy
Alt+S leaves ID5 vacant then repairing Alt+X adds5 without changing original slots.
Fixtures create ONLY synthetic settings JSON in approved opencode temp root; Drop
removes own file. No private settings paths, no OS registrations or Tauri shell startup.

Bounded bridge handoff BEFORE backend implementation (test owner does not edit source):
production owner should add under cfg(test), at module scope in actual settings.rs:
`include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/snipping_settings_acceptance.rs"));`
and in actual windows_key_hook.rs:
`include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/snipping_registry_acceptance.rs"));`
All test bodies remain in owned test files. This gives private seam access without
duplicate production modules, a mock validator, or widening runtime authority/API.
Registry compatibility: optional `[Option<RegistryBinding>;5]`, unchanged IDs1–4,
Snipping action5, strict `from_settings`; narrow legacy builder alias
`from_loaded_settings(&StandardHotkeySettings, missing_snipping: bool)` permits ONLY
missing-field migration conflict and keeps first four slots. Alias name negotiable;
must be the SAME builder used by startup, never a second test registry.

`src-tauri/tests/snipping_hotkey_settings.rs` is bounded entry runner: compiles/list-checks
actual binary unit cases, REQUIRES all seven exact test names then runs only
`snip_hotkey_` cases serially. It never runs main/Tauri builder and cannot call a0-test
listing GREEN. Child compilation deadline90s, owned compiler process only. Do not run
unfiltered entire binary tests from this entry; unrelated production cases remain excluded.

Command `npm run cargo:test -- --test snipping_hotkey_settings`: **executed RED**,
missing `snip_hotkey_default_fifth_preserves_old_four_canonical_chords` bridge/case;
seven behavioral bodies NOT executed. Before replacing the broad import harness,
compiler also directly reported missing snipping field, fifth ID/action, optional-slot
type and legacy builder. That attempt ALSO had test harness module-resolution errors,
so it is NOT claimed as clean feature-only compile RED. Removed broad import harness;
retained exact actual-module bridge path instead of fake dependency/authorization stubs.

Pending message/runtime seam (do not fake evidence): actual dispatch_hotkey currently
requires AppHandle/native registrations and SnipRuntime::install starts native observers.
Need owner-provided OS-free dispatch callback seam used by SAME WM_HOTKEY dispatcher:
ID5 invokes SnipRuntime::start_native directly, MOD_NOREPEAT, unchanged old four routes;
controlled outstanding native-start reservation rejects duplicate/repeat work; native
foreground snapshot precedes renderer prepare/UI activation with no synthesized click.
Exact injectable callback/worker seam not yet public; runtime cross-surface E2E remains
staged. Settings transaction callback test observes registration ordering/state, NOT
actual OS registration rollback/ID5 WM_HOTKEY or foreground HWND identity.

Added coordinator22nd case `postcommit_preview_show_failure_retains_actual_outcome_token_and_never_republishes`:
actual Snapshot.last_publication observer verifies committed, committed-warning,
rejected and publication-unknown metadata after show rollback, exact original token,
one automatic attempt only; stale retry/new generation never republishes/relabels it.
`npm run cargo:test -- --test snipping_coordinator`: **22 passed,0 failed,0 ignored**,
0.42s. Includes all previous21 unchanged behavior assertions. This closes bounded
postcommit-observer gap, not native consumer clipboard/composition/hardware evidence.

Focused Node rerun `node --test tests/contractsSettings.test.mjs tests/hotkeySystem.test.mjs`:
**17 passed,3 failed**,20 total. Current actual findings differ from prior two:
Rust contracts::events::ALL is not exhaustive after adding events; capability-window
list expectation is now stale after native integration; Rust settings still lacks
snipping field. No registry exception/security weakening added. Leader/owners should
update legitimate capability list contract separately and complete Rust settings/ALL.
Frontend34 and safe-save4 GREEN owner-reported; not rerun here. No original14/native13
rerun. Native real-barrier/auth/synthetic E2E belongs to next checkpoint. Manual
OS/AppBar/hardware waived/unverified. Independent review still leader-owned.

## Latest amended architecture checkpoint: tooltip-only barrier / hidden-preview precommit

Supersedes earlier "Copy before preview creation" wording below: clarified approved
sequence is `create HIDDEN preview -> buffered context -> automatic Copy attempt ->
show preview`. Group3 now asserts those exact trace stages and retains independent
126x75 pixel oracle and exactly-one publication. Group2 still asserts concrete cleanup
once, zero file effects, zero clipboard for capture/overlay/armed/preview-create/context
failures; ONLY preview-show failure expects one automatic attempt. No wrong-caller,
stale-token, duplicate publication or four additional production-failure tests removed.

Actual observed command `npm run cargo:test -- --test snipping_coordinator`:
**21 executed,15 passed,6 failed,0 ignored**,0.41s. Prior21/16-pass/5-fail was before
amended precommit assertion. Current failures for backend:
1. Group3 hidden-preview/context/Copy/show ordering incorrect.
2. Group2 precommit preview creation failure publishes anyway (focused diagnostic below).
3. Topology-invalidated preview explicit Copy publishes: observed2 calls vs baseline1.
4. Topology change while deferred picker pending still invokes file publisher.
5. Reused preview label/different concrete identity after picker still publishes.
6. Replacement inspect omits retained previous concrete preview ID (crop bytes retained).

Focused `npm run cargo:test -- --test snipping_coordinator
group2_capture_create_context_show_and_preview_failures_unwind` confirms concrete
`failure preview`: actual1 automatic attempt, expected0. Later context/show cases in
that loop are therefore not reached in this run; don't claim their postcommit coverage.
The four additional topology/identity/inspect failures remain genuine and retained.

Postcommit outcome-observer handoff (bounded, no broad redesign): current Snapshot has
no last automatic-publication metadata. Backend must expose actual token-bound attempted
Copy outcome (existing Outcome::metadata() or equivalent immutable bounded snapshot)
also after preview-show rollback, so tests can distinguish committed/warning/unknown/
rejected. Do not infer coordinator-retained truth from fixture's configured Outcome,
WindowFailed, or clipboard call count. Exact observer name/shape needs backend alias;
until available, no truth-preservation/cancellation claim for failed postcommit show.

New `tests/components/snipping-barrier.test.ts`: actual TopBar/unchanged Melt controls,
actual TopBar stylesheet inserted in jsdom, only transport/RAF clock mocked. Cases:
tooltip-only suppression with buttons/root visible; tick+RAF before exact flat ack;
one ack per token, native-hotkey prepare independent of click; newer token invalidates
old pending frame; stale release doesn't clear newer class; matching early release/
unmount prevents ack and cleans listeners; computed-visible-tooltip refusal; keyboard
click zero-arg start without mouse; exact registry command/events and no intent IPC.
No test-only suppression CSS, fake button/component or renderer HWND authority.

`npm run test:component -- tests/components/snipping-barrier.test.ts`: **7 collected,
7 RED**. Six mount cases stop at missing `snip:prepare_bar` listener; registry case lacks
`snip_bar_ready`. Deeper visibility/RAF/token assertions are authored but unexecuted.
Existing rendered27 not rerun. No native clipboard/disk/shell/display effects.

Native adapter integration acceptance still required (core's six ports cannot establish
these native contracts): live concrete top-bar auth; native observer binds eligible
external foreground BEFORE activation, continuity invalidation on HWND destroy/reuse,
keyboard/Alt+S share barrier without pointer intent; exact pending token ack, duplicate/
forged/wrong-window/late ack rejection;5-second monotonic ack timeout closes only owned
new resources/restores old preview; targeted release only matching generation on every
failure/shutdown. Accepted DOM ack -> bounded DwmFlush -> bar-visible/current-instance
recheck -> freeze ALL monitors. NEVER hide/exclude/recolor top bar/panels; normal bars
must be in synthetic native readback. DOM/computed style is not compositor evidence.
Production native mockable registry/foreground/DwmFlush seam names remain owner-owned;
do not create fake parallel coordinator or pretend core mocks establish native identity.

E2E requirements retained: actual TopBar/component journey + actual21 coordinator
integration + safePNG4 + native clipboard13 after implementation readiness. Manual
OS pointer/AppBar/mixed-hardware waived/unverified, never converted to pass. Independent
leader review required (previous nested review delegation blocked by depth limit).

## Latest test-owner executed checkpoint: automatic Copy contract

Test-artifacts only. Recovered harness imports actual `snipping/mod.rs`, which now
includes actual sibling `save.rs`; `coordinator::SaveCode` uses that production
`SaveError`. No fake save module or coordinator. Existing wrong-caller/stale/no-extra-
effect trace assertions retained unchanged.

Corrected automatic-Copy expectations: accepted completion publishes exactly once;
unauthorized preview actions leave that count at1; six explicit Copy outcome cases
bring it to7. Added independent ordering assertion: crop publication BEFORE preview
creation/show. Dimensions and independent exact PNG pixels are checked before the
ordering assertion so timing failure does not hide crop evidence.

`npm run cargo:test -- --test snipping_coordinator` now executes assertions, not compile
RED. First recovered run15:14 passed/1 failed. Latest in-flight snapshot17: **16 passed,
1 failed,0 ignored,0 filtered**,0.40s. Count differs from historical18; observed runner
count is authoritative. Additional buffered-context/publisher-error cases landed during
owner activity; not represented as a stable final18-test suite.

Concrete production failure for backend fixer:
`group3_first_monitor_binding_trusted_fractional_conversion_and_independent_png_crop`
fails `automatic crop publication must precede preview creation/show`. Current complete
creates/emits/shows preview and closes overlays before `clipboard.publish`; required
order is cropped bytes -> automatic Copy -> preview. Exactly-one call,126x75 dimensions
and independent pixel readback pass before this assertion. Do NOT move/remove ordering
assertion to manufacture GREEN. All auth, deferred pin/100ms probe, cleanup, replacement,
and six explicit truthful clipboard-outcome cases execute successfully at this snapshot.

Scoped obsolete Node expectations amended without touching registry behavior: fifth
`snipping` action/default Alt+S, sixth top-bar control appended once; original four chords
and five control relative order remain asserted. Original five-item drag fixtures remain
valid subset behavior, not changed into an unrelated test redesign.

`node --test tests/topBarControlReorder.test.mjs tests/contractsSettings.test.mjs
tests/hotkeySystem.test.mjs`: **26 passed/2 failed**,28 total,116ms. Remaining failures
are production-owned, not obsolete assertions: Rust events omit `snip:armed` and
`snip:context`; Rust StandardHotkeySettings lacks `pub snipping: CanonicalHotkeyBinding`.
No event exception or lowered backend contract added. Frontend27/check/build and save4
GREEN were owner-reported, not rerun here. No Phase1/core14/native13 rerun; no live OS
or private image/disk/cache/display effects. Manual OS/AppBar/hardware remain unverified.

Leader must review artifacts (prior delegation depth blocked). Backend must correct
automatic Copy ordering and finish event/settings/registry integration, then rerun focused
coordinator acceptance followed by required native clipboard13/safePNG evidence.

Status: RED test preparation; user waived interactive gate as implementation prerequisite,
NOT as evidence. No person, OS mouse, AppBar, display-topology or consumer-paste pass implied.
Native13/core14 evidence remains separate in snipping-phase1-handoff.md.

## Shared IPC contract for frontend/backend owners (amended architecture input)

Exact settled production ports, replacement behavior, buffered initialization and
canonical dynamic-label rules: `.slim/deepwork/snipping-implementation-spec.md`.
This document is acceptance input/evidence, not shipped-feature documentation.

Command names: start_snip, get_snip_context, get_snip_image, snip_ready,
snip_begin_selection, complete_snip, cancel_snip, copy_snip, save_snip, dismiss_snip.
All args below are flat Tauri invoke args, not a nested `request` object.

Tokens `{ generation: string, captureId: string }`: generation canonical positive
u64 decimal, captureId opaque bounded backend-owned ID; no numeric JS u64 precision loss.
Metadata context event `snip:context` uses the same shape as get_snip_context.
Context:
`{ generation, captureId, phase: 'selecting'|'preview', monitorId, width, height,
scaleFactor?, originX?, originY? }`. Dimensions/origin physical; scale trusted native
metadata, not accepted as command authority. Each overlay context addresses ONE monitor.
Preview width/height are selected crop pixels; preview uses same monitorId/token pair.
No paths, HWNDs, caller IDs, image content or display-topology authority from renderer.

- start_snip(): returns token pair; authorized native top-bar or native hotkey dispatcher.
- get_snip_context(): derives exact concrete caller window from Tauri injection/native
  registry, returns its current context or typed inactive/unauthorized error.
- get_snip_image({generation,captureId,monitorId}): binary PNG, ArrayBuffer or Uint8Array,
  not base64/data URL/JSON image. Exact caller can retrieve only its frozen/cropped image.
- snip_ready({generation,captureId,monitorId}): renderer decoded frozen image; returns
  `{generation,captureId,ready:boolean}`. False waits for targeted `snip:armed` (same
  token pair) after ALL current overlays decoded and shown. True acknowledges an already
  armed generation, closing event subscription races. Subscribe before authoritative
  buffered context fetch. Atomic show failure/timeout unwinds new frames/windows and
  restores retained old preview if present. No capture AFTER overlay shows.
- snip_begin_selection({generation,captureId,monitorId}): `{accepted:boolean}` plus tokens.
  First backend-accepted pointerdown binds one monitor for entire generation. Another
  monitor cannot complete/replace it. UI ignores secondary/foreign/duplicate pointerdown.
- complete_snip({generation,captureId,monitorId,rect:{x,y,width,height}}): LOCAL CSS rect;
  backend revalidates fresh native monitor/scale/topology and converts floor(left/top),
  ceil(right/bottom) once. No renderer origin/DPI/physical HWND params. Positive finite
  bounds only, clamp to selected monitor; exact one completion per generation.
- cancel_snip({generation,captureId}): authorized overlay cancellation, idempotent cleanup.
- copy_snip({generation,captureId}): tagged status `committed`, `committed-warning`,
  `rejected` or `publication-unknown`, committed/durable true/false/null as actual native
  result. Bounded whitelist code only; no raw OS error/pixel/path logs. Retryable rejection
  cannot masquerade as copied; warning/unknown cannot claim prior contents preserved.
- save_snip({generation,captureId}): BACKEND picker only, no renderer path; tagged
  `{status:'saved'|'cancelled'|'error',code?:boundedCode}`. Picker cancellation writes
  nothing; success follows safe sibling-temp publication, not opening/truncating target.
- dismiss_snip({generation,captureId}): authorized preview disposal, never focus-loss auto
  dismissal. Copy/Save/Dismiss serialized; pending save pins session/image, cannot start
  replacement while picker live. All command/event settlements check current token pair.

Components load context themselves, no required props. `SnipOverlaySurface.svelte`:
accessible region "Screen snip selection", decoded image alt "Frozen desktop".
`SnipPreviewSurface.svelte`: image alt "Snip preview", MeltActionButton names
"Copy", "Save", "Dismiss". Accessible status/error text; lifecycle-safe event subscriptions.
Use Blob(type image/png), revoke every owned URL on replace, stale load or unmount.
Callbacks from older context cannot mutate current controls, report success or dismiss.

TopBar accessible button "Capture screen region", control ID `snip`, appended after
sound by default; existing v1 localStorage key retained. Old user order preserved, snip
appended exactly once; no obsolete keys/IDs. Pointer reorder >=4px suppresses generated
pointer click, below-threshold/keyboard click starts capture. Fifth hotkey key `snipping`,
default Alt+S; old four names/IDs/chords retained. Existing legacy Alt+S binding must be
preserved and conflict visibly reported—not silently moved/defaulted/registered twice.
Settings button "Capture Screen snipping shortcut"; conflict text actionable, save keeps
prior settings if registration/save fails. Backend validates duplicates before persistence.

## Evidence layers / budget

Test owner: rendered Vitest fake-native flow and stale/gesture/action behavior; Node
route/order/default-hotkey behavior. Backend owner: coordinator adapter acceptance + safe
save integration below. Existing native clipboard13 independent pixel/exit evidence retained.
Leader: independent review and real-shell limits. Do not duplicate live clipboard reads
with mock claims. No browser install or dependencies, no interactive test prerequisite.

Rendered journey: real TopBar start -> real overlay reverse drag -> real preview Copy
rejection/retry -> Save cancel/error/success -> Dismiss. Native invokes mocked only at
transport; no replacement fake component/state machine. It is rendered fake-native
E2E, NOT native desktop E2E. Jsdom pointer events are synthetic, not actual OS mouse.

## Native coordinator adapters: authored against settled production interface

Production must expose its ACTUAL coordinator core independently of Tauri globals.
Module `snipping/coordinator.rs`; constructor receives adapters, not test-only
scenario code. Minimal dependencies: monotonic clock, monitor topology/capture,
concrete window registry/create/show/close, process clipboard owner, picker, safe file
publisher. Tests inject those boundaries and call SAME public coordinator operations
as commands. Do not add a second fake coordinator implementation or static source regex.
Concrete caller binding is injected by server wrapper, never constructed from IPC IDs.

`src-tauri/tests/snipping_coordinator.rs` directly imports production coordinator and
existing geometry/session/clipboard modules. Uses exact six ports and public methods
in amended spec13-54. Controlled monotonic clock, deferred capture/picker, window
concrete-instance Drop trace and independent synthetic PNG pixel decoder. No fake
coordinator. These Drop spies establish adapter-window ownership only, NOT native GDI
cleanup or direct observation of Vec frame/image deallocation.

Required executable scenarios before feature completion:
1. Start allowed owner, capture all monitors before show; wrong concrete owner, reused
   label+different instance, stale generation/captureId/monitor image request rejected
   before capture/window/clipboard/file effect. Exactly current own context readable.
2. Two monitors at unequal native scales/negative origin. Ready only all current overlays;
   stale readiness ignored, readiness timeout closes/drops all frames/windows. Failed
   capture/overlay creation/show/image decode/preview creation unwind exactly once.
3. First accepted begin binds selected monitor; later other monitor complete rejected.
   Forged scale/origin/monitor rejected; reverse CSS rect maps through trusted 1.25 scale:
   native origin(-1000,0), CSS rect(10,20,100,60) => physical(-988,25)..(-862,100),
   126x75 crop. Topology changes invalidate session before any publication.
4. Duplicate start/complete/cancel, stale worker completion, dispose/restart races cannot
   replace newer session, leak frames/windows, copy stale image or close new preview.
5. Preview caller only Copy/Save/Dismiss; current generation checks IMMEDIATELY before
   native publish. Serialize repeated operations; actual outcome passed through unchanged.
6. Controlled pending save picker pins crop/session; new start, copy/dismiss/save duplicate
   refused/serialized without holding global lock over picker; cancel/error releases pin.
   Late picker settlement cannot publish old/new session incorrectly.

Safe PNG native/integration tests (pending implementation API): expose backend-only
`publish_png(destination: &Path, png: &[u8]) -> Result<_, E>` in snipping/save.rs, E: Debug,
and `publish_png_with_failpoint(destination, png, SaveFailpoint)` under cfg(test), using
the SAME production publication path. Failpoints CreateSibling/WriteSibling/FlushSibling/
PublishSibling force rejection before final publish (PublishSibling before rename, NOT
an error fabricated after successful rename). No diagnostics exposed to renderer.
Names may be negotiated with backend owner; retain exact behavior tests and production
path equivalence. Test-only sibling fixture directory (synthetic image only); no user
picker automation/no autosave. Own fixture cleanup by Drop.
- Existing destination sentinel: picker cancel, encode/staging/flush/publish errors leave
  exact sentinel bytes unchanged. Do not truncate target before sibling complete.
- New destination: success creates exact decodable PNG, independent decoder compares all
  synthetic pixels/dimensions; failure leaves no destination and cleans staged sibling.
- Existing destination authorized overwrite: safe final publication replaces only after
  full bounded PNG ready. Staging sibling never follows symlink/reparse escape; failures
  before rename preserve target. Explicitly document any postcommit durability ambiguity.
- Unicode/space paths, permission failure, invalid selection/oversize, stale save pin,
  topology invalidation; no private paths/image content logged, no capture history/cache.

Historical initial inspection found neither coordinator nor save API. Four synthetic
safe-PNG tests targeted the above API; their initial missing-module RED is recorded below.
Backend recovery now reports save4 GREEN separately; not independently rerun here.
Coordinator tests are now authored; their current missing-module compile RED is below.
Native save readback and existing clipboard13 remain REQUIRED desktop boundary evidence.
Real shell/AppBar/manual pointer/hardware E2E is waived/unverified, not replaced by these.

## Observed RED evidence

- `npm run test:component -- tests/components/snipping-product.test.ts`: 21 collected,
  21 failed. Missing overlay/preview components, unknown snip route, absent TopBar/Settings
  accessible controls. Gesture/stale/preview assertions therefore NOT reached or proved.
- `npm run test:node`: 1208 tests, 1199 passed, 5 failed, 1 skipped, 3 todo. Four new
  snipping contract failures; fifth is pre-existing contextMenuVisualRegression CSS rule
  failure. Existing snip pure gesture tests passed. No new missing-export failure.
- `npm run cargo:test -- --test snipping_save`: compile RED, missing
  src/snipping/save.rs. Four safe-PNG test bodies not executed; no save readback claimed.
- Svelte CLI analysis attempted with no-install; package unavailable. No dependency added.
- Independent code-review delegation blocked by subagent depth limit. Leader must review
  artifacts/contract before implementation; test owner checked existing exports/default
  chords and existing png0.17 dependency, and isolated URL stub cleanup. No GREEN claim.

Latest small harness cleanup isolates URL constructor stub and restores globals; does
not alter RED reasons above. Required full native clipboard13 rerun follows relevant
production changes; not repeated during test-only preparation.

## Continuation checkpoint: real-core fixture landed

18 actual-core test functions now cover six groups plus approved replacement:
- Exact live top-bar identity denied before capture; reused overlay label/different ID,
  invalid/stale generation/captureId and wrong monitor denied before publication.
- All freeze before window creation/show; all readiness then armed; duplicate ready and
  buffered already-armed acknowledgement; caps/per-monitor/count/aggregate raw budget.
- Trusted1.25 negative-origin126x75 crop with independent nonuniform pixel oracle;
  second-monitor claim refused, invalid finite/area inputs and fresh topology rejection.
- Timeout, duplicate start/cancel/complete, partial create/context/show/armed/preview
  failures; stale readiness/completion after restart; deferred freeze expires without
  held core lock and late frames cannot install/close newer generation windows.
- Preview-only authority and unchanged existing clipboard metadata for all six native
  outcomes; successful save independently decodes exact crop and retains preview.
- Deferred picker pins original crop: concurrent start/copy/save/dismiss return Busy
  within a **2-second harness bound** while picker remains blocked (not a measured100ms
  promise). No hide/capture/publish/second picker effects. Cancel unpins; late picker after
  topology change cannot publish. Release guard and5-second adapter wait bound on panic.
- Unpinned preview replacement hides old before freeze, denies old actions while hidden;
  cancel/capture failure/readiness timeout restores same old concrete preview; success
  closes/drops old exactly once; late old callbacks cannot mutate new preview.

Command: `npm run cargo:test -- --test snipping_coordinator`.
Observed exit101: `couldn't read tests\\..\\src\\snipping\\coordinator.rs: The system
cannot find the file specified. (os error 2)`. **No test bodies executed.** Production
owners still in flight; no final validation/GREEN claim. Old14/core, clipboard13 and
safe-save4 not rerun in this preparation.

Bounded backend API compatibility handoff:
- Existing `session::Caller`, geometry types must be the SAME types used by core, not
  coordinator-local duplicates. Exact public names/methods/ports from amended spec.
- `ClipboardOutcome` alias/re-export of existing `clipboard_process::Outcome`, including
  Clone, metadata(), Rejected/Committed/Unknown/DurabilityLost/Superseded variants.
- `SaveCode` public error type; `SaveOutcome::{Saved,Cancelled,Error(SaveCode)}`. Fixture
  does not guess SaveCode variants; fake picker/publisher currently return success/cancel.
- `Snapshot` layout/retained-byte accounting seam remains unnamed in spec. Do NOT guess
  fields or allocate512MiB just to fabricate coverage: retained-old-crop staging-budget
  positive boundary and direct frame-drop counts remain required follow-up once owner
  exposes exact inspect/accounting fields. Raw cap rejection is authored, not a substitute.
- Native registry initialization buffer/exclusion/SW_SHOWNOACTIVATE are adapter-owned
  contracts; this core fixture checks context association, order and rollback, not actual
  Win32 capture exclusion/focus or early Tauri webview load callbacks.
- Public settings `validate_loaded_settings(value,missing_snipping)` signature input/
  return types and optional-fifth registry install seam still need bounded owner handoff
  for Rust migration/registration acceptance. No new parallel registry implementation.

Frontend fixture amended mechanically: realistic ready:false + targeted `snip:armed`,
canonical m0 labels/capture IDs; stale armed gate and already-armed race cases added,
image/ready failure cleanup, rendered migrated order + keyboard-click semantics,
pending Save/concurrent TopBar Busy transport, exact actionable legacy conflict. No
gesture/action/stale assertions removed. Native Busy guarantee belongs to actual core
tests, not mock transport. Strict generated-label Node case added. Existing21 intentions
retained; lifecycle/frame assertions remain unproved until real components run.

Independent review delegation again blocked by subagent depth limit; leader review
required before treating in-flight implementation as accepted. No production edits,
dependencies, live OS interaction, private images/cache or display changes.

Focused changed-fixture validation (NOT a rerun of existing21):
- `npm run test:component -- tests/components/snipping-product.test.ts -t 'decoded
  overlay cannot select|subscribes before buffered context'`: first exposed a TEST
  harness bug (`URL is not a constructor`). Fixed stub to a subclass of real URL with
  overridden create/revoke, not a prototype-only object; globals still restored.
  Rerun: **2 passed,25 skipped**,27 collected. Actual in-flight overlay satisfies these
  two armed/subscription behaviors at this snapshot only; remaining25 not run/proved.
- `node --test --test-name-pattern='canonical generation-local'
  tests/snippingProductContracts.test.mjs`: **1 passed** against existing dist-tests
  compiled snapshot. No fresh full Node/tsc or final product validation implied.
- Existing Phase1/core14, clipboard13, prior21-rendered/full Node and safe-PNG4 NOT
   rerun. Production owners remained active at that checkpoint; coordinator evidence is
   superseded by the replacement-owner run below.

## Replacement test-owner coordinator evidence (2026-10-01)

Production `coordinator.rs` appeared during dispatch; exact approved port signatures
compile directly. No aliases/fake coordinator needed. Tests import actual production
`../src/snipping/mod.rs`. Latest `npm run cargo:test -- --test snipping_coordinator`:
**21 collected, 16 passed, 5 failed, 0 ignored, 0 filtered**, elapsed0.42s. This is
executable assertion RED, **not** missing-module RED. Earlier smaller15-test run GREEN.
Only coordinator test file and this handoff edited; no production/dependencies/shell
launch, real picker, desktop capture, clipboard mutation or filesystem publication.

Current failures:
- `group3_first_monitor_binding_trusted_fractional_conversion_and_independent_png_crop`:
  leader-added ordering assertion requires automatic publication before preview creation;
  production does it after. Preserved assertion; leader must reconcile lifecycle wording.
  Independent exact126x75 synthetic crop pixel assertions pass before ordering assertion.
- `group5_topology_invalidated_preview_cannot_publish_clipboard`: changed trusted scale
  still invokes clipboard publisher (2 publications, expected unchanged1).
- `group6_topology_change_while_picker_pending_rejects_late_file_publication`: changed
  native monitor identity during deferred picker still reaches file publisher.
- `group6_reused_preview_label_during_picker_cannot_inherit_publication_authority`: old
  native-registry instance replaced under same label during picker still publishes file.
- `replacement_inspection_accounts_for_previous_concrete_preview_and_retained_crop`:
  `inspect().window_ids` omits retained previous preview; observation contract requires it.

Passing coverage now includes six groups, canonical/forged tokens, cross-monitor image
refusal and independent synthetic PNG frame readback; freeze-all before windows/show;
all-unique readiness and armed dedupe; monotonic4999/5000ms decode/readiness expiration;
preallocation count/dimension/pixel/aggregate raw caps; partial create/context/show/armed/
preview failure cleanup; first-monitor binding, topology-before-completion and NaN refusal;
duplicates/stale/expired-deferred-capture versus newer generation; preview-only actions and
unchanged six actual clipboard Outcome metadata variants; serialized deferred Copy;
deferred picker cancel/error/success and publisher failure release; replacement hide/
retain, cancel/capture/timeout/partial-show/preview failure restore and exact-once success
disposal; Coordinator Drop owns both current and previous registry resources.

Current harness bounds differ from historical checkpoint: **100ms channel probes**, with
**3-second fixture gate escape**, explicit gate release before result assertion/join.
No clock sleeps. Window registry owns concrete-resource Drop spies; closes counted
separately. Actual inspect reports zero frame/image retention after disposal. Early context
test proves core lock release/context association while emit blocks, NOT native Tauri
provisional-entry gate correctness. Maximum512MiB encode/staging budget not allocated or
proved; retained previous crop accounting observed. No direct native HWND/GDI/drop proof.

Review delegation blocked by nested-subagent depth limit; leader review still required.
Existing core14/native13/save4/frontend suites not rerun here. Final rendered journey,
independent native clipboard13 and actual-core/synthetic-PNG gates remain separate.
OS mouse/AppBar/mixed-DPI hardware/consumer paste remain **UNVERIFIED**.
