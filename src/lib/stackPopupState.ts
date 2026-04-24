import type { StackEntry } from './stackPopup';

export type StackPopupViewState = {
  currentPath: string;
  entries: StackEntry[];
  selectedPath: string | null;
  history: string[];
  historyIndex: number;
  statusMessage: string;
};

export const defaultStackPopupViewState: StackPopupViewState = {
  currentPath: '',
  entries: [],
  selectedPath: null,
  history: [],
  historyIndex: -1,
  statusMessage: 'Choose a pinned folder'
};

export function openStackFolder(
  current: StackPopupViewState,
  folderPath: string
): StackPopupViewState {
  const normalizedPath = folderPath.trim();
  if (!normalizedPath) {
    return current;
  }

  if (current.history[current.historyIndex] === normalizedPath) {
    return {
      ...current,
      currentPath: normalizedPath,
      entries: [],
      selectedPath: null
    };
  }

  const retainedHistory = current.history.slice(0, current.historyIndex + 1);
  return {
    ...current,
    currentPath: normalizedPath,
    entries: [],
    selectedPath: null,
    history: [...retainedHistory, normalizedPath],
    historyIndex: retainedHistory.length,
    statusMessage: 'Loading folder...'
  };
}

export function applyStackEntries(
  current: StackPopupViewState,
  folderPath: string,
  entries: StackEntry[]
): StackPopupViewState {
  if (folderPath !== current.currentPath) {
    return current;
  }

  return {
    ...current,
    entries,
    selectedPath: entries.some((entry) => entry.path === current.selectedPath)
      ? current.selectedPath
      : null,
    statusMessage: entries.length ? `${entries.length} items` : 'This folder is empty'
  };
}

export function selectStackEntry(
  current: StackPopupViewState,
  entryPath: string | null
): StackPopupViewState {
  return {
    ...current,
    selectedPath: entryPath
  };
}

export function navigateStackHistory(
  current: StackPopupViewState,
  direction: -1 | 1
): StackPopupViewState {
  const nextIndex = current.historyIndex + direction;
  if (nextIndex < 0 || nextIndex >= current.history.length) {
    return current;
  }

  return {
    ...current,
    currentPath: current.history[nextIndex],
    entries: [],
    selectedPath: null,
    historyIndex: nextIndex,
    statusMessage: 'Loading folder...'
  };
}

export function canNavigateStackBack(state: StackPopupViewState) {
  return state.historyIndex > 0;
}

export function canNavigateStackForward(state: StackPopupViewState) {
  return state.historyIndex >= 0 && state.historyIndex < state.history.length - 1;
}

export function selectedStackEntry(state: StackPopupViewState) {
  return state.entries.find((entry) => entry.path === state.selectedPath) ?? null;
}

export function formatStackSize(size: number | null | undefined) {
  if (size === null || size === undefined) {
    return '';
  }

  if (size < 1024) {
    return `${size} B`;
  }

  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = size / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unitIndex]}`;
}

export function stackItemNameFromPath(path: string) {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}
