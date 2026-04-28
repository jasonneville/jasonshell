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
