import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function readSource(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

test('Ctrl+Space search hotkey emits centered search open event to top-bar only', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');

  assert.match(rust, /pub const SEARCH_HOTKEY_OPEN_SEARCH_EVENT: &str = "search:open-centered";/);
  assert.match(rust, /emit_to\(\s*crate::shell_windows::TOP_BAR_LABEL,\s*SEARCH_HOTKEY_OPEN_SEARCH_EVENT/);
  assert.doesNotMatch(rust, /SEARCH_PANEL_LABEL,\s*SEARCH_HOTKEY_OPEN_SEARCH_EVENT/);
});

test('TopBar listens for native Ctrl+Space search open through existing centered path', () => {
  const source = readSource('../src/components/TopBar.svelte');

  assert.match(source, /const SEARCH_HOTKEY_OPEN_SEARCH_EVENT = 'search:open-centered';/);
  assert.match(source, /listen\(SEARCH_HOTKEY_OPEN_SEARCH_EVENT, \(\) => \{/);
  assert.match(source, /void openCenteredPanel\(\{ publishCurrentPayload: true \}\)/);
  assert.match(source, /void tick\(\)\.then\(\(\) => searchInput\?\.focus\(\{ preventScroll: true \}\)\)/);
});

test('native hook installs during setup and cleans up on exit', () => {
  const main = readSource('../src-tauri/src/main.rs');

  assert.match(main, /windows_key_hook::install_windows_key_hook\(app\.handle\(\)\.clone\(\)\)/);
  assert.match(main, /windows_key_hook::uninstall_windows_key_hook\(\)/);
});

test('startup fails when required search hotkey hook cannot install', () => {
  const main = readSource('../src-tauri/src/main.rs');

  assert.doesNotMatch(main, /search hotkey hook disabled/);
  assert.match(main, /search hotkey hook is required: \{error\}/);
  assert.match(main, /windows_key_hook::install_windows_key_hook\(app\.handle\(\)\.clone\(\)\)\s*\.map_err\(/);
});

test('native hook passes through when hook state is unavailable', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');

  assert.match(rust, /left_control_down: bool/);
  assert.match(rust, /right_control_down: bool/);
  assert.doesNotMatch(rust, /\bwin_down: bool/);
  assert.match(rust, /pub fn unavailable_hook_state_decision\(_event: SearchHotkeyEvent\) -> SearchHotkeyDecision/);
  assert.match(rust, /SearchHotkeyDecision::PassThrough/);
  assert.match(rust, /\(unavailable_hook_state_decision\(event\), None\)/);
});

test('native Ctrl+Space hotkey opens once and does not capture Windows key', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');

  assert.match(rust, /ctrl_space_opens_search_and_suppresses_space/);
  assert.match(rust, /repeated_space_down_does_not_duplicate_open_search/);
  assert.match(rust, /VK_SPACE/);
  assert.match(rust, /VK_LCONTROL/);
  assert.match(rust, /VK_RCONTROL/);
  assert.doesNotMatch(rust, /VK_LWIN|VK_RWIN|LeftWin|RightWin/);
});
