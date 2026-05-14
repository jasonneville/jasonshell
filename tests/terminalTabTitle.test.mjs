import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildTerminalTabTitle, sanitizeTitlePart } from '../dist-tests/features/terminal/terminalTabTitle.js';

const commandState = (records, extra = {}) => ({
  sessionId: 's1',
  records,
  nextId: records.length + 1,
  markerOffset: 0,
  ...extra
});

test('terminal tab title preserves explicit manual rename above dynamic state', () => {
  assert.equal(buildTerminalTabTitle({
    profileTitle: 'Windows Terminal',
    manualTitle: 'Backend API',
    cwd: 'C:/dev/project',
    currentInputText: 'npm test',
    recentOutputText: 'failing output'
  }), 'Backend API');
});

test('terminal tab title prefers minimal running command names without output text', () => {
  assert.equal(buildTerminalTabTitle({
    profileTitle: 'Windows Terminal',
    cwd: 'C:/dev/project',
    currentInputText: 'node scripts/build.mjs',
    recentOutputText: '\u001b[31mCompiled successfully\u001b[0m'
  }), 'node - project');

  assert.equal(buildTerminalTabTitle({
    profileTitle: 'PowerShell',
    commandState: commandState([{ id: 's1:1', sessionId: 's1', commandText: '"C:/Program Files/Git/bin/git.exe" status', cwd: 'C:/dev/repo', startedAt: 1 }]),
    recentOutputText: 'On branch main'
  }), 'git - repo');

  assert.equal(buildTerminalTabTitle({
    cwd: 'C:/dev/jasonshell',
    commandState: commandState([{ id: 's1:1', sessionId: 's1', commandText: 'npm test', cwd: 'C:/dev/jasonshell', startedAt: 1 }]),
    recentOutputText: 'old line\nnewest useful line'
  }), 'npm test');
});

test('terminal tab title formats common developer processes like Windows Terminal', () => {
  assert.equal(buildTerminalTabTitle({ cwd: 'C:/dev/jasonshell', commandState: commandState([{ id: 's1:1', sessionId: 's1', commandText: 'pi', cwd: 'C:/dev/jasonshell', startedAt: 1 }]) }), 'pi - jasonshell');
  assert.equal(buildTerminalTabTitle({ cwd: 'C:/dev/jasonshell', commandState: commandState([{ id: 's1:1', sessionId: 's1', commandText: 'codex --model gpt-5', cwd: 'C:/dev/jasonshell', startedAt: 1 }]) }), 'codex - jasonshell');
  assert.equal(buildTerminalTabTitle({ cwd: 'C:/dev/jasonshell', commandState: commandState([{ id: 's1:1', sessionId: 's1', commandText: 'mvn clean install', cwd: 'C:/dev/jasonshell', startedAt: 1 }]) }), 'maven install');
  assert.equal(buildTerminalTabTitle({ cwd: 'C:/dev/jasonshell', commandState: commandState([{ id: 's1:1', sessionId: 's1', commandText: './mvnw test', cwd: 'C:/dev/jasonshell', startedAt: 1 }]) }), 'maven test');
});

test('terminal tab title does not treat typed-but-unsubmitted input as a running process', () => {
  assert.equal(buildTerminalTabTitle({ cwd: 'C:/dev/jasonshell', commandState: commandState([]), currentInputText: 'codex' }), 'c:/dev/jasonshell');
});

test('terminal tab title uses normalized directory path when idle and profile only as fallback', () => {
  assert.equal(buildTerminalTabTitle({ profileTitle: 'Windows Terminal', cwd: 'C:\\dev\\jasonshell\\' }), 'c:/dev/jasonshell');
  assert.equal(buildTerminalTabTitle({ profileTitle: 'Windows Terminal', cwd: 'C:/dev/jasonshell/' }), 'c:/dev/jasonshell');
  assert.equal(buildTerminalTabTitle({ profileTitle: 'Windows Terminal' }), 'Windows Terminal');
});

test('terminal tab title strips control noise, collapses whitespace, and truncates', () => {
  assert.equal(sanitizeTitlePart('hello\u0007\u001b[31m   world'), 'hello world');
  const title = buildTerminalTabTitle({ cwd: `C:/dev/${'x'.repeat(120)}` });
  assert.ok(title.length <= 48);
  assert.match(title, /…$/);
});
