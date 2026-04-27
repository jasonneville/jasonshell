import type { TaskbarWindow } from './taskbarWindows';

export type TaskWindowGroup = {
  key: string;
  label: string;
  iconDataUrl: string;
  windows: TaskbarWindow[];
  isActive: boolean;
  isMinimized: boolean;
  isBusy: boolean;
};

export type TaskbarGroupRect = {
  key: string;
  left: number;
  width: number;
};

export type TaskbarGroupDropTarget = {
  targetKey: string;
  placement: 'before' | 'after';
};

export const TASKBAR_GROUP_DRAG_THRESHOLD_PX = 6;

const TERMINAL_ACTIVITY_PATTERNS = [
  'terminal',
  'windowsterminal',
  'windows terminal',
  'wt',
  'cmd',
  'command prompt',
  'powershell',
  'pwsh',
  'conhost',
  'console'
];

const LLM_ACTIVITY_PATTERNS = [
  'opencode',
  'open code',
  'claude',
  'copilot',
  'cursor',
  'aider',
  'continue',
  'llm',
  'chatgpt',
  'codex'
];

const BROWSER_ACTIVITY_PATTERNS = ['firefox', 'chrome', 'msedge', 'edge', 'brave', 'opera', 'vivaldi'];
const DOWNLOAD_ACTIVITY_PATTERNS = ['download', 'downloading', 'downloads'];

export function taskWindowGroupKey(taskWindow: TaskbarWindow) {
  const processName = taskWindow.processName.trim().toLocaleLowerCase();
  return processName || `window:${taskWindow.hwnd}`;
}

function taskWindowMetadata(taskWindow: TaskbarWindow) {
  return `${taskWindow.processName} ${taskWindow.title}`.trim().toLocaleLowerCase();
}

function taskWindowProcessMetadata(taskWindow: TaskbarWindow) {
  return taskWindow.processName.trim().toLocaleLowerCase();
}

function taskWindowTitleMetadata(taskWindow: TaskbarWindow) {
  return taskWindow.title.trim().toLocaleLowerCase();
}

function includesActivityPattern(metadata: string, patterns: string[]) {
  return patterns.some((pattern) => metadata.includes(pattern));
}

export function isTaskWindowActivityIndicatorEligible(taskWindow: TaskbarWindow) {
  const metadata = taskWindowMetadata(taskWindow);
  const processMetadata = taskWindowProcessMetadata(taskWindow);
  const titleMetadata = taskWindowTitleMetadata(taskWindow);
  if (!metadata && !processMetadata) {
    return false;
  }

  const isTerminalProcess = includesActivityPattern(processMetadata, TERMINAL_ACTIVITY_PATTERNS);
  const isLlmProcess = includesActivityPattern(processMetadata, LLM_ACTIVITY_PATTERNS);
  const hasTerminalTitleLlmSignal = isTerminalProcess
    && includesActivityPattern(titleMetadata, LLM_ACTIVITY_PATTERNS);
  if (isTerminalProcess || isLlmProcess || hasTerminalTitleLlmSignal) {
    return true;
  }

  return includesActivityPattern(processMetadata, BROWSER_ACTIVITY_PATTERNS)
    && includesActivityPattern(metadata, DOWNLOAD_ACTIVITY_PATTERNS);
}

export function buildTaskWindowGroups(
  windows: TaskbarWindow[],
  preferredOrder: string[] = []
): TaskWindowGroup[] {
  const groups = new Map<string, TaskWindowGroup>();

  for (const taskWindow of windows) {
    const key = taskWindowGroupKey(taskWindow);
    const group = groups.get(key);

    if (group) {
      group.windows.push(taskWindow);
      group.isActive ||= taskWindow.isActive;
      group.isMinimized &&= taskWindow.isMinimized;
      group.isBusy ||= taskWindow.activityState === 'busy'
        && isTaskWindowActivityIndicatorEligible(taskWindow);
      continue;
    }

    groups.set(key, {
      key,
      label: taskWindow.processName || taskWindow.title || 'Application',
      iconDataUrl: taskWindow.iconDataUrl,
      windows: [taskWindow],
      isActive: taskWindow.isActive,
      isMinimized: taskWindow.isMinimized,
      isBusy: taskWindow.activityState === 'busy'
        && isTaskWindowActivityIndicatorEligible(taskWindow)
    });
  }

  const firstSeenOrder = Array.from(groups.keys());
  const orderedKeys = reconcileTaskWindowGroupOrder(preferredOrder, firstSeenOrder);
  return orderedKeys.map((key) => groups.get(key)).filter((group): group is TaskWindowGroup => Boolean(group));
}

export function reconcileTaskWindowGroupOrder(previousOrder: string[], visibleKeys: string[]) {
  const visible = new Set(visibleKeys);
  const ordered = previousOrder.filter((key) => visible.has(key));

  for (const key of visibleKeys) {
    if (!ordered.includes(key)) {
      ordered.push(key);
    }
  }

  return ordered;
}

export function moveTaskWindowGroup(
  order: string[],
  sourceKey: string,
  targetKey: string,
  placement: 'before' | 'after' = 'before'
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

export function taskbarGroupDragDelta(startClientX: number, currentClientX: number) {
  return currentClientX - startClientX;
}

export function taskbarGroupReorderOffset(
  sourceKey: string,
  currentOrder: string[],
  capturedRects: TaskbarGroupRect[]
) {
  const rectsByKey = new Map(capturedRects.map((rect) => [rect.key, rect]));
  const sourceRect = rectsByKey.get(sourceKey);
  if (!sourceRect) {
    return 0;
  }

  const orderedRects = currentOrder
    .map((key) => rectsByKey.get(key))
    .filter((rect): rect is TaskbarGroupRect => Boolean(rect));
  const firstLeft = Math.min(...capturedRects.map((rect) => rect.left));
  let nextLeft = firstLeft;

  for (const rect of orderedRects) {
    if (rect.key === sourceKey) {
      return sourceRect.left - nextLeft;
    }
    nextLeft += rect.width;
  }

  return 0;
}

export function hasTaskbarGroupDragStarted(
  startClientX: number,
  currentClientX: number,
  thresholdPx = TASKBAR_GROUP_DRAG_THRESHOLD_PX
) {
  return Math.abs(taskbarGroupDragDelta(startClientX, currentClientX)) >= thresholdPx;
}

export function taskbarGroupDropPlacement(
  pointerClientX: number,
  targetLeft: number,
  targetWidth: number
): 'before' | 'after' {
  return pointerClientX > targetLeft + targetWidth / 2 ? 'after' : 'before';
}

export function taskbarGroupDropTargetFromPointer(
  pointerClientX: number,
  groupRects: TaskbarGroupRect[],
  sourceKey: string
): TaskbarGroupDropTarget | null {
  const candidates = groupRects.filter((rect) => rect.key !== sourceKey && rect.width > 0);
  if (!candidates.length) {
    return null;
  }

  const containingCandidate = candidates.find(
    (candidate) => pointerClientX >= candidate.left && pointerClientX <= candidate.left + candidate.width
  );
  if (containingCandidate) {
    return {
      targetKey: containingCandidate.key,
      placement: taskbarGroupDropPlacement(
        pointerClientX,
        containingCandidate.left,
        containingCandidate.width
      )
    };
  }

  for (const candidate of candidates) {
    if (pointerClientX <= candidate.left + candidate.width / 2) {
      return { targetKey: candidate.key, placement: 'before' };
    }
  }

  return { targetKey: candidates[candidates.length - 1].key, placement: 'after' };
}

export function taskbarGroupDropTargetFromDisplacement(
  sourceKey: string,
  initialOrder: string[],
  groupRects: TaskbarGroupRect[],
  dragDeltaX: number
): TaskbarGroupDropTarget | null {
  const sourceIndex = initialOrder.indexOf(sourceKey);
  if (sourceIndex < 0 || initialOrder.length < 2) {
    return null;
  }

  const rectsByKey = new Map(groupRects.map((rect) => [rect.key, rect]));
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
    return { targetKey: sourceKey, placement: 'before' };
  }

  return destinationIndex < sourceIndex
    ? { targetKey: initialOrder[destinationIndex], placement: 'before' }
    : { targetKey: initialOrder[destinationIndex], placement: 'after' };
}

export function taskbarGroupOrderFromDisplacement(
  sourceKey: string,
  initialOrder: string[],
  groupRects: TaskbarGroupRect[],
  dragDeltaX: number
) {
  const target = taskbarGroupDropTargetFromDisplacement(
    sourceKey,
    initialOrder,
    groupRects,
    dragDeltaX
  );
  if (!target) {
    return initialOrder;
  }

  return moveTaskWindowGroup(initialOrder, sourceKey, target.targetKey, target.placement);
}
