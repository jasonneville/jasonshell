import type { TaskWindowGroup } from '../../lib/taskbarGroups';

export type TaskbarOverflowState = {
  hasOverflow: boolean;
  summary: string;
};

export type TaskGalleryItem = {
  hwnd: string;
  title: string;
  processName: string;
  processId?: number | null;
  iconDataUrl: string;
  isActive: boolean;
  isMinimized: boolean;
  activityState?: 'idle' | 'busy';
  attentionState?: 'idle' | 'requested';
  toastCount?: number;
};

export type TaskGalleryFilterState = {
  items: TaskGalleryItem[];
};

export type TaskGalleryFocusState = {
  focusedHwnd: string | null;
  focusedIndex: number;
};

export function taskGroupStateLabel(group: TaskWindowGroup): string {
  const parts = [group.label];
  parts.push(group.windows.length === 1 ? '1 window' : `${group.windows.length} windows`);
  if (!group.isActive && group.toastCount > 0) {
    parts.push(group.toastCount === 1 ? '1 toast' : `${group.toastCount} toasts`);
  }
  if (!group.isActive && (group.hasAttention || group.toastCount > 0)) {
    parts.push('needs attention');
  }
  if (group.isActive) {
    parts.push('active');
  }
  if (group.isBusy) {
    parts.push('activity detected');
  }
  if (group.isMinimized) {
    parts.push('minimized');
  }
  return parts.join(', ');
}

export function taskbarOverflowState(
  clientWidth: number,
  scrollWidth: number,
  groupCount: number
): TaskbarOverflowState {
  const hasOverflow = scrollWidth - clientWidth > 2;
  return {
    hasOverflow,
    summary: hasOverflow
      ? `${groupCount} task groups, use arrow keys to move through hidden items`
      : `${groupCount} task groups visible`
  };
}

export function nextTaskbarFocusIndex(
  currentIndex: number,
  itemCount: number,
  key: string
): number {
  if (itemCount <= 0) {
    return -1;
  }

  const boundedIndex = Math.min(Math.max(currentIndex, 0), itemCount - 1);
  if (key === 'Home') {
    return 0;
  }
  if (key === 'End') {
    return itemCount - 1;
  }
  if (key === 'ArrowRight' || key === 'ArrowDown') {
    return Math.min(itemCount - 1, boundedIndex + 1);
  }
  if (key === 'ArrowLeft' || key === 'ArrowUp') {
    return Math.max(0, boundedIndex - 1);
  }
  return boundedIndex;
}

function includesQuery(value: string, query: string) {
  return value.toLocaleLowerCase().includes(query.toLocaleLowerCase());
}

export function filterTaskGalleryItems(
  items: TaskGalleryItem[],
  query: string
): TaskGalleryFilterState {
  const normalizedQuery = query.trim();
  const filteredItems = normalizedQuery
    ? items.filter((item) => includesQuery(item.title, normalizedQuery) || includesQuery(item.processName, normalizedQuery))
    : items.slice();

  return { items: filteredItems };
}

export function nextTaskGalleryFocusIndex(
  currentIndex: number,
  itemCount: number,
  key: string
): number {
  if (itemCount <= 0) return -1;
  const boundedIndex = Math.min(Math.max(currentIndex, 0), itemCount - 1);
  if (key === 'Home') return 0;
  if (key === 'End') return itemCount - 1;
  if (key === 'ArrowRight' || key === 'ArrowDown') return Math.min(itemCount - 1, boundedIndex + 1);
  if (key === 'ArrowLeft' || key === 'ArrowUp') return Math.max(0, boundedIndex - 1);
  return boundedIndex;
}

export function reconcileTaskGalleryFocus(
  focusedHwnd: string | null,
  items: TaskGalleryItem[]
): TaskGalleryFocusState {
  if (!items.length) {
    return { focusedHwnd: null, focusedIndex: -1 };
  }

  const focusedIndex = focusedHwnd == null ? -1 : items.findIndex((item) => item.hwnd === focusedHwnd);
  const nextIndex = focusedIndex >= 0 ? focusedIndex : 0;
  return { focusedHwnd: items[nextIndex].hwnd, focusedIndex: nextIndex };
}
