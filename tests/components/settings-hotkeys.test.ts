import { fireEvent, render, waitFor, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsPanelSurface from '../../src/components/SettingsPanelSurface.svelte';
import { defaultShellSettings } from '../../src/lib/settings';
import { tauriMocks } from './setup';

vi.mock('@tauri-apps/api/webviewWindow', () => ({ getCurrentWebviewWindow: () => ({ label: 'settings-panel' }) }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ label: 'settings-panel', listen: tauriMocks().listen }) }));

// Production-mounted Settings; only the agreed native boundary is replaced.
const actions = ['search', 'terminal', 'stackBrowser', 'speechTranscription', 'snipping', 'processManager', 'settings'];
let committed: ReturnType<typeof defaultShellSettings>;
let failSave: boolean;
let failRenew: boolean;
let leaseCounter: number;
let lease: ReturnType<typeof captureAck> | null;
let begin: ((args: any) => Promise<any>) | null;
let end: ((args: any) => Promise<any>) | null;
let renew: (() => Promise<any>) | null;
let statusOverride: ReturnType<typeof runtimeStatus> | null;
let statusUnavailableOnSave: boolean;
let runtimeRevision: number;
const handlers = new Map<string, Set<{ handler: (event: any) => void; target: any }>>();

function captureAck(revision: number) {
  return { leaseId: `lease-${++leaseCounter}`, revision, sequence: 0, expiresInMs: 2000, registeredBindings: Object.values(committed.hotkeys) };
}

function nativeCapture(binding: string, override: Record<string, unknown> = {}, targetLabel = 'settings-panel') {
  if (!lease) throw new Error('capture event requires an acknowledged lease');
  const payload = { ...lease, action: 'processManager', binding, sequence: lease.sequence + 1, ...override };
  lease.sequence = Number(payload.sequence);
  for (const entry of handlers.get('standard-hotkeys:capture') ?? []) {
    const target = entry.target;
    // Model Tauri's dangerous global default: any-target listeners receive
    // every event. Explicit foreign listeners cannot receive Settings events.
    if (!target || target.kind === 'Any' || target.label === targetLabel) entry.handler({ payload });
  }
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

function runtimeStatus() {
  return { revision: runtimeRevision, bindings: actions.map((action, index) => ({
    action, id: index + 1, binding: (committed.hotkeys as unknown as Record<string, string>)[action],
    active: true, reason: null, state: 'active'
  })) };
}

beforeEach(() => {
  localStorage.clear();
  committed = defaultShellSettings();
  failSave = false;
  failRenew = false; leaseCounter = 0; lease = null; begin = null; end = null; renew = null; statusOverride = null; statusUnavailableOnSave = false; runtimeRevision = 1; handlers.clear();
  tauriMocks().listen.mockImplementation(async (name, handler, options) => {
    const entries = handlers.get(name) ?? new Set();
    handlers.set(name, entries);
    const entry = { handler, target: options?.target }; entries.add(entry);
    return () => entries.delete(entry);
  });
  tauriMocks().invoke.mockImplementation(async (command: string, args?: Record<string, any>) => {
    if (command === 'load_shell_settings') return structuredClone(committed);
    if (command === 'get_speech_model_status') return { state: 'missing', source: null, error: null };
    if (command === 'get_standard_hotkey_status') return statusOverride ?? runtimeStatus();
    if (command === 'begin_standard_hotkey_capture') {
      lease = captureAck(Number(args?.request?.revision));
      runtimeRevision += 1;
      return begin ? begin(args) : { ...lease };
    }
    if (command === 'renew_standard_hotkey_capture') {
      if (failRenew) throw new Error('Capture expired');
      if (renew) return renew();
      return { ...lease };
    }
    if (command === 'end_standard_hotkey_capture') {
      if (end) await end(args);
      if (args?.request?.leaseId === lease?.leaseId) runtimeRevision += 1;
      return null;
    }
    if (command === 'save_standard_hotkeys') {
      if (args?.request?.revision !== runtimeRevision) throw new Error('Shortcut status changed; refresh and retry Save');
      if (failSave) throw new Error('Shortcut registration failed. Previous shortcuts restored.');
      committed = { ...committed, hotkeys: structuredClone(args?.request?.hotkeys ?? args?.hotkeys) };
      runtimeRevision += 1;
      return { committed: true, settings: structuredClone(committed), status: statusUnavailableOnSave ? null : runtimeStatus(), statusError: statusUnavailableOnSave ? 'Shortcuts saved; runtime status unavailable. Refresh shortcut status.' : null };
    }
    if (command === 'save_shell_settings') {
      committed = structuredClone(args?.settings);
      return committed;
    }
    if (command === 'hide_settings_panel') return null;
    if (command === 'save_shell_bar_lock') return { ...committed, ui: { ...committed.ui, lockTopBarHeight: args?.request?.locked } };
    throw new Error(`Unexpected Settings hotkey IPC: ${command}`);
  });
});

async function mount() {
  const view = render(SettingsPanelSurface);
  const region = within(view.getByRole('region', { name: 'Keyboard shortcuts' }));
  await waitFor(() => expect((region.getByRole('button', { name: 'Capture Search shortcut' }) as HTMLButtonElement).disabled).toBe(false));
  return { view, region };
}

function writes() {
  return tauriMocks().invoke.mock.calls.filter(([command]) => ['save_shell_settings', 'save_standard_hotkeys'].includes(String(command)));
}

async function capture(region: ReturnType<typeof within>, name: string, letter: string) {
  const button = region.getByRole('button', { name: `Capture ${name} shortcut` });
  button.focus();
  await fireEvent.click(button);
  await waitFor(() => expect(tauriMocks().invoke.mock.calls.some(([command]) => command === 'begin_standard_hotkey_capture')).toBe(true));
  await tick();
  await fireEvent.keyDown(button, { key: letter.toLowerCase(), code: `Key${letter}`, altKey: true });
  await waitFor(() => expect(button.textContent).toContain(`Alt+${letter}`));
}

describe('seven-action shortcut draft acceptance', () => {
  it('adds Process Manager and Settings without changing the original five chords', async () => {
    const { region } = await mount();
    const names = ['Search', 'Terminal', 'Stack Browser', 'Speech transcription', 'Screen snipping', 'Process Manager', 'Settings'];
    const chords = ['Ctrl+Space', 'Alt+Backquote', 'Alt+1', 'Ctrl+D', 'Alt+S', 'Alt+P', 'Alt+O'];
    names.forEach((name, index) => expect(region.getByRole('button', { name: `Capture ${name} shortcut` }).textContent).toContain(chords[index]));
    expect(writes()).toHaveLength(0);
  });

  it('stages a new unregistered chord, then Cancel restores committed bindings without writes', async () => {
    const { region } = await mount();
    await capture(region, 'Search', 'X');
    expect(writes()).toHaveLength(0);
    await fireEvent.click(region.getByRole('button', { name: /cancel/i }));
    expect(region.getByRole('button', { name: 'Capture Search shortcut' }).textContent).toContain('Ctrl+Space');
    expect(writes()).toHaveLength(0);
  });

  it('Reset stages all seven defaults and Cancel restores a customized binding', async () => {
    committed.hotkeys.search = 'Alt+X';
    const { region } = await mount();
    await fireEvent.click(region.getByRole('button', { name: 'Reset shortcuts to defaults' }));
    expect(region.getByRole('button', { name: 'Capture Search shortcut' }).textContent).toContain('Ctrl+Space');
    expect(writes()).toHaveLength(0);
    await fireEvent.click(region.getByRole('button', { name: /cancel/i }));
    expect(region.getByRole('button', { name: 'Capture Search shortcut' }).textContent).toContain('Alt+X');
  });

  it('explicit Save sends the entire section once through the strict shortcut command', async () => {
    const { region } = await mount();
    await capture(region, 'Search', 'X');
    await fireEvent.click(region.getByRole('button', { name: /^save(?: shortcuts)?$/i }));
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect(writes()[0][0]).toBe('save_standard_hotkeys');
    const args = writes()[0][1] as any;
    expect(args.request?.hotkeys ?? args.hotkeys).toEqual({ ...defaultShellSettings().hotkeys, search: 'Alt+X' });
  });

  it('failed explicit Save retains edits and permits retry without falsely committing', async () => {
    const { region } = await mount();
    await capture(region, 'Search', 'X');
    failSave = true;
    await fireEvent.click(region.getByRole('button', { name: /^save(?: shortcuts)?$/i }));
    await waitFor(() => expect(region.getByRole('alert').textContent).toMatch(/registration failed/i));
    expect(region.getByRole('button', { name: 'Capture Search shortcut' }).textContent).toContain('Alt+X');
    expect(committed.hotkeys.search).toBe('Ctrl+Space');
    failSave = false;
    await fireEvent.click(region.getByRole('button', { name: /^save(?: shortcuts)?$/i }));
    await waitFor(() => expect(committed.hotkeys.search).toBe('Alt+X'));
  });

  it('registered chords ignore local keydown and change draft only after targeted native capture', async () => {
    const { region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    search.focus(); await fireEvent.click(search); await tick();
    await fireEvent.keyDown(search, { key: 'p', code: 'KeyP', altKey: true });
    expect(search.textContent).toContain('Ctrl+Space');
    nativeCapture('Alt+P'); await tick();
    expect(search.textContent).toContain('Alt+P');
    expect(writes()).toHaveLength(0);
    const captureListener = tauriMocks().listen.mock.calls.find(([name]) => name === 'standard-hotkeys:capture');
    expect(captureListener?.[2]).toEqual({ target: { kind: 'WebviewWindow', label: 'settings-panel' } });
  });

  it('native ACK registeredBindings is authoritative, including chords not present in configured settings', async () => {
    begin = async () => ({ ...lease!, registeredBindings: ['Alt+X'] });
    const { region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    search.focus(); await fireEvent.click(search); await tick();
    await fireEvent.keyDown(search, { key: 'x', code: 'KeyX', altKey: true });
    expect(search.textContent).toContain('Ctrl+Space');
    nativeCapture('Alt+X'); await tick(); expect(search.textContent).toContain('Alt+X');
    // Alt+P remains configured but native ACK says it is NOT registered.
    search.focus(); await fireEvent.click(search); await tick();
    await waitFor(() => expect(region.getByText('Listening for Search shortcut…')).toBeTruthy());
    await fireEvent.keyDown(search, { key: 'p', code: 'KeyP', altKey: true }); await tick();
    expect(search.textContent).toContain('Alt+P'); expect(writes()).toHaveLength(0);
  });

  it('foreign-target capture event cannot populate the Settings draft', async () => {
    const { region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    search.focus(); await fireEvent.click(search); await tick();
    nativeCapture('Alt+P', {}, 'process-manager'); await tick();
    expect(search.textContent).toContain('Ctrl+Space'); expect(writes()).toHaveLength(0);
  });

  it('temporary duplicate draft permits a two-way swap, but cannot Save an unresolved duplicate', async () => {
    const { region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    search.focus(); await fireEvent.click(search); await tick(); nativeCapture('Alt+P'); await tick();
    expect(search.textContent).toContain('Alt+P');
    const save = region.getByRole('button', { name: /^save(?: shortcuts)?$/i }) as HTMLButtonElement;
    await fireEvent.click(save); expect(writes()).toHaveLength(0);
    const process = region.getByRole('button', { name: 'Capture Process Manager shortcut' });
    process.focus(); await fireEvent.click(process); await tick(); nativeCapture('Ctrl+Space', { action: 'search' }); await tick();
    expect(process.textContent).toContain('Ctrl+Space');
    await fireEvent.click(save);
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect(committed.hotkeys.search).toBe('Alt+P');
    expect((committed.hotkeys as unknown as Record<string, string>).processManager).toBe('Ctrl+Space');
  });

  it('unrelated bar control persists committed bindings and retains shortcut draft', async () => {
    const { view, region } = await mount();
    await capture(region, 'Search', 'X');
    const toggle = view.getByRole('button', { name: 'Lock top bar', exact: true });
    await fireEvent.click(toggle);
    await waitFor(() => expect(tauriMocks().invoke.mock.calls.some(([command]) => command === 'save_shell_bar_lock')).toBe(true));
    expect(committed.hotkeys.search).toBe('Ctrl+Space');
    expect(region.getByRole('button', { name: 'Capture Search shortcut' }).textContent).toContain('Alt+X');
    expect(writes()).toHaveLength(0);
  });

  it('ignores local chords before begin ACK and ends a late ACK after disposal', async () => {
    const pending = deferred<any>(); begin = () => pending.promise;
    const { view, region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    search.focus(); await fireEvent.click(search);
    await fireEvent.keyDown(search, { key: 'x', code: 'KeyX', altKey: true });
    expect(search.textContent).toContain('Ctrl+Space'); expect(writes()).toHaveLength(0);
    const acknowledged = { ...lease! };
    view.unmount(); pending.resolve(acknowledged); await pending.promise; await tick();
    await waitFor(() => expect(tauriMocks().invoke).toHaveBeenCalledWith('end_standard_hotkey_capture', {
      request: { leaseId: acknowledged.leaseId, revision: acknowledged.revision }
    }));
  });

  it('stale begin ACK releases its returned lease without ending a newer capture lease', async () => {
    const pending = deferred<any>(); let firstAck: any; let requests = 0;
    begin = async () => { if (++requests === 1) { firstAck = { ...lease! }; return pending.promise; } return { ...lease! }; };
    const { region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    const terminal = region.getByRole('button', { name: 'Capture Terminal shortcut' });
    search.focus(); await fireEvent.click(search); await tick();
    terminal.focus(); await fireEvent.click(terminal); await tick();
    const newer = { ...lease! }; expect(newer.leaseId).not.toBe(firstAck.leaseId);
    pending.resolve(firstAck); await pending.promise; await tick();
    await waitFor(() => expect(tauriMocks().invoke).toHaveBeenCalledWith('end_standard_hotkey_capture', {
      request: { leaseId: firstAck.leaseId, revision: firstAck.revision }
    }));
    expect(tauriMocks().invoke.mock.calls.filter(([command, args]) => command === 'end_standard_hotkey_capture' && (args as any)?.request?.leaseId === newer.leaseId)).toHaveLength(0);
    nativeCapture('Alt+P'); await tick(); expect(terminal.textContent).toContain('Alt+P');
    expect(search.textContent).toContain('Ctrl+Space');
  });

  it.each(['rejected stale begin', 'failed stale lease release'] as const)('%s cannot cancel newer ready capture or overwrite its feedback', async (failure) => {
    const pending = deferred<any>(); let firstAck: any; let requests = 0;
    begin = async () => { if (++requests === 1) { firstAck = { ...lease! }; return pending.promise; } return { ...lease! }; };
    const { region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    const terminal = region.getByRole('button', { name: 'Capture Terminal shortcut' });
    search.focus(); await fireEvent.click(search); await tick();
    terminal.focus(); await fireEvent.click(terminal);
    await waitFor(() => expect(region.getByText('Listening for Terminal shortcut…')).toBeTruthy());
    const newer = { ...lease! };
    if (failure === 'rejected stale begin') pending.reject(new Error('old begin rejected'));
    else {
      end = async (args) => { if (args.request.leaseId === firstAck.leaseId) throw new Error('old lease already retired'); };
      pending.resolve(firstAck);
    }
    await pending.promise.catch(() => undefined); await tick(); await tick();
    expect(region.getByText('Listening for Terminal shortcut…')).toBeTruthy();
    expect(region.queryByRole('alert')).toBeNull();
    expect(tauriMocks().invoke.mock.calls.filter(([command, args]) => command === 'end_standard_hotkey_capture' && (args as any)?.request?.leaseId === newer.leaseId)).toHaveLength(0);
    nativeCapture('Alt+P'); await tick(); expect(terminal.textContent).toContain('Alt+P');
    expect(search.textContent).toContain('Ctrl+Space');
  });

  it('Save waits for pending native release and refetches its new status revision before strict persistence', async () => {
    const pending = deferred<void>(); end = () => pending.promise;
    const { region } = await mount();
    await capture(region, 'Search', 'X');
    const before = runtimeRevision;
    await fireEvent.click(region.getByRole('button', { name: /^save(?: shortcuts)?$/i }));
    await tick(); expect(writes()).toHaveLength(0);
    pending.resolve(); await pending.promise;
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect((writes()[0][1] as any).request.revision).toBe(before + 1);
    expect(committed.hotkeys.search).toBe('Alt+X');
    const trace = tauriMocks().invoke.mock.calls;
    const saveIndex = trace.findIndex(([command]) => command === 'save_standard_hotkeys');
    expect(trace.slice(0, saveIndex).at(-1)?.[0]).toBe('get_standard_hotkey_status');
  });

  it.each(['stale ACK', 'invalid ACK'] as const)('Save waits for outstanding %s lease release before status refresh and strict Save', async (kind) => {
    const late = deferred<any>(); const release = deferred<void>(); let firstAck: any; let requests = 0;
    begin = async () => { if (++requests === 1) { firstAck = { ...lease! }; return late.promise; } return { ...lease! }; };
    end = async args => { if (args.request.leaseId === firstAck?.leaseId) await release.promise; };
    const { region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    search.focus(); await fireEvent.click(search); await tick();
    if (kind === 'stale ACK') {
      const terminal = region.getByRole('button', { name: 'Capture Terminal shortcut' });
      terminal.focus(); await fireEvent.click(terminal);
      await waitFor(() => expect(region.getByText('Listening for Terminal shortcut…')).toBeTruthy());
      await fireEvent.keyDown(terminal, { key: 'x', code: 'KeyX', altKey: true });
      await waitFor(() => expect(terminal.textContent).toContain('Alt+X'));
      late.resolve(firstAck);
    } else late.resolve({ ...firstAck, expiresInMs: 0 });
    await late.promise;
    await waitFor(() => expect(tauriMocks().invoke).toHaveBeenCalledWith('end_standard_hotkey_capture', { request: { leaseId: firstAck.leaseId, revision: firstAck.revision } }));
    const statusCalls = tauriMocks().invoke.mock.calls.filter(([command]) => command === 'get_standard_hotkey_status').length;
    await fireEvent.click(region.getByRole('button', { name: /^save(?: shortcuts)?$/i }));
    await tick(); await tick();
    expect(writes()).toHaveLength(0);
    expect(tauriMocks().invoke.mock.calls.filter(([command]) => command === 'get_standard_hotkey_status')).toHaveLength(statusCalls);
    release.resolve(); await release.promise;
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect((writes()[0][1] as any).request.revision).toBe(runtimeRevision - 1);
  });

  it('rejects stale lease/revision/sequence and does not replay native capture as opening actions', async () => {
    const { region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    search.focus(); await fireEvent.click(search); await tick();
    nativeCapture('Alt+P', { leaseId: 'retired', sequence: 1 });
    nativeCapture('Alt+P', { revision: lease!.revision - 1, sequence: 1 });
    nativeCapture('Alt+P', { sequence: 0 }); await tick();
    expect(search.textContent).toContain('Ctrl+Space');
    nativeCapture('Alt+P', { sequence: 1 }); await tick();
    expect(search.textContent).toContain('Alt+P');
    nativeCapture('Alt+O', { sequence: 1 }); await tick();
    expect(search.textContent).toContain('Alt+P');
    expect(tauriMocks().emitTo).not.toHaveBeenCalled(); expect(writes()).toHaveLength(0);
  });

  it.each(['blur', 'hide', 'unmount'] as const)('%s ends capture; queued native events cannot mutate committed bindings', async (cleanup) => {
    const { view, region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    search.focus(); await fireEvent.click(search); await tick(); const acknowledged = { ...lease! };
    if (cleanup === 'blur') await fireEvent(window, new Event('blur'));
    if (cleanup === 'hide') await fireEvent.click(view.getByRole('button', { name: 'Close settings' }));
    if (cleanup === 'unmount') view.unmount();
    await waitFor(() => expect(tauriMocks().invoke).toHaveBeenCalledWith('end_standard_hotkey_capture', {
      request: { leaseId: acknowledged.leaseId, revision: acknowledged.revision }
    }));
    nativeCapture('Alt+P'); await tick();
    if (cleanup !== 'unmount') expect(search.textContent).toContain('Ctrl+Space');
    expect(committed.hotkeys.search).toBe('Ctrl+Space'); expect(writes()).toHaveLength(0);
  });

  it('held/repeated local keys cannot fill a draft or persist shortcuts', async () => {
    const { region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    search.focus(); await fireEvent.click(search); await tick();
    await fireEvent.keyDown(search, { key: 'Alt', code: 'AltLeft', altKey: true });
    await fireEvent.keyDown(search, { key: 'x', code: 'KeyX', altKey: true, repeat: true });
    expect(search.textContent).toContain('Ctrl+Space'); expect(writes()).toHaveLength(0);
  });

  it('renews every 500ms and failed renewal disarms local and queued native capture', async () => {
    const { region } = await mount();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    search.focus(); await fireEvent.click(search); await tick();
    failRenew = true;
    await waitFor(() => expect(tauriMocks().invoke.mock.calls.some(([command]) => command === 'renew_standard_hotkey_capture')).toBe(true), { timeout: 1500 });
    await tick();
    await fireEvent.keyDown(search, { key: 'x', code: 'KeyX', altKey: true }); nativeCapture('Alt+P'); await tick();
    expect(search.textContent).toContain('Ctrl+Space'); expect(writes()).toHaveLength(0);
  });

  it('renderer deadline disarms capture even while a renewal response is indefinitely pending', async () => {
    const pending = deferred<any>(); renew = () => pending.promise;
    const { view, region } = await mount(); vi.useFakeTimers();
    const search = region.getByRole('button', { name: 'Capture Search shortcut' });
    search.focus(); await fireEvent.click(search); await tick(); await vi.advanceTimersByTimeAsync(0);
    const acknowledged = { ...lease! };
    await vi.advanceTimersByTimeAsync(2000); await tick();
    expect(region.getByRole('alert').textContent).toMatch(/expired/i);
    await fireEvent.keyDown(search, { key: 'x', code: 'KeyX', altKey: true }); nativeCapture('Alt+P'); await tick();
    expect(search.textContent).toContain('Ctrl+Space'); expect(writes()).toHaveLength(0);
    view.unmount(); pending.resolve(acknowledged); await pending.promise; await tick();
    const before = tauriMocks().invoke.mock.calls.length; await vi.advanceTimersByTimeAsync(3000);
    expect(tauriMocks().invoke.mock.calls.length).toBe(before);
  });

  it('renders native inactive reason without remapping the configured chord or treating capturePaused as an outage', async () => {
    const status = runtimeStatus();
    status.bindings[5] = { ...status.bindings[5], active: false, reason: 'Another app owns Alt+P', state: 'inactive' };
    status.bindings[0] = { ...status.bindings[0], state: 'capturePaused' };
    statusOverride = status;
    const { region } = await mount();
    expect(region.getByText(/Process Manager \(Alt\+P\) inactive: Another app owns Alt\+P/)).toBeTruthy();
    expect(region.queryByText(/Search .*inactive:/)).toBeNull();
    expect(region.getByRole('button', { name: 'Capture Process Manager shortcut' }).textContent).toContain('Alt+P');
    expect(writes()).toHaveLength(0);
  });

  it('committed Save with unavailable native status commits the draft and reports saved warning, never a failed Save', async () => {
    const { region } = await mount(); await capture(region, 'Search', 'X'); statusUnavailableOnSave = true;
    await fireEvent.click(region.getByRole('button', { name: /^save(?: shortcuts)?$/i }));
    await waitFor(() => expect(region.getByText('Shortcuts saved; runtime status unavailable. Refresh shortcut status.')).toBeTruthy());
    expect(region.queryByRole('alert')).toBeNull(); expect(committed.hotkeys.search).toBe('Alt+X');
    await fireEvent.click(region.getByRole('button', { name: 'Reset shortcuts to defaults' }));
    await fireEvent.click(region.getByRole('button', { name: /cancel/i }));
    expect(region.getByRole('button', { name: 'Capture Search shortcut' }).textContent).toContain('Alt+X');
    expect(writes()).toHaveLength(1);
  });

  it('unavailable postcommit status clears obsolete inactive claims instead of presenting them as current runtime truth', async () => {
    committed.hotkeys.search = 'Alt+X';
    const old = runtimeStatus(); old.bindings[5] = { ...old.bindings[5], active: false, state: 'inactive', reason: 'Retired conflict observation' };
    statusOverride = old; statusUnavailableOnSave = true;
    const { region } = await mount(); expect(region.getByText(/Retired conflict observation/)).toBeTruthy();
    await fireEvent.click(region.getByRole('button', { name: 'Reset shortcuts to defaults' }));
    await fireEvent.click(region.getByRole('button', { name: /^save(?: shortcuts)?$/i }));
    await waitFor(() => expect(region.getByText('Shortcuts saved; runtime status unavailable. Refresh shortcut status.')).toBeTruthy());
    expect(region.queryByText(/Retired conflict observation/)).toBeNull();
    expect(committed.hotkeys.search).toBe('Ctrl+Space'); expect(region.queryByRole('alert')).toBeNull();
  });
});
