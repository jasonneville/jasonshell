import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  clearTaskGalleryClickSuppressionOnPointerDown,
  pendingTaskGalleryPointer,
  pendingTaskbarTilePointer,
  resolveTaskGalleryPointerRelease,
  resolveTaskbarTilePointerRelease,
  shouldSuppressTaskbarTileClick,
  shouldSuppressTaskGalleryClick
} from '../dist-tests/lib/taskbarTilePointer.js';

test('promotes rapid capsule enter and primary release into immediate gallery open', () => {
  const pendingGroupKey = pendingTaskGalleryPointer(0, 'group:editor', true);

  assert.deepEqual(resolveTaskGalleryPointerRelease(pendingGroupKey, false), {
    openGroupKey: 'group:editor',
    suppressClickGroupKey: 'group:editor'
  });
});

test('capsule pointer release preserves drag suppression without opening gallery', () => {
  const pendingGroupKey = pendingTaskGalleryPointer(0, 'group:editor', true);

  assert.deepEqual(resolveTaskGalleryPointerRelease(pendingGroupKey, true), {
    openGroupKey: null,
    suppressClickGroupKey: 'group:editor'
  });
  assert.equal(pendingTaskGalleryPointer(2, 'group:editor', true), null);
  assert.equal(pendingTaskGalleryPointer(0, 'group:editor', false), null);
});

test('next primary capsule pointerdown clears stale gallery click suppression', () => {
  assert.equal(
    clearTaskGalleryClickSuppressionOnPointerDown('group:editor', 0, true),
    null
  );
  assert.equal(
    clearTaskGalleryClickSuppressionOnPointerDown('group:editor', 2, true),
    'group:editor'
  );
  assert.equal(
    clearTaskGalleryClickSuppressionOnPointerDown('group:editor', 0, false),
    'group:editor'
  );
});

test('stale gallery suppression never swallows keyboard activation', () => {
  assert.equal(shouldSuppressTaskGalleryClick('group:editor', 'group:editor', 0), false);
  assert.equal(shouldSuppressTaskGalleryClick('group:editor', 'group:editor', 1), true);
  assert.equal(shouldSuppressTaskGalleryClick('group:editor', 'group:terminal', 1), false);
});

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
