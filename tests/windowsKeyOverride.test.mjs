import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function readSource(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

test('global shortcuts use OS registration and never suppress raw keyboard events', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  assert.match(rust, /RegisterHotKey/);
  assert.match(rust, /UnregisterHotKey/);
  assert.match(rust, /MOD_NOREPEAT/);
  assert.doesNotMatch(rust, /WH_KEYBOARD_LL|SetWindowsHookExW|GetAsyncKeyState|CallNextHookEx|KBDLLHOOKSTRUCT|SearchHotkeyDecision::Suppress/);
});

test('native dedicated thread receives WM_HOTKEY in a message loop', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  assert.match(rust, /(?:thread::spawn|thread::Builder[\s\S]*?\.spawn)\s*\(/);
  assert.match(rust, /GetMessageW|PeekMessageW/);
  assert.match(rust, /WM_HOTKEY/);
  assert.match(rust, /PostThreadMessageW|PostMessageW/);
});

test('TopBar listens for native-only standard hotkey events through existing panel paths', () => {
  const source = readSource('../src/components/TopBar.svelte');

  assert.match(source, /const SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT = 'search:toggle-centered';/);
  assert.match(source, /const TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT = 'terminal:toggle-panel';/);
  assert.match(source, /const STACK_BROWSER_HOTKEY_TOGGLE_STACK_BROWSER_EVENT = 'stack-browser:toggle';/);
  assert.match(source, /listen\(SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT, \(\) => \{/);
  assert.match(source, /function toggleCenteredSearchFromHotkey\(\)/);
  assert.match(source, /if \(searchOpen\) \{\s*void closePanel\(\);/);
  assert.match(source, /void openCenteredPanel\(\{ publishCurrentPayload: true \}\)/);
  assert.match(source, /listen\(TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT, \(\) => \{\s*void toggleTerminalPanel\(terminalControl\);/);
  assert.match(source, /listen\(STACK_BROWSER_HOTKEY_TOGGLE_STACK_BROWSER_EVENT, \(\) => \{\s*void toggleStackBrowserFromHotkey\(\);/);
  assert.doesNotMatch(source, /let searchInput:|bind:this=\{searchInput\}/);
});

test('legacy frontend standard hotkey classifiers are intentionally absent from shell surfaces', () => {
  const surfaces = [
    '../src/components/TopBar.svelte',
    '../src/components/BottomBar.svelte',
    '../src/components/QuickLaunchPanelSurface.svelte',
    '../src/components/SearchPanelSurface.svelte',
    '../src/components/StackPopupSurface.svelte',
    '../src/components/CommandPanelSurface.svelte',
    '../src/components/TerminalPanelSurface.svelte'
  ];

  for (const path of surfaces) {
    const source = readSource(path);
    assert.doesNotMatch(source, /isCtrlSpaceHotkey|isAltBackquoteHotkey|isAltOneHotkey|shellSurfaceHotkeyHandled/);
    assert.doesNotMatch(source, /emit(?:To)?\([^)]*SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT/);
    assert.doesNotMatch(source, /emit(?:To)?\([^)]*TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT/);
    assert.doesNotMatch(source, /emit(?:To)?\([^)]*STACK_BROWSER_HOTKEY_TOGGLE_STACK_BROWSER_EVENT/);
  }
});

test('Alt+Backquote uses explicit layout-sensitive virtual key and existing terminal event', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const topBar = readSource('../src/components/TopBar.svelte');

  assert.match(rust, /TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT: &str = crate::contracts::events::TERMINAL_TOGGLE_PANEL/);
  assert.match(rust, /VK_OEM_3/);
  assert.match(rust, /MOD_ALT/);
  assert.match(rust, /emit_to\(\s*crate::shell_windows::TOP_BAR_LABEL,\s*crate::contracts::events::TERMINAL_TOGGLE_PANEL/);
  assert.match(topBar, /const TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT = 'terminal:toggle-panel';/);
  assert.match(topBar, /listen\(TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT, \(\) => \{\s*void toggleTerminalPanel\(terminalControl\);/);
  assert.doesNotMatch(topBar, /isAltBackquoteHotkey/);
});

test('Alt+1 toggles Stack Browser from native hotkey and top bar wiring', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const main = readSource('../src-tauri/src/main.rs');
  const contracts = readSource('../src-tauri/src/contracts.rs');
  const topBar = readSource('../src/components/TopBar.svelte');

  assert.match(rust, /STACK_BROWSER_HOTKEY_TOGGLE_STACK_BROWSER_EVENT/);
  assert.match(rust, /VK_1/);
  assert.match(main, /stack_popup::toggle_stack_popup,/);
  assert.match(contracts, /TOGGLE_STACK_POPUP: &str = "toggle_stack_popup"/);
  assert.match(contracts, /STACK_BROWSER_TOGGLE: &str = "stack-browser:toggle"/);
  assert.match(topBar, /const STACK_BROWSER_HOTKEY_TOGGLE_STACK_BROWSER_EVENT = 'stack-browser:toggle';/);
  assert.match(topBar, /listen\(STACK_BROWSER_HOTKEY_TOGGLE_STACK_BROWSER_EVENT, \(\) => \{/);
  assert.match(topBar, /toggleStackBrowserFromHotkey/);
  assert.match(topBar, /toggleStackBrowserPanel\(/);
  assert.match(topBar, /closest\('\.stack-browser-button'\)/);
  assert.match(topBar, /if \(stackBrowserOpen\) \{[\s\S]*await toggleStackPopup\(\)/);
  assert.match(topBar, /listen\(STACK_POPUP_CLOSED_EVENT, \(\) => \{\s*stackBrowserOpen = false;/);
  assert.doesNotMatch(topBar.slice(topBar.indexOf('async function toggleStackBrowserPanel('), topBar.indexOf('async function toggleStackBrowserFromHotkey(')), /await hideStackPopup\(/);
  assert.ok(topBar.indexOf('if (stackBrowserOpen)') < topBar.indexOf('const isOpen = await toggleStackPopup()'));
  assert.doesNotMatch(topBar, /isAltOneHotkey/);
});

test('top bar places Stack Browser toggle button between settings and pinned folders', () => {
  const source = readSource('../src/components/TopBar.svelte');

  const settingsIndex = source.indexOf('ariaLabel="Open JasonShell settings"');
  const stackBrowserIndex = source.indexOf('ariaLabel="Toggle Stack Browser"');
  const railWrapIndex = source.indexOf('<div class="rail-wrap">');

  assert.ok(settingsIndex !== -1 && stackBrowserIndex !== -1 && railWrapIndex !== -1);
  assert.ok(settingsIndex < stackBrowserIndex && stackBrowserIndex < railWrapIndex);
  assert.match(source, /ariaExpanded={stackBrowserOpen}/);
  assert.match(source, /ariaControls={STACK_POPUP_ID}/);
});

test('Stack Browser toggle reopens latest request without emitting open event', () => {
  const stackPopup = readSource('../src/lib/stackPopup.ts');
  const popupWindow = readSource('../src-tauri/src/stack_popup/popup_window.rs');

  assert.match(stackPopup, /toggleStackPopup/);
  assert.match(stackPopup, /getStackPopupRequest/);
  assert.match(stackPopup, /showStackPopup\(/);
  assert.match(stackPopup, /hideStackPopup\(/);
  assert.doesNotMatch(stackPopup, /toggleStackPopup[\s\S]*STACK_POPUP_OPEN_EVENT/);
  assert.match(popupWindow, /reopen_stack_popup_window[\s\S]*show_stack_popup_window_inner\([\s\S]*false/);
});

test('native hook installs during setup and cleans up on exit', () => {
  const main = readSource('../src-tauri/src/main.rs');

  assert.match(main, /let \(shell_settings, missing_snipping\) = settings::load_hotkey_startup_settings\(app\.handle\(\)\)/);
  assert.match(main, /windows_key_hook::install_loaded_windows_key_hook\(app\.handle\(\)\.clone\(\), shell_settings\.hotkeys, missing_snipping\)/);
  assert.match(main, /windows_key_hook::uninstall_windows_key_hook\(\)/);
});

test('Settings hotkey controls stay disabled until settings load succeeds', () => {
  const source = readSource('../src/components/SettingsPanelSurface.svelte');

  assert.match(source, /let shellSettingsLoaded = false;/);
  assert.match(source, /const settings = await loadShellSettings\(\);[\s\S]*shellSettings = settings;[\s\S]*shellSettingsLoaded = true;/);
  assert.match(source, /catch \(error\) \{[\s\S]*shellSettingsLoaded = false;/);
  assert.match(source, /async function saveHotkeys[\s\S]*if \(!shellSettingsLoaded \|\| hotkeyBusy\) return;/);
  assert.match(source, /function captureHotkeyBinding[\s\S]*if \(!action \|\| !shellSettingsLoaded \|\| hotkeyBusy \|\| event\.repeat\) return;/);
  assert.match(source, /data-hotkey-action="search"[\s\S]*disabled=\{!shellSettingsLoaded \|\| hotkeyBusy\}/);
  assert.match(source, /data-hotkey-action="terminal"[\s\S]*disabled=\{!shellSettingsLoaded \|\| hotkeyBusy\}/);
  assert.match(source, /data-hotkey-action="stackBrowser"[\s\S]*disabled=\{!shellSettingsLoaded \|\| hotkeyBusy\}/);
  assert.match(source, /class="hotkey-reset" disabled=\{!shellSettingsLoaded \|\| hotkeyBusy\}/);
});

test('Settings Stack Browser terminal selector cannot persist before authoritative settings load', () => {
  const source = readSource('../src/components/SettingsPanelSurface.svelte');
  const handlerStart = source.indexOf('async function handleStackTerminalProfileChange');
  const handlerEnd = source.indexOf('const hotkeyActionNames', handlerStart);
  const handler = source.slice(handlerStart, handlerEnd);
  const selectorStart = source.indexOf('<section class="settings-section" aria-labelledby="json-shell-heading">');
  const selectorEnd = source.indexOf('{#if settingsError}', selectorStart);
  const selector = source.slice(selectorStart, selectorEnd);

  assert.ok(handlerStart !== -1 && handlerEnd !== -1);
  assert.ok(selectorStart !== -1 && selectorEnd !== -1);
  assert.match(source, /let shellSettingsLoaded = false;/);
  assert.match(source, /selectedStackTerminalProfile = normalizeStackTerminalProfile\(settings\.stackBrowser\?\.terminalProfile\);[\s\S]*shellSettingsLoaded = true;/);
  assert.match(handler, /if \(!shellSettingsLoaded\) return;[\s\S]*selectedStackTerminalProfile = normalizeStackTerminalProfile\(value\);[\s\S]*saveShellSettings\(/);
  assert.ok(handler.indexOf('if (!shellSettingsLoaded) return;') < handler.indexOf('selectedStackTerminalProfile = normalizeStackTerminalProfile(value);'));
  assert.ok(handler.indexOf('if (!shellSettingsLoaded) return;') < handler.indexOf('saveShellSettings('));
  assert.match(selector, /<fieldset class="settings-select-guard" disabled=\{!shellSettingsLoaded\}>[\s\S]*label="Stack Browser terminal"[\s\S]*onChange=\{handleStackTerminalProfileChange\}/);
});

test('startup fails when required search hotkey hook cannot install', () => {
  const main = readSource('../src-tauri/src/main.rs');

  assert.doesNotMatch(main, /search hotkey hook disabled/);
  assert.match(main, /search hotkey hook is required: \{error\}/);
  assert.match(main, /windows_key_hook::install_loaded_windows_key_hook\(app\.handle\(\)\.clone\(\), shell_settings\.hotkeys, missing_snipping\)\s*\.map_err\(/);
});

test('native hotkeys do not install a low-level keyboard hook or consume Alt release', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  assert.doesNotMatch(rust, /WM_KEYUP|WM_SYSKEYUP|UnhookWindowsHookEx|SearchHotkeyEventKind::KeyUp/);
  assert.doesNotMatch(rust, /VK_LWIN|VK_RWIN|LeftWin|RightWin/);
});

test('fullscreen guard instrumentation tracks duration and wake counts', () => {
  const rust = readSource('../src-tauri/src/appbar.rs');

  assert.match(rust, /fullscreen guard/);
  assert.match(rust, /AtomicU64/);
  assert.match(rust, /FULLSCREEN_GUARD_WAKE_COUNT\.store\(0, Ordering::Relaxed\)/);
  assert.match(rust, /FULLSCREEN_GUARD_WAKE_COUNT\.fetch_add\(1, Ordering::Relaxed\)/);
  assert.match(rust, /fullscreen guard summary duration_ms=/);
  assert.match(rust, /wake_count=/);
});

test('legacy taskbar guard refreshes exact owned snapshots through reconcile', () => {
  const rust = readSource('../src-tauri/src/appbar.rs');
  const start = rust.indexOf('fn start_taskbar_guard(state: &mut ShellRuntimeState, monitor_rect: RECT)');
  const end = rust.indexOf('fn start_taskbar_guard_v2(', start);
  const guard = rust.slice(start, end);

  assert.match(guard, /explorer::reconcile_primary_taskbar_ownership\(&mut snapshots, monitor_rect\)/);
  assert.match(guard, /legacy_taskbar_guard_owned/);
  assert.doesNotMatch(guard, /enforce_primary_taskbar_hidden\(snapshot\.monitor_rect\)/);
});
