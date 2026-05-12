import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  hasTaskbarLauncherDragStarted,
  orderTaskbarLaunchers,
  reconcileTaskbarLauncherOrder,
  resolveTaskbarLauncherPointerRelease,
  taskbarLauncherOrderFromDisplacement,
  normalizeTaskbarPinTargetKey,
  preserveExplorerTaskbarPins
} from '../dist-tests/lib/taskbarPins.js';

const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const bottomBarCss = readFileSync(new URL('../src/components/BottomBar.css', import.meta.url), 'utf8');
const commandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const settingsSource = readFileSync(new URL('../src/lib/settings.ts', import.meta.url), 'utf8');
const taskbarMenusSource = readFileSync(new URL('../src/lib/taskbarMenus.ts', import.meta.url), 'utf8');

const chrome = {
  id: 'launcher-chrome',
  name: 'Chrome',
  shortcutPath: 'C:\\Pins\\Chrome.lnk',
  targetPath: 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  iconDataUrl: 'data:image/png;base64,aaa'
};

const code = {
  id: 'launcher-code',
  name: 'Code',
  shortcutPath: 'C:\\Pins\\Code.lnk',
  targetPath: 'C:\\Users\\me\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe',
  iconDataUrl: 'data:image/png;base64,bbb'
};

const terminal = {
  id: 'launcher-terminal',
  name: 'Terminal',
  shortcutPath: 'C:\\Pins\\Terminal.lnk',
  targetPath: 'C:\\Users\\me\\AppData\\Local\\Microsoft\\WindowsApps\\wt.exe',
  iconDataUrl: 'data:image/png;base64,ccc'
};

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
  const launchers = [chrome, code];

  const filtered = preserveExplorerTaskbarPins(launchers);
  assert.deepEqual(filtered.map((launcher) => launcher.name), ['Chrome', 'Code']);
});

test('reconciles local Explorer launcher order across native refreshes', () => {
  assert.deepEqual(
    reconcileTaskbarLauncherOrder(
      [terminal.shortcutPath, chrome.shortcutPath],
      [chrome, code, terminal]
    ),
    [terminal.shortcutPath, chrome.shortcutPath, code.shortcutPath]
  );
});

test('orders Explorer launchers by local drag order without mutating launcher data', () => {
  assert.deepEqual(
    orderTaskbarLaunchers([chrome, code, terminal], [terminal.shortcutPath, chrome.shortcutPath])
      .map((launcher) => launcher.name),
    ['Terminal', 'Chrome', 'Code']
  );
});

test('uses task-window-style launcher drag threshold and displacement ordering', () => {
  const order = [chrome.shortcutPath, code.shortcutPath, terminal.shortcutPath];
  const rects = [
    { key: chrome.shortcutPath, left: 0, width: 40 },
    { key: code.shortcutPath, left: 40, width: 40 },
    { key: terminal.shortcutPath, left: 80, width: 40 }
  ];

  assert.equal(hasTaskbarLauncherDragStarted(100, 105), false);
  assert.equal(hasTaskbarLauncherDragStarted(100, 106), true);
  assert.deepEqual(taskbarLauncherOrderFromDisplacement(chrome.shortcutPath, order, rects, 81), [
    code.shortcutPath,
    terminal.shortcutPath,
    chrome.shortcutPath
  ]);
  assert.deepEqual(taskbarLauncherOrderFromDisplacement(terminal.shortcutPath, order, rects, -81), [
    terminal.shortcutPath,
    chrome.shortcutPath,
    code.shortcutPath
  ]);
});

test('launcher pointer release suppresses click only after a real drag', () => {
  assert.deepEqual(resolveTaskbarLauncherPointerRelease(chrome.shortcutPath, false), {
    suppressClickKey: null
  });
  assert.deepEqual(resolveTaskbarLauncherPointerRelease(chrome.shortcutPath, true), {
    suppressClickKey: chrome.shortcutPath
  });
});

test('BottomBar wires pinned Explorer launchers into pointer drag reorder path', () => {
  assert.match(bottomBarSource, /TASKBAR_LAUNCHER_ORDER_STORAGE_KEY/);
  assert.match(bottomBarSource, /let launcherOrder: string\[\] = readPersistedLauncherOrder\(\)/);
  assert.match(bottomBarSource, /writePersistedLauncherOrder\(launcherOrder\)/);
  assert.match(bottomBarSource, /onPointerDown=\{\(event\) => startLauncherPointerDrag\(launcher, event\)\}/);
  assert.match(bottomBarSource, /onPointerMove=\{moveLauncherPointerDrag\}/);
  assert.match(bottomBarSource, /taskbarLauncherOrderFromDisplacement/);
  assert.match(bottomBarSource, /suppressClickLauncherKey/);
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
