import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function readSource(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

test('Windows-key hook emits centered search open event to top-bar only', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');

  assert.match(rust, /pub const WINDOWS_KEY_OPEN_SEARCH_EVENT: &str = "search:open-centered";/);
  assert.match(rust, /emit_to\(\s*crate::shell_windows::TOP_BAR_LABEL,\s*WINDOWS_KEY_OPEN_SEARCH_EVENT/);
  assert.doesNotMatch(rust, /SEARCH_PANEL_LABEL,\s*WINDOWS_KEY_OPEN_SEARCH_EVENT/);
});

test('TopBar listens for native Windows-key search open through existing centered path', () => {
  const source = readSource('../src/components/TopBar.svelte');

  assert.match(source, /const WINDOWS_KEY_OPEN_SEARCH_EVENT = 'search:open-centered';/);
  assert.match(source, /listen\(WINDOWS_KEY_OPEN_SEARCH_EVENT, \(\) => \{/);
  assert.match(source, /void openCenteredPanel\(\{ publishCurrentPayload: true \}\)/);
  assert.match(source, /void tick\(\)\.then\(\(\) => searchInput\?\.focus\(\{ preventScroll: true \}\)\)/);
});

test('native hook installs during setup and cleans up on exit', () => {
  const main = readSource('../src-tauri/src/main.rs');

  assert.match(main, /windows_key_hook::install_windows_key_hook\(app\.handle\(\)\.clone\(\)\)/);
  assert.match(main, /windows_key_hook::uninstall_windows_key_hook\(\)/);
});

test('startup fails when required Windows-key hook cannot install', () => {
  const main = readSource('../src-tauri/src/main.rs');

  assert.doesNotMatch(main, /Windows-key hook disabled/);
  assert.match(main, /Windows-key hook is required to suppress the Start Menu: \{error\}/);
  assert.match(main, /windows_key_hook::install_windows_key_hook\(app\.handle\(\)\.clone\(\)\)\s*\.map_err\(/);
});

test('native hook fails closed for Windows-key events when hook state is unavailable', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');

  assert.match(rust, /left_win_down: bool/);
  assert.match(rust, /right_win_down: bool/);
  assert.doesNotMatch(rust, /\bwin_down: bool/);
  assert.match(rust, /pub fn unavailable_hook_state_decision\(event: WindowsKeyEvent\) -> WindowsKeyDecision/);
  assert.match(rust, /if is_windows_key\(event\.key\) \{\s*WindowsKeyDecision::Suppress\s*\} else \{\s*WindowsKeyDecision::PassThrough\s*\}/s);
  assert.match(rust, /\(unavailable_hook_state_decision\(event\), None\)/);
});

test('native Windows-key chords preserve the OS modifier down and release path', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const plan = readSource('../windows_key_chord_preservation_p3.md');

  assert.match(plan, /Native chords such as `Win\+R`, `Win\+D`, `Win\+E`, and `Win\+L` must continue to reach Windows/);
  assert.match(rust, /native_windows_chords_preserve_modifier_down_and_release_paths/);
  assert.match(rust, /WindowsKeyCode::Other\(u32::from\(key\)\)/);
  assert.doesNotMatch(
    rust,
    /WindowsKeyCode::LeftWin \| WindowsKeyCode::RightWin, WindowsKeyEventKind::KeyDown\)[\s\S]{0,260}WindowsKeyDecision::Suppress/
  );
});
