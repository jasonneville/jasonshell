import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { expect, it, vi } from 'vitest';
import CommandPanelSurface from '../../src/components/CommandPanelSurface.svelte';
import { tauriMocks } from './setup';

it('disposes late listener registration exactly once after unmount', async () => {
  const tauri = tauriMocks();
  let resolve!: (unlisten: () => void) => void;
  tauri.listen.mockImplementation(() => new Promise<() => void>((done) => { resolve = done; }));
  tauri.invoke.mockResolvedValue({ quickCommands: { entries: [], history: [], orderVersion: 1, listWidth: 180 } });
  const unlisten = vi.fn();
  const view = render(CommandPanelSurface);
  await waitFor(() => expect(tauri.listen).toHaveBeenCalled());
  view.unmount();
  resolve(unlisten);
  await waitFor(() => expect(unlisten).toHaveBeenCalledTimes(1));
});

it('ignores deferred settings/history response after unmount without follow-up invoke or output', async () => {
  const tauri = tauriMocks();
  let resolveSettings!: (value: unknown) => void;
  let resolveHistory!: (value: unknown) => void;
  tauri.invoke.mockImplementation((command: string) => {
    if (command === 'load_shell_settings') return new Promise((resolve) => { resolveSettings = resolve; });
    if (command === 'list_quick_command_history') return new Promise((resolve) => { resolveHistory = resolve; });
    return Promise.resolve(null);
  });
  tauri.listen.mockResolvedValue(vi.fn());
  const view = render(CommandPanelSurface);
  await waitFor(() => expect(tauri.invoke.mock.calls.some((args) => args[0] === 'load_shell_settings')).toBe(true));
  await waitFor(() => expect(tauri.invoke.mock.calls.some((args) => args[0] === 'list_quick_command_history')).toBe(true));
  const callsBeforeUnmount = tauri.invoke.mock.calls.length;
  view.unmount();
  resolveSettings({ entries: [], history: [], orderVersion: 1, listWidth: 180 });
  resolveHistory([]);
  await Promise.resolve();
  expect(tauri.invoke).toHaveBeenCalledTimes(callsBeforeUnmount);
  expect(view.container.textContent).toBe('');
});

it('ignores a late registered run event after unmount without requesting history', async () => {
  const tauri = tauriMocks();
  let deliver!: (event: { payload: unknown }) => void;
  tauri.listen.mockImplementation(async (_: string, callback: typeof deliver) => { deliver = callback; return vi.fn(); });
  tauri.invoke.mockImplementation((command: string) => command === 'list_quick_command_history'
    ? Promise.resolve([]) : Promise.resolve({ entries: [], history: [], orderVersion: 1, listWidth: 180 }));
  const view = render(CommandPanelSurface);
  await waitFor(() => expect(deliver).toBeTypeOf('function'));
  view.unmount();
  const before = tauri.invoke.mock.calls.length;
  deliver({ payload: { kind: 'finished', commandId: 'cmd', runId: 'late-run' } });
  await Promise.resolve();
  expect(tauri.invoke).toHaveBeenCalledTimes(before);
  expect(view.container.textContent).toBe('');
});

it('close request followed by surface disposal rejects late settings and history completions', async () => {
  const tauri = tauriMocks();
  let resolveSettings!: (value: unknown) => void;
  let resolveHistory!: (value: unknown) => void;
  tauri.listen.mockResolvedValue(vi.fn());
  tauri.invoke.mockImplementation((command: string) => {
    if (command === 'load_shell_settings') return new Promise((resolve) => { resolveSettings = resolve; });
    if (command === 'list_quick_command_history') return new Promise((resolve) => { resolveHistory = resolve; });
    return Promise.resolve(null);
  });
  const view = render(CommandPanelSurface);
  await waitFor(() => expect(tauri.invoke.mock.calls.some((args) => args[0] === 'list_quick_command_history')).toBe(true));
  await fireEvent.click(view.getByRole('button', { name: 'Close quick commands' }));
  expect(tauri.invoke.mock.calls.some((args) => args[0] === 'hide_command_panel')).toBe(true);
  view.unmount(); // Tauri closes the surface; component disposal is the observable lifecycle boundary.
  const before = tauri.invoke.mock.calls.length;
  resolveSettings({ entries: [], history: [], orderVersion: 1, listWidth: 180 });
  resolveHistory([]);
  await Promise.resolve();
  expect(tauri.invoke).toHaveBeenCalledTimes(before);
  expect(view.container.textContent).toBe('');
});
