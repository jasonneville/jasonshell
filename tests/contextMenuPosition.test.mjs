import assert from 'node:assert/strict';
import { test } from 'node:test';

import { positionContextMenuInViewport } from '../dist-tests/contextMenuPosition.js';

test('keeps a context menu at the requested point when it fits', () => {
  assert.deepEqual(
    positionContextMenuInViewport(
      { x: 40, y: 50 },
      { width: 120, height: 90 },
      { width: 400, height: 300 }
    ),
    { x: 40, y: 50 }
  );
});

test('flips and clamps a context menu into visible viewport space', () => {
  assert.deepEqual(
    positionContextMenuInViewport(
      { x: 390, y: 290 },
      { width: 120, height: 90 },
      { width: 400, height: 300 }
    ),
    { x: 270, y: 200 }
  );
});

test('uses padding when the menu is larger than the available viewport', () => {
  assert.deepEqual(
    positionContextMenuInViewport(
      { x: 5, y: 5 },
      { width: 500, height: 400 },
      { width: 300, height: 200 },
      10
    ),
    { x: 10, y: 10 }
  );
});
