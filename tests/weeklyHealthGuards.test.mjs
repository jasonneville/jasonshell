import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

// Architecture/registry contracts only; cargo:check supplies compiler evidence.
const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

test('Windows runtime dependencies and lock roots exclude unowned platform/tool launchers', () => {
  const pkg = JSON.parse(read('package.json'));
  const lock = JSON.parse(read('package-lock.json'));
  for (const name of ['afplay', 'bun', 'bunx', 'osascript']) {
    assert.equal(Object.hasOwn(pkg.dependencies, name), false, `${name} runtime dependency`);
    assert.equal(Object.hasOwn(lock.packages[''].dependencies, name), false, `${name} lock root dependency`);
    assert.equal(Object.hasOwn(lock.packages, `node_modules/${name}`), false, `${name} orphan lock package`);
  }
});

test('clashing native extern declarations are compiler errors rather than accepted warning debt', () => {
  const main = read('src-tauri/src/main.rs');
  assert.match(main, /#!\[deny\(clashing_extern_declarations\)\]/);
  const files = [
    read('src-tauri/src/snipping/clipboard_process.rs'),
    read('src-tauri/src/stack_popup/text_document/feasibility/native.rs')
  ];
  for (const symbol of ['GetCurrentProcess', 'GetProcessHandleCount']) {
    const declarations = files.flatMap((source) => [...source.matchAll(new RegExp(`\\bfn ${symbol}\\(`, 'g'))]);
    assert.ok(declarations.length <= 1, `${symbol} must have one canonical binding`);
  }
  assert.doesNotMatch(files.join('\n'), /allow\([^)]*(?:warnings|clashing_extern_declarations)/);
});

test('dormant text-document feasibility is explicit opt-in, not every debug application build', () => {
  const module = read('src-tauri/src/stack_popup/text_document/mod.rs');
  const cargo = read('src-tauri/Cargo.toml');
  assert.match(module, /#\[cfg\(any\(test, feature = "text-document-feasibility"\)\)\]\s*pub\(crate\) mod feasibility;/);
  assert.match(cargo, /^text-document-feasibility\s*=\s*\[\]/m);
  assert.doesNotMatch(cargo, /^default\s*=.*text-document-feasibility/m);
});
