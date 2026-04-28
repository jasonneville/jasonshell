import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export type ShowTaskPreviewRequest = {
  requestId: number;
  hwnd: string;
  title: string;
  processName: string;
  iconDataUrl: string;
  isMinimized: boolean;
  anchorLeft: number;
  anchorWidth: number;
};

export type TaskPreviewPayload = {
  hwnd: string;
  title: string;
  processName: string;
  iconDataUrl: string;
  isMinimized: boolean;
  imageDataUrl?: string | null;
  width?: number | null;
  height?: number | null;
  error?: string | null;
};

export function showTaskWindowPreview(request: ShowTaskPreviewRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showTaskWindowPreview, { request });
}

export function hideTaskWindowPreview(requestId: number): Promise<void> {
  return invoke(IPC_COMMANDS.hideTaskWindowPreview, { requestId });
}
