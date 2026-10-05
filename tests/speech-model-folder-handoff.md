# Speech model folder import: RED-first acceptance handoff

Status: requested synthetic automated test scope GREEN after independent production
owner implementation and owner-installed cfg(test) bridge. Full native picker/model/UI
E2E remains UNVERIFIED. Production source/dependencies unchanged by test owner.

## CURRENT runtime seam + post-implementation acceptance evidence

Supersedes the prior missing-runtime-affordance paragraph below. Architect settled
`tests/speech-model-folder-runtime-spec.md`; production owner ses_f00e82d7cffeDVmUPxp07UXIHZ
implemented private production-used `import_model_with`/`finish_warm_model` and installed
this exact approved bridge inside existing `speech_runtime::tests`:

```rust
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/speech_model_folder_runtime.rs"));
```

Test owner authored fixture and runner ONLY. No test-owner production edits. Runner
executes actual `jason-shell` TEST BINARY, never main: exact module/name listing rejects
missing/zero/duplicate bodies, then serial actual runtime run requires every named body
`ok` and `10 passed;0 failed;0 ignored`. Each Cargo phase has100s bound and owned log
guard; no parallel fake runtime/validator/Ready type. Initial wrapper run was RED missing
cfg(test) bridge (1 failed,0 actual runtime bodies,33.74s), NOT compile-failure or executed
behavior failure. Owner then installed bridge; fixture compiled against actual settled
seams without type/compile failure. Do not retrospectively invent a compile RED.

Fresh current focused results:

| Command | Executed current evidence |
|---|---|
| `npm run cargo:test -- --test speech_model_folder_runtime` | wrapper1 passed; mandatory actual-bin listing+run ALL10 embedded bodies passed/none ignored;28.75s |
| `npm run cargo:test -- --test speech_model_directory` | 7 passed/0 failed/ignored/filtered;0.03s |
| `npm run cargo:test -- --test speech_model_install --test shell_app_acl` | archive8 passed0.19s;actual generated ACL5 passed0.07s;none ignored/filtered |
| `node --test tests/speechModelImportContracts.test.mjs` | 6 passed,0 failed/skipped/todo;76.64ms |
| `npm run test:component -- tests/components/settings-speech-import.test.ts` | 21 passed including4folder+17archive/status/style bodies;4.69s |
| opt-in native folder command below | 1 passed/0 failed/ignored,2 unrelated bodies filtered;4.86s,18.17s compile |

Native command: `npm run cargo:test -- --test shell_acl_native_ipc native_folder_import_settings_transport_preserves_denials -- --ignored --test-threads=1 --nocapture`,
with process-scoped `JASONSHELL_ACL_NATIVE_TESTS=1` restored afterward. Observed actual
Wry/WebView2 transport:4 hidden nonactivating frames/5 IPC outcomes,1 permitted Settings
folder marker,4 actual ACL denials,0 snip handler calls,owned profiles removed after child
exit. New command registration/generated production ACL now accepts Settings before its
harmless terminal; no actual folder picker or import occurs in this native test.

Ten exact runtime case names (prefix `speech_runtime::tests::`):
1. `speech_folder_runtime_busy_rejects_both_kinds_before_all_dependencies`: both kinds x
   five actual guard conditions; busy rejects chooser/data/loader/retry counts0, epoch
   unchanged/pre-existing importing retained. Controller Recording/Transcribing driven
   without microphone; InUse is admission precondition, NOT synthetic Ready construction.
2. `speech_folder_runtime_cancel_both_kinds_resets_invalidated_pool_and_releases_once`:
   both kinds x NotStarted/Loading/Failed; chooser1,data0,loader0,retry1; epoch+1,
   reservation cleared before retry/status, no app-data writes, Missing not Ready.
3. `speech_folder_runtime_chooser_error_releases_without_resolving_data_or_loading`:
   both kinds,counts1/0/0/1,error stored/no disk writes.
4. `speech_folder_runtime_data_error_occurs_only_after_selection_and_releases`:
   both kinds,counts1/1/0/1,error stored/no writes/source bytes unchanged.
5. `speech_folder_runtime_folder_loader_failure_preserves_prior_disk_generation`.
6. `speech_folder_runtime_archive_loader_failure_preserves_prior_disk_generation`:
   cases5/6 traverse ACTUAL corresponding installer; staged synthetic bytes readback,
   prior synthetic disk generation still selected DURING loader; loader Err,counts1/1/1/1,
   old disk byte-for-byte selected/new staging absent/source and archive unchanged/pool
   NotStarted. Prior disk generation is installed with archive callback Ok only for disk
   setup, never converted into runtime Ready or treated as ONNX-valid.
7. `speech_folder_runtime_folder_reservation_blocks_archive_without_clearing_owner`.
8. `speech_folder_runtime_archive_reservation_blocks_folder_without_clearing_owner`:
   cases7/8 bound channels to3s/2s and RAII release/join owned worker on assertion failure;
   first actual reserved chooser held, second busy with0dependencies, first importing
   retained until cancel,exactly1retry/1epoch increment/no disk writes.
9. `speech_folder_runtime_unwind_releases_admission_and_retries_without_disk`:
   both kinds,owned chooser panic caught; borrowed guard clears importing/retries once,
   no app-data/loader/Ready creation, locks remain usable.
10. `speech_folder_runtime_stale_error_warmup_cannot_overwrite_cancel_or_failed_import`:
   actual finish_warm_model fed Err with stale pre-import epoch after cancel/chooser
   failure; cannot change error/source/missing/pool; no fake warmup success or Ready claim.

Fixture source/app-data paths are distinct exclusively-owned Temp/opencode directories,
cleaned by Drop. All runtime loaders return Err for synthetic bytes. Assertions check
actual State/private locks/controller/epoch and independent disk readback; callback
counters forbid vacuous success. Chooser/retry/disk work runs outside admission locks.

TEST fixture repair: disk first6GREEN/1FAIL was mklink SETUP error before installer,
not production reparse acceptance. cmd built-in received slash-separated path segments.
Changed invocation ONLY to cmd /D /C raw_arg with quoted owned native-backslash paths;
assert creation success plus FILE_ATTRIBUTE_REPARSE_POINT (0x400), then actual installer
must reject before loader and target byte readback remains unchanged. Actual junction
creation/security check now passes; no ignore/fallback/skip. Required file symlink test
also passes. Neither test establishes ancestor replacement race resistance. Source
owner reports opened required handles reject reparse/deny write/delete sharing; that
is distinct from claiming ancestor replacement proof and remains review material.

Static archive tests were monolithic before source factoring. Updated ONLY to follow
production adapter -> shared transaction -> actual finish_warm_model: preserves exact
archive filter/folder picker kind routing, Settings authorization/path-free wrappers,
reservation before chooser, actual loader into validate callback/install branches before
Ready swap, busy/epoch guards, real warmup guard before mutation, borrowed reservation
clear/retry and status after release. Six tests now GREEN; no deleted safety requirement.

Production owner reports check/build/cargo:check GREEN; not redundantly executed by test
owner in this follow-up. No full-suite duplication. Independent test reviewer notified
in existing review session; verdict pending. Leader owns production review/whole-change
signoff. Requested automated acceptance established; actual native folder picker,
success-path Parakeet Ready/inference, full picker-to-rendered-Settings/restart E2E still
REQUIRED and UNVERIFIED, not waived or substituted by injected runtime/native markers.
Real-model runs remain separately consent-gated. No microphone/network/production
shell/user-folder/user-profile import/focus/capture/clipboard/display changes occurred.

## Owner API and behavior contract

Implement in actual `src-tauri/src/speech_model_install.rs`:

```rust
pub fn install_directory<F>(
    source: &std::path::Path,
    app_data: &std::path::Path,
    limits: InstallLimits,
    validate: F,
) -> Result<std::path::PathBuf, String>
where F: FnOnce(&std::path::Path) -> Result<(), String>;
```

Folder import COPIES into existing shared per-user app-local speech-model generations;
never store an external-source pointer, mutate/remove source, recursively search it or
import extras. Required root-level files ONLY: encoder-model.int8.onnx,
decoder_joint-model.int8.onnx, vocab.txt. All regular, nonempty, no symlink/reparse source
root or required file. Ignore config.json/nemo128.onnx/other extras, including nested
required names. Required bytes alone count toward file/aggregate caps; exact cap allowed.
Load actual staged ONNX before atomic generation rename. Any pre-publication failure
retains previous generation and cleans staging. Existing installed-first resolver remains.

Register `speech_runtime::import_speech_model_folder` with path-free Settings-only backend
authorization. Native folder picker only; no renderer source/directory request. Expose
`IPC_COMMANDS.importSpeechModelFolder: 'import_speech_model_folder'` and exported
`importSpeechModelFolder()` wrapper invoking that command with NO args. Return existing
ImportModelResponse cancelled/model shape; share archive reservation, busy/epoch,
warmup invalidation, loader/publish/ready, retry, shutdown and error semantics. Native
chooser must use folder mode, not filtered archive mode.

UI: adjacent shared-style button EXACT accessible name `Import speech model folder`;
preserve `Import speech model` and unrelated controls. Both buttons share busy lock in
BOTH directions. Folder selection/cancel/error/success should use existing truthful
status/error/installed-ready lifecycle; selected folder is not a persistent runtime source.

Build derives registered handler inventory and legacy permissions. Register one command
and rebuild actual generated ACL; expected inventory203/legacy192 if no other owner work
adds commands. Tests use dynamic actual registration, not a hardcoded count substitution.
Settings must resolve new app command; remote/unknown/snipping frames deny. Shared legacy
grant does NOT replace backend Settings-only authorization on other legacy labels. Do NOT
disable application ACL, globally grant command/snipping, loosen snip partitions, or patch
generated JSON instead of real build inputs.

## Executed RED and unaffected evidence

Logs are bounded targeted runs in approved Temp/opencode; no full-suite duplication.

| Command | Observed result |
|---|---|
| `npm run cargo:test -- --test speech_model_directory` | compile RED E0432 missing actual `install_directory`; NO disk bodies executed |
| `npm run test:component -- tests/components/settings-speech-import.test.ts` | 21 collected:17 existing passed,4 new failed missing folder button;20.27s |
| `node --test tests/speechModelImportContracts.test.mjs` | 5 existing passed,1 new failed missing IPC command key;87ms |
| `npm run cargo:test -- --test shell_app_acl` | 4 existing passed,1 new failed actual RuntimeAuthority Settings grant;0.06s |
| `npm run cargo:test -- --test speech_model_install` | unchanged archive baseline8 passed,0 failed/ignored;0.21s |
| opt-in native folder command below | actual native RED Settings renderer invoke denied by Tauri;1 failed/2 filtered,4.91s;parent post-exit profile cleanup asserted BEFORE failure report |
| opt-in existing native mode below | actual native1 passed/2 filtered,5.49s;5 frames,4 markers,7 ACL denials,snip calls0,post-exit profiles removed |

Native runs used process-scoped `JASONSHELL_ACL_NATIVE_TESTS=1`, restored afterward:

```
npm run cargo:test -- --test shell_acl_native_ipc native_folder_import_settings_transport_preserves_denials -- --ignored --test-threads=1 --nocapture
npm run cargo:test -- --test shell_acl_native_ipc native_crosssurface_ipc_reaches_only_authorized_harmless_terminals -- --ignored --test-threads=1 --nocapture
```

Folder mode reuses actual native Wry/WebView2 harness with unchanged generated production
ACL, never mocks RuntimeAuthority or loads production shell/modules. New test-owned
terminal returns caller marker ONLY. Four hidden/nonactivating frames complete FIVE
renderer IPC outcomes: local Settings folder positive/start_snip negative; unknown folder
negative; owned remote-loopback control-plane folder negative; local snip-preview folder
negative. Host exact label/command oracle requires one marker/four denials, live IPC bridge,
permission-classified errors, no snip handler entry, real owned-profile cleanup. Current
Settings positive goes RED as intended, not bridge/network failure. This is actual
native transport evidence, NOT native picker or model import runtime E2E.

## Coverage inventory and missed-coverage guard

New `speech_model_directory.rs` includes production installer DIRECTLY, never a fake:
seven Windows bodies for complete root+extras/nonrecursive, independent staged and
published byte readback/source unchanged+deletable, invalid/missing/empty/nonfile/nested,
file/aggregate cap rejection and exact cap acceptance, loader failure old generation
retention/staging cleanup, actual owned directory junction rejection, required file
symlink rejection. Synthetic byte payloads are deliberately NOT usable ONNX models.
Validation callback proves staging placement/order but substitutes for inference only.
Disk compile RED is prerequisite failure; NEVER claim these seven bodies passed until
API lands and runner reports seven passed/zero ignored (plus any owner-added tests).
Symlink fixture creation requires Windows privilege/developer mode; it hard-fails rather
than silently skipping if unavailable. Junction test uses cmd mklink /J only on exclusive
owned fixture directories; removes junction before target cleanup. No user's folder is read.

Four new mounted actual Settings bodies cover exact no-args IPC, both-direction busy,
cancel preserves ready, failure/retry preserves ready, immediate ready and remount ready.
Native IPC is MOCKED there. Seventeen archive/status/stale-poll/unmount/style tests remain
unchanged. Require all21, not a folder-only filter silently losing old behavior.
Static new command contract is inventory/boundary evidence only; require6 bodies.
Actual generated resolver require5 bodies, including all4 old snip/legacy tests; native
folder require1 completed body/four frames/five outcomes, not ignored/default collection.
Archive baseline require8 bodies. Do not satisfy tests with missing-API guards/skips,
fake callbacks returning Ready in production, copied installer/validator or altered caps.

## Remaining critical runtime affordance / E2E handoff

Not authored/executed yet: ACTUAL SpeechRuntime folder admission/cancel/error/publication
and end-to-end picker -> shared persistent bytes -> runtime status -> rendered UI. Current
private blocking importer directly binds AppHandle/global State/native chooser/real ONNX
loader, so synthetic disk tests and mocked mounted Settings cannot drive this boundary.
Minimal source-owner verification seam is required, NOT test-owner production changes:

- Factor one production-used archive/folder import operation with injected chooser and
  loader ONLY at test boundary, preserving actual State/commit/importing/epoch/model_pool
  transitions and actual install_directory. Supply scoped app-data root for owned fixtures.
- Expose test-only admission/status or use embedded #[cfg(test)] bodies to drive actual
  state, not a parallel fake runtime. Test busy recording/transcribing/InUse/importing/
  shutdown reject BEFORE chooser and filesystem; shared archive/folder exclusivity;
  cancel zero writes/reservation cleared/old Ready retained; staged loader failure old disk
  and Ready pool preserved; late warmup epoch cannot overwrite successful imported Ready.
  Assertion counters must reject zero collected bodies/empty seam invocation.
- Synthetic loader can check bytes and return a scoped verification handle, but must not
  claim actual Parakeet inference; owner must design seam to avoid creating fake production
  ready models. Current pool stores real ParakeetTDT so do not guess a substitute type.
- Real native picker acceptance is manual/consent-dependent, not established by above
  native harmless terminal. Need isolated scoped native runtime harness from source owner
  before automated end-to-end integration; remain no mic/network/shell/focus/private pixels.
  Real model import or inference needs separate consent; do not import the supplied
  C:/dev repository folder in tests, write real shared profile or launch production main.

Full feature E2E remains REQUIRED and UNVERIFIED. This RED-authoring handoff does not
declare feature complete. Native folder transport has been executed RED; actual Windows
application activation from previous work remains unrelated manual pending controlled restart.

Review delegation attempted; nested reviewer blocked by subagent-depth limit. Leader
independent review and seam handoff remain required. No production source, deps, user
files, microphone, shell restart, focus, capture, clipboard or display changes made.
