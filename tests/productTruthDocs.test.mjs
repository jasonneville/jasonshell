import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const read = (relativePath) => readFileSync(new URL(`../${relativePath}`, import.meta.url), 'utf8');

const readme = read('README.md');
const masterSpec = read('master_spec.md');
const controlPlaneSurface = read('src/components/ControlPlaneSurface.svelte');
const controlPlaneState = read('src/features/control-plane/controlPlaneState.ts');

test('README pairs required product truths with near-copy language', () => {
  assert.match(readme, /workspace restoration[^.;]*(?:is )?reserved[^.;]*not implemented/i);
  assert.match(readme, /startup commands[^.;]*not executed automatically/i);
  assert.match(readme, /automation forwarding[^.;]*planned[^.;]*not wired/i);
  assert.match(readme, /multi-monitor[^.;]*planning-only[^.;]*single-monitor runtime/i);
});

test('master_spec repeats the same four truths', () => {
  assert.match(masterSpec, /workspace restoration[^.;]*(?:is )?reserved[^.;]*not implemented/i);
  assert.match(masterSpec, /startup commands[^.;]*not executed automatically/i);
  assert.match(masterSpec, /automation forwarding[^.;]*planned[^.;]*not wired/i);
  assert.match(masterSpec, /multi-monitor[^.;]*planning-only[^.;]*single-monitor runtime/i);
});

test('control plane user-facing copy keeps workspace startup/restoration and automation forwarding constrained', () => {
  assert.match(controlPlaneSurface, /workspace startup[^.;]*restoration[^.;]*planning-only/i);
  assert.match(controlPlaneSurface, /automation forwarding[^.;]*planned[^.;]*not wired/i);
  assert.match(controlPlaneState, /planning-only workspace startup[^.;]*restoration/i);
  assert.match(controlPlaneState, /startup commands[^.;]*restoration[^.;]*not executed/i);
});
