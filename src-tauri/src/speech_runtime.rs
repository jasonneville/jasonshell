//! Bounded native microphone capture and in-process speech worker.

use crate::contracts;
use crate::speech::{
    CopySpeechHistoryTranscriptRequest, SpeechController, SpeechHistoryEntry, SpeechSessionNonce,
    SpeechStatusEvent, SpeechStatusKind, SpeechVoiceLevelEvent, StartSpeechCaptureRequest,
    StartSpeechCaptureResponse, StopSpeechCaptureRequest, MAX_RECORDING_DURATION,
    MAX_SPEECH_HISTORY_ENTRIES, MAX_SPEECH_HISTORY_TRANSCRIPT_BYTES,
};
use crate::speech_streaming::{
    merge_tdt_window, BoundedIntake, InputCloseReason, IntakeResult, WorkerMessage,
    FINALIZATION_TIMEOUT, OVERLAP_SEGMENTS, SEGMENT_SAMPLES, WINDOW_SEGMENTS,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use parakeet_rs::Transcriber;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

const TARGET_RATE: u32 = 16_000;
const MODEL_RESOURCE: &str = "resources/speech-models/parakeet-tdt-0.6b-v2-int8";
const GENERIC_ERROR: &str = "Speech operation failed";
const TERMINAL_RESET_DELAY: Duration = Duration::from_millis(1500);
const MIN_CAPTURE_SAMPLES: usize = 1_600;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SpeechFailure {
    CaptureStreamError,
    CaptureCallbackLoss,
    CaptureEmpty,
    CaptureShort,
    ModelLoadFailed,
    AsrInputInvalid,
    AsrFailed,
    TranscriptEmpty,
    Clipboard(crate::speech_clipboard::ClipboardFailure),
    Timeout,
    StateRace,
}

impl SpeechFailure {
    const fn code(self) -> &'static str {
        match self {
            Self::CaptureStreamError => "capture-stream-error",
            Self::CaptureCallbackLoss => "capture-callback-loss",
            Self::CaptureEmpty => "capture-empty",
            Self::CaptureShort => "capture-short",
            Self::ModelLoadFailed => "model-load-failed",
            Self::AsrInputInvalid => "asr-input-invalid",
            Self::AsrFailed => "asr-failed",
            Self::TranscriptEmpty => "transcript-empty",
            Self::Clipboard(failure) => failure.code(),
            Self::Timeout => "timeout",
            Self::StateRace => "state-race",
        }
    }
}

fn map_paste_failure(
    failure: crate::speech_target::SpeechPasteFailure,
) -> crate::speech_clipboard::ClipboardFailure {
    use crate::speech_clipboard::ClipboardFailure;
    use crate::speech_target::SpeechPasteFailure;
    match failure {
        SpeechPasteFailure::TargetUnavailable => ClipboardFailure::PasteTargetUnavailable,
        SpeechPasteFailure::TargetChanged => ClipboardFailure::PasteTargetChanged,
        SpeechPasteFailure::FocusDenied => ClipboardFailure::PasteFocusDenied,
        SpeechPasteFailure::InputRejected => ClipboardFailure::PasteInputRejected,
    }
}

#[derive(Default)]
struct CaptureHealth {
    stream_error: AtomicBool,
    callback_drops: AtomicU64,
}

struct Capture {
    stream: Stream,
    intake: Arc<BoundedIntake>,
    tail: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    health: Arc<CaptureHealth>,
    meter: Arc<AtomicU32>,
}

pub(crate) struct SpeechRuntimeState {
    inner: Mutex<RuntimeInner>,
    generation: AtomicU64,
    commit: Mutex<()>,
    /// Orders native indicator actions without extending the lifecycle commit gate.
    visibility_order: Mutex<()>,
    /// Serializes cancellation with the native clipboard/focus/input delivery.
    /// It is never held with `inner` or `history` during the native operations.
    delivery: Mutex<()>,
    focus_admission: Mutex<FocusAdmission>,
    focus_drained: Condvar,
    shutting_down: AtomicBool,
    epoch: Instant,
    history: Mutex<VecDeque<SpeechHistoryEntry>>,
    model_pool: Mutex<WarmModelState>,
    model_operation: Mutex<ModelOperation>,
}

#[derive(Default)]
struct FocusAdmission {
    closed: bool,
    in_flight: usize,
}

struct FocusLease<'a>(&'a SpeechRuntimeState);

impl Drop for FocusLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut admission) = self.0.focus_admission.lock() {
            admission.in_flight -= 1;
            self.0.focus_drained.notify_all();
        }
    }
}

enum WarmModelState {
    NotStarted,
    Loading,
    Ready(parakeet_rs::ParakeetTDT),
    InUse,
    Failed,
}

#[derive(Default)]
struct ModelOperation {
    importing: bool,
    epoch: u64,
    source: Option<&'static str>,
    error: Option<String>,
    missing: bool,
}

#[derive(Clone, serde::Serialize)]
pub(crate) struct ModelStatus {
    state: &'static str,
    source: Option<&'static str>,
    error: Option<String>,
}

#[derive(serde::Serialize)]
pub(crate) struct ImportModelResponse {
    cancelled: bool,
    model: ModelStatus,
}

struct RuntimeInner {
    controller: SpeechController,
    capture: Option<Capture>,
    paste_target: Option<SessionPasteTarget>,
    prepared: Option<PreparedPasteTarget>,
    next_reservation: u64,
}

struct PreparedPasteTarget {
    reservation_id: u64,
    target: Option<crate::speech_target::SpeechPasteTarget>,
    prepared_at: Instant,
}

struct SessionPasteTarget {
    nonce: SpeechSessionNonce,
    target: Option<crate::speech_target::SpeechPasteTarget>,
}

impl RuntimeInner {
    fn matches_prepared_request(&self, request: StartSpeechCaptureRequest) -> bool {
        self.prepared.as_ref().is_some_and(|prepared| {
            prepared.reservation_id == request.reservation_id
                && prepared.prepared_at.elapsed() < Duration::from_secs(2)
        })
    }

    fn take_session_target(&mut self, nonce: SpeechSessionNonce) -> Option<SessionPasteTarget> {
        if self
            .paste_target
            .as_ref()
            .is_some_and(|slot| slot.nonce == nonce)
        {
            self.paste_target.take()
        } else {
            None
        }
    }
}

impl Default for SpeechRuntimeState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(RuntimeInner {
                controller: SpeechController::default(),
                capture: None,
                paste_target: None,
                prepared: None,
                next_reservation: 0,
            }),
            generation: AtomicU64::new(0),
            commit: Mutex::new(()),
            visibility_order: Mutex::new(()),
            delivery: Mutex::new(()),
            focus_admission: Mutex::new(FocusAdmission::default()),
            focus_drained: Condvar::new(),
            shutting_down: AtomicBool::new(false),
            epoch: Instant::now(),
            history: Mutex::new(VecDeque::with_capacity(MAX_SPEECH_HISTORY_ENTRIES)),
            model_pool: Mutex::new(WarmModelState::NotStarted),
            model_operation: Mutex::new(ModelOperation::default()),
        }
    }
}

fn bounded_transcript(text: &str) -> String {
    let mut end = text.len().min(MAX_SPEECH_HISTORY_TRANSCRIPT_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

#[cfg(test)]
fn record_clipboard_attempt<F>(
    state: &SpeechRuntimeState,
    nonce: SpeechSessionNonce,
    transcript: &str,
    publish: F,
) -> Result<(), crate::speech_clipboard::ClipboardFailure>
where
    F: FnOnce(&SpeechHistoryEntry) -> Result<(), crate::speech_clipboard::ClipboardFailure>,
{
    let inserted = insert_history_attempt(state, nonce, transcript)
        .map_err(|_| crate::speech_clipboard::ClipboardFailure::PublishRejected)?;
    let result = publish(&inserted);
    update_history_outcome(
        state,
        nonce,
        match result {
            Ok(()) => "copied",
            Err(failure) => failure.code(),
        },
    )
    .map_err(|_| crate::speech_clipboard::ClipboardFailure::PublishRejected)?;
    result
}

fn insert_history_attempt(
    state: &SpeechRuntimeState,
    nonce: SpeechSessionNonce,
    transcript: &str,
) -> Result<SpeechHistoryEntry, ()> {
    let mut history = state.history.lock().map_err(|_| ())?;
    let entry = SpeechHistoryEntry {
        nonce,
        transcript: bounded_transcript(transcript),
        outcome: String::new(),
    };
    history.push_front(entry.clone());
    history.truncate(MAX_SPEECH_HISTORY_ENTRIES);
    Ok(entry)
}

fn update_history_outcome(
    state: &SpeechRuntimeState,
    nonce: SpeechSessionNonce,
    outcome: &str,
) -> Result<(), ()> {
    let mut history = state.history.lock().map_err(|_| ())?;
    let entry = history
        .iter_mut()
        .find(|entry| entry.nonce == nonce)
        .ok_or(())?;
    entry.outcome = outcome.to_owned();
    Ok(())
}

#[tauri::command]
pub(crate) fn get_speech_history(
    state: State<'_, SpeechRuntimeState>,
) -> Result<Vec<SpeechHistoryEntry>, String> {
    state
        .history
        .lock()
        .map(|history| history.iter().cloned().collect())
        .map_err(|_| GENERIC_ERROR.to_owned())
}

#[tauri::command]
pub(crate) fn get_speech_status(
    state: State<'_, SpeechRuntimeState>,
) -> Result<crate::speech::SpeechStatusResponse, String> {
    state
        .inner
        .lock()
        .map(|inner| inner.controller.status())
        .map_err(|_| GENERIC_ERROR.to_owned())
}

fn copy_retained_history_transcript<F>(
    state: &SpeechRuntimeState,
    request: CopySpeechHistoryTranscriptRequest,
    publish: F,
) -> Result<(), String>
where
    F: FnOnce(&str) -> Result<(), crate::speech_clipboard::ClipboardFailure>,
{
    let transcript = {
        let history = state.history.lock().map_err(|_| GENERIC_ERROR.to_owned())?;
        let entry = history
            .iter()
            .find(|entry| entry.nonce == request.nonce)
            .ok_or_else(|| GENERIC_ERROR.to_owned())?;
        entry.transcript.clone()
    };
    publish(&transcript).map_err(|_| GENERIC_ERROR.to_owned())
}

#[tauri::command]
pub(crate) fn copy_speech_history_transcript(
    state: State<'_, SpeechRuntimeState>,
    request: CopySpeechHistoryTranscriptRequest,
) -> Result<(), String> {
    copy_retained_history_transcript(&state, request, |transcript| {
        crate::speech_clipboard::write_unicode_text(transcript)
    })
}

impl SpeechRuntimeState {
    fn admit_focus(&self, nonce: SpeechSessionNonce) -> Option<FocusLease<'_>> {
        let mut admission = self.focus_admission.lock().ok()?;
        if admission.closed || !self.can_prepare_focus(nonce) {
            return None;
        }
        admission.in_flight += 1;
        Some(FocusLease(self))
    }

    fn drain_focus_admission_for_shutdown(&self) {
        if let Ok(mut admission) = self.focus_admission.lock() {
            admission.closed = true;
            while admission.in_flight != 0 {
                admission = match self.focus_drained.wait(admission) {
                    Ok(value) => value,
                    Err(_) => return,
                };
            }
        }
    }

    fn can_prepare_focus(&self, nonce: SpeechSessionNonce) -> bool {
        !self.shutting_down.load(Ordering::Acquire)
            && self.generation.load(Ordering::Acquire) == nonce.0
            && self
                .inner
                .lock()
                .ok()
                .is_some_and(|inner| inner.controller.can_complete(nonce, self.now()))
    }

    pub(crate) fn cancel_speech_preparation(&self, reservation_id: u64) {
        if let Ok(mut inner) = self.inner.lock() {
            if inner
                .prepared
                .as_ref()
                .is_some_and(|prepared| prepared.reservation_id == reservation_id)
            {
                inner.prepared = None;
            }
        }
    }

    pub(crate) fn prepare_speech_paste_target(
        &self,
    ) -> Option<crate::speech::SpeechHotkeyActivation> {
        use crate::speech::SpeechHotkeyActivation;
        if self.shutting_down.load(Ordering::Acquire) {
            return None;
        }
        let reservation_id = {
            let Ok(mut inner) = self.inner.lock() else {
                return None;
            };
            if inner.capture.is_some() || inner.controller.status().status != SpeechStatusKind::Idle
            {
                let status = inner.controller.status();
                return if status.status == SpeechStatusKind::Recording {
                    status
                        .nonce
                        .map(|nonce| SpeechHotkeyActivation::Stop { nonce: nonce })
                } else {
                    None
                };
            }
            inner.paste_target = None;
            if inner
                .prepared
                .as_ref()
                .is_some_and(|p| p.prepared_at.elapsed() < Duration::from_secs(2))
            {
                return None; // Duplicate hotkey while activation is pending.
            }
            inner.next_reservation = inner.next_reservation.wrapping_add(1);
            let id = inner.next_reservation;
            inner.prepared = Some(PreparedPasteTarget {
                reservation_id: id,
                target: None,
                prepared_at: Instant::now(),
            });
            id
        };
        // Native capture may wait on another window: never hold the runtime lock here.
        let target = crate::speech_target::capture_speech_paste_target().ok();
        if let Ok(mut inner) = self.inner.lock() {
            if inner.capture.is_some() || inner.controller.status().status != SpeechStatusKind::Idle
            {
                return None;
            }
            if let Some(prepared) = inner.prepared.as_mut() {
                if prepared.reservation_id == reservation_id {
                    prepared.target = target;
                    return Some(SpeechHotkeyActivation::Start { reservation_id });
                }
            }
        }
        None
    }

    fn now(&self) -> Duration {
        self.epoch.elapsed()
    }

    fn invalidate(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
    }

    fn take_model(&self) -> Result<parakeet_rs::ParakeetTDT, String> {
        let mut slot = self
            .model_pool
            .lock()
            .map_err(|_| GENERIC_ERROR.to_owned())?;
        match std::mem::replace(&mut *slot, WarmModelState::InUse) {
            WarmModelState::Ready(model) => Ok(model),
            WarmModelState::Failed => {
                *slot = WarmModelState::Failed;
                Err(SpeechFailure::ModelLoadFailed.code().to_owned())
            }
            state => {
                *slot = state;
                Err(SpeechFailure::ModelLoadFailed.code().to_owned())
            }
        }
    }

    fn return_model(&self, model: parakeet_rs::ParakeetTDT) {
        if let Ok(mut slot) = self.model_pool.lock() {
            *slot = WarmModelState::Ready(model);
        }
    }
}

pub(crate) fn spawn_warm_model_async(app: AppHandle) {
    let state = app.state::<SpeechRuntimeState>();
    let Ok(operation) = state.model_operation.lock() else {
        return;
    };
    if operation.importing {
        return;
    }
    let epoch = operation.epoch;
    let should_load = state.model_pool.lock().ok().is_some_and(|mut slot| {
        if matches!(*slot, WarmModelState::NotStarted) {
            *slot = WarmModelState::Loading;
            true
        } else {
            false
        }
    });
    if !should_load {
        return;
    }
    drop(operation);
    tauri::async_runtime::spawn_blocking(move || {
        let resolved = resolve_model_resource(&app);
        let source = resolved.as_ref().ok().map(|(_, source)| *source);
        let missing = resolved.is_err();
        let loaded = resolved.and_then(|(path, _)| preload_parakeet_tdt(&path));
        let state = app.state::<SpeechRuntimeState>();
        finish_warm_model(&state, epoch, source, missing, loaded);
    });
}

fn finish_warm_model(
    state: &SpeechRuntimeState,
    epoch: u64,
    source: Option<&'static str>,
    missing: bool,
    loaded: Result<parakeet_rs::ParakeetTDT, String>,
) {
    let Ok(mut operation) = state.model_operation.lock() else {
        return;
    };
    if operation.epoch != epoch
        || operation.importing
        || state.shutting_down.load(Ordering::Acquire)
    {
        return;
    }
    operation.source = source;
    operation.missing = missing;
    operation.error = if loaded.is_err() && !missing {
        Some("Model could not load. Import a matching Parakeet TDT int8 bundle.".into())
    } else {
        None
    };
    let old = state.model_pool.lock().ok().map(|mut slot| {
        std::mem::replace(
            &mut *slot,
            match loaded {
                Ok(model) => WarmModelState::Ready(model),
                Err(_) => WarmModelState::Failed,
            },
        )
    });
    drop(operation);
    drop(old);
}

fn model_status(state: &SpeechRuntimeState) -> Result<ModelStatus, String> {
    let operation = state.model_operation.lock().map_err(|_| GENERIC_ERROR)?;
    let slot = state.model_pool.lock().map_err(|_| GENERIC_ERROR)?;
    let status = if operation.importing {
        "loading"
    } else {
        match &*slot {
            WarmModelState::NotStarted => "missing",
            WarmModelState::Loading => "loading",
            WarmModelState::Ready(_) | WarmModelState::InUse => "ready",
            WarmModelState::Failed if operation.missing => "missing",
            WarmModelState::Failed => "error",
        }
    };
    Ok(ModelStatus {
        state: status,
        source: operation.source,
        error: operation.error.clone(),
    })
}

fn require_model_settings(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != crate::shell_windows::SETTINGS_PANEL_LABEL {
        return Err("Model management is available only in Settings.".into());
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn get_speech_model_status(
    window: tauri::WebviewWindow,
    state: State<'_, SpeechRuntimeState>,
) -> Result<ModelStatus, String> {
    require_model_settings(&window)?;
    model_status(&state)
}

struct ImportReservation<'a, W: FnOnce()> {
    state: &'a SpeechRuntimeState,
    retry: Option<W>,
}
impl<W: FnOnce()> Drop for ImportReservation<'_, W> {
    fn drop(&mut self) {
        if let Ok(mut operation) = self.state.model_operation.lock() {
            operation.importing = false;
        }
        if let Some(retry) = self.retry.take() {
            retry();
        }
    }
}

#[derive(Clone, Copy)]
enum ModelImportKind {
    Archive,
    Folder,
}

#[tauri::command]
pub(crate) async fn import_speech_model(
    window: tauri::WebviewWindow,
    app: AppHandle,
) -> Result<ImportModelResponse, String> {
    require_model_settings(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        import_model_blocking(app, window, ModelImportKind::Archive)
    })
    .await
    .map_err(|_| "Model import could not complete. Try again.".to_owned())?
}

#[tauri::command]
pub(crate) async fn import_speech_model_folder(
    window: tauri::WebviewWindow,
    app: AppHandle,
) -> Result<ImportModelResponse, String> {
    require_model_settings(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        import_model_blocking(app, window, ModelImportKind::Folder)
    })
    .await
    .map_err(|_| "Model import could not complete. Try again.".to_owned())?
}

fn import_model_blocking(
    app: AppHandle,
    window: tauri::WebviewWindow,
    kind: ModelImportKind,
) -> Result<ImportModelResponse, String> {
    use tauri_plugin_dialog::DialogExt;
    let state = app.state::<SpeechRuntimeState>();
    import_model_with(
        &state,
        kind,
        || {
            let picker = app.dialog().file().set_parent(&window);
            let selected = match kind {
                ModelImportKind::Archive => picker
                    .add_filter("Parakeet model archive", &["tar", "tar.gz", "tgz"])
                    .blocking_pick_file(),
                ModelImportKind::Folder => picker.blocking_pick_folder(),
            };
            selected
                .map(|selected| {
                    selected
                        .into_path()
                        .map_err(|_| "Choose a local model archive or folder.".to_owned())
                })
                .transpose()
        },
        || {
            app.path()
                .app_local_data_dir()
                .map_err(|_| "App model storage is unavailable.".to_owned())
        },
        |path| {
            crate::speech_model::load_parakeet_tdt(path)
                .map_err(|_| "Model could not load.".to_owned())
        },
        || spawn_warm_model_async(app.clone()),
    )
}

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
    W: FnOnce(),
{
    {
        let _commit = state.commit.lock().map_err(|_| GENERIC_ERROR)?;
        let mut operation = state.model_operation.lock().map_err(|_| GENERIC_ERROR)?;
        let inner = state.inner.lock().map_err(|_| GENERIC_ERROR)?;
        let slot = state.model_pool.lock().map_err(|_| GENERIC_ERROR)?;
        if state.shutting_down.load(Ordering::Acquire)
            || operation.importing
            || matches!(*slot, WarmModelState::InUse)
            || inner.capture.is_some()
            || matches!(
                inner.controller.status().status,
                SpeechStatusKind::Recording | SpeechStatusKind::Transcribing
            )
        {
            return Err("Speech is busy. Stop capture and wait before importing a model.".into());
        }
        operation.importing = true;
        // Invalidates startup work before releasing admission; no late warmup publication.
        operation.epoch = operation.epoch.wrapping_add(1);
    }
    let reservation = ImportReservation {
        state,
        retry: Some(retry_warmup),
    };
    let result = choose().and_then(|selected| {
        if let Some(source) = selected {
            (|| {
                let data = app_data()?;
                let mut loaded = None;
                let validate = |path: &std::path::Path| {
                    loaded = Some(load(path)?);
                    Ok(())
                };
                let limits = crate::speech_model_install::InstallLimits::default();
                match kind {
                    ModelImportKind::Archive => crate::speech_model_install::install_archive(
                        &source, &data, limits, validate,
                    ),
                    ModelImportKind::Folder => crate::speech_model_install::install_directory(
                        &source, &data, limits, validate,
                    ),
                }?;
                let model = loaded.ok_or_else(|| "Model could not load.".to_owned())?;
                let mut operation = state.model_operation.lock().map_err(|_| GENERIC_ERROR)?;
                let old = {
                    let mut slot = state.model_pool.lock().map_err(|_| GENERIC_ERROR)?;
                    std::mem::replace(&mut *slot, WarmModelState::Ready(model))
                };
                operation.source = Some("installed");
                operation.error = None;
                operation.missing = false;
                drop(operation);
                // Model destruction can be slow; never do it under the pool/global lock.
                drop(old);
                Ok(false)
            })()
        } else {
            Ok(true)
        }
    });
    {
        let mut operation = state.model_operation.lock().map_err(|_| GENERIC_ERROR)?;
        if let Err(error) = &result {
            operation.error = Some(error.clone());
        }
        let mut slot = state.model_pool.lock().map_err(|_| GENERIC_ERROR)?;
        // Startup work was invalidated. Cancellation/failure retries instead of
        // leaving its old Loading state stranded, retaining any existing ready model.
        if matches!(*slot, WarmModelState::Loading | WarmModelState::Failed) {
            *slot = WarmModelState::NotStarted;
        }
    }
    drop(reservation);
    let model = model_status(&state)?;
    match result {
        Ok(cancelled) => Ok(ImportModelResponse { cancelled, model }),
        Err(error) => Err(error),
    }
}

#[tauri::command]
pub(crate) fn capture_speech_paste_target(
    state: State<'_, SpeechRuntimeState>,
    capture: Option<bool>,
) -> Result<u64, String> {
    let reservation_id = {
        let mut inner = state.inner.lock().map_err(|_| GENERIC_ERROR.to_owned())?;
        if inner.capture.is_some() || inner.controller.status().status != SpeechStatusKind::Idle {
            return Err("Speech is busy".into());
        }
        inner.next_reservation = inner.next_reservation.wrapping_add(1);
        let id = inner.next_reservation;
        inner.prepared = Some(PreparedPasteTarget {
            reservation_id: id,
            target: None,
            prepared_at: Instant::now(),
        });
        id
    };
    let target = if capture.unwrap_or(true) {
        crate::speech_target::capture_speech_paste_target().ok()
    } else {
        None
    };
    let mut inner = state.inner.lock().map_err(|_| GENERIC_ERROR.to_owned())?;
    if inner.capture.is_some()
        || inner.controller.status().status != SpeechStatusKind::Idle
        || inner
            .prepared
            .as_ref()
            .is_none_or(|p| p.reservation_id != reservation_id)
    {
        return Err("Speech is busy".into());
    }
    inner.prepared.as_mut().unwrap().target = target;
    Ok(reservation_id)
}

#[tauri::command]
pub(crate) fn start_speech_capture(
    app: AppHandle,
    state: State<'_, SpeechRuntimeState>,
    request: StartSpeechCaptureRequest,
) -> Result<StartSpeechCaptureResponse, String> {
    let commit = state.commit.lock().map_err(|_| GENERIC_ERROR)?;
    if state
        .model_operation
        .lock()
        .map_err(|_| GENERIC_ERROR)?
        .importing
    {
        return Err("Model import is in progress. Wait before starting speech.".into());
    }
    if state.shutting_down.load(Ordering::Acquire) {
        state
            .inner
            .lock()
            .ok()
            .map(|mut inner| inner.prepared = None);
        return Err(GENERIC_ERROR.into());
    }
    let event = {
        let mut inner = state.inner.lock().map_err(|_| GENERIC_ERROR)?;
        if inner.capture.is_some() {
            return Err("Speech is busy".into());
        }
        if !inner.matches_prepared_request(request)
            || inner.prepared.as_ref().is_none_or(|prepared| {
                prepared.reservation_id != request.reservation_id
                    || prepared.prepared_at.elapsed() >= Duration::from_secs(2)
            })
        {
            return Err(crate::speech::SPEECH_STATE_ERROR.into());
        }
        let event = match inner.controller.start(state.now()) {
            Ok(event) => event,
            Err(error) => {
                inner.prepared = None;
                return Err(error.to_owned());
            }
        };
        // Consume the prepared snapshot, never recapture the UI's foreground.
        let prepared_target = inner.paste_target.take();
        let target = inner.prepared.take().and_then(|prepared| prepared.target);
        inner.paste_target = Some(SessionPasteTarget {
            nonce: event.nonce.ok_or(GENERIC_ERROR)?,
            target,
        });
        drop(prepared_target);
        event
    };
    let nonce = event.nonce.ok_or_else(|| GENERIC_ERROR.to_string())?;
    // Reserve the controller session before exclusively taking the process-warmed model.
    let model = match state.take_model() {
        Ok(model) => model,
        Err(error) => {
            recover_failed_stop_with(&app, &state, nonce, SpeechFailure::ModelLoadFailed);
            return Err(error);
        }
    };
    let (stream, intake_slot, tail, sample_rate, channels, health, meter) =
        match create_capture_stream() {
            Ok(capture) => capture,
            Err(_) => {
                state.return_model(model);
                recover_failed_stop_with(&app, &state, nonce, SpeechFailure::CaptureStreamError);
                report_capture_start_failure();
                return Err(capture_start_error_code());
            }
        };
    let (intake, receiver) = BoundedIntake::prepared(nonce, nonce.0);
    let intake = Arc::new(intake);
    if intake_slot.set(Arc::clone(&intake)).is_err() {
        state.return_model(model);
        recover_failed_stop_with(&app, &state, nonce, SpeechFailure::StateRace);
        return Err(GENERIC_ERROR.into());
    }
    if stream.play().is_err() {
        state.return_model(model);
        recover_failed_stop_with(&app, &state, nonce, SpeechFailure::CaptureStreamError);
        report_capture_start_failure();
        return Err(capture_start_error_code());
    }
    let activated_at = Instant::now();
    if intake.activate(activated_at).is_err() {
        state.return_model(model);
        recover_failed_stop_with(&app, &state, nonce, SpeechFailure::StateRace);
        return Err(GENERIC_ERROR.into());
    }
    if state.shutting_down.load(Ordering::Acquire) {
        intake.first_close(InputCloseReason::Shutdown);
        state.return_model(model);
        recover_failed_stop_with(&app, &state, nonce, SpeechFailure::StateRace);
        return Err(GENERIC_ERROR.into());
    }
    let mut inner = match state.inner.lock() {
        Ok(inner) => inner,
        Err(_) => {
            state.return_model(model);
            recover_failed_stop_with(&app, &state, nonce, SpeechFailure::StateRace);
            return Err(GENERIC_ERROR.into());
        }
    };
    inner.capture = Some(Capture {
        stream,
        intake: Arc::clone(&intake),
        tail,
        sample_rate,
        channels,
        health,
        meter,
    });
    drop(inner);
    state.generation.store(nonce.0, Ordering::Release);
    let worker_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_streaming_worker(worker_app, nonce, nonce.0, intake, receiver, model)
    });
    if let Err(error) = emit(&app, &event) {
        let rollback_capture = state
            .inner
            .lock()
            .ok()
            .and_then(|mut inner| inner.capture.take());
        if let Some(capture) = rollback_capture {
            capture.intake.first_close(InputCloseReason::SafeFailure);
            drop(capture);
        }
        recover_failed_stop_with(&app, &state, nonce, SpeechFailure::StateRace);
        return Err(error);
    }
    drop(commit);
    apply_indicator_visibility(app.clone(), event.clone());
    schedule_capture_health(app.clone(), nonce);
    schedule_voice_level_updates(app.clone(), nonce);
    schedule_recording_limit(app, nonce, activated_at);
    Ok(StartSpeechCaptureResponse {
        nonce,
        status: SpeechStatusKind::Recording,
    })
}

#[tauri::command]
pub(crate) fn stop_speech_capture(
    app: AppHandle,
    state: State<'_, SpeechRuntimeState>,
    request: StopSpeechCaptureRequest,
) -> Result<crate::speech::SpeechStatusResponse, String> {
    let _commit = state.commit.lock().map_err(|_| GENERIC_ERROR)?;
    let (event, capture) = {
        let mut inner = state.inner.lock().map_err(|_| GENERIC_ERROR)?;
        let event = inner
            .controller
            .stop(request.nonce, state.now())
            .map_err(|_| GENERIC_ERROR.to_string())?;
        let Some(capture) = inner.capture.take() else {
            let _ = recover_post_stop_failure(&mut inner.controller, request.nonce, state.now());
            state.invalidate();
            return Err(GENERIC_ERROR.into());
        };
        (event, capture)
    };
    if let Err(error) = emit(&app, &event) {
        capture.intake.first_close(InputCloseReason::SafeFailure);
        drop(capture);
        debug_diagnostic(
            request.nonce,
            SpeechFailure::StateRace,
            Duration::ZERO,
            0,
            0,
            0,
            true,
            0.0,
        );
        recover_failed_stop(&app, &state, request.nonce);
        return Err(error);
    }
    let Capture {
        stream,
        intake,
        sample_rate,
        channels,
        health,
        tail,
        meter: _,
    } = capture;
    drop(stream);
    handoff_tail(&intake, &tail, &health);
    intake.first_close(InputCloseReason::Finish);
    if let Some(failure) = capture_failure(&health) {
        debug_diagnostic(
            request.nonce,
            failure,
            Duration::ZERO,
            sample_rate,
            channels,
            0,
            true,
            0.0,
        );
        recover_failed_stop_with(&app, &state, request.nonce, failure);
        return Err(GENERIC_ERROR.into());
    }
    drop(intake);
    schedule_transcription_limit(app, request.nonce);
    Ok(crate::speech::SpeechStatusResponse {
        status: SpeechStatusKind::Transcribing,
        nonce: Some(request.nonce),
    })
}

fn run_streaming_worker(
    app: AppHandle,
    nonce: SpeechSessionNonce,
    generation: u64,
    intake: Arc<BoundedIntake>,
    receiver: Receiver<WorkerMessage>,
    model: parakeet_rs::ParakeetTDT,
) {
    let mut model = WorkerModel::new(app.clone(), model);
    let started = Instant::now();
    let mut sample_count = 0;
    let result: Result<_, SpeechFailure> = (|| {
        let mut pending = VecDeque::<Box<[f32]>>::with_capacity(WINDOW_SEGMENTS);
        let mut transcript = String::new();
        let mut new_segments = 0usize;
        while let Some(message) = intake.recv_until_closed(&receiver) {
            match message {
                WorkerMessage::Segment(segment) => {
                    sample_count += segment.len();
                    pending.push_back(segment);
                    new_segments += 1;
                    if pending.len() == WINDOW_SEGMENTS {
                        transcribe_window(&mut model, &pending, &mut transcript, false)?;
                        for _ in 0..WINDOW_SEGMENTS - OVERLAP_SEGMENTS {
                            pending.pop_front();
                        }
                        new_segments = 0;
                    }
                }
            }
        }
        while let Ok(WorkerMessage::Segment(segment)) = receiver.try_recv() {
            sample_count += segment.len();
            pending.push_back(segment);
            new_segments += 1;
        }
        if sample_count < MIN_CAPTURE_SAMPLES {
            return Err(if sample_count == 0 {
                SpeechFailure::CaptureEmpty
            } else {
                SpeechFailure::CaptureShort
            });
        }
        if new_segments > 0 {
            transcribe_window(&mut model, &pending, &mut transcript, true)?;
        } else if !pending.is_empty() {
            transcribe_window(&mut model, &pending, &mut transcript, true)?;
        }
        wait_for_finalizing(&app, nonce)?;
        let state = app.state::<SpeechRuntimeState>();
        let current_generation = state.generation.load(Ordering::Acquire);
        if generation != current_generation || !intake.can_commit(nonce, current_generation) {
            return Err(SpeechFailure::StateRace);
        }
        let text = transcript.trim();
        if text.is_empty() {
            return Err(SpeechFailure::TranscriptEmpty);
        }
        let _commit = state.commit.lock().map_err(|_| SpeechFailure::StateRace)?;
        if state.shutting_down.load(Ordering::Acquire)
            || state.generation.load(Ordering::Acquire) != nonce.0
        {
            return Err(SpeechFailure::StateRace);
        }
        let paste_target = state
            .inner
            .lock()
            .map_err(|_| SpeechFailure::StateRace)
            .and_then(|mut inner| {
                if !inner.controller.can_complete(nonce, state.now()) {
                    return Err(SpeechFailure::StateRace);
                }
                Ok(inner
                    .take_session_target(nonce)
                    .and_then(|slot| slot.target))
            })?;
        drop(_commit);
        publish_recorded_if_current(
            &state,
            nonce,
            text,
            || state.now(),
            || {
                crate::speech_clipboard::write_unicode_text(text)?;
                if let Some(paste_target) = paste_target {
                    if state.shutting_down.load(Ordering::Acquire)
                        || state.generation.load(Ordering::Acquire) != nonce.0
                        || !state.can_prepare_focus(nonce)
                    {
                        return Err(
                            crate::speech_clipboard::ClipboardFailure::PasteTargetUnavailable,
                        );
                    }
                    let focus_lease = state
                        .admit_focus(nonce)
                        .ok_or(crate::speech_clipboard::ClipboardFailure::PasteTargetUnavailable)?;
                    crate::speech_target::prepare_captured_target(paste_target)
                        .map_err(map_paste_failure)?;
                    drop(focus_lease);
                    let _delivery = state.delivery.lock().map_err(|_| {
                        crate::speech_clipboard::ClipboardFailure::PasteTargetUnavailable
                    })?;
                    if state.shutting_down.load(Ordering::Acquire)
                        || state.generation.load(Ordering::Acquire) != nonce.0
                    {
                        return Err(
                            crate::speech_clipboard::ClipboardFailure::PasteTargetUnavailable,
                        );
                    }
                    let authorized = state
                        .inner
                        .lock()
                        .ok()
                        .is_some_and(|inner| inner.controller.can_complete(nonce, state.now()));
                    if !authorized {
                        return Err(
                            crate::speech_clipboard::ClipboardFailure::PasteTargetUnavailable,
                        );
                    }
                    use crate::speech_target::inject_to_prepared_target as paste_to_captured_target;
                    paste_to_captured_target(paste_target).map_err(map_paste_failure)?;
                }
                Ok(())
            },
        )
    })();
    drop(model);
    let state = app.state::<SpeechRuntimeState>();
    let event = match result {
        Ok(event) => Some(event),
        Err(failure) => {
            if !matches!(failure, SpeechFailure::Clipboard(_)) {
                debug_diagnostic(
                    nonce,
                    failure,
                    started.elapsed(),
                    TARGET_RATE,
                    1,
                    sample_count,
                    true,
                    0.0,
                );
            }
            let _commit = match state.commit.lock() {
                Ok(value) => value,
                Err(_) => return,
            };
            if state.shutting_down.load(Ordering::Acquire)
                || state.generation.load(Ordering::Acquire) != nonce.0
            {
                return;
            }
            state.inner.lock().ok().and_then(|mut inner| {
                inner
                    .controller
                    .complete_error_code(nonce, state.now(), failure.code())
            })
        }
    };
    if let Some(event) = event {
        let _commit = match state.commit.lock() {
            Ok(value) => value,
            Err(_) => return,
        };
        if state.shutting_down.load(Ordering::Acquire)
            || state.generation.load(Ordering::Acquire) != nonce.0
        {
            return;
        }
        let _ = emit(&app, &event);
        schedule_terminal_reset(app.clone(), nonce);
    }
}

fn transcribe_window(
    model: &mut parakeet_rs::ParakeetTDT,
    segments: &VecDeque<Box<[f32]>>,
    transcript: &mut String,
    final_window: bool,
) -> Result<(), SpeechFailure> {
    let audio: Vec<f32> = segments
        .iter()
        .flat_map(|segment| segment.iter().copied())
        .collect();
    validate_audio(&audio)?;
    let result = model
        .transcribe_samples(audio, TARGET_RATE, 1, None)
        .map_err(|_| SpeechFailure::AsrFailed)?;
    merge_tdt_window(transcript, &result.text, final_window);
    Ok(())
}

struct WorkerModel {
    app: AppHandle,
    model: Option<parakeet_rs::ParakeetTDT>,
}

impl WorkerModel {
    fn new(app: AppHandle, model: parakeet_rs::ParakeetTDT) -> Self {
        Self {
            app,
            model: Some(model),
        }
    }
}

impl std::ops::Deref for WorkerModel {
    type Target = parakeet_rs::ParakeetTDT;

    fn deref(&self) -> &Self::Target {
        self.model.as_ref().expect("worker model remains owned")
    }
}

impl std::ops::DerefMut for WorkerModel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.model.as_mut().expect("worker model remains owned")
    }
}

impl Drop for WorkerModel {
    fn drop(&mut self) {
        if let Some(model) = self.model.take() {
            self.app.state::<SpeechRuntimeState>().return_model(model);
        }
    }
}

fn wait_for_finalizing(app: &AppHandle, nonce: SpeechSessionNonce) -> Result<(), SpeechFailure> {
    let deadline = Instant::now() + FINALIZATION_TIMEOUT;
    while Instant::now() < deadline {
        let state = app.state::<SpeechRuntimeState>();
        let status = state
            .inner
            .lock()
            .map_err(|_| SpeechFailure::StateRace)?
            .controller
            .status();
        if status.nonce != Some(nonce) {
            return Err(SpeechFailure::StateRace);
        }
        if status.status == SpeechStatusKind::Transcribing {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err(SpeechFailure::Timeout)
}

#[cfg(test)]
fn complete_authorized_publish(
    controller: &mut SpeechController,
    nonce: SpeechSessionNonce,
    authorized_at: Duration,
) -> Option<SpeechStatusEvent> {
    controller.complete_copied(nonce, authorized_at)
}

#[cfg(test)]
fn publish_if_current<N, P>(
    controller: &mut SpeechController,
    nonce: SpeechSessionNonce,
    now: N,
    publish: P,
) -> Option<SpeechStatusEvent>
where
    N: FnMut() -> Duration,
    P: FnOnce() -> Result<(), ()>,
{
    let mut now = now;
    let publish_at = now();
    if !controller.can_complete(nonce, publish_at) {
        return controller.complete_error(nonce, publish_at);
    }
    publish().ok()?;
    complete_authorized_publish(controller, nonce, publish_at)
}

fn publish_recorded_if_current<N, P>(
    state: &SpeechRuntimeState,
    nonce: SpeechSessionNonce,
    transcript: &str,
    mut now: N,
    publish: P,
) -> Result<SpeechStatusEvent, SpeechFailure>
where
    N: FnMut() -> Duration,
    P: FnOnce() -> Result<(), crate::speech_clipboard::ClipboardFailure>,
{
    // Shutdown takes this gate before it invalidates the session. Once delivery
    // starts, shutdown waits; when shutdown wins first, this preflight observes
    // the invalidated generation and sends no native input.
    // Native clipboard/focus work is outside the short final injection gate.
    // `state.delivery` protects only bounded_native_delivery (the final identity
    // check and SendInput), never clipboard publication or focus restoration.
    let publish_at = now();
    {
        // The commit gate only guards authorization. Clipboard and SendInput can block,
        // so they must run after this short critical section is released.
        let _commit = state.commit.lock().map_err(|_| SpeechFailure::StateRace)?;
        if state.shutting_down.load(Ordering::Acquire)
            || state.generation.load(Ordering::Acquire) != nonce.0
        {
            return Err(SpeechFailure::StateRace);
        }
        if !state
            .inner
            .lock()
            .map_err(|_| SpeechFailure::StateRace)?
            .controller
            .can_complete(nonce, publish_at)
        {
            return state
                .inner
                .lock()
                .map_err(|_| SpeechFailure::StateRace)?
                .controller
                .complete_error_code(nonce, publish_at, SpeechFailure::Timeout.code())
                .ok_or(SpeechFailure::StateRace);
        }
        insert_history_attempt(state, nonce, transcript).map_err(|_| SpeechFailure::StateRace)?;
    }

    if let Err(failure) = publish() {
        update_history_outcome(state, nonce, failure.code())
            .map_err(|_| SpeechFailure::StateRace)?;
        return Err(SpeechFailure::Clipboard(failure));
    }
    let event = {
        let _commit = state.commit.lock().map_err(|_| SpeechFailure::StateRace)?;
        if state.shutting_down.load(Ordering::Acquire)
            || state.generation.load(Ordering::Acquire) != nonce.0
        {
            return Err(SpeechFailure::StateRace);
        }
        state
            .inner
            .lock()
            .map_err(|_| SpeechFailure::StateRace)?
            .controller
            .complete_copied(nonce, publish_at)
            .ok_or(SpeechFailure::StateRace)?
    };
    update_history_outcome(state, nonce, "copied").map_err(|_| SpeechFailure::StateRace)?;
    Ok(event)
}

fn recover_post_stop_failure(
    controller: &mut SpeechController,
    nonce: SpeechSessionNonce,
    now: Duration,
) -> Option<SpeechStatusEvent> {
    controller.complete_error(nonce, now)
}

fn recover_failed_stop(app: &AppHandle, state: &SpeechRuntimeState, nonce: SpeechSessionNonce) {
    recover_failed_stop_with(app, state, nonce, SpeechFailure::StateRace);
}

fn recover_failed_stop_with(
    app: &AppHandle,
    state: &SpeechRuntimeState,
    nonce: SpeechSessionNonce,
    failure: SpeechFailure,
) {
    state.invalidate();
    let event = state.inner.lock().ok().and_then(|mut inner| {
        inner.paste_target = None;
        inner.prepared = None;
        if inner.controller.status().status == SpeechStatusKind::Recording {
            let _ = inner.controller.stop(nonce, state.now());
        }
        inner
            .controller
            .complete_error_code(nonce, state.now(), failure.code())
    });
    if let Some(event) = event {
        let _ = emit(app, &event);
        schedule_terminal_reset(app.clone(), nonce);
    }
}

fn expire_transcription_and_invalidate(
    state: &SpeechRuntimeState,
    nonce: SpeechSessionNonce,
    now: Duration,
) -> Option<SpeechStatusEvent> {
    let mut inner = state.inner.lock().ok()?;
    let event = inner.controller.expire_transcription(nonce, now)?;
    inner.paste_target = None;
    inner.prepared = None;
    state.invalidate();
    Some(event)
}

#[cfg(test)]
fn expire_recording_and_invalidate(
    state: &SpeechRuntimeState,
    nonce: SpeechSessionNonce,
    now: Duration,
) -> Option<(SpeechStatusEvent, Option<Capture>)> {
    let mut inner = state.inner.lock().ok()?;
    let event = inner.controller.expire_recording(nonce, now)?;
    inner.paste_target = None;
    inner.prepared = None;
    let capture = inner.capture.take();
    state.invalidate();
    Some((event, capture))
}

fn reset_terminal_if_current(
    state: &SpeechRuntimeState,
    nonce: SpeechSessionNonce,
    generation: u64,
) -> Option<SpeechStatusEvent> {
    let _commit = state.commit.lock().ok()?;
    if state.shutting_down.load(Ordering::Acquire)
        || state.generation.load(Ordering::Acquire) != generation
    {
        return None;
    }
    let mut inner = state.inner.lock().ok()?;
    let event = inner.controller.reset(nonce)?;
    inner.paste_target = None;
    inner.prepared = None;
    Some(event)
}

fn schedule_terminal_reset(app: AppHandle, nonce: SpeechSessionNonce) {
    let generation = app
        .state::<SpeechRuntimeState>()
        .generation
        .load(Ordering::Acquire);
    std::thread::spawn(move || {
        std::thread::sleep(TERMINAL_RESET_DELAY);
        let state = app.state::<SpeechRuntimeState>();
        if let Some(event) = reset_terminal_if_current(&state, nonce, generation) {
            let _commit = match state.commit.lock() {
                Ok(value) => value,
                Err(_) => return,
            };
            if state.shutting_down.load(Ordering::Acquire)
                || state.generation.load(Ordering::Acquire) != generation
            {
                return;
            }
            let _ = emit(&app, &event);
        }
    });
}

fn schedule_recording_limit(app: AppHandle, nonce: SpeechSessionNonce, activated_at: Instant) {
    std::thread::spawn(move || {
        std::thread::sleep(
            (activated_at + MAX_RECORDING_DURATION).saturating_duration_since(Instant::now()),
        );
        let state = app.state::<SpeechRuntimeState>();
        let _commit = match state.commit.lock() {
            Ok(value) => value,
            Err(_) => return,
        };
        if state.shutting_down.load(Ordering::Acquire)
            || state.generation.load(Ordering::Acquire) != nonce.0
        {
            return;
        }
        let Some((event, capture)) = state.inner.lock().ok().and_then(|mut inner| {
            let event = inner
                .controller
                .stop_for_recording_cap(nonce, state.now())
                .ok()?;
            Some((event, inner.capture.take()?))
        }) else {
            return;
        };
        drop(capture.stream);
        handoff_tail(&capture.intake, &capture.tail, &capture.health);
        capture.intake.first_close(InputCloseReason::RecordingCap);
        drop(capture.intake);
        let _ = emit(&app, &event);
        schedule_transcription_limit(app.clone(), nonce);
    });
}

fn schedule_capture_health(app: AppHandle, nonce: SpeechSessionNonce) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(10));
        let state = app.state::<SpeechRuntimeState>();
        let _commit = match state.commit.lock() {
            Ok(value) => value,
            Err(_) => return,
        };
        if state.shutting_down.load(Ordering::Acquire)
            || state.generation.load(Ordering::Acquire) != nonce.0
        {
            return;
        }
        let capture = state.inner.lock().ok().and_then(|mut inner| {
            let failure = match inner
                .capture
                .as_ref()
                .and_then(|capture| capture_failure(&capture.health))
            {
                Some(failure) => failure,
                None if inner.controller.status().status == SpeechStatusKind::Recording => {
                    return Some(None);
                }
                None => return None,
            };
            let _ = inner.controller.stop(nonce, state.now());
            let event = inner
                .controller
                .complete_error_code(nonce, state.now(), failure.code())?;
            Some(Some((event, inner.capture.take())))
        });
        let Some(capture) = capture else {
            return;
        };
        let Some((event, capture)) = capture else {
            continue;
        };
        state.invalidate();
        if let Some(capture) = capture {
            capture.intake.first_close(InputCloseReason::SafeFailure);
            drop(capture);
        }
        let _ = emit(&app, &event);
        schedule_terminal_reset(app.clone(), nonce);
        return;
    });
}

/// Publishes the most recent scalar capture meter away from CPAL's real-time callback.
/// This is intentionally rate-limited and guarded by the active session generation.
fn schedule_voice_level_updates(app: AppHandle, nonce: SpeechSessionNonce) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(50));
        let state = app.state::<SpeechRuntimeState>();
        if state.shutting_down.load(Ordering::Acquire)
            || state.generation.load(Ordering::Acquire) != nonce.0
        {
            return;
        }
        let level = state.inner.lock().ok().and_then(|inner| {
            (inner.controller.status().status == SpeechStatusKind::Recording)
                .then(|| {
                    inner
                        .capture
                        .as_ref()
                        .map(|capture| capture.meter.load(Ordering::Acquire))
                })
                .flatten()
        });
        let Some(level) = level else {
            return;
        };
        if state.generation.load(Ordering::Acquire) != nonce.0 {
            return;
        }
        let event = SpeechVoiceLevelEvent {
            nonce,
            level: f32::from_bits(level).clamp(0.0, 1.0),
        };
        let _ = app.emit_to(
            crate::shell_windows::SPEECH_INDICATOR_LABEL,
            contracts::events::SPEECH_VOICE_LEVEL,
            event,
        );
    });
}

fn schedule_transcription_limit(app: AppHandle, nonce: SpeechSessionNonce) {
    let delay = {
        let state = app.state::<SpeechRuntimeState>();
        state
            .inner
            .lock()
            .ok()
            .and_then(|inner| {
                inner
                    .controller
                    .remaining_operation_time(nonce, state.now())
            })
            .unwrap_or(Duration::ZERO)
    };
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        let (event, generation) = {
            let state = app.state::<SpeechRuntimeState>();
            let _commit = match state.commit.lock() {
                Ok(value) => value,
                Err(_) => return,
            };
            let event = expire_transcription_and_invalidate(&state, nonce, state.now());
            (event, state.generation.load(Ordering::Acquire))
        };
        if let Some(event) = event {
            let state = app.state::<SpeechRuntimeState>();
            let _commit = match state.commit.lock() {
                Ok(value) => value,
                Err(_) => return,
            };
            if state.shutting_down.load(Ordering::Acquire)
                || state.generation.load(Ordering::Acquire) != generation
            {
                return;
            }
            debug_diagnostic(
                nonce,
                SpeechFailure::Timeout,
                Duration::ZERO,
                TARGET_RATE,
                1,
                0,
                true,
                0.0,
            );
            let _ = emit(&app, &event);
            schedule_terminal_reset(app.clone(), nonce);
        }
    });
}

pub(crate) fn shutdown(state: &SpeechRuntimeState) {
    let _delivery = state.delivery.lock().ok();
    let _commit = state.commit.lock().ok();
    state.drain_focus_admission_for_shutdown();
    shutdown_after_commit(state);
}

#[cfg(test)]
fn shutdown_with_hook<F>(state: &SpeechRuntimeState, after_commit_locked: F)
where
    F: FnOnce(),
{
    let _delivery = state.delivery.lock().ok();
    let _commit = state.commit.lock().ok();
    after_commit_locked();
    state.drain_focus_admission_for_shutdown();
    shutdown_after_commit(state);
}

fn shutdown_after_commit(state: &SpeechRuntimeState) {
    state.generation.fetch_add(1, Ordering::AcqRel);
    state.shutting_down.store(true, Ordering::Release);
    let capture = state.inner.lock().ok().and_then(|mut inner| {
        inner.paste_target = None;
        inner.prepared = None;
        inner.capture.take()
    });
    if let Some(capture) = capture {
        capture.intake.first_close(InputCloseReason::Shutdown);
        drop(capture);
    }
    if let Ok(mut history) = state.history.lock() {
        history.clear();
    }
}

fn emit(app: &AppHandle, event: &SpeechStatusEvent) -> Result<(), String> {
    // Native window I/O is deferred; this function is often called with the commit gate held.
    let _speech_surface = app.get_webview_window(crate::shell_windows::SPEECH_INDICATOR_LABEL);
    // SpeechStatusKind::Recording is deferred to speech_surface.show(); _ is deferred to speech_surface.hide().
    apply_indicator_visibility(app.clone(), event.clone());
    app.emit(contracts::events::SPEECH_STATUS_CHANGED, event)
        .map_err(|_| GENERIC_ERROR.to_string())
}

/// Applies a current speech-status visibility decision outside the lifecycle commit gate.
/// No captured speech data is read, emitted, or logged here.
fn apply_indicator_visibility(app: AppHandle, event: SpeechStatusEvent) {
    std::thread::spawn(move || {
        let state = app.state::<SpeechRuntimeState>();
        // Every status plan passes through this gate. It keeps native actions ordered
        // while lifecycle transitions remain free to release `commit` before I/O.
        let Ok(_visibility_order) = state.visibility_order.lock() else {
            eprintln!("speech indicator visibility ordering unavailable");
            return;
        };
        let _generation = state.generation.load(Ordering::Acquire);
        let is_current = state.inner.lock().ok().is_some_and(|inner| {
            let current = inner.controller.status();
            current.status == event.status
                && (event.status == SpeechStatusKind::Idle || current.nonce == event.nonce)
        });
        if !is_current {
            return;
        }
        let Some(indicator) = app.get_webview_window(crate::shell_windows::SPEECH_INDICATOR_LABEL)
        else {
            return;
        };
        let result = if event.status == SpeechStatusKind::Recording {
            indicator.show()
        } else {
            indicator.hide()
        };
        if let Err(_) = result {
            eprintln!("speech indicator visibility update failed");
            return;
        }
        if event.status == SpeechStatusKind::Recording {
            let _generation = state.generation.load(Ordering::Acquire);
            let recording_still_current = state.inner.lock().ok().is_some_and(|inner| {
                let current = inner.controller.status();
                current.status == SpeechStatusKind::Recording && current.nonce == event.nonce
            });
            if !recording_still_current {
                if let Err(_) = indicator.hide() {
                    eprintln!("speech indicator stale visibility correction failed");
                }
                // A newer recording may have committed between the stale plan's
                // validation and its compensating hide. Reconcile to that state.
                let current_recording = state.inner.lock().ok().and_then(|inner| {
                    let current = inner.controller.status();
                    (current.status == SpeechStatusKind::Recording).then_some(current)
                });
                if current_recording.is_some() {
                    if let Err(_) = indicator.show() {
                        eprintln!("speech indicator recording reconciliation failed");
                    } else {
                        // The terminal plan may have committed while this stale plan
                        // reconciled. Converge before releasing visibility ordering.
                        let terminal_after_reconciliation =
                            state.inner.lock().ok().is_some_and(|inner| {
                                inner.controller.status().status != SpeechStatusKind::Recording
                            });
                        if terminal_after_reconciliation {
                            if let Err(_) = indicator.hide() {
                                eprintln!("speech indicator terminal reconciliation failed");
                            }
                        }
                    }
                }
            }
        }
    });
}

fn resolve_model_resource(app: &AppHandle) -> Result<(PathBuf, &'static str), String> {
    if let Ok(data) = app.path().app_local_data_dir() {
        if let Some(path) = crate::speech_model_install::resolve_installed_model(&data) {
            return Ok((path, "installed"));
        }
    }
    let bundled = app
        .path()
        .resolve(MODEL_RESOURCE, tauri::path::BaseDirectory::Resource)
        .map_err(|_| GENERIC_ERROR.to_string())?;
    crate::speech_model::validate_parakeet_tdt_layout(&bundled)
        .map_err(|_| GENERIC_ERROR.to_string())?;
    Ok((bundled, "bundled"))
}

fn preload_parakeet_tdt(model_path: &PathBuf) -> Result<parakeet_rs::ParakeetTDT, String> {
    crate::speech_model::load_parakeet_tdt(model_path)
        .map_err(|_| SpeechFailure::ModelLoadFailed.code().to_owned())
}

type CaptureStreamParts = (
    Stream,
    Arc<OnceLock<Arc<BoundedIntake>>>,
    Arc<Mutex<Vec<f32>>>,
    u32,
    u16,
    Arc<CaptureHealth>,
    Arc<AtomicU32>,
);

fn create_capture_stream() -> Result<CaptureStreamParts, ()> {
    let device = cpal::default_host().default_input_device().ok_or(())?;
    let supported = device.default_input_config().map_err(|_| ())?;
    let config: StreamConfig = supported.config();
    let sample_rate = config.sample_rate;
    let channels = config.channels;
    let intake = Arc::new(OnceLock::new());
    let tail = Arc::new(Mutex::new(Vec::with_capacity(SEGMENT_SAMPLES)));
    let health = Arc::new(CaptureHealth::default());
    let meter = Arc::new(AtomicU32::new(0.0_f32.to_bits()));
    let stream = match supported.sample_format() {
        SampleFormat::F32 => {
            build_stream::<f32>(&device, &config, &intake, &tail, &health, &meter, |v| v)
        }
        SampleFormat::I16 => {
            build_stream::<i16>(&device, &config, &intake, &tail, &health, &meter, |v| {
                v as f32 / i16::MAX as f32
            })
        }
        SampleFormat::U16 => {
            build_stream::<u16>(&device, &config, &intake, &tail, &health, &meter, |v| {
                (v as f32 / u16::MAX as f32) * 2.0 - 1.0
            })
        }
        _ => return Err(()),
    }?;
    Ok((stream, intake, tail, sample_rate, channels, health, meter))
}

fn capture_start_error_code() -> String {
    SpeechFailure::CaptureStreamError.code().to_owned()
}

fn report_capture_start_failure() {
    debug_diagnostic(
        SpeechSessionNonce(0),
        SpeechFailure::CaptureStreamError,
        Duration::ZERO,
        0,
        0,
        0,
        true,
        0.0,
    );
}

fn build_stream<T: cpal::SizedSample + 'static>(
    device: &cpal::Device,
    config: &StreamConfig,
    intake: &Arc<OnceLock<Arc<BoundedIntake>>>,
    tail: &Arc<Mutex<Vec<f32>>>,
    health: &Arc<CaptureHealth>,
    meter: &Arc<AtomicU32>,
    convert: fn(T) -> f32,
) -> Result<Stream, ()> {
    let target = Arc::clone(intake);
    let callback_health = Arc::clone(health);
    let error_health = Arc::clone(health);
    let channels = config.channels as usize;
    let source_rate = config.sample_rate as u64;
    let mut phase = 0_u64;
    let callback_tail = Arc::clone(tail);
    let callback_meter = Arc::clone(meter);
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let Some(intake) = target.get() else {
                    callback_health
                        .callback_drops
                        .fetch_add(1, Ordering::Relaxed);
                    return;
                };
                let Ok(mut segment) = callback_tail.try_lock() else {
                    callback_health
                        .callback_drops
                        .fetch_add(1, Ordering::Release);
                    return;
                };
                let mut peak = 0.0_f32;
                for frame in data.chunks_exact(channels) {
                    let mono = frame.iter().copied().map(convert).sum::<f32>() / channels as f32;
                    peak = peak.max(mono.abs());
                    phase += TARGET_RATE as u64;
                    while phase >= source_rate {
                        phase -= source_rate;
                        segment.push(mono);
                        if segment.len() == SEGMENT_SAMPLES {
                            let ready = std::mem::replace(
                                &mut *segment,
                                Vec::with_capacity(SEGMENT_SAMPLES),
                            );
                            match intake.try_send_now(ready.into_boxed_slice()) {
                                IntakeResult::Accepted => {}
                                IntakeResult::Rejected(InputCloseReason::RecordingCap) => return,
                                IntakeResult::Rejected(_) | IntakeResult::QueueFull => {
                                    callback_health
                                        .callback_drops
                                        .fetch_add(1, Ordering::Release);
                                    return;
                                }
                            }
                        }
                    }
                }
                let level = if peak.is_finite() {
                    peak.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                callback_meter.store(level.to_bits(), Ordering::Release);
            },
            move |_| error_health.stream_error.store(true, Ordering::Release),
            None,
        )
        .map_err(|_| ())
}

fn handoff_tail(intake: &BoundedIntake, tail: &Mutex<Vec<f32>>, health: &CaptureHealth) {
    let Ok(mut tail) = tail.lock() else {
        health.callback_drops.fetch_add(1, Ordering::Release);
        return;
    };
    if tail.is_empty() {
        return;
    }
    let mut samples = std::mem::replace(&mut *tail, Vec::with_capacity(SEGMENT_SAMPLES));
    if let Some(deadline) = intake.activation_deadline() {
        let now = Instant::now();
        if now >= deadline {
            let tail_duration = Duration::from_secs_f64(samples.len() as f64 / TARGET_RATE as f64);
            let started_at = now.checked_sub(tail_duration).unwrap_or(now);
            let samples_before_deadline = deadline
                .saturating_duration_since(started_at)
                .as_secs_f64()
                .mul_add(TARGET_RATE as f64, 0.0)
                as usize;
            samples.truncate(samples_before_deadline.min(samples.len()));
        }
    }
    if samples.is_empty() {
        return;
    }
    if !matches!(
        intake.try_send_tail(samples.into_boxed_slice()),
        IntakeResult::Accepted
    ) {
        health.callback_drops.fetch_add(1, Ordering::Release);
    }
}

fn capture_failure(health: &CaptureHealth) -> Option<SpeechFailure> {
    if health.stream_error.load(Ordering::Acquire) {
        Some(SpeechFailure::CaptureStreamError)
    } else if health.callback_drops.load(Ordering::Acquire) > 0 {
        Some(SpeechFailure::CaptureCallbackLoss)
    } else {
        None
    }
}

fn validate_audio(audio: &[f32]) -> Result<(), SpeechFailure> {
    if audio.is_empty() {
        Err(SpeechFailure::CaptureEmpty)
    } else if audio.len() < MIN_CAPTURE_SAMPLES {
        Err(SpeechFailure::CaptureShort)
    } else if audio.iter().any(|sample| !sample.is_finite()) {
        Err(SpeechFailure::AsrInputInvalid)
    } else {
        Ok(())
    }
}

#[cfg(debug_assertions)]
fn debug_diagnostic(
    nonce: SpeechSessionNonce,
    failure: SpeechFailure,
    elapsed: Duration,
    source_rate: u32,
    channels: u16,
    samples: usize,
    finite: bool,
    peak: f32,
) {
    let bucket = if peak < 0.01 {
        "quiet"
    } else if peak < 0.9 {
        "normal"
    } else {
        "high"
    };
    eprintln!(
        "speech_diag code={} stage={} nonce={} duration_ms={} source_rate={} channels={} samples={} finite={} peak={}",
        failure.code(), failure.code().split('-').next().unwrap_or("state"), nonce.0,
        elapsed.as_millis(), source_rate, channels, samples, finite, bucket
    );
}

#[cfg(not(debug_assertions))]
fn debug_diagnostic(
    _: SpeechSessionNonce,
    _: SpeechFailure,
    _: Duration,
    _: u32,
    _: u16,
    _: usize,
    _: bool,
    _: f32,
) {
}

#[cfg(test)]
fn resample_to_mono_16k(input: &[f32], source_rate: u32, channels: u16) -> Vec<f32> {
    if input.is_empty() || source_rate == 0 || channels == 0 {
        return Vec::new();
    }
    let channels = channels as usize;
    let mono: Vec<f32> = input
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();
    if source_rate == TARGET_RATE {
        return mono;
    }
    let output_len = mono.len().saturating_mul(TARGET_RATE as usize) / source_rate as usize;
    (0..output_len)
        .map(|index| {
            let position = index as f64 * source_rate as f64 / TARGET_RATE as f64;
            let left = position.floor() as usize;
            let right = (left + 1).min(mono.len() - 1);
            mono[left] + (mono[right] - mono[left]) * (position - left as f64) as f32
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::speech::MAX_SPEECH_OPERATION_DURATION;

    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/speech_model_folder_runtime.rs"));

    #[test]
    fn focus_lease_shutdown_waits_for_admitted_focus_and_rejects_late_admission() {
        let state = Arc::new(SpeechRuntimeState::default());
        let nonce = SpeechSessionNonce(1);
        {
            let mut inner = state.inner.lock().unwrap();
            inner.controller.start(Duration::ZERO).unwrap();
            inner.controller.stop(nonce, state.now()).unwrap();
        }
        state.generation.store(nonce.0, Ordering::Release);
        let lease = state
            .admit_focus(nonce)
            .expect("focus admitted before shutdown");
        let (draining_tx, draining_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let other = Arc::clone(&state);
        let shutdown = std::thread::spawn(move || {
            shutdown_with_hook(&other, || {
                draining_tx.send(()).unwrap();
            });
            done_tx.send(()).unwrap();
        });
        draining_rx.recv().unwrap();
        // Wait until shutdown has closed admission, not for a scheduling timeout.
        while !state.focus_admission.lock().unwrap().closed {
            std::thread::yield_now();
        }
        assert!(state.admit_focus(nonce).is_none());
        assert!(done_rx.try_recv().is_err());
        drop(lease);
        done_rx.recv().unwrap();
        shutdown.join().unwrap();
    }

    #[test]
    fn delayed_stop_payload_retains_original_nonce() {
        let state = SpeechRuntimeState::default();
        {
            let mut inner = state.inner.lock().unwrap();
            inner.controller.start(Duration::ZERO).unwrap();
        }
        assert_eq!(
            state.prepare_speech_paste_target(),
            Some(crate::speech::SpeechHotkeyActivation::Stop {
                nonce: SpeechSessionNonce(1)
            })
        );
        {
            let mut inner = state.inner.lock().unwrap();
            inner
                .controller
                .stop(SpeechSessionNonce(1), state.now())
                .unwrap();
        }
        assert!(state.prepare_speech_paste_target().is_none());
    }

    #[test]
    fn cancelled_hotkey_cannot_clear_replacement_reservation() {
        let state = SpeechRuntimeState::default();
        state.inner.lock().unwrap().prepared = Some(PreparedPasteTarget {
            reservation_id: 22,
            target: None,
            prepared_at: Instant::now(),
        });
        state.cancel_speech_preparation(21);
        assert_eq!(
            state
                .inner
                .lock()
                .unwrap()
                .prepared
                .as_ref()
                .unwrap()
                .reservation_id,
            22
        );
        state.cancel_speech_preparation(22);
        assert!(state.inner.lock().unwrap().prepared.is_none());
    }

    #[test]
    fn shutdown_and_stale_generation_prevent_focus_preparation() {
        let state = SpeechRuntimeState::default();
        let nonce = SpeechSessionNonce(1);
        {
            let mut inner = state.inner.lock().unwrap();
            inner.controller.start(Duration::ZERO).unwrap();
            inner.controller.stop(nonce, state.now()).unwrap();
        }
        state.generation.store(1, Ordering::Release);
        assert!(state.can_prepare_focus(nonce));
        state.generation.store(2, Ordering::Release);
        assert!(!state.can_prepare_focus(nonce));
        state.generation.store(1, Ordering::Release);
        state.shutting_down.store(true, Ordering::Release);
        assert!(!state.can_prepare_focus(nonce));
    }

    #[test]
    fn start_reservation_requires_exact_unexpired_id() {
        let mut inner = SpeechRuntimeState::default().inner.into_inner().unwrap();
        inner.prepared = Some(PreparedPasteTarget {
            reservation_id: 17,
            target: None,
            prepared_at: Instant::now(),
        });
        assert!(inner.matches_prepared_request(StartSpeechCaptureRequest { reservation_id: 17 }));
        assert!(!inner.matches_prepared_request(StartSpeechCaptureRequest { reservation_id: 16 }));
        inner.prepared.as_mut().unwrap().prepared_at = Instant::now() - Duration::from_secs(3);
        assert!(!inner.matches_prepared_request(StartSpeechCaptureRequest { reservation_id: 17 }));
    }

    #[test]
    fn duplicate_pending_hotkey_does_not_replace_reservation() {
        let state = SpeechRuntimeState::default();
        let mut inner = state.inner.lock().unwrap();
        inner.prepared = Some(PreparedPasteTarget {
            reservation_id: 42,
            target: None,
            prepared_at: Instant::now(),
        });
        drop(inner);
        assert!(state.prepare_speech_paste_target().is_none());
        assert_eq!(
            state
                .inner
                .lock()
                .unwrap()
                .prepared
                .as_ref()
                .unwrap()
                .reservation_id,
            42
        );
    }

    #[test]
    fn only_matching_nonce_takes_session_target_and_manual_reservation_replaces_stale() {
        let mut inner = SpeechRuntimeState::default().inner.into_inner().unwrap();
        inner.paste_target = Some(SessionPasteTarget {
            nonce: SpeechSessionNonce(7),
            target: None,
        });
        assert!(inner.take_session_target(SpeechSessionNonce(6)).is_none());
        assert!(inner.take_session_target(SpeechSessionNonce(7)).is_some());
        inner.prepared = Some(PreparedPasteTarget {
            reservation_id: 1,
            target: None,
            prepared_at: Instant::now(),
        });
        inner.next_reservation = 1;
        inner.next_reservation += 1;
        inner.prepared = Some(PreparedPasteTarget {
            reservation_id: inner.next_reservation,
            target: None,
            prepared_at: Instant::now(),
        });
        assert_eq!(inner.prepared.as_ref().unwrap().reservation_id, 2);
    }

    #[test]
    fn clipboard_publisher_does_not_hold_final_input_delivery_gate() {
        let state = SpeechRuntimeState::default();
        let nonce = SpeechSessionNonce(1);
        {
            let mut inner = state.inner.lock().unwrap();
            inner.controller.start(Duration::ZERO).unwrap();
            inner
                .controller
                .stop(nonce, Duration::from_secs(1))
                .unwrap();
        }
        state.generation.store(1, Ordering::Release);
        publish_recorded_if_current(
            &state,
            nonce,
            "synthetic",
            || Duration::from_secs(2),
            || {
                assert!(state.delivery.try_lock().is_ok());
                Ok(())
            },
        )
        .unwrap();
    }

    #[test]
    fn retained_history_nonce_copies_exact_transcript() {
        let state = SpeechRuntimeState::default();
        insert_history_attempt(&state, SpeechSessionNonce(7), "exact Unicode 🦀")
            .expect("history insert");
        let mut published = None;

        copy_retained_history_transcript(
            &state,
            CopySpeechHistoryTranscriptRequest {
                nonce: SpeechSessionNonce(7),
            },
            |text| {
                published = Some(text.to_owned());
                Ok(())
            },
        )
        .expect("retained entry should copy");

        assert_eq!(published.as_deref(), Some("exact Unicode 🦀"));
    }

    #[test]
    fn unknown_history_nonce_fails_closed_without_clipboard_change() {
        let state = SpeechRuntimeState::default();
        insert_history_attempt(&state, SpeechSessionNonce(7), "retained secret")
            .expect("history insert");
        let clipboard = "existing clipboard".to_owned();

        let result = copy_retained_history_transcript(
            &state,
            CopySpeechHistoryTranscriptRequest {
                nonce: SpeechSessionNonce(99),
            },
            |_| panic!("unknown nonce must not invoke clipboard publisher"),
        );

        assert_eq!(result, Err(GENERIC_ERROR.to_owned()));
        assert_eq!(clipboard, "existing clipboard");
    }
    #[test]
    fn safe_failure_codes_are_stable_and_contain_no_raw_detail() {
        for failure in [
            SpeechFailure::CaptureStreamError,
            SpeechFailure::CaptureEmpty,
            SpeechFailure::CaptureShort,
            SpeechFailure::ModelLoadFailed,
            SpeechFailure::AsrInputInvalid,
            SpeechFailure::AsrFailed,
            SpeechFailure::TranscriptEmpty,
            SpeechFailure::Clipboard(crate::speech_clipboard::ClipboardFailure::QueueFull),
            SpeechFailure::Timeout,
            SpeechFailure::StateRace,
        ] {
            let code = failure.code();
            assert!(code
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'-'));
            assert!(!code.contains(' '));
        }
    }

    #[test]
    fn history_is_newest_first_and_keeps_five_entries() {
        let state = SpeechRuntimeState::default();
        for nonce in 1..=6 {
            record_clipboard_attempt(&state, SpeechSessionNonce(nonce), "synthetic", |_| Ok(()))
                .expect("synthetic publish");
        }
        let history = state.history.lock().expect("history");
        assert_eq!(history.len(), 5);
        assert_eq!(
            history.front().map(|entry| entry.nonce),
            Some(SpeechSessionNonce(6))
        );
        assert_eq!(
            history.back().map(|entry| entry.nonce),
            Some(SpeechSessionNonce(2))
        );
    }

    #[test]
    fn transcript_is_stored_before_clipboard_failure_and_outcome_is_updated() {
        let state = SpeechRuntimeState::default();
        let result = record_clipboard_attempt(&state, SpeechSessionNonce(9), "hello", |inserted| {
            assert_eq!(inserted.transcript, "hello");
            assert!(inserted.outcome.is_empty());
            Err(crate::speech_clipboard::ClipboardFailure::QueueFull)
        });
        assert_eq!(
            result,
            Err(crate::speech_clipboard::ClipboardFailure::QueueFull)
        );
        let history = state.history.lock().expect("history");
        assert_eq!(history[0].transcript, "hello");
        assert_eq!(history[0].outcome, "clipboard-queue-full");
    }

    #[test]
    fn retained_transcript_is_utf8_safe_and_byte_bounded() {
        let text = "😀".repeat(MAX_SPEECH_HISTORY_TRANSCRIPT_BYTES);
        let bounded = bounded_transcript(&text);
        assert!(bounded.len() <= MAX_SPEECH_HISTORY_TRANSCRIPT_BYTES);
        assert!(bounded.is_char_boundary(bounded.len()));
    }

    #[test]
    fn capture_start_failure_uses_the_public_capture_code() {
        assert_eq!(capture_start_error_code(), "capture-stream-error");
    }

    #[test]
    fn captured_audio_validation_maps_distinct_failures() {
        assert_eq!(validate_audio(&[]), Err(SpeechFailure::CaptureEmpty));
        assert_eq!(
            validate_audio(&[0.0; 100]),
            Err(SpeechFailure::CaptureShort)
        );
        let mut invalid = vec![0.0; MIN_CAPTURE_SAMPLES];
        invalid[2] = f32::NAN;
        assert_eq!(
            validate_audio(&invalid),
            Err(SpeechFailure::AsrInputInvalid)
        );
        assert!(validate_audio(&vec![0.0; MIN_CAPTURE_SAMPLES]).is_ok());
    }

    #[test]
    fn capture_health_prioritizes_stream_error_then_callback_loss() {
        let health = CaptureHealth::default();
        health.callback_drops.store(1, Ordering::Release);
        assert_eq!(
            capture_failure(&health),
            Some(SpeechFailure::CaptureCallbackLoss)
        );
        health.stream_error.store(true, Ordering::Release);
        assert_eq!(
            capture_failure(&health),
            Some(SpeechFailure::CaptureStreamError)
        );
    }

    #[test]
    #[ignore = "loads multi-gigabyte packaged model; run explicitly on release-validation host"]
    fn packaged_model_completes_asr_stage_with_generated_audio() {
        let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/speech-models/parakeet-tdt-0.6b-v2-int8");
        let mut model = crate::speech_model::load_parakeet_tdt(&model_path)
            .expect("packaged model should load");
        let audio = vec![0.0; TARGET_RATE as usize];
        model
            .transcribe_samples(audio, TARGET_RATE, 1, None)
            .expect("generated non-sensitive audio should complete ASR stage");
    }
    #[test]
    fn downmixes_and_resamples() {
        let output = resample_to_mono_16k(&[1.0, -1.0, 0.5, 0.5], 32_000, 2);
        assert_eq!(output, vec![0.0]);
    }
    #[test]
    fn rejects_invalid_audio_metadata() {
        assert!(resample_to_mono_16k(&[1.0], 0, 1).is_empty());
        assert!(resample_to_mono_16k(&[1.0], 16_000, 0).is_empty());
    }
    #[test]
    fn preserves_sixteen_khz_mono() {
        assert_eq!(
            resample_to_mono_16k(&[0.25, -0.5], 16_000, 1),
            vec![0.25, -0.5]
        );
    }

    #[test]
    fn shutdown_waits_for_commit_gate_before_invalidating() {
        let state = Arc::new(SpeechRuntimeState::default());
        state.generation.store(7, Ordering::Release);
        let commit = state.commit.lock().expect("commit gate should lock");
        let (locked_tx, locked_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker_state = Arc::clone(&state);
        let shutdown = std::thread::spawn(move || {
            shutdown_with_hook(&worker_state, || {
                locked_tx.send(()).expect("signal lock");
                release_rx.recv().expect("release shutdown");
            })
        });

        assert!(!state.shutting_down.load(Ordering::Acquire));
        assert_eq!(state.generation.load(Ordering::Acquire), 7);
        drop(commit);
        locked_rx
            .recv()
            .expect("shutdown should acquire commit gate");
        assert!(!state.shutting_down.load(Ordering::Acquire));
        assert_eq!(state.generation.load(Ordering::Acquire), 7);
        release_tx.send(()).expect("release shutdown");
        shutdown.join().expect("shutdown should finish");
        assert!(state.shutting_down.load(Ordering::Acquire));
        assert_ne!(state.generation.load(Ordering::Acquire), 7);
    }

    #[test]
    fn shutdown_can_invalidate_authorized_publisher_while_native_publish_is_in_flight() {
        let state = Arc::new(SpeechRuntimeState::default());
        let nonce = SpeechSessionNonce(1);
        {
            let mut inner = state.inner.lock().expect("runtime should lock");
            inner
                .controller
                .start(Duration::ZERO)
                .expect("start should succeed");
            inner
                .controller
                .stop(nonce, Duration::from_secs(30))
                .expect("stop should succeed");
        }
        state.generation.store(nonce.0, Ordering::Release);
        let (authorized_tx, authorized_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let publisher_calls = Arc::new(AtomicU64::new(0));
        let worker_state = Arc::clone(&state);
        let worker_calls = Arc::clone(&publisher_calls);
        let worker = std::thread::spawn(move || {
            publish_recorded_if_current(
                &worker_state,
                nonce,
                "synthetic",
                || MAX_SPEECH_OPERATION_DURATION - Duration::from_nanos(1),
                || {
                    authorized_tx.send(()).expect("signal authorization");
                    release_rx.recv().expect("release publisher");
                    worker_calls.fetch_add(1, Ordering::AcqRel);
                    Ok(())
                },
            )
        });
        authorized_rx
            .recv()
            .expect("publisher should reach authorization");

        let shutdown_state = Arc::clone(&state);
        let (shutdown_locked_tx, shutdown_locked_rx) = std::sync::mpsc::channel();
        let shutdown_thread = std::thread::spawn(move || {
            shutdown_with_hook(&shutdown_state, || {
                shutdown_locked_tx.send(()).expect("signal shutdown lock")
            })
        });
        assert!(!state.shutting_down.load(Ordering::Acquire));

        release_tx.send(()).expect("release publisher");
        let event = worker
            .join()
            .expect("worker should finish")
            .expect("delivery should complete before shutdown");
        assert_eq!(event.status, SpeechStatusKind::Copied);
        assert_eq!(event.nonce, Some(nonce));
        shutdown_locked_rx
            .recv()
            .expect("shutdown should wait for native delivery before acquiring the gate");
        shutdown_thread.join().expect("shutdown should finish");
        assert_eq!(publisher_calls.load(Ordering::Acquire), 1);
        assert!(state.shutting_down.load(Ordering::Acquire));
    }

    #[test]
    fn authorized_publish_uses_authorization_timestamp_for_copied_completion() {
        let mut controller = SpeechController::default();
        let nonce = SpeechSessionNonce(1);
        controller
            .start(Duration::from_secs(1))
            .expect("start should succeed");
        controller
            .stop(nonce, Duration::from_secs(2))
            .expect("stop should succeed");
        let authorized_at =
            Duration::from_secs(2) + MAX_SPEECH_OPERATION_DURATION - Duration::from_nanos(1);

        let event = complete_authorized_publish(&mut controller, nonce, authorized_at)
            .expect("authorized publish should complete as copied");

        assert_eq!(event.status, SpeechStatusKind::Copied);
        assert_eq!(event.error, None);
    }

    #[test]
    fn successful_publish_remains_copied_after_clock_crosses_deadline() {
        let mut controller = SpeechController::default();
        let nonce = SpeechSessionNonce(1);
        controller
            .start(Duration::ZERO)
            .expect("start should succeed");
        controller
            .stop(nonce, Duration::from_secs(30))
            .expect("stop should succeed");
        let finalization_started_at = Duration::from_secs(30);
        let finalization_deadline = finalization_started_at + MAX_SPEECH_OPERATION_DURATION;
        let mut times = [
            finalization_deadline - Duration::from_nanos(1),
            finalization_deadline,
        ]
        .into_iter();

        let event = publish_if_current(
            &mut controller,
            nonce,
            || times.next().expect("clock call should be bounded"),
            || Ok(()),
        )
        .expect("committed publication should complete");

        assert_eq!(event.status, SpeechStatusKind::Copied);
        assert_eq!(event.error, None);
        assert_eq!(times.next(), Some(finalization_deadline));
    }

    #[test]
    fn delayed_publisher_cannot_mutate_clipboard_at_session_deadline() {
        let mut controller = SpeechController::default();
        let nonce = SpeechSessionNonce(1);
        controller
            .start(Duration::ZERO)
            .expect("start should succeed");
        controller
            .stop(nonce, Duration::from_secs(30))
            .expect("stop should succeed");
        let published = AtomicBool::new(false);
        let finalization_deadline = Duration::from_secs(30) + MAX_SPEECH_OPERATION_DURATION;

        let result = publish_if_current(
            &mut controller,
            nonce,
            || finalization_deadline,
            || {
                published.store(true, Ordering::Release);
                Ok(())
            },
        );

        assert_eq!(
            result.expect("expiry should terminate session").status,
            SpeechStatusKind::Error
        );
        assert!(!published.load(Ordering::Acquire));
    }

    #[test]
    fn history_delay_crossing_deadline_blocks_publisher_and_leaves_history_unchanged() {
        let state = SpeechRuntimeState::default();
        let nonce = SpeechSessionNonce(1);
        {
            let mut inner = state.inner.lock().expect("runtime should lock");
            inner
                .controller
                .start(Duration::ZERO)
                .expect("start should succeed");
            inner
                .controller
                .stop(nonce, Duration::from_secs(30))
                .expect("stop should succeed");
        }
        state.generation.store(nonce.0, Ordering::Release);
        let published = AtomicBool::new(false);
        let finalization_deadline = Duration::from_secs(30) + MAX_SPEECH_OPERATION_DURATION;

        let result = publish_recorded_if_current(
            &state,
            nonce,
            "synthetic",
            || finalization_deadline,
            || {
                published.store(true, Ordering::Release);
                Ok(())
            },
        );

        assert_eq!(
            result.expect("timeout should terminate session").status,
            SpeechStatusKind::Error
        );
        assert!(!published.load(Ordering::Acquire));
        assert!(state.history.lock().expect("history").is_empty());
    }

    #[test]
    fn shutdown_during_history_work_blocks_publisher() {
        let state = SpeechRuntimeState::default();
        let nonce = SpeechSessionNonce(1);
        {
            let mut inner = state.inner.lock().expect("runtime should lock");
            inner
                .controller
                .start(Duration::ZERO)
                .expect("start should succeed");
            inner
                .controller
                .stop(nonce, Duration::from_secs(30))
                .expect("stop should succeed");
        }
        state.generation.store(nonce.0, Ordering::Release);
        let publisher_calls = AtomicU64::new(0);
        let finalization_deadline = Duration::from_secs(30) + MAX_SPEECH_OPERATION_DURATION;

        let result = publish_recorded_if_current(
            &state,
            nonce,
            "synthetic",
            || {
                state.invalidate();
                state.shutting_down.store(true, Ordering::Release);
                finalization_deadline - Duration::from_nanos(1)
            },
            || {
                publisher_calls.fetch_add(1, Ordering::AcqRel);
                Ok(())
            },
        );

        assert_eq!(result, Err(SpeechFailure::StateRace));
        assert_eq!(publisher_calls.load(Ordering::Acquire), 0);
        assert!(state.history.lock().expect("history").is_empty());
        assert_eq!(
            state
                .inner
                .lock()
                .expect("runtime should lock")
                .controller
                .status()
                .status,
            SpeechStatusKind::Transcribing
        );
    }

    #[test]
    fn recorded_publish_just_before_deadline_publishes_and_copies() {
        let state = SpeechRuntimeState::default();
        let nonce = SpeechSessionNonce(1);
        {
            let mut inner = state.inner.lock().expect("runtime should lock");
            inner
                .controller
                .start(Duration::ZERO)
                .expect("start should succeed");
            inner
                .controller
                .stop(nonce, Duration::from_secs(30))
                .expect("stop should succeed");
        }
        state.generation.store(nonce.0, Ordering::Release);
        let published = AtomicBool::new(false);
        let finalization_deadline = Duration::from_secs(30) + MAX_SPEECH_OPERATION_DURATION;

        let event = publish_recorded_if_current(
            &state,
            nonce,
            "synthetic",
            || finalization_deadline - Duration::from_nanos(1),
            || {
                published.store(true, Ordering::Release);
                Ok(())
            },
        )
        .expect("publication should complete");

        assert_eq!(event.status, SpeechStatusKind::Copied);
        assert!(published.load(Ordering::Acquire));
        assert_eq!(state.history.lock().expect("history")[0].outcome, "copied");
    }

    #[test]
    fn post_stop_failure_recovers_session_for_next_start() {
        let mut controller = SpeechController::default();
        let nonce = SpeechSessionNonce(1);
        controller
            .start(Duration::ZERO)
            .expect("start should succeed");
        controller
            .stop(nonce, Duration::ZERO)
            .expect("stop should succeed");

        recover_post_stop_failure(&mut controller, nonce, Duration::ZERO)
            .expect("active session should recover");
        assert!(controller.start(Duration::ZERO).is_ok());
    }

    #[test]
    fn timeout_invalidates_generation_before_new_start_can_lock_runtime() {
        let state = Arc::new(SpeechRuntimeState::default());
        let nonce = SpeechSessionNonce(1);
        {
            let mut inner = state.inner.lock().expect("runtime should lock");
            inner
                .controller
                .start(Duration::ZERO)
                .expect("start should succeed");
            inner
                .controller
                .stop(nonce, Duration::ZERO)
                .expect("stop should succeed");
        }
        state.generation.store(nonce.0, Ordering::Release);

        let event =
            expire_transcription_and_invalidate(&state, nonce, MAX_SPEECH_OPERATION_DURATION)
                .expect("timeout should expire active transcription");
        let mut inner = state.inner.lock().expect("new start should lock runtime");
        let next = inner
            .controller
            .start(MAX_SPEECH_OPERATION_DURATION)
            .expect("terminal state should permit new start");

        assert_eq!(event.status, SpeechStatusKind::Error);
        assert_eq!(state.generation.load(Ordering::Acquire), 2);
        assert_eq!(next.nonce, Some(SpeechSessionNonce(2)));
    }

    #[test]
    fn matching_copied_and_error_terminal_resets_emit_nonce_tagged_idle() {
        for terminal_status in [SpeechStatusKind::Copied, SpeechStatusKind::Error] {
            let state = SpeechRuntimeState::default();
            let nonce = SpeechSessionNonce(1);
            {
                let mut inner = state.inner.lock().expect("runtime should lock");
                inner
                    .controller
                    .start(Duration::ZERO)
                    .expect("start should succeed");
                inner
                    .controller
                    .stop(nonce, Duration::ZERO)
                    .expect("stop should succeed");
                match terminal_status {
                    SpeechStatusKind::Copied => {
                        inner.controller.complete_copied(nonce, Duration::ZERO)
                    }
                    SpeechStatusKind::Error => {
                        inner.controller.complete_error(nonce, Duration::ZERO)
                    }
                    _ => unreachable!(),
                }
                .expect("terminal completion should succeed");
            }
            state.generation.store(nonce.0, Ordering::Release);

            let idle = reset_terminal_if_current(&state, nonce, nonce.0)
                .expect("matching terminal should reset");

            assert_eq!(idle.status, SpeechStatusKind::Idle);
            assert_eq!(idle.nonce, Some(nonce));
        }
    }

    #[test]
    fn stale_terminal_reset_after_new_start_does_not_change_current_phase() {
        let state = SpeechRuntimeState::default();
        let stale_nonce = SpeechSessionNonce(1);
        {
            let mut inner = state.inner.lock().expect("runtime should lock");
            inner
                .controller
                .start(Duration::ZERO)
                .expect("start should succeed");
            inner
                .controller
                .stop(stale_nonce, Duration::ZERO)
                .expect("stop should succeed");
            inner
                .controller
                .complete_copied(stale_nonce, Duration::ZERO)
                .expect("complete should succeed");
            inner
                .controller
                .start(Duration::ZERO)
                .expect("new start should succeed");
        }
        state.generation.store(2, Ordering::Release);

        assert_eq!(reset_terminal_if_current(&state, stale_nonce, 1), None);
        let status = state
            .inner
            .lock()
            .expect("runtime should lock")
            .controller
            .status();
        assert_eq!(status.status, SpeechStatusKind::Recording);
        assert_eq!(status.nonce, Some(SpeechSessionNonce(2)));
    }

    #[test]
    fn stale_recording_limit_does_not_expire_or_reset_new_recording() {
        let state = SpeechRuntimeState::default();
        let stale_nonce = SpeechSessionNonce(1);
        let current_nonce = SpeechSessionNonce(2);
        {
            let mut inner = state.inner.lock().expect("runtime should lock");
            inner
                .controller
                .start(Duration::ZERO)
                .expect("first start should succeed");
            inner
                .controller
                .stop(stale_nonce, Duration::ZERO)
                .expect("first stop should succeed");
            inner
                .controller
                .complete_error(stale_nonce, Duration::ZERO)
                .expect("first completion should succeed");
            inner
                .controller
                .start(Duration::from_secs(1))
                .expect("new start should succeed");
        }
        state.generation.store(current_nonce.0, Ordering::Release);
        let current_timeout = Duration::from_secs(1) + MAX_RECORDING_DURATION;

        assert!(expire_recording_and_invalidate(&state, stale_nonce, current_timeout).is_none());
        assert_eq!(state.generation.load(Ordering::Acquire), current_nonce.0);
        assert_eq!(
            reset_terminal_if_current(&state, stale_nonce, stale_nonce.0),
            None
        );
        assert_eq!(
            state
                .inner
                .lock()
                .expect("runtime should lock")
                .controller
                .status(),
            crate::speech::SpeechStatusResponse {
                status: SpeechStatusKind::Recording,
                nonce: Some(current_nonce),
            }
        );

        let (error, capture) =
            expire_recording_and_invalidate(&state, current_nonce, current_timeout)
                .expect("current recording should expire");
        assert!(capture.is_none());
        assert_eq!(error.status, SpeechStatusKind::Error);
        assert_eq!(error.nonce, Some(current_nonce));
        let idle = reset_terminal_if_current(
            &state,
            current_nonce,
            state.generation.load(Ordering::Acquire),
        )
        .expect("matching timeout should reset");
        assert_eq!(idle.status, SpeechStatusKind::Idle);
        assert_eq!(idle.nonce, Some(current_nonce));
    }

    #[test]
    fn shutdown_suppresses_terminal_reset() {
        let state = SpeechRuntimeState::default();
        let nonce = SpeechSessionNonce(1);
        {
            let mut inner = state.inner.lock().expect("runtime should lock");
            inner
                .controller
                .start(Duration::ZERO)
                .expect("start should succeed");
            inner
                .controller
                .stop(nonce, Duration::ZERO)
                .expect("stop should succeed");
            inner
                .controller
                .complete_error(nonce, Duration::ZERO)
                .expect("complete should succeed");
        }
        state.generation.store(nonce.0, Ordering::Release);
        state.shutting_down.store(true, Ordering::Release);

        assert_eq!(reset_terminal_if_current(&state, nonce, nonce.0), None);
        assert_eq!(
            state
                .inner
                .lock()
                .expect("runtime should lock")
                .controller
                .status()
                .status,
            SpeechStatusKind::Error
        );
    }
}
