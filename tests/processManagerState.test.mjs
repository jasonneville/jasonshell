import assert from 'node:assert/strict';
import test from 'node:test';
import {
  classifyProcessGroup,
  aggregateProcessMetrics,
  formatProcessCpu,
  formatProcessGpu,
  formatProcessMemory,
  formatProcessMemoryPercent,
  formatProcessPorts,
  formatProcessStartTime,
  formatProcessThreadCount,
  isWindowsProcess,
  isVolatileProcessSortColumn,
  nextProcessSortState,
  orderProcessRefresh,
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
  assert.deepEqual(
    sortProcesses([
      { ...processes[0], gpuPercent: 2 },
      { ...processes[1], gpuPercent: 50 },
      { ...processes[2], gpuPercent: null }
    ], { column: 'gpuPercent', direction: 'desc' }).map((process) => process.pid),
    [10, 20, 30]
  );
});

test('classifies taskbar-active applications before conservative Windows process heuristics', () => {
  assert.equal(classifyProcessGroup({ ...processes[0], taskbarActive: true }), 'applications');
  assert.equal(classifyProcessGroup({ ...processes[2], pid: 4, name: 'System', isKillable: false }), 'windows');
  assert.equal(classifyProcessGroup({ ...processes[1], name: 'svchost', executablePath: null }), 'windows');
  assert.equal(classifyProcessGroup({ ...processes[1], name: 'node.exe', executablePath: 'C:/dev/tool/node.exe' }), 'background');
  assert.equal(isWindowsProcess({ ...processes[1], name: 'svchost.exe', executablePath: 'C:/Windows/System32/svchost.exe' }), true);
  assert.equal(isWindowsProcess({ ...processes[1], name: 'svchost', executablePath: null }), true);
  assert.equal(isWindowsProcess({ ...processes[1], name: 'csrss', executablePath: null }), true);
  assert.equal(isWindowsProcess({ ...processes[1], name: 'conhost', executablePath: null }), true);
  assert.equal(isWindowsProcess({ ...processes[1], name: 'notepad.exe', executablePath: 'C:/Users/me/notepad.exe' }), false);
});

test('sortProcesses keeps Applications, Background processes, then Windows processes while sorting inside groups', () => {
  const groupedProcesses = [
    { pid: 1, name: 'System', cpuPercent: 99, memoryBytes: 100, threadCount: 1, status: 'running', isKillable: false },
    { pid: 20, name: 'zeta', cpuPercent: 1, memoryBytes: 200, threadCount: 1, status: 'running', isKillable: true, taskbarActive: true },
    { pid: 10, name: 'Alpha', cpuPercent: 10, memoryBytes: 100, threadCount: 1, status: 'running', isKillable: true },
    { pid: 30, name: 'beta', cpuPercent: 20, memoryBytes: 300, threadCount: 1, status: 'running', isKillable: true }
  ];

  assert.deepEqual(
    sortProcesses(groupedProcesses, { column: 'cpuPercent', direction: 'desc' }).map((process) => process.pid),
    [20, 30, 10, 1]
  );
});

test('orderProcessRefresh preserves volatile metric reading order while refreshing row values', () => {
  const previous = [
    { pid: 10, name: 'Alpha', cpuPercent: 50, memoryBytes: 100, threadCount: 1, status: 'running', isKillable: true },
    { pid: 20, name: 'zeta', cpuPercent: 10, memoryBytes: 200, threadCount: 1, status: 'running', isKillable: true }
  ];
  const refreshed = [
    { pid: 10, name: 'Alpha', cpuPercent: 1, memoryBytes: 100, threadCount: 1, status: 'running', isKillable: true },
    { pid: 20, name: 'zeta', cpuPercent: 99, memoryBytes: 200, threadCount: 1, status: 'running', isKillable: true },
    { pid: 30, name: 'beta', cpuPercent: 70, memoryBytes: 300, threadCount: 1, status: 'running', isKillable: true }
  ];

  const ordered = orderProcessRefresh(previous, refreshed, { column: 'cpuPercent', direction: 'desc' }, {
    preserveExistingOrder: true
  });

  assert.deepEqual(ordered.map((process) => [process.pid, process.cpuPercent]), [
    [10, 1],
    [20, 99],
    [30, 70]
  ]);
});

test('orderProcessRefresh preserves volatile order within groups while group order stays stable', () => {
  const previous = [
    { pid: 10, name: 'Alpha', cpuPercent: 50, memoryBytes: 100, threadCount: 1, status: 'running', isKillable: true },
    { pid: 20, name: 'zeta', cpuPercent: 10, memoryBytes: 200, threadCount: 1, status: 'running', isKillable: true },
    { pid: 4, name: 'System', cpuPercent: 1, memoryBytes: 50, threadCount: 1, status: 'running', isKillable: false }
  ];
  const refreshed = [
    { pid: 10, name: 'Alpha', cpuPercent: 50, memoryBytes: 100, threadCount: 1, status: 'running', isKillable: true },
    { pid: 20, name: 'zeta', cpuPercent: 10, memoryBytes: 200, threadCount: 1, status: 'running', isKillable: true, taskbarActive: true },
    { pid: 4, name: 'System', cpuPercent: 99, memoryBytes: 50, threadCount: 1, status: 'running', isKillable: false },
    { pid: 30, name: 'beta', cpuPercent: 80, memoryBytes: 300, threadCount: 1, status: 'running', isKillable: true }
  ];

  assert.deepEqual(
    orderProcessRefresh(previous, refreshed, { column: 'cpuPercent', direction: 'desc' }, {
      preserveExistingOrder: true,
      taskbarActivePids: [20]
    }).map((process) => process.pid),
    [20, 10, 30, 4]
  );
});

test('detects volatile process sort columns', () => {
  assert.equal(isVolatileProcessSortColumn('cpuPercent'), true);
  assert.equal(isVolatileProcessSortColumn('memoryBytes'), true);
  assert.equal(isVolatileProcessSortColumn('gpuPercent'), true);
  assert.equal(isVolatileProcessSortColumn('threadCount'), true);
  assert.equal(isVolatileProcessSortColumn('name'), false);
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
  assert.equal(formatProcessGpu(0.55), '0.6%');
  assert.equal(formatProcessGpu(44.2), '44%');
  assert.equal(formatProcessMemory(1024 * 1024 * 512), '512 MB');
  assert.equal(formatProcessMemory(1024 * 1024 * 1024 * 2.5), '2.5 GB');
  assert.equal(formatProcessMemoryPercent(25), '25%');
  assert.equal(formatProcessMemoryPercent(null), '—');
  assert.equal(formatProcessThreadCount(14), '14');
  assert.equal(formatProcessThreadCount(1234), '1,234');
  assert.equal(formatProcessThreadCount(null), '—');
  assert.equal(formatProcessStartTime(null), '—');
  assert.equal(formatProcessStartTime(Number.NaN), '—');
  assert.notEqual(formatProcessStartTime(1_700_000_000_000), '—');
  assert.equal(formatProcessPorts([3000, 5173, 1420, 8080, 9229]), '3000, 5173, 1420, 8080 +1');
  assert.equal(formatProcessPorts([]), '—');
});

test('aggregateProcessMetrics sums visible task-manager percentages and clamps totals', () => {
  const aggregates = aggregateProcessMetrics([
    { pid: 1, name: 'Code', cpuPercent: 12.5, memoryBytes: 1024, memoryPercent: 4, gpuPercent: 1.25, threadCount: 5, status: 'running', isKillable: true },
    { pid: 2, name: 'Browser', cpuPercent: 90, memoryBytes: 2048, memoryPercent: 8, gpuPercent: 200, threadCount: 9, status: 'running', isKillable: true },
    { pid: 3, name: 'Unknown', cpuPercent: null, memoryBytes: null, memoryPercent: null, gpuPercent: null, threadCount: null, status: 'running', isKillable: true }
  ]);

  assert.deepEqual(aggregates, {
    cpuPercent: 100,
    memoryBytes: 3072,
    memoryPercent: 12,
    gpuPercent: 100,
    threadCount: 14
  });

  assert.deepEqual(aggregateProcessMetrics([{ pid: 4, name: 'Unknown', status: 'running', isKillable: true }]), {
    cpuPercent: null,
    memoryBytes: 0,
    memoryPercent: null,
    gpuPercent: null,
    threadCount: null
  });
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
