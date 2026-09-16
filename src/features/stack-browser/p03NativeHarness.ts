export const P03_PROJECTION_UNITS = 65_536;

export interface HarnessLease { id: string; text: string }
export interface ImeHarnessState {
  leases: HarnessLease[];
  projection: string;
  composing: boolean;
  compositionPinnedLeaseId: string | null;
  pendingComposition: boolean;
  inputCommits: number;
  lastOutcome: 'none' | 'commit' | 'cancel' | 'blur';
}

export function createImeHarnessState(text: string): ImeHarnessState {
  if (text.length > P03_PROJECTION_UNITS) throw new Error('ResourceLimit');
  return { leases: [{ id: 'left', text }], projection: text, composing: false, compositionPinnedLeaseId: null, pendingComposition: false, inputCommits: 0, lastOutcome: 'none' };
}

export function startComposition(state: ImeHarnessState): ImeHarnessState {
  return { ...state, composing: true, compositionPinnedLeaseId: state.leases[0]?.id ?? null, pendingComposition: false, lastOutcome: 'none' };
}

export function reconcileAdjacentLease(state: ImeHarnessState, id: string, text: string): ImeHarnessState {
  const leftUnits = state.leases.length === 1 ? state.projection.length : (state.leases[0]?.text.length ?? 0);
  const left = { ...(state.leases[0] ?? { id: 'left', text: '' }), text: state.projection.slice(0, leftUnits) };
  const leases = [left, { id, text }];
  const projection = leases.map(lease => lease.text).join('');
  if (leases.length > 2 || projection.length > P03_PROJECTION_UNITS) throw new Error('ResourceLimit');
  return { ...state, leases, projection };
}

export function finishComposition(state: ImeHarnessState, data: string): ImeHarnessState {
  if (!state.composing) return state;
  if (!data) return { ...state, composing: false, compositionPinnedLeaseId: null, pendingComposition: false, lastOutcome: 'cancel' };
  return { ...state, composing: false, pendingComposition: true };
}

export function cancelCompositionOnBlur(state: ImeHarnessState): ImeHarnessState {
  if (!state.composing && !state.pendingComposition) return state;
  return { ...state, composing: false, compositionPinnedLeaseId: null, pendingComposition: false, lastOutcome: 'blur' };
}

export function noteCommittedInput(state: ImeHarnessState, event: { isTrusted: boolean; inputType: string; data: string | null; isComposing?: boolean }): ImeHarnessState {
  const isFinalCompositionInput = state.pendingComposition || (state.composing && event.isComposing === false);
  if (!isFinalCompositionInput || !event.isTrusted || !event.inputType.startsWith('insert')) return state;
  return { ...state, composing: false, pendingComposition: false, compositionPinnedLeaseId: null, inputCommits: state.inputCommits + 1, lastOutcome: 'commit' };
}

export function redactEventText(text: string | null): { units: number; preview: string } {
  const units = text?.length ?? 0;
  return { units, preview: `[text:${units}u]` };
}
