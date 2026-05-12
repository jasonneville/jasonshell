import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  nextTaskbarFocusIndex,
  taskbarOverflowState,
  taskGroupStateLabel
} from '../dist-tests/features/bottom-bar/taskbarUxState.js';

const bottomBarCss = readFileSync(new URL('../src/components/BottomBar.css', import.meta.url), 'utf8');

test('detects taskbar overflow and exposes keyboard guidance', () => {
  assert.deepEqual(taskbarOverflowState(320, 500, 9), {
    hasOverflow: true,
    summary: '9 task groups, use arrow keys to move through hidden items'
  });
  assert.deepEqual(taskbarOverflowState(500, 500, 3), {
    hasOverflow: false,
    summary: '3 task groups visible'
  });
});

test('moves keyboard focus across taskbar items without wrapping unexpectedly', () => {
  assert.equal(nextTaskbarFocusIndex(0, 4, 'ArrowLeft'), 0);
  assert.equal(nextTaskbarFocusIndex(0, 4, 'ArrowRight'), 1);
  assert.equal(nextTaskbarFocusIndex(2, 4, 'End'), 3);
  assert.equal(nextTaskbarFocusIndex(2, 4, 'Home'), 0);
  assert.equal(nextTaskbarFocusIndex(0, 0, 'ArrowRight'), -1);
});

test('summarizes task group state for stronger accessible indicators', () => {
  const label = taskGroupStateLabel({
    key: 'code',
    label: 'Code',
    iconDataUrl: '',
    windows: [{ hwnd: '1' }, { hwnd: '2' }],
    isActive: true,
    isMinimized: false,
    isBusy: true
  });

  assert.equal(label, 'Code, 2 windows, active, activity detected');
});

test('sizes task buttons by content with minimum and maximum bounds', () => {
  const taskButtonRule = bottomBarCss.match(/\.bottom-bar \.task-button \{[\s\S]*?\n\}/)?.[0] ?? '';
  const taskLabelRule = bottomBarCss.match(/\.bottom-bar \.task-label \{[\s\S]*?\n\}/)?.[0] ?? '';

  assert.match(taskButtonRule, /flex:\s*0 1 auto;/);
  assert.match(taskButtonRule, /min-width:\s*6\.2rem;/);
  assert.match(taskButtonRule, /max-width:\s*14rem;/);
  assert.match(taskLabelRule, /text-overflow:\s*ellipsis;/);
  assert.match(taskLabelRule, /min-width:\s*0;/);
});
