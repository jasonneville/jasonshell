import type { SearchPanelResult } from '../../lib/searchPanel';
import type { SearchMode } from '../../lib/searchSettings';

export type SearchResultGroupId = 'everything' | 'apps' | 'windows' | 'places' | 'files' | 'commands';

export type SearchVisibleGroupId =
  | 'bestMatch'
  | 'apps'
  | 'folders'
  | 'files'
  | 'settings'
  | 'windows'
  | 'commands';

export type GroupedSearchResult = {
  result: SearchPanelResult;
  index: number;
};

export type SearchResultGroup = {
  id: SearchResultGroupId;
  label: string;
  items: GroupedSearchResult[];
};

export type SearchVisibleRow = {
  id: string;
  rowKey: string;
  domId: string;
  result: SearchPanelResult;
  resultIndex: number;
  visibleIndex: number;
  groupId: SearchVisibleGroupId;
  groupLabel: string;
  showGroupLabel: boolean;
};

export type SearchResultActionHints = {
  primary: string;
  secondary: string | null;
};

export type SearchResultRefreshRequest = {
  query: string;
  sequence: number;
};

export type SearchEngineQueryRequestState = {
  query: string;
  sequence: number;
};

export type LatestSearchQueryController = {
  next(query: string): SearchEngineQueryRequestState;
  currentSequence(): number;
  shouldApply(request: SearchEngineQueryRequestState, currentQuery: string): boolean;
};

export type SearchKeyboardAction =
  | 'none'
  | 'openTopRight'
  | 'openCentered'
  | 'close'
  | 'activate'
  | 'selectPrevious'
  | 'selectNext';

const GROUP_ORDER: SearchResultGroupId[] = ['everything', 'apps', 'windows', 'places', 'files', 'commands'];
const VISIBLE_GROUP_ORDER: SearchVisibleGroupId[] = [
  'apps',
  'folders',
  'files',
  'settings',
  'windows',
  'commands'
];

export function groupSearchResults(results: readonly SearchPanelResult[]): SearchResultGroup[] {
  const groups = new Map<SearchResultGroupId, SearchResultGroup>();

  results.forEach((result, index) => {
    const id = searchResultGroupId(result);
    const group = groups.get(id) ?? {
      id,
      label: searchResultGroupLabel(id),
      items: []
    };
    group.items.push({ result, index });
    groups.set(id, group);
  });

  return GROUP_ORDER.map((id) => groups.get(id)).filter(
    (group): group is SearchResultGroup => Boolean(group)
  );
}

export function buildVisibleSearchRows(results: readonly SearchPanelResult[]): SearchVisibleRow[] {
  const visibleRows: SearchVisibleRow[] = [];
  const bestMatchCount = Math.min(results.length, 3);
  const remainingGroups = new Map<SearchVisibleGroupId, GroupedSearchResult[]>();

  results.slice(0, bestMatchCount).forEach((result, index) => {
    visibleRows.push({
      id: result.id,
      rowKey: visibleRowKey(result.id, index),
      domId: visibleRowDomId(index),
      result,
      resultIndex: index,
      visibleIndex: visibleRows.length,
      groupId: 'bestMatch',
      groupLabel: visibleGroupLabel('bestMatch'),
      showGroupLabel: index === 0
    });
  });

  results.slice(bestMatchCount).forEach((result, offset) => {
    const resultIndex = bestMatchCount + offset;
    const groupId = searchVisibleGroupId(result);
    const items = remainingGroups.get(groupId) ?? [];
    items.push({ result, index: resultIndex });
    remainingGroups.set(groupId, items);
  });

  for (const groupId of VISIBLE_GROUP_ORDER) {
    const items = remainingGroups.get(groupId);
    if (!items?.length) {
      continue;
    }
    items.forEach((item, itemIndex) => {
      visibleRows.push({
        id: item.result.id,
        rowKey: visibleRowKey(item.result.id, item.index),
        domId: visibleRowDomId(item.index),
        result: item.result,
        resultIndex: item.index,
        visibleIndex: visibleRows.length,
        groupId,
        groupLabel: visibleGroupLabel(groupId),
        showGroupLabel: itemIndex === 0
      });
    });
  }

  return visibleRows;
}

export function selectedVisibleRowIndex(
  visibleRows: readonly SearchVisibleRow[],
  selectedIndex: number
): number {
  return visibleRows.findIndex((row) => row.resultIndex === selectedIndex);
}

export function nextVisibleRowIndex(
  visibleRows: readonly SearchVisibleRow[],
  currentVisibleIndex: number,
  direction: -1 | 1
): number {
  if (!visibleRows.length) {
    return -1;
  }
  if (currentVisibleIndex < 0) {
    return direction > 0 ? 0 : visibleRows.length - 1;
  }
  return Math.max(0, Math.min(visibleRows.length - 1, currentVisibleIndex + direction));
}

export function searchResultActionHints(result: SearchPanelResult): SearchResultActionHints {
  if (result.kind === 'app') {
    return { primary: 'Launch', secondary: null };
  }
  if (result.kind === 'window') {
    return { primary: 'Focus', secondary: null };
  }
  if (result.kind === 'folder') {
    return { primary: 'Open', secondary: result.path ? 'Pin' : null };
  }
  if (result.kind === 'file') {
    return { primary: 'Open', secondary: null };
  }
  if (result.kind === 'setting') {
    return { primary: 'Open', secondary: null };
  }
  if (result.kind === 'calculator') {
    return { primary: 'Copy', secondary: null };
  }
  if (result.kind === 'web' || result.kind === 'bookmark') {
    return { primary: 'Open', secondary: null };
  }
  return { primary: 'Run', secondary: null };
}

export function searchResultGroupId(result: SearchPanelResult): SearchResultGroupId {
  if (result.kind === 'app') {
    return 'apps';
  }
  if (result.kind === 'window') {
    return 'windows';
  }
  if (
    result.kind === 'command' ||
    result.kind === 'setting' ||
    result.kind === 'calculator' ||
    result.kind === 'web' ||
    result.kind === 'bookmark'
  ) {
    return 'commands';
  }
  if (result.kind === 'folder') {
    return 'places';
  }
  return 'files';
}

export function nextSearchPanelFallbackDelay(attempt: number): number | null {
  const delays = [120, 240, 500, 1_000, 2_000];
  return delays[attempt] ?? 2_000;
}

export function shouldContinueSearchPanelFallbackPolling(
  attempt: number,
  receivedPayload: boolean,
  hasVisiblePayload: boolean
): boolean {
  return !receivedPayload || hasVisiblePayload;
}

export function nextSearchResultRefreshRequest(
  currentSequence: number,
  query: string
): SearchResultRefreshRequest {
  return {
    query,
    sequence: currentSequence + 1
  };
}

export function shouldApplySearchResultRefresh(
  request: SearchResultRefreshRequest,
  currentQuery: string,
  currentSequence: number
): boolean {
  return request.sequence === currentSequence && request.query === currentQuery;
}

export function nextSearchEngineQueryRequest(
  currentSequence: number,
  query: string
): SearchEngineQueryRequestState {
  return {
    query: query.trim(),
    sequence: currentSequence + 1
  };
}

export function shouldApplySearchEngineResponse(
  request: SearchEngineQueryRequestState,
  currentQuery: string,
  currentSequence: number
): boolean {
  return request.sequence === currentSequence && request.query === currentQuery.trim();
}

export function createLatestSearchQueryController(initialSequence = 0): LatestSearchQueryController {
  let sequence = initialSequence;
  return {
    next(query: string) {
      const request = nextSearchEngineQueryRequest(sequence, query);
      sequence = request.sequence;
      return request;
    },
    currentSequence() {
      return sequence;
    },
    shouldApply(request: SearchEngineQueryRequestState, currentQuery: string) {
      return shouldApplySearchEngineResponse(request, currentQuery, sequence);
    }
  };
}

export function searchModeFromSettings(value: unknown): SearchMode {
  return value === 'topRight' ? 'topRight' : 'centeredHotkey';
}

export function configuredSearchOpenAction(mode: SearchMode): SearchKeyboardAction {
  return mode === 'centeredHotkey' ? 'openCentered' : 'openTopRight';
}

export function searchPanelKeyboardAction(key: string): SearchKeyboardAction {
  switch (key) {
    case 'ArrowDown':
      return 'selectNext';
    case 'ArrowUp':
      return 'selectPrevious';
    case 'Enter':
      return 'activate';
    case 'Escape':
      return 'close';
    default:
      return 'none';
  }
}

function searchResultGroupLabel(id: SearchResultGroupId): string {
  switch (id) {
    case 'apps':
      return 'Apps and Programs';
    case 'windows':
      return 'Open Windows';
    case 'places':
      return 'Places';
    case 'files':
      return 'Files';
    case 'commands':
      return 'Commands';
    case 'everything':
      return 'Everything';
  }
}

function searchVisibleGroupId(result: SearchPanelResult): SearchVisibleGroupId {
  if (result.kind === 'app') {
    return 'apps';
  }
  if (result.kind === 'folder') {
    return 'folders';
  }
  if (result.kind === 'file') {
    return 'files';
  }
  if (result.kind === 'setting') {
    return 'settings';
  }
  if (result.kind === 'window') {
    return 'windows';
  }
  return 'commands';
}

function visibleGroupLabel(id: SearchVisibleGroupId): string {
  switch (id) {
    case 'bestMatch':
      return 'Best match';
    case 'apps':
      return 'Apps';
    case 'folders':
      return 'Folders';
    case 'files':
      return 'Files';
    case 'settings':
      return 'Settings';
    case 'windows':
      return 'Windows';
    case 'commands':
      return 'Commands';
  }
}

function visibleRowKey(resultId: string, resultIndex: number): string {
  return `${resultIndex}:${resultId}`;
}

function visibleRowDomId(resultIndex: number): string {
  return `search-result-${resultIndex}`;
}
