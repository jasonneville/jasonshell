import type { SearchPanelPayload } from './searchPanel';

export type SearchPanelAnchorState = {
  left: number;
  width: number;
};

export function shouldApplySystemSearchResponse(
  requestQuery: string,
  requestSequence: number,
  currentQuery: string,
  currentSequence: number
) {
  return requestSequence === currentSequence && requestQuery === currentQuery.trim();
}

export function shouldRetryIndexedSearch(resultsLength: number, refreshAttempt: number) {
  return resultsLength === 0 && refreshAttempt < 2;
}

export function shouldRefreshSystemSearchAfterIndexUpdate(
  searchOpen: boolean,
  currentQuery: string
) {
  return searchOpen && currentQuery.trim().length > 0;
}

export function searchPanelAnchorState(rect: Pick<DOMRect, 'left' | 'width'>): SearchPanelAnchorState {
  return {
    left: Math.round(rect.left),
    width: Math.round(rect.width)
  };
}

export function shouldShowSearchPanelForAnchor(
  searchOpen: boolean,
  previousAnchor: SearchPanelAnchorState | null,
  nextAnchor: SearchPanelAnchorState
) {
  return !searchOpen
    || !previousAnchor
    || previousAnchor.left !== nextAnchor.left
    || previousAnchor.width !== nextAnchor.width;
}

export function searchPanelPayloadSignature(payload: SearchPanelPayload): string {
  return JSON.stringify({
    query: payload.query,
    presentation: payload.presentation ?? null,
    selectedIndex: payload.selectedIndex,
    statusMessage: payload.statusMessage,
    results: payload.results.map((result) => ({
      id: result.id,
      providerId: result.providerId ?? null,
      kind: result.kind,
      title: result.title,
      subtitle: result.subtitle,
      terms: result.terms,
      priority: result.priority,
      iconDataUrl: result.iconDataUrl ?? null,
      path: result.path ?? null,
      url: result.url ?? null,
      actionId: result.actionId ?? null,
      actionArgs: result.actionArgs ?? null,
      recordKey: result.recordKey ?? null,
      runCount: result.runCount ?? null,
      topMost: result.topMost ?? null
    }))
  });
}

export function shouldPublishSearchPanelPayload(
  previousSignature: string | null,
  payload: SearchPanelPayload
) {
  return previousSignature !== searchPanelPayloadSignature(payload);
}
