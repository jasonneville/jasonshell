import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

type TaskbarWindowPayload = {
  hwnd: string | number | bigint;
  title: string;
  processId?: number;
  processName: string;
  iconDataUrl: string;
  isActive: boolean;
  isMinimized: boolean;
  activityState?: TaskbarWindowActivityState;
};

export type TaskbarWindowActivityState = 'idle' | 'busy';

export type TaskbarWindow = {
  hwnd: string;
  title: string;
  processId: number | null;
  processName: string;
  iconDataUrl: string;
  isActive: boolean;
  isMinimized: boolean;
  activityState: TaskbarWindowActivityState;
};

export type TaskbarProcessWindow = {
  hwnd: string;
  title: string;
  processId: number | null;
  isActive: boolean;
};

function normalizeTaskbarWindow(window: TaskbarWindowPayload): TaskbarWindow {
  return {
    ...window,
    hwnd: String(window.hwnd),
    processId: typeof window.processId === 'number' ? window.processId : null,
    activityState: window.activityState === 'busy' ? 'busy' : 'idle'
  };
}

export async function listOpenTaskWindows(): Promise<TaskbarWindow[]> {
  const windows = await invoke<TaskbarWindowPayload[]>(IPC_COMMANDS.listOpenTaskWindows);
  return windows.map(normalizeTaskbarWindow);
}

export async function listTaskbarProcessWindows(): Promise<TaskbarProcessWindow[]> {
  const windows = await invoke<Array<Omit<TaskbarProcessWindow, 'processId'> & { processId?: number }>>(
    IPC_COMMANDS.listTaskbarProcessWindows
  );
  return windows.map((window) => ({
    ...window,
    hwnd: String(window.hwnd),
    processId: typeof window.processId === 'number' ? window.processId : null
  }));
}

export function activateTaskWindow(hwnd: string, minimizeIfActive = false): Promise<void> {
  return invoke(IPC_COMMANDS.activateTaskWindow, { hwnd, minimizeIfActive });
}

export function maximizeTaskWindow(hwnd: string): Promise<void> {
  return invoke(IPC_COMMANDS.maximizeTaskWindow, { hwnd });
}

export function closeTaskWindow(hwnd: string): Promise<void> {
  return invoke(IPC_COMMANDS.closeTaskWindow, { hwnd });
}
