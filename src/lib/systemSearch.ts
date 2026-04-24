import { invoke } from '@tauri-apps/api/core';
import type { SearchPanelResult, SearchPanelResultKind } from './searchPanel';

export const SEARCH_INDEX_REFRESHED_EVENT = 'search-index:refreshed';

type SystemSearchResult = {
  id: string;
  kind: SearchPanelResultKind;
  title: string;
  subtitle: string;
  terms: string;
  priority: number;
  path: string;
};

export function searchSystem(query: string): Promise<SearchPanelResult[]> {
  return invoke<SystemSearchResult[]>('search_system', { query }).then((results) =>
    results.map((result) => ({
      id: result.id,
      kind: result.kind,
      path: result.path,
      priority: result.priority,
      subtitle: result.subtitle,
      terms: result.terms,
      title: result.title
    }))
  );
}

export function isSystemPathResult(result: SearchPanelResult): boolean {
  return result.id.startsWith('system:') && Boolean(result.path);
}
