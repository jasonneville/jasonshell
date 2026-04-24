import { invoke } from '@tauri-apps/api/core';

export type ShowTaskWindowContextMenuRequest = {
  hwnd: string;
  isMinimized: boolean;
  x: number;
  y: number;
};

export type ShowLauncherContextMenuRequest = {
  shortcutPath: string;
  x: number;
  y: number;
};

export function showTaskWindowContextMenu(
  request: ShowTaskWindowContextMenuRequest
): Promise<void> {
  return invoke('show_task_window_context_menu', { request });
}

export function showLauncherContextMenu(
  request: ShowLauncherContextMenuRequest
): Promise<void> {
  return invoke('show_launcher_context_menu', { request });
}
