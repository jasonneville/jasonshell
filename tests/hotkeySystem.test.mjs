import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { test } from 'node:test';

function readSource(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

test('settings contract exposes fifth snipping hotkey while preserving original four defaults', () => {
  const settings = readSource('../src/lib/settings.ts');
  const rustSettings = readSource('../src-tauri/src/settings.rs');

  assert.match(settings, /export type StandardHotkeyAction = 'search' \| 'terminal' \| 'stackBrowser' \| 'speechTranscription' \| 'snipping' \| 'processManager' \| 'settings' \| 'quickCommands';/);
  assert.match(settings, /export interface StandardHotkeySettings \{[^}]*search: CanonicalHotkeyBinding;[^}]*terminal: CanonicalHotkeyBinding;[^}]*stackBrowser: CanonicalHotkeyBinding;[^}]*speechTranscription: CanonicalHotkeyBinding;[^}]*\}/);
  assert.match(settings, /hotkeys: defaultStandardHotkeySettings\(\)/);
  assert.match(settings, /search: 'Ctrl\+Space'/);
  assert.match(settings, /terminal: 'Alt\+Backquote'/);
  assert.match(settings, /stackBrowser: 'Alt\+1'/);
  assert.match(settings, /speechTranscription: 'Ctrl\+D'/);
  assert.match(settings, /snipping: 'Alt\+S'/);
  assert.match(settings, /snipping: CanonicalHotkeyBinding/);

  assert.match(rustSettings, /pub struct StandardHotkeySettings \{[^}]*pub search: CanonicalHotkeyBinding,[^}]*pub terminal: CanonicalHotkeyBinding,[^}]*pub stack_browser: CanonicalHotkeyBinding,[^}]*pub speech_transcription: CanonicalHotkeyBinding,[^}]*\}/);
  assert.match(rustSettings, /fn default_standard_hotkeys\(\) -> StandardHotkeySettings/);
  assert.match(rustSettings, /search:[\s\S]*"Ctrl\+Space"/);
  assert.match(rustSettings, /terminal:[\s\S]*"Alt\+Backquote"/);
  assert.match(rustSettings, /stack_browser:[\s\S]*"Alt\+1"/);
  assert.match(rustSettings, /speech_transcription:[\s\S]*"Ctrl\+D"/);
  assert.match(rustSettings, /pub snipping: CanonicalHotkeyBinding/);
  assert.doesNotMatch(settings + rustSettings, /customCommand|commandLine|scriptPath|arbitraryAction|shellCommand/);
});

test('persisted hotkeys validate canonical bindings and reject unsafe chords', () => {
  const rustSettings = readSource('../src-tauri/src/settings.rs');

  assert.match(rustSettings, /fn validate_standard_hotkey_settings\(/);
  assert.match(rustSettings, /fn canonicalize_hotkey_binding\(/);
  assert.match(rustSettings, /rejects_unsafe_hotkey_bindings_before_persisting/);
  assert.match(rustSettings, /rejects_duplicate_standard_hotkeys/);
  assert.match(rustSettings, /canonicalizes_hotkey_bindings_before_persisting/);
});

test('existing standard action events remain available with one native Search delivery path', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const contracts = readSource('../src-tauri/src/contracts.rs');
  assert.match(rust, /WM_HOTKEY/);
  assert.match(rust, /RegisterHotKey/);
  assert.match(rust, /MOD_NOREPEAT/);
  for (const action of ['Search', 'Terminal', 'StackBrowser', 'Speech']) {
    assert.match(rust, new RegExp(`ConfiguredHotkeyAction::${action}`));
  }

  assert.match(contracts, /pub const SEARCH_TOGGLE_CENTERED: &str = "search:toggle-centered"/);
  assert.match(contracts, /pub const TERMINAL_TOGGLE_PANEL: &str = "terminal:toggle-panel"/);
  assert.match(contracts, /pub const STACK_BROWSER_TOGGLE: &str = "stack-browser:toggle"/);
  assert.match(rust, /crate::contracts::events::SEARCH_TOGGLE_CENTERED/);
  assert.match(rust, /crate::contracts::events::TERMINAL_TOGGLE_PANEL/);
  assert.match(rust, /crate::contracts::events::STACK_BROWSER_TOGGLE/);
  assert.match(rust, /crate::contracts::events::SPEECH_TOGGLE/);
  const dispatchStart = rust.indexOf('fn dispatch_hotkey(');
  const dispatchEnd = rust.indexOf('fn run_hotkey_thread(', dispatchStart);
  assert.ok(dispatchStart !== -1 && dispatchEnd > dispatchStart);
  const dispatch = rust.slice(dispatchStart, dispatchEnd);
  const searchArm = dispatch.match(/ConfiguredHotkeyAction::Search\s*=>\s*(\{[^}]*\}|[^,]*,)/);
  assert.ok(searchArm, 'WM_HOTKEY dispatcher must explicitly ignore Search');
  assert.doesNotMatch(searchArm[0], /emit_to|SEARCH_TOGGLE_CENTERED/, 'Search must not double-toggle via WM_HOTKEY');
});

test('startup and successful settings saves apply hook configuration at runtime', () => {
  const main = readSource('../src-tauri/src/main.rs');
  const rustSettings = readSource('../src-tauri/src/settings.rs');
  const tsSettings = readSource('../src/lib/settings.ts');

  assert.match(main, /let \(shell_settings, missing_snipping\) = settings::load_hotkey_startup_settings\(app\.handle\(\)\)/);
  assert.match(main, /windows_key_hook::install_loaded_windows_key_hook\(app\.handle\(\)\.clone\(\),\s*shell_settings\.hotkeys, missing_snipping/);
  // Shortcut writes now have their own strict command; generic writes preserve
  // committed hotkeys instead of applying an unsaved renderer draft.
  assert.match(rustSettings, /save_shell_settings_for_app[\s\S]*save_unrelated_settings\(&path, settings, false\)/);
  assert.match(rustSettings, /save_settings_panel_for_app[\s\S]*save_unrelated_settings\(&path, settings, true\)/);
  assert.match(rustSettings, /settings\.hotkeys = previous\.hotkeys/);
  assert.match(rustSettings, /pub async fn save_standard_hotkeys/);
  assert.match(rustSettings, /register_standard_hotkeys/);
  assert.match(rustSettings, /restore_standard_hotkey_snapshot/);
  assert.match(tsSettings, /SETTINGS_COMMANDS[\s\S]*save: 'save_shell_settings'/);
  assert.match(tsSettings, /saveShellSettings[\s\S]*broadcastShellSettings\(saved\)/);
});

test('Settings UI has accessible hotkey capture, conflict feedback, reset defaults, and no arbitrary action code', () => {
  const surface = readSource('../src/components/SettingsPanelSurface.svelte');
  const hotkeys = readSource('../src/lib/standardHotkeys.ts');

  assert.match(surface, /aria-labelledby="hotkeys-heading"/);
  assert.match(surface, /<h2 id="hotkeys-heading">Keyboard shortcuts<\/h2>/);
  assert.match(surface, /aria-label=\{`Capture \$\{HOTKEY_ACTION_NAMES\[action\]\} shortcut`\}/);
  for (const name of ['Search', 'Terminal', 'Stack Browser']) assert.ok(hotkeys.includes(`'${name}'`));
  assert.match(surface, /on:keydown=\{captureHotkeyBinding/);
  assert.match(surface, /aria-describedby=\{[^}]*hotkey[^}]*error/);
  assert.match(surface, /role="alert"/);
  assert.match(hotkeys, /Duplicate shortcut/);
  assert.match(surface, /Reset shortcuts to defaults/);
  assert.match(surface, /defaultStandardHotkeySettings\(\)/);
  assert.doesNotMatch(surface, /<input[^>]+name="action"|<textarea[^>]+command|customCommand|shellCommand|scriptPath/);
});

test('speech transcription is a fourth persisted, configurable and accessible standard action', () => {
  const ts = readSource('../src/lib/settings.ts');
  const rust = readSource('../src-tauri/src/settings.rs');
  const ui = readSource('../src/components/SettingsPanelSurface.svelte');
  assert.match(ts, /StandardHotkeyAction[^\n]*'speechTranscription'/);
  assert.match(ts, /interface StandardHotkeySettings\s*\{[^}]*speechTranscription: CanonicalHotkeyBinding/s);
  assert.match(rust, /struct StandardHotkeySettings\s*\{[^}]*speech_transcription: CanonicalHotkeyBinding/s);
  assert.match(ui, /HOTKEY_ACTION_NAMES\[action\]/);
  assert.match(readSource('../src/lib/standardHotkeys.ts'), /speechTranscription: 'Speech transcription'/);
});

test('speech uses registered OS no-repeat hotkey without key-up suppression', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  const contracts = readSource('../src-tauri/src/contracts.rs');
  assert.match(rust, /ConfiguredHotkeyAction::Speech/);
  assert.match(rust, /speech_transcription/);
  assert.match(rust, /MOD_NOREPEAT/);
  assert.match(rust, /WM_HOTKEY/);
  assert.match(rust, /SPEECH_TOGGLE/);
  assert.match(contracts, /SPEECH_TOGGLE[^\n]*speech:toggle/);
  const speechArm = rust.slice(rust.indexOf('ConfiguredHotkeyAction::Speech =>'), rust.indexOf('ConfiguredHotkeyAction::Snipping =>'));
  assert.doesNotMatch(speechArm, /SearchKeyOwnership|WM_SYSKEYUP|CallNextHookEx/, 'speech remains outside Search trigger ownership');
});

test('failed OS replacement retains or restores previous hotkeys and reports failure', () => {
  const rust = readSource('../src-tauri/src/windows_key_hook.rs');
  assert.match(rust, /RegisterHotKey/);
  assert.match(rust, /UnregisterHotKey/);
  assert.match(rust, /(?:rollback|restore|previous|old_bindings|old_registrations)/i);
  assert.match(rust, /(?:Err\(|map_err\(|is_err\()/);
});

test('speech hotkey reaches same mic toggle command path as clicking control', () => {
  const top = readSource('../src/components/TopBar.svelte');
  const mic = readSource('../src/components/TopBarMicControl.svelte');
  assert.match(top + mic, /listen(?:<[^>]+>)?\(SPEECH_[A-Z_]*TOGGLE[A-Z_]*_EVENT,/);
  assert.match(mic, /function toggleSpeech[\s\S]*startSpeechCapture\([\s\S]*stopSpeechCapture\(/);
  assert.match(mic, /onClick=\{[^}]*toggleSpeech/);
});

test('hotkey toggles reconcile native/manual search and Stack Browser close before reopening', () => {
  const top = readSource('../src/components/TopBar.svelte');
  assert.match(top, /listen\(SEARCH_PANEL_CLOSED_EVENT,[\s\S]*?searchOpen = false/);
  assert.match(top, /listen\([^\n]*STACK[^\n]*CLOSED[^\n]*,[\s\S]*?stackBrowserOpen = false/);
  assert.match(top, /function toggleCenteredSearchFromHotkey\(\)[\s\S]*?if \(searchOpen\)[\s\S]*?closePanel\(\)[\s\S]*?openCenteredPanel/);
  assert.match(top, /function toggleStackBrowserFromHotkey\(\)[\s\S]*?toggleStackBrowserPanel/);
});

test('standard hotkey commands are Settings-only capabilities and never legacy process authority', () => {
  const commands = ['get_standard_hotkey_status', 'save_standard_hotkeys', 'begin_standard_hotkey_capture', 'renew_standard_hotkey_capture', 'end_standard_hotkey_capture'];
  const settings = JSON.parse(readSource('../src-tauri/capabilities/settings-panel.json'));
  const build = readSource('../src-tauri/build.rs');
  const legacy = readSource('../src-tauri/permissions/legacy-shell.toml');
  assert.deepEqual(settings.windows, ['settings-panel']);
  for (const command of commands) {
    const permission = `allow-${command.replaceAll('_', '-')}`;
    assert.ok(settings.permissions.includes(permission), `${command} missing Settings permission`);
    assert.ok(!legacy.includes(`"${command}"`), `${command} leaked into legacy group`);
    assert.ok(build.includes(`"${command}"`));
    for (const file of readdirSync(new URL('../src-tauri/capabilities/', import.meta.url)).filter(name => name.endsWith('.json') && name !== 'settings-panel.json')) {
      assert.ok(!JSON.parse(readSource(`../src-tauri/capabilities/${file}`)).permissions.includes(permission), `${command} leaked to ${file}`);
    }
  }
  assert.match(build, /!STANDARD_HOTKEY_COMMANDS\.contains\(command\)/);
  const process = JSON.parse(readSource('../src-tauri/capabilities/process-manager.json'));
  assert.deepEqual(process.permissions, ['core:default', 'allow-get-process-manager-context', 'allow-set-process-manager-tab', 'allow-get-system-info-snapshot', 'core:window:default']);
});

test('native lease expiry and lifecycle retirement are backend-owned rather than renderer-timer authority', () => {
  const hook = readSource('../src-tauri/src/windows_key_hook.rs');
  assert.match(hook, /SetTimer\(None, 0, 100, None\)/);
  assert.match(hook, /fn expire\([\s\S]*expire_observed\(/);
  assert.match(hook, /fn capture_command[\s\S]*capture_transition\(/);
  assert.match(hook, /WM_HOTKEY[\s\S]*route_hotkey_observed\(/);
  const lifecycle = readSource('../src-tauri/src/settings_panel.rs') + readSource('../src-tauri/src/main.rs');
  assert.match(lifecycle, /retire_settings_capture\(/);
});

test('eighth Quick Commands native dispatch pairs scoped top-bar event with existing launcher toggle handler', () => {
  const native = readSource('../src-tauri/src/windows_key_hook.rs');
  const contracts = readSource('../src-tauri/src/contracts.rs');
  const top = readSource('../src/components/TopBar.svelte');
  const events = readSource('../src/ipc/events.ts');
  const arm = native.match(/ConfiguredHotkeyAction::QuickCommands => \{([^}]+)\}/);
  assert.ok(arm, 'native Quick Commands dispatch arm missing');
  assert.match(arm[1], /emit_to\(tauri::EventTarget::webview_window\(crate::contracts::surfaces::TOP_BAR\), crate::contracts::events::QUICK_COMMANDS_TOGGLE, \(\)\)/);
  assert.match(contracts, /QUICK_COMMANDS_TOGGLE: &str = "quick-commands:toggle"/);
  assert.match(events, /quickCommandsToggle: 'quick-commands:toggle'/);
  assert.match(top, /listen\(IPC_EVENTS\.quickCommandsToggle,[\s\S]*?toggleCommandPanel\(document\.querySelector\('\.command-button'\)\)[\s\S]*?target: \{ kind: 'WebviewWindow', label: getCurrentWebviewWindow\(\)\.label \}/);
  assert.match(top, /onClick=\{\(event\) => void toggleCommandPanel\(event\.currentTarget\)\}/);
});
