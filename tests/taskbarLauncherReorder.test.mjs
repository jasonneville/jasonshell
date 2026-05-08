import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  orderTaskbarLaunchers,
  resolveTaskbarLauncherPointerRelease,
  taskbarLauncherOrderFromDisplacement
} from '../dist-tests/lib/taskbarPins.js';

const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');

const chrome = { id: 'chrome', name: 'Chrome', shortcutPath: 'C:\\Pins\\Chrome.lnk' };
const code = { id: 'code', name: 'Code', shortcutPath: 'C:\\Pins\\Code.lnk' };
const terminal = { id: 'terminal', name: 'Terminal', shortcutPath: 'C:\\Pins\\Terminal.lnk' };

test('reorders pinned taskbar launchers left and right by shortcut path displacement', () => {
  const order = [chrome.shortcutPath, code.shortcutPath, terminal.shortcutPath];
  const rects = [
    { key: chrome.shortcutPath, left: 0, width: 40 },
    { key: code.shortcutPath, left: 40, width: 40 },
    { key: terminal.shortcutPath, left: 80, width: 40 }
  ];

  assert.deepEqual(
    orderTaskbarLaunchers(
      [chrome, code, terminal],
      taskbarLauncherOrderFromDisplacement(chrome.shortcutPath, order, rects, 81)
    ),
    [code, terminal, chrome]
  );
  assert.deepEqual(
    orderTaskbarLaunchers(
      [chrome, code, terminal],
      taskbarLauncherOrderFromDisplacement(terminal.shortcutPath, order, rects, -81)
    ),
    [terminal, chrome, code]
  );
});

test('keeps launcher order stable for missing or same-position moves', () => {
  const order = [chrome.shortcutPath, code.shortcutPath, terminal.shortcutPath];
  assert.deepEqual(
    taskbarLauncherOrderFromDisplacement('C:\\Pins\\Missing.lnk', order, [], 40),
    order
  );
  assert.deepEqual(
    taskbarLauncherOrderFromDisplacement(code.shortcutPath, order, [], 0),
    order
  );
});

test('resolves launcher pointer release without launching after a drag', () => {
  assert.deepEqual(resolveTaskbarLauncherPointerRelease(chrome.shortcutPath, false), {
    suppressClickKey: null
  });
  assert.deepEqual(resolveTaskbarLauncherPointerRelease(chrome.shortcutPath, true), {
    suppressClickKey: chrome.shortcutPath
  });
});

test('BottomBar wires launcher pointer reorder separately from click and native menus', () => {
  assert.match(bottomBarSource, /TASKBAR_LAUNCHER_ORDER_STORAGE_KEY/);
  assert.match(bottomBarSource, /readPersistedLauncherOrder\(\)/);
  assert.match(bottomBarSource, /writePersistedLauncherOrder\(launcherOrder\)/);
  assert.match(bottomBarSource, /resolveTaskbarLauncherPointerRelease/);
  assert.match(bottomBarSource, /orderTaskbarLaunchers/);
  assert.match(bottomBarSource, /taskbarLauncherOrderFromDisplacement/);
  assert.match(bottomBarSource, /startLauncherPointerDrag\(launcher, event\)/);
  assert.match(bottomBarSource, /moveLauncherPointerDrag/);
  assert.match(bottomBarSource, /finishLauncherPointerDrag/);
  assert.match(bottomBarSource, /onClick=\{\(event\) => handleLauncherClick\(launcher, event\)\}/);
  assert.match(bottomBarSource, /onContextMenu=\{\(event\) => void openLauncherMenu\(launcher, event\)\}/);
});
