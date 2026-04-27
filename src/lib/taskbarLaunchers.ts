import { invoke } from '@tauri-apps/api/core';

export type PinnedTaskbarLauncher = {
  id: string;
  name: string;
  shortcutPath: string;
  iconDataUrl: string;
};

export function listPinnedTaskbarLaunchers(): Promise<PinnedTaskbarLauncher[]> {
  return invoke('list_pinned_taskbar_apps');
}

export function launchPinnedTaskbarLauncher(shortcutPath: string): Promise<void> {
  return invoke('launch_pinned_taskbar_app', { shortcutPath });
}
