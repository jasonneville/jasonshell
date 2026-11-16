import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { expect, it, vi } from 'vitest';
import ProcessManagerSurface from '../../src/components/ProcessManagerSurface.svelte';
import { tauriMocks } from '../components/setup';
import { context } from '../components/fixtures/system-info';

async function mount() {
  const handlers = new Map<string, (event: any) => void>();
  const tauri = tauriMocks();
  let resolveKill!: () => void;
  let rejectKill!: (error: Error) => void;
  const kill = new Promise<void>((resolve, reject) => { resolveKill = resolve; rejectKill = reject; });
  const rows = [1000, 2000].map((pid) => ({ pid, parentPid: 0, name: `Pending-${pid}`, status: 'running', isKillable: true,
    cpuPercent: 0, memoryBytes: 1024, creationTime100ns: `${pid}12345`, normalizedImagePath: `c:\\fixture\\pending-${pid}.exe` }));
  tauri.listen.mockImplementation(async (name, callback) => { handlers.set(name, callback); return () => handlers.delete(name); });
  tauri.invoke.mockImplementation(async (command) => {
    if (command === 'get_process_manager_context') return context(0, 0, 'processes', null, false);
    if (command === 'list_processes') return rows;
    if (command === 'list_taskbar_process_windows') return [{ processId: 1000, title: 'Fixture task', isActive: true }];
    if (command === 'kill_process') return kill;
    if (command === 'hide_process_manager') return;
    throw new Error(`Unexpected pending-kill fixture IPC: ${command}`);
  });
  const view = render(ProcessManagerSurface);
  await waitFor(() => expect(handlers.size).toBe(2)); await tick();
  const open = async (generation: number) => {
    handlers.get('process-manager:open')!({ payload: { focusPid: null, context: context(generation, generation * 2, 'processes') } });
    await waitFor(() => expect(view.queryByText('Pending-1000')).not.toBeNull());
  };
  const close = async (route: string) => {
    if (route === 'native') handlers.get('process-manager:closed')!({ payload: { context: context(1, 3, 'processes', null, false) } });
    else await fireEvent.click(view.getByRole('button', { name: 'Close process manager' }));
    await tick();
  };
  const calls = () => tauri.invoke.mock.calls.filter(([command]) => command === 'kill_process');
  await open(1);
  return { view, open, close, calls, resolveKill, rejectKill };
}

it.each(['native', 'button'])('%s close cancels armed confirmation without dispatching a destructive call', async (route) => {
  const { view, open, close, calls } = await mount();
  await fireEvent.click(view.getByRole('button', { name: 'Arm single-process kill confirmation for Pending-1000 process 1000' }));
  expect(view.queryByRole('button', { name: 'Confirm kill for Pending-1000 process 1000' })).not.toBeNull();
  await close(route); await open(2);
  expect(view.queryByRole('button', { name: 'Confirm kill for Pending-1000 process 1000' })).toBeNull();
  await fireEvent.click(view.getByRole('button', { name: 'Arm single-process kill confirmation for Pending-1000 process 1000' }));
  expect(calls()).toHaveLength(0);
  expect(view.queryByRole('button', { name: 'Confirm kill for Pending-1000 process 1000' })).not.toBeNull();
});

it.each([
  ['native', 'resolve'], ['native', 'reject'], ['button', 'resolve'], ['button', 'reject']
])('%s close/reopen retains single destructive invocation while original kill will %s', async (route, outcome) => {
  vi.spyOn(console, 'error').mockImplementation(() => {});
  const { view, open, close, calls, resolveKill, rejectKill } = await mount();
  await fireEvent.click(view.getByRole('button', { name: 'Arm single-process kill confirmation for Pending-1000 process 1000' }));
  await fireEvent.click(view.getByRole('button', { name: 'Confirm kill for Pending-1000 process 1000' }));
  await waitFor(() => expect(calls()).toHaveLength(1));
  const firstPayload = calls()[0][1];
  await close(route); await open(2);
  const guarded = view.getAllByRole('button', { name: 'Kill action in progress for process 1000' });
  expect(guarded).toHaveLength(2);
  // Includes the same-PID button, which stays enabled for status presentation;
  // the production handler must independently reject duplicate dispatch.
  for (let i = 0; i < 3; i++) for (const button of guarded) await fireEvent.click(button);
  expect(calls()).toHaveLength(1);
  expect(calls()[0][1]).toEqual(firstPayload);
  const before = (view.component as any).__ramProbe();
  expect(before.killingPid).toBe(1000);
  expect(before.armedKillPid).toBeNull();
  if (outcome === 'resolve') resolveKill(); else rejectKill(new Error('retired generation kill failure'));
  await waitFor(() => expect((view.component as any).__ramProbe().killingPid).toBeNull());
  expect(view.container.textContent).not.toContain('retired generation kill failure');
  expect(view.container.textContent).not.toContain('Killed Pending-1000');
  expect(calls()).toHaveLength(1);
  await fireEvent.click(view.getByRole('button', { name: 'Arm single-process kill confirmation for Pending-2000 process 2000' }));
  expect(calls()).toHaveLength(1);
  expect(view.queryByRole('button', { name: 'Confirm kill for Pending-2000 process 2000' })).not.toBeNull();
  console.info('ram-pending-kill-guard', JSON.stringify({ route, outcome, destructiveCalls: calls().length, retainedPendingPidBeforeSettlement: before.killingPid, pendingPidAfterSettlement: (view.component as any).__ramProbe().killingPid }));
});
