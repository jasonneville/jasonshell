import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const mic = readFileSync(new URL('../src/components/TopBarMicControl.svelte', import.meta.url), 'utf8');
const panel = readFileSync(new URL('../src/components/SpeechHistoryPanelSurface.svelte', import.meta.url), 'utf8');
const loader = readFileSync(new URL('../src/lib/surfaceLoader.ts', import.meta.url), 'utf8');
const css = readFileSync(new URL('../src/components/SpeechHistoryPanelSurface.css', import.meta.url), 'utf8');

test('mic keeps primary capture action and opens history from context-menu equivalents', () => {
  assert.match(mic, /onClick=\{handleMicControl\}/);
  assert.match(mic, /ariaHaspopup="dialog"/);
  assert.match(mic, /onContextMenu=\{openSpeechHistory\}/);
  assert.match(mic, /onKeyDown=\{handleSpeechHistoryKeydown\}/);
  assert.match(mic, /event\.key === 'ContextMenu'/);
  assert.match(mic, /event\.shiftKey && event\.key === 'F10'/);
  assert.match(mic, /showSpeechHistoryPanel\(\{\s*anchorLeft: rect\.left,\s*anchorWidth: rect\.width\s*\}\)/);
});

test('speech history surface fetches only on open and closes on Escape or focus loss', () => {
  assert.match(panel, /listen\(SPEECH_HISTORY_PANEL_OPEN_EVENT/);
  assert.match(panel, /bind:this=\{dialogElement\}/);
  assert.match(panel, /if \(!disposed\) \{\s*dialogElement\?\.focus\(\);\s*void loadHistory\(\);\s*\}/);
  assert.match(panel, /getSpeechHistory\(\)/);
  assert.doesNotMatch(panel, /onMount\(\(\) => \{\s*void getSpeechHistory\(\)/);
  assert.match(panel, /event\.key === 'Escape'/);
  assert.match(panel, /window\.addEventListener\('blur', handleFocusLoss\)/);
  assert.match(panel, /hideSpeechHistoryPanel\(\)/);
  assert.match(panel, /disposed/);
});

test('speech history is a semantic list with controlled outcomes and native row copy actions', () => {
  assert.match(panel, /<h1[^>]*>Speech history<\/h1>/);
  assert.match(panel, /<ol[^>]*aria-label="Recent speech transcriptions"/);
  assert.match(panel, /No speech transcriptions this session\./);
  assert.match(panel, /Copied to clipboard/);
  assert.match(panel, /Not copied — clipboard unavailable/);
  assert.match(panel, /copySpeechHistoryTranscript\(\{ nonce: entry\.nonce \}\)/);
  assert.match(panel, /<button[^>]*type="button"[^>]*aria-label="Copy speech transcription"/);
  assert.match(panel, /disabled=\{copyingNonce === entry\.nonce\}/);
  assert.match(panel, /<img[^>]*src=\{contentCopyIconUrl\}[^>]*alt=""[^>]*aria-hidden="true"[^>]*draggable="false"/);
  assert.match(panel, /Copy failed\. Try again\./);
  assert.doesNotMatch(panel, /console\.|aria-live|localStorage|fetch\(|navigator\.clipboard/);
  assert.doesNotMatch(panel, /error\.(?:message|toString)|String\(error\)/);
});

test('copy settlement is row-scoped and stale-safe across close or reload', () => {
  assert.match(panel, /const sequence = openSequence;/);
  assert.match(panel, /if \(disposed \|\| sequence !== openSequence\) return;/);
  assert.match(panel, /copyingNonce = entry\.nonce/);
  assert.match(panel, /copyingNonce === entry\.nonce/);
  assert.match(css, /\.speech-history-row-main/);
  assert.match(css, /\.speech-history-content[\s\S]*-webkit-user-select: text;[\s\S]*user-select: text;/);
  assert.match(css, /\.speech-history-transcript[\s\S]*-webkit-user-select: text;[\s\S]*user-select: text;/);
  assert.match(css, /\.speech-history-copy[\s\S]*-webkit-user-select: none;[\s\S]*user-select: none;/);
  assert.doesNotMatch(panel, /on:(?:pointerdown|mousedown)[^=]*=.*preventDefault/);
  assert.match(css, /@media \(forced-colors: active\)[\s\S]*\.speech-history-copy/);
});

test('surface loader imports dedicated speech history UI', () => {
  assert.match(loader, /'speech-history-panel': \(\) => import\('\.\.\/components\/SpeechHistoryPanelSurface\.svelte'\)/);
});
