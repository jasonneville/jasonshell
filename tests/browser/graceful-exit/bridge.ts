import { defaultShellSettings } from '../../../src/lib/settings';
import * as stack from '../stack-mouse/bridge';

const surface = new URLSearchParams(location.search).get('surface');
const label = surface === 'stack' ? 'stack-popup' : surface === 'control' ? 'control-plane' : 'settings-panel';
export const ledger: { command: string; args?: any }[] = [];
export const listen = stack.listen;
export const emit = stack.emit;
export const emitTo = stack.emitTo;
export const getCurrentWindow = () => ({ label, listen, onDragDropEvent: async () => () => {} });
export const getCurrentWebviewWindow = getCurrentWindow;
export async function invoke<T>(command: string, args: any = {}): Promise<T> {
  ledger.push({ command, args });
  if (command === 'request_shell_exit') {
    if (label !== 'settings-panel' || Object.keys(args).length !== 0) throw new Error('Unexpected exit request');
    return undefined as T;
  }
  if (command.includes('shell_exit') || command === 'get_shell_shutdown_status') throw new Error(`Retired exit command: ${command}`);
  if (command === 'load_shell_settings') return defaultShellSettings() as T;
  if (command === 'get_standard_hotkey_status') return { revision: 1, bindings: [] } as T;
  if (command === 'get_speech_model_status') return { state: 'missing', source: null, error: null } as T;
  if (command === 'start_stack_folder_watch') return { watchId: 'fixture-watch', canonicalPath: args.path } as T;
  if (command === 'stop_stack_folder_watch') return undefined as T;
  if (command === 'hide_settings_panel' || command === 'hide_stack_popup') throw new Error('Exit fixture forbids accidental hide');
  return stack.invoke(command, args);
}
