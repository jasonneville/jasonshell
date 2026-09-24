//! Bounded native microphone capture and in-process speech worker.

use crate::contracts;
use crate::speech::{
    CopySpeechHistoryTranscriptRequest, SpeechController, SpeechHistoryEntry, SpeechSessionNonce,
    SpeechStatusEvent, SpeechStatusKind, StartSpeechCaptureResponse, StopSpeechCaptureRequest,
    MAX_RECORDING_DURATION, MAX_SPEECH_HISTORY_ENTRIES, MAX_SPEECH_HISTORY_TRANSCRIPT_BYTES,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use parakeet_rs::Transcriber;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
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

#[derive(Default)]
struct CaptureHealth {
    stream_error: AtomicBool,
    callback_drops: AtomicU64,
}

struct Capture {
    stream: Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    health: Arc<CaptureHealth>,
}

pub(crate) struct SpeechRuntimeState {
    inner: Mutex<RuntimeInner>,
    generation: AtomicU64,
    commit: Mutex<()>,
    shutting_down: AtomicBool,
    epoch: Instant,
    history: Mutex<VecDeque<SpeechHistoryEntry>>,
}

struct RuntimeInner {
    controller: SpeechController,
    capture: Option<Capture>,
}

impl Default for SpeechRuntimeState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(RuntimeInner {
                controller: SpeechController::default(),
                capture: None,
            }),
            generation: AtomicU64::new(0),
            commit: Mutex::new(()),
            shutting_down: AtomicBool::new(false),
            epoch: Instant::now(),
            history: Mutex::new(VecDeque::with_capacity(MAX_SPEECH_HISTORY_ENTRIES)),
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

fn copy_retained_history_transcript<F>(
    state: &SpeechRuntimeState,
    request: CopySpeechHistoryTranscriptRequest,
    publish: F,
) -> Result<(), String>
where
    F: FnOnce(&str) -> Result<(), crate::speech_clipboard::ClipboardFailure>,
{
    let history = state.history.lock().map_err(|_| GENERIC_ERROR.to_owned())?;
    let entry = history
        .iter()
        .find(|entry| entry.nonce == request.nonce)
        .ok_or_else(|| GENERIC_ERROR.to_owned())?;
    publish(&entry.transcript).map_err(|_| GENERIC_ERROR.to_owned())
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
    fn now(&self) -> Duration {
        self.epoch.elapsed()
    }

    fn invalidate(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
    }
}

#[tauri::command]
pub(crate) fn start_speech_capture(
    app: AppHandle,
    state: State<'_, SpeechRuntimeState>,
) -> Result<StartSpeechCaptureResponse, String> {
    {
        let inner = state.inner.lock().map_err(|_| GENERIC_ERROR)?;
        if inner.capture.is_some() {
            return Err("Speech is busy".into());
        }
    }
    let capture = match create_capture() {
        Ok(capture) => capture,
        Err(_) => {
            report_capture_start_failure();
            return Err(capture_start_error_code());
        }
    };
    if capture.stream.play().is_err() {
        report_capture_start_failure();
        return Err(capture_start_error_code());
    }
    let event = {
        let mut inner = state.inner.lock().map_err(|_| GENERIC_ERROR)?;
        let event = inner.controller.start(state.now()).map_err(str::to_owned)?;
        inner.capture = Some(capture);
        event
    };
    let nonce = event.nonce.ok_or_else(|| GENERIC_ERROR.to_string())?;
    state.generation.store(nonce.0, Ordering::Release);
    emit(&app, &event)?;
    schedule_recording_limit(app, nonce);
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
        samples,
        sample_rate,
        channels,
        health,
    } = capture;
    drop(stream);
    let samples = match samples.lock() {
        Ok(samples) => samples.clone(),
        Err(_) => {
            debug_diagnostic(
                request.nonce,
                SpeechFailure::StateRace,
                Duration::ZERO,
                sample_rate,
                channels,
                0,
                true,
                0.0,
            );
            recover_failed_stop(&app, &state, request.nonce);
            return Err(GENERIC_ERROR.into());
        }
    };
    let (sample_count, finite, peak) = sample_metrics(&samples);
    if let Some(failure) = capture_failure(&health) {
        debug_diagnostic(
            request.nonce,
            failure,
            Duration::ZERO,
            sample_rate,
            channels,
            sample_count,
            finite,
            peak,
        );
        recover_failed_stop_with(&app, &state, request.nonce, failure);
        return Err(GENERIC_ERROR.into());
    }
    let audio = resample_to_mono_16k(&samples, sample_rate, channels);
    if let Err(failure) = validate_audio(&audio) {
        let (sample_count, finite, peak) = sample_metrics(&audio);
        debug_diagnostic(
            request.nonce,
            failure,
            Duration::ZERO,
            TARGET_RATE,
            1,
            sample_count,
            finite,
            peak,
        );
        recover_failed_stop_with(&app, &state, request.nonce, failure);
        return Err(GENERIC_ERROR.into());
    }
    let model_path = match resolve_model_resource(&app) {
        Ok(path) => path,
        Err(error) => {
            let (sample_count, finite, peak) = sample_metrics(&audio);
            debug_diagnostic(
                request.nonce,
                SpeechFailure::ModelLoadFailed,
                Duration::ZERO,
                TARGET_RATE,
                1,
                sample_count,
                finite,
                peak,
            );
            recover_failed_stop_with(&app, &state, request.nonce, SpeechFailure::ModelLoadFailed);
            return Err(error);
        }
    };
    let worker_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_worker(worker_app, request.nonce, audio, model_path)
    });
    schedule_transcription_limit(app, request.nonce);
    Ok(crate::speech::SpeechStatusResponse {
        status: SpeechStatusKind::Transcribing,
        nonce: Some(request.nonce),
    })
}

fn run_worker(app: AppHandle, nonce: SpeechSessionNonce, audio: Vec<f32>, model_path: PathBuf) {
    let started = Instant::now();
    let (sample_count, finite, peak) = sample_metrics(&audio);
    let result: Result<_, SpeechFailure> = (|| {
        validate_audio(&audio)?;
        let mut model = crate::speech_model::load_parakeet_tdt(&model_path)
            .map_err(|_| SpeechFailure::ModelLoadFailed)?;
        let result = model
            .transcribe_samples(audio, TARGET_RATE, 1, None)
            .map_err(|_| SpeechFailure::AsrFailed)?;
        let text = result.text.trim();
        if text.is_empty() {
            return Err(SpeechFailure::TranscriptEmpty);
        }
        let state = app.state::<SpeechRuntimeState>();
        publish_recorded_if_current(
            &state,
            nonce,
            text,
            || state.now(),
            || crate::speech_clipboard::write_unicode_text(text),
        )
    })();
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
                    finite,
                    peak,
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
        let _ = app.emit(contracts::events::SPEECH_STATUS_CHANGED, event);
        schedule_terminal_reset(app, nonce);
    }
}

fn complete_authorized_publish(
    controller: &mut SpeechController,
    nonce: SpeechSessionNonce,
    authorized_at: Duration,
) -> Option<SpeechStatusEvent> {
    controller.complete_copied(nonce, authorized_at)
}

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
    insert_history_attempt(state, nonce, transcript).map_err(|_| SpeechFailure::StateRace)?;

    let commit = state.commit.lock().map_err(|_| SpeechFailure::StateRace)?;
    let publish_at = now();
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
        drop(commit);
        update_history_outcome(state, nonce, SpeechFailure::Timeout.code())
            .map_err(|_| SpeechFailure::StateRace)?;
        return state
            .inner
            .lock()
            .map_err(|_| SpeechFailure::StateRace)?
            .controller
            .complete_error_code(nonce, publish_at, SpeechFailure::Timeout.code())
            .ok_or(SpeechFailure::StateRace);
    }

    if let Err(failure) = publish() {
        drop(commit);
        update_history_outcome(state, nonce, failure.code())
            .map_err(|_| SpeechFailure::StateRace)?;
        return Err(SpeechFailure::Clipboard(failure));
    }
    let event = state
        .inner
        .lock()
        .map_err(|_| SpeechFailure::StateRace)?
        .controller
        .complete_copied(nonce, publish_at)
        .ok_or(SpeechFailure::StateRace)?;
    drop(commit);
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
    state.invalidate();
    Some(event)
}

fn expire_recording_and_invalidate(
    state: &SpeechRuntimeState,
    nonce: SpeechSessionNonce,
    now: Duration,
) -> Option<(SpeechStatusEvent, Option<Capture>)> {
    let mut inner = state.inner.lock().ok()?;
    let event = inner.controller.expire_recording(nonce, now)?;
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
    state.inner.lock().ok()?.controller.reset(nonce)
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
            let _ = app.emit(contracts::events::SPEECH_STATUS_CHANGED, event);
        }
    });
}

fn schedule_recording_limit(app: AppHandle, nonce: SpeechSessionNonce) {
    std::thread::spawn(move || {
        std::thread::sleep(MAX_RECORDING_DURATION);
        let state = app.state::<SpeechRuntimeState>();
        let Some((event, capture)) = expire_recording_and_invalidate(&state, nonce, state.now())
        else {
            return;
        };
        let (source_rate, channels, samples, finite, peak) = capture
            .as_ref()
            .map(capture_metrics)
            .unwrap_or((0, 0, 0, true, 0.0));
        debug_diagnostic(
            nonce,
            SpeechFailure::Timeout,
            Duration::ZERO,
            source_rate,
            channels,
            samples,
            finite,
            peak,
        );
        drop(capture);
        let event_nonce = event.nonce;
        let _ = app.emit(contracts::events::SPEECH_STATUS_CHANGED, event);
        if let Some(event_nonce) = event_nonce {
            schedule_terminal_reset(app, event_nonce);
        }
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
        let event = {
            let state = app.state::<SpeechRuntimeState>();
            let _commit = match state.commit.lock() {
                Ok(value) => value,
                Err(_) => return,
            };
            expire_transcription_and_invalidate(&state, nonce, state.now())
        };
        if let Some(event) = event {
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
            let _ = app.emit(contracts::events::SPEECH_STATUS_CHANGED, event);
            schedule_terminal_reset(app, nonce);
        }
    });
}

pub(crate) fn shutdown(state: &SpeechRuntimeState) {
    let _commit = state.commit.lock().ok();
    shutdown_after_commit(state);
}

#[cfg(test)]
fn shutdown_with_hook<F>(state: &SpeechRuntimeState, after_commit_locked: F)
where
    F: FnOnce(),
{
    let _commit = state.commit.lock().ok();
    after_commit_locked();
    shutdown_after_commit(state);
}

fn shutdown_after_commit(state: &SpeechRuntimeState) {
    state.invalidate();
    state.shutting_down.store(true, Ordering::Release);
    let capture = state
        .inner
        .lock()
        .ok()
        .and_then(|mut inner| inner.capture.take());
    drop(capture);
    if let Ok(mut history) = state.history.lock() {
        history.clear();
    }
}

fn emit(app: &AppHandle, event: &SpeechStatusEvent) -> Result<(), String> {
    app.emit(contracts::events::SPEECH_STATUS_CHANGED, event)
        .map_err(|_| GENERIC_ERROR.to_string())
}

fn resolve_model_resource(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .resolve(MODEL_RESOURCE, tauri::path::BaseDirectory::Resource)
        .map_err(|_| GENERIC_ERROR.to_string())
}

fn create_capture() -> Result<Capture, ()> {
    let device = cpal::default_host().default_input_device().ok_or(())?;
    let supported = device.default_input_config().map_err(|_| ())?;
    let config: StreamConfig = supported.config();
    let sample_rate = config.sample_rate;
    let channels = config.channels;
    let max_samples =
        sample_rate as usize * channels as usize * MAX_RECORDING_DURATION.as_secs() as usize;
    let samples = Arc::new(Mutex::new(Vec::with_capacity(max_samples)));
    let health = Arc::new(CaptureHealth::default());
    let stream = match supported.sample_format() {
        SampleFormat::F32 => {
            build_stream::<f32>(&device, &config, &samples, &health, max_samples, |v| v)
        }
        SampleFormat::I16 => {
            build_stream::<i16>(&device, &config, &samples, &health, max_samples, |v| {
                v as f32 / i16::MAX as f32
            })
        }
        SampleFormat::U16 => {
            build_stream::<u16>(&device, &config, &samples, &health, max_samples, |v| {
                (v as f32 / u16::MAX as f32) * 2.0 - 1.0
            })
        }
        _ => return Err(()),
    }?;
    Ok(Capture {
        stream,
        samples,
        sample_rate,
        channels,
        health,
    })
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
    samples: &Arc<Mutex<Vec<f32>>>,
    health: &Arc<CaptureHealth>,
    max: usize,
    convert: fn(T) -> f32,
) -> Result<Stream, ()> {
    let target = Arc::clone(samples);
    let callback_health = Arc::clone(health);
    let error_health = Arc::clone(health);
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                if let Ok(mut output) = target.try_lock() {
                    let remaining = max.saturating_sub(output.len());
                    output.extend(data.iter().copied().take(remaining).map(convert));
                } else {
                    callback_health
                        .callback_drops
                        .fetch_add(1, Ordering::Relaxed);
                }
            },
            move |_| error_health.stream_error.store(true, Ordering::Release),
            None,
        )
        .map_err(|_| ())
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

fn capture_metrics(capture: &Capture) -> (u32, u16, usize, bool, f32) {
    let (samples, finite, peak) = capture
        .samples
        .lock()
        .map(|samples| sample_metrics(&samples))
        .unwrap_or((0, true, 0.0));
    (capture.sample_rate, capture.channels, samples, finite, peak)
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

fn sample_metrics(audio: &[f32]) -> (usize, bool, f32) {
    (
        audio.len(),
        audio.iter().all(|sample| sample.is_finite()),
        audio
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs())),
    )
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
    fn shutdown_cannot_overtake_authorized_publisher() {
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
        worker
            .join()
            .expect("worker should finish")
            .expect("publish should succeed");
        shutdown_locked_rx
            .recv()
            .expect("shutdown should acquire commit gate after publisher");
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
        let authorized_at = Duration::from_secs(120);

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
        let mut times = [
            MAX_SPEECH_OPERATION_DURATION - Duration::from_nanos(1),
            MAX_SPEECH_OPERATION_DURATION,
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
        assert_eq!(times.next(), Some(MAX_SPEECH_OPERATION_DURATION));
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

        let result = publish_if_current(
            &mut controller,
            nonce,
            || MAX_SPEECH_OPERATION_DURATION,
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
    fn history_delay_crossing_deadline_blocks_publisher_and_records_timeout() {
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

        let result = publish_recorded_if_current(
            &state,
            nonce,
            "synthetic",
            || MAX_SPEECH_OPERATION_DURATION,
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
        assert_eq!(state.history.lock().expect("history")[0].outcome, "timeout");
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

        let result = publish_recorded_if_current(
            &state,
            nonce,
            "synthetic",
            || {
                state.invalidate();
                state.shutting_down.store(true, Ordering::Release);
                MAX_SPEECH_OPERATION_DURATION - Duration::from_nanos(1)
            },
            || {
                publisher_calls.fetch_add(1, Ordering::AcqRel);
                Ok(())
            },
        );

        assert_eq!(result, Err(SpeechFailure::StateRace));
        assert_eq!(publisher_calls.load(Ordering::Acquire), 0);
        assert!(state.history.lock().expect("history")[0].outcome.is_empty());
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

        let event = publish_recorded_if_current(
            &state,
            nonce,
            "synthetic",
            || MAX_SPEECH_OPERATION_DURATION - Duration::from_nanos(1),
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
