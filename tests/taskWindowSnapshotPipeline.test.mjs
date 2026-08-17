import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const taskbarWindowsSource = readFileSync(new URL('../src/lib/taskbarWindows.ts', import.meta.url), 'utf8');
const taskbarUiSource = readFileSync(new URL('../src/lib/taskbarUi.ts', import.meta.url), 'utf8');
const modSource = readFileSync(new URL('../src-tauri/src/task_windows/mod.rs', import.meta.url), 'utf8');
const windowsSource = readFileSync(new URL('../src-tauri/src/task_windows/windows.rs', import.meta.url), 'utf8');
const notificationsSource = readFileSync(new URL('../src-tauri/src/task_windows/notifications.rs', import.meta.url), 'utf8');
const contractsSource = readFileSync(new URL('../src-tauri/src/contracts.rs', import.meta.url), 'utf8');

test('task window snapshots are producer-driven, not permanent frontend polls', () => {
  assert.doesNotMatch(topBarSource, /setInterval\(\(\) => \{[\s\S]*listOpenTaskWindows\(\)/);
  assert.doesNotMatch(bottomBarSource, /setInterval\(\(\) => \{[\s\S]*refreshTaskbarWindows\(\)/);
  assert.match(taskbarWindowsSource, /listOpenTaskWindows\(\): Promise<TaskbarWindow\[\]>/);
  assert.match(bottomBarSource, /taskbar:windows-snapshot/);
  assert.match(topBarSource, /taskbar:windows-snapshot/);
  assert.match(taskbarWindowsSource, /requestTaskbarWindowsRefresh/);
});

test('task window snapshot stream is sequenced with last snapshot fallback', () => {
  assert.match(modSource, /TaskbarWindowsSnapshot/);
  assert.match(modSource, /TASKBAR_WINDOWS_SNAPSHOT_EVENT/);
  assert.match(windowsSource, /TASKBAR_SNAPSHOT_SEQUENCE/);
  assert.match(contractsSource, /LIST_OPEN_TASK_WINDOWS/);
});

test('notification lookup cache is bounded and negative-cached', () => {
  assert.match(notificationsSource, /negative|miss/i);
  assert.match(notificationsSource, /ttl|time to live|Duration::from_secs/i);
  assert.match(notificationsSource, /cache/i);
  assert.match(notificationsSource, /MAX_APP_INSTALL_PATH_CACHE_ENTRIES/);
  assert.match(notificationsSource, /VecDeque|evict|retain/);
});

test('taskbar snapshot consumers reject stale or equal sequences', () => {
  assert.match(topBarSource, /lastTaskbarSnapshotSequence/);
  assert.match(bottomBarSource, /lastTaskbarSnapshotSequence/);
  assert.match(topBarSource, /sequence <= lastTaskbarSnapshotSequence/);
  assert.match(bottomBarSource, /sequence <= lastTaskbarSnapshotSequence/);
});

test('producer avoids holding locks across enum/winrt/icon emit', () => {
  assert.match(windowsSource, /EnumWindows/);
  assert.match(windowsSource, /task_window_activity_state/);
  assert.doesNotMatch(windowsSource, /lock\(\)[\s\S]{0,300}(EnumWindows|window_icon_data_url|emit_to)/);
});
