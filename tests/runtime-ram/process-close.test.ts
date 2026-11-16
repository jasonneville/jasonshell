import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { expect, it, vi } from 'vitest';
import ProcessManagerSurface from '../../src/components/ProcessManagerSurface.svelte';
import { tauriMocks } from '../components/setup';
import { context } from '../components/fixtures/system-info';

const probe = (view: ReturnType<typeof render>) => (view.component as any).__ramProbe();
async function mount() {
  const handlers = new Map<string, (event: any) => void>();
  const tauri = tauriMocks();
  tauri.listen.mockImplementation(async (name, callback) => { handlers.set(name, callback); return () => handlers.delete(name); });
  let rows: any[] = Array.from({ length: 40 }, (_, i) => ({ pid: i + 1000, parentPid: 0, name: `Retained-${i}`, status: 'running', isKillable: true,
    cpuPercent: 0, memoryBytes: 1024, iconDataUrl: `data:image/png;base64,${'A'.repeat(8192)}`, commandLine: 'cmd '.repeat(2048) }));
  let read = () => Promise.resolve(rows);
  tauri.invoke.mockImplementation(async (command) => {
    if (command === 'get_process_manager_context') return context(0, 0, 'processes', null, false);
    if (command === 'list_processes') return read();
    if (command === 'list_taskbar_process_windows') return [];
    if (command === 'hide_process_manager') return;
    throw new Error(`Unexpected RAM fixture IPC: ${command}`);
  });
  const view = render(ProcessManagerSurface);
  await waitFor(() => expect(handlers.size).toBe(2));
  await tick();
  const open = (generation: number) => handlers.get('process-manager:open')!({ payload: { focusPid: null, context: context(generation, generation * 2, 'processes') } });
  open(1);
  await waitFor(() => expect(probe(view).rows).toBe(40));
  return { view, handlers, open, setRead: (next: typeof read) => { read = next; }, setRows: (next: any[]) => { rows = next; } };
}

it.each(['button', 'native-close'])('%s releases loaded process payload; reopening obtains fresh rows', async (route) => {
  const { view, handlers, open, setRows } = await mount();
  const before = probe(view);
  if (route === 'button') await fireEvent.click(view.getByRole('button', { name: 'Close process manager' }));
  else handlers.get('process-manager:closed')!({ payload: { context: context(1, 3, 'processes', null, false) } });
  await tick();
  const after = probe(view);
  console.info('ram-process-close', JSON.stringify({ route, before, after }));
  expect.soft(after.rows).toBe(0);
  expect.soft(after.payloadCharacters).toBe(0);
  expect.soft(after.taskbarPids).toBe(0);
  expect.soft(after.armedKillPid).toBeNull();
  setRows([{ pid: 9000, name: 'Fresh reopen', status: 'running', isKillable: false }]);
  open(2);
  await waitFor(() => expect(view.queryByText('Fresh reopen')).not.toBeNull());
  expect(probe(view).rows).toBe(1);
});

it('late refresh cannot restore retired process payload after close or contaminate reopening', async () => {
  const { view, open, setRead, setRows } = await mount();
  let resolve!: (rows: any[]) => void;
  setRead(() => new Promise((done) => { resolve = done; }));
  await fireEvent.click(view.getByRole('button', { name: 'Refresh process manager', exact: true }));
  await waitFor(() => expect(resolve).toBeTypeOf('function'));
  await fireEvent.click(view.getByRole('button', { name: 'Close process manager' }));
  resolve([{ pid: 9999, name: 'STALE', status: 'running', isKillable: false }]);
  await new Promise((done) => setTimeout(done, 0)); await tick();
  expect.soft(probe(view).rows).toBe(0);
  setRows([{ pid: 9001, name: 'Current reopen', status: 'running', isKillable: false }]);
  setRead(() => Promise.resolve([{ pid: 9001, name: 'Current reopen', status: 'running', isKillable: false }]));
  open(2);
  await waitFor(() => expect(view.queryByText('Current reopen')).not.toBeNull());
  expect(view.queryByText('STALE')).toBeNull();
});
