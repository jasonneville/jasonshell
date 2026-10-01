import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const rustContracts = readFileSync(new URL('../src-tauri/src/contracts.rs', import.meta.url), 'utf8');
const tsCommands = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const tsEvents = readFileSync(new URL('../src/ipc/events.ts', import.meta.url), 'utf8');
const tauriConfig = JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const shellWindows = readFileSync(new URL('../src-tauri/src/shell_windows.rs', import.meta.url), 'utf8');
const surfaces = readFileSync(new URL('../src/ipc/surfaces.ts', import.meta.url), 'utf8');
const surfaceLoader = readFileSync(new URL('../src/lib/surfaceLoader.ts', import.meta.url), 'utf8');
const speechApi = readFileSync(new URL('../src/lib/speech.ts', import.meta.url), 'utf8');
const rustSpeech = readFileSync(new URL('../src-tauri/src/speech.rs', import.meta.url), 'utf8');

test('speech command and event names match Rust and TypeScript contracts', () => {
  for (const command of [
    'start_speech_capture',
    'stop_speech_capture',
    'get_speech_history',
    'copy_speech_history_transcript'
  ]) {
    assert.match(rustContracts, new RegExp(`"${command}"`));
    assert.match(tsCommands, new RegExp(`'${command}'`));
  }
  assert.match(rustContracts, /"speech:status-changed"/);
  assert.match(tsEvents, /'speech:status-changed'/);
  assert.match(speechApi, /export function copySpeechHistoryTranscript/);
  assert.match(speechApi, /IPC_COMMANDS\.copySpeechHistoryTranscript/);
});

test('speech history panel native and frontend contracts align', () => {
  for (const command of ['show_speech_history_panel', 'hide_speech_history_panel']) {
    assert.match(rustContracts, new RegExp(`"${command}"`));
    assert.match(tsCommands, new RegExp(`'${command}'`));
  }
  for (const event of ['speech-history-panel:open', 'speech-history-panel:closed']) {
    assert.match(rustContracts, new RegExp(`"${event}"`));
    assert.match(tsEvents, new RegExp(`'${event}'`));
  }
  assert.match(shellWindows, /SPEECH_HISTORY_PANEL_LABEL: &str = "speech-history-panel"/);
  assert.match(surfaces, /speechHistoryPanel: 'speech-history-panel'/);
  assert.match(surfaceLoader, /'speech-history-panel': \(\) => import\('\.\.\/components\/SpeechHistoryPanelSurface\.svelte'\)/);
  assert.match(mainSource, /speech_history_panel::show_speech_history_panel/);
  assert.match(mainSource, /speech_history_panel::hide_speech_history_panel/);
});

test('speech commands and managed runtime are registered in Tauri main', () => {
  assert.match(mainSource, /\.manage\(speech_runtime::SpeechRuntimeState::default\(\)\)/);
  assert.match(mainSource, /speech_runtime::start_speech_capture/);
  assert.match(mainSource, /speech_runtime::get_speech_history/);
  assert.match(mainSource, /speech_runtime::copy_speech_history_transcript/);
  assert.match(mainSource, /speech_runtime::stop_speech_capture/);
  assert.match(mainSource, /speech_runtime::shutdown/);
});

test('model-free fresh builds do not require ignored speech binaries as bundle resources', () => {
  const resources = tauriConfig.bundle?.resources ?? [];
  assert.doesNotMatch(JSON.stringify(resources), /speech-models|parakeet/);
});

test('planned speech cap finalization reason remains optional and cap-only', (t) => {
  if (!tsEvents.includes('finalizationReason') && !rustSpeech.includes('finalization_reason')) {
    t.skip('RED-ready acceptance: enable when local TDT fallback IPC field is implemented');
    return;
  }

  assert.match(tsEvents, /finalizationReason\?: 'recording_cap'/);
  assert.match(rustSpeech, /FinalizationReason/);
  assert.match(rustSpeech, /RecordingCap/);
  assert.match(rustSpeech, /#\[serde\(skip_serializing_if = "Option::is_none"\)\][\s\S]*finalization_reason/);
  assert.doesNotMatch(tsEvents, /finalizationReason\?:\s*'user_finish'/);
});

test('planned speech fallback keeps current IPC names and does not expose streaming claims', () => {
  assert.match(tsCommands, /startSpeechCapture: 'start_speech_capture'/);
  assert.match(tsCommands, /stopSpeechCapture: 'stop_speech_capture'/);
  assert.match(tsEvents, /speechStatusChanged: 'speech:status-changed'/);
  assert.match(tsEvents, /nonce: SpeechSessionNonce \| null/);
  assert.doesNotMatch(speechApi, /true stateful streaming|stateful streaming/i);
});
