import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const actionsSource = readFileSync(new URL('../src-tauri/src/task_windows/actions.rs', import.meta.url), 'utf8');
const modSource = readFileSync(new URL('../src-tauri/src/task_windows/mod.rs', import.meta.url), 'utf8');
const taskbarWindowsSource = readFileSync(new URL('../src/lib/taskbarWindows.ts', import.meta.url), 'utf8');
const taskbarUiSource = readFileSync(new URL('../src/lib/taskbarUi.ts', import.meta.url), 'utf8');

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
  assert.match(actionsSource, /WM_SYSCOMMAND,[\s\S]*WPARAM\(SC_MINIMIZE as usize\)/);
});

test('taskbar labels expose active-window minimize toggle', () => {
  assert.match(taskbarUiSource, /taskWindow\.isActive \? 'Minimize' : 'Focus'/);
});
