import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export const TERMINAL_PANEL_CLOSED_EVENT = 'terminal-panel:closed';

export type ShowTerminalPanelRequest = {
  anchorLeft: number;
  anchorWidth: number;
};

export function showTerminalPanel(request: ShowTerminalPanelRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showTerminalPanel, { request });
}

export function hideTerminalPanel(): Promise<void> {
  return invoke(IPC_COMMANDS.hideTerminalPanel);
}
