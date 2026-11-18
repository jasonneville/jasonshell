import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function readSource(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

function rustFunction(source, name) {
  const start = source.indexOf(`fn ${name}(`);
  assert.notEqual(start, -1, `${name} must exist`);
  const open = source.indexOf('{', start);
  let depth = 0;
  for (let index = open; index < source.length; index += 1) {
    if (source[index] === '{') depth += 1;
    if (source[index] === '}' && --depth === 0) return source.slice(open + 1, index);
  }
  assert.fail(`unterminated ${name}`);
}

test('callback source forwards negative and injected input before borrowing ownership', () => {
  const callback = rustFunction(readSource('../src-tauri/src/windows_key_hook.rs'), 'search_keyboard_hook');
  assert.match(callback, /if code < 0\s*\{\s*return CallNextHookEx\(None, code, wparam, lparam\)/);
  assert.ok(callback.indexOf('if code < 0') < callback.indexOf('lparam.0 as *const KBDLLHOOKSTRUCT'),
    'negative hook code must not dereference callback data');
  assert.match(callback, /event\.flags\.contains\(LLKHF_INJECTED\)[\s\S]*?return CallNextHookEx/);
  assert.ok(callback.indexOf('LLKHF_INJECTED') < callback.indexOf('SEARCH_HOOK.with('));
  assert.match(callback, /if consumed\s*\{\s*LRESULT\(1\)\s*\}\s*else\s*\{\s*CallNextHookEx/);
  assert.doesNotMatch(callback, /emit_to|GetAsyncKeyState|eprintln!|println!|format!|Mutex|\.lock\(/,
    'callback stays free of IPC, input logging, async-state queries, allocation formatting, and mutex waits');
});

test('message-thread source refreshes ownership from verified registrations even after failed rollback', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const run = rustFunction(rust, 'run_hotkey_thread');
  const pause = run.indexOf('refresh_search_binding(None)');
  const execute = run.indexOf('(request.run)(&mut state, &app_handle)');
  const refresh = run.indexOf('refresh_search_state(&state)', execute);
  assert.ok(pause >= 0 && pause < execute && execute < refresh,
    'opaque native requests pause acquisition and always refresh verified state afterward');
  const requestStart = rust.indexOf('fn request_on_thread<');
  const requestEnd = rust.indexOf('pub fn install_windows_key_hook(', requestStart);
  assert.ok(requestStart >= 0 && requestEnd > requestStart);
  const request = rust.slice(requestStart, requestEnd);
  const result = request.indexOf('let result = run(state, app)');
  const verified = request.indexOf('refresh_search_state(state)', result);
  const reply = request.indexOf('result_tx.send(result)');
  assert.ok(result >= 0 && result < verified && verified < reply,
    'refresh actual active Search after the closure on success or failure BEFORE publishing ACK');
  assert.doesNotMatch(request.slice(result, verified), /result\.(?:is_ok|is_err)\(|if let Ok/,
    'failed rollback may leave Search vacant and must still refresh');
  assert.match(rustFunction(rust, 'refresh_search_state'), /refresh_search_binding\(state\.current\[0\]\)/);
  const unhook = run.indexOf('UnhookWindowsHookEx(hook)');
  const clear = run.indexOf('SEARCH_HOOK.with(|slot| *slot.borrow_mut() = None)', unhook);
  const join = run.indexOf('dispatch_worker.join()', unhook);
  assert.ok(unhook >= 0 && unhook < clear && clear < join,
    'remove native callback before clearing its state; finish old dispatcher before reaping thread');
});

test('capture and fence source pause Search admission while preserving owned trigger state', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const pause = rustFunction(rust, 'pause_search_dispatch');
  assert.match(pause, /emission_gate\.fetch_or\(4,/);
  assert.match(pause, /pending\.store\(false,/);
  assert.doesNotMatch(pause, /ownership\s*=|owned\.(?:fill|clear)|owned\s*=/);
  const refresh = rustFunction(rust, 'refresh_search_state');
  assert.match(refresh, /if state\.capture\.is_some\(\)\s*\{\s*pause_search_dispatch\(\);\s*refresh_search_binding\(None\)/);
  assert.match(refresh, /refresh_search_binding\(state\.current\[0\]\)[\s\S]*emission_gate\.fetch_and\(!4,/);
  assert.match(rustFunction(rust, 'fence'), /pause_search_dispatch\(\)/);
});

test('startup source installs callback before reconciling authoritative physical key snapshot', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const run = rustFunction(rust, 'run_hotkey_thread');
  const install = run.indexOf('SetWindowsHookExW(');
  const snapshot = run.indexOf('GetAsyncKeyState(');
  assert.ok(install >= 0 && snapshot > install,
    'pre-install snapshot can miss Ctrl release and steal later bare Space; sample only after hook install');
  const reconcile = run.indexOf('reconcile_search_startup_keys(');
  const ready = run.indexOf('ready.send(Ok(');
  assert.ok(reconcile > snapshot && ready > reconcile,
    'merge callback-observed transitions and seed preheld ownership before startup ready');
  const callback = rustFunction(rust, 'search_keyboard_hook');
  assert.match(callback, /observed\[[^\]]+\]\s*=\s*Some\(down\)/,
    'startup reconciliation must preserve real callback transitions, not only merge an empty fixture');
});

test('dispatcher source holds explicit emission admission and retires through the same gate', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const worker = rustFunction(rust, 'start_search_dispatch');
  const emit = worker.indexOf('app.emit_to(');
  assert.ok(emit >= 0);
  assert.match(worker.slice(0, emit), /let Some\([^)]*(?:permit|admission|lease)[^)]*\)\s*=\s*dispatch\.try_admit_emission\(\)/,
    'a retired=false load is not serialized emission admission');
  assert.match(worker, /dispatch\.retire\(\)/);
  const run = rustFunction(rust, 'run_hotkey_thread');
  const stop = run.slice(run.indexOf('WM_STOP_HOTKEYS =>'));
  assert.match(stop, /dispatch\.retire\(\)/,
    'stop must revoke future emission admission, not only set an independent flag');
});

test('Search owns its native trigger cycle while other shortcuts retain OS registration', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  assert.match(rust, /RegisterHotKey/);
  assert.match(rust, /UnregisterHotKey/);
  assert.match(rust, /MOD_NOREPEAT/);
  for (const nativeBoundary of ['WH_KEYBOARD_LL', 'SetWindowsHookExW', 'UnhookWindowsHookEx', 'CallNextHookEx', 'KBDLLHOOKSTRUCT']) {
    assert.ok(rust.includes(nativeBoundary), `Search ownership requires ${nativeBoundary}`);
  }
  assert.match(rust, /SearchKeyOwnership/);
  assert.match(rust, /GetAsyncKeyState/, 'install/rebind must not treat a preheld trigger as fresh');
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

test('native hook installs during setup and accepted exit attempts its drain', () => {
  const main = readSource('../src-tauri/src/main.rs');

  assert.match(main, /let \(shell_settings, missing_snipping\) = settings::load_hotkey_startup_settings\(app\.handle\(\)\)/);
  assert.match(main, /windows_key_hook::install_loaded_windows_key_hook\(app\.handle\(\)\.clone\(\), shell_settings\.hotkeys, missing_snipping\)/);
  const exit = readSource('../src-tauri/src/graceful_exit.rs');
  const hook = readSource('../src-tauri/src/windows_key_hook.rs');
  assert.match(exit, /crate::windows_key_hook::stop_and_drain_hotkeys\(\)/);
  assert.match(rustFunction(hook, 'stop_and_drain_hotkeys'), /EXIT_RETIRED\.store\(true,[\s\S]*retire_settings_capture\(\);[\s\S]*if let Err\(error\) = stop_hotkey_thread\(&mut guard\)[\s\S]*return Err\(error\);[\s\S]*if EXIT_DRAIN_FAILED\.load[\s\S]*return Err\([\s\S]*DrainReceipt \{ finished: true \}/);
  assert.match(main, /RunEvent::ExitRequested \{ api, code, \.\. \} = event[\s\S]*?api\.prevent_exit\(\)/);
});

test('Settings hotkey controls stay disabled until settings load succeeds', () => {
  const source = readSource('../src/components/SettingsPanelSurface.svelte');

  assert.match(source, /let shellSettingsLoaded = false;/);
  assert.match(source, /const settings = await loadShellSettings\(\);[\s\S]*shellSettings = settings;[\s\S]*shellSettingsLoaded = true;/);
  assert.match(source, /catch \(error\) \{[\s\S]*shellSettingsLoaded = false;/);
  assert.match(source, /async function saveHotkeys[\s\S]*if \(!shellSettingsLoaded \|\| hotkeyBusy\) return;/);
  assert.match(source, /async function startCapture[\s\S]*if \(!shellSettingsLoaded \|\| hotkeyBusy \|\| !captureListenerReady\) return;/);
  assert.match(source, /function captureHotkeyBinding[\s\S]*capturePending[\s\S]*event\.repeat/);
  assert.match(source, /\{#each HOTKEY_ACTIONS as action[\s\S]*data-hotkey-action=\{action\}[\s\S]*disabled=\{!shellSettingsLoaded \|\| hotkeyBusy[^}]*\}/);
  assert.match(source, /class="hotkey-reset" disabled=\{!shellSettingsLoaded \|\| hotkeyBusy\}/);
});

test('Settings Stack Browser terminal selector cannot persist before authoritative settings load', () => {
  const source = readSource('../src/components/SettingsPanelSurface.svelte');
  const handlerStart = source.indexOf('async function handleStackTerminalProfileChange');
  const handlerEnd = source.indexOf('$: draftValidation', handlerStart);
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

test('native Search handles normal and system key transitions without a frontend release workaround', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  for (const transition of ['WM_KEYDOWN', 'WM_KEYUP', 'WM_SYSKEYDOWN', 'WM_SYSKEYUP']) {
    assert.ok(rust.includes(transition), `Search ownership requires ${transition}`);
  }
  assert.doesNotMatch(rust, /SendInput|keybd_event/, 'ownership must never synthesize replacement input');
  const topBar = readSource('../src/components/TopBar.svelte');
  const toggle = topBar.slice(topBar.indexOf('function toggleCenteredSearchFromHotkey('), topBar.indexOf('function toggleCenteredSearchFromHotkey(') + 500);
  assert.doesNotMatch(toggle, /setTimeout|requestAnimationFrame|keyup|event\.repeat/, 'native ownership must not delay Search hide');
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
