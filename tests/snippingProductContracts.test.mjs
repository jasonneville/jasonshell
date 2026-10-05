import assert from 'node:assert/strict';
import { test } from 'node:test';
import { resolveSurfaceFromLabel } from '../dist-tests/lib/shellSurface.js';
import { DEFAULT_TOP_BAR_CONTROL_ORDER, normalizeTopBarControlOrder, TOP_BAR_CONTROL_ORDER_STORAGE_KEY } from '../dist-tests/lib/topBarControls.js';
import { defaultShellSettings } from '../dist-tests/lib/settings.js';

test('snipping resolves overlay and preview as native routed surfaces', () => {
  assert.equal(resolveSurfaceFromLabel('snip-overlay'), 'snip-overlay');
  assert.equal(resolveSurfaceFromLabel('snip-preview'), 'snip-preview');
  assert.equal(resolveSurfaceFromLabel('snip-overlay-forged'), 'unknown');
});

test('snipping routes canonical generation-local labels only, bounded to u64 and m0..m31', () => {
  for (const label of ['snip-overlay-7-m0', 'snip-overlay-18446744073709551615-m31'])
    assert.equal(resolveSurfaceFromLabel(label), 'snip-overlay');
  assert.equal(resolveSurfaceFromLabel('snip-preview-18446744073709551615'), 'snip-preview');
  for (const label of ['snip-overlay-0-m0', 'snip-overlay-01-m0', 'snip-overlay-7-m00',
    'snip-overlay-7-m32', 'snip-overlay-18446744073709551616-m0', 'snip-preview-0',
    'snip-preview-01', 'snip-preview-7-forged', 'snip-overlay-7-m0-forged',
    'snip-overlay-７-m0', 'snip-preview-' + '9'.repeat(97)])
    assert.equal(resolveSurfaceFromLabel(label), 'unknown', label);
});

test('snipping appends after sound by default without changing persisted v1 order key', () => {
  assert.deepEqual(DEFAULT_TOP_BAR_CONTROL_ORDER, ['terminal', 'command', 'tray', 'mic', 'sound', 'snip']);
  assert.equal(TOP_BAR_CONTROL_ORDER_STORAGE_KEY, 'jasonshell:top-bar:control-order:v1');
});

test('snipping migrates reordered legacy controls and duplicate snip exactly once', () => {
  assert.deepEqual(normalizeTopBarControlOrder(JSON.stringify(['sound', 'mic', 'tray', 'command', 'terminal'])),
    ['sound', 'mic', 'tray', 'command', 'terminal', 'snip']);
  assert.deepEqual(normalizeTopBarControlOrder(JSON.stringify(['snip', 'snip', 'sound', 'unknown'])),
    ['snip', 'sound', 'terminal', 'command', 'tray', 'mic']);
});

test('snipping adds Alt+S while preserving all four existing default chords', () => {
  assert.deepEqual(defaultShellSettings().hotkeys, {
    search: 'Ctrl+Space', terminal: 'Alt+Backquote', stackBrowser: 'Alt+1',
    speechTranscription: 'Ctrl+D', snipping: 'Alt+S'
  });
});
