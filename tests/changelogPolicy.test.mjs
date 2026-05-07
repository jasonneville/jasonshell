import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

const masterSpec = readFileSync(new URL('../master_spec.md', import.meta.url), 'utf8');
const policy = readFileSync(new URL('../CHANGELOG_POLICY.md', import.meta.url), 'utf8');
const agents = readFileSync(new URL('../AGENTS.md', import.meta.url), 'utf8');

test('master spec no longer owns per-request change ledger protocol', () => {
  assert.doesNotMatch(masterSpec, /## Change Ledger/);
  assert.doesNotMatch(masterSpec, /Mandatory first-step ledger protocol/);
  assert.doesNotMatch(masterSpec, /Immediately append a new `Change Ledger` entry/);
  assert.match(masterSpec, /changelog\.md/);
});

test('dedicated changelog policy and agent instructions route history out of master spec', () => {
  assert.equal(existsSync(new URL('../changelog.md', import.meta.url)), true);
  assert.match(policy, /changelog\.md/);
  assert.match(policy, /not `master_spec\.md`/);
  assert.match(agents, /changelog\.md/);
  assert.doesNotMatch(agents, /Master Spec Ledger/);
  assert.doesNotMatch(agents, /Immediately append a new `Change Ledger` entry/);
});
