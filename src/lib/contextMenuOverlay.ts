import { emitTo } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export const CONTEXT_MENU_OVERLAY_LABEL = 'context-menu-overlay';
export const CONTEXT_MENU_OVERLAY_OPEN_EVENT = 'context-menu-overlay:open';
export const CONTEXT_MENU_OVERLAY_SELECT_EVENT = 'context-menu-overlay:select';

export type ContextMenuOverlaySource = 'top-bar' | 'bottom-bar';
export type ContextMenuOverlayKind = 'pin' | 'task-window' | 'launcher';

export type ContextMenuOverlayRequest = {
  source: ContextMenuOverlaySource;
  kind: ContextMenuOverlayKind;
  token: string;
  x: number;
  y: number;
  isMinimized?: boolean;
  processId?: number | null;
};

export type ContextMenuOverlaySelection = Pick<ContextMenuOverlayRequest, 'source' | 'kind' | 'token'> & {
  action: string;
};

export async function showContextMenuOverlay(request: ContextMenuOverlayRequest): Promise<void> {
  await invoke(IPC_COMMANDS.showContextMenuOverlay, { request });
  await emitTo(CONTEXT_MENU_OVERLAY_LABEL, CONTEXT_MENU_OVERLAY_OPEN_EVENT, request);
}

export function hideContextMenuOverlay(
  source?: ContextMenuOverlaySource,
  restoreOriginFocus = false
): Promise<void> {
  return invoke(IPC_COMMANDS.hideContextMenuOverlay, { request: { source, restoreOriginFocus } });
}

export function dispatchContextMenuOverlaySelection(selection: ContextMenuOverlaySelection): Promise<void> {
  return emitTo(selection.source, CONTEXT_MENU_OVERLAY_SELECT_EVENT, selection);
}
