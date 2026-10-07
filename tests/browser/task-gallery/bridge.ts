type Handler = (event: { payload: unknown }) => unknown;
export const handlers = new Map<string, Set<Handler>>();
export const ledger: { kind: string; name: string; args: unknown }[] = [];
export const pending: { args: unknown; resolve: () => void; reject: (error: Error) => void }[] = [];
let sequence = 0;
function record(kind: string, name: string, args?: unknown) {
  ledger.push({ kind, name, args });
  document.querySelector('#ledger')!.textContent = JSON.stringify(ledger, null, 2);
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
export async function emit(name: string, payload?: unknown) { record('outgoing event', name, payload); }
export async function emitTo(_target: unknown, name: string, payload?: unknown) { await emit(name, payload); }
export function getCurrentWindow() { return { label: 'task-gallery', listen }; }
export async function invoke<T>(name: string, args: Record<string, unknown> = {}): Promise<T> {
  record('command', name, args);
  switch (name) {
    case 'allocate_task_preview_request_id': return ++sequence as T;
    case 'close_task_gallery_window':
      return await new Promise<void>((resolve, reject) => pending.push({ args, resolve, reject })) as T;
    case 'hide_task_gallery':
      await publish('task-gallery:closed', { nonce: args.nonce }); return undefined as T;
    case 'hide_task_gallery_on_focus_loss':
    case 'activate_task_gallery_window':
    case 'show_task_gallery_window_preview':
    case 'hide_task_gallery_window_preview': return undefined as T;
    default: throw new Error(`Isolated gallery fixture refuses ${name}`);
  }
}
