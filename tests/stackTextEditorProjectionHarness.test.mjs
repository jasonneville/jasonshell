import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import {
  cancelCompositionOnBlur,
  createImeHarnessState,
  finishComposition,
  noteCommittedInput,
  reconcileAdjacentLease,
  redactEventText,
  startComposition,
} from '../dist-tests/features/stack-browser/p03NativeHarness.js';

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), 'utf8');

test('P03 native harness has exact test guard and remains outside product graph', async () => {
  const [host, launcher, app, main, surfaceLoader] = await Promise.all([
    read('src-tauri/examples/stack_text_p03.rs'),
    read('scripts/stack-text-editor/launch-p03-experiment.ps1'),
    read('src/p03-experiment.ts'),
    read('src/main.ts'),
    read('src/lib/surfaceLoader.ts'),
  ]);
  assert.match(host, /--p03-experiment/);
  assert.match(host, /P03_EXPERIMENT_ONLY/);
  assert.match(launcher, /--p03-experiment/);
  assert.match(app, /TextEditorProjectionExperiment\.svelte/);
  assert.doesNotMatch(`${main}\n${surfaceLoader}`, /TextEditorProjectionExperiment|p03-experiment/);
});

test('P03 surface uses bounded real projection fixtures without fake newline or full a11y mirror', async () => {
  const surface = await read('src/components/TextEditorProjectionExperiment.svelte');
  assert.match(surface, /GIANT_COMBINING_MARKS\s*=\s*20_000/);
  assert.match(surface, /PROJECTION_LIMITS\.units/);
  assert.match(surface, /EditorView\.updateListener/);
  assert.match(surface, /compositionstart/);
  assert.match(surface, /key:\s*'Ctrl-Shift-l'/);
  assert.doesNotMatch(surface, /on:click=\{arriveAdjacentLease\}/);
  assert.match(surface, /navigator\.clipboard\.writeText/);
  assert.doesNotMatch(surface, /aria-(?:label|description)\s*=\s*\{?(?:full|document|source)/i);
  assert.doesNotMatch(surface, /dispatchEvent|new\s+(?:CompositionEvent|InputEvent)/);
});

test('P03 host bounds event and artifact writes', async () => {
  const host = await read('src-tauri/examples/stack_text_p03.rs');
  assert.match(host, /MAX_EVENTS:\s*usize\s*=\s*4096/);
  assert.match(host, /MAX_EVENT_BYTES:\s*usize\s*=\s*16 \* 1024/);
  assert.match(host, /create_new\(true\)/);
  assert.match(host, /p03-events\.ndjson/);
});

test('editor-focused adjacent arrival preserves active composition and canonical two-lease state', () => {
  let state = startComposition(createImeHarnessState('left 境'));
  state = reconcileAdjacentLease(state, 'right', '界 adjacent');
  assert.equal(state.composing, true);
  assert.equal(state.compositionPinnedLeaseId, 'left');
  assert.deepEqual(state.leases.map(({ id, text }) => ({ id, text })), [
    { id: 'left', text: 'left 境' },
    { id: 'right', text: '界 adjacent' },
  ]);
  assert.equal(state.projection, 'left 境界 adjacent');
  const replaced = reconcileAdjacentLease(state, 'right-2', 'replacement');
  assert.deepEqual(replaced.leases.map(lease => lease.id), ['left', 'right-2']);
  assert.equal(replaced.projection, 'left 境replacement');
});

test('composition end is pending until trusted committed input; cancellation and blur commit nothing', () => {
  let committed = finishComposition(startComposition(createImeHarnessState('left')), '候補');
  assert.equal(committed.inputCommits, 0);
  committed = noteCommittedInput(committed, { isTrusted: true, inputType: 'insertCompositionText', data: '候補' });
  assert.equal(committed.inputCommits, 1);
  assert.equal(committed.lastOutcome, 'commit');
  assert.equal(noteCommittedInput(committed, { isTrusted: true, inputType: 'insertCompositionText', data: 'dup' }).inputCommits, 1);

  const cancelled = finishComposition(startComposition(createImeHarnessState('left')), '');
  assert.equal(cancelled.lastOutcome, 'cancel');
  assert.equal(cancelled.inputCommits, 0);

  const blurred = cancelCompositionOnBlur(startComposition(createImeHarnessState('left')));
  assert.equal(blurred.lastOutcome, 'blur');
  assert.equal(blurred.inputCommits, 0);
});

test('event text is redacted and bounded', () => {
  assert.deepEqual(redactEventText('候補secret'), { units: 8, preview: '[text:8u]' });
  assert.deepEqual(redactEventText('x'.repeat(10_000)), { units: 10_000, preview: '[text:10000u]' });
});
