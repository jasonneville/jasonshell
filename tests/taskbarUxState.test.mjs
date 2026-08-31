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

function cssRule(source, selector) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = source.match(new RegExp(`${escapedSelector}\\s*\\{([\\s\\S]*?)\\n\\}`));
  assert.ok(match, `${selector} rule exists`);
  return match[1];
}

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
    hasAttention: true,
    toastCount: 4
  });

  assert.equal(label, 'Code, 2 windows, active, activity detected');
});

test('summarizes inactive attention and toast state with combined labels', () => {
  const label = taskGroupStateLabel({
    key: 'code',
    label: 'Code',
    iconDataUrl: '',
    windows: [{ hwnd: '1' }, { hwnd: '2' }],
    isActive: false,
    isMinimized: true,
    isBusy: true,
    hasAttention: true,
    toastCount: 4
  });

  assert.equal(label, 'Code, 2 windows, 4 toasts, needs attention, activity detected, minimized');
});

test('renders attentive task groups with toast badge, amber cue, and active suppression', () => {
  assert.match(bottomBarSource, /taskWindowHasVisibleAttention\(taskWindow\) \? ' task-window-attention' : ''/);
  assert.match(bottomBarSource, /return taskWindow\.attentionState === 'requested' && !taskWindow\.isActive;/);
  assert.match(bottomBarSource, /class:task-group-toasted=\{taskGroupHasToast\(group\)\}/);
  assert.match(bottomBarSource, /ariaLabel=\{taskGroupLabel\(group\)\}/);
  assert.match(bottomBarCss, /\.bottom-bar \.task-window-attention,\s*\.bottom-bar \.task-group-toasted \{[\s\S]*box-shadow: inset 0 2px 0 var\(--js-color-warning-border\);/);
  const attentionShadow = cssRule(bottomBarCss, '.bottom-bar .task-button.task-window-attention')
    .match(/box-shadow:\s*([^;]+);/)?.[1] ?? '';
  assert.match(attentionShadow, /\binset\b/, 'attention cue stays inside task button bounds');
  assert.ok(Number(attentionShadow.match(/inset\s+\S+\s+([\d.]+)px/)?.[1]) > 0, 'attention cue has visible thickness');
  assert.match(attentionShadow, /(?:var\(--[\w-]+\)|#[\da-f]{3,8}|rgba?\()/i, 'attention cue has a visible color');
  assert.doesNotMatch(bottomBarCss, /taskbar-attention-flash/);
  assert.match(bottomBarCss, /\.bottom-bar \.task-count \{[\s\S]*background: var\(--js-color-warning\);[\s\S]*color: var\(--js-color-text-strong\);/);
  assert.match(bottomBarCss, /\.bottom-bar \.task-group-toasted \.task-count \{[\s\S]*box-shadow: 0 0 0 1px var\(--js-color-warning-border\), 0 0 0\.55rem rgba\(245, 191, 92, 0\.28\);/);
  assert.match(bottomBarCss, /@media \(prefers-reduced-motion: reduce\) \{[\s\S]*\.bottom-bar \.task-group-busy::after \{[\s\S]*animation: none;[\s\S]*background: var\(--js-color-warning-border\);/);
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
