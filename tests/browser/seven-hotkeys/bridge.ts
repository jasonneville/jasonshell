import { defaultShellSettings } from '../../../src/lib/settings';
import { context, processRow, snapshot } from '../../components/fixtures/system-info';

type Listener = { name: string; receiver: string; target: any; handler: (event: any) => void };
export type NativeFixture = ReturnType<typeof createNative>;
export function createNative() {
  return {
    settings: defaultShellSettings(), revision: 1, nextLease: 0, nextPreview: 0,
    context: context(0, 0, 'processes', null, false), failSave: false, settingsFocused: true, loseFocusOnSave: false, statusUnavailableOnSave: false,
    visible: { 'settings-panel': false, 'process-manager': false },
    listeners: new Set<Listener>(), trace: [] as Array<Record<string, any>>,
    lease: null as null | { leaseId: string; revision: number; sequence: number; expiresInMs: number; registeredBindings: string[] },
    record(type: string, data: Record<string, any>) { this.trace.push({ at: Math.round(performance.now()), type, ...structuredClone(data) }); },
    emit(target: string, name: string, payload: unknown = null) {
      this.record('emit', { target, name, payload });
      for (const item of this.listeners) {
        // Tauri's default is ANY: target on emitter alone is insufficient.
        if (item.name === name && (!item.target || item.target.kind === 'Any' || item.target.label === target)) {
          this.record('deliver', { target, name, receiver: item.receiver, listenerTarget: item.target ?? 'Any' });
          item.handler({ payload });
        }
      }
    },
    status() {
      return { revision: this.revision, bindings: Object.entries(this.settings.hotkeys).map(([action, binding], index) => ({ action, binding, id: index + 1, active: true, reason: null, state: 'active' })) };
    },
    hotkey(action: 'processManager' | 'settings' | 'quickCommands') {
      if (this.lease) { this.capture((this.settings.hotkeys as unknown as Record<string, string>)[action], { action }); return; }
      if (action === 'quickCommands') { this.emit('top-bar', 'quick-commands:toggle'); return; }
      this.emit(action === 'processManager' ? 'bottom-bar' : 'top-bar', action === 'processManager' ? 'process-manager:open-requested' : 'settings:open-requested');
    },
    dismiss(label: 'settings-panel' | 'process-manager') {
      this.record('dismiss', { label, reason: 'mocked-external-blur' });
      this.visible[label] = false;
      if (label === 'settings-panel') {
        this.settingsFocused = false; this.lease = null;
        this.emit(label, 'tauri://focus', false);
      } else {
        const retired = this.context; this.context = { ...retired, visible: false };
        this.emit(label, 'process-manager:closed', { context: retired });
      }
    },
    capture(binding: string, override: Record<string, any> = {}) {
      if (!this.lease) throw new Error('No acknowledged native capture lease');
      this.emit('settings-panel', 'standard-hotkeys:capture', { leaseId: this.lease.leaseId, revision: this.lease.revision, sequence: ++this.lease.sequence, action: 'processManager', binding, ...override });
    }
  };
}

function native(): NativeFixture {
  const host = parent as Window & { __sevenNative?: NativeFixture };
  if (!host.__sevenNative) host.__sevenNative = createNative();
  return host.__sevenNative;
}
const label = () => new URLSearchParams(location.search).get('surface') ?? 'host';
export async function listen(name: string, handler: Listener['handler'], options?: { target?: any }) {
  const state = native(); const item = { name, handler, receiver: label(), target: options?.target };
  state.listeners.add(item); state.record('listen', { name, receiver: item.receiver, target: item.target ?? 'Any' });
  return () => { state.listeners.delete(item); };
}
export async function emitTo(target: string | { label: string }, name: string, payload?: unknown) { native().emit(typeof target === 'string' ? target : target.label, name, payload); }
export async function emit(name: string, payload?: unknown) { native().emit('Any', name, payload); }
export function getCurrentWindow() {
  return { label: label(), listen: (name: string, handler: Listener['handler']) => listen(name, handler, { target: { kind: 'Window', label: label() } }),
     onFocusChanged: (handler: Listener['handler']) => listen('tauri://focus', handler, { target: { kind: 'Window', label: label() } }),
     onDragDropEvent: async (_handler: Listener['handler']) => () => {},
     onResized: async () => () => {}, scaleFactor: async () => 1 };
}
export function getCurrentWebviewWindow() { return getCurrentWindow(); }
export function convertFileSrc(path: string) { return path; }

export async function invoke(command: string, args: Record<string, any> = {}) {
  const n = native(); n.record('invoke', { caller: label(), command, args });
  if (command === 'load_shell_settings') return structuredClone(n.settings);
  if (command === 'get_standard_hotkey_status') { if (!n.settingsFocused) throw new Error('Settings must be visible and focused'); return n.status(); }
  if (command === 'begin_standard_hotkey_capture') {
    n.lease = { leaseId: `browser-lease-${++n.nextLease}`, revision: args.request.revision, sequence: 0, expiresInMs: 2000, registeredBindings: Object.values(n.settings.hotkeys) };
    n.revision++;
    return structuredClone(n.lease);
  }
  if (command === 'renew_standard_hotkey_capture') { if (!n.lease) throw new Error('Expired lease'); return structuredClone(n.lease); }
  if (command === 'end_standard_hotkey_capture') { if (args.request.leaseId === n.lease?.leaseId) { n.lease = null; n.revision++; } return; }
  if (command === 'save_standard_hotkeys') {
    if (args.request.revision !== n.revision) throw new Error('Shortcut status changed; refresh and retry Save');
    if (n.failSave) throw new Error('Shortcut registration failed. Previous shortcuts restored.');
    n.settings.hotkeys = structuredClone(args.request.hotkeys); n.revision++;
    if (n.loseFocusOnSave) { n.loseFocusOnSave = false; n.settingsFocused = false; }
    // Mirrors the trusted committed native snapshot, not the public status
    // command (which rejects the same observation above).
    return { committed: true, settings: structuredClone(n.settings), status: n.statusUnavailableOnSave ? null : n.status(), statusError: n.statusUnavailableOnSave ? 'Shortcuts saved; runtime status unavailable. Refresh shortcut status.' : null };
  }
  if (command === 'save_shell_settings') { n.settings = structuredClone(args.settings); return structuredClone(n.settings); }
  if (command === 'save_shell_bar_lock') { n.settings.ui[args.request.edge === 'top' ? 'lockTopBarHeight' : 'lockBottomBarHeight'] = args.request.locked; return structuredClone(n.settings); }
  if (command === 'show_settings_panel') {
    if (args.request?.toggle && n.visible['settings-panel']) { n.dismiss('settings-panel'); return; }
    n.visible['settings-panel'] = true; n.settingsFocused = true; return;
  }
  if (command === 'hide_settings_panel') { n.dismiss('settings-panel'); return; }
  if (command === 'hide_process_manager') { n.dismiss('process-manager'); return; }
  if (command === 'show_process_manager') {
    if (args.request?.toggle && n.visible['process-manager']) { n.dismiss('process-manager'); return; }
    n.visible['process-manager'] = true;
    const focusPid = args.request?.focusPid ?? null;
    n.context = context(n.context.openGeneration + 1, n.context.tabRevision + 1, 'processes', focusPid);
    n.emit('process-manager', 'process-manager:open', { focusPid, context: n.context }); return;
  }
  if (command === 'get_process_manager_context') return n.context;
  if (command === 'set_process_manager_tab') { n.context = { ...n.context, activeTab: args.tab, tabRevision: args.tabRevision }; return n.context; }
  if (command === 'get_system_info_snapshot') return snapshot(n.context.openGeneration, n.context.tabRevision);
  if (command === 'list_processes') return [processRow(101, 'Fixture process')];
  if (command === 'get_speech_model_status') return { state: 'missing', source: null, error: null };
  if (command === 'get_speech_status') return { state: 'idle' };
  if (command === 'allocate_task_preview_request_id') return ++n.nextPreview;
  if (command.startsWith('list_') || command === 'get_stack_pins') return [];
  if (command === 'search_engine') return { query: args.request?.query ?? '', sequence: args.request?.sequence ?? 0, results: [], providerTimings: [] };
  if (['show_command_panel', 'hide_command_panel', 'hide_audio_panel', 'hide_tray_panel', 'hide_terminal_panel', 'hide_calendar_panel', 'resize_shell_bar', 'report_shell_surface_runtime_metrics', 'request_taskbar_windows_refresh', 'hide_search_panel', 'publish_search_panel', 'hide_task_window_preview', 'hide_task_gallery'].includes(command)) return;
  n.record('unexpected', { caller: label(), command });
  throw new Error(`Unexpected fixture IPC: ${command}`);
}
