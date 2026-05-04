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
