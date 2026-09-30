import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { expect, it, vi } from 'vitest';
import TopBar from '../../src/components/TopBar.svelte';
import { tauriMocks } from './setup';

vi.mock('@tauri-apps/api/window', async () => {
  const { tauriMocks } = await import('./setup');
  return { getCurrentWindow: () => ({
    label: 'top-bar',
    listen: tauriMocks().listen,
    onDragDropEvent: vi.fn().mockResolvedValue(vi.fn())
  }) };
});

it('outside pointer closes open Search with blank publish then native hide', async () => {
  const tauri = tauriMocks();
  tauri.listen.mockResolvedValue(vi.fn());
  tauri.invoke.mockImplementation((command: string) => {
    if (command === 'load_shell_settings') return Promise.resolve({ ui: { searchMode: 'centeredHotkey', lockTopBarHeight: true }, search: {}, quickCommands: { entries: [], history: [] } });
    return Promise.resolve([]);
  });
  const view = render(TopBar);
  const button = await view.findByRole('button', { name: 'Open search' });
  await fireEvent.click(button);
  await waitFor(() => expect(button.getAttribute('aria-expanded')).toBe('true'));
  await fireEvent.pointerDown(document.body);
  await waitFor(() => expect(tauri.invoke.mock.calls.some(([name]) => name === 'hide_search_panel')).toBe(true));
  expect(tauri.invoke.mock.calls.some(([name, args]) => name === 'publish_search_panel' && args?.payload?.query === '' && args?.payload?.results?.length === 0)).toBe(true);
  view.unmount();
});

it('native Search closed event collapses expanded control without an extra hide request', async () => {
  const tauri = tauriMocks();
  const handlers = new Map<string, () => void>();
  tauri.listen.mockImplementation(async (name: string, callback: () => void) => {
    handlers.set(name, callback);
    return vi.fn();
  });
  tauri.invoke.mockImplementation((command: string) => command === 'load_shell_settings'
    ? Promise.resolve({ ui: { searchMode: 'centeredHotkey', lockTopBarHeight: true }, search: {}, quickCommands: { entries: [], history: [] } })
    : Promise.resolve([]));
  const view = render(TopBar);
  const button = await view.findByRole('button', { name: 'Open search' });
  await fireEvent.click(button);
  await waitFor(() => expect(button.getAttribute('aria-expanded')).toBe('true'));
  await waitFor(() => expect(handlers.has('search-panel:closed')).toBe(true));
  const hidesBefore = tauri.invoke.mock.calls.filter(([name]) => name === 'hide_search_panel').length;
  handlers.get('search-panel:closed')?.();
  await waitFor(() => expect(button.getAttribute('aria-expanded')).toBe('false'));
  expect(tauri.invoke.mock.calls.filter(([name]) => name === 'hide_search_panel')).toHaveLength(hidesBefore);
  view.unmount();
});
