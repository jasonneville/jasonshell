import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';
import type { PinnedTaskbarLauncher } from './taskbarLaunchers.js';

export type QuickIcon = {
  id: string;
  name: string;
  targetPath: string;
  iconDataUrl: string;
};

export interface QuickIconsSettings {
  entries: QuickIcon[];
}

export type PinTaskWindowQuickIconRequest = {
  hwnd: string;
};

export type UnpinQuickIconRequest = {
  id: string;
};

export function defaultQuickIconsSettings(): QuickIconsSettings {
  return {
    entries: []
  };
}

export function listQuickIcons(): Promise<QuickIcon[]> {
  return invoke<QuickIcon[]>(IPC_COMMANDS.listQuickIcons);
}

export function pinTaskWindowQuickIcon(request: PinTaskWindowQuickIconRequest): Promise<QuickIcon[]> {
  return invoke<QuickIcon[]>(IPC_COMMANDS.pinTaskWindowQuickIcon, { request });
}

export function unpinQuickIcon(request: UnpinQuickIconRequest): Promise<QuickIcon[]> {
  return invoke<QuickIcon[]>(IPC_COMMANDS.unpinQuickIcon, { request });
}

export function launchQuickIcon(request: UnpinQuickIconRequest): Promise<void> {
  return invoke(IPC_COMMANDS.launchQuickIcon, { request });
}

export function normalizeQuickIconTargetKey(path: string): string {
  const trimmed = path.trim();
  if (!trimmed) {
    return '';
  }
  if (/^[a-zA-Z]:[\\/]/u.test(trimmed) || /^\\\\[^\\]/u.test(trimmed)) {
    return trimmed.replace(/\//g, '\\').toLocaleLowerCase();
  }
  return trimmed.toLocaleLowerCase();
}

export function filterExplorerLaunchersForQuickIcons(
  quickIcons: readonly QuickIcon[],
  launchers: readonly PinnedTaskbarLauncher[]
): PinnedTaskbarLauncher[] {
  void quickIcons;
  return [...launchers];
}
