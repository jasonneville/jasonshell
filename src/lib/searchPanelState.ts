import type { SearchPanelPayload, SearchPanelResult } from './searchPanel';

export type SearchPanelViewState = {
  query: string;
  results: SearchPanelResult[];
  selectedIndex: number;
  statusMessage: string;
  presentation: 'anchored' | 'centered';
};

export const defaultSearchPanelViewState: SearchPanelViewState = {
  query: '',
  results: [],
  selectedIndex: 0,
  statusMessage: 'Search is ready',
  presentation: 'centered'
};

export function applySearchPanelPayload(
  current: SearchPanelViewState,
  payload: SearchPanelPayload | null
): SearchPanelViewState {
  if (!payload) {
    return current;
  }

  return {
    query: payload.query,
    results: payload.results,
    selectedIndex: payload.selectedIndex,
    statusMessage: payload.statusMessage,
    presentation: payload.presentation ?? current.presentation
  };
}

export function shouldRevealSelectedResult(selectedIndex: number, resultCount: number) {
  return selectedIndex >= 0 && selectedIndex < resultCount;
}
