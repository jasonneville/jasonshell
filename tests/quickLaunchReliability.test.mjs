import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  buildQuickIconLaunchFailureState,
  createQuickIconLaunchError,
  removeQuickIconEntry
} from '../dist-tests/lib/quickIcons.js';

const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const quickIconsSource = readFileSync(new URL('../src/lib/quickIcons.ts', import.meta.url), 'utf8');
const taskbarMenusSource = readFileSync(new URL('../src/lib/taskbarMenus.ts', import.meta.url), 'utf8');
const quickIconsRs = readFileSync(new URL('../src-tauri/src/quick_icons.rs', import.meta.url), 'utf8');

const terminal = {
  id: 'terminal',
  name: 'Terminal',
  targetPath: 'C:\\Users\\me\\AppData\\Local\\Microsoft\\WindowsApps\\wt.exe',
  iconDataUrl: 'data:image/png;base64,aaa'
};
const spotify = {
  id: 'spotify',
  name: 'Spotify',
  targetPath: 'C:\\Users\\me\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Spotify.lnk',
  iconDataUrl: 'data:image/png;base64,bbb'
};

test('launchQuickIconFailureKeepsEntry stores stable per-icon error without mutating entries', () => {
  const before = [terminal, spotify];
  const snapshot = JSON.stringify(before);
  const error = createQuickIconLaunchError('spawnFailed', 'ShellExecuteW failed', terminal.id);

  const state = buildQuickIconLaunchFailureState(before, terminal.id, error);

  assert.equal(JSON.stringify(before), snapshot);
  assert.deepEqual(state.quickIcons, before);
  assert.deepEqual(state.errorById[terminal.id], {
    code: 'spawnFailed',
    message: 'ShellExecuteW failed',
    pathOrId: terminal.id
  });
});

test('launch-only BottomBar error branch never removes or persists quick icons', () => {
  assert.match(bottomBarSource, /async function launchQuickIconFromBottomBar\(quickIcon: QuickIcon\)/);
  assert.match(bottomBarSource, /await launchQuickIcon\(\{ id: quickIcon\.id \}\)/);
  assert.match(bottomBarSource, /quickIconLaunchErrors = \{/);
  const launchFn = bottomBarSource.match(
    /async function launchQuickIconFromBottomBar\(quickIcon: QuickIcon\) \{[\s\S]*?^  \}/m
  )?.[0] ?? '';
  assert.doesNotMatch(launchFn, /quickIcons = quickIcons\.filter/);
  assert.doesNotMatch(launchFn, /saveShellSettings/);
  assert.doesNotMatch(launchFn, /unpinQuickIcon/);
});

test('quick icon context model names explicit unpin action and payload', () => {
  assert.match(taskbarMenusSource, /QUICK_ICON_MENU_ACTIONS/);
  assert.match(taskbarMenusSource, /unpinQuickIcon: 'unpinQuickIcon'/);
  assert.match(taskbarMenusSource, /type QuickIconMenuActionPayload/);
});

test('removeQuickIconEntry removes exactly one app-managed icon and preserves Explorer pins elsewhere', () => {
  const removed = removeQuickIconEntry([terminal, spotify], 'terminal');

  assert.deepEqual(removed.map((entry) => entry.id), ['spotify']);
  assert.deepEqual(removeQuickIconEntry([terminal, spotify], 'missing').map((entry) => entry.id), [
    'terminal',
    'spotify'
  ]);
});

test('Rust quick icon pin/unpin is app-managed and non-destructive to Explorer taskbar folder', () => {
  assert.match(quickIconsRs, /upsert_quick_icon_entry/);
  assert.match(quickIconsRs, /quick_icon_entry_from_task_window/);
  assert.match(quickIconsRs, /save_shell_settings_for_app/);
  assert.doesNotMatch(quickIconsRs, /pin_executable_to_taskbar_shortcut/);
  assert.match(quickIconsRs, /remove_quick_icon_entry/);
});
