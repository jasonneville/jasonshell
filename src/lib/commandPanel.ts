import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export interface ShowCommandPanelRequest {
  anchorLeft: number;
  anchorWidth: number;
}

export const COMMAND_PANEL_CLOSED_EVENT = 'command-panel:closed';

export function showCommandPanel(request: ShowCommandPanelRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showCommandPanel, { request });
}

export function hideCommandPanel(): Promise<void> {
  return invoke(IPC_COMMANDS.hideCommandPanel);
}

export function saveCommandPanelSize(widthPhysical: number, heightPhysical: number): Promise<unknown> {
  return invoke(IPC_COMMANDS.saveCommandPanelSize, { widthPhysical, heightPhysical });
}
