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

test('taskbar snapshot worker keeps watchdog cadence and prioritizes native refreshes', () => {
  assert.match(windowsSource, /const TASKBAR_SNAPSHOT_REFRESH_CADENCE: Duration = Duration::from_secs\(1\);/);
  assert.match(windowsSource, /const TASKBAR_REFRESH_NATIVE_COALESCE: Duration = Duration::from_millis\(30\);/);
  assert.match(windowsSource, /const TASKBAR_REFRESH_MANUAL_COALESCE: Duration = Duration::from_millis\(120\);/);
  assert.match(windowsSource, /const TASKBAR_SNAPSHOT_MAX_AGE: Duration = Duration::from_millis\(1_120\);/);
  assert.doesNotMatch(windowsSource, /refresh_taskbar_snapshot_now\(Some\(&app\)\);\s*loop \{/);
  assert.match(windowsSource, /thread::spawn\(move \|\| \{/);
  assert.match(windowsSource, /mpsc::sync_channel::<TaskbarRefreshRequest>\(8\)/);
  assert.match(windowsSource, /let mut last_refresh_at = Instant::now\(\);/);
  assert.match(windowsSource, /let next_refresh_at = last_refresh_at \+ TASKBAR_SNAPSHOT_REFRESH_CADENCE;/);
  assert.match(windowsSource, /rx\.recv_timeout\(next_refresh_at\.saturating_duration_since\(Instant::now\(\)\)\)/);
  assert.match(windowsSource, /let mut due = request_due_at\(request\);/);
  assert.match(windowsSource, /request = coalesce_request\(request, next\);/);
  assert.match(windowsSource, /TaskbarRefreshReason::Native => TASKBAR_REFRESH_NATIVE_COALESCE/);
  assert.match(windowsSource, /request_taskbar_snapshot_refresh_native/);
  assert.doesNotMatch(windowsSource, /thread::sleep\(remaining\)/);
  assert.match(windowsSource, /last_refresh_at = Instant::now\(\);/);
  assert.match(windowsSource, /refresh_taskbar_snapshot_now\(Some\(&app\)\)/);
  assert.match(modSource, /refresh_taskbar_snapshot_now\(Some\(app\)\)\.ok\(\);\s*windows::ensure_taskbar_snapshot_worker_started/);
  assert.doesNotMatch(windowsSource, /setInterval|setTimeout|frontend poll/i);
});
