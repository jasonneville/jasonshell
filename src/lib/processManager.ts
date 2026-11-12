import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';
export { getSystemInfoSnapshot } from './systemInfo.js';
export type { SystemInfoSnapshot } from './systemInfo.js';

export type ProcessManagerTab = 'processes' | 'system_info';
export type ProcessManagerContext = {
  visible: boolean; activeTab: ProcessManagerTab; openGeneration: number;
  tabRevision: number; focusPid: number | null;
};
export type ProcessManagerOpenPayload = { focusPid: number | null; context: ProcessManagerContext };
export function getProcessManagerContext(): Promise<ProcessManagerContext> {
  return invoke<ProcessManagerContext>(IPC_COMMANDS.getProcessManagerContext);
}
export function setProcessManagerTab(args: { openGeneration: number; tabRevision: number; tab: ProcessManagerTab }): Promise<ProcessManagerContext> {
  return invoke<ProcessManagerContext>(IPC_COMMANDS.setProcessManagerTab, args);
}
// SystemInfoSnapshot.snapshotRevision is a JS-safe monotonic cache revision;
// getSystemInfoSnapshot accepts afterRevision without scheduling native work.

export const PROCESS_MANAGER_OPEN_EVENT = 'process-manager:open';
export const PROCESS_MANAGER_CLOSED_EVENT = 'process-manager:closed';

export type ProcessInfo = {
  pid: number;
  parentPid?: number | null;
  parentName?: string | null;
  name: string;
  iconDataUrl?: string | null;
  executablePath?: string | null;
  commandLine?: string | null;
  listeningPorts?: number[];
  cpuPercent?: number | null;
  memoryBytes?: number | null;
  memoryPercent?: number | null;
  gpuPercent?: number | null;
  threadCount?: number | null;
  startTimeMs?: number | null;
  creationTime100ns?: string | null;
  normalizedImagePath?: string | null;
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
  focusPid?: number | null;
};

export type ProcessKillConfirmation = {
  confirmedTargetPid: number;
  mode: 'single' | 'tree-plan';
  affectedPids: number[];
  descendantPids: number[];
  acknowledgedWarningCount: number;
  requiresSecondConfirmation: boolean;
  canExecute: boolean;
  creationTime100ns?: string | null;
  normalizedImagePath?: string | null;
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
