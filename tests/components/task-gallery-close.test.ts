import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { beforeEach, expect, it, vi } from 'vitest';

// DOM/IPC contract evidence only: no native HWNDs, Win32 dispatch, or shell startup.
const bridge = vi.hoisted(() => ({
  handlers: new Map<string, (event: { payload: unknown }) => unknown>(),
  invoke: vi.fn(), emit: vi.fn(), sequence: 0
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: bridge.invoke }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string, handler: (event: { payload: unknown }) => unknown) => {
    bridge.handlers.set(name, handler);
    return () => bridge.handlers.delete(name);
  },
  emit: bridge.emit, emitTo: bridge.emit
}));

import TaskGallerySurface from '../../src/components/TaskGallerySurface.svelte';
import { TASKBAR_REFRESH_WINDOWS_EVENT } from '../../src/lib/taskbarUi';

const windows = ['Alpha', 'Beta', 'Gamma'].map((title, index) => ({
  hwnd: String(101 + index), title, processName: 'Editor', processId: 42,
  iconDataUrl: '', isActive: index === 0, isMinimized: false
}));
type Row = typeof windows[number];
async function publish(nonce = 'session-A', rows: Row[] = windows, focusGallery = false) {
  await bridge.handlers.get('task-gallery:open')?.({ payload: {
    nonce, groupKey: 'editor', label: 'Editor', focusGallery, windows: rows
  } });
  await tick();
}
async function open() {
  const view = render(TaskGallerySurface);
  await waitFor(() => expect(bridge.handlers.has('task-gallery:open')).toBe(true));
  await publish();
  return view;
}
function activation(title: string): HTMLButtonElement {
  const button = screen.getByText(title).closest('button');
  expect(button).not.toBeNull();
  return button as HTMLButtonElement;
}
function close(title: string): HTMLButtonElement {
  return screen.getByRole('button', { name: `Close ${title}` }) as HTMLButtonElement;
}
function calls(command: string) {
  return bridge.invoke.mock.calls.filter(([name]) => name === command);
}
function deferred() {
  let resolve!: () => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<void>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
beforeEach(() => {
  bridge.handlers.clear(); bridge.invoke.mockReset(); bridge.emit.mockReset(); bridge.sequence = 0;
  bridge.invoke.mockImplementation(async (name: string) => name === 'allocate_task_preview_request_id' ? ++bridge.sequence : undefined);
  bridge.emit.mockResolvedValue(undefined);
});

it('each tile has sibling activation and window-named Close controls; selected pair alone is tabbable', async () => {
  await open();
  for (const [index, row] of windows.entries()) {
    const primary = activation(row.title);
    const x = close(row.title);
    expect(primary.contains(x)).toBe(false);
    expect(primary.parentElement).toBe(x.parentElement);
    expect(primary.tabIndex).toBe(index === 0 ? 0 : -1);
    expect(x.tabIndex).toBe(index === 0 ? 0 : -1);
    expect(x.type).toBe('button');
  }
});

it.each(['click', 'Enter', ' '] as const)('%s on X dispatches captured exact nonce/HWND once, never activates', async (gesture) => {
  await open();
  const x = close('Beta');
  if (gesture === 'click') await fireEvent.click(x);
  else {
    x.focus();
    await userEvent.setup().keyboard(gesture === ' ' ? '[Space]' : '[Enter]');
  }
  await waitFor(() => expect(calls('close_task_gallery_window')).toEqual([
    ['close_task_gallery_window', { args: { hwnd: '102', nonce: 'session-A' } }]
  ]));
  expect(calls('activate_task_gallery_window')).toHaveLength(0);
});

it('Tab moves selected activation to X; arrows/Home/End retain roving navigation', async () => {
  await open();
  activation('Alpha').focus();
  await userEvent.setup().tab();
  expect(document.activeElement).toBe(close('Alpha'));
  await fireEvent.keyDown(close('Alpha'), { key: 'ArrowRight' });
  await waitFor(() => expect(document.activeElement).toBe(activation('Beta')));
  expect(close('Beta').tabIndex).toBe(0);
  expect(close('Alpha').tabIndex).toBe(-1);
  await fireEvent.keyDown(activation('Beta'), { key: 'End' });
  await waitFor(() => expect(document.activeElement).toBe(activation('Gamma')));
  await fireEvent.keyDown(activation('Gamma'), { key: 'Home' });
  await waitFor(() => expect(document.activeElement).toBe(activation('Alpha')));
});

it('resolved close emits refresh but keeps all three tiles and gallery until authoritative removal', async () => {
  await open();
  await fireEvent.click(close('Beta'));
  await waitFor(() => expect(bridge.emit).toHaveBeenCalledWith(TASKBAR_REFRESH_WINDOWS_EVENT));
  expect(screen.getAllByRole('button', { name: /^Close / })).toHaveLength(3);
  expect(calls('hide_task_gallery')).toHaveLength(0);
  expect(calls('hide_task_gallery_window_preview')).toHaveLength(0);
  await publish('session-A', [windows[0], windows[2]]);
  expect(screen.queryByText('Beta')).toBeNull();
  expect(screen.getAllByRole('button', { name: /^Close / })).toHaveLength(2);
});

it('pending duplicate suppression is per target; rejection leaves rows and allows retry', async () => {
  const pending = deferred();
  bridge.invoke.mockImplementation((name: string) => name === 'close_task_gallery_window' ? pending.promise : Promise.resolve(++bridge.sequence));
  await open();
  close('Alpha').focus();
  await tick();
  await fireEvent.click(close('Alpha'));
  await fireEvent.click(close('Alpha'));
  await fireEvent.click(close('Beta'));
  expect(calls('close_task_gallery_window')).toHaveLength(2);
  expect(close('Alpha').getAttribute('aria-busy')).toBe('true');
  expect(close('Alpha').getAttribute('aria-disabled')).toBe('true');
  expect(document.activeElement).toBe(close('Alpha'));
  pending.reject(new Error('WM_CLOSE access denied'));
  await waitFor(() => expect(close('Alpha').getAttribute('aria-busy')).toBe('false'));
  expect(screen.getByRole('status').textContent).toMatch(/could not|failed|error|denied/i);
  expect(bridge.emit.mock.calls.filter(([name]) => name === TASKBAR_REFRESH_WINDOWS_EVENT)).toHaveLength(0);
  expect(screen.getAllByRole('button', { name: /^Close / })).toHaveLength(3);
  bridge.invoke.mockResolvedValue(undefined);
  await fireEvent.click(close('Alpha'));
  await waitFor(() => expect(calls('close_task_gallery_window')).toHaveLength(3));
});

it('same-session removal preserves surviving DOM focus/preview and clears stale context menu', async () => {
  await open();
  activation('Gamma').focus();
  await waitFor(() => expect(calls('show_task_gallery_window_preview').length).toBeGreaterThan(0));
  await fireEvent.contextMenu(activation('Beta'));
  expect(screen.getByRole('menu', { name: 'Beta actions' })).toBeTruthy();
  const survivingButton = activation('Gamma');
  survivingButton.focus();
  const shownBefore = calls('show_task_gallery_window_preview').length;
  await publish('session-A', [windows[0], windows[2]]);
  expect(screen.queryByRole('menu')).toBeNull();
  expect(document.activeElement).toBe(survivingButton);
  expect(close('Gamma').tabIndex).toBe(0);
  expect(calls('hide_task_gallery_window_preview')).toHaveLength(0);
  expect(calls('show_task_gallery_window_preview')).toHaveLength(shownBefore);
});

it('authoritative removal of previewed target hides that preview and reconciles selected pair', async () => {
  await open();
  activation('Beta').focus();
  await waitFor(() => expect(calls('show_task_gallery_window_preview').length).toBeGreaterThan(0));
  await publish('session-A', [windows[0], windows[2]]);
  expect(calls('hide_task_gallery_window_preview')).toHaveLength(1);
  expect(calls('hide_task_gallery_window_preview')[0][1]).toMatchObject({ nonce: 'session-A', hwnd: '102' });
  expect(activation('Alpha').tabIndex).toBe(0);
  expect(close('Alpha').tabIndex).toBe(0);
});

it.each(['replacement', 'closed', 'Escape hide', 'unmount'] as const)('late settlement after %s is inert', async (boundary) => {
  const pending = deferred();
  bridge.invoke.mockImplementation((name: string) => name === 'close_task_gallery_window' ? pending.promise : Promise.resolve(++bridge.sequence));
  const view = await open();
  await fireEvent.click(close('Alpha'));
  if (boundary === 'replacement') {
    await publish('session-B');
    await fireEvent.click(close('Alpha'));
    expect(calls('close_task_gallery_window')).toHaveLength(2);
    expect(calls('close_task_gallery_window')[1][1]).toEqual({ args: { nonce: 'session-B', hwnd: '101' } });
  } else if (boundary === 'closed') {
    await bridge.handlers.get('task-gallery:closed')?.({ payload: { nonce: 'session-A' } });
    await tick();
  } else if (boundary === 'Escape hide') {
    await fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => expect(calls('hide_task_gallery')).toHaveLength(1));
  } else view.unmount();
  // In replacement mode both sessions shared this promise: only current B may emit.
  pending.resolve();
  await tick(); await Promise.resolve(); await tick();
  const refreshes = bridge.emit.mock.calls.filter(([name]) => name === TASKBAR_REFRESH_WINDOWS_EVENT);
  expect(refreshes).toHaveLength(boundary === 'replacement' ? 1 : 0);
  expect(calls('activate_task_gallery_window')).toHaveLength(0);
});

it('late rejection after replacement cannot report old error or disturb a new session', async () => {
  const pending = deferred();
  bridge.invoke.mockImplementation((name: string) => name === 'close_task_gallery_window' ? pending.promise : Promise.resolve(++bridge.sequence));
  await open();
  await fireEvent.click(close('Alpha'));
  await publish('session-B');
  const errors = vi.spyOn(console, 'error').mockImplementation(() => {});
  pending.reject(new Error('old session denied'));
  await tick(); await Promise.resolve(); await tick();
  expect(errors).not.toHaveBeenCalled();
  expect(screen.getByRole('status').textContent).toBe('');
  expect(close('Alpha').getAttribute('aria-busy')).toBe('false');
  expect(close('Alpha').disabled).toBe(false);
  expect(bridge.emit.mock.calls.filter(([name]) => name === TASKBAR_REFRESH_WINDOWS_EVENT)).toHaveLength(0);
});

it.each(['Enter', ' '] as const)('activation %s retains active-window minimize intent, not close', async (key) => {
  await open();
  activation('Alpha').focus();
  await tick();
  await fireEvent.keyDown(activation('Alpha'), { key });
  await waitFor(() => expect(calls('activate_task_gallery_window')).toEqual([
    ['activate_task_gallery_window', { args: { hwnd: '101', nonce: 'session-A', minimizeIfActive: true } }]
  ]));
  expect(calls('close_task_gallery_window')).toHaveLength(0);
});

it('context activation/minimize and Escape retain existing behavior without close dispatch', async () => {
  await open();
  activation('Beta').focus();
  await tick();
  await fireEvent.keyDown(activation('Beta'), { key: 'F10', shiftKey: true });
  await fireEvent.click(screen.getByText('Restore / Switch'));
  await waitFor(() => expect(calls('activate_task_gallery_window')[0][1]).toEqual({ args: { hwnd: '102', nonce: 'session-A', minimizeIfActive: false } }));
  await fireEvent.contextMenu(activation('Alpha'));
  await fireEvent.click(screen.getByText('Minimize'));
  await waitFor(() => expect(calls('activate_task_gallery_window')[1][1]).toEqual({ args: { hwnd: '101', nonce: 'session-A', minimizeIfActive: true } }));
  await fireEvent.keyDown(window, { key: 'Escape' });
  await waitFor(() => expect(calls('hide_task_gallery').length).toBeGreaterThan(0));
  expect(calls('close_task_gallery_window')).toHaveLength(0);
});
