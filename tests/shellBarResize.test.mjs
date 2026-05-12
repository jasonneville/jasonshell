import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  clampShellBarHeight,
  shellBarHeightForSettingsUpdate,
  shellBarHeightFromDrag
} from '../dist-tests/lib/shellBarResize.js';
import { defaultShellSettings } from '../dist-tests/lib/settings.js';

const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const settingsPanelSource = readFileSync(new URL('../src/components/SettingsPanelSurface.svelte', import.meta.url), 'utf8');
const appbarSource = readFileSync(new URL('../src-tauri/src/appbar.rs', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

test('JSON shell settings default both shell bar height locks on', () => {
  assert.deepEqual(defaultShellSettings().ui, {
    activeWorkspaceId: null,
    enableDiagnosticsExport: false,
    searchMode: 'centeredHotkey',
    lockTopBarHeight: true,
    lockBottomBarHeight: true,
    topBarHeightLogical: 23.4,
    bottomBarHeightLogical: 32.4
  });
});

test('shell bar drag math grows top downward and bottom upward within bounds', () => {
  assert.equal(clampShellBarHeight('top', 1), 18);
  assert.equal(clampShellBarHeight('bottom', Number.NaN), 24);
  assert.equal(clampShellBarHeight('top', 500), 120);
  assert.equal(shellBarHeightFromDrag('top', 23.4, 40, 35), 18.4);
  assert.equal(shellBarHeightFromDrag('bottom', 32.4, 70, 80), 24);
  assert.equal(shellBarHeightFromDrag('top', 23.4, 10, 40), 53.4);
  assert.equal(shellBarHeightFromDrag('bottom', 32.4, 100, 70), 62.4);
});

test('settings sync preserves optimistic resize heights until drag persistence settles', () => {
  assert.equal(shellBarHeightForSettingsUpdate('top', 23.4, 48, true), 48);
  assert.equal(shellBarHeightForSettingsUpdate('bottom', 32.4, 58, true), 58);
  assert.equal(shellBarHeightForSettingsUpdate('top', 23.4, 48, false), 23.4);
  assert.equal(shellBarHeightForSettingsUpdate('bottom', 32.4, 58, false), 32.4);
  assert.equal(shellBarHeightForSettingsUpdate('top', Number.NaN, 48, false), 18);
});

test('resize drags optimistically update local height and coalesce native IPC', () => {
  assert.match(topBarSource, /createShellBarResizeScheduler\('top'/);
  assert.match(topBarSource, /topBarPendingPersistedHeight/);
  assert.match(topBarSource, /shellBarHeightForSettingsUpdate\(\s*'top'/);
  assert.match(topBarSource, /saveShellBarHeight\('top', nextHeight\)/);
  assert.match(topBarSource, /topBarHeightLogical = nextHeight;\s*topBarResizeScheduler\.schedule\(nextHeight\)/);
  assert.match(topBarSource, /topBarResizeScheduler\.flush\(nextHeight\)/);
  assert.match(bottomBarSource, /createShellBarResizeScheduler\('bottom'/);
  assert.match(bottomBarSource, /bottomBarPendingPersistedHeight/);
  assert.match(bottomBarSource, /shellBarHeightForSettingsUpdate\(\s*'bottom'/);
  assert.match(bottomBarSource, /saveShellBarHeight\('bottom', nextHeight\)/);
  assert.match(bottomBarSource, /bottomBarHeightLogical = nextHeight;\s*bottomBarResizeScheduler\.schedule\(nextHeight\)/);
  assert.match(bottomBarSource, /bottomBarResizeScheduler\.flush\(nextHeight\)/);
});

test('height persistence and locks use merge-safe backend commands instead of whole settings saves', () => {
  assert.match(settingsPanelSource, /updateShellBarLock\('top', lockTopBarHeight\)/);
  assert.match(settingsPanelSource, /updateShellBarLock\('bottom', lockBottomBarHeight\)/);
  assert.match(settingsPanelSource, /saveShellBarLock\(edge, locked\)/);
  assert.match(topBarSource, /saveShellBarHeight\('top', nextHeight\)/);
  assert.doesNotMatch(topBarSource, /saveShellSettings\(shellSettings\)[\s\S]*Failed to save top bar height/);
  assert.match(bottomBarSource, /saveShellBarHeight\('bottom', nextHeight\)/);
  assert.doesNotMatch(bottomBarSource, /saveShellSettings\(shellSettings\)[\s\S]*Failed to save bottom bar height/);
  assert.match(mainSource, /settings::save_shell_bar_height/);
  assert.match(mainSource, /settings::save_shell_bar_lock/);
});

test('settings panel exposes lock toggles for both shell bars', () => {
  assert.match(settingsPanelSource, /label="Lock top bar"/);
  assert.match(settingsPanelSource, /label="Lock bottom bar"/);
  assert.match(settingsPanelSource, /lockTopBarHeight/);
  assert.match(settingsPanelSource, /lockBottomBarHeight/);
  assert.match(settingsPanelSource, /updateShellBarLock\('top', lockTopBarHeight\)/);
  assert.match(settingsPanelSource, /updateShellBarLock\('bottom', lockBottomBarHeight\)/);
});

test('top and bottom bars gate visible resize handles behind unlocked settings', () => {
  assert.match(topBarSource, /topBarHeightLocked = settings\.ui\.lockTopBarHeight/);
  assert.match(topBarSource, /{#if !topBarHeightLocked}/);
  assert.match(topBarSource, /class="bar-resize-handle top-bar-resize-handle"/);
  assert.match(topBarSource, /shellBarHeightFromDrag\(\s*'top'/);
  assert.match(topBarSource, /resizeShellBar\(\{ edge: 'top'/);
  assert.match(topBarSource, /saveShellBarHeight\('top', nextHeight\)/);

  assert.match(bottomBarSource, /bottomBarHeightLocked = settings\.ui\.lockBottomBarHeight/);
  assert.match(bottomBarSource, /{#if !bottomBarHeightLocked}/);
  assert.match(bottomBarSource, /class="bar-resize-handle bottom-bar-resize-handle"/);
  assert.match(bottomBarSource, /shellBarHeightFromDrag\(\s*'bottom'/);
  assert.match(bottomBarSource, /resizeShellBar\(\{ edge: 'bottom'/);
  assert.match(bottomBarSource, /saveShellBarHeight\('bottom', nextHeight\)/);
});

test('backend resize command re-reserves appbars and updates work area', () => {
  assert.match(mainSource, /appbar::resize_shell_bar/);
  assert.match(appbarSource, /pub fn resize_shell_bar/);
  assert.match(appbarSource, /reserve_appbar\(top_hwnd, AppBarEdge::Top/);
  assert.match(appbarSource, /reserve_appbar\(bottom_hwnd, AppBarEdge::Bottom/);
  assert.match(appbarSource, /reserved shell work area after shell bar resize/);
});
