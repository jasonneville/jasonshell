import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const trayPanelSource = readFileSync(new URL('../src/components/TrayPanelSurface.svelte', import.meta.url), 'utf8');
const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const systemTrayRs = readFileSync(new URL('../src-tauri/src/system_tray.rs', import.meta.url), 'utf8');

function functionBody(source, name) {
  const start = source.indexOf(`function ${name}`);
  assert.notEqual(start, -1, `${name} exists`);
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let index = braceStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0) return source.slice(braceStart + 1, index);
  }
  assert.fail(`${name} body closes`);
}

test('tray icon invoke has local retention guard and never closes tray panel', () => {
  const triggerBody = functionBody(trayPanelSource, 'triggerTrayIcon');

  assert.match(trayPanelSource, /let isInvokingTrayIcon\s*=\s*false/);
  assert.match(triggerBody, /if \(isInvokingTrayIcon\)/);
  assert.match(triggerBody, /isInvokingTrayIcon\s*=\s*true/);
  assert.match(triggerBody, /await invokeTrayPanelIcon\(icon\.id, button\)/);
  assert.match(triggerBody, /isInvokingTrayIcon\s*=\s*false/);
  assert.doesNotMatch(triggerBody, /hideTrayPanel\s*\(/);
  assert.doesNotMatch(triggerBody, /tray-panel:closed|TRAY_PANEL_CLOSED_EVENT/);
  assert.doesNotMatch(triggerBody, /dispatch\([^)]*closed|createEventDispatcher/);
});

test('tray invoke failure stores inline invoke error and keeps snapshot visible', () => {
  const triggerBody = functionBody(trayPanelSource, 'triggerTrayIcon');

  assert.match(trayPanelSource, /let invokeError\s*=\s*''/);
  assert.match(triggerBody, /invokeError\s*=\s*''/);
  assert.match(triggerBody, /catch \(error\)[\s\S]*invokeError\s*=/);
  assert.doesNotMatch(triggerBody, /catch \(error\)[\s\S]*icons\s*=\s*\[\]/);
  assert.match(trayPanelSource, /invokeError[\s\S]*role="alert"/);
});

test('rapid left and right tray icon activation share same invoke guard', () => {
  const triggerBody = functionBody(trayPanelSource, 'triggerTrayIcon');

  assert.match(trayPanelSource, /on:click=\{\(\) => void triggerTrayIcon\(icon, 'left'\)\}/);
  assert.match(trayPanelSource, /on:contextmenu=\{\(event\) => handleTrayContextMenu\(event, icon\)\}/);
  assert.match(functionBody(trayPanelSource, 'handleTrayContextMenu'), /void triggerTrayIcon\(icon, 'right'\)/);
  assert.doesNotMatch(triggerBody, /button\s*===\s*['"]left['"][\s\S]*isInvokingTrayIcon/);
  assert.doesNotMatch(triggerBody, /button\s*===\s*['"]right['"][\s\S]*isInvokingTrayIcon/);
});

test('top bar only observes tray-panel closed event, never tray icon invoke outcome', () => {
  assert.match(topBarSource, /(?:void listen|registerAsyncUnlistener\(listen)\(TRAY_PANEL_CLOSED_EVENT, \(\) => \{[\s\S]*trayOpen = false;/);
  assert.doesNotMatch(topBarSource, /invokeSystemTrayIcon|invokeTrayPanelIcon|trayClickRequest/);

  const closeTrayPanelBody = functionBody(topBarSource, 'closeTrayPanel');
  assert.match(closeTrayPanelBody, /trayOpen\s*=\s*false/);
  assert.match(closeTrayPanelBody, /hideTrayPanel\(\)/);
});

test('system tray backend has explicit no-panel-close error contract for stale invoke', () => {
  assert.match(systemTrayRs, /stale_tray_icon_invoke_returns_error_without_panel_close_payload/);
  assert.match(systemTrayRs, /Invalid tray icon id/);
  assert.doesNotMatch(systemTrayRs, /tray-panel:closed[\s\S]*invoke_system_tray_icon/);
});
