import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  buildProcessGroups,
  buildProcessKillPlan,
  buildProcessTreeRows,
  enrichProcessesWithTaskbarWindows,
  filterProcesses,
  isProcessGroupExpanded,
  killConfirmationFromPlan,
  processKillErrorMessage,
  processMetricPercent,
  safeKillButtonState,
  taskbarActiveProcessIds,
  toggleProcessGroupExpansion
} from '../dist-tests/features/process-manager/processManagerUxState.js';

const processes = [
  { pid: 1, parentPid: null, name: 'System', executablePath: null, cpuPercent: 1, memoryBytes: 100, threadCount: 2, status: 'running', isKillable: false },
  { pid: 20, parentPid: 1, parentName: 'System', name: 'Code Helper', executablePath: 'C:/Code/helper.exe', commandLine: 'code --inspect C:/dev/jasonshell', listeningPorts: [9229], workspaceHint: { kind: 'path-associated', label: 'jasonshell', path: 'C:/dev/jasonshell', source: 'process-path' }, cpuPercent: 4, memoryBytes: 200, threadCount: 4, descendantProcessCount: 1, status: 'running', isKillable: true },
  { pid: 30, parentPid: 20, parentName: 'Code Helper', name: 'TypeScript Server', executablePath: 'C:/Code/tsserver.exe', commandLine: 'tsserver --stdio', listeningPorts: [], cpuPercent: 2, memoryBytes: 300, threadCount: 8, status: 'running', isKillable: true },
  { pid: 40, parentPid: 999, name: 'Orphan Tool', executablePath: null, cpuPercent: null, memoryBytes: null, threadCount: null, status: 'unknown', isKillable: true }
];

test('filters processes by name, pid, parent pid, path, and status tokens', () => {
  assert.deepEqual(filterProcesses(processes, 'code helper').map((process) => process.pid), [20, 30]);
  assert.deepEqual(filterProcesses(processes, '999').map((process) => process.pid), [40]);
  assert.deepEqual(filterProcesses(processes, 'running c:/code').map((process) => process.pid), [20, 30]);
  assert.deepEqual(filterProcesses(processes, '9229 jasonshell').map((process) => process.pid), [20]);
  assert.deepEqual(filterProcesses(processes, 'tsserver stdio').map((process) => process.pid), [30]);
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

test('promotes taskbar-active processes to readable top-level rows', () => {
  const rows = buildProcessTreeRows(processes, { promotedRootPids: [30] });

  assert.deepEqual(rows.map((row) => [row.process.pid, row.depth, row.childCount]), [
    [1, 0, 1],
    [20, 1, 0],
    [30, 0, 0],
    [40, 0, 0]
  ]);
});

test('builds Task Manager process groups in stable visual order', () => {
  const groups = buildProcessGroups([
    { pid: 10, parentPid: null, name: 'node.exe', executablePath: 'C:/dev/node.exe', cpuPercent: 1, memoryBytes: 100, threadCount: 1, status: 'running', isKillable: true },
    { pid: 20, parentPid: null, name: 'Code.exe', executablePath: 'C:/Code/Code.exe', cpuPercent: 1, memoryBytes: 100, threadCount: 1, status: 'running', isKillable: true, taskbarActive: true },
    { pid: 4, parentPid: null, name: 'System', executablePath: null, cpuPercent: 1, memoryBytes: 100, threadCount: 1, status: 'running', isKillable: false }
  ]);

  assert.deepEqual(groups.map((group) => group.label), ['Applications', 'Background processes', 'Windows processes']);
  assert.deepEqual(groups.map((group) => group.rows.map((row) => row.process.pid)), [[20], [10], [4]]);
});

test('filtering keeps all process groups visible while rows are scoped to matches', () => {
  const filtered = filterProcesses(processes, 'orphan');
  const groups = buildProcessGroups(filtered);

  assert.deepEqual(groups.map((group) => group.id), ['applications', 'background', 'windows']);
  assert.deepEqual(groups.map((group) => group.rows.map((row) => row.process.pid)), [[], [40], []]);
});

test('process group expansion defaults open and toggles per group independently', () => {
  let expansionState = {};

  assert.equal(isProcessGroupExpanded('applications', expansionState), true);
  assert.equal(isProcessGroupExpanded('background', expansionState), true);
  assert.equal(isProcessGroupExpanded('windows', expansionState), true);

  expansionState = toggleProcessGroupExpansion('background', expansionState);
  assert.equal(isProcessGroupExpanded('applications', expansionState), true);
  assert.equal(isProcessGroupExpanded('background', expansionState), false);
  assert.equal(isProcessGroupExpanded('windows', expansionState), true);

  expansionState = toggleProcessGroupExpansion('background', expansionState);
  assert.equal(isProcessGroupExpanded('background', expansionState), true);
});

test('enriches processes with taskbar-active window metadata', () => {
  const enriched = enrichProcessesWithTaskbarWindows(processes, [
    {
      hwnd: '100',
      processId: 20,
      title: 'JasonShell - Code',
      processName: 'Code Helper',
      iconDataUrl: '',
      isActive: true,
      isMinimized: false,
      activityState: 'idle'
    },
    {
      hwnd: '200',
      processId: 20,
      title: 'Preview',
      processName: 'Code Helper',
      iconDataUrl: '',
      isActive: false,
      isMinimized: true,
      activityState: 'busy'
    }
  ]);

  assert.equal(enriched.find((process) => process.pid === 20)?.taskbarActive, true);
  assert.equal(enriched.find((process) => process.pid === 20)?.taskbarForeground, true);
  assert.equal(enriched.find((process) => process.pid === 20)?.taskbarWindowCount, 2);
  assert.deepEqual(enriched.find((process) => process.pid === 20)?.taskbarTitles, ['JasonShell - Code', 'Preview']);
  assert.equal(enriched.find((process) => process.pid === 1)?.taskbarActive, false);
});

test('extracts unique taskbar-active process ids', () => {
  assert.deepEqual(taskbarActiveProcessIds([
    { hwnd: '100', processId: 20, title: 'A', processName: 'a', iconDataUrl: '', isActive: false, isMinimized: false, activityState: 'idle' },
    { hwnd: '200', processId: 20, title: 'B', processName: 'a', iconDataUrl: '', isActive: true, isMinimized: false, activityState: 'idle' },
    { hwnd: '300', processId: null, title: 'C', processName: 'c', iconDataUrl: '', isActive: false, isMinimized: false, activityState: 'idle' }
  ]), [20]);
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
  assert.equal(safeKillButtonState(processes[1], null, null).label, 'Review');
  assert.equal(safeKillButtonState(processes[1], 20, null).label, 'Kill 1');
  assert.equal(safeKillButtonState(processes[1], null, 20).label, 'Killing...');
});

test('plans kill-tree guardrails without enabling unsafe default tree kill', () => {
  const singlePlan = buildProcessKillPlan(processes, processes[1], false);
  assert.equal(singlePlan.mode, 'single');
  assert.deepEqual(singlePlan.affectedPids, [20]);
  assert.deepEqual(singlePlan.descendantPids, [30]);
  assert.equal(singlePlan.canExecute, true);
  assert.equal(singlePlan.requiresSecondConfirmation, true);
  assert.match(singlePlan.warnings[0], /workspace jasonshell/);
  assert.match(singlePlan.warnings[1], /leaves 1 descendant/);

  const treePlan = buildProcessKillPlan(processes, processes[1], true);
  assert.equal(treePlan.mode, 'tree-plan');
  assert.deepEqual(treePlan.affectedPids, [20, 30]);
  assert.equal(treePlan.canExecute, false);
  assert.match(treePlan.warnings.at(-1), /plan-only/);
});

test('builds behavioral kill confirmation payload with identity copied from plan', () => {
  const process = {
    ...processes[1],
    creationTime100ns: '987654321',
    normalizedImagePath: 'c:\\tools\\code helper.exe'
  };
  const plan = buildProcessKillPlan(processes, process, false);

  assert.deepEqual(killConfirmationFromPlan(plan), {
    confirmedTargetPid: 20,
    mode: 'single',
    affectedPids: [20],
    descendantPids: [30],
    acknowledgedWarningCount: 2,
    requiresSecondConfirmation: true,
    canExecute: true,
    creationTime100ns: '987654321',
    normalizedImagePath: 'c:\\tools\\code helper.exe'
  });
});

test('keeps classified process kill errors visible and bounded', () => {
  assert.equal(
    processKillErrorMessage(new Error('Access denied opening process 20 for termination/query')),
    'Access denied opening process 20 for termination/query'
  );
  assert.equal(processKillErrorMessage('  stale\n identity  '), 'stale identity');
  assert.equal(processKillErrorMessage('x'.repeat(300)).length, 240);
  assert.equal(processKillErrorMessage(null), 'Process termination failed');
});
