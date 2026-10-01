import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { expect, it, vi } from 'vitest';
import SearchPanelSurface from '../../src/components/SearchPanelSurface.svelte';
import { tauriMocks } from './setup';

it('releases listener exactly once when registration resolves after unmount', async () => {
  let resolve!: (unlisten: () => void) => void;
  const tauri = tauriMocks();
  tauri.listen.mockImplementation(() => new Promise<() => void>((done) => { resolve = done; }));
  tauri.invoke.mockResolvedValue(null);
  const unlisten = vi.fn();
  const view = render(SearchPanelSurface);
  await waitFor(() => expect(tauri.listen).toHaveBeenCalled());
  view.unmount();
  resolve(unlisten);
  await waitFor(() => expect(unlisten).toHaveBeenCalledTimes(1));
});

it('emits increasing query sequences for rapid input and Escape hides with interaction intent', async () => {
  const tauri = tauriMocks();
  tauri.listen.mockResolvedValue(vi.fn());
  tauri.invoke.mockResolvedValue(null);
  const view = render(SearchPanelSurface);
  const input = view.getByRole('combobox', { name: 'Search Everything' }) as HTMLInputElement;
  await fireEvent.input(input, { target: { value: 'g' } });
  await fireEvent.input(input, { target: { value: 'git' } });
  const queries = tauri.emitTo.mock.calls.filter((args) => args[1] === 'search-panel:query').map((args) => args[2]);
  expect(queries).toEqual([{ query: 'g', inputSequence: 1 }, { query: 'git', inputSequence: 2 }]);
  expect(input.value).toBe('git');
  await fireEvent.keyDown(input, { key: 'Escape' });
  expect(tauri.emitTo.mock.calls.some((args) => args[1] === 'search-panel:key' && args[2] === 'Escape')).toBe(true);
  expect(tauri.emitTo.mock.calls.some((args) => args[1] === 'search-panel:interaction')).toBe(true);
  expect(tauri.invoke.mock.calls.some((args) => args[0] === 'hide_search_panel')).toBe(true);
  view.unmount();
});

it('ignores a deferred fallback response after a newer event and keeps the current query', async () => {
  const tauri = tauriMocks();
  let deliver!: (event: { payload: unknown }) => void;
  let resolveFallback!: (payload: unknown) => void;
  tauri.listen.mockImplementation(async (_: string, callback: typeof deliver) => { deliver = callback; return vi.fn(); });
  tauri.invoke.mockImplementation((command: string) => command === 'get_search_panel_payload'
    ? new Promise((resolve) => { resolveFallback = resolve; }) : Promise.resolve(null));
  const view = render(SearchPanelSurface);
  await waitFor(() => expect(tauri.listen).toHaveBeenCalled());
  await waitFor(() => expect(tauri.invoke.mock.calls.some((args) => args[0] === 'get_search_panel_payload')).toBe(true));
  deliver({ payload: { query: 'newer', results: [], selectedIndex: -1, statusMessage: 'newer results', presentation: 'centered' } });
  resolveFallback({ query: 'old', results: [], selectedIndex: -1, statusMessage: 'stale results', presentation: 'centered' });
  await waitFor(() => expect((view.getByRole('combobox') as HTMLInputElement).value).toBe('newer'));
  expect(view.queryByText('stale results')).toBeNull();
  view.unmount();
});

it('Escape blurs the input and cancels a pending fallback response', async () => {
  const tauri = tauriMocks();
  tauri.listen.mockResolvedValue(vi.fn());
  let resolveFallback!: (payload: unknown) => void;
  tauri.invoke.mockImplementation((command: string) => command === 'get_search_panel_payload'
    ? new Promise((resolve) => { resolveFallback = resolve; }) : Promise.resolve(null));
  const view = render(SearchPanelSurface);
  await waitFor(() => expect(tauri.invoke.mock.calls.some((args) => args[0] === 'get_search_panel_payload')).toBe(true));
  const input = view.getByRole('combobox') as HTMLInputElement;
  input.focus();
  expect(document.activeElement).toBe(input);
  await fireEvent.keyDown(input, { key: 'Escape' });
  expect(document.activeElement).not.toBe(input);
  resolveFallback({ query: 'should-not-reappear', results: [], selectedIndex: -1, statusMessage: 'stale fallback', presentation: 'centered' });
  await Promise.resolve();
  expect((view.getByRole('combobox') as HTMLInputElement).value).not.toBe('should-not-reappear');
  expect(view.queryByText('stale fallback')).toBeNull();
  view.unmount();
});
