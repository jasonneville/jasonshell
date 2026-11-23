import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

// Index-dependent hygiene (needs the untracking pass to have run). Read-only git.
const git = (...args) => {
  const r = spawnSync('git', args, { cwd: new URL('../', import.meta.url), encoding: 'utf8' });
  assert.equal(r.status, 0, r.stderr);
  return r.stdout.split('\n').map((l) => l.trim()).filter(Boolean);
};

test('HYG-04 bun.lock is not tracked', () => {
  assert.deepEqual(git('ls-files', 'bun.lock'), []);
});

test('HYG-02/03/09 no tracked file matches an ignore rule', () => {
  assert.deepEqual(git('ls-files', '-ci', '--exclude-standard'), []);
});
