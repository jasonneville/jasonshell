import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const audioWrapper = readFileSync(new URL('../src/lib/audio.ts', import.meta.url), 'utf8');
const audioPanelSource = readFileSync(new URL('../src/components/AudioPanelSurface.svelte', import.meta.url), 'utf8');
const ipcEventsSource = readFileSync(new URL('../src/ipc/events.ts', import.meta.url), 'utf8');

test('audio refresh has typed reasons without dead cross-window event contract', () => {
  assert.doesNotMatch(ipcEventsSource, /audioRefresh: 'audio:refresh'/);
  assert.doesNotMatch(audioWrapper, /AUDIO_REFRESH_EVENT|AudioRefreshPayload|audio:refresh/);
  assert.doesNotMatch(audioPanelSource, /AUDIO_REFRESH_EVENT|AudioRefreshPayload|audio:refresh/);
  assert.match(audioWrapper, /export type AudioRefreshReason =\s*\| 'device-added'\s*\| 'device-removed'\s*\| 'default-changed'\s*\| 'session-changed'/);
});

test('audio panel does not subscribe to removed refresh event contract', () => {
  assert.doesNotMatch(audioPanelSource, /AUDIO_REFRESH_EVENT/);
  assert.doesNotMatch(audioPanelSource, /type AudioRefreshPayload/);
  assert.match(audioPanelSource, /const unlisteners: Array<\(\) => void> = \[\]/);
  assert.match(audioPanelSource, /let disposed = false/);
  assert.doesNotMatch(audioPanelSource, /registerAsyncUnlistener\(listen<AudioRefreshPayload>\(AUDIO_REFRESH_EVENT/);
  assert.match(audioPanelSource, /if \(disposed\) \{\s*unlisten\(\);\s*return;\s*\}/);
  assert.match(audioPanelSource, /unlisteners\.push\(unlisten\)/);
});

test('audio panel debounces event bursts while keeping manual refresh button', () => {
  assert.match(audioPanelSource, /const AUDIO_REFRESH_DEBOUNCE_MS = (?:1\d\d|2[0-4]\d|250)/);
  assert.match(audioPanelSource, /let audioRefreshTimer: ReturnType<typeof setTimeout> \| null = null/);
  assert.match(audioPanelSource, /function scheduleAudioRefresh\(reason: AudioRefreshReason\)/);
  assert.match(audioPanelSource, /clearTimeout\(audioRefreshTimer\)/);
  assert.match(audioPanelSource, /audioRefreshTimer = setTimeout\(\(\) => \{[\s\S]*void refreshAudioState\(\{ reason \}\)/);
  assert.match(audioPanelSource, /ariaLabel="Refresh sound devices"/);
  assert.match(audioPanelSource, /onClick=\{\(\) => void refreshAudioState\(\)\}/);
});

test('audio panel keeps a bounded polling fallback only while visible', () => {
  const initialMountBody = audioPanelSource.match(/onMount\(\(\) => \{[\s\S]*?void listen\(AUDIO_PANEL_OPEN_EVENT/)?.[0] ?? '';

  assert.match(audioPanelSource, /const AUDIO_REFRESH_POLL_MS = \d+/);
  assert.match(audioPanelSource, /let audioRefreshPollTimer: ReturnType<typeof setInterval> \| null = null/);
  assert.match(audioPanelSource, /let audioPanelVisible = false/);
  assert.match(audioPanelSource, /function startAudioRefreshPolling\(\)/);
  assert.match(audioPanelSource, /function stopAudioRefreshPolling\(\)/);
  assert.match(audioPanelSource, /audioRefreshPollTimer = setInterval\(\(\) => \{[\s\S]*void refreshAudioState\(\{ reason: 'session-changed' \}\)/);
  assert.match(audioPanelSource, /audioPanelVisible = true;[\s\S]*startAudioRefreshPolling\(\)/);
  assert.match(audioPanelSource, /audioPanelVisible = false;[\s\S]*stopAudioRefreshPolling\(\)/);
  assert.doesNotMatch(initialMountBody, /audioPanelVisible = true/);
  assert.doesNotMatch(initialMountBody, /startAudioRefreshPolling\(\)/);
  assert.doesNotMatch(initialMountBody, /void refreshAudioState\(\)/);
});
