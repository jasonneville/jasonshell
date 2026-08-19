import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const taskbarWindowsSource = readFileSync(new URL('../src/lib/taskbarWindows.ts', import.meta.url), 'utf8');
const taskbarUiSource = readFileSync(new URL('../src/lib/taskbarUi.ts', import.meta.url), 'utf8');
const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');

test('task window frontend command keeps optimistic activation and minimize intent', () => {
  assert.match(taskbarWindowsSource, /export function activateTaskWindow\(hwnd: string, minimizeIfActive = false\): Promise<void>/);
  assert.match(taskbarWindowsSource, /invoke\(IPC_COMMANDS\.activateTaskWindow, \{ hwnd, minimizeIfActive \}\)/);
});

test('taskbar labels expose active-window minimize toggle', () => {
  assert.match(taskbarUiSource, /taskWindow\.isActive \? 'Minimize' : 'Focus'/);
});

test('task window activation updates active highlight before cached snapshot refresh', () => {
  assert.match(bottomBarSource, /function applyOptimisticTaskWindowActivation\(taskWindow: TaskbarWindow\)/);
  assert.match(bottomBarSource, /isActive: window\.hwnd === taskWindow\.hwnd \? !taskWindow\.isActive : false/);
  assert.match(bottomBarSource, /isMinimized: window\.hwnd === taskWindow\.hwnd \? taskWindow\.isActive : window\.isMinimized/);
  assert.match(bottomBarSource, /await activateTaskWindow\(taskWindow\.hwnd, taskWindow\.isActive\);\s*applyOptimisticTaskWindowActivation\(taskWindow\);\s*void requestTaskbarWindowsRefresh\(\);/);
});

test('taskbar window frontend payload normalizes attention and toast counts', () => {
  assert.match(taskbarWindowsSource, /export type TaskbarWindowAttentionState = 'idle' \| 'requested';/);
  assert.match(taskbarWindowsSource, /attentionState: window\.attentionState === 'requested' \? 'requested' : 'idle'/);
  assert.match(taskbarWindowsSource, /toastCount\n\s*};/);
  assert.doesNotMatch(taskbarWindowsSource, /notificationCount/);
});
