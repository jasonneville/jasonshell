import { cleanup, fireEvent, render, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import TerminalPanelSurface from '../../src/components/TerminalPanelSurface.svelte';
import type { StackTerminalSession } from '../../src/lib/persistentTerminal';
import { tauriMocks } from './setup';

// A DOM-owning xterm double: attachment/replay/focus exercise the real pane actions.
const xterms = vi.hoisted(() => ({ instances: [] as any[] }));
vi.mock('@xterm/xterm', () => ({ Terminal: class {
  element: HTMLDivElement | null = null;
  options: Record<string, unknown>;
  cols = 80;
  rows = 24;
  buffer = { active: { baseY: 0, cursorY: 0, cursorX: 0 } };
  parser = { registerOscHandler: vi.fn(() => ({ dispose: vi.fn() })) };
  data: (value: string) => void = () => {};
  write = vi.fn((value: string) => { if (this.element) this.element.textContent += value; });
  focus = vi.fn(() => this.element?.focus());
  dispose = vi.fn(() => this.element?.remove());
  constructor(options: Record<string, unknown>) { this.options = options; xterms.instances.push(this); }
  open(host: HTMLElement) { this.element = document.createElement('div'); this.element.className = 'xterm'; this.element.tabIndex = -1; host.append(this.element); }
  loadAddon() {}
  onData(callback: (value: string) => void) { this.data = callback; return { dispose: vi.fn() }; }
  attachCustomKeyEventHandler() {}
  hasSelection() { return false; }
  getSelection() { return ''; }
} }));
vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit = vi.fn(); dispose = vi.fn(); } }));
vi.mock('@xterm/addon-search', () => ({ SearchAddon: class { dispose = vi.fn(); } }));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}
const makeSession = (sessionId = 'session-1'): StackTerminalSession => ({ sessionId, cwd: 'C:\\dev', profile: 'powershell', title: 'PowerShell', running: true });
const listeners = new Map<string, (event: { payload: any }) => void>();
const unlisteners: ReturnType<typeof vi.fn>[] = [];
let sessions: StackTerminalSession[];
let nextSessionNumber: number;
let start: () => Promise<StackTerminalSession>;
let read: (sessionId: string) => Promise<unknown>;
let resize: () => Promise<unknown>;
let stop: (sessionId: string) => Promise<void>;

// Promise/tick drain only: no sleep, real backend, or native shell.
async function settle() { for (let i = 0; i < 40; i += 1) { await Promise.resolve(); await tick(); } }
async function event(name: string, payload: unknown = {}) {
  expect(listeners.has(name)).toBe(true);
  listeners.get(name)!({ payload });
  await settle();
}
async function advance(ms: number) { await vi.advanceTimersByTimeAsync(ms); await settle(); }
async function open() {
  const view = render(TerminalPanelSurface);
  await settle();
  await event('terminal-panel:open');
  return view;
}
function noStartupNotice(view: ReturnType<typeof render>) {
  expect(view.queryAllByRole('alert')).toHaveLength(0);
  expect(view.queryAllByRole('status')).toHaveLength(0);
  expect(view.container.textContent).not.toMatch(/starting terminal|prewarming|attaching terminal|waiting for terminal output|idle prewarm|starts with the app/i);
}
async function action(view: ReturnType<typeof render>, name: string) {
  await fireEvent.click(view.getByRole('button', { name: 'Open terminal actions menu' }));
  await fireEvent.click(view.getByRole('menuitem', { name, exact: true }));
  await settle();
}

beforeEach(() => {
  vi.useFakeTimers();
  xterms.instances.length = 0;
  listeners.clear();
  unlisteners.length = 0;
  sessions = [];
  nextSessionNumber = 1;
  start = async () => { const session = makeSession(`session-${nextSessionNumber++}`); sessions.push(session); return session; };
  read = async (sessionId) => ({ sessionId, cwd: 'C:\\dev', chunks: [], output: '', exited: false });
  resize = async () => undefined;
  stop = async () => undefined;
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(480);
  vi.spyOn(console, 'error').mockImplementation(() => {});
  const tauri = tauriMocks();
  tauri.listen.mockImplementation(async (name: string, callback: (event: { payload: any }) => void) => {
    listeners.set(name, callback);
    const unlisten = vi.fn(() => listeners.delete(name));
    unlisteners.push(unlisten);
    return unlisten;
  });
  tauri.invoke.mockImplementation(async (command: string, args?: any) => {
    if (command === 'list_stack_terminals') return [...sessions];
    if (command === 'start_persistent_terminal' || command === 'start_stack_terminal') return start();
    if (command === 'read_stack_terminal') return read(args.sessionId);
    if (command === 'resize_stack_terminal') return resize();
    if (command === 'stop_stack_terminal') {
      await stop(args.sessionId);
      sessions = sessions.filter((session) => session.sessionId !== args.sessionId);
      return;
    }
    if (command === 'write_stack_terminal') return;
    throw new Error(`Unexpected terminal test IPC: ${command}`);
  });
});
afterEach(() => { cleanup(); vi.clearAllTimers(); vi.unstubAllGlobals(); });

describe('persistent terminal quiet startup presentation', () => {
  it('stays quiet on scheduled mount and slow first-open startup without inventing a running pane', async () => {
    const pending = deferred<StackTerminalSession>();
    start = () => pending.promise;
    const view = render(TerminalPanelSurface);
    await settle();
    noStartupNotice(view);
    expect(tauriMocks().invoke).not.toHaveBeenCalled();
    await event('terminal-panel:open');
    await advance(6200);
    noStartupNotice(view);
    expect(view.queryByRole('log')).toBeNull();
    expect(tauriMocks().invoke.mock.calls.filter(([name]) => name === 'start_persistent_terminal')).toHaveLength(1);
    sessions = [makeSession()];
    pending.resolve(sessions[0]);
    await settle();
  });

  it('silently attaches an empty session and stays quiet beyond the 1200ms waiting timer', async () => {
    const view = await open();
    expect(view.getByRole('log', { name: 'Terminal output' }).querySelector('.xterm')).not.toBeNull();
    await advance(1201);
    noStartupNotice(view);
    expect(view.container.querySelector('.terminal-tab-status')?.textContent).toBe('●');
  });

  it.each(['', ' \r\n\t', '\x1b[2J\x1b[H\x1b[?25h', 'PS C:\\dev> '])('does not show startup text for output %j', async (text) => {
    const view = await open();
    await event('stack-terminal:output', { sessionId: 'session-1', sequence: 1, text });
    await advance(1201);
    if (text) expect(xterms.instances[0].write).toHaveBeenCalledWith(text);
    noStartupNotice(view);
  });

  it('joins hidden idle prewarm on repeated open/focus, then reuses the session', async () => {
    const pending = deferred<StackTerminalSession>();
    start = () => pending.promise;
    const view = render(TerminalPanelSurface);
    await settle();
    await advance(4999);
    expect(tauriMocks().invoke).not.toHaveBeenCalled();
    await advance(1);
    await event('terminal-panel:open');
    window.dispatchEvent(new Event('focus'));
    await settle();
    pending.resolve(makeSession());
    sessions = [makeSession()];
    await settle();
    await event('terminal-panel:open');
    await advance(1201);
    noStartupNotice(view);
    expect(tauriMocks().invoke.mock.calls.filter(([name]) => name === 'start_persistent_terminal')).toHaveLength(1);
    expect(xterms.instances.at(-1).focus).toHaveBeenCalled();
  });

  it('reattaches an existing backend session rather than creating another', async () => {
    sessions = [makeSession('existing')];
    const view = await open();
    await advance(1201);
    noStartupNotice(view);
    expect(tauriMocks().invoke.mock.calls.some(([name]) => name === 'start_persistent_terminal')).toBe(false);
    expect(tauriMocks().invoke).toHaveBeenCalledWith('read_stack_terminal', { sessionId: 'existing' });
  });
});

describe('persistent terminal terminal-state notices', () => {
  it('shows a startup exception on the already-created pane without a duplicate fallback notice', async () => {
    const schedule = window.setTimeout.bind(window);
    let startupSchedules = 0;
    vi.spyOn(window, 'setTimeout').mockImplementation(((handler: TimerHandler, timeout?: number, ...args: unknown[]) => {
      // First 1200ms timer precedes startup; the second is scheduled after the
      // primary runtime exists, inside startTerminalOnce's guarded startup.
      if (timeout === 1200 && ++startupSchedules === 2) throw new Error('Pane startup timer unavailable');
      return schedule(handler, timeout, ...args);
    }) as typeof window.setTimeout);
    const view = await open();
    expect(startupSchedules).toBe(2);
    const pane = view.getByRole('log').closest('section')!;
    expect(view.getAllByRole('alert')).toHaveLength(1);
    expect(pane.contains(view.getByRole('alert'))).toBe(true);
    expect(view.getByRole('alert').textContent).toContain('Pane startup timer unavailable');
    await event('terminal-panel:open');
    await advance(1201);
    expect(view.getAllByRole('alert')).toHaveLength(1);
    expect(view.getByRole('alert').textContent).toContain('Pane startup timer unavailable');
  });

  it.each(['ConPTY launch denied', new Error('PowerShell launch denied')])('shows startup rejection before a pane exists: %s', async (error) => {
    start = async () => { throw error; };
    const view = await open();
    expect(view.queryByRole('log')).toBeNull();
    const message = error instanceof Error ? error.message : error;
    expect(view.getAllByRole('alert')).toHaveLength(1);
    expect(view.getByRole('alert').textContent).toContain(message);
    await advance(6200);
    expect(view.getByRole('alert').textContent).toContain(message);
  });

  it('clears pre-pane failure on retry immediately and does not duplicate notices', async () => {
    start = async () => { throw new Error('Launch unavailable'); };
    const view = await open();
    expect(view.getAllByRole('alert')).toHaveLength(1);
    const pending = deferred<StackTerminalSession>();
    start = () => pending.promise;
    await action(view, 'Restart terminal');
    noStartupNotice(view);
    sessions = [makeSession()];
    pending.resolve(sessions[0]);
    await settle();
    await advance(1201);
    noStartupNotice(view);
    await event('stack-terminal:closed', { sessionId: 'session-1' });
    expect(view.getAllByRole('alert')).toHaveLength(1);
  });

  it.each(['event', 'poll'])('shows exit before output through %s', async (transport) => {
    const view = await open();
    if (transport === 'event') await event('stack-terminal:closed', { sessionId: 'session-1' });
    else { read = async (sessionId) => ({ sessionId, chunks: [], exited: true }); await advance(1000); }
    expect(view.getAllByRole('alert')).toHaveLength(1);
    expect(view.getByRole('alert').textContent).toContain('Terminal exited before output');
    await advance(1201);
    expect(view.getByRole('alert').textContent).toContain('Terminal exited before output');
  });

  it.each(['event', 'poll', 'read-error'])('keeps one truthful notice after prior output through %s', async (transport) => {
    const view = await open();
    await event('stack-terminal:output', { sessionId: 'session-1', sequence: 1, text: 'retained output' });
    expect(view.getByRole('log').textContent).toContain('retained output');
    if (transport === 'event') await event('stack-terminal:closed', { sessionId: 'session-1' });
    else {
      read = transport === 'read-error' ? async () => { throw new Error('PTY read failed'); }
        : async (sessionId) => ({ sessionId, chunks: [], exited: true });
      await advance(1000);
    }
    const notices = [...view.queryAllByRole('alert'), ...view.queryAllByRole('status')];
    expect(notices).toHaveLength(1);
    expect(notices[0].textContent).toContain(transport === 'read-error' ? 'PTY read failed' : 'Terminal exited');
    expect(view.getByRole('log').textContent).toContain('retained output');
  });

  it('shows a read failure without prior output and clears it on pane restart', async () => {
    read = async () => { throw new Error('PTY unavailable'); };
    const view = await open();
    expect(view.getAllByRole('alert')).toHaveLength(1);
    expect(view.getByRole('alert').textContent).toContain('PTY unavailable');
    read = async (sessionId) => ({ sessionId, chunks: [], exited: false });
    await action(view, 'Restart terminal');
    await advance(1201);
    noStartupNotice(view);
    expect(view.getAllByRole('log')).toHaveLength(1);
  });

  it('shows stopped state after stopping the final tab without creating a replacement', async () => {
    const view = await open();
    await event('stack-terminal:output', { sessionId: 'session-1', sequence: 1, text: 'previous output' });
    const startsBefore = tauriMocks().invoke.mock.calls.filter(([name]) => name === 'start_persistent_terminal' || name === 'start_stack_terminal').length;
    await action(view, 'Stop terminal');
    expect(view.queryAllByRole('tab')).toHaveLength(0);
    expect(view.queryAllByRole('log')).toHaveLength(0);
    expect(view.getAllByRole('status')).toHaveLength(1);
    expect(view.getByRole('status').textContent).toContain('Terminal stopped');
    expect(tauriMocks().invoke.mock.calls.filter(([name]) => name === 'start_persistent_terminal' || name === 'start_stack_terminal')).toHaveLength(startsBefore);
  });

  it('does not announce final Stop success while backend confirmation is pending, then preserves confirmed stopped status', async () => {
    const view = await open();
    const pending = deferred<void>();
    stop = () => pending.promise;
    const startsBefore = tauriMocks().invoke.mock.calls.filter(([name]) => name === 'start_persistent_terminal' || name === 'start_stack_terminal').length;
    try {
      await action(view, 'Stop terminal');
      // Closing the final tab must remain immediate, not await backend teardown.
      expect(view.queryAllByRole('tab')).toHaveLength(0);
      expect(view.queryAllByRole('log')).toHaveLength(0);
      expect(tauriMocks().invoke.mock.calls.filter(([name]) => name === 'stop_stack_terminal')).toEqual([
        ['stop_stack_terminal', { sessionId: 'session-1' }]
      ]);
      expect(view.queryByText('Terminal stopped', { exact: true })).toBeNull();
      expect(view.queryAllByRole('alert')).toHaveLength(0);
      await advance(1201);
      expect(view.queryByText('Terminal stopped', { exact: true })).toBeNull();
      expect(tauriMocks().invoke.mock.calls.filter(([name]) => name === 'start_persistent_terminal' || name === 'start_stack_terminal')).toHaveLength(startsBefore);
    } finally {
      pending.resolve();
      await settle();
    }
    expect(view.getAllByRole('status')).toHaveLength(1);
    expect(view.getByRole('status').textContent).toContain('Terminal stopped');
    expect(view.queryAllByRole('alert')).toHaveLength(0);
    await advance(1201);
    expect(view.getByRole('status').textContent).toContain('Terminal stopped');
  });

  it('reports rejected final Stop as a real failure alert, not stopped success or a duplicate session', async () => {
    const view = await open();
    const pending = deferred<void>();
    stop = () => pending.promise;
    const startsBefore = tauriMocks().invoke.mock.calls.filter(([name]) => name === 'start_persistent_terminal' || name === 'start_stack_terminal').length;
    try {
      await action(view, 'Stop terminal');
      expect(view.queryAllByRole('tab')).toHaveLength(0);
      expect(view.queryAllByRole('log')).toHaveLength(0);
    } finally {
      pending.reject(new Error('ConPTY stop denied'));
      await settle();
    }
    expect(view.getAllByRole('alert')).toHaveLength(1);
    expect(view.getByRole('alert').textContent).toContain('ConPTY stop denied');
    expect(view.queryByText('Terminal stopped', { exact: true })).toBeNull();
    expect(view.queryAllByRole('status')).toHaveLength(0);
    await advance(1201);
    expect(view.getByRole('alert').textContent).toContain('ConPTY stop denied');
    expect(tauriMocks().invoke.mock.calls.filter(([name]) => name === 'stop_stack_terminal')).toEqual([
      ['stop_stack_terminal', { sessionId: 'session-1' }]
    ]);
    expect(tauriMocks().invoke.mock.calls.filter(([name]) => name === 'start_persistent_terminal' || name === 'start_stack_terminal')).toHaveLength(startsBefore);
  });

  it.each([
    ['Restart', 'confirmed'], ['Restart', 'rejected'],
    ['New tab', 'confirmed'], ['New tab', 'rejected']
  ])('keeps %s replacement live after stale Stop %s settlement', async (intent, outcome) => {
    const view = await open();
    const pending = deferred<void>();
    stop = () => pending.promise;
    let replacementLog!: HTMLElement;
    let replacementXterm!: Element | null;
    try {
      await action(view, 'Stop terminal');
      expect(view.queryAllByRole('tab')).toHaveLength(0);
      expect(view.queryAllByRole('log')).toHaveLength(0);
      // Model a backend list snapshot omitting the session already being closed;
      // otherwise Restart's normal reuse policy would reattach that old session.
      sessions = [];
      if (intent === 'Restart') await action(view, 'Restart terminal');
      else {
        await fireEvent.click(view.getByRole('button', { name: 'New terminal tab' }));
        await settle();
      }
      expect(view.getAllByRole('tab')).toHaveLength(1);
      replacementLog = view.getByRole('log');
      replacementXterm = replacementLog.querySelector('.xterm');
      expect(replacementXterm).not.toBeNull();
      await event('stack-terminal:output', { sessionId: 'session-2', sequence: 1, text: 'replacement remains live' });
      noStartupNotice(view);
    } finally {
      if (outcome === 'rejected') pending.reject(new Error('Stale ConPTY stop denied'));
      else pending.resolve();
      await settle();
    }
    await advance(1201);
    noStartupNotice(view);
    expect(view.queryByText('Terminal stopped', { exact: true })).toBeNull();
    expect(view.queryByText('Stale ConPTY stop denied', { exact: true })).toBeNull();
    expect(view.getAllByRole('tab')).toHaveLength(1);
    expect(view.getByRole('log')).toBe(replacementLog);
    expect(replacementLog.querySelector('.xterm')).toBe(replacementXterm);
    expect(replacementLog.textContent).toContain('replacement remains live');
    const replacementTerminal = xterms.instances.find((terminal) => terminal.element === replacementXterm);
    expect(replacementTerminal).toBeDefined();
    replacementTerminal.data('echo replacement\r');
    await settle();
    expect(tauriMocks().invoke).toHaveBeenCalledWith('write_stack_terminal', { sessionId: 'session-2', input: 'echo replacement\r' });
    expect(tauriMocks().invoke.mock.calls.filter(([name]) => name === 'stop_stack_terminal')).toEqual([
      ['stop_stack_terminal', { sessionId: 'session-1' }]
    ]);
    expect(tauriMocks().invoke.mock.calls.filter(([name]) => name === 'start_persistent_terminal' || name === 'start_stack_terminal')).toHaveLength(2);
  });
});

describe('persistent terminal presentation preserves workbench contracts', () => {
  it.each(['exit', 'read-error'])('scopes split %s to its pane and preserves the notice on reopen', async (failure) => {
    const view = await open();
    await event('stack-terminal:output', { sessionId: 'session-1', sequence: 1, text: 'healthy primary output' });
    await fireEvent.click(view.getByRole('button', { name: 'Split terminal pane right' }));
    await settle();
    await event('stack-terminal:output', { sessionId: 'session-2', sequence: 1, text: 'secondary output' });
    const [primaryLog, secondaryLog] = view.getAllByRole('log');
    const primaryPane = primaryLog.closest('section')!;
    const secondaryPane = secondaryLog.closest('section')!;
    if (failure === 'exit') {
      sessions = sessions.map((session) => session.sessionId === 'session-2' ? { ...session, running: false } : session);
      await event('stack-terminal:closed', { sessionId: 'session-2' });
    } else {
      read = async (sessionId) => {
        if (sessionId === 'session-2') throw new Error('Secondary PTY read failed');
        return { sessionId, chunks: [], exited: false };
      };
      await advance(1000);
    }
    const message = failure === 'exit' ? 'Terminal exited' : 'Secondary PTY read failed';
    const assertScopedNotice = () => {
      expect([...within(primaryPane).queryAllByRole('alert'), ...within(primaryPane).queryAllByRole('status')]).toHaveLength(0);
      const notices = [...view.queryAllByRole('alert'), ...view.queryAllByRole('status')];
      expect(notices).toHaveLength(1);
      expect(secondaryPane.contains(notices[0])).toBe(true);
      expect(notices[0].textContent).toContain(message);
    };
    assertScopedNotice();
    await event('terminal-panel:open');
    window.dispatchEvent(new Event('focus'));
    await settle();
    await advance(1201);
    assertScopedNotice();

    expect(primaryLog.textContent).toContain('healthy primary output');
    expect(secondaryLog.textContent).toContain('secondary output');
  });

  it('targets the newly active split session and then the explicitly activated primary pane for input', async () => {
    const view = await open();
    await fireEvent.click(view.getByRole('button', { name: 'Split terminal pane right' }));
    await settle();
    const [primaryLog, secondaryLog] = view.getAllByRole('log');
    const attachedTerminal = (log: HTMLElement) => {
      const element = log.querySelector('.xterm');
      expect(element, 'activated split pane must own an attached xterm child').not.toBeNull();
      const terminal = xterms.instances.find((candidate) => candidate.element === element);
      expect(terminal, 'attached child must belong to a live terminal double').toBeDefined();
      return terminal;
    };
    // New split is active; do not merely check that some backend write occurred.
    attachedTerminal(secondaryLog).data('echo secondary\r');
    await settle();
    await fireEvent.mouseDown(primaryLog);
    await settle();
    expect(primaryLog.closest('section')!.classList.contains('focused')).toBe(true);
    attachedTerminal(primaryLog).data('echo primary\r');
    await settle();
    expect(tauriMocks().invoke.mock.calls.filter(([name]) => name === 'write_stack_terminal')).toEqual([
      ['write_stack_terminal', { sessionId: 'session-2', input: 'echo secondary\r' }],
      ['write_stack_terminal', { sessionId: 'session-1', input: 'echo primary\r' }]
    ]);
  });

  it('retains a pane read diagnostic while its tab is inactive without leaking it into the healthy tab', async () => {
    const view = await open();
    await event('stack-terminal:output', { sessionId: 'session-1', sequence: 1, text: 'first tab transcript' });
    read = async () => { throw new Error('First tab PTY unavailable'); };
    await advance(1000);
    const assertFirstTabDiagnostic = () => {
      const notices = [...view.queryAllByRole('alert'), ...view.queryAllByRole('status')];
      expect(notices).toHaveLength(1);
      expect(notices[0].textContent).toContain('First tab PTY unavailable');
      expect(view.getByRole('log').closest('section')!.contains(notices[0])).toBe(true);
      expect(view.getByRole('log').textContent).toContain('first tab transcript');
    };
    assertFirstTabDiagnostic();
    // A successful empty read is not an explicit retry/restart of the failed pane.
    read = async (sessionId) => ({ sessionId, chunks: [], exited: false });
    await fireEvent.click(view.getByRole('button', { name: 'New terminal tab' }));
    await settle();
    expect(view.getAllByRole('tab')).toHaveLength(2);
    noStartupNotice(view);
    await event('terminal-panel:open');
    await advance(1201);
    noStartupNotice(view);
    await fireEvent.click(view.getAllByRole('tab')[0]);
    await settle();
    assertFirstTabDiagnostic();
    await event('terminal-panel:open');
    await advance(1201);
    assertFirstTabDiagnostic();
    await fireEvent.click(view.getAllByRole('tab')[1]);
    await settle();
    noStartupNotice(view);
  });

  it('reports an inactive tab exit only when that tab is restored, retaining its replayed output', async () => {
    const view = await open();
    await event('stack-terminal:output', { sessionId: 'session-1', sequence: 1, text: 'hidden tab transcript' });
    await fireEvent.click(view.getByRole('button', { name: 'New terminal tab' }));
    await settle();
    sessions = sessions.map((session) => session.sessionId === 'session-1' ? { ...session, running: false } : session);
    read = async (sessionId) => ({ sessionId, chunks: [], exited: sessionId === 'session-1' });
    await event('stack-terminal:closed', { sessionId: 'session-1' });
    noStartupNotice(view);
    await fireEvent.click(view.getAllByRole('tab')[0]);
    await settle();
    const notices = [...view.queryAllByRole('alert'), ...view.queryAllByRole('status')];
    expect(notices).toHaveLength(1);
    expect(notices[0].textContent).toContain('Terminal exited');
    expect(view.getByRole('log').textContent).toContain('hidden tab transcript');
    await fireEvent.click(view.getAllByRole('tab')[1]);
    await settle();
    noStartupNotice(view);
  });

  it('keeps tabs/splits, replays hidden output once, and does not duplicate a push/poll chunk', async () => {
    const view = await open();
    const chunk = { sessionId: 'session-1', sequence: 1, text: 'original transcript' };
    await event('stack-terminal:output', chunk);
    read = async (sessionId) => ({ sessionId, chunks: sessionId === 'session-1' ? [chunk] : [], exited: false });
    await advance(1000);
    expect(xterms.instances[0].write.mock.calls.filter(([text]: [string]) => text === chunk.text)).toHaveLength(1);
    await fireEvent.click(view.getByRole('button', { name: 'New terminal tab' }));
    await settle();
    expect(view.getAllByRole('tab')).toHaveLength(2);
    expect(view.getAllByRole('log')).toHaveLength(1);
    await event('stack-terminal:output', { sessionId: 'session-1', sequence: 2, text: ' hidden output' });
    await fireEvent.click(view.getAllByRole('tab')[0]);
    await settle();
    expect(view.getByRole('log').textContent).toBe('original transcript hidden output');
    await fireEvent.click(view.getByRole('button', { name: 'Split terminal pane right' }));
    await settle();
    await fireEvent.click(view.getByRole('button', { name: 'Split terminal pane down' }));
    await settle();
    expect(view.getAllByRole('log')).toHaveLength(3);
    expect(view.getAllByRole('tab')).toHaveLength(2);
    await advance(1201);
    noStartupNotice(view);
  });

  it('fits and focuses the attached pane, resizing before its first input write', async () => {
    const view = await open();
    await settle();
    const terminal = xterms.instances.at(-1);
    expect(view.getByRole('log').contains(terminal.element)).toBe(true);
    expect(terminal.focus).toHaveBeenCalled();
    terminal.data('echo ready\r');
    await settle();
    const calls = tauriMocks().invoke.mock.calls;
    const fitIndex = calls.findIndex(([name]) => name === 'resize_stack_terminal');
    const inputIndex = calls.findIndex(([name]) => name === 'write_stack_terminal');
    expect(fitIndex).toBeGreaterThanOrEqual(0);
    expect(inputIndex).toBeGreaterThan(fitIndex);
    expect(calls[inputIndex]).toEqual(['write_stack_terminal', { sessionId: 'session-1', input: 'echo ready\r' }]);
  });

  it('unmount disposes xterm/listeners and cancels idle, startup and poll timers', async () => {
    const view = await open();
    await advance(60);
    view.unmount();
    await settle();
    const count = tauriMocks().invoke.mock.calls.length;
    await advance(10000);
    expect(tauriMocks().invoke).toHaveBeenCalledTimes(count);
    expect(listeners.size).toBe(0);
    for (const unlisten of unlisteners) expect(unlisten).toHaveBeenCalledTimes(1);
    for (const terminal of xterms.instances) expect(terminal.dispose).toHaveBeenCalled();
    expect(vi.getTimerCount()).toBe(0);
  });
});
