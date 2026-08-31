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
  nextDuplicateQuickCommandLabel,
  nextUniqueQuickCommandId,
  quickCommandRunRequest,
  deriveQuickCommandPendingInputRequest,
  normalizeQuickCommandInputMaxLength,
  normalizeQuickCommandInputValue,
  normalizeQuickCommandsListWidth,
  mergeQuickCommandRunHistoryEntries,
  listQuickCommandHistory,
  stopQuickCommand,
  saveQuickCommandsSettings
} from '../dist-tests/lib/quickCommands.js';

const quickCommandsSource = readFileSync(new URL('../src-tauri/src/quick_commands.rs', import.meta.url), 'utf8');

const source = readFileSync(new URL('../src/lib/quickCommands.ts', import.meta.url), 'utf8');

test('quick command wrapper exposes stable mode contract and defaults', () => {
  assert.deepEqual(QUICK_COMMAND_MODES, ['direct', 'commandBlock']);
  assert.deepEqual(defaultQuickCommandsSettings(), { entries: [], history: [], listWidth: 180 });
  assert.equal(normalizeQuickCommandsListWidth(99), 128);
  assert.equal(normalizeQuickCommandsListWidth(999), 420);
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
  const request = quickCommandRunRequest('  git-status  ');
  assert.deepEqual(request, { id: 'git-status' });
  assert.throws(() => quickCommandRunRequest('   '), /must not be empty/);
  assert.match(source, /IPC_COMMANDS\.runQuickCommand/);
  assert.match(source, /IPC_COMMANDS\.listQuickCommandHistory/);
  assert.match(source, /IPC_COMMANDS\.saveQuickCommandsSettings/);
  assert.match(source, /IPC_COMMANDS\.sendQuickCommandInput/);
  assert.equal(typeof listQuickCommandHistory, 'function');
  assert.equal(typeof saveQuickCommandsSettings, 'function');
  assert.equal(typeof stopQuickCommand, 'function');
  assert.doesNotMatch(source, /invoke\('run_quick_command'/);
  assert.match(source, /IPC_COMMANDS\.stopQuickCommand/);
  assert.match(source, /IPC_COMMANDS\.openQuickCommandUrl/);
});

test('quick command URL opener is command-panel only and rejects unsafe URLs', () => {
  assert.match(quickCommandsSource, /OPEN_QUICK_COMMAND_URL/);
  assert.match(quickCommandsSource, /COMMAND_PANEL_LABEL/);
  assert.match(quickCommandsSource, /validate_quick_command_url/);
  assert.match(quickCommandsSource, /ShellExecuteW failed to open quick command URL/);
  assert.match(quickCommandsSource, /Opening quick command URLs is only supported on Windows/);
  assert.match(quickCommandsSource, /authority\.contains\('@'\)/);
  assert.match(quickCommandsSource, /chars\(\)\.any\(char::is_whitespace\)/);
  assert.match(quickCommandsSource, /strip_prefix\("https:\/\/"\)/);
  assert.match(quickCommandsSource, /strip_prefix\("http:\/\/"\)/);
});

test('quick command URL validator accepts http and https and rejects unsafe forms', () => {
  assert.match(quickCommandsSource, /validate_quick_command_url\("http:\/\/example\.com"\)/);
  assert.match(quickCommandsSource, /validate_quick_command_url\("https:\/\/example\.com\/path\?q=1#frag"\)/);
  assert.match(quickCommandsSource, /validate_quick_command_url\("javascript:alert\(1\)"\)/);
  assert.match(quickCommandsSource, /validate_quick_command_url\("https:\/\/example\.com\/with space"\)/);
  assert.match(quickCommandsSource, /validate_quick_command_url\("https:\/\/user:pass@example\.com"\)/);
  assert.match(quickCommandsSource, /validate_quick_command_url\("https:\/\/"\)/);
});

test('quick command settings discard legacy history without a run id', () => {
  const settings = coerceQuickCommandsSettings({
    entries: [],
    history: [{ commandId: 'old-command', startedAtEpochMs: 1 }]
  });
  assert.deepEqual(settings.history, []);
});

test('quick command pending input derives from transcript request markers', () => {
  const pending = deriveQuickCommandPendingInputRequest({
    runId: 'run-1',
    commandId: 'deploy',
    processId: 123,
    transcript: [
      { kind: 'output', body: 'hi', requestId: null, prompt: null, secret: false, redacted: false, sequence: 1, atEpochMs: 10, pending: false },
      { kind: 'input-request', body: '', requestId: 'req-1', prompt: 'Password', secret: true, redacted: true, maxLength: 8, sequence: 2, atEpochMs: 20, pending: true }
    ]
  });

  assert.deepEqual(pending, {
    runId: 'run-1',
    commandId: 'deploy',
    processId: 123,
    requestId: 'req-1',
    kind: 'password',
    prompt: 'Password',
    secret: true,
    redacted: true,
    maxLength: 8,
    sequence: 2,
    atEpochMs: 20,
    pending: true
  });
});

test('quick command input helpers bound max length and trim unicode by code point', () => {
  assert.equal(normalizeQuickCommandInputMaxLength(0), 1);
  assert.equal(normalizeQuickCommandInputMaxLength(99_999), 16384);
  assert.equal(normalizeQuickCommandInputMaxLength(12.8), 12);
  assert.equal(normalizeQuickCommandInputValue('a🙂b', 2), 'a🙂');
});

test('quick command history merge flips running false on exit snapshots', () => {
  const merged = mergeQuickCommandRunHistoryEntries(
    [{ runId: 'run-1', commandId: 'a', startedAtEpochMs: 1, finishedAtEpochMs: 2, processId: 3, exitCode: null, stdout: 'old', stderr: 'old-err', stdoutTruncated: true, stderrTruncated: true, running: true, transcript: [{ kind: 'output', body: 'A', requestId: null, prompt: null, secret: false, redacted: false, sequence: 1, atEpochMs: 1, pending: false }] }],
    [{ runId: 'run-1', commandId: 'a', startedAtEpochMs: 1, finishedAtEpochMs: 3, processId: 3, exitCode: 0, stdout: '', stderr: '', stdoutTruncated: false, stderrTruncated: false, running: false, transcript: [{ kind: 'exit', body: '0', requestId: null, prompt: null, secret: false, redacted: false, sequence: 9, atEpochMs: 3, pending: false }] }]
  );
  assert.equal(merged[0].running, false);
  assert.equal(merged[0].exitCode, 0);
  assert.equal(merged[0].stdout, '');
  assert.equal(merged[0].stderr, '');
  assert.equal(merged[0].stdoutTruncated, false);
  assert.equal(merged[0].stderrTruncated, false);
  assert.equal(merged[0].transcript.at(-1)?.kind, 'exit');
});

test('quick command history merge cannot resurrect ended run from stale running response', () => {
  const ended = { runId: 'run-ended', commandId: 'a', startedAtEpochMs: 1, finishedAtEpochMs: 3, processId: 3, exitCode: null, stdout: 'done', stderr: '', stdoutTruncated: false, stderrTruncated: false, running: false, transcript: [{ kind: 'stopped', body: '', requestId: null, prompt: null, secret: false, redacted: false, sequence: 9, atEpochMs: 3, pending: false }] };
  const staleRunning = { ...ended, finishedAtEpochMs: 2, running: true, transcript: [{ kind: 'stopping', body: '', requestId: null, prompt: null, secret: false, redacted: false, sequence: 8, atEpochMs: 2, pending: false }] };

  const [merged] = mergeQuickCommandRunHistoryEntries([ended], [staleRunning]);

  assert.equal(merged.running, false);
  assert.equal(merged.finishedAtEpochMs, 3);
  assert.equal(merged.transcript.at(-1)?.kind, 'stopped');
});

test('quick command pending input maps prompt kinds and confirm can be empty', () => {
  const pending = deriveQuickCommandPendingInputRequest({
    runId: 'run-2',
    commandId: 'confirm',
    processId: 5,
    transcript: [
      { kind: 'confirm', body: '', requestId: 'req-2', prompt: 'Continue?', secret: false, redacted: false, maxLength: 24, sequence: 1, atEpochMs: 1, pending: true }
    ]
  });

  assert.deepEqual(pending, {
    runId: 'run-2',
    commandId: 'confirm',
    processId: 5,
    requestId: 'req-2',
    kind: 'confirm',
    prompt: 'Continue?',
    secret: false,
    redacted: false,
    maxLength: 24,
    sequence: 1,
    atEpochMs: 1,
    pending: true
  });
});

test('quick command history merge prefers runId and transcript sequence', () => {
  const merged = mergeQuickCommandRunHistoryEntries(
    [{ runId: 'run-1', commandId: 'a', startedAtEpochMs: 1, finishedAtEpochMs: 2, processId: 3, exitCode: null, stdout: '', stderr: '', stdoutTruncated: false, stderrTruncated: false, running: true, transcript: [{ kind: 'output', body: 'A', requestId: null, prompt: null, secret: false, redacted: false, sequence: 1, atEpochMs: 1, pending: false }] }],
    [{ runId: 'run-1', commandId: 'a', startedAtEpochMs: 1, finishedAtEpochMs: 2, processId: 3, exitCode: null, stdout: '', stderr: '', stdoutTruncated: false, stderrTruncated: false, running: true, transcript: [{ kind: 'input-request', body: '', requestId: 'r', prompt: 'p', secret: false, redacted: false, sequence: 2, atEpochMs: 2, pending: true }] }]
  );
  assert.equal(merged[0].transcript.length, 2);
  assert.equal(merged[0].transcript[0].sequence, 1);
  assert.equal(merged[0].transcript[1].sequence, 2);
});

test('quick command backend emits merged transcript snapshots with ordered stream chunks', () => {
  assert.match(quickCommandsSource, /append_running_output/);
  assert.match(quickCommandsSource, /kind = if is_stdout \{ "stdout" \} else \{ "stderr" \}/);
  assert.match(quickCommandsSource, /push_transcript\(/);
  assert.match(quickCommandsSource, /emit_run_updated_from_transcript\(/);
  assert.match(quickCommandsSource, /sequence: next_sequence\(\)/);
  assert.match(quickCommandsSource, /VecDeque<QuickCommandTranscriptEntry>/);
  assert.doesNotMatch(quickCommandsSource, /remove\(0\)/);
});

test('quick command backend uses suspended spawn plus per-run job object stop authority', () => {
  assert.match(quickCommandsSource, /System::JobObjects/);
  assert.match(quickCommandsSource, /CREATE_SUSPENDED/);
  assert.match(quickCommandsSource, /CreateJobObjectW/);
  assert.match(quickCommandsSource, /JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE/);
  assert.match(quickCommandsSource, /AssignProcessToJobObject/);
  assert.match(quickCommandsSource, /PROCESS_SET_QUOTA/);
  assert.match(quickCommandsSource, /ResumeThread/);
  assert.match(quickCommandsSource, /TerminateJobObject/);
  assert.doesNotMatch(quickCommandsSource, /taskkill\.exe/i);
});

test('quick command backend assigns one sequence per terminal semantic entry and reuses it in payload plus persisted transcript', () => {
  assert.match(quickCommandsSource, /let terminal_kind = if stopped \{ "stopped" \} else \{ "exit" \}/);
  assert.match(quickCommandsSource, /let sequence = next_sequence\(\);[\s\S]*?push_transcript\([\s\S]*?kind: terminal_kind\.into\(\),[\s\S]*?sequence,[\s\S]*?\}\);/);
  assert.match(quickCommandsSource, /kind: terminal_kind\.into\(\),[\s\S]*?sequence,/);
  assert.doesNotMatch(quickCommandsSource, /state\.transcript\.push_back\(QuickCommandTranscriptEntry \{[\s\S]*?kind: "stopped"\.into\(\)/);
});

test('quick command backend exit snapshot preserves redaction contract', () => {
  assert.match(quickCommandsSource, /if stopped \{\s*String::new\(\)\s*\} else \{\s*exit_code\.map\(\|c\| c\.to_string\(\)\)\.unwrap_or_default\(\)\s*\}/);
  assert.match(quickCommandsSource, /redacted: false/);
  assert.match(quickCommandsSource, /"\[redacted\]"\.to_string\(\)/);
});

test('quick command backend history and transcript payload order stay stable under bounded retention', () => {
  assert.match(quickCommandsSource, /state\.transcript\.iter\(\)\.cloned\(\)\.collect\(\)/);
  assert.match(quickCommandsSource, /push_transcript\(\s*&mut state\.transcript/);
  assert.match(quickCommandsSource, /push_back\(/);
  assert.match(quickCommandsSource, /fn append_running_output[\s\S]*?if !run\.running \{\s*return None;/);
  assert.match(quickCommandsSource, /fn handle_marker[\s\S]*?if !run\.running \|\| run\.stopping \|\| run\.pending\.is_some\(\)/);
});

test('quick command backend decodes terminal bytes and strips ansi controls before transcript storage', () => {
  assert.match(quickCommandsSource, /decode_terminal_bytes\(/);
  assert.match(quickCommandsSource, /sanitize_terminal_text\(/);
  assert.match(quickCommandsSource, /GetOEMCP/);
  assert.match(quickCommandsSource, /MultiByteToWideChar/);
});

test('Quick Command stop does not shell out to taskkill tree kill by default', () => {
  assert.match(quickCommandsSource, /fn stop_running_quick_command\(/);
  assert.doesNotMatch(quickCommandsSource, /taskkill\.exe/i);
  assert.doesNotMatch(quickCommandsSource, /"\/T"/);
  assert.doesNotMatch(quickCommandsSource, /"\/F"/);
});

test('Quick Command stop uses async blocking boundary', () => {
  assert.match(quickCommandsSource, /pub\s+async\s+fn\s+stop_quick_command/);
  assert.match(quickCommandsSource, /spawn_blocking\s*\(/);
  assert.match(quickCommandsSource, /\.await\s*\.map_err/);
});

test('Quick Command live update exposes stopping state or pending stopped transition', () => {
  const stopBody = quickCommandsSource.match(/fn stop_running_quick_command\([\s\S]*?\n}\nfn process_creation_time_for_handle/)?.[0] ?? '';
  assert.match(quickCommandsSource, /kind:\s*"stopping"\.into\(\)/);
  assert.match(quickCommandsSource, /run\.stopping\s*=\s*true|QuickCommandRunPhase::Stopping/);
  assert.doesNotMatch(stopBody, /runs_guard\.remove\(run_id\)/);
  assert.match(quickCommandsSource, /!run\.running\s*\|\|\s*run\.stopping/);
});

test('quick command duplicate labels stay unique case-insensitively', () => {
  assert.equal(nextDuplicateQuickCommandLabel('Build', ['Build', 'build (1)', 'BUILD (2)']), 'Build (3)');
  assert.equal(nextUniqueQuickCommandId('Build', ['build', 'build-1']), 'build-2');
});
