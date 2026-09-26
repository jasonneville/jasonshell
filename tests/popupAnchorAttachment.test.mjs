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

test('task preview touches bottom bar both above and on fallback below', () => {
  const source = rust('task_preview.rs');
  const normalized = compact(source);
  assert.match(normalized, /let above_y = host_position\.y - preview_height(?: - TASK_PREVIEW_MARGIN_PHYSICAL)?;/);
  assert.match(normalized, /let below_y = host_position\.y \+ host_size\.height as i32(?: \+ TASK_PREVIEW_MARGIN_PHYSICAL)?;/);
  if (normalized.includes('TASK_PREVIEW_MARGIN_PHYSICAL;')) {
    assert.match(source, /const TASK_PREVIEW_MARGIN_PHYSICAL: i32 = 0;/);
  }
});

test('process manager touches bottom bar above', () => {
  const source = rust('process_manager.rs');
  assertEdge(source, 'bottom_position.y - height as i32', 'PROCESS_MANAGER_MARGIN_PHYSICAL', 'process manager');
});

for (const name of ['quick_launch_panel.rs', 'task_gallery.rs']) {
  test(`${name} touches bottom bar above`, () => {
    assert.match(compact(rust(name)), /bottom_position\.y - height as i32/);
    assert.doesNotMatch(compact(rust(name)), /bottom_position\.y - height as i32 - [A-Z_]*MARGIN/);
  });
}
