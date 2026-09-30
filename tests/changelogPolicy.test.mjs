import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

const masterSpec = readFileSync(new URL('../master_spec.md', import.meta.url), 'utf8');
const policy = readFileSync(new URL('../CHANGELOG_POLICY.md', import.meta.url), 'utf8');
const agents = readFileSync(new URL('../AGENTS.md', import.meta.url), 'utf8');
const docsIndex = readFileSync(new URL('../docs/README.md', import.meta.url), 'utf8');
const readme = readFileSync(new URL('../README.md', import.meta.url), 'utf8');
const packageJson = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
const archivedMasterSpec = readFileSync(
  new URL('../docs/archive/master_spec_full_legacy_2026-09-23.md', import.meta.url),
  'utf8',
);

const repoRoot = new URL('../', import.meta.url);

const localDocLinks = [...docsIndex.matchAll(/`([^`]+\.md|[^`]+\/)`/g)].map((match) => match[1]);
const rootDocLinks = new Set(['AGENTS.md', 'README.md', 'master_spec.md', 'CHANGELOG_POLICY.md', 'changelog.md']);

function resolveIndexedLink(link) {
  if (link.startsWith('docs/')) {
    return new URL(link, repoRoot);
  }

  if (link.startsWith('archive/') || link.startsWith('architecture/') || link.startsWith('plans/') || link.startsWith('remediation-plans/')) {
    return new URL(`docs/${link}`, repoRoot);
  }

  if (rootDocLinks.has(link)) {
    return new URL(link, repoRoot);
  }

  return new URL(`docs/${link}`, repoRoot);
}

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

test('docs onboarding contract keeps current intake narrow and archive historical', () => {
  assert.equal(existsSync(new URL('../docs/current-state-brief.md', import.meta.url)), true);
  assert.equal(existsSync(new URL('../docs/README.md', import.meta.url)), true);
  assert.equal(existsSync(new URL('../docs/archive/master_spec_full_legacy_2026-09-23.md', import.meta.url)), true);

  assert.match(archivedMasterSpec.slice(0, 800), /Historical archive/i);
  assert.match(archivedMasterSpec.slice(0, 800), /Superseded/i);
  assert.match(archivedMasterSpec.slice(0, 800), /not (a )?living spec/i);
  assert.match(archivedMasterSpec.slice(0, 800), /not mandatory intake/i);
  const archiveBanner = archivedMasterSpec.split('\n').slice(0, 7).join('\n');
  const sourceAuthorityIndex = archiveBanner.indexOf('source, tests, and observed runtime behavior are final');
  const masterSpecIndex = archiveBanner.indexOf('current `master_spec.md`');
  assert.notEqual(sourceAuthorityIndex, -1);
  assert.notEqual(masterSpecIndex, -1);
  assert.ok(
    sourceAuthorityIndex < masterSpecIndex,
    'archive banner must put source/tests/runtime final authority before prose references',
  );
  assert.match(archiveBanner, /AGENTS\.md/);
  assert.match(archiveBanner, /docs\/current-state-brief\.md/);
  assert.match(archiveBanner, /Use this archive only for targeted historical lookup/);

  assert.match(agents, /docs\/current-state-brief\.md/);
  assert.match(agents, /Do not read full history by default/);
  assert.match(agents, /Read full current `master_spec\.md` plus targeted historical material for architecture, security, persistence, native Windows, or cross-cutting work/);
  assert.doesNotMatch(agents, /Read full current `master_spec\.md`[^\n]+every task/);
  assert.doesNotMatch(agents, /Read full history[^\n]+every task/);

  assert.match(agents, /Changelog lookup is targeted and is not mandatory for every request/);
  assert.match(docsIndex, /Changelog\/history question/);
  assert.match(docsIndex, /keyword-search `changelog\.md`/);
});

test('docs index local pointers resolve and scheduled docs stay explicitly indexed', () => {
  assert.doesNotMatch(docsIndex, /scheduled\/\*_/);
  assert.doesNotMatch(docsIndex, /no scheduled-docs directory/i);
  assert.match(docsIndex, /Scheduled reports are point-in-time evidence, not current product truth/);
  assert.match(docsIndex, /docs\/scheduled\/09212026_WEEKLY_HEALTH_AUDIT\.md`/);
  assert.match(docsIndex, /latest weekly health audit/);
  assert.match(docsIndex, /docs\/scheduled\/09142026_WEEKLY_HEALTH_AUDIT\.md`/);
  assert.match(docsIndex, /historical comparison only/);
  assert.match(docsIndex, /docs\/scheduled\/09142026_session_learn\.md`/);
  assert.match(docsIndex, /workflow\/process observations, not product behavior/);

  for (const link of localDocLinks) {
    if (link.includes('*')) {
      continue;
    }

    const target = link.endsWith('/') ? link : link.replace(/#.*$/, '');
    assert.equal(existsSync(resolveIndexedLink(target)), true, `${link} should resolve`);
  }
});

test('README validation scripts match package scripts without stale copies', () => {
  const scriptBlock = readme.match(/Exact package scripts:\r?\n\r?\n```powershell\r?\n([\s\S]*?)\r?\n```/);
  assert.notEqual(scriptBlock, null);

  const documentedScripts = scriptBlock[1]
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => line.replace(/^npm run /, ''));

  assert.deepEqual(documentedScripts, Object.keys(packageJson.scripts));
  assert.ok(documentedScripts.includes('smoke:runtime'));
});
