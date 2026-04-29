import type { SearchPanelResult } from '../../lib/searchPanel';
import type { SearchMode } from '../../lib/searchSettings';

export type SearchResultGroupId = 'apps' | 'windows' | 'places' | 'files' | 'commands';

export type GroupedSearchResult = {
  result: SearchPanelResult;
  index: number;
};

export type SearchResultGroup = {
  id: SearchResultGroupId;
  label: string;
  items: GroupedSearchResult[];
};

export type SearchResultActionHints = {
  primary: string;
  secondary: string | null;
};

export type SearchResultRefreshRequest = {
  query: string;
  sequence: number;
};

export type SearchKeyboardAction =
  | 'none'
  | 'openTopRight'
  | 'openCentered'
  | 'close'
  | 'activate'
  | 'selectPrevious'
  | 'selectNext';

const GROUP_ORDER: SearchResultGroupId[] = ['apps', 'windows', 'places', 'files', 'commands'];

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

export function searchModeFromSettings(value: unknown): SearchMode {
  return value === 'centeredHotkey' ? 'centeredHotkey' : 'topRight';
}

export function ctrlKSearchAction(mode: SearchMode): SearchKeyboardAction {
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
  }
}
