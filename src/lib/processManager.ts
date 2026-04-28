import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export const PROCESS_MANAGER_OPEN_EVENT = 'process-manager:open';
export const PROCESS_MANAGER_CLOSED_EVENT = 'process-manager:closed';

export type ProcessInfo = {
  pid: number;
  parentPid?: number | null;
  parentName?: string | null;
  name: string;
  executablePath?: string | null;
  commandLine?: string | null;
  listeningPorts?: number[];
  cpuPercent?: number | null;
  memoryBytes?: number | null;
  threadCount?: number | null;
  startTimeMs?: number | null;
  childProcessCount?: number;
  descendantProcessCount?: number;
  workspaceHint?: ProcessWorkspaceHint | null;
  taskbarWindowCount?: number;
  taskbarActive?: boolean;
  taskbarForeground?: boolean;
  taskbarTitles?: string[];
  status: string;
  isKillable: boolean;
};

export type ProcessWorkspaceHint = {
  kind: string;
  label: string;
  path?: string | null;
  source: string;
};

export type ShowProcessManagerRequest = {
  anchorLeft: number;
  anchorWidth: number;
};

export type ProcessKillConfirmation = {
  confirmedTargetPid: number;
  mode: 'single' | 'tree-plan';
  affectedPids: number[];
  descendantPids: number[];
  acknowledgedWarningCount: number;
  requiresSecondConfirmation: boolean;
  canExecute: boolean;
};

export function showProcessManager(request: ShowProcessManagerRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showProcessManager, { request });
}

export function hideProcessManager(): Promise<void> {
  return invoke(IPC_COMMANDS.hideProcessManager);
}

export function listProcesses(): Promise<ProcessInfo[]> {
  return invoke<ProcessInfo[]>(IPC_COMMANDS.listProcesses);
}

export function killProcess(pid: number, confirmation?: ProcessKillConfirmation): Promise<void> {
  return invoke(IPC_COMMANDS.killProcess, { pid, confirmation });
}
