import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import ProcessManagerSurface from '../../src/components/ProcessManagerSurface.svelte';
import { tauriMocks } from './setup';
import { context } from './fixtures/system-info';

const openEvent = { payload: { focusPid: null, context: context() } };
const closedEvent = { payload: { context: context(1,2,'processes',null,false) } };

it('closing an open process manager cancels polling while a refresh is unresolved', async () => {
  const tauri = tauriMocks();
  const handlers = new Map<string, (event: { payload: unknown }) => void>();
  tauri.listen.mockImplementation(async (name: string, callback: (event: { payload: unknown }) => void) => {
    handlers.set(name, callback);
    return vi.fn();
  });
  tauri.invoke.mockImplementation(() => new Promise(() => {}));
  const clear = vi.spyOn(window, 'clearInterval');
  const view = render(ProcessManagerSurface);
  await waitFor(() => expect(handlers.size).toBeGreaterThanOrEqual(2));
  handlers.get('process-manager:open')?.(openEvent);
  await waitFor(() => expect(tauri.invoke).toHaveBeenCalled());
  const clearedBeforeClose=clear.mock.calls.length;
  handlers.get('process-manager:closed')?.(closedEvent);
  expect(clear.mock.calls.length).toBeGreaterThan(clearedBeforeClose);
  view.unmount();
  clear.mockRestore();
});

it('late process snapshot cannot repopulate closed surface; unmount releases registered listeners', async () => {
  const tauri = tauriMocks();
  const handlers = new Map<string, (event: { payload: unknown }) => void>();
  const releases = [vi.fn(), vi.fn()];
  tauri.listen.mockImplementation(async (name: string, callback: (event: { payload: unknown }) => void) => {
    handlers.set(name, callback);
    return releases[handlers.size - 1];
  });
  let resolveProcesses!: (rows: unknown[]) => void;
  tauri.invoke.mockImplementation((command: string) => command === 'list_processes'
    ? new Promise((resolve) => { resolveProcesses = resolve; }) : Promise.resolve(command === 'get_process_manager_context' ? context(0, 0, 'processes', null, false) : []));
  const view = render(ProcessManagerSurface);
  await waitFor(() => expect(handlers.size).toBe(2));
  handlers.get('process-manager:open')?.(openEvent);
  await waitFor(() => expect(tauri.invoke.mock.calls.some((args) => args[0] === 'list_processes')).toBe(true));
  handlers.get('process-manager:closed')?.(closedEvent);
  resolveProcesses([{ pid: 987654, name: 'LateSnapshotSentinel', status: 'running', isKillable: false }]);
  // Drain the settled Promise.all and Svelte rendering before asserting absence.
  await new Promise((resolve)=>setTimeout(resolve,0));await tick();
  expect(view.queryByText('LateSnapshotSentinel')).toBeNull();
  view.unmount();
  for (const release of releases) expect(release).toHaveBeenCalledTimes(1);
});

it('unmount directly stops active polling even without a close event', async () => {
  const tauri = tauriMocks();
  const handlers = new Map<string, (event: { payload: unknown }) => void>();
  const unlisten = vi.fn();
  tauri.listen.mockImplementation(async (name: string, callback: (event: { payload: unknown }) => void) => {
    handlers.set(name, callback);
    return unlisten;
  });
  tauri.invoke.mockImplementation(() => new Promise(() => {}));
  const clear = vi.spyOn(window, 'clearInterval');
  const view = render(ProcessManagerSurface);
  await waitFor(() => expect(handlers.size).toBe(2));
  handlers.get('process-manager:open')?.(openEvent);
  await waitFor(() => expect(tauri.invoke).toHaveBeenCalled());
  view.unmount();
  expect(clear).toHaveBeenCalled();
  expect(unlisten).toHaveBeenCalledTimes(2);
  clear.mockRestore();
});

it('accessible close control requests native hide and stops pending refresh polling', async () => {
  const tauri = tauriMocks();
  const handlers = new Map<string, (event: { payload: unknown }) => void>();
  tauri.listen.mockImplementation(async (name: string, callback: (event: { payload: unknown }) => void) => {
    handlers.set(name, callback);
    return vi.fn();
  });
  tauri.invoke.mockImplementation((command: string) => command === 'list_processes'
    ? new Promise(() => {}) : Promise.resolve(command === 'get_process_manager_context' ? context(0, 0, 'processes', null, false) : []));
  const clear = vi.spyOn(window, 'clearInterval');
  const view = render(ProcessManagerSurface);
  await waitFor(() => expect(handlers.size).toBe(2));
  handlers.get('process-manager:open')?.(openEvent);
  await waitFor(() => expect(tauri.invoke.mock.calls.some(([command]) => command === 'list_processes')).toBe(true));
  await fireEvent.click(view.getByRole('button', { name: 'Close process manager' }));
  expect(tauri.invoke.mock.calls.some(([command]) => command === 'hide_process_manager')).toBe(true);
  expect(clear).toHaveBeenCalled();
  view.unmount();
  clear.mockRestore();
});
