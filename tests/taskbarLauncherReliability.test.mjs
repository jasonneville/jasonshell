import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const launchersRs = readFileSync(new URL('../src-tauri/src/launchers.rs', import.meta.url), 'utf8');
const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const mainRs = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

function extractFunction(source, name, occurrence = 0) {
  const pattern = new RegExp(`(?:pub\\s+)?(?:async\\s+)?function\\s+${name}|(?:pub\\s+)?fn\\s+${name}`, 'g');
  let match = null;
  for (let index = 0; index <= occurrence; index += 1) {
    match = pattern.exec(source);
    if (!match) {
      assert.fail(`${name} occurrence ${occurrence} should exist`);
    }
  }
  const start = match.index;
  const braceStart = source.indexOf('{', start);
  assert.notEqual(braceStart, -1, `${name} should have a body`);
  let depth = 0;
  for (let index = braceStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') {
      depth += 1;
    } else if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        return source.slice(braceStart + 1, index);
      }
    }
  }
  assert.fail(`${name} body should close`);
}

test('Explorer taskbar pin listing does not hide shortcuts just because Resolve fails', () => {
  const listBody = extractFunction(launchersRs, 'list_pinned_taskbar_apps', 1);

  assert.doesNotMatch(listBody, /if\s+!\s*shortcut_resolves[\s\S]*continue/);
  assert.match(listBody, /fallback_launcher_icon_data_url/);
});

test('Explorer taskbar pin launch ShellExecutes the shortcut path without Resolve preflight', () => {
  const launchBody = extractFunction(launchersRs, 'launch_pinned_taskbar_app', 1);

  assert.doesNotMatch(launchBody, /shortcut_resolves/);
  assert.match(launchBody, /shell_execute_shortcut/);
});

test('Explorer taskbar pin launch retries with elevation on access denied', () => {
  const launchBody = extractFunction(launchersRs, 'launch_pinned_taskbar_app', 1);
  const helperBody = extractFunction(launchersRs, 'shell_execute_shortcut');

  assert.match(launchBody, /Some\("runas"\)/);
  assert.match(launchBody, /SE_ERR_ACCESSDENIED/);
  assert.match(helperBody, /Ok\(code\)/);
});

test('Explorer launch failure does not remove the visible launcher row', () => {
  const launchAppBody = extractFunction(bottomBarSource, 'launchApp');

  assert.doesNotMatch(launchAppBody, /launchers\s*=\s*launchers\.filter/);
  assert.match(launchAppBody, /Launch unavailable/);
});

test('app-managed quick icon backend commands are no longer registered', () => {
  assert.doesNotMatch(mainRs, /mod quick_icons/);
  assert.doesNotMatch(mainRs, /quick_icons::/);
});

test('Explorer launcher context menu exposes Windows taskbar unpin only through validated lnk path', () => {
  const taskbarMenuRs = readFileSync(new URL('../src-tauri/src/taskbar_menu.rs', import.meta.url), 'utf8');

  assert.match(taskbarMenuRs, /LAUNCHER_MENU_PREFIX\}:unpin/);
  assert.match(taskbarMenuRs, /"Unpin from taskbar"/);
  assert.match(taskbarMenuRs, /"unpin"\s*=>\s*launchers::unpin_pinned_taskbar_app/);
  assert.match(launchersRs, /pub fn unpin_pinned_taskbar_app\(shortcut_path: String\) -> Result<\(\), String>/);
  assert.match(launchersRs, /let shortcut_path = validate_shortcut_path\(&shortcut_path\)\?/);
  assert.match(launchersRs, /fs::remove_file\(&shortcut_path\)/);
});
