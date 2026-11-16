import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const read = (relativePath) => readFileSync(new URL(`../${relativePath}`, import.meta.url), 'utf8');

const readme = read('README.md');
const masterSpec = read('master_spec.md');
const currentBrief = read('docs/current-state-brief.md');
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
  assert.match(masterSpec, /primary-monitor AppBar work-area reservation/i);
  assert.match(masterSpec, /multi-monitor support is planning-only/i);
});

test('active product docs agree on all eight global shortcut action names', () => {
  const actions = ['Search', 'Terminal', 'Stack Browser', 'Speech transcription', 'Screen snipping', 'Process Manager', 'Settings', 'Quick Commands'];
  for (const [name, source] of [['README', readme], ['master_spec', masterSpec], ['current-state-brief', currentBrief]]) {
    const paragraph = source.split(/\r?\n/).find((line) => /(?:global shortcuts|global hotkeys)/i.test(line) && /eight/i.test(line) && actions.every((action) => line.includes(action)));
    assert.ok(paragraph, `${name} must identify eight global shortcuts`);
    for (const action of actions) assert.ok(paragraph.includes(action), `${name} omits ${action}`);
    for (const chord of ['Ctrl+Space', 'Alt+Backquote', 'Alt+1', 'Ctrl+D', 'Alt+S', 'Alt+P', 'Alt+O', 'Alt+Q']) {
      assert.ok(paragraph.includes(chord), `${name} omits default ${chord}`);
    }
    assert.doesNotMatch(source, /currently number seven|strict seven-field Save/);
  }
});

test('control plane user-facing copy keeps workspace startup/restoration and automation forwarding constrained', () => {
  assert.match(controlPlaneSurface, /workspace startup[^.;]*restoration[^.;]*planning-only/i);
  assert.match(controlPlaneSurface, /automation forwarding[^.;]*planned[^.;]*not wired/i);
  assert.match(controlPlaneState, /planning-only workspace startup[^.;]*restoration/i);
  assert.match(controlPlaneState, /startup commands[^.;]*restoration[^.;]*not executed/i);
});
