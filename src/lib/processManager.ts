import { invoke } from '@tauri-apps/api/core';

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
  return invoke('show_process_manager', { request });
}

export function hideProcessManager(): Promise<void> {
  return invoke('hide_process_manager');
}

export function listProcesses(): Promise<ProcessInfo[]> {
  return invoke<ProcessInfo[]>('list_processes');
}

export function killProcess(pid: number): Promise<void> {
  return invoke('kill_process', { pid });
}
