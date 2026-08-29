import type { ProcessInfo } from '../../lib/processManager';
import {
  classifyProcessGroup,
  PROCESS_GROUPS,
  type ProcessGroupId,
  type ProcessOrderOptions
} from '../../lib/processManagerState.js';
import type { TaskbarProcessWindow } from '../../lib/taskbarWindows';

export type ProcessTreeRow = {
  process: ProcessInfo;
  depth: number;
  childCount: number;
};

export type ProcessTreeRowOptions = {
  promotedRootPids?: Iterable<number>;
};

export type ProcessGroupRows = {
  id: ProcessGroupId;
  label: string;
  emptyMessage: string;
  rows: ProcessTreeRow[];
};

export type ProcessGroupExpansionState = Partial<Record<ProcessGroupId, boolean>>;

export type SafeKillButtonState = {
  disabled: boolean;
  label: string;
  ariaLabel: string;
  isArmed: boolean;
};

export type ProcessKillPlan = {
  targetPid: number;
  mode: 'single' | 'tree-plan';
  affectedPids: number[];
  descendantPids: number[];
  warnings: string[];
  requiresSecondConfirmation: boolean;
  canExecute: boolean;
  creationTime100ns?: string | null;
  normalizedImagePath?: string | null;
};

export type ProcessKillConfirmationPayload = {
  confirmedTargetPid: number;
  mode: ProcessKillPlan['mode'];
  affectedPids: number[];
  descendantPids: number[];
  acknowledgedWarningCount: number;
  requiresSecondConfirmation: boolean;
  canExecute: boolean;
  creationTime100ns?: string | null;
  normalizedImagePath?: string | null;
};

export function killConfirmationFromPlan(killPlan: ProcessKillPlan): ProcessKillConfirmationPayload {
  return {
    confirmedTargetPid: killPlan.targetPid,
    mode: killPlan.mode,
    affectedPids: killPlan.affectedPids,
    descendantPids: killPlan.descendantPids,
    acknowledgedWarningCount: killPlan.warnings.length,
    requiresSecondConfirmation: killPlan.requiresSecondConfirmation,
    canExecute: killPlan.canExecute,
    creationTime100ns: killPlan.creationTime100ns,
    normalizedImagePath: killPlan.normalizedImagePath
  };
}

export function processKillErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error ?? '');
  const bounded = message.replace(/\s+/g, ' ').trim().slice(0, 240);
  return bounded || 'Process termination failed';
}

export function enrichProcessesWithTaskbarWindows(
  processes: readonly ProcessInfo[],
  taskbarWindows: readonly TaskbarProcessWindow[]
): ProcessInfo[] {
  const windowsByPid = new Map<number, TaskbarProcessWindow[]>();
  for (const window of taskbarWindows) {
    if (typeof window.processId !== 'number') {
      continue;
    }
    const windows = windowsByPid.get(window.processId) ?? [];
    windows.push(window);
    windowsByPid.set(window.processId, windows);
  }

  return processes.map((process) => {
    const windows = windowsByPid.get(process.pid) ?? [];
    if (!windows.length) {
      return {
        ...process,
        taskbarWindowCount: 0,
        taskbarActive: false,
        taskbarForeground: false,
        taskbarTitles: []
      };
    }

    return {
      ...process,
      taskbarWindowCount: windows.length,
      taskbarActive: true,
      taskbarForeground: windows.some((window) => window.isActive),
      taskbarTitles: windows.map((window) => window.title).filter((title) => title.trim().length > 0)
    };
  });
}

export function taskbarActiveProcessIds(taskbarWindows: readonly TaskbarProcessWindow[]): number[] {
  return Array.from(
    new Set(
      taskbarWindows
        .map((window) => window.processId)
        .filter((processId): processId is number => typeof processId === 'number')
    )
  );
}

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
      process.parentName ?? '',
      process.executablePath ?? '',
      process.commandLine ?? '',
      process.listeningPorts?.join(' ') ?? '',
      process.workspaceHint?.label ?? '',
      process.workspaceHint?.path ?? '',
      process.status
    ].join(' '));
    return tokens.every((token) => haystack.includes(token));
  });
}

export function buildProcessTreeRows(
  processes: readonly ProcessInfo[],
  options: ProcessTreeRowOptions = {}
): ProcessTreeRow[] {
  const byPid = new Map(processes.map((process) => [process.pid, process]));
  const childrenByParent = new Map<number, ProcessInfo[]>();
  const order = new Map(processes.map((process, index) => [process.pid, index]));
  const promotedRootPids = new Set(options.promotedRootPids ?? []);

  for (const process of processes) {
    if (
      promotedRootPids.has(process.pid)
      || typeof process.parentPid !== 'number'
      || !byPid.has(process.parentPid)
    ) {
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
    (process) => promotedRootPids.has(process.pid)
      || typeof process.parentPid !== 'number'
      || !byPid.has(process.parentPid)
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

export function buildProcessGroups(
  processes: readonly ProcessInfo[],
  options: ProcessOrderOptions = {}
): ProcessGroupRows[] {
  const processesByGroup = new Map<ProcessGroupId, ProcessInfo[]>(
    PROCESS_GROUPS.map((group) => [group.id, []])
  );

  for (const process of processes) {
    processesByGroup.get(classifyProcessGroup(process, options))?.push(process);
  }

  const taskbarActivePids = new Set(options.taskbarActivePids ?? []);
  return PROCESS_GROUPS.map((group) => ({
    ...group,
    rows: buildProcessTreeRows(processesByGroup.get(group.id) ?? [], {
      promotedRootPids: group.id === 'applications' ? taskbarActivePids : []
    })
  }));
}

export function isProcessGroupExpanded(
  groupId: ProcessGroupId,
  expansionState: ProcessGroupExpansionState
): boolean {
  return expansionState[groupId] !== false;
}

export function toggleProcessGroupExpansion(
  groupId: ProcessGroupId,
  expansionState: ProcessGroupExpansionState
): ProcessGroupExpansionState {
  return {
    ...expansionState,
    [groupId]: !isProcessGroupExpanded(groupId, expansionState)
  };
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
      label: 'Kill 1',
      ariaLabel: `Confirm kill for ${process.name} process ${process.pid}`,
      isArmed: true
    };
  }
  return {
    disabled: false,
    label: (process.descendantProcessCount ?? 0) > 0 ? 'Review' : 'Kill',
    ariaLabel: `Arm single-process kill confirmation for ${process.name} process ${process.pid}`,
    isArmed: false
  };
}

export function buildProcessKillPlan(
  processes: readonly ProcessInfo[],
  process: ProcessInfo,
  includeTreeRequested = false
): ProcessKillPlan {
  const descendantPids = processDescendantPids(processes, process.pid);
  const warnings = processGuardrailWarnings(processes, process, descendantPids);
  if (includeTreeRequested) {
    return {
      targetPid: process.pid,
      mode: 'tree-plan',
      affectedPids: [process.pid, ...descendantPids],
      descendantPids,
      warnings: [
        ...warnings,
        'Tree kill is guarded and plan-only; JasonShell does not terminate descendants by default.'
      ],
      requiresSecondConfirmation: true,
      canExecute: false,
      creationTime100ns: process.creationTime100ns ?? null,
      normalizedImagePath: process.normalizedImagePath ?? null
    };
  }

  return {
    targetPid: process.pid,
    mode: 'single',
    affectedPids: [process.pid],
    descendantPids,
    warnings: [
      ...warnings,
      ...(descendantPids.length
        ? [`Single-process kill leaves ${descendantPids.length} descendant process(es) running.`]
        : [])
    ],
    requiresSecondConfirmation: true,
    canExecute: process.isKillable,
    creationTime100ns: process.creationTime100ns ?? null,
    normalizedImagePath: process.normalizedImagePath ?? null
  };
}

function processGuardrailWarnings(
  processes: readonly ProcessInfo[],
  process: ProcessInfo,
  descendantPids: readonly number[]
): string[] {
  const warnings: string[] = [];
  if (process.workspaceHint) {
    warnings.push(`Target process is associated with workspace ${process.workspaceHint.label}.`);
  }

  const descendantPidSet = new Set(descendantPids);
  if (processes.some((candidate) => descendantPidSet.has(candidate.pid) && candidate.workspaceHint)) {
    warnings.push('Process tree includes workspace-associated descendant process(es).');
  }

  return warnings;
}

function processDescendantPids(processes: readonly ProcessInfo[], pid: number): number[] {
  const childrenByParent = new Map<number, number[]>();
  for (const process of processes) {
    if (typeof process.parentPid !== 'number') {
      continue;
    }
    const children = childrenByParent.get(process.parentPid) ?? [];
    children.push(process.pid);
    childrenByParent.set(process.parentPid, children);
  }

  const descendants: number[] = [];
  const visited = new Set<number>();
  const stack = [...(childrenByParent.get(pid) ?? [])];
  while (stack.length) {
    const childPid = stack.pop();
    if (typeof childPid !== 'number' || visited.has(childPid)) {
      continue;
    }
    visited.add(childPid);
    descendants.push(childPid);
    stack.push(...(childrenByParent.get(childPid) ?? []));
  }

  return descendants.sort((left, right) => left - right);
}

function normalize(value: string): string {
  return value.trim().toLocaleLowerCase().replace(/\s+/g, ' ');
}
