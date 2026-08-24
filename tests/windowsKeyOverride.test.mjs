import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function readSource(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

test('native hook callback only classifies and hands off to bounded queue', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const procStart = rust.indexOf('unsafe extern "system" fn windows_key_hook_proc');
  const procEnd = rust.indexOf('#[cfg(windows)]\nfn control_key_is_down()', procStart);
  const proc = rust.slice(procStart, procEnd);

  assert.match(rust, /mpsc::sync_channel\(8\)/);
  assert.match(proc, /try_send\(decision\)/);
  assert.doesNotMatch(proc, /AppHandle/);
  assert.doesNotMatch(proc, /emit_to\(/);
});

test('worker owns AppHandle and emit_to for hook events', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const procStart = rust.indexOf('unsafe extern "system" fn windows_key_hook_proc');
  const procEnd = rust.indexOf('#[cfg(windows)]\nfn control_key_is_down()', procStart);
  const proc = rust.slice(procStart, procEnd);

  assert.match(rust, /let worker_app_handle = app_handle\.clone\(\);/);
  assert.match(rust, /worker_app_handle\.emit_to\(/);
  assert.doesNotMatch(proc, /emit_to\(/);
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
  const quickLaunchPanel = readSource('../src/components/QuickLaunchPanelSurface.svelte');
  const stackPopup = readSource('../src/components/StackPopupSurface.svelte');
  const terminalPanel = readSource('../src/components/TerminalPanelSurface.svelte');
  const commandPanel = readSource('../src/components/CommandPanelSurface.svelte');

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
  assert.match(quickLaunchPanel, /function isCtrlSpaceHotkey\(event: KeyboardEvent\)/);
  assert.match(quickLaunchPanel, /function handleSearchHotkeyKeydown\(event: KeyboardEvent\)/);
  assert.match(quickLaunchPanel, /window\.addEventListener\('keydown', handleSearchHotkeyKeydown, true\)/);
  assert.match(quickLaunchPanel, /window\.removeEventListener\('keydown', handleSearchHotkeyKeydown, true\)/);
  assert.doesNotMatch(quickLaunchPanel, /window\.addEventListener\('keydown', handleKeydown, true\)/);
  assert.match(quickLaunchPanel, /void emitTo\(TOP_BAR_TARGET, SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT/);
  assert.match(quickLaunchPanel, /event\.code !== 'Space' \|\| !shellSurfaceHotkeyHandled/);
  assert.match(stackPopup, /function isCtrlSpaceHotkey\(event: KeyboardEvent\)/);
  assert.match(stackPopup, /function handleSearchHotkeyKeydown\(event: KeyboardEvent\)/);
  assert.match(stackPopup, /window\.addEventListener\('keydown', handleSearchHotkeyKeydown, true\)/);
  assert.match(stackPopup, /window\.removeEventListener\('keydown', handleSearchHotkeyKeydown, true\)/);
  assert.doesNotMatch(stackPopup, /window\.addEventListener\('keydown', handleKeydown, true\)/);
  assert.match(stackPopup, /void emitTo\(TOP_BAR_TARGET, SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT/);
  assert.match(stackPopup, /event\.code !== 'Space' \|\| !shellSurfaceHotkeyHandled/);
  assert.match(terminalPanel, /function isCtrlSpaceHotkey\(event: KeyboardEvent\)/);
  assert.match(terminalPanel, /window\.addEventListener\('keydown', keydownHandler, true\)/);
  assert.match(terminalPanel, /window\.removeEventListener\('keydown', keydownHandler, true\)/);
  assert.match(terminalPanel, /void emitTo\(TOP_BAR_TARGET, SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT/);
  assert.match(terminalPanel, /event\.code === 'Space' && shellSurfaceHotkeyHandled/);
  assert.match(commandPanel, /function isCtrlSpaceHotkey\(event: KeyboardEvent\)/);
  assert.match(commandPanel, /window\.addEventListener\('keydown', keydownHandler, true\)/);
  assert.match(commandPanel, /window\.removeEventListener\('keydown', keydownHandler, true\)/);
  assert.match(commandPanel, /void emitTo\(TOP_BAR_TARGET, SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT/);
  assert.match(commandPanel, /event\.code === 'Space' && shellSurfaceHotkeyHandled/);
});

test('Alt+Backquote terminal hotkey emits and shell surfaces toggle terminal panel', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const topBar = readSource('../src/components/TopBar.svelte');
  const bottomBar = readSource('../src/components/BottomBar.svelte');

  assert.match(rust, /TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT: &str = "terminal:toggle-panel"/);
  assert.match(rust, /SearchHotkeyDecision::ToggleTerminal/);
  assert.match(rust, /VK_OEM_3/);
  assert.match(rust, /VK_LMENU/);
  assert.match(rust, /VK_RMENU/);
  assert.match(rust, /emit_to\(\s*crate::shell_windows::TOP_BAR_LABEL,\s*TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT/);
  assert.match(rust, /alt_backquote_toggles_terminal_and_suppresses_backquote/);
  assert.match(topBar, /const TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT = 'terminal:toggle-panel';/);
  assert.match(topBar, /listen\(TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT, \(\) => \{\s*void toggleTerminalPanel\(terminalControl\);/);
  assert.match(topBar, /function isAltBackquoteHotkey\(event: KeyboardEvent\)/);
  assert.match(bottomBar, /void emit\(TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT\)/);
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
  assert.match(rust, /unavailable_hook_state_decision\(event\)/);
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
