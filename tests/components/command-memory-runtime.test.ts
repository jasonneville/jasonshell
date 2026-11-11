import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { expect, it, vi } from 'vitest';
import CommandPanelSurface from '../../src/components/CommandPanelSurface.svelte';
import type { QuickCommandRunHistoryEntry, QuickCommandRunUpdatedEvent } from '../../src/lib/quickCommands';
import { tauriMocks } from './setup';

// Real mounted production component; only Tauri is mocked. No Windows shell launch.
function event(index: number, sequence: number, kind = 'stdout', commandId = 'stream'): QuickCommandRunUpdatedEvent {
  return { runId: `run-${index}`, commandId, processId: index + 100, sequence,
    atEpochMs: index * 10000 + sequence, kind, body: kind === 'exit' ? '0' : `row-${sequence}`,
    requestId: null, prompt: null, secret: false, redacted: false, pending: false };
}
function snapshot(index: number, running = false): QuickCommandRunHistoryEntry {
  return { runId: `run-${index}`, commandId: 'stream', processId: index + 100, startedAtEpochMs: index * 10000 + 1,
    finishedAtEpochMs: index * 10000 + 601, running, exitCode: running ? null : 0, stdout: '', stderr: '',
    stdoutTruncated: false, stderrTruncated: false,
    transcript: Array.from({ length: 256 }, (_, i) => event(index, 346 + i, i === 255 ? 'exit' : 'stdout')) };
}
async function fixture() {
  // Polls are explicitly triggered below, never wall-clock timing dependent.
  const nativeInterval = window.setInterval.bind(window);
  vi.spyOn(window, 'setInterval').mockImplementation((handler, delay, ...args) =>
    delay === 1100 ? 0 : nativeInterval(handler, delay, ...args));
  let deliver!: (event: { payload: QuickCommandRunUpdatedEvent }) => void;
  let history: QuickCommandRunHistoryEntry[] = [];
  let pending: Promise<QuickCommandRunHistoryEntry[]> | null = null;
  const tauri = tauriMocks();
  tauri.listen.mockImplementation(async (_name, callback) => { deliver = callback; return vi.fn(); });
  tauri.invoke.mockImplementation(async (command: string) => {
    if (command === 'load_shell_settings') return { quickCommands: { orderVersion: 1, listWidth: 180, history: [],
      entries: ['stream', 'interactive'].map(id => ({ id, label: id, mode: 'direct', targetPath: 'fixture.exe', args: [], commands: [], cwd: null })) } };
    if (command === 'list_quick_command_history') return pending ?? history;
    return null;
  });
  const view = render(CommandPanelSurface);
  await fireEvent.click(await view.findByRole('button', { name: 'Edit stream' }));
  await fireEvent.click(view.getByRole('tab', { name: 'Previous runs' }));
  await waitFor(() => expect(deliver).toBeTypeOf('function'));
  const flush = async () => { await new Promise<void>(resolve => requestAnimationFrame(() => resolve())); await tick(); };
  return { view, tauri, flush, emit: (payload: QuickCommandRunUpdatedEvent) => deliver({ payload }),
    setHistory: (next: QuickCommandRunHistoryEntry[]) => { history = next; },
    resume: () => { pending = null; },
    delay: (promise: Promise<QuickCommandRunHistoryEntry[]>) => { pending = promise; } };
}

it('mounted live stream bounds recent runs/rows after delayed stale poll and late retired events', async () => {
  const f = await fixture();
  let release!: (value: QuickCommandRunHistoryEntry[]) => void;
  f.delay(new Promise(resolve => { release = resolve; }));
  await fireEvent.click(f.view.getByRole('tab', { name: 'Previous runs' }));
  for (let index = 1; index <= 24; index++) {
    for (let sequence = 1; sequence <= 600; sequence++) f.emit(event(index, sequence));
    f.emit(event(index, 601, 'exit'));
  }
  await f.flush();
  // This poll began before the events; its old running row cannot regain authority.
  release([{ ...snapshot(1, true), transcript: [event(1, 1)] }]);
  await f.flush();
  f.resume(); f.setHistory(Array.from({ length: 20 }, (_, i) => snapshot(24 - i)));
  await fireEvent.click(f.view.getByRole('tab', { name: 'Previous runs' })); await f.flush();
  f.emit(event(1, 2)); f.emit(event(1, 601, 'exit')); await f.flush();
  const cards = [...f.view.container.querySelectorAll('.command-history-run')];
  const rows = [...f.view.container.querySelectorAll('.command-transcript-line')];
  const retained = { runs: cards.length, rows: rows.length,
    bodyCharacters: rows.reduce((n, row) => n + (row.textContent?.length ?? 0), 0) };
  process.stdout.write(`mounted memory-retention fixture ${JSON.stringify(retained)}\n`);
  expect(retained).toEqual({ runs: 20, rows: 5120, bodyCharacters: 35720 });
  expect(cards.length).toBe(20);
  expect(rows.length).toBe(20 * 256);
  for (const card of cards) {
    const retained = card.querySelectorAll('.command-transcript-line');
    expect(retained[0]?.textContent).toBe('row-346');
    expect(retained[255]?.getAttribute('data-kind')).toBe('exit');
  }
  expect(cards.map(card => card.textContent).join(' ')).not.toMatch(/PID 10[1-4]\b/);
  expect(f.view.container.querySelector('[data-command-id="stream"]')?.getAttribute('data-selected')).toBe('true');
  expect(f.view.getByRole('tab', { name: 'Previous runs' }).getAttribute('aria-selected')).toBe('true');
  expect(f.view.queryByRole('button', { name: 'Stop stream' })).toBeNull();
}, 30000);

it('old active input and Stop survive retention; terminal event clears pending input without resurrection', async () => {
  const f = await fixture();
  const prompt = { ...event(1, 1, 'text', 'interactive'), requestId: 'request-1', prompt: 'Continue?', pending: true, maxLength: 24 };
  f.emit(prompt); await f.flush();
  // Pending control must outlive transcript eviction, not pin all intervening output.
  for (let sequence = 2; sequence <= 601; sequence++) f.emit(event(1, sequence, 'stdout', 'interactive'));
  await f.flush();
  for (let index = 2; index <= 25; index++) f.emit(event(index, 601, 'exit'));
  f.setHistory(Array.from({ length: 20 }, (_, i) => snapshot(25 - i)));
  await fireEvent.click(f.view.getByRole('tab', { name: 'Previous runs' })); await f.flush();
  expect(f.view.getByRole('region', { name: 'Backend input required' }).textContent).toContain('Continue?');
  expect((f.view.getByRole('button', { name: 'Stop interactive' }) as HTMLButtonElement).disabled).toBe(false);
  await fireEvent.click(f.view.getByRole('button', { name: 'Edit interactive' }));
  await fireEvent.click(f.view.getByRole('tab', { name: 'Previous runs' })); await f.flush();
  expect.soft(f.view.container.querySelectorAll('.command-transcript-line').length).toBeLessThanOrEqual(256);
  await fireEvent.input(f.view.getByLabelText('Input', { exact: true }), { target: { value: 'yes' } });
  await fireEvent.click(f.view.getByRole('button', { name: 'Submit input' }));
  expect(f.tauri.invoke.mock.calls.find(([name]) => name === 'send_quick_command_input')?.[1]).toMatchObject({
    request: { id: 'interactive', runId: 'run-1', processId: 101, requestId: 'request-1', value: 'yes', maxLength: 24 } });
  await fireEvent.click(f.view.getByRole('button', { name: 'Stop interactive' }));
  await waitFor(() => expect(f.tauri.invoke.mock.calls.find(([name]) => name === 'stop_quick_command')?.[1]).toEqual({
    request: { id: 'interactive', runId: 'run-1', processId: 101 } }));
  f.emit({ ...event(1, 900, 'stopped', 'interactive'), body: '' }); await f.flush();
  f.emit(prompt); await f.flush();
  expect(f.view.queryByRole('region', { name: 'Backend input required' })).toBeNull();
  expect(f.view.queryByRole('button', { name: 'Stop interactive' })).toBeNull();
}, 15000);
