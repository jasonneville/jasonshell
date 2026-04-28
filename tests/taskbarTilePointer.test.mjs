import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  pendingTaskbarTilePointer,
  resolveTaskbarTilePointerRelease,
  shouldSuppressTaskbarTileClick
} from '../dist-tests/lib/taskbarTilePointer.js';

test('tracks pending taskbar tile activation only for the primary button', () => {
  assert.equal(pendingTaskbarTilePointer(0, '101'), '101');
  assert.equal(pendingTaskbarTilePointer(2, '101'), null);
});

test('promotes a non-drag pointer release into taskbar tile activation', () => {
  assert.deepEqual(resolveTaskbarTilePointerRelease('101', false), {
    activateHwnd: '101',
    suppressClickHwnd: '101'
  });
});

test('suppresses post-drag taskbar tile clicks without activating the window', () => {
  assert.deepEqual(resolveTaskbarTilePointerRelease('101', true), {
    activateHwnd: null,
    suppressClickHwnd: '101'
  });
});

test('suppresses only the matching taskbar tile click after pointer release handling', () => {
  assert.equal(shouldSuppressTaskbarTileClick('101', '101'), true);
  assert.equal(shouldSuppressTaskbarTileClick('101', '202'), false);
});
