import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';
import {
  filterTaskGalleryItems,
  nextTaskGalleryFocusIndex,
  reconcileTaskGalleryFocus,
  type TaskGalleryFilterState,
  type TaskGalleryFocusState,
  type TaskGalleryItem
} from '../features/bottom-bar/taskbarUxState';
import type { TaskbarWindow } from './taskbarWindows';

export type TaskGalleryOpenPayload = {
  nonce: string;
  groupKey: string;
  label: string;
  anchorLeft: number;
  anchorWidth: number;
  focusGallery: boolean;
  refreshExisting: boolean;
  windows: TaskbarWindow[];
};

export type TaskGalleryContextMenuRequest = { nonce: string; hwnd: string; x: number; y: number };
export type TaskGalleryPreviewRequest = {
  nonce: string;
  requestId: number;
  hwnd: string;
  title: string;
  processName: string;
  iconDataUrl: string;
  isMinimized: boolean;
  anchorLeft: number;
  anchorWidth: number;
};

export type TaskGalleryWindow = TaskbarWindow;

export function normalizeTaskGalleryProcessId(processId?: number | null): number | null {
  return typeof processId === 'number' && processId > 0 ? processId : null;
}

export function showTaskGallery(request: TaskGalleryOpenPayload): Promise<void> {
  return invoke(IPC_COMMANDS.showTaskGallery, { args: request });
}

export function hideTaskGallery(nonce?: string | null): Promise<void> {
  return invoke(IPC_COMMANDS.hideTaskGallery, { nonce: nonce ?? null });
}

export function hideTaskGalleryOnFocusLoss(): Promise<void> {
  return invoke(IPC_COMMANDS.hideTaskGalleryOnFocusLoss, { args: {} });
}

export function activateTaskGalleryWindow(hwnd: string, nonce: string, minimizeIfActive = false): Promise<void> {
  return invoke(IPC_COMMANDS.activateTaskGalleryWindow, { args: { hwnd, nonce, minimizeIfActive } });
}

export function showTaskGalleryWindowContextMenu(args: TaskGalleryContextMenuRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showTaskGalleryWindowContextMenu, { args });
}

export function showTaskGalleryWindowPreview(args: TaskGalleryPreviewRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showTaskGalleryWindowPreview, { args });
}

export function hideTaskGalleryWindowPreview(args: { nonce: string; requestId: number; hwnd: string }): Promise<void> {
  return invoke(IPC_COMMANDS.hideTaskGalleryWindowPreview, args);
}

export { filterTaskGalleryItems, nextTaskGalleryFocusIndex, reconcileTaskGalleryFocus };
export type { TaskGalleryFilterState, TaskGalleryFocusState, TaskGalleryItem };
