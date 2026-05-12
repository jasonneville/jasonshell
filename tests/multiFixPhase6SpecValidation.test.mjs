import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const masterSpec = readFileSync(new URL('../master_spec.md', import.meta.url), 'utf8');

test('master spec records multi-fix phase 5 and phase 6 durable behavior', () => {
  for (const phrase of [
    'Multi-fix Phase 5/6',
    'Settings power actions',
    'trigger_system_power_action',
    'in-panel confirmation',
    'no native confirm()',
    'Windows power API',
    'shutdown.exe argument-vector',
    'settingsPowerActionsPhase5.test.mjs',
    'multiFixPhase6SpecValidation.test.mjs'
  ]) {
    assert.match(masterSpec, new RegExp(phrase.replace(/[()]/g, '\\$&')));
  }
});

test('master spec validation notes include required phase acceptance checks', () => {
  for (const phrase of [
    'search typing',
    'Stack Browser rectangle selection',
    'bottom quick icons',
    'VS Code folder actions',
    'settings confirmations',
    'npm run validate'
  ]) {
    assert.match(masterSpec, new RegExp(phrase));
  }
});
