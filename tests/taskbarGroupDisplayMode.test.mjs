import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  buildTaskWindowGroups,
  deriveTaskGroupAggregateState,
  taskGroupDisplayMode,
  taskGroupGalleryItems,
  taskGroupRepresentativeWindow,
  taskbarStripPressureState
} from '../dist-tests/lib/taskbarGroups.js';

function taskWindow(overrides) {
  return {
    hwnd: overrides.hwnd,
    title: overrides.title ?? overrides.processName,
    processName: overrides.processName,
    iconDataUrl: overrides.iconDataUrl ?? `data:image/png;base64,${overrides.hwnd}`,
    isActive: overrides.isActive ?? false,
    isMinimized: overrides.isMinimized ?? false,
    activityState: overrides.activityState ?? 'idle',
    attentionState: overrides.attentionState ?? 'idle',
    toastCount: overrides.toastCount ?? 0
  };
}

function group(count, overrides = {}) {
  const windows = Array.from({ length: count }, (_, index) => taskWindow({
    hwnd: `${index + 1}`,
    processName: 'Code',
    title: `Window ${index + 1}`,
    ...overrides[index]
  }));
  return buildTaskWindowGroups(windows)[0];
}

test('supported policies keep one-window groups direct', () => {
  for (const policy of ['auto', 'always', 'never']) {
    assert.equal(taskGroupDisplayMode(group(1), { policy, pressure: false }), 'direct');
  }
});

test('auto policy capsules groups with two or more windows', () => {
  assert.equal(taskGroupDisplayMode(group(2), { policy: 'auto', pressure: false }), 'capsule');
  assert.equal(taskGroupDisplayMode(group(3), { policy: 'auto', pressure: false }), 'capsule');
  assert.equal(taskGroupDisplayMode(group(4), { policy: 'auto', pressure: false }), 'capsule');
  assert.equal(taskGroupDisplayMode(group(7), { policy: 'auto', pressure: false }), 'capsule');
});

test('pressure only collapses multi-window groups under auto policy', () => {
  assert.equal(taskGroupDisplayMode(group(1), { policy: 'auto', pressure: true }), 'direct');
  assert.equal(taskGroupDisplayMode(group(2), { policy: 'auto', pressure: true }), 'capsule');
});

test('always policy capsules multi-window groups while preserving single direct window', () => {
  assert.equal(taskGroupDisplayMode(group(1), { policy: 'always', pressure: false }), 'direct');
  assert.equal(taskGroupDisplayMode(group(2), { policy: 'always', pressure: false }), 'capsule');
});

test('never policy keeps multi-window groups direct even under pressure', () => {
  assert.equal(taskGroupDisplayMode(group(5), { policy: 'never', pressure: true }), 'direct');
});

test('representative window prefers active child and otherwise first stable child', () => {
  const activeGroup = group(3, { 1: { isActive: true, title: 'Active child' } });
  const idleGroup = group(3);

  assert.equal(taskGroupRepresentativeWindow(activeGroup).hwnd, '2');
  assert.equal(taskGroupRepresentativeWindow(idleGroup).hwnd, '1');
});

test('gallery items preserve original window order', () => {
  const taskGroup = buildTaskWindowGroups([
    taskWindow({ hwnd: 'a', processName: 'Code', title: 'Alpha' }),
    taskWindow({ hwnd: 'b', processName: 'code', title: 'Beta', isActive: true }),
    taskWindow({ hwnd: 'c', processName: 'code', title: 'Gamma' })
  ])[0];

  assert.deepEqual(taskGroupGalleryItems(taskGroup).map((item) => item.hwnd), ['a', 'b', 'c']);
});

test('aggregate state derives active minimized attention busy and toast metadata from children', () => {
  const windows = [
    taskWindow({ hwnd: '1', processName: 'WindowsTerminal', isMinimized: true, toastCount: 1 }),
    taskWindow({ hwnd: '2', processName: 'WindowsTerminal', isActive: true, isMinimized: false, activityState: 'busy', attentionState: 'requested', toastCount: 5 })
  ];

  assert.deepEqual(deriveTaskGroupAggregateState(windows), {
    isActive: true,
    isMinimized: false,
    isBusy: true,
    hasAttention: true,
    toastCount: 5
  });
});

test('strip pressure enters only after the deficit exceeds the enter threshold', () => {
  assert.equal(taskbarStripPressureState({
    previousPressure: false,
    availableWidth: 500,
    requiredDirectWidth: 524,
    enterThreshold: 24,
    exitThreshold: 48
  }), false);

  assert.equal(taskbarStripPressureState({
    previousPressure: false,
    availableWidth: 500,
    requiredDirectWidth: 525,
    enterThreshold: 24,
    exitThreshold: 48
  }), true);
});

test('strip pressure remains active near the boundary even with small spare width', () => {
  assert.equal(taskbarStripPressureState({
    previousPressure: true,
    availableWidth: 516,
    requiredDirectWidth: 500,
    enterThreshold: 24,
    exitThreshold: 48
  }), true);
});

test('strip pressure exits only when spare width reaches the exit threshold', () => {
  assert.equal(taskbarStripPressureState({
    previousPressure: true,
    availableWidth: 547,
    requiredDirectWidth: 500,
    enterThreshold: 24,
    exitThreshold: 48
  }), true);

  assert.equal(taskbarStripPressureState({
    previousPressure: true,
    availableWidth: 548,
    requiredDirectWidth: 500,
    enterThreshold: 24,
    exitThreshold: 48
  }), false);
});

test('strip pressure preserves previous pressure when available width is zero or unmeasured', () => {
  assert.equal(taskbarStripPressureState({
    previousPressure: false,
    availableWidth: 0,
    requiredDirectWidth: 999,
    enterThreshold: 24,
    exitThreshold: 48
  }), false);

  assert.equal(taskbarStripPressureState({
    previousPressure: true,
    availableWidth: 0,
    requiredDirectWidth: 100,
    enterThreshold: 24,
    exitThreshold: 48
  }), true);

  assert.equal(taskbarStripPressureState({
    previousPressure: false,
    availableWidth: undefined,
    requiredDirectWidth: 999,
    enterThreshold: 24,
    exitThreshold: 48
  }), false);
});
