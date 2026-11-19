import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { expect, it, vi } from 'vitest';
import CommandPanelSurface from '../../src/components/CommandPanelSurface.svelte';
import { tauriMocks } from './setup';

function notificationTargetFixture(runId?: string) {
  const tauri = tauriMocks();
  const runs = ['recent', 'older'].map((id, index) => ({
    commandId: 'build', runId: id, startedAtEpochMs: 3000 - 2000 * index,
    finishedAtEpochMs: 3500 - 2000 * index, processId: 10 + index,
    exitCode: 0, running: false, stdout: '', stderr: '', stdoutTruncated: false, stderrTruncated: false,
    transcript: [{ kind: 'stdout', body: `${id} output sentinel`, requestId: null, prompt: null,
      secret: false, redacted: false, sequence: 1, atEpochMs: 3200 - 2000 * index, pending: false }],
  }));
  let target = runId ? { commandId: 'build', runId } : null;
  const listeners = new Map<string, (event: { payload: unknown }) => void>();
  tauri.listen.mockImplementation(async (name, callback) => { listeners.set(name, callback); return vi.fn(); });
  let nextHistory: Promise<typeof runs> | null = null;
  tauri.invoke.mockImplementation(async (command: string) => {
    if (command === 'load_shell_settings') return { quickCommands: {
      entries: [{ id: 'build', label: 'Build', mode: 'direct', targetPath: 'fixture.exe', args: [], commands: [], cwd: null, artifactLocation: null }],
      history: [], orderVersion: 1, listWidth: 180,
    } };
    if (command === 'list_quick_command_history') {
      const response = nextHistory ?? runs;
      nextHistory = null;
      return response;
    }
    if (command === 'take_quick_command_notification_target') {
      const next = target;
      target = null;
      return next;
    }
    return null;
  });
  const view = render(CommandPanelSurface);
  return Object.assign(view, {
    wake(runId: string) { target = { commandId: 'build', runId }; listeners.get('command-panel:notification-target')!({ payload: null }); },
    deferHistory() {
      let resolve!: (rows: typeof runs) => void;
      nextHistory = new Promise(done => { resolve = done; });
      return (retain = true) => resolve(retain ? runs : []);
    },
    historyCalls: () => tauri.invoke.mock.calls.filter(([cmd]) => cmd === 'list_quick_command_history').length,
  });
}

it('opens the exact older run from a notification target retained before panel mount', async () => {
  const view = notificationTargetFixture('older');
  await view.findByRole('button', { name: 'Edit Build' });
  await waitFor(() => {
    expect(view.queryByText('older output sentinel')?.closest('details')?.open).toBe(true);
  });
  expect(view.getByText('recent output sentinel').closest('details')?.open).toBe(false);
});

it('explains evicted notification output without expanding another retained run', async () => {
  const view = notificationTargetFixture('evicted');
  await view.findByRole('button', { name: 'Edit Build' });
  await waitFor(() => expect(view.queryByRole('alert')?.textContent).toMatch(/unavailable|no longer available/i));
  expect(view.container.querySelector('details[open]')).toBeNull();
});

it('does not navigate when the panel hides during a deferred notification history read', async () => {
  const view = notificationTargetFixture();
  await view.findByRole('button', { name: 'Edit Build' });
  const count = view.historyCalls();
  const release = view.deferHistory();
  view.wake('older');
  await waitFor(() => expect(view.historyCalls()).toBe(count + 1));
  const visibility = vi.spyOn(document, 'hidden', 'get').mockReturnValue(true);
  try {
    document.dispatchEvent(new Event('visibilitychange'));
    release();
    // Cross a task boundary so the deferred IPC and DOM update have settled.
    await new Promise(resolve => setTimeout(resolve, 0));
    expect(view.queryByText('older output sentinel')).toBeNull();
  } finally { visibility.mockRestore(); }
});

it('preserves a dirty command label when a retained notification target arrives', async () => {
  const view = notificationTargetFixture();
  await fireEvent.click(await view.findByRole('button', { name: 'Edit Build' }));
  const label = view.getByLabelText('Label') as HTMLInputElement;
  await fireEvent.input(label, { target: { value: 'Unsaved build' } });
  view.wake('older');
  await waitFor(() => expect(view.queryByRole('alert')?.textContent ?? '').toMatch(/edit/i));
  await tick();
  expect(label.value).toBe('Unsaved build');
  expect(view.queryByText('older output sentinel')).toBeNull();
});

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
