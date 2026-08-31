import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  filterTaskGalleryItems,
  nextTaskGalleryFocusIndex,
  reconcileTaskGalleryFocus
} from '../dist-tests/features/bottom-bar/taskbarUxState.js';

function item(hwnd, title, processName = 'Code') {
  return { hwnd, title, processName };
}

test('gallery filtering matches title and process name while preserving order', () => {
  const state = filterTaskGalleryItems([
    item('1', 'Docs', 'Firefox'),
    item('2', 'Terminal', 'WindowsTerminal'),
    item('3', 'Design Doc', 'Code'),
    item('4', 'Mail', 'Firefox')
  ], 'doc');

  assert.deepEqual(state.items.map((entry) => entry.hwnd), ['1', '3']);
  assert.deepEqual(filterTaskGalleryItems(state.items, 'code').items.map((entry) => entry.hwnd), ['3']);
});

test('gallery keyboard focus handles arrows Home and End without wrapping', () => {
  assert.equal(nextTaskGalleryFocusIndex(0, 4, 'ArrowLeft'), 0);
  assert.equal(nextTaskGalleryFocusIndex(0, 4, 'ArrowRight'), 1);
  assert.equal(nextTaskGalleryFocusIndex(1, 4, 'ArrowDown'), 2);
  assert.equal(nextTaskGalleryFocusIndex(3, 4, 'ArrowRight'), 3);
  assert.equal(nextTaskGalleryFocusIndex(2, 4, 'Home'), 0);
  assert.equal(nextTaskGalleryFocusIndex(2, 4, 'End'), 3);
  assert.equal(nextTaskGalleryFocusIndex(0, 0, 'ArrowRight'), -1);
});

test('gallery focus reconciliation keeps focused HWND, removes stale HWND, and handles empty lists', () => {
  const items = [item('10', 'First'), item('20', 'Second'), item('30', 'Third')];

  assert.deepEqual(reconcileTaskGalleryFocus('20', items), { focusedHwnd: '20', focusedIndex: 1 });
  assert.deepEqual(reconcileTaskGalleryFocus('99', items), { focusedHwnd: '10', focusedIndex: 0 });
  assert.deepEqual(reconcileTaskGalleryFocus('20', []), { focusedHwnd: null, focusedIndex: -1 });
});
