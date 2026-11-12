import type { SearchPanelPayload } from '../../../src/lib/searchPanel';
type Handler = (event: { payload: SearchPanelPayload }) => unknown;
export const handlers = new Map<string, Handler>();
export const ledger: Array<{ name: string; payload?: unknown }> = [];
export let current: SearchPanelPayload = { query: '', results: [], selectedIndex: -1, statusMessage: '', presentation: 'centered' };
export async function listen(name: string, handler: Handler) { handlers.set(name, handler); return () => handlers.delete(name); }
export const getCurrentWindow = () => ({ label: 'search-panel', listen });
export async function publish(payload: SearchPanelPayload) { current = payload; await handlers.get('search-panel:update')?.({ payload }); }
export async function emitTo(_target: unknown, name: string, payload?: unknown) { ledger.push({ name, payload }); }
export async function invoke(name: string) {
  if (name === 'get_search_panel_payload') return current;
  if (name === 'hide_search_panel' || name === 'resize_search_panel') { ledger.push({ name }); return null; }
  throw new Error(`Search fixture refuses native command ${name}`);
}
