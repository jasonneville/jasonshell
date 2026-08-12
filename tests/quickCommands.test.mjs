import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  QUICK_COMMAND_MODES,
  coerceQuickCommandsSettings,
  defaultQuickCommandsSettings,
  formatQuickCommandArgsTextarea,
  formatQuickCommandCommandsTextarea,
  parseQuickCommandArgsTextarea,
  parseQuickCommandCommandsTextarea,
  quickCommandRunRequest,
  listQuickCommandHistory,
  saveQuickCommandsSettings
} from '../dist-tests/lib/quickCommands.js';

const source = readFileSync(new URL('../src/lib/quickCommands.ts', import.meta.url), 'utf8');

test('quick command wrapper exposes stable mode contract and defaults', () => {
  assert.deepEqual(QUICK_COMMAND_MODES, ['direct', 'commandBlock']);
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
        commands: [],
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
    commands: [],
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
            commands: [],
            cwd: null
          }
        ]
      }),
    /secret-like/
  );
});

test('quick command settings support sequential command blocks', () => {
  const settings = coerceQuickCommandsSettings({
    entries: [
      {
        id: 'Run-Server',
        label: 'Run Server',
        mode: 'commandBlock',
        targetPath: '',
        args: ['ignored'],
        commands: ['cd C:\\dev\\app', 'python app.py'],
        cwd: 'C:\\dev'
      }
    ]
  });
  assert.deepEqual(settings.entries[0], {
    id: 'run-server',
    label: 'Run Server',
    mode: 'commandBlock',
    targetPath: '',
    args: [],
    commands: ['cd C:\\dev\\app', 'python app.py'],
    cwd: 'C:\\dev'
  });

  assert.throws(
    () =>
      coerceQuickCommandsSettings({
        entries: [
          {
            id: 'empty-block',
            label: 'Empty',
            mode: 'commandBlock',
            targetPath: '',
            args: [],
            commands: [],
            cwd: null
          }
        ]
      }),
    /at least one command/
  );
});

test('quick command args textarea helpers preserve argv-per-line semantics', () => {
  assert.deepEqual(parseQuickCommandArgsTextarea('status\n--short\n\nmain'), ['status', '--short', 'main']);
  assert.equal(formatQuickCommandArgsTextarea(['status', '--short', 'main']), 'status\n--short\nmain');
});

test('quick command block textarea helpers preserve command-per-line semantics', () => {
  assert.deepEqual(parseQuickCommandCommandsTextarea('cd C:\\dev\\app\npython app.py\n'), [
    'cd C:\\dev\\app',
    'python app.py'
  ]);
  assert.equal(
    formatQuickCommandCommandsTextarea(['cd C:\\dev\\app', 'python app.py']),
    'cd C:\\dev\\app\npython app.py'
  );
});

test('quick command run request validates id and wrapper uses IPC constants', () => {
  assert.deepEqual(quickCommandRunRequest('  git-status  '), { id: 'git-status' });
  assert.throws(() => quickCommandRunRequest('   '), /must not be empty/);
  assert.match(source, /IPC_COMMANDS\.runQuickCommand/);
  assert.match(source, /IPC_COMMANDS\.listQuickCommandHistory/);
  assert.match(source, /IPC_COMMANDS\.saveQuickCommandsSettings/);
  assert.equal(typeof listQuickCommandHistory, 'function');
  assert.equal(typeof saveQuickCommandsSettings, 'function');
  assert.doesNotMatch(source, /invoke\('run_quick_command'/);
});
