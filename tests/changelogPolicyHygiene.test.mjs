import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

const masterSpec = readFileSync(new URL('../master_spec.md', import.meta.url), 'utf8');
const changelog = readFileSync(new URL('../changelog.md', import.meta.url), 'utf8');
const policyUrl = new URL('../CHANGELOG_POLICY.md', import.meta.url);
const agentsUrl = new URL('../AGENTS.md', import.meta.url);

test('master spec stays behavior-focused and delegates per-change history to changelog policy', () => {
  assert.doesNotMatch(masterSpec, /^## Mandatory first-step ledger protocol/m);
  assert.doesNotMatch(masterSpec, /^## Change Ledger/m);
  assert.doesNotMatch(masterSpec, /Add a change-ledger entry for every user request/);
  assert.doesNotMatch(masterSpec, /Immediately append a new `Change Ledger` entry/);
  assert.match(masterSpec, /`master_spec\.md` is the durable, compaction-safe master briefing/);
  assert.match(masterSpec, /Per-change history belongs in `changelog\.md` under `CHANGELOG_POLICY\.md`/);
});

test('dedicated changelog policy owns future ledger protocol without rewriting history', () => {
  assert.equal(existsSync(policyUrl), true);
  const policy = readFileSync(policyUrl, 'utf8');
  assert.match(policy, /^# JasonShell Changelog Policy/m);
  assert.match(policy, /`changelog\.md` is the append-only repository change history/);
  assert.match(policy, /Do not require a `changelog\.md` entry before every request/);
  assert.match(policy, /`master_spec\.md` remains canonical for behavior and architecture/);
  assert.match(policy, /Source-test docs contract/);
  assert.match(changelog, /^## Change Ledger/m);
  assert.match(changelog, /2026-04-29T22:53:18-05:00/);
});

test('repo agent instructions route changelog work out of master spec', () => {
  assert.equal(existsSync(agentsUrl), true);
  const agents = readFileSync(agentsUrl, 'utf8');
  assert.match(agents, /Read `master_spec\.md` first/);
  assert.match(agents, /Use `CHANGELOG_POLICY\.md` for changelog rules/);
  assert.doesNotMatch(agents, /Mandatory first-step ledger protocol/);
  assert.doesNotMatch(agents, /append a new `Change Ledger` entry before implementation/);
});
