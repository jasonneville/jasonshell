import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export function showQuickLaunchPanel(args: { anchorLeft: number; anchorWidth: number; nonce: string; rows: unknown[] }): Promise<void> {
  return invoke(IPC_COMMANDS.showQuickLaunchPanel, { args });
}

export function hideQuickLaunchPanel(): Promise<void> {
  return invoke(IPC_COMMANDS.hideQuickLaunchPanel);
}

export function selectQuickLaunchPanel(args: { nonce: string; shortcutPath: string }): Promise<void> {
  return invoke(IPC_COMMANDS.selectQuickLaunchPanel, { args });
}

export function runQuickLaunchPanelAsAdmin(args: { nonce: string; shortcutPath: string }): Promise<void> {
  return invoke(IPC_COMMANDS.runQuickLaunchPanelAsAdmin, { args });
}

export function showQuickLaunchPanelContextMenu(args: { nonce: string; shortcutPath: string; x: number; y: number }): Promise<void> {
  return invoke(IPC_COMMANDS.showQuickLaunchPanelContextMenu, { args });
}
