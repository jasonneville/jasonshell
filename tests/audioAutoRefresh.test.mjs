import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const audioWrapper = readFileSync(new URL('../src/lib/audio.ts', import.meta.url), 'utf8');
const audioPanelSource = readFileSync(new URL('../src/components/AudioPanelSurface.svelte', import.meta.url), 'utf8');
const ipcEventsSource = readFileSync(new URL('../src/ipc/events.ts', import.meta.url), 'utf8');

test('audio refresh event has typed reasons and centralized event name', () => {
  assert.match(ipcEventsSource, /audioRefresh: 'audio:refresh'/);
  assert.match(audioWrapper, /export const AUDIO_REFRESH_EVENT = IPC_EVENTS\.audioRefresh/);
  assert.match(audioWrapper, /export type AudioRefreshReason =\s*\| 'device-added'\s*\| 'device-removed'\s*\| 'default-changed'\s*\| 'session-changed'/);
  assert.match(audioWrapper, /export type AudioRefreshPayload = \{\s*reason: AudioRefreshReason;\s*\}/);
});

test('audio panel subscribes to refresh events on mount and unsubscribes on destroy', () => {
  assert.match(audioPanelSource, /AUDIO_REFRESH_EVENT/);
  assert.match(audioPanelSource, /type AudioRefreshPayload/);
  assert.match(audioPanelSource, /let unlistenRefresh: \(\(\) => void\) \| null = null/);
  assert.match(audioPanelSource, /listen<AudioRefreshPayload>\(AUDIO_REFRESH_EVENT, \(event\) => \{/);
  assert.match(audioPanelSource, /scheduleAudioRefresh\(event\.payload\.reason\)/);
  assert.match(audioPanelSource, /unlistenRefresh = unlisten/);
  assert.match(audioPanelSource, /unlistenRefresh\?\.\(\)/);
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
