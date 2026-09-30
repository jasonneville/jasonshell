import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function readSource(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

test('settings contract exposes four built-in hotkey actions with preserved existing defaults', () => {
  const settings = readSource('../src/lib/settings.ts');
  const rustSettings = readSource('../src-tauri/src/settings.rs');

  assert.match(settings, /export type StandardHotkeyAction = 'search' \| 'terminal' \| 'stackBrowser' \| 'speechTranscription';/);
  assert.match(settings, /export interface StandardHotkeySettings \{[^}]*search: CanonicalHotkeyBinding;[^}]*terminal: CanonicalHotkeyBinding;[^}]*stackBrowser: CanonicalHotkeyBinding;[^}]*speechTranscription: CanonicalHotkeyBinding;[^}]*\}/);
  assert.match(settings, /hotkeys: defaultStandardHotkeySettings\(\)/);
  assert.match(settings, /search: 'Ctrl\+Space'/);
  assert.match(settings, /terminal: 'Alt\+Backquote'/);
  assert.match(settings, /stackBrowser: 'Alt\+1'/);
  assert.match(settings, /speechTranscription: 'Ctrl\+D'/);

  assert.match(rustSettings, /pub struct StandardHotkeySettings \{[^}]*pub search: CanonicalHotkeyBinding,[^}]*pub terminal: CanonicalHotkeyBinding,[^}]*pub stack_browser: CanonicalHotkeyBinding,[^}]*pub speech_transcription: CanonicalHotkeyBinding,[^}]*\}/);
  assert.match(rustSettings, /fn default_standard_hotkeys\(\) -> StandardHotkeySettings/);
  assert.match(rustSettings, /search:[\s\S]*"Ctrl\+Space"/);
  assert.match(rustSettings, /terminal:[\s\S]*"Alt\+Backquote"/);
  assert.match(rustSettings, /stack_browser:[\s\S]*"Alt\+1"/);
  assert.match(rustSettings, /speech_transcription:[\s\S]*"Ctrl\+D"/);
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

test('registered hotkey IDs dispatch four existing action events', () => {
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
});

test('startup and successful settings saves apply hook configuration at runtime', () => {
  const main = readSource('../src-tauri/src/main.rs');
  const rustSettings = readSource('../src-tauri/src/settings.rs');
  const tsSettings = readSource('../src/lib/settings.ts');

  assert.match(main, /let shell_settings = settings::load_shell_settings_for_app\(app\.handle\(\)\)/);
  assert.match(main, /windows_key_hook::install_windows_key_hook\(app\.handle\(\)\.clone\(\),\s*shell_settings\.hotkeys/);
  assert.match(rustSettings, /windows_key_hook::configure_standard_hotkeys\(app_handle,\s*&settings\.hotkeys\)/);
  assert.match(rustSettings, /save_shell_settings_for_app[\s\S]*configure_standard_hotkeys/);
  const save = rustSettings.slice(rustSettings.indexOf('pub(crate) fn save_shell_settings_for_app('), rustSettings.indexOf('pub(crate) fn update_shell_settings_for_app('));
  assert.ok(save.indexOf('configure_standard_hotkeys(') < save.indexOf('save_settings_to_path('), 'OS registration must succeed before writing settings to disk');
  assert.match(tsSettings, /SETTINGS_COMMANDS[\s\S]*save: 'save_shell_settings'/);
  assert.match(tsSettings, /saveShellSettings[\s\S]*broadcastShellSettings\(saved\)/);
});

test('Settings UI has accessible hotkey capture, conflict feedback, reset defaults, and no arbitrary action code', () => {
  const surface = readSource('../src/components/SettingsPanelSurface.svelte');

  assert.match(surface, /aria-labelledby="hotkeys-heading"/);
  assert.match(surface, /<h2 id="hotkeys-heading">Keyboard shortcuts<\/h2>/);
  assert.match(surface, /aria-label="Capture Search shortcut"/);
  assert.match(surface, /aria-label="Capture Terminal shortcut"/);
  assert.match(surface, /aria-label="Capture Stack Browser shortcut"/);
  assert.match(surface, /on:keydown=\{captureHotkeyBinding/);
  assert.match(surface, /aria-describedby=\{[^}]*hotkey[^}]*error/);
  assert.match(surface, /role="alert"/);
  assert.match(surface, /Duplicate shortcut/);
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
  assert.match(ui, /aria-label="Capture Speech transcription shortcut"/i);
  assert.match(ui, /speechTranscription/);
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
  assert.doesNotMatch(rust, /SearchHotkeyEventKind::KeyUp|WM_SYSKEYUP/);
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
  assert.match(top + mic, /listen\(SPEECH_[A-Z_]*TOGGLE[A-Z_]*_EVENT,/);
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
