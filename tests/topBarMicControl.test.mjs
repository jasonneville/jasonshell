import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  INITIAL_MIC_CONTROL_MODEL,
  MIC_CONTROL_PRESENTATION,
  acceptSpeechStart,
  acceptSpeechStop,
  beginSpeechStart,
  beginSpeechStop,
  failSpeechCommand,
  reduceSpeechEvent,
  settleSpeechCommand,
  normalizeSpeechCommandError,
  normalizeSpeechErrorCode,
  speechControlLabel
} from '../dist-tests/features/top-bar/micControlState.js';

const source = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const topBarSource = source('../src/components/TopBar.svelte');
const micSource = source('../src/components/TopBarMicControl.svelte');
const speechSource = source('../src/lib/speech.ts');
const css = source('../src/components/TopBar.css');

test('mic uses packaged filled microphone mask', () => {
  assert.match(micSource, /mic_24dp_E3E3E3_FILL1_wght300_GRAD0_opsz24\.svg/);
  assert.doesNotMatch(micSource, /mic_24dp_E3E3E3_FILL0_wght300_GRAD0_opsz24\.svg/);
});

test('native start alone establishes active recording nonce', () => {
  const pending = beginSpeechStart(INITIAL_MIC_CONTROL_MODEL);
  assert.equal(pending.state, 'starting');
  assert.equal(pending.nonce, null);
  assert.deepEqual(acceptSpeechStart(pending, { nonce: 5, status: 'recording' }), {
    state: 'recording', nonce: 5, announcement: 'Recording speech', errorCode: null
  });
  assert.equal(acceptSpeechStart(pending, { nonce: 5, status: 'error' }).state, 'starting');
});

test('stop waits for native response and sends current nonce', () => {
  const recording = { state: 'recording', nonce: 5, announcement: 'Recording speech' };
  const pending = beginSpeechStop(recording);
  assert.equal(pending.state, 'stopping');
  assert.equal(acceptSpeechStop(pending, { nonce: 4, status: 'transcribing' }).state, 'stopping');
  assert.equal(acceptSpeechStop(pending, { nonce: 5, status: 'transcribing' }).state, 'transcribing');
  assert.match(micSource, /stopSpeechCapture\(\{ nonce \}\)/);
});

test('matching terminal idle reset returns copied control to retry-ready idle', () => {
  const active = { state: 'transcribing', nonce: 5, announcement: 'Transcribing speech' };
  assert.strictEqual(reduceSpeechEvent(active, { nonce: 4, status: 'copied' }), active);
  const copied = reduceSpeechEvent(active, { nonce: 5, status: 'copied' });
  assert.equal(copied.state, 'copied');
  assert.strictEqual(reduceSpeechEvent(copied, { nonce: 4, status: 'error' }), copied);
  assert.deepEqual(reduceSpeechEvent(copied, { nonce: 5, status: 'idle' }), INITIAL_MIC_CONTROL_MODEL);
  assert.equal(MIC_CONTROL_PRESENTATION.idle.disabled, false);
});

test('mismatched terminal idle reset cannot clear a newer nonce', () => {
  const copied = { state: 'copied', nonce: 8, announcement: 'Speech copied to clipboard' };
  assert.strictEqual(reduceSpeechEvent(copied, { nonce: 7, status: 'idle' }), copied);
});

test('disposed start and stop settlements cannot mutate model', async () => {
  for (const commandKind of ['start', 'stop']) {
    for (const outcome of ['resolve', 'reject']) {
      let disposed = false;
      let mutations = 0;
      let resolveCommand;
      let rejectCommand;
      const command = new Promise((resolve, reject) => {
        resolveCommand = resolve;
        rejectCommand = reject;
      });
      const settled = settleSpeechCommand(
        command,
        () => disposed,
        () => { mutations += 1; },
        () => { mutations += 1; }
      );
      disposed = true;
      if (outcome === 'resolve') {
        resolveCommand({ nonce: 8, status: commandKind === 'start' ? 'recording' : 'transcribing' });
      } else {
        rejectCommand(new Error('disposed command failure'));
      }
      await settled;
      assert.equal(mutations, 0, `disposed ${commandKind} ${outcome} mutated model`);
    }
  }
});

test('command and native failures never fabricate copied status', () => {
  const failedStart = failSpeechCommand(beginSpeechStart(INITIAL_MIC_CONTROL_MODEL));
  assert.deepEqual(failedStart, {
    state: 'error',
    nonce: null,
    errorCode: 'state-race',
    announcement: 'Speech transcription failed: state-race'
  });
  assert.equal(MIC_CONTROL_PRESENTATION.error.pressed, false);
  assert.equal(MIC_CONTROL_PRESENTATION.error.disabled, false);
  assert.match(MIC_CONTROL_PRESENTATION.error.label, /failed/i);
  assert.doesNotMatch(micSource, /clipboard|copied\s*=/i);
});

test('native failures expose only whitelisted diagnostic codes', () => {
  const active = { state: 'transcribing', nonce: 5, announcement: 'Transcribing speech', errorCode: null };
  const asrFailure = reduceSpeechEvent(active, { nonce: 5, status: 'error', error: 'asr-failed' });
  assert.equal(asrFailure.errorCode, 'asr-failed');
  assert.equal(asrFailure.announcement, 'Speech transcription failed: asr-failed');
  assert.equal(speechControlLabel(asrFailure), 'Speech transcription failed: asr-failed');
  assert.equal(normalizeSpeechErrorCode('ORT error C:\\private\\model.onnx'), 'state-race');
  assert.equal(normalizeSpeechCommandError('capture-stream-error'), 'capture-stream-error');
  assert.equal(normalizeSpeechCommandError({ message: 'ASR driver C:\\private\\model.onnx' }), 'state-race');
  assert.doesNotMatch(speechControlLabel(reduceSpeechEvent(active, {
    nonce: 5,
    status: 'error',
    error: 'ORT error C:\\private\\model.onnx'
  })), /ORT|private|onnx/i);
});

test('opaque clipboard failures normalize and render unchanged while unknown details stay private', () => {
  const active = { state: 'transcribing', nonce: 5, announcement: 'Transcribing speech', errorCode: null };
  const clipboardCodes = [
    'clipboard-sta-unavailable',
    'clipboard-queue-full',
    'clipboard-invalid-text',
    'clipboard-timeout',
    'clipboard-publish-rejected'
  ];

  for (const code of clipboardCodes) {
    assert.equal(normalizeSpeechErrorCode(code), code);
    const failed = reduceSpeechEvent(active, { nonce: 5, status: 'error', error: code });
    assert.equal(failed.errorCode, code);
    assert.equal(speechControlLabel(failed), `Speech transcription failed: ${code}`);
  }

  for (const unknown of ['HRESULT 0x800401D0', 'raw OS clipboard failure', '', null, undefined]) {
    assert.equal(normalizeSpeechErrorCode(unknown), 'state-race');
  }
});

test('terminal idle reset removes current failure without persistent diagnostic UI or state', () => {
  const active = { state: 'transcribing', nonce: 5, announcement: 'Transcribing speech', errorCode: null };
  const failed = reduceSpeechEvent(active, { nonce: 5, status: 'error', error: 'capture-short' });
  const reset = reduceSpeechEvent(failed, { nonce: 5, status: 'idle' });
  assert.deepEqual(reset, INITIAL_MIC_CONTROL_MODEL);
  assert.doesNotMatch(micSource, /micDiagnostic|mic-diagnostic|data-speech-(?:last-)?error-code/);
  assert.doesNotMatch(css, /\.mic-diagnostic/);
  assert.match(css, /\.top-bar \.mic-control[\s\S]*display: inline-flex/);
});

test('typed Tauri wrapper owns canonical invoke and listen contracts', () => {
  assert.match(speechSource, /invoke<StartSpeechCaptureResponse>\(IPC_COMMANDS\.startSpeechCapture\)/);
  assert.match(speechSource, /invoke<SpeechStatusResponse>\(IPC_COMMANDS\.stopSpeechCapture, \{ request \}\)/);
  assert.match(speechSource, /listen<SpeechStatusEvent>\(SPEECH_STATUS_CHANGED_EVENT/);
});

test('component uses lifecycle-safe listener and accessible native-driven presentation', () => {
  assert.match(topBarSource, /<TopBarMicControl \/>[\s\S]*class="sound-control"/);
  assert.match(micSource, /let disposed = false;[\s\S]*onMount\(\(\) => \{[\s\S]*if \(disposed\) resolvedUnlisten\(\)/);
  assert.match(micSource, /role="status" aria-live="polite"/);
  assert.equal(MIC_CONTROL_PRESENTATION.idle.label, 'Start recording and transcribe');
  assert.equal(MIC_CONTROL_PRESENTATION.recording.label, 'Stop recording and transcribe');
  assert.equal(MIC_CONTROL_PRESENTATION.transcribing.label, 'Transcribing speech');
  assert.equal(MIC_CONTROL_PRESENTATION.recording.pressed, true);
  for (const [state, presentation] of Object.entries(MIC_CONTROL_PRESENTATION)) {
    if (state !== 'recording') assert.equal(presentation.pressed, false);
  }
  assert.equal(MIC_CONTROL_PRESENTATION.starting.disabled, true);
  assert.equal(MIC_CONTROL_PRESENTATION.stopping.disabled, true);
  assert.equal(MIC_CONTROL_PRESENTATION.transcribing.disabled, true);
  assert.match(micSource, /ariaLabel=\{micLabel\}[\s\S]*tooltip=\{micLabel\}/);
  assert.doesNotMatch(micSource, /class="mic-diagnostic" role="status"/);
  assert.doesNotMatch(micSource, /console\.error\([^)]*error/);
});

test('accepted visual cues remain non-color, reduced-motion, and forced-color safe', () => {
  assert.match(css, /\.mic-button--recording::after/);
  assert.match(micSource, /\{#if micModel\.state === 'stopping' \|\| micModel\.state === 'transcribing'\}[\s\S]*class="mic-progress-spinner" aria-hidden="true"/);
  assert.match(micSource, /\{#if micModel\.state === 'copied'\}[\s\S]*class="mic-copy-check" aria-hidden="true">✓<\/span>/);
  assert.match(css, /\.mic-progress-spinner[\s\S]*animation: mic-progress-spin/);
  assert.match(css, /\.mic-copy-check/);
  assert.match(css, /\.mic-button--error::after[\s\S]*content: '!'/);
  assert.match(css, /@media \(prefers-reduced-motion: reduce\)[\s\S]*\.mic-progress-spinner[\s\S]*animation: none/);
  assert.match(css, /@media \(forced-colors: active\)[\s\S]*\.mic-progress-spinner[\s\S]*\.mic-copy-check/);
});
