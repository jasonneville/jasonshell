import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands';

export interface ShowSettingsPanelRequest {
  anchorLeft: number;
  anchorWidth: number;
}

export function showSettingsPanel(request: ShowSettingsPanelRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showSettingsPanel, { request });
}

export function hideSettingsPanel(): Promise<void> {
  return invoke(IPC_COMMANDS.hideSettingsPanel);
}
