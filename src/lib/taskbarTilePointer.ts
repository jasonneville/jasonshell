export interface TaskbarTilePointerReleaseResult {
  activateHwnd: string | null;
  suppressClickHwnd: string | null;
}

export interface TaskGalleryPointerReleaseResult {
  openGroupKey: string | null;
  suppressClickGroupKey: string | null;
}

export function pendingTaskGalleryPointer(
  button: number,
  groupKey: string,
  isCapsule: boolean
): string | null {
  return button === 0 && isCapsule ? groupKey : null;
}

export function clearTaskGalleryClickSuppressionOnPointerDown(
  suppressClickGroupKey: string | null,
  button: number,
  isCapsule: boolean
): string | null {
  return button === 0 && isCapsule ? null : suppressClickGroupKey;
}

export function shouldSuppressTaskGalleryClick(
  suppressClickGroupKey: string | null,
  clickedGroupKey: string,
  clickDetail: number
): boolean {
  return clickDetail > 0 && suppressClickGroupKey === clickedGroupKey;
}

export function resolveTaskGalleryPointerRelease(
  pendingGroupKey: string | null,
  dragStarted: boolean
): TaskGalleryPointerReleaseResult {
  if (!pendingGroupKey) {
    return { openGroupKey: null, suppressClickGroupKey: null };
  }

  return {
    openGroupKey: dragStarted ? null : pendingGroupKey,
    suppressClickGroupKey: pendingGroupKey
  };
}

export function pendingTaskbarTilePointer(button: number, hwnd: string): string | null {
  return button === 0 ? hwnd : null;
}

export function resolveTaskbarTilePointerRelease(
  pendingHwnd: string | null,
  dragStarted: boolean
): TaskbarTilePointerReleaseResult {
  if (!pendingHwnd) {
    return {
      activateHwnd: null,
      suppressClickHwnd: null
    };
  }

  return dragStarted
    ? {
        activateHwnd: null,
        suppressClickHwnd: pendingHwnd
      }
    : {
        activateHwnd: pendingHwnd,
        suppressClickHwnd: pendingHwnd
      };
}

export function shouldSuppressTaskbarTileClick(
  suppressClickHwnd: string | null,
  clickedHwnd: string
): boolean {
  return suppressClickHwnd === clickedHwnd;
}
