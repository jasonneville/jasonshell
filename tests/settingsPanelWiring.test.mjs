import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const appSource = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const settingsPanelSource = readFileSync(new URL('../src/components/SettingsPanelSurface.svelte', import.meta.url), 'utf8');
const settingsPanelCss = readFileSync(new URL('../src/components/SettingsPanelSurface.css', import.meta.url), 'utf8');
const shellSurfaceSource = readFileSync(new URL('../src/lib/shellSurface.ts', import.meta.url), 'utf8');
const ipcCommandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const wrapperSource = readFileSync(new URL('../src/lib/settingsPanel.ts', import.meta.url), 'utf8');
const shellWindowsSource = readFileSync(new URL('../src-tauri/src/shell_windows.rs', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const settingsPanelRs = readFileSync(new URL('../src-tauri/src/settings_panel.rs', import.meta.url), 'utf8');
const contractsSource = readFileSync(new URL('../src-tauri/src/contracts.rs', import.meta.url), 'utf8');
const capabilitySource = readFileSync(new URL('../src-tauri/capabilities/settings-panel.json', import.meta.url), 'utf8');

test('settings panel is routed as an anchored auxiliary shell surface', () => {
  assert.match(shellSurfaceSource, /'settings-panel'/);
  assert.match(appSource, /SettingsPanelSurface/);
  assert.match(shellWindowsSource, /SETTINGS_PANEL_LABEL: &str = "settings-panel"/);
  assert.match(shellWindowsSource, /build_settings_panel_window/);
  assert.match(mainSource, /settings_panel::show_settings_panel/);
  assert.match(mainSource, /SETTINGS_PANEL_LABEL[\s\S]*WindowEvent::Focused\(false\)/);
  assert.match(contractsSource, /SETTINGS_PANEL/);
  assert.match(capabilitySource, /"settings-panel"/);
});

test('top-left JasonShell button opens settings instead of search', () => {
  assert.match(topBarSource, /showSettingsPanel/);
  assert.match(topBarSource, /aria-label="Open JasonShell settings"/);
  assert.match(topBarSource, /aria-haspopup="dialog"/);
  assert.doesNotMatch(topBarSource, /JasonShell Home: open command search/);
  assert.match(wrapperSource, /invoke\(IPC_COMMANDS\.showSettingsPanel/);
  assert.match(ipcCommandsSource, /showSettingsPanel: 'show_settings_panel'/);
});

test('settings panel exposes live theme, font, date, clock, and useful UI preferences', () => {
  for (const text of [
    'Theme',
    'Font',
    'Date format',
    'Compact density',
    'Strong focus rings',
    'Reduce transparency',
    'Search shortcut hint',
    '24-hour time',
    'Show seconds'
  ]) {
    assert.match(settingsPanelSource, new RegExp(text));
  }
  assert.match(settingsPanelSource, /setShellTheme\(selectedThemeId\)/);
  assert.match(settingsPanelSource, /patchShellPreferences/);
  assert.match(settingsPanelSource, /formatShellDate/);
  assert.match(settingsPanelSource, /formatShellTime/);
  assert.match(settingsPanelCss, /settings-panel/);
});

test('settings panel Rust placement clamps to the top-bar host bounds', () => {
  assert.match(settingsPanelRs, /anchors_settings_panel_to_button_left_edge/);
  assert.match(settingsPanelRs, /clamps_settings_panel_inside_top_bar_edges/);
  assert.match(settingsPanelRs, /SETTINGS_PANEL_EDGE_PADDING_PHYSICAL/);
});
