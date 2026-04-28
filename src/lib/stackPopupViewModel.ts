import type { StackEntry } from './stackPopup';
import type { StackBreadcrumbSegment } from './stackPopupState';

export const STACK_BROWSER_ROW_HEIGHT_PX = 30;
export const STACK_BROWSER_VIRTUAL_OVERSCAN_ROWS = 8;
export const STACK_BROWSER_VIRTUAL_MIN_ROWS = 160;

export const STACK_BROWSER_FRONTEND_EVENTS = {
  folderWatchRefreshRequested: 'stack-browser:folder-watch-refresh-requested',
  folderRowsWindowChanged: 'stack-browser:rows-window-changed'
} as const;

export type StackBrowserVirtualOptions = {
  rowHeight?: number;
  overscan?: number;
  minRows?: number;
};

export type StackBrowserVirtualRow<T> = {
  item: T;
  index: number;
};

export type StackBrowserVirtualWindow<T> = {
  enabled: boolean;
  rows: Array<StackBrowserVirtualRow<T>>;
  startIndex: number;
  endIndex: number;
  beforeHeight: number;
  afterHeight: number;
  rowHeight: number;
  totalHeight: number;
};

export function stackBrowserVirtualWindow<T>(
  items: T[],
  scrollTop: number,
  viewportHeight: number,
  options: StackBrowserVirtualOptions = {}
): StackBrowserVirtualWindow<T> {
  const rowHeight = positiveNumber(options.rowHeight, STACK_BROWSER_ROW_HEIGHT_PX);
  const overscan = Math.max(0, Math.floor(positiveNumber(options.overscan, STACK_BROWSER_VIRTUAL_OVERSCAN_ROWS)));
  const minRows = Math.max(0, Math.floor(positiveNumber(options.minRows, STACK_BROWSER_VIRTUAL_MIN_ROWS)));
  const normalizedScrollTop = Math.max(0, Number.isFinite(scrollTop) ? scrollTop : 0);
  const normalizedViewportHeight = Math.max(0, Number.isFinite(viewportHeight) ? viewportHeight : 0);
  const totalHeight = items.length * rowHeight;

  if (items.length <= minRows || normalizedViewportHeight <= 0) {
    return {
      enabled: false,
      rows: items.map((item, index) => ({ item, index })),
      startIndex: 0,
      endIndex: items.length,
      beforeHeight: 0,
      afterHeight: 0,
      rowHeight,
      totalHeight
    };
  }

  const firstVisibleIndex = Math.floor(normalizedScrollTop / rowHeight);
  const visibleRowCount = Math.ceil(normalizedViewportHeight / rowHeight);
  const startIndex = clamp(firstVisibleIndex - overscan, 0, items.length);
  const endIndex = clamp(firstVisibleIndex + visibleRowCount + overscan, startIndex, items.length);

  return {
    enabled: true,
    rows: items.slice(startIndex, endIndex).map((item, offset) => ({ item, index: startIndex + offset })),
    startIndex,
    endIndex,
    beforeHeight: startIndex * rowHeight,
    afterHeight: Math.max(0, (items.length - endIndex) * rowHeight),
    rowHeight,
    totalHeight
  };
}

export function stackBrowserScrollTopForIndex(
  index: number,
  currentScrollTop: number,
  viewportHeight: number,
  totalRows: number,
  options: StackBrowserVirtualOptions = {}
): number {
  const rowHeight = positiveNumber(options.rowHeight, STACK_BROWSER_ROW_HEIGHT_PX);
  const normalizedIndex = clamp(Math.floor(index), 0, Math.max(0, totalRows - 1));
  const normalizedScrollTop = Math.max(0, Number.isFinite(currentScrollTop) ? currentScrollTop : 0);
  const normalizedViewportHeight = Math.max(rowHeight, Number.isFinite(viewportHeight) ? viewportHeight : rowHeight);
  const visibleStart = Math.floor(normalizedScrollTop / rowHeight);
  const visibleCount = Math.max(1, Math.floor(normalizedViewportHeight / rowHeight));
  const visibleEnd = visibleStart + visibleCount - 1;

  if (normalizedIndex < visibleStart) {
    return normalizedIndex * rowHeight;
  }
  if (normalizedIndex > visibleEnd) {
    return Math.max(0, (normalizedIndex + 1) * rowHeight - normalizedViewportHeight);
  }
  return normalizedScrollTop;
}

export type StackBrowserBreadcrumbOverflow = {
  visibleSegments: StackBreadcrumbSegment[];
  hiddenSegments: StackBreadcrumbSegment[];
  hiddenCount: number;
  hiddenTitle: string;
};

export function stackBrowserBreadcrumbOverflow(
  segments: StackBreadcrumbSegment[],
  maxVisible = 5
): StackBrowserBreadcrumbOverflow {
  const visibleLimit = Math.max(2, Math.floor(maxVisible));
  if (segments.length <= visibleLimit) {
    return {
      visibleSegments: [...segments],
      hiddenSegments: [],
      hiddenCount: 0,
      hiddenTitle: ''
    };
  }

  const tailCount = visibleLimit - 1;
  const hiddenSegments = segments.slice(1, -tailCount);
  const visibleSegments = [segments[0], ...segments.slice(-tailCount)];
  return {
    visibleSegments,
    hiddenSegments,
    hiddenCount: hiddenSegments.length,
    hiddenTitle: hiddenSegments.map((segment) => segment.name).join(' / ')
  };
}

export type StackBrowserDeletePrompt = {
  canDelete: boolean;
  paths: string[];
  itemCount: number;
  label: string;
  title: string;
  message: string;
};

export function stackBrowserDeletePrompt(
  entries: StackEntry[],
  selectedPaths: string[],
  selectedPath: string | null
): StackBrowserDeletePrompt {
  const requestedPaths = selectedPaths.length ? selectedPaths : (selectedPath ? [selectedPath] : []);
  const entriesByPath = new Map(entries.map((entry) => [entry.path, entry]));
  const paths = requestedPaths.filter((path) => entriesByPath.has(path));
  const selectedEntries = paths.map((path) => entriesByPath.get(path)).filter(Boolean) as StackEntry[];
  const itemCount = selectedEntries.length;
  const label = itemCount === 1 ? selectedEntries[0].name : `${itemCount} items`;

  return {
    canDelete: itemCount > 0,
    paths,
    itemCount,
    label,
    title: itemCount === 1 ? `Delete ${selectedEntries[0]?.name ?? 'item'}?` : `Delete ${itemCount} items?`,
    message: itemCount === 1
      ? `Delete "${selectedEntries[0]?.name ?? 'item'}"? This cannot be undone.`
      : `Delete ${itemCount} selected items? This cannot be undone.`
  };
}

function positiveNumber(value: number | undefined, fallback: number) {
  return value !== undefined && Number.isFinite(value) && value > 0 ? value : fallback;
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}
