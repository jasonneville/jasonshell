import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';

const main = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

test('debug P02 probe can finish without granting incidental close to the real shell', () => {
  assert.match(main, /cfg!\(all\(debug_assertions, feature = "text-document-feasibility"\)\)[\s\S]*?P02_NATIVE_PROBE_ROOT/);
  const run = main.slice(main.lastIndexOf('app.run('));
  assert.match(run, /RunEvent::ExitRequested[\s\S]*?if isolated_text_probe && code == Some\(0\) \{\s*return;\s*\}[\s\S]*?api\.prevent_exit\(\)/);
});

test('cancellable ExitRequested never shares unconditional teardown with final Exit', () => {
  // This exact pre-feature route tears down even an exit that another callback
  // prevents. Separate routing is necessary; native behavioral tests must also
  // establish that coordinator cancellation produces zero cleanup effects.
  assert.doesNotMatch(main, /if\s+matches!\(event,\s*RunEvent::Exit\s*\|\s*RunEvent::ExitRequested\s*\{\s*\.\.\s*\}\)/,
    'ExitRequested requires admission/prevent_exit routing, not unconditional cleanup');
});

test('exit control capability is Settings-only; retired protocol has no capability', () => {
  const directory = new URL('../src-tauri/capabilities/', import.meta.url);
  const control = [], participants = [];
  for (const name of readdirSync(directory).filter(name => name.endsWith('.json'))) {
    const capability = JSON.parse(readFileSync(new URL(name, directory), 'utf8'));
    if (capability.permissions.includes('allow-request-shell-exit')) control.push(...capability.windows);
    if (capability.permissions.includes('allow-shell-exit-participation')) participants.push(...capability.windows);
  }
  assert.deepEqual(control, ['settings-panel']);
  assert.deepEqual(participants, []);
});

test('one Settings request replaces participant registration, status, and recovery IPC', () => {
  assert.match(main, /graceful_exit::request_shell_exit\b/);
  assert.doesNotMatch(main, /\b(?:(?:register|unregister|update|ack)_shell_exit_participant|(?:retry|cancel)_shell_exit|get_shell_shutdown_status)\b/);
});

test('exit admission requires the current registered Settings instance before scheduling effects', () => {
  const native = readFileSync(new URL('../src-tauri/src/graceful_exit.rs', import.meta.url), 'utf8');
  const keyHook = readFileSync(new URL('../src-tauri/src/windows_key_hook.rs', import.meta.url), 'utf8');
  assert.match(native, /let initial_caller = authorize_settings_instance\(&window, app\)\?;[\s\S]*?coordinator\.request_checked\(\s*\|\|\s*\{\s*if authorize_settings_instance\(&window, app\)\? != initial_caller/);
  assert.match(native, /revalidate\(\)\?;[\s\S]*?\*accepted = Some\(deadline\)/);
  assert.match(keyHook, /fn authorize_settings_instance[\s\S]*?\.registry\.resolve\(window\)[\s\S]*?registered_settings_window\(app, &caller\)\?/);
  assert.match(keyHook, /fn settings_eligibility[\s\S]*?registered != Some\(caller\)[\s\S]*?!visible \|\| !focused/);
});
