import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

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

export type ShowQuickIconContextMenuRequest = {
  quickIconId: string;
  x: number;
  y: number;
};

export const QUICK_ICON_MENU_ACTIONS = {
  launchQuickIcon: 'launchQuickIcon',
  unpinQuickIcon: 'unpinQuickIcon'
} as const;

export type QuickIconMenuActionPayload = {
  action: (typeof QUICK_ICON_MENU_ACTIONS)[keyof typeof QUICK_ICON_MENU_ACTIONS];
  quickIconId: string;
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

export function showQuickIconContextMenu(
  request: ShowQuickIconContextMenuRequest
): Promise<void> {
  return invoke(IPC_COMMANDS.showQuickIconContextMenu, { request });
}

export function showTopBarPinContextMenu(
  request: ShowTopBarPinContextMenuRequest
): Promise<void> {
  return invoke(IPC_COMMANDS.showTopBarPinContextMenu, { request });
}
