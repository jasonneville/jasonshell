export type BasicTextEditorExitAction = () => void | Promise<void>;

export type PendingBasicTextEditorExit = Readonly<{
  action: BasicTextEditorExitAction;
}>;

export type BasicTextEditorViewport = Readonly<{
  scrollTop: number;
  height: number;
}>;

export type BasicTextEditorExitRequest = Readonly<{
  state: PendingBasicTextEditorExit | null;
  accepted: boolean;
  completion: Promise<void> | null;
}>;

export function resetBasicTextEditorViewport(_viewport: BasicTextEditorViewport): BasicTextEditorViewport {
  return { scrollTop: 0, height: 0 };
}

export function beginBasicTextEditorExit(
  state: PendingBasicTextEditorExit | null,
  editorActive: boolean,
  editorDirty: boolean,
  action: BasicTextEditorExitAction,
  dismissEditor: () => void
): BasicTextEditorExitRequest {
  if (!editorActive) {
    return { state, accepted: true, completion: Promise.resolve().then(action) };
  }
  if (!editorDirty) {
    dismissEditor();
    return { state, accepted: true, completion: Promise.resolve().then(action) };
  }
  const pending = enqueuePendingEditorExit(state, action);
  return { ...pending, completion: null };
}

export function enqueuePendingEditorExit(
  state: PendingBasicTextEditorExit | null,
  action: BasicTextEditorExitAction
): { state: PendingBasicTextEditorExit; accepted: boolean } {
  if (state) return { state, accepted: false };
  return { state: { action }, accepted: true };
}

export function cancelPendingEditorExit(_state: PendingBasicTextEditorExit | null): null {
  return null;
}

export function takePendingEditorExit(
  state: PendingBasicTextEditorExit | null
): { state: null; action: BasicTextEditorExitAction | null } {
  return { state: null, action: state?.action ?? null };
}
