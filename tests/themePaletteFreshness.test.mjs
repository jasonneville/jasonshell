import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const repository = fileURLToPath(new URL('../', import.meta.url));
const scriptPath = 'scripts/generate-theme-palettes.mjs';
const cssPath = 'src/app.css';
const registryPath = 'src/lib/themes.ts';
const generatedPath = 'src/lib/builtinThemePalettes.generated.ts';
const asLF = text => text.replace(/\r\n/g, '\n');
const asCRLF = text => asLF(text).replace(/\n/g, '\r\n');

function fixture(t, targetEOL = asLF) {
  assert.ok(process.env.LOCALAPPDATA, 'approved Windows LOCALAPPDATA location required');
  const root = mkdtempSync(join(process.env.LOCALAPPDATA, 'Temp', 'opencode', 'theme-freshness-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  for (const relative of [scriptPath, cssPath, registryPath, generatedPath]) {
    const destination = join(root, relative);
    mkdirSync(dirname(destination), { recursive: true });
    const original = readFileSync(join(repository, relative));
    // The generator is copied byte-for-byte. Only isolated fixture input EOLs
    // change; never rewrite repository source/generated files for this test.
    writeFileSync(destination, relative === scriptPath ? original
      : (relative === generatedPath ? targetEOL : asLF)(original.toString('utf8')));
  }
  return root;
}

function run(root, check = true) {
  const result = spawnSync(process.execPath, [join(root, scriptPath), ...(check ? ['--check'] : [])], {
    cwd: root, encoding: 'utf8', timeout: 10_000
  });
  assert.equal(result.error, undefined, result.error?.message);
  return result;
}

test('freshness checker accepts equivalent LF generated target', t => {
  const result = run(fixture(t));
  assert.equal(result.status, 0, result.stderr);
});

test('freshness checker accepts equivalent CRLF generated target', t => {
  const result = run(fixture(t, asCRLF));
  assert.equal(result.status, 0, result.stderr);
});

test('LF and CRLF source inputs generate identical palette data', t => {
  const root = fixture(t);
  assert.equal(run(root).status, 0);
  assert.equal(run(root, false).status, 0);
  const expected = readFileSync(join(root, generatedPath), 'utf8');
  for (const relative of [cssPath, registryPath]) {
    const path = join(root, relative);
    writeFileSync(path, asCRLF(readFileSync(path, 'utf8')));
  }
  const checked = run(root);
  assert.equal(checked.status, 0, checked.stderr);
  const generated = run(root, false);
  assert.equal(generated.status, 0, generated.stderr);
  assert.equal(readFileSync(join(root, generatedPath), 'utf8'), expected);
});

test('semantic generated color drift still fails with the stale-palette diagnostic', t => {
  const root = fixture(t);
  const target = join(root, generatedPath);
  const original = readFileSync(target, 'utf8');
  const changed = original.replace('"--js-color-bg": "#090c13"', '"--js-color-bg": "#123456"');
  assert.notEqual(changed, original, 'fixture must change a real palette value');
  writeFileSync(target, changed);
  const result = run(root);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Builtin palette data stale/);
});
