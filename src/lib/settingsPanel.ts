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

export const SYSTEM_POWER_ACTIONS = ['sleep', 'restart', 'shutdown'] as const;

export type SystemPowerAction = 'sleep' | 'restart' | 'shutdown';

export interface SystemPowerActionRequest {
  action: SystemPowerAction;
}

export function triggerSystemPowerAction(request: SystemPowerActionRequest): Promise<void> {
  if (!SYSTEM_POWER_ACTIONS.includes(request.action)) {
    return Promise.reject(new Error('Invalid system power action'));
  }
  return invoke(IPC_COMMANDS.triggerSystemPowerAction, { request });
}
