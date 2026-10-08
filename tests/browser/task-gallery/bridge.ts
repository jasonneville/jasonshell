import { taskGalleryLayout, type TaskGalleryLayout } from '../../../src/lib/taskGalleryLayout';
type Handler = (event: { payload: unknown }) => unknown;
type Row = { hwnd: string; title: string; isMinimized?: boolean; [key: string]: unknown };
type SyncArgs = { nonce: string; revision: number; requestId: number; frames: { hwnd: string; bounds: unknown }[]; viewport: unknown };
export const handlers = new Map<string, Set<Handler>>();
export const ledger: { kind: string; name: string; args: any }[] = [];
export const pending: { args: unknown; resolve: () => void; reject: (error: Error) => void }[] = [];
export const pendingSync: { args: SyncArgs; resolve: (result?: unknown) => void; reject: (error: Error) => void }[] = [];
export const state = { rows: [] as Row[], availableWidth: 960, availableHeight: 600, revision: 0,
  deferSync: false, failSync: false, liveFrames: [] as string[], latestSyncRequestId: 0, open: null as any,
  onLayout: (_layout: TaskGalleryLayout) => {} };
let sequence = 0;
function record(kind: string, name: string, args?: unknown) {
  ledger.push({ kind, name, args });
  const output = document.querySelector('#ledger');
  if (output) output.textContent = JSON.stringify(ledger, null, 2);
}
export async function listen(name: string, handler: Handler) {
  const subscribers = handlers.get(name) ?? new Set<Handler>();
  subscribers.add(handler); handlers.set(name, subscribers);
  return () => { subscribers.delete(handler); };
}
export async function publish(name: string, payload: unknown) {
  record('incoming event', name, payload);
  await Promise.all([...handlers.get(name) ?? []].map((handler) => handler({ payload })));
}
export async function emit(name: string, payload?: unknown) {
  record('outgoing event', name, payload);
  // Real Tauri events cross an asynchronous IPC boundary. Delivering renderer
  // emits reentrantly inside pointerenter reverses native pointer/mouse ordering
  // when both distinct native surfaces share this test document.
  await new Promise<void>((resolve) => window.setTimeout(resolve, 0));
  await Promise.all([...handlers.get(name) ?? []].map((handler) => handler({ payload })));
}
export async function emitTo(_target: unknown, name: string, payload?: unknown) { await emit(name, payload); }
export function getCurrentWindow() { return { label: 'task-gallery', listen }; }
export async function openNative(args: any) {
  const layout = taskGalleryLayout(args.windows.length, state.availableWidth, state.availableHeight);
  const reusable = state.open?.nonce === args.nonce && JSON.stringify(state.open?.layout) === JSON.stringify(layout);
  state.open = { ...args, layout, revision: ++state.revision };
  if (!reusable) { state.latestSyncRequestId = 0; state.liveFrames = []; }
  else state.liveFrames = state.liveFrames.filter(hwnd => args.windows.some((row: Row) => row.hwnd === hwnd));
  state.onLayout(layout);
  await publish('task-gallery:open', state.open);
  return layout;
}
// Clearly synthetic captured test image. Real production component displays the
// mocked native response; this fixture never pretends to certify DWM composition.
export function capturedTestImage(color = '#789abc') {
  const canvas = document.createElement('canvas'); canvas.width = 172; canvas.height = 96;
  const context = canvas.getContext('2d')!; context.fillStyle = color; context.fillRect(0, 0, 172, 96);
  return canvas.toDataURL('image/png');
}
export function syncResult(args: SyncArgs) {
  return { nonce: args.nonce, revision: args.revision, requestId: args.requestId,
    thumbnails: args.frames.map(({ hwnd }, index) => index % 3 === 0
      ? { hwnd, source: 'live', imageDataUrl: capturedTestImage('#527ca8'), error: null }
      : index % 3 === 1 ? { hwnd, source: 'captured', imageDataUrl: capturedTestImage(), error: null }
      : { hwnd, source: 'unavailable', imageDataUrl: null, error: 'Mock capture unavailable' }) };
}
export async function invoke<T>(name: string, args: Record<string, any> = {}): Promise<T> {
  record('command', name, args);
  switch (name) {
    case 'allocate_task_preview_request_id': return ++sequence as T;
    case 'list_open_task_windows': return state.rows as T;
    case 'list_pinned_taskbar_apps': return [] as T;
    case 'load_shell_settings': return { ui: { lockBottomBarHeight: true, bottomBarHeightLogical: 24 } } as T;
    case 'show_task_gallery': return await openNative(args.args) as T;
    case 'sync_task_gallery_thumbnails': {
      const request = args.args as SyncArgs;
      if (state.failSync) throw new Error('Mock thumbnail sync failed');
      state.latestSyncRequestId = Math.max(state.latestSyncRequestId, request.requestId);
      const acknowledge = (result: any) => {
        // Receipt queues native work; only an accepted current result changes
        // visible DWM registrations. Pending/rejected clears preserve truth.
        if (state.open?.nonce === request.nonce && state.open?.revision === request.revision
          && state.latestSyncRequestId === request.requestId && result?.nonce === request.nonce
          && result?.revision === request.revision && result?.requestId === request.requestId) {
          state.liveFrames = result.thumbnails.filter((row: any) => row.source === 'live').map((row: any) => row.hwnd);
        }
        return result;
      };
      if (state.deferSync) return await new Promise((resolve, reject) => pendingSync.push({ args: request,
        resolve: (result) => resolve(acknowledge(result ?? syncResult(request))), reject })) as T;
      return acknowledge(syncResult(request)) as T;
    }
    case 'close_task_gallery_window':
      return await new Promise<void>((resolve, reject) => pending.push({ args, resolve, reject })) as T;
    case 'hide_task_gallery':
      state.liveFrames = [];
      await publish('task-gallery:closed', { nonce: args.nonce }); return undefined as T;
    case 'hide_task_gallery_on_focus_loss':
    case 'activate_task_gallery_window':
    case 'show_task_gallery_window_preview':
    case 'hide_task_gallery_window_preview':
    case 'request_taskbar_windows_refresh':
    case 'report_shell_surface_runtime_metrics':
    case 'hide_task_window_preview':
    case 'run_task_window_action':
    case 'hide_context_menu_overlay': return undefined as T;
    case 'show_context_menu_overlay': await publish('context-menu-overlay:open', args.request); return undefined as T;
    default: throw new Error(`Isolated gallery fixture refuses ${name}`);
  }
}
