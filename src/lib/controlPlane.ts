import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export const CONTROL_PLANE_LABEL = 'control-plane';

export function showControlPlane(): Promise<void> {
  return invoke(IPC_COMMANDS.showControlPlane);
}

export function hideControlPlane(): Promise<void> {
  return invoke(IPC_COMMANDS.hideControlPlane);
}
