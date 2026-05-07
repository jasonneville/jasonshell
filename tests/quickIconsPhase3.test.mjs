import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const commandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const settingsSource = readFileSync(new URL('../src/lib/settings.ts', import.meta.url), 'utf8');
const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const taskbarMenusSource = readFileSync(new URL('../src/lib/taskbarMenus.ts', import.meta.url), 'utf8');
const taskbarMenuRs = readFileSync(new URL('../src-tauri/src/taskbar_menu.rs', import.meta.url), 'utf8');
const settingsRs = readFileSync(new URL('../src-tauri/src/settings.rs', import.meta.url), 'utf8');
const quickIconsRs = readFileSync(new URL('../src-tauri/src/quick_icons.rs', import.meta.url), 'utf8');

test('phase 3 wires quick-icon IPC commands and shell settings contracts', () => {
  assert.match(commandsSource, /listQuickIcons: 'list_quick_icons'/);
  assert.match(commandsSource, /pinTaskWindowQuickIcon: 'pin_task_window_quick_icon'/);
  assert.match(commandsSource, /unpinQuickIcon: 'unpin_quick_icon'/);
  assert.match(commandsSource, /launchQuickIcon: 'launch_quick_icon'/);

  assert.match(settingsSource, /quickIcons: QuickIconsSettings/);
  assert.match(settingsSource, /defaultQuickIconsSettings\(\)/);

  assert.match(settingsRs, /pub quick_icons: QuickIconsSettings/);
  assert.match(settingsRs, /struct QuickIconsSettings/);
  assert.match(settingsRs, /struct QuickIconEntry/);
  assert.match(settingsRs, /validate_quick_icons_settings/);
});

test('phase 3 updates taskbar context menus for pinning and unpinning app-managed quick icons', () => {
  assert.match(taskbarMenusSource, /showQuickIconContextMenu/);
  assert.match(taskbarMenusSource, /type ShowQuickIconContextMenuRequest/);

  assert.match(taskbarMenuRs, /"Pin to quick icons"/);
  assert.match(taskbarMenuRs, /"Unpin from quick icons"/);
  assert.match(taskbarMenuRs, /pin_task_window_quick_icon/);
  assert.match(taskbarMenuRs, /unpin_quick_icon/);
});

test('phase 3 bottom bar renders quick icons and keeps Explorer launchers available', () => {
  assert.match(bottomBarSource, /listQuickIcons/);
  assert.match(bottomBarSource, /launchQuickIcon/);
  assert.match(bottomBarSource, /showQuickIconContextMenu/);
  assert.match(bottomBarSource, /quick-icon-button/);
  assert.match(bottomBarSource, /Pinned quick icons/);
  assert.match(bottomBarSource, /Pinned Explorer taskbar apps/);
});

test('quick icon launch uses audited app launcher instead of generic shell open', () => {
  assert.match(quickIconsRs, /launch_audited_app_path\(entry\.target_path\)/);
  assert.doesNotMatch(quickIconsRs, /open_shell_path\(entry\.target_path\)/);
});
