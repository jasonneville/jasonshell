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
  galleryNonce?: string | null;
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
  galleryNonce?: string | null;
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

export function allocateTaskPreviewRequestId(): Promise<number> {
  return invoke(IPC_COMMANDS.allocateTaskPreviewRequestId);
}

export function hideTaskWindowPreview(requestId: number): Promise<void> {
  return invoke(IPC_COMMANDS.hideTaskWindowPreview, { requestId });
}

export function closePreviewedTaskWindow(hwnd: string, galleryNonce?: string | null): Promise<void> {
  if (!hwnd.trim()) {
    return Promise.reject(new Error('Missing preview task window handle'));
  }
  return galleryNonce
    ? invoke(IPC_COMMANDS.closeTaskGalleryPreviewedWindow, { args: { nonce: galleryNonce, hwnd } })
    : invoke(IPC_COMMANDS.closeTaskWindow, { hwnd });
}
