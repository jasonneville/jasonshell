import type { SearchPanelPayload, SearchPanelResult } from './searchPanel';

export type SearchPanelViewState = {
  query: string;
  results: SearchPanelResult[];
  selectedIndex: number;
  statusMessage: string;
  presentation: 'anchored' | 'centered';
  phase: 'typing' | 'local' | 'provider' | 'complete' | 'error';
  sequence: number;
};

export const defaultSearchPanelViewState: SearchPanelViewState = {
  query: '',
  results: [],
  selectedIndex: 0,
  statusMessage: 'Search is ready',
  presentation: 'centered',
  phase: 'complete',
  sequence: 0
};

export function applySearchPanelPayload(
  current: SearchPanelViewState,
  payload: SearchPanelPayload | null
): SearchPanelViewState {
  if (!payload) {
    return current;
  }
  if (!shouldApplySearchPanelPayload(current, payload)) {
    return current;
  }

  const nextSequence = payload.sequence ?? current.sequence;
  const nextPhase = payload.phase ?? current.phase;
  const currentQueryIdentity = current.query.trim();
  const payloadQueryIdentity = payload.query.trim();
  const shouldKeepResults =
    (payload.phase === 'typing' && payloadQueryIdentity === currentQueryIdentity)
    || (payload.phase === 'error' && payload.results.length === 0 && nextSequence === current.sequence);

  return {
    query: payload.sequence === undefined ? payload.query : payloadQueryIdentity,
    results: shouldKeepResults ? current.results : payload.results,
    selectedIndex: payload.selectedIndex,
    statusMessage: payload.statusMessage,
    presentation: payload.presentation ?? current.presentation,
    phase: nextPhase,
    sequence: nextSequence
  };
}

export function shouldRevealSelectedResult(selectedIndex: number, resultCount: number) {
  return selectedIndex >= 0 && selectedIndex < resultCount;
}

function shouldApplySearchPanelPayload(
  current: SearchPanelViewState,
  payload: SearchPanelPayload
): boolean {
  if (payload.sequence === undefined) {
    return true;
  }
  if (payload.sequence < current.sequence) {
    return false;
  }
  if (payload.sequence > current.sequence) {
    return true;
  }
  if (payload.query.trim() !== current.query.trim()) {
    return false;
  }
  return phaseRank(payload.phase) >= phaseRank(current.phase);
}

function phaseRank(phase: SearchPanelPayload['phase'] | SearchPanelViewState['phase'] | undefined): number {
  switch (phase) {
    case 'typing':
      return 0;
    case 'local':
      return 1;
    case 'provider':
      return 2;
    case 'error':
      return 2;
    case 'complete':
      return 3;
    default:
      return 3;
  }
}
