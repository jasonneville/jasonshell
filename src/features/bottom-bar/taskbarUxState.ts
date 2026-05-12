import type { TaskWindowGroup } from '../../lib/taskbarGroups';

export type TaskbarOverflowState = {
  hasOverflow: boolean;
  summary: string;
};

export function taskGroupStateLabel(group: TaskWindowGroup): string {
  const parts = [group.label];
  parts.push(group.windows.length === 1 ? '1 window' : `${group.windows.length} windows`);
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
