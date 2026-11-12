import { fireEvent, render, waitFor, within } from '@testing-library/svelte';
import { expect, it, vi } from 'vitest';
import SearchPanelSurface from '../../src/components/SearchPanelSurface.svelte';
import { tauriMocks } from './setup';
import type { SearchPanelPayload, SearchPanelResult } from '../../src/lib/searchPanel';

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
   await waitFor(() => expect((view.getByRole('combobox', { name: 'Search Everything' }) as HTMLInputElement).value).toBe('newer'));
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
  const input = view.getByRole('combobox', { name: 'Search Everything' }) as HTMLInputElement;
  input.focus();
  expect(document.activeElement).toBe(input);
  await fireEvent.keyDown(input, { key: 'Escape' });
  expect(document.activeElement).not.toBe(input);
  resolveFallback({ query: 'should-not-reappear', results: [], selectedIndex: -1, statusMessage: 'stale fallback', presentation: 'centered' });
  await Promise.resolve();
  expect((view.getByRole('combobox', { name: 'Search Everything' }) as HTMLInputElement).value).not.toBe('should-not-reappear');
  expect(view.queryByText('stale fallback')).toBeNull();
  view.unmount();
});

const scopedResults: SearchPanelResult[] = [
  { id: 'folder', kind: 'folder', title: 'Local folder', path: 'C:\\work', providerId: 'local' },
  { id: 'duplicate', kind: 'app', title: 'First app', providerId: 'apps', recordKey: 'app:first' },
  { id: 'window', kind: 'window', title: 'Open window', providerId: 'openWindows' },
  { id: 'setting', kind: 'setting', title: 'Sound setting', providerId: 'settings' },
  { id: 'command', kind: 'command', title: 'Saved command', providerId: 'commands' },
  { id: 'everything-folder', kind: 'folder', title: 'Indexed folder', providerId: 'everything', path: 'C:\\indexed' },
  { id: 'everything-file', kind: 'file', title: 'Indexed file', providerId: 'everything' },
  { id: 'local-file', kind: 'file', title: 'Local file', providerId: 'local' },
  { id: 'duplicate', kind: 'app', title: 'Second app', providerId: 'apps', recordKey: 'app:second' },
  { id: 'web', kind: 'web', title: 'Web result', providerId: 'web' }
].map((result) => ({ subtitle: '', terms: '', priority: 0, ...result })) as SearchPanelResult[];

async function mountScopedSearch(presentation: 'centered' | 'anchored' = 'centered') {
  const tauri = tauriMocks();
  let deliver!: (event: { payload: SearchPanelPayload }) => void;
  const payload: SearchPanelPayload = { query: 'test', results: scopedResults, selectedIndex: 0, statusMessage: '', presentation };
  tauri.listen.mockImplementation(async (_event, callback) => { deliver = callback; return vi.fn(); });
  tauri.invoke.mockImplementation(async (command) => command === 'get_search_panel_payload' ? payload : null);
  HTMLElement.prototype.scrollIntoView = vi.fn();
  const view = render(SearchPanelSurface);
  await waitFor(() => expect(within(view.getByRole('listbox')).getAllByRole('option').length).toBe(scopedResults.length));
  return { view, tauri, refresh: (changes: Partial<SearchPanelPayload> = {}) => deliver({ payload: { ...payload, ...changes } }) };
}

function chooseScope(view: ReturnType<typeof render>, label: string) {
  const select = view.container.querySelector('select');
  expect(select, 'centered search must expose a native scope select').not.toBeNull();
  const option = Array.from(select!.options).find((item) => item.text === label);
  expect(option, `scope ${label} exists`).toBeDefined();
  return fireEvent.change(select!, { target: { value: option!.value } });
}

it.each([
  ['Apps', ['First app', 'Second app']],
  ['Windows', ['Open window']],
  ['Settings', ['Sound setting']],
  ['Folders', ['Local folder', 'Indexed folder']],
  ['Commands', ['Saved command']],
  ['Everything', ['Indexed folder', 'Indexed file']]
])('filters supplied results by %s without changing query IPC; All restores every kind', async (scope, titles) => {
  const { view, tauri } = await mountScopedSearch();
  await chooseScope(view, scope as string);
  const rows = within(view.getByRole('listbox')).getAllByRole('option');
  expect(rows).toHaveLength(titles.length);
  for (const title of titles) expect(rows.some((row) => row.textContent?.includes(title))).toBe(true);
  expect(tauri.emitTo.mock.calls.filter((args) => args[1] === 'search-panel:query')).toHaveLength(0);
  await chooseScope(view, 'All');
  expect(within(view.getByRole('listbox')).getAllByRole('option')).toHaveLength(scopedResults.length);
});

it('activates first scoped row when backend selection is hidden, preserving original index and record key', async () => {
  const { view, tauri } = await mountScopedSearch();
  await chooseScope(view, 'Apps');
  const input = view.getByRole('combobox', { name: 'Search Everything' });
  await fireEvent.keyDown(input, { key: 'Enter' });
  expect(tauri.emitTo.mock.calls.find((args) => args[1] === 'search-panel:activate')?.[2]).toMatchObject({ id: 'duplicate', resultIndex: 1, recordKey: 'app:first' });
  expect(tauri.emitTo.mock.calls.some((args) => args[1] === 'search-panel:key' && args[2] === 'Enter')).toBe(false);
  await fireEvent.keyDown(input, { key: 'Enter', ctrlKey: true });
  expect(tauri.emitTo.mock.calls.some((args) => args[1] === 'search-panel:pin-folder')).toBe(false);
  await fireEvent.keyDown(input, { key: 'ArrowDown' });
  expect(tauri.emitTo.mock.calls.filter((args) => args[1] === 'search-panel:select').at(-1)?.[2]).toMatchObject({ resultIndex: 8, recordKey: 'app:second' });
  await fireEvent.click(within(view.getByRole('listbox')).getAllByRole('option')[1]);
  expect(tauri.emitTo.mock.calls.filter((args) => args[1] === 'search-panel:select').at(-1)?.[2]).toMatchObject({ resultIndex: 8, recordKey: 'app:second' });
});

it('does not activate or pin hidden backend selection when scoped results are empty', async () => {
  const { view, tauri, refresh } = await mountScopedSearch();
  await chooseScope(view, 'Apps');
  refresh({ results: [scopedResults[0]], selectedIndex: 0 });
  await waitFor(() => expect(within(view.getByRole('listbox')).queryAllByRole('option')).toHaveLength(0));
  const input = view.getByRole('combobox', { name: 'Search Everything' });
  await fireEvent.keyDown(input, { key: 'Enter' });
  await fireEvent.keyDown(input, { key: 'Enter', ctrlKey: true });
  expect(tauri.emitTo.mock.calls.some((args) => ['search-panel:activate', 'search-panel:pin-folder'].includes(args[1]))).toBe(false);
  expect(tauri.emitTo.mock.calls.some((args) => args[1] === 'search-panel:key' && args[2] === 'Enter')).toBe(false);
});

it('preserves native select focus and scope across payload refresh; clear still focuses query', async () => {
  const { view, refresh, tauri } = await mountScopedSearch();
  await chooseScope(view, 'Everything');
  const select = view.container.querySelector('select')!;
  select.focus();
  refresh({ query: 'refreshed', selectedIndex: 8 });
  await waitFor(() => expect((view.getByRole('combobox', { name: 'Search Everything' }) as HTMLInputElement).value).toBe('refreshed'));
  expect(document.activeElement).toBe(select);
  expect(select.selectedOptions[0].text).toBe('Everything');
  expect(within(view.getByRole('listbox')).getAllByRole('option')).toHaveLength(2);
  await fireEvent.click(view.getByRole('button', { name: 'Clear search' }));
  await waitFor(() => expect(document.activeElement).toBe(view.getByRole('combobox', { name: 'Search Everything' })));
  expect(tauri.emitTo.mock.calls.filter((args) => args[1] === 'search-panel:query').at(-1)?.[2]).toMatchObject({ query: '' });
});

it('leaves anchored presentation unfiltered and without the centered scope picker', async () => {
  const { view } = await mountScopedSearch('anchored');
  expect(view.container.querySelector('select')).toBeNull();
  expect(view.queryByRole('combobox')).toBeNull();
  expect(within(view.getByRole('listbox')).getAllByRole('option')).toHaveLength(scopedResults.length);
});
