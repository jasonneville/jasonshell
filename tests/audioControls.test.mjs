import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  AUDIO_COMMANDS,
  normalizeVolumePercent
} from '../dist-tests/lib/audio.js';

const audioWrapper = readFileSync(new URL('../src/lib/audio.ts', import.meta.url), 'utf8');
const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const topBarCss = readFileSync(new URL('../src/components/TopBar.css', import.meta.url), 'utf8');
const audioPanelSource = readFileSync(new URL('../src/components/AudioPanelSurface.svelte', import.meta.url), 'utf8');
const appSource = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
const shellSurfaceSource = readFileSync(new URL('../src/lib/shellSurface.ts', import.meta.url), 'utf8');
const shellWindowsSource = readFileSync(new URL('../src-tauri/src/shell_windows.rs', import.meta.url), 'utf8');
const mainRustSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const audioPanelRustSource = readFileSync(new URL('../src-tauri/src/audio_panel.rs', import.meta.url), 'utf8');

test('audio wrapper exposes stable command constants and clamps slider values', () => {
  assert.deepEqual(AUDIO_COMMANDS, {
    showPanel: 'show_audio_panel',
    hidePanel: 'hide_audio_panel',
    getState: 'get_audio_state',
    setMasterVolume: 'set_master_volume',
    setAppVolume: 'set_app_volume',
    setDefaultInputDevice: 'set_default_audio_input_device',
    setDefaultOutputDevice: 'set_default_audio_output_device'
  });

  assert.equal(normalizeVolumePercent(-5), 0);
  assert.equal(normalizeVolumePercent(47.4), 47);
  assert.equal(normalizeVolumePercent(1000), 100);
  assert.equal(normalizeVolumePercent(Number.NaN), 0);
  assert.doesNotMatch(audioWrapper, /invoke\('[-_a-z]+/);
});

test('top bar adds sound control left of time with immediate audio command calls', () => {
  assert.match(topBarSource, /from '\.\.\/lib\/audio'/);
  assert.match(topBarSource, /class="sound-control"[\s\S]*class="time-pill"/);
  assert.match(topBarSource, /class="sound-button"[\s\S]*ariaControls=\{SOUND_PANEL_ID\}/);
  assert.match(topBarSource, /ariaHaspopup="dialog"/);
  assert.match(topBarSource, /showAudioPanel\(\{/);
  assert.match(topBarSource, /hideAudioPanel\(\)/);
  assert.match(topBarSource, /AUDIO_PANEL_CLOSED_EVENT/);
  assert.doesNotMatch(topBarSource, /class="sound-menu"/);
  assert.doesNotMatch(topBarSource, /role="menu"/);
  assert.doesNotMatch(topBarCss, /\.top-bar \.sound-menu \{/);
  assert.doesNotMatch(topBarCss, /linear-gradient|radial-gradient|conic-gradient/);
});

test('audio panel surface owns usable dialog controls and immediate audio commands', () => {
  assert.match(appSource, /loadSurfaceComponent\(surface\)/);
  assert.match(appSource, /<SurfaceComponent \/>/);
  assert.match(shellSurfaceSource, /\| 'audio-panel'/);
  assert.match(audioPanelSource, /id="audio-panel" role="dialog"/);
  assert.match(audioPanelSource, /AUDIO_PANEL_OPEN_EVENT/);
  assert.match(audioPanelSource, /on:input=\{handleMasterVolumeInput\}/);
  assert.match(audioPanelSource, /commitAudioCommand\(setMasterVolume\(volumePercent\)\)/);
  assert.match(audioPanelSource, /on:input=\{\(event\) => handleSessionVolumeInput\(session\.id, event\)\}/);
  assert.match(audioPanelSource, /commitAudioCommand\(setAppVolume\(sessionId, volumePercent\)\)/);
  assert.match(audioPanelSource, /on:change=\{handleOutputDeviceChange\}/);
  assert.match(audioPanelSource, /setDefaultAudioOutputDevice\(deviceId\)/);
  assert.match(audioPanelSource, /on:change=\{handleInputDeviceChange\}/);
  assert.match(audioPanelSource, /setDefaultAudioInputDevice\(deviceId\)/);
  assert.doesNotMatch(audioPanelSource, /role="menu"/);
});

test('audio panel ignores stale failed commands and reverts current failures', () => {
  assert.match(
    audioPanelSource,
    /\.catch\(\(error\) => \{\s*if \(sequence !== audioCommandSequence\) \{\s*return;\s*\}[\s\S]*audioError = 'Audio update failed';/
  );
  assert.match(audioPanelSource, /getAudioState\(\)\.then\(\(nextAudioState\) => \{/);
  assert.match(audioPanelSource, /applyAudioState\(nextAudioState, \{ clearError: false \}\)/);
  assert.doesNotMatch(
    audioPanelSource,
    /\.catch\(\(error\) => \{\s*console\.error\('Failed to update audio state'/
  );
});

test('audio panel is a dedicated top-bar anchored webview so controls are not clipped by the compact top bar', () => {
  assert.match(shellWindowsSource, /pub const AUDIO_PANEL_LABEL: &str = "audio-panel"/);
  assert.match(shellWindowsSource, /AUDIO_PANEL_HEIGHT_LOGICAL: f64 = 430\.0/);
  assert.match(shellWindowsSource, /build_audio_panel_window\(app\)/);
  assert.match(mainRustSource, /mod audio_panel;/);
  assert.match(mainRustSource, /audio_panel::show_audio_panel/);
  assert.match(mainRustSource, /audio_panel::hide_audio_panel/);
  assert.match(mainRustSource, /shell_windows::AUDIO_PANEL_LABEL[\s\S]*WindowEvent::Focused\(false\)/);
  assert.match(audioPanelRustSource, /pub fn show_audio_panel/);
  assert.match(audioPanelRustSource, /TOP_BAR_LABEL/);
  assert.match(audioPanelRustSource, /emit_to\(TOP_BAR_LABEL, AUDIO_PANEL_CLOSED_EVENT/);
});
