import assert from 'node:assert/strict';
import test from 'node:test';
import {
  formatProcessCpu,
  formatProcessMemory,
  formatProcessPorts,
  formatProcessStartTime,
  nextProcessSortState,
  processDeveloperSummary,
  sortProcesses
} from '../dist-tests/lib/processManagerState.js';

const processes = [
  { pid: 20, name: 'zeta', cpuPercent: 1.5, memoryBytes: 2048, threadCount: 2, startTimeMs: 1_700_000_000_000, status: 'running', isKillable: true, listeningPorts: [5173] },
  { pid: 10, name: 'Alpha', cpuPercent: 12.25, memoryBytes: 1024 * 1024 * 512, threadCount: 8, startTimeMs: 1_800_000_000_000, status: 'running', isKillable: true },
  { pid: 30, name: 'beta', cpuPercent: null, memoryBytes: null, threadCount: 1, startTimeMs: null, status: 'running', isKillable: false }
];

test('sortProcesses sorts names with numeric locale semantics', () => {
  assert.deepEqual(
    sortProcesses(processes, { column: 'name', direction: 'asc' }).map((process) => process.name),
    ['Alpha', 'beta', 'zeta']
  );
});

test('sortProcesses sorts metrics descending and pushes unknown values last', () => {
  assert.deepEqual(
    sortProcesses(processes, { column: 'cpuPercent', direction: 'desc' }).map((process) => process.pid),
    [10, 20, 30]
  );
});

test('sortProcesses sorts process start time with unknown values last by default', () => {
  assert.deepEqual(
    sortProcesses(processes, { column: 'startTimeMs', direction: 'desc' }).map((process) => process.pid),
    [10, 20, 30]
  );
});

test('nextProcessSortState toggles same column and chooses metric defaults', () => {
  assert.deepEqual(
    nextProcessSortState({ column: 'name', direction: 'asc' }, 'name'),
    { column: 'name', direction: 'desc' }
  );
  assert.deepEqual(
    nextProcessSortState({ column: 'name', direction: 'asc' }, 'memoryBytes'),
    { column: 'memoryBytes', direction: 'desc' }
  );
});

test('formatters keep compact task-manager style labels', () => {
  assert.equal(formatProcessCpu(4.44), '4.4%');
  assert.equal(formatProcessCpu(12.44), '12%');
  assert.equal(formatProcessCpu(null), '—');
  assert.equal(formatProcessMemory(1024 * 1024 * 512), '512 MB');
  assert.equal(formatProcessMemory(1024 * 1024 * 1024 * 2.5), '2.5 GB');
  assert.equal(formatProcessStartTime(null), '—');
  assert.equal(formatProcessStartTime(Number.NaN), '—');
  assert.notEqual(formatProcessStartTime(1_700_000_000_000), '—');
  assert.equal(formatProcessPorts([3000, 5173, 1420, 8080, 9229]), '3000, 5173, 1420, 8080 +1');
  assert.equal(formatProcessPorts([]), '—');
});

test('processDeveloperSummary includes ports workspace parent descendants and command line', () => {
  assert.equal(
    processDeveloperSummary({
      pid: 40,
      parentPid: 20,
      parentName: 'pwsh',
      name: 'node',
      commandLine: 'node C:/dev/jasonshell/server.js',
      listeningPorts: [5173],
      descendantProcessCount: 2,
      workspaceHint: {
        kind: 'path-associated',
        label: 'jasonshell',
        path: 'C:/dev/jasonshell',
        source: 'process-path'
      },
      status: 'running',
      isKillable: true
    }),
    'ports 5173 • workspace jasonshell • parent pwsh (20) • 2 descendants • node C:/dev/jasonshell/server.js'
  );
});
