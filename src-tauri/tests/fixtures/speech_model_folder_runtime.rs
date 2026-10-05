// Included ONLY inside speech_runtime::tests by source owner. Actual private
// State/transaction/installer; chooser + Err-only loader replace native boundaries.
const FOLDER_RUNTIME_FILES: [&str; 3] = ["encoder-model.int8.onnx", "decoder_joint-model.int8.onnx", "vocab.txt"];
const FOLDER_RUNTIME_BYTES: [&[u8]; 3] = [b"owned encoder", b"owned decoder", b"owned vocabulary"];
static FOLDER_RUNTIME_NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct FolderRuntimeFixture(std::path::PathBuf);
impl FolderRuntimeFixture {
    fn new() -> Self {
        let path = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp/opencode")
            .join(format!("speech-runtime-folder-{}-{}", std::process::id(), FOLDER_RUNTIME_NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap(); Self(path)
    }
    fn data(&self) -> std::path::PathBuf { self.0.join("shared-app-data") }
    fn source(&self) -> std::path::PathBuf {
        let path = self.0.join("source"); std::fs::create_dir(&path).unwrap();
        for (name, bytes) in FOLDER_RUNTIME_FILES.iter().zip(FOLDER_RUNTIME_BYTES) { std::fs::write(path.join(name), bytes).unwrap(); } path
    }
    fn archive(&self) -> std::path::PathBuf {
        let path = self.0.join("synthetic.tar"); let mut tar = tar::Builder::new(std::fs::File::create(&path).unwrap());
        for (name, bytes) in FOLDER_RUNTIME_FILES.iter().zip(FOLDER_RUNTIME_BYTES) {
            let mut header = tar::Header::new_gnu(); header.set_size(bytes.len() as u64); header.set_mode(0o600); header.set_cksum();
            tar.append_data(&mut header, name, bytes).unwrap();
        }
        tar.finish().unwrap(); path
    }
    fn no_writes(&self) { assert!(!self.data().exists(), "cancel/rejection must not even create app-data root"); }
    fn no_staging(&self) {
        assert!(std::fs::read_dir(self.data().join("speech-models")).unwrap().all(|entry| !entry.unwrap().file_name().to_string_lossy().starts_with("staging-")));
    }
}
impl Drop for FolderRuntimeFixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
#[derive(Default)]
struct FolderRuntimeCounts {
    chooser: std::cell::Cell<usize>, data: std::cell::Cell<usize>, loader: std::cell::Cell<usize>, retry: std::cell::Cell<usize>,
}
impl FolderRuntimeCounts {
    fn assert(&self, expected: [usize; 4]) { assert_eq!([self.chooser.get(), self.data.get(), self.loader.get(), self.retry.get()], expected); }
}
fn folder_runtime_readback(path: &std::path::Path) {
    for (name, bytes) in FOLDER_RUNTIME_FILES.iter().zip(FOLDER_RUNTIME_BYTES) { assert_eq!(std::fs::read(path.join(name)).unwrap(), bytes); }
}
fn folder_runtime_epoch(state: &SpeechRuntimeState) -> u64 { state.model_operation.lock().unwrap().epoch }
fn folder_runtime_missing(state: &SpeechRuntimeState) {
    assert!(matches!(*state.model_pool.lock().unwrap(), WarmModelState::NotStarted));
    assert_eq!(model_status(state).unwrap().state, "missing");
    assert!(!state.model_operation.lock().unwrap().importing);
}
#[test]
fn speech_folder_runtime_busy_rejects_both_kinds_before_all_dependencies() {
    for kind in [ModelImportKind::Archive, ModelImportKind::Folder] { for condition in 0..5 {
        let f = FolderRuntimeFixture::new(); let state = SpeechRuntimeState::default(); let counts = FolderRuntimeCounts::default();
        state.model_operation.lock().unwrap().epoch = 19;
        match condition {
            0 | 1 => { let mut inner = state.inner.lock().unwrap(); inner.controller.start(std::time::Duration::ZERO).unwrap();
                if condition == 1 { inner.controller.stop(crate::speech::SpeechSessionNonce(1), std::time::Duration::from_millis(1)).unwrap(); } },
            2 => *state.model_pool.lock().unwrap() = WarmModelState::InUse,
            3 => state.model_operation.lock().unwrap().importing = true,
            _ => state.shutting_down.store(true, std::sync::atomic::Ordering::Release),
        }
        let result = import_model_with(&state, kind,
            || { counts.chooser.set(counts.chooser.get()+1); Ok(None) },
            || { counts.data.set(counts.data.get()+1); Ok(f.data()) },
            |_| { counts.loader.set(counts.loader.get()+1); Err("never create synthetic Ready".into()) },
            || counts.retry.set(counts.retry.get()+1));
        assert!(result.err().unwrap().to_ascii_lowercase().contains("busy")); counts.assert([0,0,0,0]); f.no_writes();
        assert_eq!(folder_runtime_epoch(&state), 19); assert_eq!(state.model_operation.lock().unwrap().importing, condition == 3);
    }}
}
#[test]
fn speech_folder_runtime_cancel_both_kinds_resets_invalidated_pool_and_releases_once() {
    for kind in [ModelImportKind::Archive, ModelImportKind::Folder] { for pool in 0..3 {
        let f = FolderRuntimeFixture::new(); let state = SpeechRuntimeState::default(); let counts = FolderRuntimeCounts::default();
        *state.model_pool.lock().unwrap() = match pool { 0 => WarmModelState::NotStarted, 1 => WarmModelState::Loading, _ => WarmModelState::Failed };
        { let mut op = state.model_operation.lock().unwrap(); op.epoch = 9; op.error = Some("existing error".into()); }
        let response = import_model_with(&state, kind,
            || { counts.chooser.set(counts.chooser.get()+1); assert!(state.model_operation.lock().unwrap().importing);
                assert_eq!(folder_runtime_epoch(&state), 10); assert!(state.commit.try_lock().is_ok()); Ok(None) },
            || { counts.data.set(counts.data.get()+1); Ok(f.data()) },
            |_| { counts.loader.set(counts.loader.get()+1); Err("never called".into()) },
            || { assert!(!state.model_operation.lock().unwrap().importing); assert!(state.commit.try_lock().is_ok()); counts.retry.set(counts.retry.get()+1); }).unwrap();
        assert!(response.cancelled); assert_eq!(response.model.state, "missing"); counts.assert([1,0,0,1]); f.no_writes();
        assert_eq!(folder_runtime_epoch(&state), 10); assert_eq!(response.model.error.as_deref(), Some("existing error")); folder_runtime_missing(&state);
    }}
}
#[test]
fn speech_folder_runtime_chooser_error_releases_without_resolving_data_or_loading() {
    for kind in [ModelImportKind::Archive, ModelImportKind::Folder] {
        let f = FolderRuntimeFixture::new(); let state = SpeechRuntimeState::default(); let counts = FolderRuntimeCounts::default();
        let result = import_model_with(&state, kind,
            || { counts.chooser.set(counts.chooser.get()+1); Err("owned chooser error".into()) },
            || { counts.data.set(counts.data.get()+1); Ok(f.data()) },
            |_| { counts.loader.set(counts.loader.get()+1); Err("never called".into()) }, || counts.retry.set(counts.retry.get()+1));
        assert_eq!(result.err().as_deref(), Some("owned chooser error")); counts.assert([1,0,0,1]); f.no_writes();
        assert_eq!(folder_runtime_epoch(&state), 1); folder_runtime_missing(&state);
        assert_eq!(model_status(&state).unwrap().error.as_deref(), Some("owned chooser error"));
    }
}
#[test]
fn speech_folder_runtime_data_error_occurs_only_after_selection_and_releases() {
    for kind in [ModelImportKind::Archive, ModelImportKind::Folder] {
        let f = FolderRuntimeFixture::new(); let source = f.source(); let state = SpeechRuntimeState::default(); let counts = FolderRuntimeCounts::default();
        let result = import_model_with(&state, kind,
            || { counts.chooser.set(counts.chooser.get()+1); Ok(Some(source.clone())) },
            || { counts.data.set(counts.data.get()+1); Err("owned app-data error".into()) },
            |_| { counts.loader.set(counts.loader.get()+1); Err("never called".into()) }, || counts.retry.set(counts.retry.get()+1));
        assert_eq!(result.err().as_deref(), Some("owned app-data error")); counts.assert([1,1,0,1]); f.no_writes();
        folder_runtime_readback(&source); folder_runtime_missing(&state); assert_eq!(folder_runtime_epoch(&state), 1);
        assert_eq!(model_status(&state).unwrap().error.as_deref(), Some("owned app-data error"));
    }
}
fn folder_runtime_loader_failure(kind: ModelImportKind) {
    let f = FolderRuntimeFixture::new(); let source = f.source(); let archive = f.archive();
    // Baseline disk publication only; callback Ok here NEVER creates runtime Ready.
    let old = crate::speech_model_install::install_archive(&archive, &f.data(), crate::speech_model_install::InstallLimits::default(), |_| Ok(())).unwrap();
    let chosen = match kind { ModelImportKind::Archive => archive.clone(), ModelImportKind::Folder => source.clone() };
    let original_archive = std::fs::read(&archive).unwrap(); let state = SpeechRuntimeState::default(); let counts = FolderRuntimeCounts::default();
    *state.model_pool.lock().unwrap() = WarmModelState::Loading;
    let result = import_model_with(&state, kind,
        || { counts.chooser.set(counts.chooser.get()+1); Ok(Some(chosen)) },
        || { counts.data.set(counts.data.get()+1); Ok(f.data()) },
        |stage| { counts.loader.set(counts.loader.get()+1); assert!(stage.starts_with(f.data())); assert_ne!(stage, source); assert_ne!(stage, old);
            folder_runtime_readback(stage); assert_eq!(std::fs::read_dir(stage).unwrap().count(), 3);
            assert_eq!(crate::speech_model_install::resolve_installed_model(&f.data()), Some(old.clone()));
            assert!(state.commit.try_lock().is_ok()); assert!(state.model_pool.try_lock().is_ok()); Err("deliberate synthetic ONNX rejection".into()) },
        || { assert!(!state.model_operation.lock().unwrap().importing); counts.retry.set(counts.retry.get()+1); });
    let error = result.err().expect("synthetic loader cannot report Ready"); assert!(!error.is_empty()); counts.assert([1,1,1,1]);
    assert_eq!(model_status(&state).unwrap().error.as_deref(), Some(error.as_str())); folder_runtime_missing(&state);
    assert_eq!(folder_runtime_epoch(&state), 1); assert_eq!(crate::speech_model_install::resolve_installed_model(&f.data()), Some(old.clone()));
    folder_runtime_readback(&old); folder_runtime_readback(&source); assert_eq!(std::fs::read(archive).unwrap(), original_archive); f.no_staging();
    assert_eq!(std::fs::read_dir(f.data().join("speech-models")).unwrap().count(), 1);
}
#[test]
fn speech_folder_runtime_folder_loader_failure_preserves_prior_disk_generation() { folder_runtime_loader_failure(ModelImportKind::Folder); }
#[test]
fn speech_folder_runtime_archive_loader_failure_preserves_prior_disk_generation() { folder_runtime_loader_failure(ModelImportKind::Archive); }

// RAII release/join prevents an assertion failure from stranding the owned worker.
struct FolderRuntimeWorker {
    release: Option<std::sync::mpsc::Sender<()>>,
    join: Option<std::thread::JoinHandle<()>>,
}
impl Drop for FolderRuntimeWorker { fn drop(&mut self) {
    if let Some(tx) = self.release.take() { let _ = tx.send(()); }
    if let Some(join) = self.join.take() { let _ = join.join(); }
} }
fn folder_runtime_exclusive(first: ModelImportKind, second: ModelImportKind) {
    let f = FolderRuntimeFixture::new(); let state = std::sync::Arc::new(SpeechRuntimeState::default());
    let (entered_tx, entered_rx) = std::sync::mpsc::channel(); let (release_tx, release_rx) = std::sync::mpsc::channel();
    let retries = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)); let copied = std::sync::Arc::clone(&state); let retried = std::sync::Arc::clone(&retries);
    let join = std::thread::spawn(move || {
        let result = import_model_with(&copied, first,
            || { entered_tx.send(()).unwrap(); release_rx.recv_timeout(std::time::Duration::from_secs(3)).expect("bounded chooser release"); Ok(None) },
            || panic!("cancel must not resolve data"), |_| Err("cancel must not load".into()),
            || { retried.fetch_add(1, std::sync::atomic::Ordering::Relaxed); });
        assert!(result.unwrap().cancelled);
    });
    let mut worker = FolderRuntimeWorker { release: Some(release_tx), join: Some(join) };
    entered_rx.recv_timeout(std::time::Duration::from_secs(2)).expect("actual chooser entered after admission");
    assert!(state.model_operation.lock().unwrap().importing); assert_eq!(folder_runtime_epoch(&state), 1);
    let counts = FolderRuntimeCounts::default(); let rejected = import_model_with(&state, second,
        || { counts.chooser.set(counts.chooser.get()+1); Ok(None) }, || { counts.data.set(counts.data.get()+1); Ok(f.data()) },
        |_| { counts.loader.set(counts.loader.get()+1); Err("never called".into()) }, || counts.retry.set(counts.retry.get()+1));
    assert!(rejected.err().unwrap().to_ascii_lowercase().contains("busy")); counts.assert([0,0,0,0]);
    assert!(state.model_operation.lock().unwrap().importing); assert_eq!(folder_runtime_epoch(&state), 1);
    worker.release.take().unwrap().send(()).unwrap(); worker.join.take().unwrap().join().unwrap();
    assert_eq!(retries.load(std::sync::atomic::Ordering::Relaxed), 1); folder_runtime_missing(&state); f.no_writes();
}
#[test]
fn speech_folder_runtime_folder_reservation_blocks_archive_without_clearing_owner() { folder_runtime_exclusive(ModelImportKind::Folder, ModelImportKind::Archive); }
#[test]
fn speech_folder_runtime_archive_reservation_blocks_folder_without_clearing_owner() { folder_runtime_exclusive(ModelImportKind::Archive, ModelImportKind::Folder); }
#[test]
fn speech_folder_runtime_unwind_releases_admission_and_retries_without_disk() {
    for kind in [ModelImportKind::Archive, ModelImportKind::Folder] {
        let f = FolderRuntimeFixture::new(); let state = SpeechRuntimeState::default(); let retry = std::cell::Cell::new(0);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| import_model_with(&state, kind,
            || -> Result<Option<std::path::PathBuf>, String> { panic!("owned chooser unwind") },
            || panic!("unwind must not resolve data"), |_| Err("never load".into()), || retry.set(retry.get()+1))));
        assert!(panic.is_err()); assert_eq!(retry.get(), 1); assert_eq!(folder_runtime_epoch(&state), 1); folder_runtime_missing(&state); f.no_writes();
    }
}
#[test]
fn speech_folder_runtime_stale_error_warmup_cannot_overwrite_cancel_or_failed_import() {
    for failed in [false, true] {
        let f = FolderRuntimeFixture::new(); let state = SpeechRuntimeState::default();
        { let mut op = state.model_operation.lock().unwrap(); op.epoch = 23; op.source = Some("installed"); op.missing = true; op.error = Some("retained error".into()); }
        *state.model_pool.lock().unwrap() = WarmModelState::Loading;
        let retry = std::cell::Cell::new(0);
        let result = import_model_with(&state, ModelImportKind::Folder,
            || if failed { Err("owned failed import".into()) } else { Ok(None) },
            || panic!("no data on failure/cancel"), |_| Err("never load".into()), || retry.set(retry.get()+1));
        assert_eq!(result.is_err(), failed); assert_eq!(retry.get(), 1); assert_eq!(folder_runtime_epoch(&state), 24);
        let before = model_status(&state).unwrap();
        finish_warm_model(&state, 23, Some("bundled"), false, Err("stale warmup error".into()));
        let after = model_status(&state).unwrap(); assert_eq!((after.state, after.source, after.error), (before.state, before.source, before.error));
        assert!(state.model_operation.lock().unwrap().missing); folder_runtime_missing(&state); f.no_writes();
    }
}
