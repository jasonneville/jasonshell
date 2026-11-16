import { createCustomTheme, validateCustomTheme, type CustomTheme } from './themePalette.js';
import { persistCustomTheme, persistDeleteCustomTheme, type SavedThemeState } from './themePersistence.js';
import { applyShellTheme, applyThemeToElement, commitShellThemeSelection, getSavedThemeState, publishSavedThemeState } from './themes.js';

export const THEME_PREVIEW_CHANNEL = 'jasonshell.themes.preview.v1';
export const THEME_PREVIEW_PROTOCOL_VERSION = 1;
const MAX_MESSAGE_BYTES = 96 * 1024;
const PRESENCE_INTERVAL_MS = 15_000;
const PRESENCE_TIMEOUT_MS = 8_000;
type Session = { owner: string; session: string; startedAt: number; revision: number; theme: CustomTheme };
type Packet = {
  version: 1; type: 'preview' | 'end' | 'query' | 'presence' | 'select' | 'selection-result';
  sender: string; owner?: string; session?: string; startedAt?: number; revision?: number;
  theme?: CustomTheme; requestId?: string; selectedId?: string; expiresAt?: number; accepted?: boolean; error?: string;
};
export interface ThemePreviewStatus { active: boolean; owner: string | null; session: string | null; isOwner: boolean; error: string | null }
export interface ThemeEditor {
  getDraft(): CustomTheme;
  update(draft: CustomTheme): void;
  reset(): void;
  isDirty(): boolean;
  save(): CustomTheme;
  cancel(): void;
  dispose(): void;
}
export interface ThemeEditorOptions { onSelectionRequest?: (id: string) => boolean | Promise<boolean> }

let rendererId: string | null = null;
function identity(): string { return rendererId ??= crypto.randomUUID(); }
let active: Session | null = null;
let localEditor: { editor: ThemeEditor; session: Session; options: ThemeEditorOptions } | null = null;
let channel: BroadcastChannel | null = null;
let syncUsers = 0;
let cleanupSync: (() => void) | null = null;
let error: string | null = null;
let lastSessionOrder = '';
const retired = new Set<string>();
const selectionRequestIds = new Set<string>();
const statusListeners = new Set<(status: ThemePreviewStatus) => void>();
const requests = new Map<string, { owner: string; session: string; resolve: (accepted: boolean) => void; reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> }>();
let presenceTimer: ReturnType<typeof setTimeout> | null = null;
let presenceRequest: { id: string; session: string; startedAt: number; misses: number } | null = null;

function copy<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T; }
function order(session: Session): string { return `${String(session.startedAt).padStart(16, '0')}:${session.session}`; }
function status(): ThemePreviewStatus { return { active: !!active, owner: active?.owner ?? null, session: active?.session ?? null, isOwner: !!active && active.owner === identity(), error }; }
function notify(): void {
  for (const listener of statusListeners) {
    try { listener(status()); } catch { /* Observers cannot interrupt commit/cleanup. */ }
  }
}
export function reportThemeSyncError(message: string): void { error = message; notify(); }
export function subscribeThemePreview(listener: (state: ThemePreviewStatus) => void): () => void {
  statusListeners.add(listener); listener(status()); return () => { statusListeners.delete(listener); };
}
export function getThemePreviewStatus(): ThemePreviewStatus { return status(); }
export function currentPreviewTheme(): CustomTheme | null { return active ? copy(active.theme) : null; }
export function previewSelectionGuard(): boolean { return !!active; }

function send(packet: Omit<Packet, 'version' | 'sender'>): void {
  try { channel?.postMessage({ ...packet, version: 1, sender: identity() }); }
  catch { error = 'Theme preview synchronization is unavailable'; notify(); }
}
function sessionPacket(type: 'preview' | 'end' | 'presence', session: Session, requestId?: string): Omit<Packet, 'version' | 'sender'> {
  return { type, owner: session.owner, session: session.session, startedAt: session.startedAt, revision: session.revision,
    ...(type === 'preview' ? { theme: copy(session.theme) } : {}), ...(requestId ? { requestId } : {}) };
}
function restoreSaved(): void { applyShellTheme(getSavedThemeState().selectedId, { storage: null }); }
function retire(session: Session): void {
  retired.add(session.session);
  // The order watermark rejects earlier sessions after bounded tombstone eviction.
  lastSessionOrder = order(session) > lastSessionOrder ? order(session) : lastSessionOrder;
  if (retired.size > 256) retired.delete(retired.values().next().value!);
}
function clearPresence(): void {
  if (presenceTimer) clearTimeout(presenceTimer);
  presenceTimer = null; presenceRequest = null;
}
function finish(session: Session): void {
  retire(session);
  if (active?.session !== session.session) return;
  active = null;
  clearPresence();
  restoreSaved();
  notify();
}

/** Strict bounded decoder, also exported for deterministic protocol acceptance tests. */
export function validateThemePreviewMessage(value: unknown): Packet | null {
  try {
    if (!value || typeof value !== 'object' || Array.isArray(value) || JSON.stringify(value).length > MAX_MESSAGE_BYTES) return null;
    const packet = value as Record<string, unknown>;
    if (packet.version !== 1 || typeof packet.sender !== 'string' || !/^[\w-]{1,100}$/.test(packet.sender)) return null;
    if (!['preview', 'end', 'query', 'presence', 'select', 'selection-result'].includes(packet.type as string)) return null;
    if (['preview', 'end', 'presence', 'select', 'selection-result'].includes(packet.type as string)) {
      if (typeof packet.owner !== 'string' || !/^[\w-]{1,100}$/.test(packet.owner) || typeof packet.session !== 'string' || !/^[\w-]{1,100}$/.test(packet.session)) return null;
    }
    if (['preview', 'end', 'presence'].includes(packet.type as string)) {
      if (!Number.isSafeInteger(packet.startedAt) || (packet.startedAt as number) < 0 || (packet.startedAt as number) > Date.now() + 60_000 || !Number.isSafeInteger(packet.revision) || (packet.revision as number) < 1) return null;
      if (packet.owner !== packet.sender) return null;
    }
    if (packet.requestId !== undefined && (typeof packet.requestId !== 'string' || !/^[\w-]{1,100}$/.test(packet.requestId))) return null;
    if (packet.type === 'select' && (typeof packet.selectedId !== 'string' || packet.selectedId.length > 120 || !packet.requestId || !Number.isSafeInteger(packet.expiresAt) || (packet.expiresAt as number) <= Date.now() || (packet.expiresAt as number) > Date.now() + 31_000)) return null;
    if (packet.type === 'selection-result' && (typeof packet.accepted !== 'boolean' || !packet.requestId || packet.owner !== packet.sender)) return null;
    if (packet.error !== undefined && (typeof packet.error !== 'string' || packet.error.length > 240)) return null;
    return { ...packet, ...(packet.type === 'preview' ? { theme: validateCustomTheme(packet.theme) } : {}) } as Packet;
  } catch { return null; }
}

function receive(input: unknown): void {
  const packet = validateThemePreviewMessage(input);
  if (!packet || packet.sender === identity()) return;
  if (packet.type === 'query') {
    if (localEditor && active?.session === localEditor.session.session) {
      send(sessionPacket('preview', localEditor.session));
      send(sessionPacket('presence', localEditor.session, packet.requestId));
    }
    return;
  }
  if (packet.type === 'preview') {
    const incoming: Session = { owner: packet.owner!, session: packet.session!, startedAt: packet.startedAt!, revision: packet.revision!, theme: packet.theme! };
    if (retired.has(incoming.session)) return;
    if (active?.session === incoming.session) {
      if (active.owner !== incoming.owner || active.startedAt !== incoming.startedAt || incoming.revision <= active.revision) return;
    } else {
      if (order(incoming) <= lastSessionOrder || (active && order(incoming) <= order(active))) return;
      if (active) retire(active);
    }
    active = incoming;
    error = null;
    applyThemeToElement(incoming.theme);
    notify();
    return;
  }
  if (packet.type === 'end') {
    if (active && active.session === packet.session && active.owner === packet.owner && active.startedAt === packet.startedAt && packet.revision! > active.revision) finish(active);
    else if (!active) {
      const ended: Session = { owner: packet.owner!, session: packet.session!, startedAt: packet.startedAt!, revision: packet.revision!, theme: {} as CustomTheme };
      retire(ended);
    }
    return;
  }
  if (packet.type === 'presence') {
    if (presenceRequest && presenceRequest.id === packet.requestId && active && active.session === packet.session && active.owner === packet.owner) clearPresence();
    return;
  }
  if (packet.type === 'select') {
    if (localEditor && packet.owner === identity() && packet.session === localEditor.session.session) {
      if (selectionRequestIds.has(packet.requestId!)) return;
      selectionRequestIds.add(packet.requestId!);
      if (selectionRequestIds.size > 256) selectionRequestIds.delete(selectionRequestIds.values().next().value!);
      void handleSelection(packet.selectedId!, packet.expiresAt).then((accepted) => send({ type: 'selection-result', owner: identity(), session: packet.session, requestId: packet.requestId, accepted }))
        .catch(() => send({ type: 'selection-result', owner: identity(), session: packet.session, requestId: packet.requestId, accepted: false, error: 'Theme selection failed; retry in the editor' }));
    }
    return;
  }
  if (packet.type === 'selection-result') {
    const request = requests.get(packet.requestId!);
    if (!request || packet.owner !== request.owner || packet.session !== request.session) return;
    clearTimeout(request.timer); requests.delete(packet.requestId!);
    if (packet.error) request.reject(new Error(packet.error)); else request.resolve(packet.accepted!);
  }
}
let selectionPending = false;
async function handleSelection(id: string, expiresAt?: number): Promise<boolean> {
  const owner = localEditor;
  if (!owner || active?.session !== owner.session.session) throw new Error('Theme preview owner is unavailable; retry');
  if (selectionPending) return false;
  selectionPending = true;
  try {
    if (owner.options.onSelectionRequest) {
      if (!await owner.options.onSelectionRequest(id)) return false;
    } else if (owner.editor.isDirty()) throw new Error('Return to the theme editor to confirm this selection');
    // Confirmation may be asynchronous. Recheck ownership before committing.
    if (localEditor !== owner || active?.session !== owner.session.session) throw new Error('Theme editor changed; retry');
    if (expiresAt !== undefined && Date.now() >= expiresAt) throw new Error('Theme selection request expired; retry');
    commitShellThemeSelection(id);
    owner.editor.cancel();
    return true;
  } finally { selectionPending = false; }
}

export function requestPreviewSelection(id: string): Promise<boolean> {
  if (!active) { commitShellThemeSelection(id); return Promise.resolve(true); }
  if (active.owner === identity()) return handleSelection(id);
  if (!channel) return Promise.reject(new Error('Theme preview owner is unavailable; reopen the editor and retry'));
  const requestId = crypto.randomUUID();
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      requests.delete(requestId);
      error = 'Theme editor did not answer; return to the editor or retry after recovery';
      notify(); reject(new Error(error));
    }, 30_000);
    requests.set(requestId, { owner: active!.owner, session: active!.session, resolve, reject, timer });
    send({ type: 'select', owner: active!.owner, session: active!.session, requestId, selectedId: id, expiresAt: Date.now() + 30_000 });
  });
}

/** Query-response presence, not a hidden-owner timer lease. Hidden initialized
 * WebViews can answer events; there is no TrySuspend call in the shell. A
 * throttled/resumed observer restarts probes rather than expiring the owner. */
export function queryThemePreviewPresence(): void {
  if (!active || active.owner === identity() || presenceRequest || !channel) return;
  const session = active.session;
  const requestId = crypto.randomUUID();
  const probe = (misses: number) => {
    presenceRequest = { id: requestId, session, startedAt: Date.now(), misses };
    send({ type: 'query', requestId });
    presenceTimer = setTimeout(() => {
      if (!presenceRequest || active?.session !== session) { clearPresence(); return; }
      const elapsed = Date.now() - presenceRequest.startedAt;
      // A suspended/hidden observer has no evidence of a continuously live
      // deadline. Retry on resume and never infer owner death from timer lag.
      if (elapsed > PRESENCE_TIMEOUT_MS * 2 || (typeof document !== 'undefined' && document.visibilityState === 'hidden')) { clearPresence(); return; }
      if (misses < 1) { probe(misses + 1); return; }
      const lost = active;
      finish(lost);
      error = 'Theme preview owner closed or stopped responding; saved theme restored. Reopen the editor to retry.';
      notify();
    }, PRESENCE_TIMEOUT_MS);
  };
  probe(0);
}

export function installThemePreviewSync(): () => void {
  syncUsers++;
  if (!cleanupSync) {
    try {
      if (typeof window !== 'undefined' && typeof BroadcastChannel !== 'undefined') {
        channel = new BroadcastChannel(THEME_PREVIEW_CHANNEL);
        channel.onmessage = (event) => receive(event.data);
        send({ type: 'query', requestId: crypto.randomUUID() });
      }
    } catch { error = 'Cross-window theme preview is unavailable'; }
    const interval = channel ? setInterval(queryThemePreviewPresence, PRESENCE_INTERVAL_MS) : null;
    const resume = () => { clearPresence(); queryThemePreviewPresence(); };
    const unload = () => { localEditor?.editor.dispose(); };
    // document listeners do not fire on a native hide as editor disposal.
    if (typeof document !== 'undefined' && document.addEventListener) document.addEventListener('visibilitychange', resume);
    if (typeof window !== 'undefined' && typeof document !== 'undefined' && typeof document.addEventListener === 'function') window.addEventListener('pagehide', unload);
    cleanupSync = () => {
      localEditor?.editor.dispose();
      if (interval) clearInterval(interval);
      clearPresence();
      channel?.close(); channel = null;
      if (typeof document !== 'undefined' && document.removeEventListener) document.removeEventListener('visibilitychange', resume);
      if (typeof window !== 'undefined' && typeof document !== 'undefined' && typeof document.removeEventListener === 'function') window.removeEventListener('pagehide', unload);
      for (const request of requests.values()) { clearTimeout(request.timer); request.reject(new Error('Theme synchronization disposed')); }
      requests.clear();
      if (active) finish(active);
    };
  }
  let disposed = false;
  return () => {
    if (disposed) return;
    disposed = true;
    if (--syncUsers === 0) { cleanupSync?.(); cleanupSync = null; }
  };
}

export function createThemeEditor(source: string | CustomTheme, options: ThemeEditorOptions = {}): ThemeEditor {
  if (active || localEditor) throw new Error('A theme editor is already active; return to its window');
  const initial = typeof source === 'string' ? createCustomTheme(source, `${source} copy`) : validateCustomTheme(source);
  let draft = copy(initial);
  const saved = getSavedThemeState().customThemes.find((theme) => theme.id === initial.id);
  if (saved && saved.revision !== initial.revision || !saved && initial.revision > 0) throw new Error('This theme changed or was deleted in another window');
  const expectedRevision = saved ? initial.revision : null;
  // Sequential sessions remain strictly ordered even when created within one
  // millisecond or immediately after an observed renderer's session ended.
  const startedAt = Math.max(Date.now(), Number(lastSessionOrder.split(':')[0] || 0) + 1);
  const session: Session = { owner: identity(), session: crypto.randomUUID(), startedAt, revision: 1, theme: copy(draft) };
  let closed = false;
  const assertOpen = () => { if (closed || active?.session !== session.session) throw new Error('Theme editor is no longer active'); };
  const close = () => {
    if (closed) return;
    closed = true;
    session.revision++;
    send(sessionPacket('end', session));
    if (localEditor?.session.session === session.session) localEditor = null;
    finish(session);
  };
  const editor: ThemeEditor = {
    getDraft: () => copy(draft),
    update: (value) => {
      assertOpen();
      const next = validateCustomTheme(value);
      // Editing never changes identity, ancestry or original reset baseline.
      if (next.id !== initial.id || next.baseBuiltinId !== initial.baseBuiltinId || JSON.stringify(next.startingPalette) !== JSON.stringify(initial.startingPalette)) throw new Error('Cannot change a theme baseline while editing');
      draft = { ...next, revision: initial.revision };
      session.theme = copy(draft); session.revision++;
      active = copy(session); applyThemeToElement(draft); send(sessionPacket('preview', session)); notify();
    },
    reset: () => { editor.update({ ...draft, palette: { ...initial.startingPalette } }); },
    isDirty: () => JSON.stringify(draft) !== JSON.stringify(initial),
    save: () => {
      assertOpen();
      const state: SavedThemeState = persistCustomTheme(draft, expectedRevision);
      // Collection + selection must be committed before ending the preview.
      publishSavedThemeState();
      const result = state.customThemes.find((theme) => theme.id === draft.id)!;
      close();
      return copy(result);
    },
    cancel: close,
    dispose: close
  };
  active = copy(session); localEditor = { editor, session, options };
  error = typeof window !== 'undefined' && !channel ? 'Cross-window live preview is unavailable; this window can still save or cancel' : null;
  applyThemeToElement(draft); send(sessionPacket('preview', session)); notify();
  return editor;
}

export function deleteCustomTheme(id: string, expectedRevision: number): void {
  if (active) throw new Error('Close the theme editor before deleting a theme');
  persistDeleteCustomTheme(id, expectedRevision); publishSavedThemeState();
}
