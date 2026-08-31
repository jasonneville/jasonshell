import type { TaskbarWindow } from './taskbarWindows';

export const TASKBAR_REFRESH_LAUNCHERS_EVENT = 'taskbar:refresh-launchers';
export const TASKBAR_REFRESH_WINDOWS_EVENT = 'taskbar:refresh-windows';
export const TASKBAR_WINDOWS_SNAPSHOT_EVENT = 'taskbar:windows-snapshot';
export const TASK_PREVIEW_HOVER_ENTER_EVENT = 'task-preview:hover-enter';
export const TASK_PREVIEW_HIDE_REQUEST_EVENT = 'task-preview:hide-request';
export const TASK_PREVIEW_DELAY_MS = 180;
export const TASK_PREVIEW_HIDE_DELAY_MS = 140;

export type TaskPreviewHideRequestMode = 'schedule' | 'immediate';

export type TaskPreviewHideRequest = {
  mode: TaskPreviewHideRequestMode;
  preserveGallery?: boolean;
};

export type TaskbarWindowsSnapshotPayload = {
  sequence: number;
  windows: TaskbarWindow[];
};

export function taskWindowLabel(taskWindow: TaskbarWindow) {
  return taskWindow.title || taskWindow.processName;
}

export function taskWindowActionLabel(taskWindow: TaskbarWindow) {
  return `${taskWindow.isActive ? 'Minimize' : 'Focus'} ${taskWindowLabel(taskWindow)}`;
}
