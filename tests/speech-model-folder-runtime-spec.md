# Speech folder import: minimal runtime test seam (source-owner handoff)

Status: design only. No production runtime, picker, or real Parakeet inference is verified here. `tests/speech-model-folder-handoff.md:1-105` is the settled acceptance contract; do not revise its product/API/security decisions.

## Source facts and seam

`speech_runtime.rs:558-669` currently couples the admission/epoch/cleanup/pool transaction to `AppHandle`, `State`, `WebviewWindow`, the file chooser, app-data lookup, and the real `ParakeetTDT` loader. The pool is `WarmModelState::{NotStarted,Loading,Ready(ParakeetTDT),InUse,Failed}`. There is no valid synthetic `Ready` model. `install_archive` accepts a staged loader callback and publishes only after it returns `Ok`; the new `install_directory` must retain that contract. `spawn_warm_model_async` may overwrite the pool only if its captured epoch still matches and import is not active. `SpeechController::start/stop` can drive recording/transcribing states in an embedded test without opening a microphone.

Implement the following **private, production-used** factoring in `speech_runtime.rs` (names may vary, types/ordering may not):

```rust
#[derive(Clone, Copy)]
enum ModelImportKind { Archive, Folder }

fn import_model_with<C, D, L, W>(
    state: &SpeechRuntimeState,
    kind: ModelImportKind,
    choose: C,
    app_data: D,
    load: L,
    retry_warmup: W,
) -> Result<ImportModelResponse, String>
where
    C: FnOnce() -> Result<Option<PathBuf>, String>,
    D: FnOnce() -> Result<PathBuf, String>,
    L: FnOnce(&std::path::Path) -> Result<parakeet_rs::ParakeetTDT, String>,
    W: FnOnce();
```

Use the **existing** `state.commit -> model_operation -> inner -> model_pool` lock order and busy predicate (shutdown, importing, InUse, capture, recording, transcribing). Set `importing=true` and increment `epoch` before calling `choose`; release all locks before chooser, disk, or ONNX work. Resolve `app_data` ONLY for `Some(path)`; invoke exactly one actual `speech_model_install::{install_archive,install_directory}` based on `kind`, with `load(stage)` in its validation callback, capturing the actual `ParakeetTDT` for pool publication after the installer returns. No synthetic verification token or bool may enter `WarmModelState::Ready`. Shared import guard should borrow `&SpeechRuntimeState` (not require `AppHandle`), clear `importing` on every post-admission exit including unwind, then invoke `retry_warmup` after releasing locks; the production callback calls `spawn_warm_model_async(app.clone())`, tests can count it without starting background OS work. Keep existing error text/status rules: on failure store error, on cancel keep previous error behavior, reset invalidated `Loading|Failed` to `NotStarted`, retain any real `Ready`; on successful install set `source=installed`, `missing=false`, `error=None`, swap the **actual loaded model** into Ready and destroy the old model outside locks. Preserve command's `Result<ImportModelResponse,String>` shape; status must be read after releasing the guard. Admission rejection must not call any dependency or trigger retry. Production chooses `archive: app.dialog().file().set_parent(&window).add_filter(...).blocking_pick_file()` or `folder: app.dialog().file().set_parent(&window).blocking_pick_folder()` and converts `FilePath::into_path()` to a local path, with no renderer path input. Tauri official v2 plugin source uses `blocking_pick_folder()` returning `Option<FilePath>`: https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/dialog/src/commands.rs . Keep Settings authorization **before** spawning the blocking task for both commands.

If practical, extract just the existing warmup completion's epoch/importing/shutdown guard and pool mutation into `finish_warm_model(state, epoch, source, missing, loaded: Result<ParakeetTDT,String>)` called by `spawn_warm_model_async`; tests can feed only the **error** variant. Do not invent a fake model, widen pool type, make installer generic over model, or move actual app-data path into renderer.

## Embedded tests and assertions (RED first)

Add `#[cfg(test)]` cases in the existing `speech_runtime.rs` test module to call `import_model_with` on a fresh `SpeechRuntimeState::default()` and owned `Temp/opencode` fixture, not a Tauri application or production profile. Set separate `Cell`/atomic counters on chooser, app-data resolver, loader, retry; assert each expected invocation count is nonzero when applicable. Use distinct source/app-data roots, the same three synthetic nonempty root files as the seven installer tests, and inspect actual `speech-models` generations/staging via the real installer. No test may call `Ok(ParakeetTDT)` for synthetic bytes.

1. For **both kinds**, set each actual guarded condition in state: controller Recording via `start`, Transcribing via `stop`, `WarmModelState::InUse`, `model_operation.importing`, `shutting_down`. Assert busy `Err`, no chooser/data/loader/retry call, zero new files, unchanged epoch/importing (for pre-existing importing, remains true). Do not create fake `Capture { stream }` merely for a capture-only check; that condition is covered by the existing shared predicate; live capture needs consent.
2. Folder chooser `Ok(None)`: assert response `cancelled=true`, epoch incremented once, `importing=false`, chooser once, data/loader never, no staging/generation writes, retry once; when pool starts `NotStarted`, no fake Ready claim. Repeat archive cancel to establish shared exclusivity and cleanup. Test `Err` from chooser/data resolver separately and check error/status and retry, no disk writes.
3. Folder chooser `Some(owned source)` + loader that reads all three staged bytes, confirms `resolve_installed_model` still points to a prior **synthetically installed** generation if present, then returns `Err`. Assert loader once, importer `Err`, error stored, importing cleared, retry once; previous disk generation still resolves byte-for-byte, new staging absent, source untouched, pool remains `NotStarted` (or unchanged pre-existing `InUse` cannot enter chooser). Repeat archive failing-loader route using an owned tar fixture to ensure archive handler actually traverses the same runtime transaction; existing archive installer 8 tests remain required.
4. Hold folder chooser at a barrier after reservation. Call archive importer concurrently: busy before its chooser/data/filesystem, without clearing first reservation. Release first chooser with `None`; assert exactly one retry and both operations' epoch/importing outcomes. Reverse archive/folder order. Use bounded channels/barriers, join threads and clean fixtures even on assertion failure.
5. Start a warmup epoch before a cancel/failing import, then call the extracted real warmup completion with its stale epoch and `Err`; assert it cannot change the import's error/source/missing/pool. For successful Ready retention and overwrite prevention, **do not assert these as proven** without a real model: error-only stale completion can prove epoch guard but cannot exercise `Ready(ParakeetTDT)` publication. Protect the production conditional with code review and a real-model run when separately authorized.

`SpeechRuntimeState::default()` owns real locks/controller/pool/op fields. Tests may mutate these private fields under locks to establish preconditions; this is a probe of the real production-used transaction, not a copy of its admission/status logic. A deliberate test-only *loader failure* is permitted; a test-only success bypass that renames invalid bytes then reports model ready is not. Avoid persisted pointers, microphone/network/shell launch, focus/clipboard, or access to any supplied external folder.

## Evidence boundary and run gates

Run focused runtime embedded test cases via `npm run cargo:test -- speech_runtime::tests::` (confirm their actual module path in test output; require each new body executed, none ignored); rerun `npm run cargo:test -- --test speech_model_directory` (seven bodies), `npm run cargo:test -- --test speech_model_install` (eight), `npm run cargo:test -- --test shell_app_acl` (five), `node --test tests/speechModelImportContracts.test.mjs` (six), and `npm run test:component -- tests/components/settings-speech-import.test.ts` (21). For native harmless transport, use the consent-independent opt-in native ACL harness command in the settled handoff, with process-scoped `JASONSHELL_ACL_NATIVE_TESTS=1` and no production shell. Run relevant compile/check gates after implementation; failures are not waived by this design. Read fixture counts from runner, not from filtered-out/ignored tests.

These automated cases establish real runtime admission, cancellation, cleanup, failure, disk rollback, and stale-epoch guards **only**; they do not establish success-path ONNX `Ready`, actual native folder picker selection, app activation, inference, or complete picker-to-rendered-UI E2E. Full feature E2E remains REQUIRED and UNVERIFIED until a source-owned isolated native runtime harness with a real authorized model exercises chooser -> owned persistent bytes -> runtime Ready -> rendered Settings and restart reuse. Real model/inference and live Tauri/Windows shell smoke require explicit human consent. Do not report those passes without execution; repository policy permits `Complete` for requested automated scope if all its gates pass and no concrete blocker exists, while listing manual/real-model gaps under Further info or Next Steps.
