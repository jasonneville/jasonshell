import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  buildProcessTreeRows,
  filterProcesses,
  processMetricPercent,
  safeKillButtonState
} from '../dist-tests/features/process-manager/processManagerUxState.js';

const processes = [
  { pid: 1, parentPid: null, name: 'System', executablePath: null, cpuPercent: 1, memoryBytes: 100, threadCount: 2, status: 'running', isKillable: false },
  { pid: 20, parentPid: 1, name: 'Code Helper', executablePath: 'C:/Code/helper.exe', cpuPercent: 4, memoryBytes: 200, threadCount: 4, status: 'running', isKillable: true },
  { pid: 30, parentPid: 20, name: 'TypeScript Server', executablePath: 'C:/Code/tsserver.exe', cpuPercent: 2, memoryBytes: 300, threadCount: 8, status: 'running', isKillable: true },
  { pid: 40, parentPid: 999, name: 'Orphan Tool', executablePath: null, cpuPercent: null, memoryBytes: null, threadCount: null, status: 'unknown', isKillable: true }
];

test('filters processes by name, pid, parent pid, path, and status tokens', () => {
  assert.deepEqual(filterProcesses(processes, 'code helper').map((process) => process.pid), [20]);
  assert.deepEqual(filterProcesses(processes, '999').map((process) => process.pid), [40]);
  assert.deepEqual(filterProcesses(processes, 'running c:/code').map((process) => process.pid), [20, 30]);
});

test('builds tree-aware process rows while preserving supplied sibling order', () => {
  const rows = buildProcessTreeRows(processes);

  assert.deepEqual(rows.map((row) => [row.process.pid, row.depth, row.childCount]), [
    [1, 0, 1],
    [20, 1, 1],
    [30, 2, 0],
    [40, 0, 0]
  ]);
});

test('normalizes metric bars and safe kill confirmation state', () => {
  assert.equal(processMetricPercent(25, 100), 25);
  assert.equal(processMetricPercent(200, 100), 100);
  assert.equal(processMetricPercent(null, 100), 0);

  assert.deepEqual(safeKillButtonState(processes[0], null, null), {
    disabled: true,
    label: 'Guarded',
    ariaLabel: 'System process 1 is protected from kill actions',
    isArmed: false
  });
  assert.equal(safeKillButtonState(processes[1], 20, null).label, 'Confirm');
  assert.equal(safeKillButtonState(processes[1], null, 20).label, 'Killing...');
});
