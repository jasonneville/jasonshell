import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function readSource(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

test('Ctrl+Space search hotkey emits centered search toggle event to top-bar only', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');

  assert.match(rust, /pub const SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT: &str = "search:toggle-centered";/);
  assert.match(rust, /emit_to\(\s*crate::shell_windows::TOP_BAR_LABEL,\s*SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT/);
  assert.doesNotMatch(rust, /SEARCH_PANEL_LABEL,\s*SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT/);
});

test('TopBar listens for native Ctrl+Space search toggle through existing centered paths', () => {
  const source = readSource('../src/components/TopBar.svelte');

  assert.match(source, /const SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT = 'search:toggle-centered';/);
  assert.match(source, /listen\(SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT, \(\) => \{/);
  assert.match(source, /function toggleCenteredSearchFromHotkey\(\)/);
  assert.match(source, /if \(searchOpen\) \{\s*void closePanel\(\);/);
  assert.match(source, /void openCenteredPanel\(\{ publishCurrentPayload: true \}\)/);
  assert.match(source, /void tick\(\)\.then\(\(\) => searchInput\?\.focus\(\{ preventScroll: true \}\)\)/);
});

test('top and bottom shell surfaces catch Ctrl+Space when their webviews have focus', () => {
  const topBar = readSource('../src/components/TopBar.svelte');
  const bottomBar = readSource('../src/components/BottomBar.svelte');
  const searchPanel = readSource('../src/components/SearchPanelSurface.svelte');

  assert.match(topBar, /window\.addEventListener\('keydown', keydownHandler, true\)/);
  assert.match(topBar, /function isSpaceKey\(event: KeyboardEvent\)/);
  assert.match(topBar, /!event\.ctrlKey && !shellSurfaceHotkeyHandled/);
  assert.match(topBar, /window\.removeEventListener\('keydown', keydownHandler, true\)/);
  assert.match(bottomBar, /import \{ emit, listen \} from '@tauri-apps\/api\/event';/);
  assert.match(bottomBar, /void emit\(SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT\)/);
  assert.match(bottomBar, /function isSpaceKey\(event: KeyboardEvent\)/);
  assert.match(bottomBar, /!event\.ctrlKey && !shellSurfaceHotkeyHandled/);
  assert.match(bottomBar, /window\.addEventListener\('keydown', keydownHandler, true\)/);
  assert.match(bottomBar, /window\.removeEventListener\('keydown', keydownHandler, true\)/);
  assert.match(searchPanel, /function closeCenteredPanelFromHotkey\(\)/);
  assert.match(searchPanel, /hideCenteredPanelImmediately\(\);/);
  assert.match(searchPanel, /if \(isCtrlSpaceHotkey\(event\)\) \{/);
  assert.doesNotMatch(searchPanel, /emitTo\(topBarTarget, SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT/);
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

test('native Ctrl+Space hotkey toggles once and does not capture Windows key', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');

  assert.match(rust, /ctrl_space_toggles_search_and_suppresses_space/);
  assert.match(rust, /repeated_space_down_does_not_duplicate_open_search/);
  assert.match(rust, /async_control_state_opens_when_control_down_was_not_observed/);
  assert.match(rust, /released_control_state_passes_through_stale_classifier_control/);
  assert.match(rust, /GetAsyncKeyState/);
  assert.match(rust, /VK_SPACE/);
  assert.match(rust, /VK_LCONTROL/);
  assert.match(rust, /VK_RCONTROL/);
  assert.doesNotMatch(rust, /VK_LWIN|VK_RWIN|LeftWin|RightWin/);
});
