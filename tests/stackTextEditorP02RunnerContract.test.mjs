import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { persistCommandRecord } from '../scripts/stack-text-editor/commandManifest.mjs';

const runner = await readFile('scripts/stack-text-editor/p02-run.mjs', 'utf8');

test('P02 runner keeps deferred P12 scale debt outside non-scale T02 gate', () => {
  assert.match(runner, /blockedOutcome\('P12\/NFR-6'/u);
  assert.match(runner, /const nonScaleOutcomes = outcomes\.filter/u);
  assert.match(runner, /:'IN_REVIEW'/u);
  assert.match(runner, /retained as non-gate P12\/NFR-6 debt/u);
});

test('P02 runner emits explicit review and coordinator disposition packet', () => {
  assert.match(runner, /review-packet\.json/u);
  assert.match(runner, /snapshotCompleteIsRecoveryComplete: false/u);
  assert.match(runner, /coordinatorDisposition: 'PENDING'/u);
});

test('P02 runner behaviorally persists actual native probe command record', async () => {
  const output = await mkdtemp(join(tmpdir(), 'p02-command-manifest-'));
  const commands = [{ id: '024', args: ['test'], exitCode: 0 }];
  const native = {
    id: '025',
    executable: 'cargo',
    args: ['run', '--manifest-path', 'src-tauri/Cargo.toml'],
    exitCode: 0,
    stdout: '025-actual-window-stdout.log',
    stderr: '025-actual-window-stderr.log',
  };

  await persistCommandRecord(output, commands, native);

  const persisted = JSON.parse(await readFile(join(output, 'commands.json'), 'utf8'));
  assert.deepEqual(persisted, [commands[0], native]);
});
