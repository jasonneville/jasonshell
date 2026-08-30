import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const scriptUrl = new URL('../scripts/runtime-smoke.ps1', import.meta.url);
const packageJson = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
const smokeDocs = readFileSync(new URL('../docs/smoke-test-windows.md', import.meta.url), 'utf8');

function runtimeSmokeScript() {
  assert.ok(existsSync(scriptUrl), 'scripts/runtime-smoke.ps1 must exist');
  return readFileSync(scriptUrl, 'utf8');
}

function runRuntimeSmoke(args = []) {
  const resultsRoot = mkdtempSync(join(tmpdir(), 'jasonshell-runtime-smoke-'));
  const result = spawnSync(
    'pwsh',
    ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', fileURLToPath(scriptUrl), '-ResultsRoot', resultsRoot, ...args],
    { encoding: 'utf8' }
  );
  const artifactDirs = readdirSync(resultsRoot);
  assert.equal(artifactDirs.length, 1, `expected one artifact directory; stderr: ${result.stderr}`);
  const evidence = JSON.parse(readFileSync(join(resultsRoot, artifactDirs[0], 'evidence.json'), 'utf8'));
  rmSync(resultsRoot, { recursive: true, force: true });
  return { evidence, result };
}

test('runtime smoke script defaults to dry run and records evidence path', () => {
  const script = runtimeSmokeScript();

  assert.match(script, /\[switch\]\s*\$DryRun\s*=\s*\$true/i);
  assert.match(script, /test-results[\\/]runtime-smoke/i);
  assert.match(script, /evidence\.json/);
  assert.match(script, /summary\.md/);
  assert.match(script, /manual-evidence-template\.md/);
  assert.match(script, /schemaVersion/);
  assert.match(script, /startedAt/);
  assert.doesNotMatch(script, /(?:COMPUTERNAME|USERNAME)/);
  assert.doesNotMatch(script, /npm\s+run\s+tauri\s+dev/i);

  const { evidence, result } = runRuntimeSmoke();
  assert.equal(result.status, 0, result.stderr);
  assert.equal(evidence.mode, 'dry-run');
  assert.equal(evidence.consent.DryRun, true);
  assert.notEqual(evidence.overallStatus, 'passed', 'unperformed consent-gated checks prevent overall pass');
});

test('runtime smoke script requires explicit consent for desktop mutation and process termination', () => {
  const script = runtimeSmokeScript();

  assert.match(script, /ConsentDesktopMutation/);
  assert.match(script, /ConsentProcessTermination/);
  assert.match(script, /ConsentGlobalHooks/);
  assert.match(script, /DryRun/);

  for (const riskyStep of ['desktop-mutation', 'global-hooks', 'process-termination']) {
    assert.match(script, new RegExp(`${riskyStep}[\\s\\S]{0,220}(?:Consent|blocked)`, 'i'), `${riskyStep} must name a consent gate near its check`);
  }

  const { evidence, result } = runRuntimeSmoke([
    '-DryRun:$false',
    '-ConsentDesktopMutation',
    '-ConsentGlobalHooks',
    '-ConsentProcessTermination'
  ]);
  assert.notEqual(result.status, 0, 'live request must fail while live smoke is unimplemented');
  assert.equal(evidence.mode, 'live-requested');
  assert.equal(evidence.overallStatus, 'blocked');
  assert.ok(evidence.blocked.includes('DryRun=false live smoke not implemented'));
});

test('runtime smoke docs forbid automated assistive technology claims without manual evidence', () => {
  assert.match(smokeDocs, /Do not claim automated assistive-technology support/i);
  assert.match(smokeDocs, /NVDA, JAWS, DPI\/scaling, and multi-monitor evidence is manual-only/i);
});

test('package scripts expose non-destructive runtime smoke entrypoint', () => {
  const script = packageJson.scripts?.['smoke:runtime'];

  assert.equal(typeof script, 'string', 'package.json must expose scripts.smoke:runtime');
  assert.match(script, /runtime-smoke\.ps1/);
  assert.match(script, /-DryRun\b/);
  assert.doesNotMatch(script, /-DryRun:\$false/i);
  assert.doesNotMatch(script, /-Consent(?:DesktopMutation|ProcessTermination|GlobalHooks)\b/i);
});

test('official node tests clean dist-tests before execution', () => {
  const script = packageJson.scripts?.['test:node'] ?? '';

  assert.match(script, /node scripts\/clean-dist-tests\.mjs/);
  assert.match(script, /tsc -p tsconfig\.test\.json/);
  assert.match(script, /node --test tests\/\*\.test\.mjs/);
  assert.ok(
    script.indexOf('node scripts/clean-dist-tests.mjs') < script.indexOf('tsc -p tsconfig.test.json') &&
      script.indexOf('tsc -p tsconfig.test.json') < script.indexOf('node --test tests/*.test.mjs'),
    'test:node must clean dist-tests, compile, then run node tests'
  );
});
