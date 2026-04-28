import type { SearchPanelResult } from '../../lib/searchPanel';

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
  return { primary: 'Run', secondary: null };
}

export function searchResultGroupId(result: SearchPanelResult): SearchResultGroupId {
  if (result.kind === 'app') {
    return 'apps';
  }
  if (result.kind === 'window') {
    return 'windows';
  }
  if (result.kind === 'command') {
    return 'commands';
  }
  if (result.kind === 'folder') {
    return 'places';
  }
  return 'files';
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
