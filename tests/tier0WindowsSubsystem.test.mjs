import assert from 'node:assert/strict';
import test from 'node:test';
import { readRust } from './support/rustSource.mjs';

// BLD-01 / WIN-05: release builds must not allocate a console window.
test('main.rs declares the release windows_subsystem attribute at crate root', () => {
  const main = readRust('src-tauri/src/main.rs');
  const attr = /^#!\[cfg_attr\(not\(debug_assertions\), windows_subsystem = "windows"\)\]\s*$/m;
  assert.match(main, attr, 'missing #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]');
  const attrIndex = main.search(attr);
  const firstItem = main.search(/^(mod |use |fn |static |const |pub )/m);
  assert.ok(attrIndex < firstItem, 'inner attribute must precede all items');
});
