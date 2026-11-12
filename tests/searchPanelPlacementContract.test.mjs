import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const source = readFileSync('src-tauri/src/search_panel.rs', 'utf8');

function command(name, nextName) {
  const start = source.indexOf(`pub fn ${name}(`);
  const end = source.indexOf(`pub fn ${nextName}(`, start);
  assert.ok(start >= 0 && end > start, `${name} command boundary exists`);
  return source.slice(start, end);
}

// Static call-path evidence only: these tests do not observe a native window.
test('centered show applies production placement to scaled size and selected monitor before show/focus', () => {
  const show = command('show_centered_search_panel', 'resize_search_panel');
  assert.match(show, /top_bar\s*\.current_monitor\(\)/);
  assert.match(show, /let width = \(logical_size\.width \* scale_factor\)\.round\(\) as u32/);
  assert.match(show, /let height = \(logical_size\.height \* scale_factor\)\.round\(\) as u32/);
  assert.match(show, /let \(x, y\) = centered_search_panel_position\(\s*monitor_position\.x,\s*monitor_position\.y,\s*monitor_size\.width,\s*monitor_size\.height,\s*width,\s*height,?\s*\)/);
  assert.match(show, /set_position\(PhysicalPosition::new\(x, y\)\)/);
  const size = show.indexOf('.set_size(');
  const position = show.indexOf('.set_position(');
  const visible = show.indexOf('.show()');
  const focus = show.indexOf('.set_focus()');
  assert.ok(size >= 0 && size < position && position < visible && visible < focus);
});

test('anchored show retains control-edge placement without centered placement helper', () => {
  const show = command('show_search_panel', 'show_centered_search_panel');
  assert.match(show, /let panel_x = anchored_panel_x\(/);
  assert.match(show, /let panel_y = top_position\.y \+ top_size\.height as i32 \+ SEARCH_PANEL_MARGIN_PHYSICAL/);
  assert.doesNotMatch(show, /centered_search_panel_position\(/);
});

test('live resizing changes bounded size without repositioning or reopening panel', () => {
  const resize = command('resize_search_panel', 'hide_search_panel');
  assert.match(resize, /bounded_centered_search_size\(request\)/);
  assert.match(resize, /\.set_size\(PhysicalSize::new\(/);
  assert.doesNotMatch(resize, /centered_search_panel_position\(|\.set_position\(|\.show\(|\.set_focus\(/);
});
