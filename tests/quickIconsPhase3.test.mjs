import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const commandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const settingsSource = readFileSync(new URL('../src/lib/settings.ts', import.meta.url), 'utf8');
const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const taskbarMenusSource = readFileSync(new URL('../src/lib/taskbarMenus.ts', import.meta.url), 'utf8');
const taskbarMenuRs = readFileSync(new URL('../src-tauri/src/taskbar_menu.rs', import.meta.url), 'utf8');
const settingsRs = readFileSync(new URL('../src-tauri/src/settings.rs', import.meta.url), 'utf8');

test('frontend retires phase 3 app-managed quick-icon IPC and settings contracts', () => {
  assert.doesNotMatch(commandsSource, /listQuickIcons|pinTaskWindowQuickIcon|unpinQuickIcon|launchQuickIcon/);
  assert.doesNotMatch(commandsSource, /showQuickIconContextMenu/);

  assert.doesNotMatch(settingsSource, /quickIcons: QuickIconsSettings/);
  assert.doesNotMatch(settingsSource, /defaultQuickIconsSettings\(\)/);

  assert.doesNotMatch(settingsRs, /pub quick_icons: QuickIconsSettings/);
  assert.doesNotMatch(settingsRs, /struct QuickIconsSettings/);
  assert.doesNotMatch(settingsRs, /struct QuickIconEntry/);
  assert.doesNotMatch(settingsRs, /validate_quick_icons_settings/);
});

test('frontend taskbar context menu wrapper exposes Explorer launcher and task-window menus only', () => {
  assert.match(taskbarMenusSource, /showLauncherContextMenu/);
  assert.match(taskbarMenusSource, /showTaskWindowContextMenu/);
  assert.doesNotMatch(taskbarMenusSource, /showQuickIconContextMenu/);
  assert.doesNotMatch(taskbarMenusSource, /type ShowQuickIconContextMenuRequest/);

  assert.doesNotMatch(taskbarMenuRs, /"Pin to quick icons"/);
  assert.doesNotMatch(taskbarMenuRs, /"Unpin from quick icons"/);
  assert.doesNotMatch(taskbarMenuRs, /pin_task_window_quick_icon/);
  assert.doesNotMatch(taskbarMenuRs, /unpin_quick_icon/);
});

test('bottom bar renders Explorer taskbar pins with no separate quick-icon strip', () => {
  assert.doesNotMatch(bottomBarSource, /listQuickIcons/);
  assert.doesNotMatch(bottomBarSource, /launchQuickIcon/);
  assert.doesNotMatch(bottomBarSource, /showQuickIconContextMenu/);
  assert.doesNotMatch(bottomBarSource, /quick-icon-button/);
  assert.doesNotMatch(bottomBarSource, /Pinned quick icons/);
  assert.match(bottomBarSource, /Pinned Explorer taskbar apps/);
});
