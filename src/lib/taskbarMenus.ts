import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export type ShowTaskWindowContextMenuRequest = {
  hwnd: string;
  processId: number | null;
  isMinimized: boolean;
  x: number;
  y: number;
};

export type ShowLauncherContextMenuRequest = {
  shortcutPath: string;
  x: number;
  y: number;
};

export type ShowTopBarPinContextMenuRequest = {
  path: string;
  x: number;
  y: number;
};

export type TopBarPinMenuActionPayload = {
  action: 'open' | 'openInVscode' | 'unpin';
  path: string;
};

export const TOP_BAR_PIN_MENU_ACTION_EVENT = 'top-bar:pin-menu-action';

export type TaskbarLauncherAction = 'launch' | 'runas' | 'properties' | 'reveal' | 'revealTarget' | 'copyPath' | 'unpin';
export type TaskWindowAction = 'focus' | 'minimize' | 'close' | 'pin' | 'process';

export function runTaskbarLauncherAction(shortcutPath: string, action: TaskbarLauncherAction): Promise<void> {
  return invoke(IPC_COMMANDS.runTaskbarLauncherAction, { request: { shortcutPath, action } });
}

export function runTaskWindowAction(hwnd: string, action: TaskWindowAction, processId: number | null = null): Promise<void> {
  return invoke(IPC_COMMANDS.runTaskWindowAction, { request: { hwnd, action, processId } });
}

export function showTaskWindowContextMenu(
  request: ShowTaskWindowContextMenuRequest
): Promise<void> {
  return invoke(IPC_COMMANDS.showTaskWindowContextMenu, { request });
}

export function showLauncherContextMenu(
  request: ShowLauncherContextMenuRequest
): Promise<void> {
  return invoke(IPC_COMMANDS.showLauncherContextMenu, { request });
}

export function showTopBarPinContextMenu(
  request: ShowTopBarPinContextMenuRequest
): Promise<void> {
  return invoke(IPC_COMMANDS.showTopBarPinContextMenu, { request });
}
