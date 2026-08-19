import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = readFileSync(new URL('../src-tauri/src/explorer.rs', import.meta.url), 'utf8');

test('explorer suppression source contract exposes owned taskbar collection and identity checks', () => {
  assert.match(source, /ExplorerTaskbarSnapshot/);
  assert.match(source, /primary_taskbar_snapshots/);
  assert.match(source, /TaskbarEdge::Bottom/);
  assert.match(source, /hidden_by_jasonshell/);
  assert.match(source, /MonitorFromWindow|GetMonitorInfoW/);
});
