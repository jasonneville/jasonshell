// Source parity guard, not OS-delivery evidence. The native retained-focus
// pointer journey separately verifies the Windows/WebView keyboard boundary.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

for (const [name, path] of [
  ['production shell builder', '../src-tauri/src/main.rs'],
  ['native synthetic overlay builder', '../src-tauri/examples/snip_synthetic_journey.rs']
]) {
  test(`${name} explicitly filters raw device events for focused Escape hook delivery`, () => {
    const source = readFileSync(new URL(path, import.meta.url), 'utf8');
    const start = source.indexOf('tauri::Builder::default()');
    const end = source.indexOf('.setup(', start);
    assert.ok(start >= 0 && end > start, 'check the real application builder before setup');
    const builder = source.slice(start, end).replace(/\/\*[\s\S]*?\*\/|\/\/[^\r\n]*/g, '');
    assert.match(builder, /\.device_event_filter\(\s*tauri::DeviceEventFilter::Always\s*\)/,
      'actual production and native fixture builders must both select Always; a fixture-only fix is insufficient');
    assert.doesNotMatch(builder, /\.device_event_filter\(\s*tauri::DeviceEventFilter::(?:Never|Unfocused)\s*\)/);
  });
}

test('merged production startup retains both hotkey lifecycle owners and passive Escape forwarding', () => {
  const main = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
  const runtime = readFileSync(new URL('../src-tauri/src/snipping/runtime.rs', import.meta.url), 'utf8');
  const escape = readFileSync(new URL('../src-tauri/src/snipping/escape.rs', import.meta.url), 'utf8');
  assert.match(main, /windows_key_hook::install_loaded_windows_key_hook/);
  const exit = readFileSync(new URL('../src-tauri/src/graceful_exit.rs', import.meta.url), 'utf8');
  assert.match(exit, /crate::windows_key_hook::stop_and_drain_hotkeys\(\)/);
  assert.match(exit, /\.state::<std::sync::Arc<crate::snipping::runtime::SnipRuntime>>\(\)\s*\.stop_and_drain\(\)/);
  assert.match(main, /RunEvent::ExitRequested \{ api, code, \.\. \} = event[\s\S]*?api\.prevent_exit\(\)/);
  assert.match(main, /SnipRuntime::install/);
  assert.match(runtime, /super::escape::install\(Arc::downgrade\(&runtime\)\)/);
  const callback = escape.slice(escape.indexOf('unsafe extern "system" fn keyboard('), escape.indexOf('pub(crate) fn install('));
  assert.match(callback, /CallNextHookEx\(None, code, message, data\)/);
  assert.doesNotMatch(callback, /return\s+LRESULT\(1\)/, 'Escape observer must never swallow input');
});
