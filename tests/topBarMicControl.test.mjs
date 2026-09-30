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

function cssRule(selector, sourceText = css) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = sourceText.match(new RegExp(`${escapedSelector}\\s*\\{([^}]*)\\}`));
  assert.ok(match, `missing CSS rule: ${selector}`);
  return match[1];
}

function cssDeclarationValue(ruleText, property) {
  const escapedProperty = property.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = ruleText.match(new RegExp(`(?:^|;)\\s*${escapedProperty}\\s*:\\s*([^;]+)`));
  return match?.[1].trim() ?? null;
}

function forcedColorsCss() {
  let mediaStart = css.indexOf('@media (forced-colors: active)');
  while (mediaStart !== -1) {
    const blockStart = css.indexOf('{', mediaStart);
    let depth = 0;
    for (let index = blockStart; index < css.length; index += 1) {
      if (css[index] === '{') depth += 1;
      if (css[index] === '}') depth -= 1;
      if (depth === 0) {
        const block = css.slice(blockStart + 1, index);
        if (block.includes('.top-bar .mic-copy-check')) return block;
        break;
      }
    }
    mediaStart = css.indexOf('@media (forced-colors: active)', mediaStart + 1);
  }
  assert.fail('missing forced-colors mic block');
}

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
  assert.match(speechSource, /startSpeechCapture\(reservation: StartSpeechCaptureRequest\)/);
  assert.match(speechSource, /const request = reservation;\s*return invoke<StartSpeechCaptureResponse>\(IPC_COMMANDS\.startSpeechCapture, \{ request \}\)/);
  assert.match(source('../src/ipc/commands.ts'), /interface StartSpeechCaptureRequest\s*\{\s*reservationId: number;/);
  assert.match(speechSource, /invoke<SpeechStatusResponse>\(IPC_COMMANDS\.stopSpeechCapture, \{ request \}\)/);
  assert.match(speechSource, /listen<SpeechStatusEvent>\(SPEECH_STATUS_CHANGED_EVENT/);
});

test('component uses lifecycle-safe listener and accessible native-driven presentation', () => {
  assert.match(topBarSource, /<TopBarMicControl bind:this=\{micControl\} \/>[\s\S]*class="sound-control"/);
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

test('idle mic uses the same text color token as adjacent top bar icons', () => {
  const micControlRule = cssRule('.top-bar .mic-control');
  const micButtonRule = cssRule('.top-bar .mic-button');

  assert.equal(cssDeclarationValue(micControlRule, '--mic-idle-color'), 'var(--js-color-text)');
  assert.equal(cssDeclarationValue(micButtonRule, 'color'), 'var(--mic-idle-color)');
});

test('recording mic keeps color and pulse without a visible border or outer glow', () => {
  const recordingRule = cssRule('.top-bar .mic-button--recording');
  const pulseRule = cssRule('.top-bar .mic-button--recording::after');

  assert.ok(cssDeclarationValue(recordingRule, 'background'), 'recording background remains visible');
  assert.equal(cssDeclarationValue(recordingRule, 'color'), 'var(--mic-recording-color)');
  for (const property of ['border', 'border-color', 'box-shadow']) {
    assert.equal(cssDeclarationValue(recordingRule, property), null, `recording rule must omit ${property}`);
  }
  assert.ok(cssDeclarationValue(pulseRule, 'animation')?.includes('mic-recording-pulse'));
  assert.equal(cssDeclarationValue(pulseRule, 'background'), 'currentColor');
});

test('copied speech button drops success ring and circular badge treatment', () => {
  const copiedButtonRule = cssRule('.top-bar .mic-button--copied');
  const checkRule = cssRule('.top-bar .mic-copy-check');

  assert.equal(cssDeclarationValue(copiedButtonRule, 'border-color'), null);
  assert.equal(cssDeclarationValue(copiedButtonRule, 'border-style'), null);
  assert.equal(cssDeclarationValue(copiedButtonRule, 'box-shadow'), null);

  assert.equal(cssDeclarationValue(checkRule, 'color'), '#22c55e');
  assert.equal(cssDeclarationValue(checkRule, 'font-size'), '0.52rem');
  assert.equal(cssDeclarationValue(checkRule, 'background'), null);
  assert.equal(cssDeclarationValue(checkRule, 'border'), null);
  assert.equal(cssDeclarationValue(checkRule, 'border-radius'), null);
  assert.equal(cssDeclarationValue(checkRule, 'height'), null);
  assert.equal(cssDeclarationValue(checkRule, 'width'), null);
});

test('copied speech forced-colors mode stays legible without reintroducing success rings', () => {
  const forcedColors = forcedColorsCss();
  const forcedCopiedButtonRule = cssRule('.top-bar .mic-button--copied', forcedColors);
  const forcedCheckRule = cssRule('.top-bar .mic-copy-check', forcedColors);

  assert.equal(cssDeclarationValue(forcedCopiedButtonRule, 'border-color'), null);
  assert.equal(cssDeclarationValue(forcedCopiedButtonRule, 'box-shadow'), null);
  assert.equal(cssDeclarationValue(forcedCopiedButtonRule, 'color'), 'ButtonText');
  assert.equal(cssDeclarationValue(forcedCheckRule, 'color'), 'Highlight');
  assert.equal(cssDeclarationValue(forcedCheckRule, 'background'), null);
  assert.equal(cssDeclarationValue(forcedCheckRule, 'border-color'), null);
});

test('mic hover matches adjacent control and successful paste check overlays visible mic glyph', () => {
  const micHover = cssRule('.top-bar .mic-button:hover');
  const terminalHover = css.match(/\.top-bar \.terminal-button:hover,\s*\.top-bar \.command-button:hover\s*\{([^}]*)\}/)?.[1];
  assert.ok(terminalHover, 'missing shared neighboring hover rule');
  for (const property of ['background', 'border-color']) {
    assert.equal(cssDeclarationValue(micHover, property), cssDeclarationValue(terminalHover, property));
  }
  const button = cssRule('.top-bar .mic-button');
  const check = cssRule('.top-bar .mic-copy-check');
  assert.equal(cssDeclarationValue(button, 'position'), 'relative');
  assert.equal(cssDeclarationValue(check, 'position'), 'absolute');
  assert.equal(cssDeclarationValue(check, 'color'), '#22c55e');
  assert.match(cssDeclarationValue(check, 'bottom') ?? '', /^-?0\./);
  assert.match(cssDeclarationValue(check, 'right') ?? '', /^-?0\./);
  assert.match(micSource, /micModel\.state === 'copied'[\s\S]*mic-copy-check/);
  assert.match(micSource, /mic-glyph[\s\S]*mic-copy-check/);
});

test('planned cap finalization event announces finishing without countdown or cancellation copy', (t) => {
  if (!speechSource.includes('finalizationReason') && !micSource.includes('finalizationReason')) {
    t.skip('RED-ready acceptance: enable when cap finalization UI contract is implemented');
    return;
  }

  const recording = { state: 'recording', nonce: 5, announcement: 'Recording speech', errorCode: null };
  const capped = reduceSpeechEvent(recording, {
    nonce: 5,
    status: 'transcribing',
    finalizationReason: 'recording_cap'
  });
  assert.equal(capped.state, 'transcribing');
  assert.equal(capped.announcement, 'Recording limit reached; finishing dictation.');

  const manual = reduceSpeechEvent(recording, { nonce: 5, status: 'transcribing' });
  assert.equal(manual.announcement, 'Transcribing speech');
  assert.strictEqual(
    reduceSpeechEvent(recording, { nonce: 4, status: 'transcribing', finalizationReason: 'recording_cap' }),
    recording
  );
  assert.doesNotMatch(micSource, /countdown|seconds remaining|cancelled|discarded/i);
});
