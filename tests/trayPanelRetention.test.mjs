import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const trayPanelSource = readFileSync(new URL('../src/components/TrayPanelSurface.svelte', import.meta.url), 'utf8');
const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const systemTrayRs = readFileSync(new URL('../src-tauri/src/system_tray.rs', import.meta.url), 'utf8');
const trayPanelRs = readFileSync(new URL('../src-tauri/src/tray_panel.rs', import.meta.url), 'utf8');
const mainRs = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

function functionBody(source, name) {
  let start = source.indexOf(`function ${name}`);
  if (start === -1) {
    start = source.indexOf(`fn ${name}`);
  }
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

test('native tray icon invoke suppresses the next tray focus-loss close only', () => {
  assert.match(trayPanelRs, /const TRAY_PANEL_FOCUS_LOSS_SUPPRESSION_TTL_MS:\s*u64\s*=\s*1_000/);
  assert.match(trayPanelRs, /static SUPPRESS_TRAY_PANEL_FOCUS_LOSS_UNTIL_MS:\s*AtomicU64/);
  assert.match(trayPanelRs, /pub fn suppress_next_tray_panel_focus_loss\(\)[\s\S]*saturating_add\(TRAY_PANEL_FOCUS_LOSS_SUPPRESSION_TTL_MS\)[\s\S]*store\(expires_at, Ordering::SeqCst\)/);
  assert.match(trayPanelRs, /pub fn take_tray_panel_focus_loss_suppression\(\) -> bool[\s\S]*swap\(0, Ordering::SeqCst\)[\s\S]*current_time_millis\(\) <= expires_at/);
  assert.doesNotMatch(functionBody(systemTrayRs, 'invoke_system_tray_icon'), /suppress_next_tray_panel_focus_loss/);
  assert.match(systemTrayRs, /fn click_toolbar_button[\s\S]*crate::tray_panel::suppress_next_tray_panel_focus_loss\(\);[\s\S]*SetForegroundWindow\(button\.toolbar\)/);
  assert.match(mainRs, /TRAY_PANEL_LABEL[\s\S]*WindowEvent::Focused\(false\)[\s\S]*take_tray_panel_focus_loss_suppression\(\)[\s\S]*return;[\s\S]*TRAY_PANEL_CLOSED_EVENT[\s\S]*window\.hide\(\)/);
  assert.match(trayPanelRs, /fn clear_tray_panel_focus_loss_suppression\(\)[\s\S]*store\(0, Ordering::SeqCst\)/);
  assert.match(trayPanelRs, /pub fn hide_tray_panel[\s\S]*clear_tray_panel_focus_loss_suppression\(\)/);
  assert.match(trayPanelRs, /tray_focus_loss_suppression_is_one_shot/);
  assert.match(trayPanelRs, /expired_tray_focus_loss_suppression_does_not_hide_future_focus_loss/);
});
