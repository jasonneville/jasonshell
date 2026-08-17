import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  nextTaskbarFocusIndex,
  taskbarOverflowState,
  taskGroupStateLabel
} from '../dist-tests/features/bottom-bar/taskbarUxState.js';

const bottomBarCss = readFileSync(new URL('../src/components/BottomBar.css', import.meta.url), 'utf8');
const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');

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
    isBusy: true,
    notificationCount: 4
  });

  assert.equal(label, 'Code, 2 windows, 4 notifications, active, activity detected');
});

test('renders notified task groups with red badge and top border', () => {
  assert.match(bottomBarSource, /class:task-group-notified=\{group\.notificationCount > 0\}/);
  assert.match(bottomBarSource, /aria-label=\{`\$\{group\.notificationCount\} notifications`\}/);
  assert.match(bottomBarCss, /\.bottom-bar \.task-group-notified \{[\s\S]*box-shadow: inset 0 2px 0 var\(--js-color-error-border\);/);
  assert.match(bottomBarCss, /\.bottom-bar \.task-count \{[\s\S]*background: var\(--js-color-error\);[\s\S]*color: var\(--js-color-error-text\);/);
});

test('sizes task buttons as equal flex items without content-sized bounds', () => {
  assert.match(bottomBarSource, /style=\{taskGroupStyle\(group\)\}/);
  assert.match(bottomBarSource, /--task-window-count:\s*\$\{Math\.max\(group\.windows\.length, 1\)\};/);

  const taskGroupRule = bottomBarCss.match(/\.bottom-bar \.task-group \{[\s\S]*?\n\}/)?.[0] ?? '';
  const taskButtonRule = bottomBarCss.match(/\.bottom-bar \.task-button \{[\s\S]*?\n\}/)?.[0] ?? '';
  const taskLabelRule = bottomBarCss.match(/\.bottom-bar \.task-label \{[\s\S]*?\n\}/)?.[0] ?? '';

  assert.match(taskGroupRule, /flex:\s*var\(--task-window-count, 1\) 1 0;/);
  assert.match(taskGroupRule, /max-width:\s*calc\(10rem \* var\(--task-window-count, 1\)\);/);
  assert.match(taskButtonRule, /flex:\s*1 1 0;/);
  assert.match(taskButtonRule, /min-width:\s*0;/);
  assert.doesNotMatch(taskButtonRule, /min-width:\s*6\.2rem;/);
  assert.doesNotMatch(taskButtonRule, /max-width:\s*14rem;/);
  assert.match(taskLabelRule, /text-overflow:\s*ellipsis;/);
  assert.match(taskLabelRule, /min-width:\s*0;/);
});
