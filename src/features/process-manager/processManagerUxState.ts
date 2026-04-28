import type { ProcessInfo } from '../../lib/processManager';

export type ProcessTreeRow = {
  process: ProcessInfo;
  depth: number;
  childCount: number;
};

export type SafeKillButtonState = {
  disabled: boolean;
  label: string;
  ariaLabel: string;
  isArmed: boolean;
};

export function filterProcesses(processes: readonly ProcessInfo[], query: string): ProcessInfo[] {
  const tokens = normalize(query).split(' ').filter(Boolean);
  if (!tokens.length) {
    return [...processes];
  }

  return processes.filter((process) => {
    const haystack = normalize([
      process.name,
      process.pid,
      process.parentPid ?? '',
      process.executablePath ?? '',
      process.status
    ].join(' '));
    return tokens.every((token) => haystack.includes(token));
  });
}

export function buildProcessTreeRows(processes: readonly ProcessInfo[]): ProcessTreeRow[] {
  const byPid = new Map(processes.map((process) => [process.pid, process]));
  const childrenByParent = new Map<number, ProcessInfo[]>();
  const order = new Map(processes.map((process, index) => [process.pid, index]));

  for (const process of processes) {
    if (typeof process.parentPid !== 'number' || !byPid.has(process.parentPid)) {
      continue;
    }
    const children = childrenByParent.get(process.parentPid) ?? [];
    children.push(process);
    childrenByParent.set(process.parentPid, children);
  }

  for (const children of childrenByParent.values()) {
    children.sort((left, right) => (order.get(left.pid) ?? 0) - (order.get(right.pid) ?? 0));
  }

  const rows: ProcessTreeRow[] = [];
  const visited = new Set<number>();
  const roots = processes.filter(
    (process) => typeof process.parentPid !== 'number' || !byPid.has(process.parentPid)
  );

  function visit(process: ProcessInfo, depth: number) {
    if (visited.has(process.pid)) {
      return;
    }
    visited.add(process.pid);
    const children = childrenByParent.get(process.pid) ?? [];
    rows.push({ process, depth, childCount: children.length });
    for (const child of children) {
      visit(child, depth + 1);
    }
  }

  for (const root of roots) {
    visit(root, 0);
  }
  for (const process of processes) {
    visit(process, 0);
  }

  return rows;
}

export function processMetricPercent(
  value: number | null | undefined,
  maxValue: number | null | undefined
): number {
  if (
    typeof value !== 'number'
    || typeof maxValue !== 'number'
    || !Number.isFinite(value)
    || !Number.isFinite(maxValue)
    || maxValue <= 0
  ) {
    return 0;
  }
  return Math.max(0, Math.min(100, (value / maxValue) * 100));
}

export function safeKillButtonState(
  process: ProcessInfo,
  armedPid: number | null,
  killingPid: number | null
): SafeKillButtonState {
  if (!process.isKillable) {
    return {
      disabled: true,
      label: 'Guarded',
      ariaLabel: `${process.name} process ${process.pid} is protected from kill actions`,
      isArmed: false
    };
  }
  if (killingPid !== null) {
    return {
      disabled: killingPid !== process.pid,
      label: killingPid === process.pid ? 'Killing...' : 'Wait',
      ariaLabel: `Kill action in progress for process ${killingPid}`,
      isArmed: false
    };
  }
  if (armedPid === process.pid) {
    return {
      disabled: false,
      label: 'Confirm',
      ariaLabel: `Confirm kill for ${process.name} process ${process.pid}`,
      isArmed: true
    };
  }
  return {
    disabled: false,
    label: 'Kill',
    ariaLabel: `Arm kill confirmation for ${process.name} process ${process.pid}`,
    isArmed: false
  };
}

function normalize(value: string): string {
  return value.trim().toLocaleLowerCase().replace(/\s+/g, ' ');
}
