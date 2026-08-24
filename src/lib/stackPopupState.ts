import type { StackEntry, StackFolderListing, StackFolderListingPage } from './stackPopup';

export type StackPopupViewState = {
  currentPath: string;
  entriesPath: string | null;
  entries: StackEntry[];
  sortColumn: StackSortColumn;
  sortDirection: StackSortDirection;
  selectedPath: string | null;
  selectedPaths: string[];
  selectionAnchorPath: string | null;
  history: string[];
  historyIndex: number;
  statusMessage: string;
};

export type StackSortColumn = 'name' | 'type' | 'size' | 'modified';
export type StackSortDirection = 'asc' | 'desc';

export const defaultStackPopupViewState: StackPopupViewState = {
  currentPath: '',
  entriesPath: null,
  entries: [],
  sortColumn: 'name',
  sortDirection: 'asc',
  selectedPath: null,
  selectedPaths: [],
  selectionAnchorPath: null,
  history: [],
  historyIndex: -1,
  statusMessage: 'Choose a pinned folder'
};

export type StackPopupOpenPayload = string | {
  path?: string | null;
  folderPath?: string | null;
  requestId?: string | null;
} | null | undefined;

export type StackSelectionMode = 'single' | 'toggle' | 'range';

export type StackBreadcrumbSegment = {
  name: string;
  path: string;
};

export type StackSortHeaderState = {
  active: boolean;
  ariaSort: 'none' | 'ascending' | 'descending';
  className: string;
  indicator: '' | '↑' | '↓';
};

export type StackOpenWithSuggestion = {
  id: string;
  label: string;
  commandContract: 'open_stack_item_with_app';
};

export type StackEntryIconUpdate = {
  path: string;
  iconDataUrl: string | null;
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
      selectedPath: null,
      selectedPaths: [],
      selectionAnchorPath: null,
      statusMessage: 'Loading folder...'
    };
  }

  const retainedHistory = current.history.slice(0, current.historyIndex + 1);
  return {
    ...current,
    currentPath: normalizedPath,
    selectedPath: null,
    selectedPaths: [],
    selectionAnchorPath: null,
    history: [...retainedHistory, normalizedPath],
    historyIndex: retainedHistory.length,
    statusMessage: 'Loading folder...'
  };
}

export function stackPopupOpenPath(payload: StackPopupOpenPayload): string | null {
  const path = typeof payload === 'string' ? payload : payload?.path ?? payload?.folderPath;
  const normalizedPath = path?.trim() ?? '';
  return normalizedPath || null;
}

export function stackPopupRequestKey(payload: StackPopupOpenPayload): string | null {
  if (!payload) {
    return null;
  }
  if (typeof payload !== 'string') {
    const requestId = payload.requestId?.trim();
    if (requestId) {
      return `request:${requestId}`;
    }
  }
  const path = stackPopupOpenPath(payload);
  return path ? `legacy:${path}` : null;
}

export function normalizeStackDisplayPath(path: string) {
  const trimmed = path.trim();
  if (!trimmed) {
    return '';
  }

  const windowsLike = /^[a-zA-Z]:/.test(trimmed)
    || /^[\\/]{2}[^\\/]/.test(trimmed)
    || trimmed.startsWith('\\\\?\\')
    || trimmed.startsWith('\\??\\');

  if (!windowsLike) {
    return trimmed;
  }

  const normalized = trimmed.replace(/\//g, '\\');
  if (normalized.startsWith('\\\\?\\UNC\\')) {
    return `\\\\${normalized.slice(8)}`;
  }
  if (normalized.startsWith('\\??\\UNC\\')) {
    return `\\\\${normalized.slice(8)}`;
  }
  if (normalized.startsWith('\\\\?\\')) {
    return normalized.slice(4);
  }
  if (normalized.startsWith('\\??\\')) {
    return normalized.slice(4);
  }
  return normalized;
}

export function normalizeStackPathKey(path: string) {
  return normalizeStackDisplayPath(path)
    .replace(/\//g, '\\')
    .replace(/\\+$/, '')
    .toLocaleLowerCase();
}

export function stackGitStatusPathMatchesEntry(entryPath: string, statusPath: string, isFolder: boolean) {
  const entryKey = normalizeStackPathKey(entryPath);
  const statusKey = normalizeStackPathKey(statusPath);
  return statusKey === entryKey || (isFolder && statusKey.startsWith(`${entryKey}\\`));
}

export function stackBreadcrumbSegments(path: string): StackBreadcrumbSegment[] {
  const normalized = normalizeStackDisplayPath(path);
  if (!normalized) {
    return [];
  }

  const driveMatch = normalized.match(/^([a-zA-Z]:)(?:\\(.*))?$/);
  if (driveMatch) {
    const root = `${driveMatch[1]}\\`;
    const segments: StackBreadcrumbSegment[] = [{ name: driveMatch[1], path: root }];
    let current = root;
    for (const segment of (driveMatch[2] ?? '').split(/\\+/).filter(Boolean)) {
      current = `${current}${segment}`;
      segments.push({ name: segment, path: current });
      current = `${current}\\`;
    }
    return segments;
  }

  const uncMatch = normalized.match(/^(\\\\[^\\]+\\[^\\]+)(?:\\(.*))?$/);
  if (uncMatch) {
    const root = uncMatch[1];
    const segments: StackBreadcrumbSegment[] = [{ name: root, path: root }];
    let current = root;
    for (const segment of (uncMatch[2] ?? '').split(/\\+/).filter(Boolean)) {
      current = `${current}\\${segment}`;
      segments.push({ name: segment, path: current });
    }
    return segments;
  }

  if (normalized.startsWith('/')) {
    const segments: StackBreadcrumbSegment[] = [{ name: '/', path: '/' }];
    let current = '';
    for (const segment of normalized.split('/').filter(Boolean)) {
      current = `${current}/${segment}`;
      segments.push({ name: segment, path: current });
    }
    return segments;
  }

  const segments: StackBreadcrumbSegment[] = [];
  let current = '';
  for (const segment of normalized.split(/[\\/]+/).filter(Boolean)) {
    current = current ? `${current}/${segment}` : segment;
    segments.push({ name: segment, path: current });
  }
  return segments;
}

export function parentStackPath(path: string) {
  const segments = stackBreadcrumbSegments(path);
  if (segments.length <= 1) {
    return segments[0]?.path ?? '';
  }
  return segments[segments.length - 2].path;
}

export function applyStackEntries(
  current: StackPopupViewState,
  folderPath: string,
  entries: StackEntry[],
  statusMessage?: string
): StackPopupViewState {
  if (folderPath !== current.currentPath) {
    return current;
  }

  const sortedEntries = sortStackEntries(entries, current.sortColumn, current.sortDirection);
  const visiblePaths = new Set(sortedEntries.map((entry) => entry.path));
  const selectedPaths = (current.selectedPaths.length ? current.selectedPaths : [current.selectedPath])
    .filter((path): path is string => Boolean(path && visiblePaths.has(path)));
  const selectedPath = selectedPaths.includes(current.selectedPath ?? '')
    ? current.selectedPath
    : sortedEntries.some((entry) => entry.path === current.selectedPath)
      ? current.selectedPath
      : (selectedPaths[0] ?? null);

  return {
    ...current,
    entriesPath: folderPath,
    entries: sortedEntries,
    selectedPath,
    selectedPaths,
    selectionAnchorPath: current.selectionAnchorPath && visiblePaths.has(current.selectionAnchorPath)
      ? current.selectionAnchorPath
      : selectedPath,
    statusMessage: statusMessage ?? (entries.length ? `${entries.length} items` : 'This folder is empty')
  };
}

export function stackPopupHasRetainedRows(current: StackPopupViewState) {
  return current.entries.length > 0
    && Boolean(current.currentPath)
    && current.entriesPath !== current.currentPath;
}

export function stackListingStatus(listing: StackFolderListing) {
  const itemStatus = listing.entries.length
    ? `${listing.entries.length} of ${listing.total} items`
    : 'This folder is empty';
  if (!listing.warnings.length) {
    return itemStatus;
  }

  return `${itemStatus} - partial listing: ${listing.warnings.length} warning${listing.warnings.length === 1 ? '' : 's'}`;
}

export function stackIconHydrationStatus(resolvedCount: number, totalCount: number) {
  const resolved = Math.max(0, Math.floor(resolvedCount));
  const total = Math.max(0, Math.floor(totalCount));
  if (!total || resolved >= total) {
    return '';
  }
  return `Loading icons ${resolved} of ${total}`;
}

export function applyStackFolderListing(
  current: StackPopupViewState,
  folderPath: string,
  listing: StackFolderListing
): StackPopupViewState {
  return applyStackEntries(current, folderPath, listing.entries, stackListingStatus(listing));
}

export function commitValidatedStackFolderListing(
  current: StackPopupViewState,
  folderPath: string,
  listing: StackFolderListing
): StackPopupViewState {
  const opened = openStackFolder(current, folderPath);
  return applyStackFolderListing(opened, opened.currentPath, listing);
}

export function mergeStackFolderListings(
  current: StackFolderListing | null,
  page: StackFolderListingPage
): StackFolderListing {
  if (!current || current.path !== page.path || page.offset <= 0) {
    return {
      path: page.path,
      entries: [...page.entries],
      total: page.total,
      warnings: [...page.warnings]
    };
  }

  return {
    path: page.path,
    entries: [...current.entries, ...page.entries],
    total: page.total,
    warnings: [...current.warnings, ...page.warnings]
  };
}

export function selectStackEntry(
  current: StackPopupViewState,
  entryPath: string | null,
  mode: StackSelectionMode = 'single'
): StackPopupViewState {
  if (!entryPath) {
    return clearStackSelection(current);
  }

  const visiblePaths = current.entries.map((entry) => entry.path);
  if (!visiblePaths.includes(entryPath)) {
    return current;
  }

  if (mode === 'range') {
    const anchorPath = current.selectionAnchorPath && visiblePaths.includes(current.selectionAnchorPath)
      ? current.selectionAnchorPath
      : current.selectedPath;
    const anchorIndex = anchorPath ? visiblePaths.indexOf(anchorPath) : -1;
    const entryIndex = visiblePaths.indexOf(entryPath);
    if (anchorIndex >= 0 && entryIndex >= 0) {
      const start = Math.min(anchorIndex, entryIndex);
      const end = Math.max(anchorIndex, entryIndex);
      return {
        ...current,
        selectedPath: entryPath,
        selectedPaths: visiblePaths.slice(start, end + 1),
        selectionAnchorPath: anchorPath
      };
    }
  }

  if (mode === 'toggle') {
    const selectedPaths = current.selectedPaths.includes(entryPath)
      ? current.selectedPaths.filter((path) => path !== entryPath)
      : [...current.selectedPaths, entryPath];
    return {
      ...current,
      selectedPath: selectedPaths.includes(entryPath) ? entryPath : (selectedPaths.at(-1) ?? null),
      selectedPaths,
      selectionAnchorPath: entryPath
    };
  }

  return {
    ...current,
    selectedPath: entryPath,
    selectedPaths: [entryPath],
    selectionAnchorPath: entryPath
  };
}

export function updateStackSort(
  current: StackPopupViewState,
  column: StackSortColumn
): StackPopupViewState {
  const sortDirection = current.sortColumn === column && current.sortDirection === 'asc' ? 'desc' : 'asc';
  return {
    ...current,
    sortColumn: column,
    sortDirection,
    entries: sortStackEntries(current.entries, column, sortDirection)
  };
}

export function applyStackEntryIconUpdates(
  current: StackPopupViewState,
  folderPath: string,
  updates: StackEntryIconUpdate[]
): StackPopupViewState {
  if (folderPath !== current.currentPath || current.entriesPath !== folderPath || !updates.length) {
    return current;
  }

  const updatesByPath = new Map(
    updates
      .filter((update) => Boolean(update.path))
      .map((update) => [update.path, update.iconDataUrl])
  );
  if (!updatesByPath.size) {
    return current;
  }

  let changed = false;
  const entries = current.entries.map((entry) => {
    if (!updatesByPath.has(entry.path)) {
      return entry;
    }
    const iconDataUrl = updatesByPath.get(entry.path) ?? null;
    if (entry.iconDataUrl === iconDataUrl) {
      return entry;
    }
    changed = true;
    return {
      ...entry,
      iconDataUrl
    };
  });

  if (!changed) {
    return current;
  }
  return {
    ...current,
    entries
  };
}

export function stackSortHeaderState(
  current: StackPopupViewState,
  column: StackSortColumn
): StackSortHeaderState {
  const active = current.sortColumn === column;
  if (!active) {
    return {
      active,
      ariaSort: 'none',
      className: 'details-sort',
      indicator: ''
    };
  }

  return {
    active,
    ariaSort: current.sortDirection === 'asc' ? 'ascending' : 'descending',
    className: `details-sort active ${current.sortDirection}`,
    indicator: current.sortDirection === 'asc' ? '↑' : '↓'
  };
}

export function stackOpenWithSuggestions(entry: StackEntry | null | undefined): StackOpenWithSuggestion[] {
  if (!entry || entry.entryType !== 'File') {
    return [];
  }

  const extension = stackEntryExtension(entry.name);
  const type = entry.typeLabel.toLocaleLowerCase();
  const ids = new Set<string>();
  const suggestions: StackOpenWithSuggestion[] = [];
  const add = (id: string, label: string) => {
    if (!ids.has(id)) {
      ids.add(id);
      suggestions.push({ id, label, commandContract: 'open_stack_item_with_app' });
    }
  };

  if (TEXT_OPEN_WITH_EXTENSIONS.has(extension) || type.includes('text') || type.includes('code') || type.includes('json')) {
    add('notepad', 'Notepad');
    add('notepad-plus-plus', 'Notepad++');
    add('vscode', 'Visual Studio Code');
  } else if (IMAGE_OPEN_WITH_EXTENSIONS.has(extension) || type.includes('image')) {
    add('paint', 'Paint');
    add('vscode', 'Visual Studio Code');
  } else if (ARCHIVE_OPEN_WITH_EXTENSIONS.has(extension) || type.includes('archive')) {
    add('vscode', 'Visual Studio Code');
  } else {
    add('vscode', 'Visual Studio Code');
    add('notepad', 'Notepad');
  }

  return suggestions;
}

const TEXT_OPEN_WITH_EXTENSIONS = new Set([
  'txt', 'md', 'markdown', 'log', 'json', 'jsonc', 'yaml', 'yml', 'xml', 'toml',
  'ini', 'csv', 'ts', 'tsx', 'js', 'jsx', 'mjs', 'cjs', 'svelte', 'rs', 'py',
  'ps1', 'bat', 'cmd', 'sh', 'html', 'css', 'scss', 'sql'
]);

const IMAGE_OPEN_WITH_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg', 'ico']);
const ARCHIVE_OPEN_WITH_EXTENSIONS = new Set(['zip', '7z', 'rar', 'tar', 'gz']);

function stackEntryExtension(name: string) {
  const leaf = name.split(/[\\/]/).filter(Boolean).at(-1) ?? name;
  const index = leaf.lastIndexOf('.');
  return index > 0 && index < leaf.length - 1 ? leaf.slice(index + 1).toLocaleLowerCase() : '';
}

export function sortStackEntries(
  entries: StackEntry[],
  column: StackSortColumn,
  direction: StackSortDirection
): StackEntry[] {
  const factor = direction === 'asc' ? 1 : -1;
  return [...entries].sort((a, b) => {
    const folderOrder = column === 'modified' ? 0 : folderRank(a) - folderRank(b);
    if (folderOrder !== 0) {
      return folderOrder;
    }
    return factor * compareStackEntries(a, b, column);
  });
}

function folderRank(entry: StackEntry) {
  return entry.entryType === 'Folder' ? 0 : 1;
}

function compareStackEntries(a: StackEntry, b: StackEntry, column: StackSortColumn) {
  const nameCompare = compareStrings(a.name, b.name);
  if (column === 'type') {
    return compareStrings(a.typeLabel, b.typeLabel) || nameCompare;
  }
  if (column === 'size') {
    return compareNullableNumbers(a.size, b.size) || nameCompare;
  }
  if (column === 'modified') {
    return compareNullableNumbers(a.modifiedMs, b.modifiedMs) || nameCompare;
  }
  return nameCompare;
}

function compareStrings(a: string, b: string) {
  return a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' });
}

function compareNullableNumbers(a: number | null | undefined, b: number | null | undefined) {
  if (a === b) {
    return 0;
  }
  if (a === null || a === undefined) {
    return 1;
  }
  if (b === null || b === undefined) {
    return -1;
  }
  return a - b;
}

export function clearStackSelection(current: StackPopupViewState): StackPopupViewState {
  return {
    ...current,
    selectedPath: null,
    selectedPaths: [],
    selectionAnchorPath: null
  };
}

export function selectAllStackEntries(current: StackPopupViewState): StackPopupViewState {
  const selectedPaths = current.entries.map((entry) => entry.path);
  return {
    ...current,
    selectedPath: selectedPaths[0] ?? null,
    selectedPaths,
    selectionAnchorPath: selectedPaths[0] ?? null
  };
}

export function selectStackEntryPaths(
  current: StackPopupViewState,
  paths: readonly string[]
): StackPopupViewState {
  const visiblePaths = new Set(current.entries.map((entry) => entry.path));
  const selectedPaths = paths.filter((path, index) => visiblePaths.has(path) && paths.indexOf(path) === index);
  return {
    ...current,
    selectedPath: selectedPaths.at(-1) ?? null,
    selectedPaths,
    selectionAnchorPath: selectedPaths[0] ?? null
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
    selectedPath: null,
    selectedPaths: [],
    selectionAnchorPath: null,
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

export function selectedStackPaths(state: StackPopupViewState) {
  if (state.selectedPaths.length) {
    return state.selectedPaths;
  }
  return state.selectedPath ? [state.selectedPath] : [];
}

export function findTypeToSelectPath(
  entries: StackEntry[],
  prefix: string,
  selectedPath: string | null = null
): string | null {
  const needle = prefix.trim().toLocaleLowerCase();
  if (!needle || !entries.length) {
    return null;
  }

  const startIndex = Math.max(0, entries.findIndex((entry) => entry.path === selectedPath) + 1);
  const ordered = [...entries.slice(startIndex), ...entries.slice(0, startIndex)];
  return ordered.find((entry) => entry.name.toLocaleLowerCase().startsWith(needle))?.path ?? null;
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
