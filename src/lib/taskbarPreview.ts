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

export const TASK_PREVIEW_SOURCES = {
  capturedImage: 'captured-image',
  nativeDwmThumbnail: 'native-dwm-thumbnail',
  unavailable: 'unavailable'
} as const;

export type TaskPreviewSource = (typeof TASK_PREVIEW_SOURCES)[keyof typeof TASK_PREVIEW_SOURCES];

export type TaskPreviewPayload = {
  hwnd: string;
  title: string;
  processName: string;
  iconDataUrl: string;
  isMinimized: boolean;
  previewSource?: TaskPreviewSource | null;
  nativeLiveThumbnailActive?: boolean | null;
  imageDataUrl?: string | null;
  width?: number | null;
  height?: number | null;
  error?: string | null;
};

export function isNativeLiveTaskPreviewPayload(payload: TaskPreviewPayload): boolean {
  return (
    payload.nativeLiveThumbnailActive === true ||
    payload.previewSource === TASK_PREVIEW_SOURCES.nativeDwmThumbnail
  );
}

export function showTaskWindowPreview(request: ShowTaskPreviewRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showTaskWindowPreview, { request });
}

export function hideTaskWindowPreview(requestId: number): Promise<void> {
  return invoke(IPC_COMMANDS.hideTaskWindowPreview, { requestId });
}
