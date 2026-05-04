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

export type QuickIconLaunchError = {
  code: string;
  message: string;
  pathOrId: string;
};

export type QuickIconLaunchFailureState = {
  quickIcons: QuickIcon[];
  errorById: Record<string, QuickIconLaunchError>;
};

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

export function createQuickIconLaunchError(
  code: string,
  message: string,
  pathOrId: string
): QuickIconLaunchError {
  return { code, message, pathOrId };
}

export function quickIconLaunchErrorFromUnknown(error: unknown, pathOrId: string): QuickIconLaunchError {
  const message = error instanceof Error ? error.message : String(error || 'Quick icon launch failed');
  return createQuickIconLaunchError('launchFailed', message, pathOrId);
}

export function buildQuickIconLaunchFailureState(
  quickIcons: readonly QuickIcon[],
  id: string,
  error: QuickIconLaunchError
): QuickIconLaunchFailureState {
  return {
    quickIcons: [...quickIcons],
    errorById: {
      [id]: error
    }
  };
}

export function removeQuickIconEntry(
  quickIcons: readonly QuickIcon[],
  id: string
): QuickIcon[] {
  return quickIcons.filter((entry) => entry.id !== id);
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
