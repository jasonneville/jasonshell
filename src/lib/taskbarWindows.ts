import { invoke } from '@tauri-apps/api/core';

type TaskbarWindowPayload = {
  hwnd: string | number | bigint;
  title: string;
  processName: string;
  iconDataUrl: string;
  isActive: boolean;
  isMinimized: boolean;
};

export type TaskbarWindow = {
  hwnd: string;
  title: string;
  processName: string;
  iconDataUrl: string;
  isActive: boolean;
  isMinimized: boolean;
};

function normalizeTaskbarWindow(window: TaskbarWindowPayload): TaskbarWindow {
  return {
    ...window,
    hwnd: String(window.hwnd)
  };
}

export async function listOpenTaskWindows(): Promise<TaskbarWindow[]> {
  const windows = await invoke<TaskbarWindowPayload[]>('list_open_task_windows');
  return windows.map(normalizeTaskbarWindow);
}

export function activateTaskWindow(hwnd: string, wasActive: boolean): Promise<void> {
  return invoke('activate_task_window', { hwnd, wasActive });
}

export function maximizeTaskWindow(hwnd: string): Promise<void> {
  return invoke('maximize_task_window', { hwnd });
}
