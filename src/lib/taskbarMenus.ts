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

export type ShowTopBarPinContextMenuRequest = {
  path: string;
  x: number;
  y: number;
};

export type TopBarPinMenuActionPayload = {
  action: 'open' | 'unpin';
  path: string;
};

export const TOP_BAR_PIN_MENU_ACTION_EVENT = 'top-bar:pin-menu-action';

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

export function showTopBarPinContextMenu(
  request: ShowTopBarPinContextMenuRequest
): Promise<void> {
  return invoke('show_top_bar_pin_context_menu', { request });
}
