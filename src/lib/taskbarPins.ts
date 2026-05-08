import type { PinnedTaskbarLauncher } from './taskbarLaunchers.js';

export type TaskbarLauncherLike = {
  shortcutPath: string;
};

export type TaskbarLauncherRect = {
  key: string;
  left: number;
  width: number;
};

export type TaskbarLauncherPointerRelease = {
  suppressClickKey: string | null;
};

export const TASKBAR_LAUNCHER_DRAG_THRESHOLD_PX = 6;

export function normalizeTaskbarPinTargetKey(path: string): string {
  const trimmed = path.trim();
  if (!trimmed) {
    return '';
  }
  if (/^[a-zA-Z]:[\\/]/u.test(trimmed) || /^\\\\[^\\]/u.test(trimmed)) {
    return trimmed.replace(/\//g, '\\').toLocaleLowerCase();
  }
  return trimmed.toLocaleLowerCase();
}

export function preserveExplorerTaskbarPins(
  launchers: readonly PinnedTaskbarLauncher[]
): PinnedTaskbarLauncher[] {
  return [...launchers];
}

export function taskbarLauncherKey(launcher: TaskbarLauncherLike) {
  return launcher.shortcutPath;
}

export function reconcileTaskbarLauncherOrder(
  previousOrder: string[],
  launchers: readonly TaskbarLauncherLike[]
) {
  const visibleKeys = launchers.map(taskbarLauncherKey);
  const visible = new Set(visibleKeys);
  const ordered = previousOrder.filter((key) => visible.has(key));

  for (const key of visibleKeys) {
    if (!ordered.includes(key)) {
      ordered.push(key);
    }
  }

  return ordered;
}

export function orderTaskbarLaunchers<T extends TaskbarLauncherLike>(
  launchers: readonly T[],
  preferredOrder: string[]
): T[] {
  const byKey = new Map(launchers.map((launcher) => [taskbarLauncherKey(launcher), launcher]));
  const ordered = preferredOrder
    .map((key) => byKey.get(key))
    .filter((launcher): launcher is T => Boolean(launcher));
  const orderedKeys = new Set(ordered.map(taskbarLauncherKey));

  for (const launcher of launchers) {
    if (!orderedKeys.has(taskbarLauncherKey(launcher))) {
      ordered.push(launcher);
    }
  }

  return ordered;
}

export function taskbarLauncherDragDelta(startClientX: number, currentClientX: number) {
  return currentClientX - startClientX;
}

export function hasTaskbarLauncherDragStarted(
  startClientX: number,
  currentClientX: number,
  thresholdPx = TASKBAR_LAUNCHER_DRAG_THRESHOLD_PX
) {
  return Math.abs(taskbarLauncherDragDelta(startClientX, currentClientX)) >= thresholdPx;
}

export function resolveTaskbarLauncherPointerRelease(
  pendingLauncherKey: string | null,
  dragStarted: boolean
): TaskbarLauncherPointerRelease {
  return {
    suppressClickKey: dragStarted ? pendingLauncherKey : null
  };
}

function moveTaskbarLauncher(
  order: string[],
  sourceKey: string,
  targetKey: string,
  placement: 'before' | 'after'
) {
  if (sourceKey === targetKey) {
    return order;
  }

  const sourceIndex = order.indexOf(sourceKey);
  const targetIndex = order.indexOf(targetKey);
  if (sourceIndex < 0 || targetIndex < 0) {
    return order;
  }

  const next = order.filter((key) => key !== sourceKey);
  const nextTargetIndex = next.indexOf(targetKey);
  next.splice(placement === 'after' ? nextTargetIndex + 1 : nextTargetIndex, 0, sourceKey);
  return next;
}

function taskbarLauncherDropTargetFromDisplacement(
  sourceKey: string,
  initialOrder: string[],
  rects: TaskbarLauncherRect[],
  dragDeltaX: number
) {
  const sourceIndex = initialOrder.indexOf(sourceKey);
  if (sourceIndex < 0 || initialOrder.length < 2) {
    return null;
  }

  const rectsByKey = new Map(rects.map((rect) => [rect.key, rect]));
  const sourceRect = rectsByKey.get(sourceKey);
  if (!sourceRect || sourceRect.width <= 0) {
    return null;
  }

  const movedCenter = sourceRect.left + sourceRect.width / 2 + dragDeltaX;
  let destinationIndex = sourceIndex;

  if (dragDeltaX > 0) {
    for (let index = sourceIndex + 1; index < initialOrder.length; index += 1) {
      const rect = rectsByKey.get(initialOrder[index]);
      if (rect && rect.width > 0 && movedCenter > rect.left + rect.width / 2) {
        destinationIndex = index;
      }
    }
  } else if (dragDeltaX < 0) {
    for (let index = sourceIndex - 1; index >= 0; index -= 1) {
      const rect = rectsByKey.get(initialOrder[index]);
      if (rect && rect.width > 0 && movedCenter < rect.left + rect.width / 2) {
        destinationIndex = index;
      }
    }
  }

  if (destinationIndex === sourceIndex) {
    return { targetKey: sourceKey, placement: 'before' as const };
  }

  return destinationIndex < sourceIndex
    ? { targetKey: initialOrder[destinationIndex], placement: 'before' as const }
    : { targetKey: initialOrder[destinationIndex], placement: 'after' as const };
}

export function taskbarLauncherOrderFromDisplacement(
  sourceKey: string,
  initialOrder: string[],
  rects: TaskbarLauncherRect[],
  dragDeltaX: number
) {
  const target = taskbarLauncherDropTargetFromDisplacement(sourceKey, initialOrder, rects, dragDeltaX);
  if (!target) {
    return initialOrder;
  }

  return moveTaskbarLauncher(initialOrder, sourceKey, target.targetKey, target.placement);
}
