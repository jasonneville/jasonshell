import { invoke, type Channel } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands';

export type QuickCommandBanner = { id: string; label: string; status: 'success' | 'failure' };
export type QuickCommandBannerAction = 'open' | 'dismiss' | 'pause' | 'resume';
export type QuickCommandRunTarget = { commandId: string; runId: string };

export function getQuickCommandNotification(options?: { channel?: Channel<null>; renderedId?: string }): Promise<QuickCommandBanner | null> {
  return invoke(IPC_COMMANDS.getQuickCommandNotification, options);
}

export function actOnQuickCommandNotification(id: string, action: QuickCommandBannerAction): Promise<void> {
  return invoke(IPC_COMMANDS.actOnQuickCommandNotification, { id, action });
}

export function takeQuickCommandNotificationTarget(): Promise<QuickCommandRunTarget | null> {
  return invoke(IPC_COMMANDS.takeQuickCommandNotificationTarget);
}
