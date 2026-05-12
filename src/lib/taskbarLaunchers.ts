import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export type PinnedTaskbarLauncher = {
  id: string;
  name: string;
  shortcutPath: string;
  targetPath: string | null;
  iconDataUrl: string;
};

export function listPinnedTaskbarLaunchers(): Promise<PinnedTaskbarLauncher[]> {
  return invoke(IPC_COMMANDS.listPinnedTaskbarApps);
}

export function launchPinnedTaskbarLauncher(shortcutPath: string): Promise<void> {
  return invoke(IPC_COMMANDS.launchPinnedTaskbarApp, { shortcutPath });
}
