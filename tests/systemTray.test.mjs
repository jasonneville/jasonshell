import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  EMPTY_TRAY_ICON_DATA_URL,
  normalizeTrayIcons,
  trayClickRequest
} from '../dist-tests/lib/systemTray.js';

test('normalizes tray icons with stable labels and image fallback', () => {
  assert.deepEqual(normalizeTrayIcons([
    { id: '100:1', commandId: 1, index: 0, label: '  ', iconDataUrl: '' },
    { id: '100:2', commandId: 2, index: 1, label: 'Network', iconDataUrl: 'data:image/png;base64,abc', hasNativeIcon: true }
  ]), [
    { id: '100:1', commandId: 1, index: 0, label: 'Notification area icon 1', iconDataUrl: EMPTY_TRAY_ICON_DATA_URL, hasNativeIcon: false },
    { id: '100:2', commandId: 2, index: 1, label: 'Network', iconDataUrl: 'data:image/png;base64,abc', hasNativeIcon: true }
  ]);
});

test('treats placeholder-only tray snapshots as fallback glyphs', () => {
  assert.deepEqual(normalizeTrayIcons([
    { id: '100:1', commandId: 1, index: 0, label: 'Volume', iconDataUrl: EMPTY_TRAY_ICON_DATA_URL, hasNativeIcon: false },
    { id: '100:2', commandId: 2, index: 1, label: 'Broken native flag', iconDataUrl: '', hasNativeIcon: true }
  ]), [
    { id: '100:1', commandId: 1, index: 0, label: 'Volume', iconDataUrl: EMPTY_TRAY_ICON_DATA_URL, hasNativeIcon: false },
    { id: '100:2', commandId: 2, index: 1, label: 'Broken native flag', iconDataUrl: EMPTY_TRAY_ICON_DATA_URL, hasNativeIcon: false }
  ]);
});

test('drops duplicate tray ids from snapshots', () => {
  assert.deepEqual(normalizeTrayIcons([
    { id: '100:1', commandId: 1, index: 0, label: 'One', iconDataUrl: 'a' },
    { id: '100:1', commandId: 1, index: 1, label: 'Duplicate', iconDataUrl: 'b' }
  ]), [
    { id: '100:1', commandId: 1, index: 0, label: 'One', iconDataUrl: 'a', hasNativeIcon: false }
  ]);
});

test('preserves app and overflow tray entries when source-qualified ids differ', () => {
  assert.deepEqual(normalizeTrayIcons([
    { id: 'tray-notify:100:7', commandId: 7, index: 0, label: 'Steam', iconDataUrl: 'data:image/png;base64,visible', hasNativeIcon: true },
    { id: 'overflow:200:7', commandId: 7, index: 0, label: 'Steam', iconDataUrl: 'data:image/png;base64,overflow', hasNativeIcon: true },
    { id: 'tray-notify:100:8', commandId: 8, index: 1, label: 'Volume', iconDataUrl: 'data:image/png;base64,volume', hasNativeIcon: true }
  ]), [
    { id: 'tray-notify:100:7', commandId: 7, index: 0, label: 'Steam', iconDataUrl: 'data:image/png;base64,visible', hasNativeIcon: true },
    { id: 'overflow:200:7', commandId: 7, index: 0, label: 'Steam', iconDataUrl: 'data:image/png;base64,overflow', hasNativeIcon: true },
    { id: 'tray-notify:100:8', commandId: 8, index: 1, label: 'Volume', iconDataUrl: 'data:image/png;base64,volume', hasNativeIcon: true }
  ]);
});

test('serializes tray click request payloads for backend relay', () => {
  assert.deepEqual(trayClickRequest('100:1', 'left'), { id: '100:1', button: 'left' });
  assert.deepEqual(trayClickRequest('100:1', 'right'), { id: '100:1', button: 'right' });
});
