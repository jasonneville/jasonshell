import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import test from 'node:test';

const packageJson = readFileSync(new URL('../package.json', import.meta.url), 'utf8');
const cleanScript = readFileSync(new URL('../scripts/clean-dist-tests.mjs', import.meta.url), 'utf8');
const gitignore = readFileSync(new URL('../.gitignore', import.meta.url), 'utf8');
const masterSpec = readFileSync(new URL('../master_spec.md', import.meta.url), 'utf8');

test('test:node cleans repo-local dist-tests before compiling helpers', () => {
  const scripts = JSON.parse(packageJson).scripts;
  assert.match(scripts['test:node'], /^node scripts\/clean-dist-tests\.mjs && tsc -p tsconfig\.test\.json && node --test tests\/\*\.test\.mjs$/);
  assert.match(cleanScript, /const repoRoot = path\.resolve/);
  assert.match(cleanScript, /const distTests = path\.resolve\(repoRoot, 'dist-tests'\)/);
  assert.match(cleanScript, /relative !== 'dist-tests'/);
  assert.match(cleanScript, /rmSync\(distTests, \{ recursive: true, force: true \}\)/);
});

test('generated dist-tests output stays ignored and documented as cleaned before compile', () => {
  assert.match(gitignore, /^dist-tests\/$/m);
  assert.match(masterSpec, /`npm run test:node`: `node scripts\/clean-dist-tests\.mjs && tsc -p tsconfig\.test\.json && node --test tests\/\*\.test\.mjs`/);
  assert.match(masterSpec, /`scripts\/clean-dist-tests\.mjs` refuses to clean anything except the repo-local `dist-tests` directory/);
});

test('tests do not import root-level stale dist-tests outputs', () => {
  const offenders = [];
  for (const name of readdirSync(new URL('.', import.meta.url)).filter((file) => file.endsWith('.test.mjs'))) {
    const source = readFileSync(new URL(name, import.meta.url), 'utf8');
    if (/\.\.\/dist-tests\/[^/'"`]+\.js/.test(source)) {
      offenders.push(name);
    }
  }
  assert.deepEqual(offenders, []);
});
