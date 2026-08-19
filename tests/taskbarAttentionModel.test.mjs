import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

function readSourceOrEmpty(relativePath) {
  try {
    return readFileSync(new URL(relativePath, import.meta.url), 'utf8');
  } catch (error) {
    if (error && typeof error === 'object' && error.code === 'ENOENT') {
      return '';
    }
    throw error;
  }
}

const contractsSource = readSourceOrEmpty('../src-tauri/src/contracts.rs');
const mainSource = readSourceOrEmpty('../src-tauri/src/main.rs');
const taskWindowsModSource = readSourceOrEmpty('../src-tauri/src/task_windows/mod.rs');
const nativeHooksSource = readSourceOrEmpty('../src-tauri/src/task_windows/native_hooks.rs');
const flashFixtureSource = readSourceOrEmpty('../src-tauri/src/task_windows/flash_fixture.rs');
const attentionSource = readSourceOrEmpty('../src-tauri/src/task_windows/attention.rs');
const windowsSource = readSourceOrEmpty('../src-tauri/src/task_windows/windows.rs');
const bottomBarSource = readSourceOrEmpty('../src/components/BottomBar.svelte');
const taskbarGroupsSource = readSourceOrEmpty('../src/lib/taskbarGroups.ts');

test('phase 0 native attention contract stays independent from phase 2/4 fields', () => {
  assert.match(contractsSource, /TASKBAR_WINDOW/);
  assert.match(taskWindowsModSource, /mod native_hooks;/);
  assert.doesNotMatch(contractsSource, /taskbarWindows\.ts|taskbarWindows/i);
});

test('native attention flash hook exists in native_hooks.rs and does not infer flash from toast terms', () => {
  assert.match(nativeHooksSource, /JASONSHELL_TASKBAR_NATIVE_HOOKS/);
  assert.match(nativeHooksSource, /map_or\(true, \|value\| value != "0"\)/);
  assert.match(nativeHooksSource, /shell_msg_id:\s*u32/);
  assert.match(nativeHooksSource, /RegisterWindowMessageW\(shellhook_name\(\)\)/);
  assert.match(nativeHooksSource, /NativeTaskbarLifecycleEvent::Flash/);
  assert.match(nativeHooksSource, /NativeTaskbarLifecycleEvent::Foreground/);
  assert.match(nativeHooksSource, /RegisterShellHookWindow/);
  assert.match(nativeHooksSource, /HSHELL_FLASH/);
  assert.match(nativeHooksSource, /wparam\.0\s+as\s+i32\s*==\s*HSHELL_FLASH/);
  assert.match(nativeHooksSource, /PostThreadMessageW\([\s\S]*WM_TASKBAR_FLASH/);
  assert.match(nativeHooksSource, /msg\.message\s*==\s*WM_TASKBAR_FLASH/);
  assert.match(nativeHooksSource, /SetWinEventHook\([\s\S]*EVENT_SYSTEM_FOREGROUND/);
  assert.match(nativeHooksSource, /try_send\(/);
  assert.match(nativeHooksSource, /GetWindowThreadProcessId/);
  assert.match(nativeHooksSource, /clear_taskbar_attention_if_matches/);
  assert.match(nativeHooksSource, /PostThreadMessageW/);
  assert.match(nativeHooksSource, /WM_QUIT/);
  assert.match(nativeHooksSource, /link_name\s*=\s*"DeregisterShellHookWindow"/);
  assert.doesNotMatch(nativeHooksSource, /toast|notification|activity|busy/i);
  assert.doesNotMatch(nativeHooksSource, /derive\s+Flash/i);
});

test('flash fixture lives under task_windows and has bounded deterministic helper args', () => {
  assert.match(flashFixtureSource, /FlashWindowEx/);
  assert.match(flashFixtureSource, /SW_SHOWNOACTIVATE/);
  assert.match(flashFixtureSource, /SW_SHOWMINNOACTIVE/);
  assert.match(flashFixtureSource, /SetForegroundWindow/);
  assert.match(flashFixtureSource, /GetForegroundWindow/);
  assert.match(flashFixtureSource, /UnregisterClassW/);
  assert.match(flashFixtureSource, /writeln!\(/);
  assert.match(flashFixtureSource, /\.flush\(\)/);
  assert.match(flashFixtureSource, /visible/i);
  assert.match(flashFixtureSource, /minimized/i);
  assert.match(flashFixtureSource, /timestamp/i);
  assert.match(flashFixtureSource, /cleanup/i);
  assert.match(flashFixtureSource, /std::process::exit\(0\)/);
  assert.match(mainSource, /std::process::exit\(1\)/);
  assert.match(flashFixtureSource, /timeout_ms:\s*u64/);
  assert.match(flashFixtureSource, /Instant::now\(\)\s*\+\s*Duration::from_millis\(args\.timeout_ms\)/);
  assert.match(flashFixtureSource, /--taskbar-flash-fixture/);
  assert.match(flashFixtureSource, /--flash-count/);
  assert.match(flashFixtureSource, /--interval-ms/);
  assert.match(flashFixtureSource, /--timeout-ms/);
  assert.doesNotMatch(flashFixtureSource, /--fixture-case|--jsonl/);
  assert.match(taskWindowsModSource, /flash_fixture::handle_taskbar_flash_fixture_args/);
  assert.match(mainSource, /task_windows::handle_taskbar_flash_fixture_args\(\)/);
});

test('task_windows facade wrappers stay exposed and native hooks module stays private', () => {
  assert.match(taskWindowsModSource, /pub\(crate\) fn start_taskbar_hooks/);
  assert.match(taskWindowsModSource, /pub\(crate\) fn stop_taskbar_hooks/);
  assert.match(taskWindowsModSource, /mod native_hooks;/);
  assert.doesNotMatch(taskWindowsModSource, /pub\s+mod native_hooks;/);
  assert.match(taskWindowsModSource, /pub attention_state: TaskbarWindowAttentionState,/);
  assert.match(taskWindowsModSource, /pub toast_count: u32,/);
  assert.match(attentionSource, /clear_taskbar_attention_if_matches/);
  assert.match(windowsSource, /pub\(super\) fn attention_identity_for_hwnd/);
});

test('attention survives missing creation time until a window snapshot resolves it', () => {
  assert.match(attentionSource, /fn creation_times_match/);
  assert.match(attentionSource, /\(Some\(left\), Some\(right\)\) => left == right/);
  assert.match(attentionSource, /reconcile_keeps_none_creation_time_request_when_visible_snapshot_gains_creation_time/);
  assert.match(attentionSource, /clear_with_known_creation_time_clears_provisional_request/);
});

test('bottom bar binds attention only on the requesting tile, not the whole group', () => {
  assert.match(bottomBarSource, /taskWindowHasVisibleAttention\(taskWindow\) \? ' task-window-attention' : ''/);
  assert.match(bottomBarSource, /return taskWindow\.attentionState === 'requested' && !taskWindow\.isActive;/);
  assert.doesNotMatch(bottomBarSource, /class:task-group-attention=\{taskGroupHasVisibleAttention\(group\)\}/);
  assert.match(taskbarGroupsSource, /group\.hasAttention \|\|= taskWindow\.attentionState === 'requested'/);
});
