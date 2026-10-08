import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const rust = (name) => readFileSync(new URL(`../src-tauri/src/${name}`, import.meta.url), 'utf8');
const compact = (text) => text.replace(/\s+/g, ' ');

// Accept either a direct edge expression or an explicitly zero-valued spacer.
function assertEdge(source, expression, marginName, label) {
  const normalized = compact(source);
  const edge = compact(expression);
  const withoutMargin = normalized.includes(edge);
  assert.ok(withoutMargin, `${label}: position must use the host bar edge`);
  if (new RegExp(`const ${marginName}: i32 =`).test(source)) {
    assert.match(source, new RegExp(`const ${marginName}: i32 = 0;`), `${label}: spacer must be zero`);
  }
  assert.doesNotMatch(normalized, new RegExp(`${marginName}\\s*:\\s*i32\\s*=\\s*[1-9]`));
}

for (const name of [
  'audio_panel.rs', 'calendar_panel.rs', 'command_panel.rs', 'search_panel.rs',
  'settings_panel.rs', 'speech_history_panel.rs', 'terminal_panel.rs', 'tray_panel.rs',
]) {
  test(`${name} opens flush below top bar`, () => {
    const source = rust(name);
    const margin = name === 'speech_history_panel.rs' ? 'PANEL_MARGIN_PHYSICAL' :
      `${name.replace('.rs', '').toUpperCase()}_MARGIN_PHYSICAL`;
    assertEdge(source, 'top_position.y + top_size.height as i32', margin, name);
  });
}

test('Stack Browser popup opens flush below top bar', () => {
  assert.match(compact(rust('stack_popup/popup_window.rs')), /let y = top_position\.y \+ top_size\.height as i32;/);
});

test('task preview floats eight logical pixels above host or below fallback at scaled DPI', () => {
  const source = rust('task_preview.rs');
  const normalized = compact(source);
  assert.match(source, /const TASK_PREVIEW_MARGIN_LOGICAL: f64 = 8\.0;/);
  assert.match(normalized, /let margin = \(TASK_PREVIEW_MARGIN_LOGICAL \* scale_factor\)\.round\(\) as i32;/);
  assert.match(normalized, /let above_y = host_position\.y - preview_height - margin;/);
  assert.match(normalized, /let below_y = host_position\.y \+ host_size\.height as i32 \+ margin;/);
});

test('process manager touches bottom bar above', () => {
  const source = rust('process_manager.rs');
  assertEdge(source, 'bottom_position.y - height as i32', 'PROCESS_MANAGER_MARGIN_PHYSICAL', 'process manager');
});

test('quick_launch_panel.rs touches bottom bar above', () => {
  assert.match(compact(rust('quick_launch_panel.rs')), /bottom_position\.y - height as i32/);
  assert.doesNotMatch(compact(rust('quick_launch_panel.rs')), /bottom_position\.y - height as i32 - [A-Z_]*MARGIN/);
});

test('task gallery floats eight logical pixels above bar with DPI scaling and safe monitor clamp', () => {
  const source = compact(rust('task_gallery.rs'));
  assert.match(source, /let scale_factor = bottom_bar\.scale_factor\(\)/);
  assert.match(source, /if !scale_factor\.is_finite\(\) \|\| scale_factor <= 0\.0/);
  assert.match(source, /let bottom_position = bottom_bar\.outer_position\(\)/);
  assert.match(source, /let padding = \(8\.0 \* scale_factor\)\.round\(\) as i32;/);
  assert.match(source, /let height = \(layout\.panel_height \* scale_factor\)\.round\(\) as u32;/);
  assert.match(source, /let min_y = monitor\.position\(\)\.y \+ padding;/);
  assert.match(source, /let available_height = \(\(bottom_position\.y - padding - min_y\) as f64 \/ scale_factor\)\.max\(0\.0\);/);
  assert.match(source, /let y = \(bottom_position\.y - padding - height as i32\)\.max\(min_y\);/);
  assert.match(source, /PhysicalPosition::new\(x, y\)/);
  // Insufficient safe host space fails closed instead of falling below the bar
  // or overlapping it. Monitor-edge clamping still takes precedence.
  assert.match(source, /if available_width < 60\.0 \|\| available_height < 80\.0 \{ return Err\("Insufficient safe space for task gallery"\.into\(\)\); \}/);
  assert.doesNotMatch(source, /let y = .*bottom_position\.y \+ .*height/);
});
