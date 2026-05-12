import type { SearchPanelResult, SearchPanelResultKind } from './searchPanel';

// Legacy compatibility wrapper for historical tests/diagnostics only.
// Visible typed search must use searchEngine.ts and Rust search_engine.
export const SEARCH_INDEX_REFRESHED_EVENT = 'search-index:refreshed';

type SystemSearchResult = {
  id: string;
  providerId?: string;
  kind: SearchPanelResultKind;
  title: string;
  subtitle: string;
  terms: string;
  priority: number;
  path: string;
  recordKey?: string;
  runCount?: number;
  topMost?: boolean;
};

export function searchSystem(query: string): Promise<SearchPanelResult[]> {
  void query;
  return Promise.reject(
    new Error('search_system is deprecated; visible search must call searchEngine')
  );
}

export function isSystemPathResult(result: SearchPanelResult): boolean {
  return result.id.startsWith('system:') && Boolean(result.path);
}
