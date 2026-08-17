import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const actionsSource = readFileSync(new URL('../src-tauri/src/task_windows/actions.rs', import.meta.url), 'utf8');
const modSource = readFileSync(new URL('../src-tauri/src/task_windows/mod.rs', import.meta.url), 'utf8');
const taskbarWindowsSource = readFileSync(new URL('../src/lib/taskbarWindows.ts', import.meta.url), 'utf8');
const taskWindowsSource = readFileSync(new URL('../src-tauri/src/task_windows/windows.rs', import.meta.url), 'utf8');
const taskbarUiSource = readFileSync(new URL('../src/lib/taskbarUi.ts', import.meta.url), 'utf8');
const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');

test('task window activation validates live hwnds and verifies focus outcome', () => {
  assert.match(actionsSource, /if !window_exists\(target\) \|\| !window_exists\(raw_hwnd\) \{[\s\S]*Task window is no longer available/);
  assert.match(actionsSource, /fn activate_window\([\s\S]*-> Result<\(\), String>/);
  assert.match(actionsSource, /PostMessageW\([\s\S]*WM_SYSCOMMAND,[\s\S]*WPARAM\(SC_RESTORE as usize\),[\s\S]*LPARAM\(0\),[\s\S]*\)/);
  assert.match(actionsSource, /wait_for_windows_restore\(&restore_targets\);/);
  assert.match(actionsSource, /targets[\s\S]*\.iter\(\)[\s\S]*\.all\(\|hwnd\| !unsafe \{ IsIconic\(\*hwnd\)\.as_bool\(\) \}\)/);
  assert.match(actionsSource, /if !restore_targets\.is_empty\(\) \{/);
  assert.match(actionsSource, /Err\("Failed to focus task window"\.to_string\(\)\)/);
});

test('task window command carries guarded active-click intent', () => {
  assert.match(modSource, /pub fn activate_task_window\(hwnd: String, minimize_if_active: bool\) -> Result<\(\), String>/);
  assert.match(modSource, /actions::activate_task_window\(hwnd, minimize_if_active\)/);
  assert.match(taskbarWindowsSource, /export function activateTaskWindow\(hwnd: string, minimizeIfActive = false\): Promise<void>/);
  assert.match(taskbarWindowsSource, /invoke\(IPC_COMMANDS\.activateTaskWindow, \{ hwnd, minimizeIfActive \}\)/);
  assert.match(actionsSource, /minimize_if_active && shell_is_foreground/);
  assert.match(actionsSource, /hwnd\.0 as isize/);
  assert.doesNotMatch(actionsSource, /format!\("\{:\?\}", hwnd\)/);
  assert.match(actionsSource, /WM_SYSCOMMAND,[\s\S]*WPARAM\(SC_MINIMIZE as usize\)/);
});

test('task window close flow captures initial identity and escalates only on access denied', () => {
  assert.match(actionsSource, /let initial_identity = capture_task_window_identity_at_start\(hwnd\)\?/);
  assert.match(actionsSource, /revalidate_close_target\(hwnd, &initial_identity\)\?/);
  assert.match(actionsSource, /let send_error = if send_timeout_succeeded \{ 0 \} else \{ unsafe \{ GetLastError\(\)\.0 \} \};/);
  assert.match(actionsSource, /if send_error != 0 && should_elevate_after_access_denied\(send_error\) \{/);
  assert.match(actionsSource, /Err\(error\) if should_elevate_after_access_denied\(error\.code\(\)\.0 as u32\) => \{\s*return elevate_close_target\(hwnd, &initial_identity\);/);
  assert.match(actionsSource, /Err\(error\) => \{\s*return Err\(format!\("Failed to close task window: \{error\}"\)\);/);
  assert.match(actionsSource, /fn elevate_close_target\(/);
  assert.match(actionsSource, /spawn_task_window_helper\(/);
  assert.match(actionsSource, /capture_task_window_identity\([\s\S]*if current\.process_id != process_id/);
});

test('task window close flow does not elevate on ordinary post failure', () => {
  assert.match(actionsSource, /PostMessageW\(Some\(hwnd\), WM_CLOSE, WPARAM\(0\), LPARAM\(0\)\) \}/);
  assert.match(actionsSource, /Err\(error\) => \{\s*return Err\(format!\("Failed to close task window: \{error\}"\)\);/);
  assert.match(actionsSource, /should_elevate_after_access_denied\(send_error\)/);
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

test('task window source no longer uses monitor-specific gating', () => {
  assert.doesNotMatch(taskWindowsSource, /\bmonitor\b/i);
  assert.doesNotMatch(taskWindowsSource, /\bprimary\s+monitor\b/i);
  assert.doesNotMatch(taskWindowsSource, /is_primary_monitor|primary_monitor|monitor_index|monitor_handle|MonitorFrom/i);
});
