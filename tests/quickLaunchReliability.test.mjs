import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const taskbarPinsSource = readFileSync(new URL('../src/lib/taskbarPins.ts', import.meta.url), 'utf8');
const taskbarMenusSource = readFileSync(new URL('../src/lib/taskbarMenus.ts', import.meta.url), 'utf8');

test('Explorer pin launch failures never remove or hide launcher buttons', () => {
  assert.match(bottomBarSource, /async function launchApp\(launcher: PinnedTaskbarLauncher\)/);
  assert.match(bottomBarSource, /await launchPinnedTaskbarLauncher\(launcher\.shortcutPath\)/);
  const launchFn = bottomBarSource.match(
    /async function launchApp\(launcher: PinnedTaskbarLauncher\) \{[\s\S]*?^  \}/m
  )?.[0] ?? '';
  assert.doesNotMatch(launchFn, /launchers = launchers\.filter/);
  assert.doesNotMatch(launchFn, /launchers = \[\]/);
  assert.doesNotMatch(launchFn, /saveShellSettings/);
  assert.doesNotMatch(launchFn, /unpin/);
});

test('app-managed quick icon frontend path is retired', () => {
  assert.doesNotMatch(taskbarPinsSource, /invoke|listQuickIcons|launchQuickIcon|QuickIcon/);
  assert.doesNotMatch(taskbarMenusSource, /QuickIcon|showQuickIconContextMenu|QUICK_ICON_MENU_ACTIONS/);
  assert.doesNotMatch(bottomBarSource, /quickIcons|quick-icon|Pinned quick icons/);
});
