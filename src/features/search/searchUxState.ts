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
export type SearchExpandableGroupId = Exclude<SearchVisibleGroupId, 'bestMatch'>;

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

export type SearchVisibleRowIdentity = {
  id: string;
  rowKey?: string;
  recordKey?: string;
  resultIndex?: number;
};
export type SearchVisibleGroupOverflow = {
  groupId: SearchExpandableGroupId;
  groupLabel: string;
  totalCount: number;
  visibleCount: number;
  hiddenCount: number;
};
export type SearchVisibleRowsBuildOptions = {
  expandedGroups?: ReadonlySet<SearchExpandableGroupId>;
  perGroupLimit?: number;
};

export type SearchProgressiveResultSet = {
  query: string;
  results: SearchPanelResult[];
};

export type SearchProgressiveResultUpdate = {
  query: string;
  phase?: 'typing' | 'local' | 'provider' | 'complete' | 'error';
  results: SearchPanelResult[];
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

export type SearchProviderCacheRetryTiming = {
  providerId?: string;
  cache?: string;
  resultCount?: number;
};

export type LatestSearchQueryController = {
  next(query: string): SearchEngineQueryRequestState;
  currentSequence(): number;
  shouldApply(request: SearchEngineQueryRequestState, currentQuery: string): boolean;
};

export type LatestSearchExecutionQueue<TRequest = SearchEngineQueryRequestState> = {
  enqueue(request: TRequest): void;
  flush(): TRequest | null;
  clear(): void;
  pending(): TRequest | null;
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
const VISIBLE_GROUP_ORDER: SearchExpandableGroupId[] = [
  'apps',
  'folders',
  'files',
  'settings',
  'windows',
  'commands'
];
export const DEFAULT_VISIBLE_GROUP_LIMIT = 7;

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

export function buildVisibleSearchRows(
  results: readonly SearchPanelResult[],
  options: SearchVisibleRowsBuildOptions = {}
): SearchVisibleRow[] {
  const orderedResults = reorderVisibleSearchResults(results);

  void options;

  return orderedResults.map((entry, visibleIndex) => ({
    result: entry.result,
    resultIndex: entry.index,
    id: entry.result.id,
    rowKey: visibleRowKey(entry.result.id, entry.index),
    domId: visibleRowDomId(entry.index),
    visibleIndex,
    groupId: 'bestMatch',
    groupLabel: visibleGroupLabel('bestMatch'),
    showGroupLabel: false
  }));
}

export function buildVisibleSearchGroupOverflows(
  results: readonly SearchPanelResult[],
  options: SearchVisibleRowsBuildOptions = {}
): SearchVisibleGroupOverflow[] {
  void results;
  void options;

  return [];
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

export function searchVisibleRowIdentity(row: SearchVisibleRow): SearchVisibleRowIdentity {
  return {
    id: row.id,
    rowKey: row.rowKey,
    recordKey: row.result.recordKey,
    resultIndex: row.resultIndex
  };
}

export function resolveVisibleSearchRowResultIndex(
  results: readonly SearchPanelResult[],
  identity: SearchVisibleRowIdentity | string
): number {
  if (typeof identity === 'string') {
    return results.findIndex((result) => result.id === identity);
  }

  if (
    Number.isInteger(identity.resultIndex) &&
    identity.resultIndex !== undefined &&
    identity.resultIndex >= 0 &&
    identity.resultIndex < results.length
  ) {
    const result = results[identity.resultIndex];
    if (
      result.id === identity.id &&
      (identity.recordKey === undefined || identity.recordKey === result.recordKey)
    ) {
      return identity.resultIndex;
    }
  }

  if (identity.recordKey) {
    const recordKeyIndex = results.findIndex((result) =>
      result.id === identity.id && result.recordKey === identity.recordKey
    );
    if (recordKeyIndex >= 0) {
      return recordKeyIndex;
    }
  }

  return results.findIndex((result) => result.id === identity.id);
}

export function nextProgressiveSearchResultSet(
  current: SearchProgressiveResultSet,
  update: SearchProgressiveResultUpdate
): SearchProgressiveResultSet {
  if (update.phase === 'typing') {
    return current;
  }
  if (update.phase === 'error' && update.results.length === 0) {
    return current;
  }
  return {
    query: update.query,
    results: update.results
  };
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

export function shouldRetrySearchFreshness(
  currentQuery: string,
  resultQuery: string,
  statusMessage: string,
  requestSequence: number,
  currentSequence: number
): boolean {
  const normalizedQuery = currentQuery.trim();
  if (!normalizedQuery || requestSequence !== currentSequence) {
    return false;
  }
  if (!statusMessage.startsWith('Searching')) {
    return false;
  }
  return resultQuery.trim() !== normalizedQuery || statusMessage === 'Searching...'
    || statusMessage === 'Searching local providers...';
}

export function shouldRetrySearchAfterProviderCacheWarm(
  request: SearchEngineQueryRequestState,
  currentQuery: string,
  currentSequence: number,
  providerTimings: readonly SearchProviderCacheRetryTiming[] = []
): boolean {
  if (!shouldApplySearchEngineResponse(request, currentQuery, currentSequence)) {
    return false;
  }

  return providerTimings.some((timing) => {
    if (timing.providerId !== 'apps') {
      return false;
    }
    if (timing.cache === 'miss' || timing.cache === 'indexing') {
      return true;
    }
    return timing.cache === 'refresh';
  });
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

export function clearLatestSearchQuery(
  controller: Pick<LatestSearchQueryController, 'next'>
): SearchEngineQueryRequestState {
  return controller.next('');
}

export function createLatestSearchExecutionQueue<TRequest>(
  execute: (request: TRequest) => void
): LatestSearchExecutionQueue<TRequest> {
  let pendingRequest: TRequest | null = null;
  return {
    enqueue(request: TRequest) {
      pendingRequest = request;
    },
    flush() {
      const request = pendingRequest;
      pendingRequest = null;
      if (request !== null) {
        execute(request);
      }
      return request;
    },
    clear() {
      pendingRequest = null;
    },
    pending() {
      return pendingRequest;
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

function reorderVisibleSearchResults(results: readonly SearchPanelResult[]): GroupedSearchResult[] {
  const leadingApps: GroupedSearchResult[] = [];
  let tailStartIndex = results.length;

  for (let index = 0; index < results.length; index += 1) {
    const result = results[index];
    if (result.kind === 'app' && tailStartIndex === results.length) {
      leadingApps.push({ result, index });
      continue;
    }
    tailStartIndex = index;
    break;
  }

  const tail = results.slice(tailStartIndex).map((result, index) => ({ result, index: tailStartIndex + index }));

  if (!tail.length || leadingApps.length <= 4) {
    return results.map((result, index) => ({ result, index }));
  }

  return leadingApps.slice(0, 4).concat(tail, leadingApps.slice(4));
}
