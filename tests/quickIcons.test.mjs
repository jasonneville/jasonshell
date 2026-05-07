import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  normalizeTaskbarPinTargetKey,
  preserveExplorerTaskbarPins
} from '../dist-tests/lib/taskbarPins.js';

const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const bottomBarCss = readFileSync(new URL('../src/components/BottomBar.css', import.meta.url), 'utf8');
const commandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const settingsSource = readFileSync(new URL('../src/lib/settings.ts', import.meta.url), 'utf8');
const taskbarMenusSource = readFileSync(new URL('../src/lib/taskbarMenus.ts', import.meta.url), 'utf8');

test('normalizes Windows-like taskbar pin target keys for diagnostics', () => {
  assert.equal(
    normalizeTaskbarPinTargetKey('C:/Program Files/Google/Chrome/Application/chrome.exe'),
    'c:\\program files\\google\\chrome\\application\\chrome.exe'
  );
  assert.equal(
    normalizeTaskbarPinTargetKey('  \\\\Server\\Share\\Tool.exe  '),
    '\\\\server\\share\\tool.exe'
  );
});

test('preserves Explorer launchers without app-managed quick icon dedupe', () => {
  const launchers = [
    {
      id: 'launcher-chrome',
      name: 'Chrome',
      shortcutPath: 'C:\\Users\\me\\AppData\\Roaming\\Microsoft\\Internet Explorer\\Quick Launch\\User Pinned\\TaskBar\\Chrome.lnk',
      targetPath: 'c:/program files/google/chrome/application/chrome.exe',
      iconDataUrl: 'data:image/png;base64,aaa'
    },
    {
      id: 'launcher-code',
      name: 'Code',
      shortcutPath: 'C:\\Users\\me\\AppData\\Roaming\\Microsoft\\Internet Explorer\\Quick Launch\\User Pinned\\TaskBar\\Code.lnk',
      targetPath: 'C:\\Users\\me\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe',
      iconDataUrl: 'data:image/png;base64,bbb'
    }
  ];

  const filtered = preserveExplorerTaskbarPins(launchers);
  assert.deepEqual(filtered.map((launcher) => launcher.name), ['Chrome', 'Code']);
});

test('bottom bar renders only Explorer taskbar pins before open windows', () => {
  assert.match(bottomBarSource, /listPinnedTaskbarLaunchers/);
  assert.match(bottomBarSource, /Pinned Explorer taskbar apps/);
  assert.doesNotMatch(bottomBarSource, /listQuickIcons|launchQuickIcon|showQuickIconContextMenu/);
  assert.doesNotMatch(bottomBarSource, /quickIcons|quick-icon|Pinned quick icons/);
  assert.doesNotMatch(bottomBarCss, /quick-icon/);
});

test('frontend no longer exposes app-managed quick icon IPC or settings path', () => {
  assert.doesNotMatch(commandsSource, /listQuickIcons|pinTaskWindowQuickIcon|unpinQuickIcon|launchQuickIcon/);
  assert.doesNotMatch(commandsSource, /showQuickIconContextMenu/);
  assert.doesNotMatch(settingsSource, /quickIcons|QuickIconsSettings|defaultQuickIconsSettings/);
  assert.doesNotMatch(taskbarMenusSource, /QuickIcon|showQuickIconContextMenu|QUICK_ICON_MENU_ACTIONS/);
});
