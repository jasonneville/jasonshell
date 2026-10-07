type Handler = (event: { payload: unknown }) => unknown;
export const handlers = new Map<string, Handler>();
export const ledger: unknown[] = [];
export async function listen(name: string, handler: Handler) { handlers.set(name, handler); return () => handlers.delete(name); }
export async function publish(payload: unknown) { await handlers.get('task-preview:update')?.({ payload }); }
export async function emit(name: string, payload?: unknown) { ledger.push({ name, payload }); }
export const emitTo = emit;
export async function invoke(name: string, args: unknown) {
  if (!['maximize_task_window', 'close_task_window', 'close_task_gallery_previewed_window'].includes(name)) throw new Error(`Fixture refuses ${name}`);
  ledger.push({ name, args });
}
