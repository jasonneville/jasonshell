import type { ProcessInfo } from './processManager';

export type ProcessSortColumn = 'name' | 'pid' | 'cpuPercent' | 'memoryBytes' | 'gpuPercent' | 'threadCount' | 'startTimeMs';
export type SortDirection = 'asc' | 'desc';

export type ProcessSortState = {
  column: ProcessSortColumn;
  direction: SortDirection;
};

export type ProcessGroupId = 'applications' | 'background' | 'windows';

export type ProcessGroupDefinition = {
  id: ProcessGroupId;
  label: string;
  emptyMessage: string;
};

export type ProcessMetricAggregates = {
  cpuPercent: number | null;
  memoryBytes: number;
  memoryPercent: number | null;
  gpuPercent: number | null;
  threadCount: number | null;
};

export type ProcessOrderOptions = {
  taskbarActivePids?: Iterable<number>;
};

export const PROCESS_GROUPS: readonly ProcessGroupDefinition[] = [
  {
    id: 'applications',
    label: 'Applications',
    emptyMessage: 'No taskbar-active applications in this snapshot'
  },
  {
    id: 'background',
    label: 'Background processes',
    emptyMessage: 'No background processes in this snapshot'
  },
  {
    id: 'windows',
    label: 'Windows processes',
    emptyMessage: 'No Windows processes in this snapshot'
  }
];

const STRING_COLLATOR = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: 'base'
});

const PROCESS_GROUP_INDEX = new Map(PROCESS_GROUPS.map((group, index) => [group.id, index]));

const WINDOWS_PROCESS_NAMES = new Set([
  'audiodg',
  'conhost',
  'csrss',
  'dwm',
  'fontdrvhost',
  'lsass',
  'memory compression',
  'registry',
  'runtimebroker',
  'secure system',
  'services',
  'sihost',
  'smss',
  'spoolsv',
  'svchost',
  'system',
  'system idle process',
  'taskhostw',
  'wininit',
  'winlogon',
  'wmiprvse'
]);

const WINDOWS_EXECUTABLE_PATH = /(?:^|[\\/])windows[\\/](?:system32|syswow64|systemapps|servicing|winsxs)[\\/]/i;

export function nextProcessSortState(
  current: ProcessSortState,
  column: ProcessSortColumn
): ProcessSortState {
  if (current.column !== column) {
    return { column, direction: defaultProcessSortDirection(column) };
  }

  return {
    column,
    direction: current.direction === 'asc' ? 'desc' : 'asc'
  };
}

export function sortProcesses(
  processes: readonly ProcessInfo[],
  sort: ProcessSortState,
  options: ProcessOrderOptions = {}
): ProcessInfo[] {
  return [...processes].sort((left, right) => {
    const groupComparison = compareProcessGroups(left, right, options);
    if (groupComparison !== 0) {
      return groupComparison;
    }

    const comparison = compareProcessValues(left, right, sort.column);
    if (comparison !== 0) {
      return sort.direction === 'asc' ? comparison : -comparison;
    }
    return left.pid - right.pid;
  });
}

export function orderProcessRefresh(
  previousProcesses: readonly ProcessInfo[],
  nextProcesses: readonly ProcessInfo[],
  sort: ProcessSortState,
  options: ProcessOrderOptions & { preserveExistingOrder?: boolean } = {}
): ProcessInfo[] {
  const sortedNextProcesses = sortProcesses(nextProcesses, sort, options);
  if (!options.preserveExistingOrder || previousProcesses.length === 0) {
    return sortedNextProcesses;
  }

  const previousIndexByPid = new Map(previousProcesses.map((process, index) => [process.pid, index]));
  const sortedIndexByPid = new Map(sortedNextProcesses.map((process, index) => [process.pid, index]));

  return [...sortedNextProcesses].sort((left, right) => {
    const groupComparison = compareProcessGroups(left, right, options);
    if (groupComparison !== 0) {
      return groupComparison;
    }

    const leftPreviousIndex = previousIndexByPid.get(left.pid);
    const rightPreviousIndex = previousIndexByPid.get(right.pid);
    if (typeof leftPreviousIndex === 'number' && typeof rightPreviousIndex === 'number') {
      return leftPreviousIndex - rightPreviousIndex;
    }
    if (typeof leftPreviousIndex === 'number') {
      return -1;
    }
    if (typeof rightPreviousIndex === 'number') {
      return 1;
    }

    return (sortedIndexByPid.get(left.pid) ?? 0) - (sortedIndexByPid.get(right.pid) ?? 0);
  });
}

export function classifyProcessGroup(
  process: ProcessInfo,
  options: ProcessOrderOptions = {}
): ProcessGroupId {
  const taskbarActivePids = new Set(options.taskbarActivePids ?? []);
  if (taskbarActivePids.has(process.pid) || process.taskbarActive === true) {
    return 'applications';
  }

  if (isWindowsProcess(process)) {
    return 'windows';
  }

  return 'background';
}

export function isWindowsProcess(process: ProcessInfo): boolean {
  const name = normalizeProcessName(process.name);
  if (WINDOWS_PROCESS_NAMES.has(name)) {
    return true;
  }

  const executablePath = process.executablePath?.trim() ?? '';
  if (executablePath && WINDOWS_EXECUTABLE_PATH.test(executablePath)) {
    return true;
  }

  const parentName = normalizeProcessName(process.parentName ?? '');
  return parentName === 'services' && executablePath.toLocaleLowerCase().includes('\\windows\\system32\\');
}

export function isVolatileProcessSortColumn(column: ProcessSortColumn): boolean {
  return column === 'cpuPercent' || column === 'memoryBytes' || column === 'gpuPercent' || column === 'threadCount';
}

export function aggregateProcessMetrics(processes: readonly ProcessInfo[]): ProcessMetricAggregates {
  return {
    cpuPercent: aggregatePercent(processes, (process) => process.cpuPercent),
    memoryBytes: processes.reduce((total, process) => total + finiteOrZero(process.memoryBytes), 0),
    memoryPercent: aggregatePercent(processes, (process) => process.memoryPercent),
    gpuPercent: aggregatePercent(processes, (process) => process.gpuPercent),
    threadCount: aggregateNumber(processes, (process) => process.threadCount)
  };
}

export function formatProcessCpu(cpuPercent: number | null | undefined): string {
  return formatProcessPercent(cpuPercent);
}

export function formatProcessGpu(gpuPercent: number | null | undefined): string {
  return formatProcessPercent(gpuPercent);
}

export function formatProcessMemoryPercent(
  memoryPercent: number | null | undefined
): string {
  return formatProcessPercent(memoryPercent);
}

export function formatProcessMemory(memoryBytes: number | null | undefined): string {
  if (typeof memoryBytes !== 'number') {
    return '—';
  }
  if (memoryBytes >= 1024 * 1024 * 1024) {
    return `${(memoryBytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }
  return `${Math.max(1, Math.round(memoryBytes / (1024 * 1024)))} MB`;
}

export function formatProcessThreadCount(threadCount: number | null | undefined): string {
  if (typeof threadCount !== 'number' || !Number.isFinite(threadCount)) {
    return '—';
  }

  return Math.round(threadCount).toLocaleString();
}

export function formatProcessStartTime(startTimeMs: number | null | undefined): string {
  if (typeof startTimeMs !== 'number' || !Number.isFinite(startTimeMs)) {
    return '—';
  }

  return new Date(startTimeMs).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  });
}

export function formatProcessPorts(ports: readonly number[] | null | undefined): string {
  if (!ports?.length) {
    return '—';
  }

  return ports.slice(0, 4).join(', ') + (ports.length > 4 ? ` +${ports.length - 4}` : '');
}

export function processDeveloperSummary(process: ProcessInfo): string {
  const parts: string[] = [];
  const ports = formatProcessPorts(process.listeningPorts);
  if (ports !== '—') {
    parts.push(`ports ${ports}`);
  }
  if (process.workspaceHint) {
    parts.push(`workspace ${process.workspaceHint.label}`);
  }
  if (process.parentName) {
    parts.push(`parent ${process.parentName} (${process.parentPid ?? 'unknown'})`);
  } else if (typeof process.parentPid === 'number') {
    parts.push(`parent ${process.parentPid}`);
  }
  if ((process.descendantProcessCount ?? 0) > 0) {
    parts.push(`${process.descendantProcessCount} descendant${process.descendantProcessCount === 1 ? '' : 's'}`);
  }
  const command = process.commandLine?.trim() || process.executablePath?.trim();
  if (command) {
    parts.push(command);
  }

  return parts.join(' • ');
}

function defaultProcessSortDirection(column: ProcessSortColumn): SortDirection {
  return column === 'name' ? 'asc' : 'desc';
}

function formatProcessPercent(value: number | null | undefined): string {
  return typeof value === 'number' && Number.isFinite(value)
    ? `${value.toFixed(value >= 10 ? 0 : 1)}%`
    : '—';
}

function normalizeProcessName(name: string): string {
  return name.trim().toLocaleLowerCase().replace(/\.exe$/, '');
}

function aggregatePercent(
  processes: readonly ProcessInfo[],
  readValue: (process: ProcessInfo) => number | null | undefined
): number | null {
  const total = aggregateNumber(processes, readValue);
  return total === null ? null : Math.max(0, Math.min(100, total));
}

function aggregateNumber(
  processes: readonly ProcessInfo[],
  readValue: (process: ProcessInfo) => number | null | undefined
): number | null {
  let total = 0;
  let hasValue = false;
  for (const process of processes) {
    const value = readValue(process);
    if (typeof value !== 'number' || !Number.isFinite(value)) {
      continue;
    }
    total += Math.max(0, value);
    hasValue = true;
  }
  return hasValue ? total : null;
}

function finiteOrZero(value: number | null | undefined): number {
  return typeof value === 'number' && Number.isFinite(value) ? Math.max(0, value) : 0;
}

function compareProcessValues(
  left: ProcessInfo,
  right: ProcessInfo,
  column: ProcessSortColumn
): number {
  if (column === 'name') {
    return STRING_COLLATOR.compare(left.name, right.name);
  }

  return nullableNumber(left[column], right[column]);
}

function compareProcessGroups(
  left: ProcessInfo,
  right: ProcessInfo,
  options: ProcessOrderOptions
): number {
  return groupIndex(classifyProcessGroup(left, options)) - groupIndex(classifyProcessGroup(right, options));
}

function groupIndex(groupId: ProcessGroupId): number {
  return PROCESS_GROUP_INDEX.get(groupId) ?? PROCESS_GROUPS.length;
}

function nullableNumber(left: number | null | undefined, right: number | null | undefined): number {
  const leftValue = typeof left === 'number' ? left : Number.NEGATIVE_INFINITY;
  const rightValue = typeof right === 'number' ? right : Number.NEGATIVE_INFINITY;
  return leftValue - rightValue;
}
