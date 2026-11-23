import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import test from 'node:test';
import { readRepoFile } from './support/repoSource.mjs';

const testsDir = new URL('./', import.meta.url);

// Patterns that silently turn a missing source file into '' and let assertions pass/skip vacuously.
const FORBIDDEN = [
  { name: "existsSync(...) ? readFileSync(...) : ''", re: /existsSync\([^;]*?\?\s*readFileSync\([^;]*?:\s*(?:''|""|``)/ },
  { name: "readFileSync in try/catch returning ''", re: /try\s*\{[^}]*readFileSync\([^}]*\}\s*catch(?:\s*\([^)]*\))?\s*\{\s*return\s*(?:''|""|``)/ },
  { name: ".catch(() => '') on a file read", re: /readFile\([^;]*\.catch\(\s*\(\)\s*=>\s*(?:''|"")/ },
  { name: 'readFileSync(...) ?? \'\'', re: /readFileSync\([^;]*\)\s*\?\?\s*(?:''|"")/ },
];

test('readRepoFile throws a clear error for a missing file', () => {
  assert.throws(() => readRepoFile('src/definitely-missing-file.ts'), /missing or unreadable: src\/definitely-missing-file\.ts/);
  assert.ok(readRepoFile('package.json').includes('"name"'));
});

test('test files do not silently fall back to empty source on a missing file', () => {
  const offenders = [];
  const files = readdirSync(testsDir, { withFileTypes: true }).filter((e) => e.isFile() && /\.test\.mjs$/.test(e.name) && e.name !== 'missingSourceGuard.test.mjs');
  for (const { name } of files) {
    const text = readFileSync(new URL(name, testsDir), 'utf8');
    for (const { name: label, re } of FORBIDDEN) if (re.test(text)) offenders.push(`${name}: ${label}`);
  }
  assert.deepEqual(offenders, [], 'use tests/support/repoSource.mjs readRepoFile (throws on missing files)');
});

test('guard patterns detect the known bad shapes', () => {
  assert.ok(FORBIDDEN[0].re.test("return existsSync(url) ? readFileSync(url, 'utf8') : '';"));
  assert.ok(FORBIDDEN[1].re.test("try { return readFileSync(u, 'utf8'); } catch { return ''; }"));
});
