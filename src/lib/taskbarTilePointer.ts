export interface TaskbarTilePointerReleaseResult {
  activateHwnd: string | null;
  suppressClickHwnd: string | null;
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
