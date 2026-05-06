import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function readSource(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

function extractFunction(source, name) {
  const start = source.indexOf(`fn ${name}`);
  const pubStart = source.indexOf(`pub fn ${name}`);
  const functionStart = pubStart >= 0 ? pubStart : start;
  assert.notEqual(functionStart, -1, `${name} should exist`);
  const bodyStart = source.indexOf('{', functionStart);
  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0) return source.slice(functionStart, index + 1);
  }
  throw new Error(`could not extract ${name}`);
}

test('search close helpers target top-bar for native and explicit close reset', () => {
  const main = readSource('../src-tauri/src/main.rs');
  const searchPanel = readSource('../src-tauri/src/search_panel.rs');
  const nativeFocusLoss = main.match(/window\.label\(\) == shell_windows::SEARCH_PANEL_LABEL[\s\S]*?WindowEvent::Focused\(false\)[\s\S]*?return;/)?.[0] ?? '';
  const hideSearchPanel = extractFunction(searchPanel, 'hide_search_panel');

  assert.match(searchPanel, /pub fn emit_search_panel_closed_to_top_bar/);
  assert.match(nativeFocusLoss, /search_panel::emit_search_panel_closed_to_top_bar\(window\.app_handle\(\)\)/);
  assert.doesNotMatch(nativeFocusLoss, /window\.emit\(search_panel::SEARCH_PANEL_CLOSED_EVENT/);
  assert.match(hideSearchPanel, /emit_search_panel_closed_to_top_bar\(&app_handle\)/);
});

test('audio close event is delivered to both top-bar and audio-panel owners', () => {
  const main = readSource('../src-tauri/src/main.rs');
  const audioPanel = readSource('../src-tauri/src/audio_panel.rs');
  const audioFocusLoss = main.match(/window\.label\(\) == shell_windows::AUDIO_PANEL_LABEL[\s\S]*?WindowEvent::Focused\(false\)[\s\S]*?return;/)?.[0] ?? '';
  const hideAudioPanel = extractFunction(audioPanel, 'hide_audio_panel');

  assert.match(audioPanel, /pub fn emit_audio_panel_closed/);
  assert.match(audioFocusLoss, /audio_panel::emit_audio_panel_closed\(window\.app_handle\(\)\)/);
  assert.match(hideAudioPanel, /emit_audio_panel_closed\(&app_handle\)/);
  assert.match(audioPanel, /emit_to\(TOP_BAR_LABEL,\s*AUDIO_PANEL_CLOSED_EVENT/);
  assert.match(audioPanel, /emit_to\(AUDIO_PANEL_LABEL,\s*AUDIO_PANEL_CLOSED_EVENT/);
});

test('audio panel surface listens for own close event and stops polling', () => {
  const audioSurface = readSource('../src/components/AudioPanelSurface.svelte');
  const audioLib = readSource('../src/lib/audio.ts');
  const initialMountBody = audioSurface.match(/onMount\(\(\) => \{[\s\S]*?void listen\(AUDIO_PANEL_OPEN_EVENT/)?.[0] ?? '';

  assert.match(audioLib, /AUDIO_PANEL_CLOSED_EVENT = 'audio-panel:closed'/);
  assert.match(audioSurface, /AUDIO_PANEL_CLOSED_EVENT/);
  assert.match(audioSurface, /listen\(AUDIO_PANEL_CLOSED_EVENT, \(\) => \{/);
  assert.match(audioSurface, /audioPanelVisible = false;[\s\S]*stopAudioRefreshPolling\(\)/);
  assert.doesNotMatch(initialMountBody, /audioPanelVisible = true/);
  assert.doesNotMatch(initialMountBody, /startAudioRefreshPolling\(\)/);
  assert.doesNotMatch(initialMountBody, /void refreshAudioState\(\)/);
});

test('tray open event targets tray-panel and reloads icons on every show', () => {
  const trayPanel = readSource('../src-tauri/src/tray_panel.rs');
  const traySurface = readSource('../src/components/TrayPanelSurface.svelte');
  const trayLib = readSource('../src/lib/trayPanel.ts');
  const contracts = readSource('../src-tauri/src/contracts.rs');

  assert.match(trayPanel, /pub const TRAY_PANEL_OPEN_EVENT: &str = "tray-panel:open";/);
  assert.match(trayLib, /TRAY_PANEL_OPEN_EVENT = 'tray-panel:open'/);
  assert.match(contracts, /TRAY_PANEL_OPEN/);
  assert.match(trayPanel, /emit_to\(TRAY_PANEL_LABEL,\s*TRAY_PANEL_OPEN_EVENT/);
  assert.match(traySurface, /listen\(TRAY_PANEL_OPEN_EVENT, \(\) => \{[\s\S]*void loadTrayIcons\(\)/);
  assert.doesNotMatch(traySurface, /onMount\(\(\) => \{[\s\S]*void loadTrayIcons\(\);[\s\S]*listen\(TRAY_PANEL_OPEN_EVENT/);
});
