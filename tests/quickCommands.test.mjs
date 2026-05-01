import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  QUICK_COMMAND_MODES,
  coerceQuickCommandsSettings,
  defaultQuickCommandsSettings,
  formatQuickCommandArgsTextarea,
  parseQuickCommandArgsTextarea,
  quickCommandRunRequest
} from '../dist-tests/lib/quickCommands.js';

const source = readFileSync(new URL('../src/lib/quickCommands.ts', import.meta.url), 'utf8');

test('quick command wrapper exposes stable mode contract and defaults', () => {
  assert.deepEqual(QUICK_COMMAND_MODES, ['direct', 'powershellFile', 'cmdFile']);
  assert.deepEqual(defaultQuickCommandsSettings(), { entries: [] });
});

test('quick command settings coercion normalizes entries and validates security rules', () => {
  const settings = coerceQuickCommandsSettings({
    entries: [
      {
        id: 'Git-Status',
        label: '  Git Status  ',
        mode: 'direct',
        targetPath: 'git.exe',
        args: ['status', '--short'],
        cwd: null
      }
    ]
  });
  assert.deepEqual(settings.entries[0], {
    id: 'git-status',
    label: 'Git Status',
    mode: 'direct',
    targetPath: 'git.exe',
    args: ['status', '--short'],
    cwd: null
  });

  assert.throws(
    () =>
      coerceQuickCommandsSettings({
        entries: [
          {
            id: 'bad',
            label: 'Bad',
            mode: 'direct',
            targetPath: 'git.exe',
            args: ['--token secret'],
            cwd: null
          }
        ]
      }),
    /secret-like/
  );
});

test('quick command args textarea helpers preserve argv-per-line semantics', () => {
  assert.deepEqual(parseQuickCommandArgsTextarea('status\n--short\n\nmain'), ['status', '--short', 'main']);
  assert.equal(formatQuickCommandArgsTextarea(['status', '--short', 'main']), 'status\n--short\nmain');
});

test('quick command run request validates id and wrapper uses IPC constants', () => {
  assert.deepEqual(quickCommandRunRequest('  git-status  '), { id: 'git-status' });
  assert.throws(() => quickCommandRunRequest('   '), /must not be empty/);
  assert.match(source, /IPC_COMMANDS\.runQuickCommand/);
  assert.doesNotMatch(source, /invoke\('run_quick_command'/);
});
