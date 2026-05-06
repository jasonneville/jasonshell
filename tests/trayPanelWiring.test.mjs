import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const appSource = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const topBarCss = readFileSync(new URL('../src/components/TopBar.css', import.meta.url), 'utf8');
const trayPanelSource = readFileSync(new URL('../src/components/TrayPanelSurface.svelte', import.meta.url), 'utf8');
const trayPanelCss = readFileSync(new URL('../src/components/TrayPanelSurface.css', import.meta.url), 'utf8');
const shellSurfaceSource = readFileSync(new URL('../src/lib/shellSurface.ts', import.meta.url), 'utf8');
const ipcSurfacesSource = readFileSync(new URL('../src/ipc/surfaces.ts', import.meta.url), 'utf8');
const ipcEventsSource = readFileSync(new URL('../src/ipc/events.ts', import.meta.url), 'utf8');
const ipcCommandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const trayPanelWrapper = readFileSync(new URL('../src/lib/trayPanel.ts', import.meta.url), 'utf8');
const shellWindowsSource = readFileSync(new URL('../src-tauri/src/shell_windows.rs', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const trayPanelRs = readFileSync(new URL('../src-tauri/src/tray_panel.rs', import.meta.url), 'utf8');
const contractsSource = readFileSync(new URL('../src-tauri/src/contracts.rs', import.meta.url), 'utf8');
const capabilitySource = readFileSync(new URL('../src-tauri/capabilities/tray-panel.json', import.meta.url), 'utf8');

test('tray panel is routed as a dedicated auxiliary shell surface', () => {
  assert.match(appSource, /import TrayPanelSurface/);
  assert.match(appSource, /surface === 'tray-panel'[\s\S]*<TrayPanelSurface \/>/);
  assert.match(shellSurfaceSource, /\| 'tray-panel'/);
  assert.match(shellWindowsSource, /TRAY_PANEL_LABEL: &str = "tray-panel"/);
  assert.match(shellWindowsSource, /TRAY_PANEL_WIDTH_LOGICAL: f64 = 252\.0/);
  assert.match(shellWindowsSource, /TRAY_PANEL_HEIGHT_LOGICAL: f64 = 220\.0/);
  assert.match(shellWindowsSource, /build_tray_panel_window\(app\)/);
  assert.match(mainSource, /mod tray_panel;/);
  assert.match(mainSource, /tray_panel::show_tray_panel/);
  assert.match(mainSource, /tray_panel::hide_tray_panel/);
  assert.match(mainSource, /shell_windows::TRAY_PANEL_LABEL[\s\S]*WindowEvent::Focused\(false\)/);
  assert.match(mainSource, /emit_to\(\s*shell_windows::TOP_BAR_LABEL,\s*tray_panel::TRAY_PANEL_CLOSED_EVENT/);
  assert.match(trayPanelRs, /pub fn show_tray_panel/);
  assert.match(trayPanelRs, /pub fn hide_tray_panel/);
  assert.match(capabilitySource, /"tray-panel"/);
});

test('tray panel contracts and wrappers use constant-backed IPC/event names', () => {
  assert.match(ipcCommandsSource, /showTrayPanel: 'show_tray_panel'/);
  assert.match(ipcCommandsSource, /hideTrayPanel: 'hide_tray_panel'/);
  assert.match(ipcEventsSource, /trayPanelClosed: 'tray-panel:closed'/);
  assert.match(ipcSurfacesSource, /trayPanel: 'tray-panel'/);
  assert.match(contractsSource, /TRAY_PANEL/);
  assert.match(contractsSource, /SHOW_TRAY_PANEL/);
  assert.match(contractsSource, /HIDE_TRAY_PANEL/);
  assert.match(contractsSource, /TRAY_PANEL_CLOSED/);
  assert.match(trayPanelWrapper, /invoke\(IPC_COMMANDS\.showTrayPanel/);
  assert.match(trayPanelWrapper, /invoke\(IPC_COMMANDS\.hideTrayPanel/);
  assert.match(trayPanelWrapper, /listSystemTrayIcons\(\)/);
  assert.match(trayPanelWrapper, /invokeSystemTrayIcon\(/);
  assert.doesNotMatch(trayPanelWrapper, /invoke\('show_tray_panel'/);
  assert.doesNotMatch(trayPanelWrapper, /invoke\('hide_tray_panel'/);
});

test('top bar exposes tray button left of sound and keeps popup state mutually exclusive', () => {
  assert.match(topBarSource, /from '\.\.\/lib\/trayPanel'/);
  assert.match(topBarSource, /TRAY_PANEL_CLOSED_EVENT/);
  assert.match(topBarSource, /class="tray-control"[\s\S]*class="sound-control"/);
  assert.match(topBarSource, /class="tray-button"[\s\S]*ariaControls=\{TRAY_PANEL_ID\}/);
  assert.match(topBarSource, /ariaLabel="Open notification area icons"/);
  assert.match(topBarSource, /ariaHaspopup="dialog"/);
  assert.match(topBarSource, /await closePanel\(\);[\s\S]*await closeAudioPanel\(\);[\s\S]*await showTrayPanel\(\{/);
  assert.match(topBarSource, /if \(trayOpen && \(!target \|\| !trayControl\?\.contains\(target\)\)\) \{[\s\S]*void closeTrayPanel\(\);/);
  assert.match(topBarSource, /(?:void listen|registerAsyncUnlistener\(listen)\(TRAY_PANEL_CLOSED_EVENT, \(\) => \{[\s\S]*trayOpen = false;/);
  assert.doesNotMatch(topBarSource, /role="menu"/);
  assert.match(topBarCss, /\.top-bar \.tray-button \{/);
});

test('tray panel surface renders icon-only grid with loading, error, and empty states', () => {
  assert.match(trayPanelSource, /id="tray-panel" role="dialog"/);
  assert.match(trayPanelSource, /class="tray-content"[\s\S]*class="tray-grid"/);
  assert.match(trayPanelSource, /Loading notification icons[\s\S]*No notification icons are currently available[\s\S]*class="tray-content"/);
  assert.match(trayPanelSource, /class="tray-grid"/);
  assert.match(trayPanelSource, /on:click=\{\(\) => void triggerTrayIcon\(icon, 'left'\)\}/);
  assert.match(trayPanelSource, /on:contextmenu=\{\(event\) => handleTrayContextMenu\(event, icon\)\}/);
  assert.match(trayPanelSource, /aria-label=\{icon\.label\}/);
  assert.match(trayPanelSource, /title=\{icon\.label\}/);
  assert.match(trayPanelSource, /Loading notification icons/);
  assert.match(trayPanelSource, /Notification area is unavailable/);
  assert.match(trayPanelSource, /No notification icons are currently available/);
  assert.doesNotMatch(trayPanelSource, /<span class="tray-label">/);
  assert.match(trayPanelCss, /\.tray-panel \{[\s\S]*height:\s*100%;[\s\S]*overflow:\s*hidden;/);
  assert.match(trayPanelCss, /\.tray-content \{[\s\S]*flex:\s*1 1 auto;[\s\S]*min-height:\s*0;[\s\S]*overflow:\s*auto;/);
  assert.match(trayPanelCss, /\.tray-grid \{/);
});

test('tray panel Rust placement anchors to right edge and clamps within top bar bounds', () => {
  assert.match(trayPanelRs, /anchors_tray_panel_to_button_right_edge/);
  assert.match(trayPanelRs, /clamps_tray_panel_inside_top_bar_edges/);
  assert.match(trayPanelRs, /TRAY_PANEL_EDGE_PADDING_PHYSICAL/);
  assert.match(trayPanelRs, /TRAY_PANEL_MARGIN_PHYSICAL/);
  assert.match(trayPanelRs, /emit_to\(TOP_BAR_LABEL, TRAY_PANEL_CLOSED_EVENT/);
});
