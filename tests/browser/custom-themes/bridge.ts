import { defaultShellSettings } from '../../../src/lib/settings';

type Handler = (event: { payload: unknown }) => unknown;
const handlers = new Map<string, Set<Handler>>();
export const ledger: { command: string; args?: unknown }[] = [];
export async function listen(name: string, handler: Handler) {
  const listeners = handlers.get(name) ?? new Set<Handler>();
  listeners.add(handler); handlers.set(name, listeners);
  return () => { listeners.delete(handler); };
}
export async function emit(name: string, payload?: unknown) {
  for (const listener of handlers.get(name) ?? []) await listener({ payload });
}
export async function emitTo(_target: unknown, name: string, payload?: unknown) { await emit(name, payload); }
export const getCurrentWindow = () => ({ label: 'theme-browser-fixture', listen,
  show: async () => { ledger.push({ command: 'mock-window-show' }); },
  hide: async () => { ledger.push({ command: 'mock-window-hide' }); } });
export const getCurrentWebviewWindow = getCurrentWindow;
export async function invoke(command: string, args?: unknown) {
  ledger.push({ command, args });
  if (command === 'load_shell_settings') return defaultShellSettings();
  if (command === 'get_speech_model_status') return { state: 'missing', source: null, error: null };
  if (command === 'get_speech_status') return { status: 'recording', nonce: 1 };
  if (command === 'hide_settings_panel') { document.getElementById('app')!.hidden = true; return; }
  throw new Error(`Custom-theme fixture refuses native IPC: ${command}`);
}
