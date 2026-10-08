import assert from 'node:assert/strict';
import { test } from 'node:test';
import { taskGalleryLayout } from '../dist-tests/lib/taskGalleryLayout.js';

// Fixture/legacy helper must match native serialized geometry; production events
// remain native-authoritative. Native counterpart: task_gallery_layout.rs.
test('2/3 Ribbon geometry reserves image and independent 32px footer', () => {
  for (const [count, panelWidth] of [[2, 384], [3, 572]]) {
    assert.deepEqual(taskGalleryLayout(count, 1920, 1000), {
      mode: 'ribbon', columns: count, cardWidth: 180, cardHeight: 136,
      thumbnailWidth: 172, thumbnailHeight: 96, footerHeight: 32,
      viewportWidth: panelWidth - 16, viewportHeight: 136, panelWidth, panelHeight: 184
    });
  }
});
test('4/6/30 Matrix bounded three columns/two visible rows with reserved scrollbar', () => {
  for (const count of [4, 6, 30]) assert.deepEqual(taskGalleryLayout(count, 1920, 1000), {
    mode: 'matrix', columns: 3, cardWidth: 180, cardHeight: 136,
    thumbnailWidth: 172, thumbnailHeight: 96, footerHeight: 32,
    viewportWidth: 568, viewportHeight: 280, panelWidth: 584, panelHeight: 328
  });
});
test('Ribbon exact-fit boundaries and narrow Matrix wrapping', () => {
  for (const [count, width, mode, columns, panelWidth] of [
    [2, 384, 'ribbon', 2, 384], [2, 383, 'matrix', 1, 208],
    [3, 572, 'ribbon', 3, 572], [3, 571, 'matrix', 2, 396],
    [30, 584, 'matrix', 3, 584], [30, 583, 'matrix', 2, 396],
    [30, 396, 'matrix', 2, 396], [30, 395, 'matrix', 1, 208],
    [30, 208, 'matrix', 1, 208], [30, 144, 'matrix', 1, 144]
  ]) {
    const actual = taskGalleryLayout(count, width, 1000);
    assert.equal(actual.mode, mode); assert.equal(actual.columns, columns);
    assert.equal(actual.panelWidth, panelWidth);
    assert.ok(actual.panelWidth <= width);
  }
});
test('undersized monitor reduces one card and viewport, never thumbnail frame overflow', () => {
  const actual = taskGalleryLayout(30, 144, 220);
  assert.equal(actual.cardWidth, 116); assert.equal(actual.thumbnailWidth, 108);
  assert.equal(actual.panelHeight, 220); assert.equal(actual.viewportHeight, 172);
});
