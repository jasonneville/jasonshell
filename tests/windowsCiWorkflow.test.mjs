import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

// CI-01: the workflow must run on the default branch and cover the component
// suite; redundant work removed.
const text = readFileSync(new URL('../.github/workflows/windows-ci.yml', import.meta.url), 'utf8').replace(/\r\n/g, '\n');

test('push trigger targets master (the default branch), not main', () => {
  const push = text.match(/push:\n\s+branches:\n((?:\s+- .+\n)+)/);
  assert.ok(push, 'push.branches list not found');
  assert.match(push[1], /- master\s*$/m);
  assert.ok(!/- main\s*$/m.test(push[1]), 'stale main branch trigger');
  assert.match(text, /^\s{2}pull_request:/m);
});

test('component tests run in CI', () => {
  assert.match(text, /run:\s*npm run test:component/);
});

test('rust cache is configured for src-tauri', () => {
  assert.match(text, /uses:\s*Swatinem\/rust-cache@v2/);
  assert.match(text, /workspaces:\s*src-tauri/);
});

test('redundant cargo:check step after cargo test is gone', () => {
  assert.ok(!/npm run cargo:check/.test(text));
  assert.match(text, /run:\s*npm run cargo:test/);
});

test('required validation steps remain', () => {
  for (const cmd of ['npm ci', 'npm run check', 'npm run build', 'npm run test:node']) {
    assert.ok(text.includes(`run: ${cmd}`), `missing ${cmd}`);
  }
});
