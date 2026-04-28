import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export const PROCESS_MANAGER_OPEN_EVENT = 'process-manager:open';
export const PROCESS_MANAGER_CLOSED_EVENT = 'process-manager:closed';

export type ProcessInfo = {
  pid: number;
  parentPid?: number | null;
  name: string;
  executablePath?: string | null;
  cpuPercent?: number | null;
  memoryBytes?: number | null;
  threadCount?: number | null;
  startTimeMs?: number | null;
  status: string;
  isKillable: boolean;
};

export type ShowProcessManagerRequest = {
  anchorLeft: number;
  anchorWidth: number;
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

export function killProcess(pid: number): Promise<void> {
  return invoke(IPC_COMMANDS.killProcess, { pid });
}
