import type { StackEntry } from './stackPopup';
import type { StackBreadcrumbSegment } from './stackPopupState';

export const STACK_BROWSER_ROW_HEIGHT_PX = 30;
export const STACK_BROWSER_VIRTUAL_OVERSCAN_ROWS = 8;
export const STACK_BROWSER_VIRTUAL_MIN_ROWS = 160;
export const STACK_BROWSER_BACKGROUND_CONTEXT_MENU_IGNORE_SELECTORS = [
  '[role="row"]',
  '.context-menu',
  '.delete-confirm-dialog',
  '.inline-editor',
  '.stack-toolbar',
  '.stack-resize-grip'
] as const;

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

export type StackBrowserMarqueePoint = {
  x: number;
  y: number;
};

export type StackBrowserMarqueeRect = {
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
};

export type StackBrowserMarqueeRowBounds = {
  path: string;
  rect: Pick<StackBrowserMarqueeRect, 'left' | 'top' | 'right' | 'bottom'>;
};

export type StackBrowserMarqueeVirtualOptions = {
  rowHeight?: number;
  rowLeft: number;
  rowRight: number;
  viewportTop: number;
  scrollTop: number;
  existingSelection?: readonly string[];
  additive?: boolean;
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

export function stackBrowserMarqueeRect(
  start: StackBrowserMarqueePoint,
  current: StackBrowserMarqueePoint
): StackBrowserMarqueeRect {
  const left = Math.min(start.x, current.x);
  const top = Math.min(start.y, current.y);
  const right = Math.max(start.x, current.x);
  const bottom = Math.max(start.y, current.y);
  return {
    left,
    top,
    right,
    bottom,
    width: right - left,
    height: bottom - top
  };
}

export function stackBrowserMarqueeSelectedPaths(
  rows: readonly StackBrowserMarqueeRowBounds[],
  marquee: StackBrowserMarqueeRect,
  existingSelection: readonly string[] = [],
  additive = false
): string[] {
  const selected = rows
    .filter((row) => rectsIntersect(row.rect, marquee))
    .map((row) => row.path);

  if (!additive) {
    return selected;
  }

  const merged = [...existingSelection];
  const seen = new Set(merged);
  for (const path of selected) {
    if (!seen.has(path)) {
      seen.add(path);
      merged.push(path);
    }
  }
  return merged;
}

export function stackBrowserMarqueeSelectedVirtualPaths(
  paths: readonly string[],
  marquee: StackBrowserMarqueeRect,
  options: StackBrowserMarqueeVirtualOptions
): string[] {
  const rowHeight = positiveNumber(options.rowHeight, STACK_BROWSER_ROW_HEIGHT_PX);
  const scrollTop = Math.max(0, Number.isFinite(options.scrollTop) ? options.scrollTop : 0);
  const viewportTop = Number.isFinite(options.viewportTop) ? options.viewportTop : 0;
  const rowLeft = Math.min(options.rowLeft, options.rowRight);
  const rowRight = Math.max(options.rowLeft, options.rowRight);
  const selected = paths.filter((path, index) => {
    if (!path) {
      return false;
    }
    const rowTop = viewportTop + index * rowHeight - scrollTop;
    return rectsIntersect(
      {
        left: rowLeft,
        top: rowTop,
        right: rowRight,
        bottom: rowTop + rowHeight
      },
      marquee
    );
  });

  if (!options.additive) {
    return selected;
  }

  return uniquePaths([...(options.existingSelection ?? []), ...selected]);
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

export function stackBrowserSearchEntries(entries: StackEntry[], query: string): StackEntry[] {
  const normalizedQuery = query.trim().toLocaleLowerCase();
  if (!normalizedQuery) {
    return entries;
  }

  return entries.filter((entry) =>
    entry.name.toLocaleLowerCase().includes(normalizedQuery)
    || entry.path.toLocaleLowerCase().includes(normalizedQuery)
  );
}

export type StackBrowserTemplateAction = {
  id: string;
  label: string;
  fileName: string;
  kind: 'file' | 'folder';
};

export type StackBrowserGitOperation = 'diff' | 'stage' | 'restore';

export type StackBrowserContextActionKind =
  | 'open-editor'
  | 'open-terminal'
  | 'copy-path'
  | 'copy-directory-path'
  | 'copy-name'
  | 'create-from-template'
  | 'git-operation';

export type StackBrowserContextActionPlan = {
  id: string;
  kind: StackBrowserContextActionKind;
  label: string;
  targetPath: string;
  workingDirectory?: string;
  clipboardText?: string;
  templateId?: string;
  templateKind?: StackBrowserTemplateAction['kind'];
  plannedPath?: string;
  gitOperation?: StackBrowserGitOperation;
  destructive: boolean;
  requiresConfirmation: boolean;
};

export type StackBrowserGitContext = {
  repositoryRoot?: string | null;
  changedPaths?: readonly string[];
};

export type StackBrowserContextActionInput = {
  currentFolderPath: string;
  entry?: StackEntry | null;
  templates?: readonly StackBrowserTemplateAction[];
  git?: StackBrowserGitContext | null;
};

export function stackBrowserContextActionPlans(
  input: StackBrowserContextActionInput
): StackBrowserContextActionPlan[] {
  const entry = input.entry ?? null;
  const targetPath = entry?.path ?? input.currentFolderPath;
  const targetName = entry?.name ?? basename(targetPath);
  const workingDirectory = entry?.entryType === 'File' ? parentPath(targetPath) : targetPath;
  const plans: StackBrowserContextActionPlan[] = [
    {
      id: 'open-editor',
      kind: 'open-editor',
      label: `Open ${entry ? targetName : 'folder'} in editor`,
      targetPath,
      workingDirectory,
      destructive: false,
      requiresConfirmation: false
    },
    {
      id: 'open-terminal',
      kind: 'open-terminal',
      label: 'Open terminal here',
      targetPath: workingDirectory,
      workingDirectory,
      destructive: false,
      requiresConfirmation: false
    },
    {
      id: 'copy-path',
      kind: 'copy-path',
      label: 'Copy path',
      targetPath,
      clipboardText: targetPath,
      destructive: false,
      requiresConfirmation: false
    }
  ];

  if (entry) {
    plans.push(
      {
        id: 'copy-directory-path',
        kind: 'copy-directory-path',
        label: 'Copy containing folder path',
        targetPath,
        clipboardText: workingDirectory,
        destructive: false,
        requiresConfirmation: false
      },
      {
        id: 'copy-name',
        kind: 'copy-name',
        label: 'Copy name',
        targetPath,
        clipboardText: targetName,
        destructive: false,
        requiresConfirmation: false
      }
    );
  }

  for (const template of input.templates ?? []) {
    plans.push({
      id: `template:${template.id}`,
      kind: 'create-from-template',
      label: template.label,
      targetPath: workingDirectory,
      workingDirectory,
      templateId: template.id,
      templateKind: template.kind,
      plannedPath: joinPath(workingDirectory, template.fileName),
      destructive: false,
      requiresConfirmation: false
    });
  }

  plans.push(...stackBrowserGitActionPlans(targetPath, workingDirectory, input.git ?? null));

  return plans;
}

function stackBrowserGitActionPlans(
  targetPath: string,
  workingDirectory: string,
  git: StackBrowserGitContext | null
): StackBrowserContextActionPlan[] {
  if (!git?.repositoryRoot || !isWithinPath(targetPath, git.repositoryRoot)) {
    return [];
  }

  const changed = (git.changedPaths ?? []).some((path) => samePath(path, targetPath));
  const plans: StackBrowserContextActionPlan[] = [
    {
      id: 'git:diff',
      kind: 'git-operation',
      label: 'Show git diff plan',
      targetPath,
      workingDirectory,
      gitOperation: 'diff',
      destructive: false,
      requiresConfirmation: false
    }
  ];

  if (changed) {
    plans.push(
      {
        id: 'git:stage',
        kind: 'git-operation',
        label: 'Stage file plan',
        targetPath,
        workingDirectory,
        gitOperation: 'stage',
        destructive: false,
        requiresConfirmation: false
      },
      {
        id: 'git:restore',
        kind: 'git-operation',
        label: 'Restore file plan',
        targetPath,
        workingDirectory,
        gitOperation: 'restore',
        destructive: true,
        requiresConfirmation: true
      }
    );
  }

  return plans;
}

function positiveNumber(value: number | undefined, fallback: number) {
  return value !== undefined && Number.isFinite(value) && value > 0 ? value : fallback;
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

function uniquePaths(paths: readonly string[]) {
  const seen = new Set<string>();
  const unique: string[] = [];
  for (const path of paths) {
    if (!seen.has(path)) {
      seen.add(path);
      unique.push(path);
    }
  }
  return unique;
}

function rectsIntersect(
  left: Pick<StackBrowserMarqueeRect, 'left' | 'top' | 'right' | 'bottom'>,
  right: Pick<StackBrowserMarqueeRect, 'left' | 'top' | 'right' | 'bottom'>
) {
  return left.right >= right.left
    && left.left <= right.right
    && left.bottom >= right.top
    && left.top <= right.bottom;
}

function basename(path: string): string {
  const normalized = path.replace(/\//g, '\\').replace(/\\+$/g, '');
  return normalized.split('\\').filter(Boolean).at(-1) ?? normalized;
}

function parentPath(path: string): string {
  const normalized = path.replace(/\//g, '\\').replace(/\\+$/g, '');
  const index = normalized.lastIndexOf('\\');
  return index > 0 ? normalized.slice(0, index) : normalized;
}

function joinPath(parent: string, child: string): string {
  return `${parent.replace(/[\\/]+$/g, '')}\\${child.replace(/^[\\/]+/g, '')}`;
}

function samePath(left: string, right: string): boolean {
  return normalizePath(left) === normalizePath(right);
}

function isWithinPath(path: string, root: string): boolean {
  const normalizedPath = normalizePath(path);
  const normalizedRoot = normalizePath(root);
  return normalizedPath === normalizedRoot || normalizedPath.startsWith(`${normalizedRoot}\\`);
}

function normalizePath(path: string): string {
  return path.replace(/\//g, '\\').replace(/\\+$/g, '').toLocaleLowerCase();
}
