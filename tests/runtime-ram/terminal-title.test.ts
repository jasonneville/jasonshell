import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import TerminalPanelSurface from '../../src/components/TerminalPanelSurface.svelte';
import { buildTerminalTabTitle } from '../../src/features/terminal/terminalTabTitle';
import { tauriMocks } from '../components/setup';

const terminals = vi.hoisted(() => [] as any[]);
vi.mock('@xterm/xterm', () => ({ Terminal: class {
  element: HTMLElement | null = null; cols = 80; rows = 24;
  buffer = { active: { baseY: 0, cursorY: 0, cursorX: 0 } };
  parser = { registerOscHandler: () => ({ dispose() {} }) };
  unicode = { activeVersion: '' }; options: Record<string, unknown>;
  write = vi.fn((text: string) => { if (this.element) this.element.textContent += text; });
  constructor(options: Record<string, unknown>) { this.options = options; terminals.push(this); }
  open(host: HTMLElement) { this.element = document.createElement('div'); this.element.className = 'xterm'; host.append(this.element); }
  loadAddon() {} registerLinkProvider() {} onData() { return { dispose() {} }; } attachCustomKeyEventHandler() {}
  focus() {} scrollToBottom() {} hasSelection() { return false; } getSelection() { return ''; }
  reset() { if (this.element) this.element.textContent = ''; } dispose() { this.element?.remove(); }
} }));
vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() {} dispose() {} } }));
vi.mock('@xterm/addon-search', () => ({ SearchAddon: class { dispose() {} } }));
vi.mock('@xterm/addon-serialize', () => ({ SerializeAddon: class { serialize() { return ''; } dispose() {} } }));
vi.mock('@xterm/addon-unicode11', () => ({ Unicode11Addon: class {} }));
vi.mock('@xterm/addon-web-links', () => ({ WebLinksAddon: class {} }));
vi.mock('@xterm/addon-webgl', () => ({ WebglAddon: class { onContextLoss() {} dispose() {} } }));
const listeners = new Map<string, (event: { payload: any }) => void>();
let sessions: any[] = [];
let startGate: Promise<void> | null = null;
const probe = (view: ReturnType<typeof render>) => (view.component as any).__ramProbe();
async function settle() { for (let i = 0; i < 40; i++) { await Promise.resolve(); await tick(); } }
async function emit(name: string, payload: any = {}) { expect(listeners.has(name)).toBe(true); listeners.get(name)!({ payload }); await settle(); }
beforeEach(() => {
  vi.useFakeTimers(); terminals.length = 0; listeners.clear(); sessions = []; startGate = null;
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(480);
  tauriMocks().listen.mockImplementation(async (name, callback) => { listeners.set(name, callback); return () => listeners.delete(name); });
  tauriMocks().invoke.mockImplementation(async (command: string, args?: any) => {
    if (command === 'list_stack_terminals') return [...sessions];
    if (command === 'start_persistent_terminal' || command === 'start_stack_terminal') {
      if (startGate) await startGate;
      const session = { sessionId: `session-${sessions.length + 1}`, cwd: 'C:\\dev', profile: 'powershell', title: 'PowerShell', running: true };
      sessions.push(session); return session;
    }
    if (command === 'poll_stack_terminal_session') return { sessionId: args.sessionId, cwd: 'C:\\dev', running: true, chunks: [] };
    if (['resize_stack_terminal', 'write_stack_terminal'].includes(command)) return;
    if (command === 'stop_stack_terminal') { sessions = sessions.filter((row) => row.sessionId !== args.sessionId); return; }
    throw new Error(`Unexpected title RAM IPC: ${command}`);
  });
});
afterEach(() => { cleanup(); vi.clearAllTimers(); });

it('title helper output is invariant across large/ANSI/empty/misleading output for all title inputs', () => {
  const inputs = [{}, { profileTitle: 'PowerShell' }, { cwd: 'C:\\dev\\repo' }, { manualTitle: 'My console', cwd: 'C:\\dev' },
    { cwd: 'C:\\dev\\repo', currentInputText: 'npm test' }, { cwd: 'C:\\dev\\repo', currentInputText: 'codex' }];
  for (const input of inputs) for (const recentOutputText of ['', 'error: rename title', '\x1b[31mRED\x1b[0m', 'X'.repeat(200000)]) {
    expect(buildTerminalTabTitle({ ...input, recentOutputText })).toBe(buildTerminalTabTitle(input));
  }
});

it('runtime-less output retains replay but zero unused title-output metadata; reattach and hidden-tab churn remain exact', async () => {
  const view = render(TerminalPanelSurface); await settle();
  let finishStart!: () => void;
  startGate = new Promise<void>((resolve) => { finishStart = resolve; });
  await emit('terminal-panel:open');
  const chunk = { sessionId: 'session-1', sequence: 1, stream: 'stdout', text: 'R'.repeat(24000) };
  await emit('stack-terminal:output', chunk);
  const beforeOpen = probe(view);
  console.info('ram-terminal-title-runtime-less', JSON.stringify(beforeOpen));
  expect.soft(beforeOpen.titleOutputCharacters).toBe(0);
  expect(beforeOpen.runtimes).toBe(0);
  startGate = null; finishStart(); await settle();
  await vi.advanceTimersByTimeAsync(50); await settle();
  expect(view.getByRole('log').textContent).toBe(chunk.text);
  await emit('stack-terminal:output', chunk);
  expect(view.getByRole('log').textContent).toBe(chunk.text);
  const title = view.getAllByRole('tab')[0].textContent;
  await fireEvent.click(view.getByRole('button', { name: 'New terminal tab' })); await settle();
  await emit('stack-terminal:output', { ...chunk, sequence: 2, text: 'hidden tail' });
  await fireEvent.click(view.getAllByRole('tab')[0]); await settle();
  expect(view.getByRole('log').textContent).toBe(chunk.text + 'hidden tail');
  expect(view.getAllByRole('tab')[0].textContent).toBe(title);
  console.info('ram-terminal-title-reattach', JSON.stringify(probe(view)));
  expect.soft(probe(view).titleOutputCharacters).toBe(0);
  view.unmount();
  expect(listeners.size).toBe(0);
});
