import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import TerminalPanelSurface from '../../src/components/TerminalPanelSurface.svelte';
import StackTerminalPane from '../../src/components/StackTerminalPane.svelte';
import * as stackApi from '../../src/lib/stackPopup';
import * as persistentApi from '../../src/lib/persistentTerminal';
import { tauriMocks } from './setup';

const terminals = vi.hoisted(() => [] as any[]);
vi.mock('@xterm/xterm', () => ({ Terminal: class {
  element: HTMLElement | null = null;
  cols = 80; rows = 24;
  buffer = { active: { baseY: 0, cursorY: 0, cursorX: 0 } };
  parser = { registerOscHandler: () => ({ dispose() {} }) };
  unicode = { activeVersion: '' };
  options: Record<string, unknown>;
  write = vi.fn((text: string) => { if (this.element) this.element.textContent += text; });
  constructor(options: Record<string, unknown>) { this.options = options; terminals.push(this); }
  open(host: HTMLElement) { this.element = document.createElement('div'); this.element.className = 'xterm'; host.append(this.element); }
  loadAddon() {} registerLinkProvider() {} onData() { return { dispose() {} }; } attachCustomKeyEventHandler() {}
  focus() {} scrollToBottom() {} hasSelection() { return false; } getSelection() { return ''; }
  reset() { if (this.element) this.element.textContent = ''; }
  dispose() { this.element?.remove(); }
} }));
vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() {} dispose() {} } }));
vi.mock('@xterm/addon-search', () => ({ SearchAddon: class { dispose() {} } }));
vi.mock('@xterm/addon-serialize', () => ({ SerializeAddon: class { serialize() { return ''; } dispose() {} } }));
vi.mock('@xterm/addon-unicode11', () => ({ Unicode11Addon: class {} }));
vi.mock('@xterm/addon-web-links', () => ({ WebLinksAddon: class {} }));
vi.mock('@xterm/addon-webgl', () => ({ WebglAddon: class { onContextLoss() {} dispose() {} } }));

const listeners = new Map<string, (event: { payload: any }) => void>();
const makeSession = (sessionId: string) => ({ sessionId, cwd: 'C:\\dev', profile: 'powershell' as const, title: 'PowerShell', running: true });
let sessions: ReturnType<typeof makeSession>[];
let counter: number;
let poll: (id: string) => Promise<any>;
let start: () => Promise<any>;
const chunk = (sequence: number, text: string, sessionId = 'session-1', stream = 'stdout') => ({ sessionId, sequence, text, stream });
async function settle() { for (let i = 0; i < 40; i += 1) { await Promise.resolve(); await tick(); } }
async function advance(ms = 1000) { await vi.advanceTimersByTimeAsync(ms); await settle(); }
async function emit(name: string, payload: unknown = {}) { expect(listeners.has(name)).toBe(true); listeners.get(name)!({ payload }); await settle(); }
async function openPanel() { const view = render(TerminalPanelSurface); await settle(); await emit('terminal-panel:open'); return view; }
function deferred<T>() { let resolve!: (v: T) => void; const promise = new Promise<T>((r) => { resolve = r; }); return { promise, resolve }; }

beforeEach(() => {
  vi.useFakeTimers(); terminals.length = 0; listeners.clear(); sessions = []; counter = 0;
  start = async () => { const session = makeSession(`session-${++counter}`); sessions.push(session); return session; };
  poll = async (sessionId) => ({ sessionId, cwd: 'C:\\dev', running: true, chunks: [] });
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(480);
  vi.spyOn(console, 'error').mockImplementation(() => {});
  tauriMocks().listen.mockImplementation(async (name, callback) => { listeners.set(name, callback); return () => listeners.delete(name); });
  tauriMocks().invoke.mockImplementation(async (command: string, args?: any) => {
    if (command === 'list_stack_terminals') return [...sessions];
    if (command === 'start_persistent_terminal' || command === 'start_stack_terminal') return start();
    if (command === 'poll_stack_terminal_session') return poll(args.sessionId);
    // Deliberately reject legacy reads: mounted journeys must use chunks-only IPC.
    if (command === 'read_stack_terminal') throw new Error('Legacy aggregate output must not be requested');
    if (command === 'resize_stack_terminal' || command === 'write_stack_terminal') return;
    if (command === 'stop_stack_terminal') { sessions = sessions.filter((s) => s.sessionId !== args.sessionId); return; }
    throw new Error(`Unexpected terminal retention IPC: ${command}`);
  });
});
afterEach(() => { cleanup(); vi.clearAllTimers(); });

describe('typed chunks-only adapter', () => {
  it.each([['stack', stackApi], ['persistent', persistentApi]])('%s adapter invokes authorized existing poll command and normalizes running/cwd', async (_name, api) => {
    const adapter = (api as any).pollStackTerminal;
    expect(adapter, 'pollStackTerminal export').toBeTypeOf('function');
    const chunks = [chunk(1, 'first'), chunk(2, 'second')];
    for (const running of [true, false]) {
      poll = async (sessionId) => ({ sessionId, cwd: 'C:\\next', running, chunks });
      const result = await adapter('session-1');
      expect(result).toEqual({ sessionId: 'session-1', cwd: 'C:\\next', chunks, exited: !running, exitCode: null });
      expect(result).not.toHaveProperty('output');
    }
    expect(tauriMocks().invoke.mock.calls).toEqual([
      ['poll_stack_terminal_session', { sessionId: 'session-1' }], ['poll_stack_terminal_session', { sessionId: 'session-1' }]
    ]);
  });

  it('measures serialized IPC response bytes for identical chunks without aggregate duplication', async () => {
    const adapter = (stackApi as any).pollStackTerminal;
    expect(adapter).toBeTypeOf('function');
    const chunks = Array.from({ length: 100 }, (_, i) => chunk(i, 'x'.repeat(100)));
    const nativePoll = { sessionId: 'session-1', cwd: 'C:\\dev', running: true, chunks };
    poll = async () => nativePoll;
    const legacy = { sessionId: 'session-1', cwd: 'C:\\dev', output: chunks.map((c) => c.text).join(''), chunks, exited: false, exitCode: null };
    const result = await adapter('session-1');
    expect(result.chunks).toEqual(legacy.chunks);
    const pollBytes = new TextEncoder().encode(JSON.stringify(nativePoll)).length;
    const legacyBytes = new TextEncoder().encode(JSON.stringify(legacy)).length;
    expect(pollBytes).toBeLessThan(legacyBytes);
    console.info(JSON.stringify({ chunks: chunks.length, pollBytes, legacyBytes, savedBytes: legacyBytes - pollBytes }));
    expect(tauriMocks().invoke).not.toHaveBeenCalledWith('read_stack_terminal', expect.anything());
  });
});

describe('mounted production terminal retention journeys (mock Tauri, no native shell)', () => {
  it.each([
    ['panel', 'push-first'], ['panel', 'poll-first'],
    ['stack', 'push-first'], ['stack', 'poll-first']
  ])('%s %s preserves holes/streams and renders the final chunk before exit', async (surface, order) => {
    const onCwdChange = vi.fn();
    const view = surface === 'panel' ? await openPanel() : render(StackTerminalPane, { currentPath: 'C:\\dev', onCwdChange });
    if (surface === 'stack') { await settle(); await (view.component as any).startTerminal(); await settle(); }
    const first = [chunk(5, 'A'), chunk(5, 'E', 'session-1', 'stderr')];
    const push = async () => { for (const value of first) await emit('stack-terminal:output', value); await advance(20); };
    poll = async (sessionId) => ({ sessionId, cwd: 'C:\\dev', running: true, chunks: first });
    if (order === 'push-first') { await push(); await advance(2000); }
    else { await advance(2000); await push(); }
    await emit('stack-terminal:output', chunk(3, 'B')); await advance(20);
    await emit('stack-terminal:output', chunk(3, 'B')); await advance(20);
    const log = view.getByRole('log');
    expect(log.textContent).toBe('AEB');
    poll = async (sessionId) => ({ sessionId, cwd: 'C:\\next', running: false, chunks: [...first, chunk(3, 'B'), chunk(4, 'F')] });
    await advance(2000);
    expect(log.textContent).toBe('AEBF');
    expect(view.container.textContent).toMatch(/exited/i);
    if (surface === 'stack') expect(onCwdChange).toHaveBeenCalledWith('C:\\next');
    else expect(view.container.textContent).toContain('c:/next');
    expect(tauriMocks().invoke.mock.calls.some(([name]) => name === 'read_stack_terminal')).toBe(false);
  });

  it('stack recovers a push delivered before the start response through poll exactly once', async () => {
    const pending = deferred<any>(); start = () => pending.promise;
    const value = chunk(1, 'early stack prompt');
    poll = async (sessionId) => ({ sessionId, cwd: 'C:\\dev', running: true, chunks: [value] });
    const view = render(StackTerminalPane, { currentPath: 'C:\\dev' }); await settle();
    const starting = (view.component as any).startTerminal(); await settle();
    await emit('stack-terminal:output', value);
    sessions = [makeSession('session-1')]; pending.resolve(sessions[0]); await starting; await settle(); await advance(2000);
    expect(view.getByRole('log').textContent).toBe(value.text);
    await emit('stack-terminal:output', value); await advance(2000);
    expect(view.getByRole('log').textContent).toBe(value.text);
  });

  it('stack replacement rejects old poll completion and late push without poisoning fresh sequence identity', async () => {
    const view = render(StackTerminalPane, { currentPath: 'C:\\dev' }); await settle();
    await (view.component as any).startTerminal(); await settle();
    const pending = deferred<any>(); let pendingStarted = false;
    poll = () => { pendingStarted = true; return pending.promise; };
    await advance(2000); expect(pendingStarted).toBe(true);
    await (view.component as any).stopTerminal();
    poll = async (sessionId) => ({ sessionId, cwd: 'C:\\dev', running: true, chunks: [] });
    await (view.component as any).startTerminal(); await settle();
    pending.resolve({ sessionId: 'session-1', cwd: 'C:\\stale', running: false, chunks: [chunk(1, 'STALE')] }); await settle();
    await emit('stack-terminal:output', chunk(1, 'OLD'));
    await emit('stack-terminal:output', chunk(1, 'NEW', 'session-2')); await advance(20);
    expect(view.getByRole('log').textContent).toBe('NEW');
    expect(view.container.textContent).not.toMatch(/STALE|C:\\stale/);
  });

  it.each(['panel', 'stack'])('%s preserves exceptional numeric Set identity across push and poll', async (surface) => {
    const view = surface === 'panel' ? await openPanel() : render(StackTerminalPane, { currentPath: 'C:\\dev' });
    if (surface === 'stack') { await settle(); await (view.component as any).startTerminal(); await settle(); }
    const values = [NaN, Infinity, -Infinity, 1.5, -0, 0, Number.MAX_SAFE_INTEGER + 1];
    const oracleKeys = new Set<string>();
    let expected = '';
    const chunks = values.map((sequence, index) => chunk(sequence, String.fromCharCode(65 + index)));
    for (const value of chunks) {
      const key = `${value.sessionId}:${value.stream}:${value.sequence}`;
      if (!oracleKeys.has(key)) { oracleKeys.add(key); expected += value.text; }
      await emit('stack-terminal:output', value); await advance(20);
      await emit('stack-terminal:output', value); await advance(20);
    }
    expect(view.getByRole('log').textContent).toBe(expected);
    poll = async (sessionId) => ({ sessionId, cwd: 'C:\\dev', running: true,
      chunks: [...chunks, chunk(NaN, 'S', 'session-1', 'stderr'), chunk(NaN, 'WRONG', 'other-session')] });
    await advance(2000);
    expect(view.getByRole('log').textContent).toBe(`${expected}S`);
  });

  it('deduplicates push/poll sparse interleaved streams, retains hidden replay, and restores once', async () => {
    const view = await openPanel();
    const initial = [chunk(10, 'A'), chunk(8, 'B'), chunk(10, 'E', 'session-1', 'stderr')];
    for (const value of initial) await emit('stack-terminal:output', value);
    poll = async (sessionId) => ({ sessionId, cwd: 'C:\\dev', running: true, chunks: sessionId === 'session-1' ? [...initial, chunk(9, 'C')] : [] });
    await advance();
    expect(view.getByRole('log').textContent).toBe('ABEC');
    await fireEvent.click(view.getByRole('button', { name: 'New terminal tab' })); await settle();
    await emit('stack-terminal:output', chunk(11, 'H'));
    await emit('stack-terminal:output', chunk(11, 'H'));
    await emit('stack-terminal:output', chunk(7, 'L'));
    await fireEvent.click(view.getAllByRole('tab')[0]); await settle();
    expect(view.getByRole('log').textContent).toBe('ABECHL');
    await emit('stack-terminal:output', chunk(11, 'H'));
    await emit('terminal-panel:open');
    expect(view.getByRole('log').textContent).toBe('ABECHL');
    expect(tauriMocks().invoke.mock.calls.some(([name]) => name === 'poll_stack_terminal_session')).toBe(true);
    expect(tauriMocks().invoke.mock.calls.some(([name]) => name === 'read_stack_terminal')).toBe(false);
  });

  it('retains legitimate initial output delivered before runtime creation', async () => {
    const pending = deferred<any>(); start = () => pending.promise;
    const view = render(TerminalPanelSurface); await settle(); await emit('terminal-panel:open');
    await emit('stack-terminal:output', chunk(1, 'early prompt'));
    sessions = [makeSession('session-1')]; pending.resolve(sessions[0]); await settle();
    expect(view.getByRole('log').textContent).toBe('early prompt');
    await emit('stack-terminal:output', chunk(1, 'early prompt'));
    expect(view.getByRole('log').textContent).toBe('early prompt');
  });

  it('ignores delayed poll settlement from a closed runtime while replacement remains writable', async () => {
    const view = await openPanel();
    const pending = deferred<any>(); let pendingStarted = false;
    poll = () => { pendingStarted = true; return pending.promise; };
    await advance();
    expect(pendingStarted).toBe(true);
    expect(tauriMocks().invoke).toHaveBeenCalledWith('poll_stack_terminal_session', { sessionId: 'session-1' });
    await fireEvent.click(view.getByRole('button', { name: 'Open terminal actions menu' }));
    await fireEvent.click(view.getByRole('menuitem', { name: 'Stop terminal', exact: true })); await settle();
    await fireEvent.click(view.getByRole('button', { name: 'New terminal tab' })); await settle();
    pending.resolve({ sessionId: 'session-1', cwd: 'C:\\stale', running: false, chunks: [chunk(1, 'STALE')] }); await settle();
    await emit('stack-terminal:output', chunk(1, 'replacement', 'session-2'));
    expect(view.getByRole('log').textContent).toBe('replacement');
    expect(view.container.textContent).not.toContain('C:\\stale');
  });

  it('rejects a poll response bearing another session identity without consuming its chunk keys', async () => {
    const view = await openPanel();
    poll = async () => ({ sessionId: 'wrong-session', cwd: 'C:\\wrong', running: false, chunks: [chunk(5, 'WRONG')] });
    await advance();
    expect(tauriMocks().invoke).toHaveBeenCalledWith('poll_stack_terminal_session', { sessionId: 'session-1' });
    expect(view.getByRole('log').textContent).toBe('');
    await emit('stack-terminal:output', chunk(5, 'legitimate'));
    expect(view.getByRole('log').textContent).toBe('legitimate');
    expect(view.container.textContent).not.toContain('C:\\wrong');
  });

  it('stack pane consumes chunks-only poll, suppresses push duplicate, updates cwd and exited presentation', async () => {
    const onCwdChange = vi.fn();
    const view = render(StackTerminalPane, { currentPath: 'C:\\dev', onCwdChange }); await settle();
    await (view.component as any).startTerminal(); await settle();
    const value = chunk(1, 'stack output');
    await emit('stack-terminal:output', value); await advance(20);
    poll = async (sessionId) => ({ sessionId, cwd: 'C:\\next', running: false, chunks: [value, chunk(2, ' final')] });
    await advance(2000);
    expect(terminals.at(-1).element.textContent).toBe('stack output final');
    expect(onCwdChange).toHaveBeenCalledWith('C:\\next');
    expect(view.container.textContent).toMatch(/exited/i);
    expect(tauriMocks().invoke.mock.calls.some(([name]) => name === 'read_stack_terminal')).toBe(false);
  });
});
