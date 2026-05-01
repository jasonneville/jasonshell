import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';
import {
  invokeSystemTrayIcon,
  listSystemTrayIcons,
  trayClickRequest,
  type SystemTrayIconSnapshot,
  type SystemTrayMouseButton
} from './systemTray.js';

export type { SystemTrayIconSnapshot, SystemTrayMouseButton };

export interface ShowTrayPanelRequest {
  anchorLeft: number;
  anchorWidth: number;
}

export const TRAY_PANEL_CLOSED_EVENT = 'tray-panel:closed';

export function showTrayPanel(request: ShowTrayPanelRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showTrayPanel, { request });
}

export function hideTrayPanel(): Promise<void> {
  return invoke(IPC_COMMANDS.hideTrayPanel);
}

export async function listTrayPanelIcons(): Promise<SystemTrayIconSnapshot[]> {
  return listSystemTrayIcons();
}

export async function invokeTrayPanelIcon(id: string, button: SystemTrayMouseButton): Promise<void> {
  await invokeSystemTrayIcon(trayClickRequest(id, button));
}
