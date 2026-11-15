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
