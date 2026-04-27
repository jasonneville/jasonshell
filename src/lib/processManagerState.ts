import type { ProcessInfo } from './processManager';

export type ProcessSortColumn = 'name' | 'pid' | 'cpuPercent' | 'memoryBytes' | 'threadCount' | 'startTimeMs';
export type SortDirection = 'asc' | 'desc';

export type ProcessSortState = {
  column: ProcessSortColumn;
  direction: SortDirection;
};

const STRING_COLLATOR = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: 'base'
});

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
  sort: ProcessSortState
): ProcessInfo[] {
  return [...processes].sort((left, right) => {
    const comparison = compareProcessValues(left, right, sort.column);
    if (comparison !== 0) {
      return sort.direction === 'asc' ? comparison : -comparison;
    }
    return left.pid - right.pid;
  });
}

export function formatProcessCpu(cpuPercent: number | null | undefined): string {
  return typeof cpuPercent === 'number' ? `${cpuPercent.toFixed(cpuPercent >= 10 ? 0 : 1)}%` : '—';
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

function defaultProcessSortDirection(column: ProcessSortColumn): SortDirection {
  return column === 'name' ? 'asc' : 'desc';
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

function nullableNumber(left: number | null | undefined, right: number | null | undefined): number {
  const leftValue = typeof left === 'number' ? left : Number.NEGATIVE_INFINITY;
  const rightValue = typeof right === 'number' ? right : Number.NEGATIVE_INFINITY;
  return leftValue - rightValue;
}
