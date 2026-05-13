import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function readSource(path) {
  return readFileSync(path, 'utf8');
}

const shellWindows = readSource('src-tauri/src/shell_windows.rs');

const auxiliaryLabels = [
  'task-preview',
  'search-panel',
  'stack-popup',
  'process-manager',
  'control-plane',
  'settings-panel',
  'tray-panel',
  'terminal-panel',
  'command-panel',
  'audio-panel',
  'calendar-panel'
];

const panelSources = [
  'src-tauri/src/search_panel.rs',
  'src-tauri/src/stack_popup/popup_window.rs',
  'src-tauri/src/process_manager.rs',
  'src-tauri/src/terminal_panel.rs',
  'src-tauri/src/settings_panel.rs',
  'src-tauri/src/tray_panel.rs',
  'src-tauri/src/command_panel.rs',
  'src-tauri/src/audio_panel.rs',
  'src-tauri/src/calendar_panel.rs',
  'src-tauri/src/control_plane.rs',
  'src-tauri/src/task_preview.rs'
];

test('startup shell creation only builds top and bottom AppBar labels', () => {
  const createStart = shellWindows.indexOf('pub fn create_shell_windows');
  const helperStart = shellWindows.indexOf('pub fn is_startup_label', createStart);
  assert.notEqual(createStart, -1, 'create_shell_windows should exist');
  assert.notEqual(helperStart, -1, 'label helpers should follow create_shell_windows');
  const createBody = shellWindows.slice(createStart, helperStart);

  assert.match(createBody, /TOP_BAR_LABEL/);
  assert.match(createBody, /BOTTOM_BAR_LABEL/);
  for (const label of auxiliaryLabels) {
    assert.doesNotMatch(createBody, new RegExp(label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  }
});

test('lazy shell registry keeps startup and auxiliary labels distinct without dropping capabilities', () => {
  assert.match(shellWindows, /pub const STARTUP_LABELS: &\[&str\] = &\[TOP_BAR_LABEL, BOTTOM_BAR_LABEL\];/);
  for (const label of auxiliaryLabels) {
    assert.match(shellWindows, new RegExp(`${label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`));
  }

  const capabilitySources = auxiliaryLabels.map((label) => {
    const path = `src-tauri/capabilities/${label}.json`;
    return JSON.parse(readSource(path));
  });
  assert.deepEqual(capabilitySources.map((capability) => capability.windows[0]).sort(), auxiliaryLabels.toSorted());
});

test('auxiliary panel show and publish paths route through central lazy creation helper', () => {
  for (const path of panelSources) {
    const source = readSource(path);
    assert.match(source, /ensure_shell_window\(&app_handle, [A-Z_]+_LABEL\)/, `${path} should lazily ensure its auxiliary window`);
  }
});

test('hide-only paths remain idempotent for windows that have not been lazy-created', () => {
  for (const path of panelSources) {
    const source = readSource(path);
    const hideMatches = source.match(/pub(?:\(crate\))? fn hide_[\s\S]*?\n}/g) ?? [];
    for (const hideFunction of hideMatches) {
      assert.doesNotMatch(hideFunction, /ensure_shell_window\(&app_handle/, `${path} hide path should not create an auxiliary window`);
    }
  }
});
