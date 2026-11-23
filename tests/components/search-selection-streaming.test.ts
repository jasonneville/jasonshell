// Review M1 (component, mocked IPC): a row the user chose (web:search or a weak quick command) is
// followed by identity across streamed payloads. While an interim payload lacks it nothing is
// selected (Enter activates nothing); the final payload selects it again, or falls back to the
// implicit choice when it is gone. An arrow choice during the gap is a new choice; a new query resets.
import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import TopBar from '../../src/components/TopBar.svelte';
import actionFixture from '../fixtures/search-actions.json';
import { tauriMocks } from './setup';

vi.mock('@tauri-apps/api/window', async () => {
  const { tauriMocks } = await import('./setup');
  return { getCurrentWindow: () => ({
    label: 'top-bar',
    listen: tauriMocks().listen,
    onDragDropEvent: vi.fn().mockResolvedValue(vi.fn())
  }) };
});
vi.mock('@tauri-apps/api/webviewWindow', async () => {
  const { tauriMocks } = await import('./setup');
  return { getCurrentWebviewWindow: () => ({ label: 'top-bar',
    listen: (name: string, callback: (event: { payload: unknown }) => void) =>
      tauriMocks().listen(name, callback, { target: { kind: 'WebviewWindow', label: 'top-bar' } }) }) };
});

type Handler = (event: { payload: unknown }) => void;
type Row = Record<string, unknown>;
const A = actionFixture.actions;
const generatedAt = '2026-11-23T10:00:00.000Z';

function row(id: string, kind: string, providerId: string, matchReason: string, action: unknown, recordKey = id): Row {
  return { id, providerId, kind, title: id, terms: [], aliases: [], score: 100, matchReason, recordKey, action };
}
const file = row('everything:a', 'file', 'everything', 'prefix', A.openFile);
const file2 = row('everything:b', 'file', 'everything', 'prefix', A.openFile);
const webSearch = (query: string) => row('web:search', 'web', 'web', 'webSearch', A.openWebSearch, `web:search:${query}`);
const weakQuick = row('quick:w', 'command', 'commands', 'wordPrefix', A.runCommandQuick);

let handlers: Map<string, Handler>;
let pending: Array<{ request: { query: string; sequence: number }; resolve: (rows: Row[]) => void }>;
let holdNext: boolean;
let initialRows: Row[];

beforeEach(() => { handlers = new Map(); pending = []; holdNext = false; initialRows = []; });
afterEach(() => { vi.unstubAllGlobals(); });

function response(request: { query: string; sequence: number }, results: Row[]) {
  return { query: request.query, sequence: request.sequence, results, providerTimings: [], health: [], generatedAt };
}

async function mountSearch(rows: Row[], query: string) {
  initialRows = rows;
  const tauri = tauriMocks();
  tauri.listen.mockImplementation(async (name: string, callback: Handler) => { handlers.set(name, callback); return vi.fn(); });
  tauri.invoke.mockImplementation(async (command: string, args?: { request?: { query: string; sequence: number } }) => {
    if (command === 'load_shell_settings') {
      return { ui: { searchMode: 'centeredHotkey', lockTopBarHeight: true }, search: {}, quickCommands: { entries: [], history: [] } };
    }
    if (command === 'search_engine') {
      const request = args!.request!;
      if (holdNext) {
        holdNext = false;
        return new Promise((resolve) => pending.push({ request, resolve: (results) => resolve(response(request, results)) }));
      }
      return response(request, initialRows);
    }
    return [];
  });
  const view = render(TopBar);
  await fireEvent.click(await view.findByRole('button', { name: 'Open search' }));
  await waitFor(() => expect(handlers.has('search-panel:query')).toBe(true));
  handlers.get('search-panel:query')!({ payload: { query, inputSequence: 1 } });
  await waitFor(() => expect(lastPayload(tauri)?.results?.length).toBe(rows.length));
  return { view, tauri };
}

function lastPayload(tauri: ReturnType<typeof tauriMocks>) {
  const published = tauri.invoke.mock.calls.filter(([name]) => name === 'publish_search_panel');
  return published.at(-1)?.[1]?.payload as { query: string; selectedIndex: number; results: Array<{ id: string }>; phase?: string } | undefined;
}

function select(row: Row, resultIndex: number) {
  handlers.get('search-panel:select')!({ payload: { id: row.id, recordKey: row.recordKey, resultIndex } });
}

async function startRetry(tauri: ReturnType<typeof tauriMocks>, generation: number) {
  holdNext = true;
  handlers.get('search-index:refreshed')!({ payload: { providerId: 'apps', entryCount: 5, generatedAtEpochSecs: generation } });
  await waitFor(() => expect(pending.length).toBeGreaterThan(0));
  void tauri;
  return pending.shift()!;
}

function interim(request: { query: string; sequence: number }, results: Row[]) {
  handlers.get('search-engine:progress')!({ payload: {
    query: request.query, sequence: request.sequence, phase: 'local', results,
    providerTimings: [], statusMessage: 'local results', generatedAt
  } });
}

function enterKey() {
  handlers.get('search-panel:key')!({ payload: 'Enter' });
}

function sideEffects(tauri: ReturnType<typeof tauriMocks>) {
  return tauri.invoke.mock.calls.filter(([name]) =>
    ['open_search_url', 'run_search_quick_command', 'copy_search_text', 'open_shell_path', 'launch_app_path'].includes(String(name)));
}

it.each([
  ['web:search', () => webSearch('foo'), (query: string) => webSearch(query)],
  ['a weak quick command', () => weakQuick, () => weakQuick]
])('a chosen %s survives an interim payload without it and is reselected by the final payload', async (_name, make) => {
  const chosen = make();
  const { tauri } = await mountSearch([file, chosen], 'foo');
  expect(lastPayload(tauri)?.selectedIndex).toBe(0);
  select(chosen, 1);
  await waitFor(() => expect(lastPayload(tauri)?.selectedIndex).toBe(1));

  const retry = await startRetry(tauri, 100);
  interim(retry.request, [file, file2]);
  await waitFor(() => expect(lastPayload(tauri)?.results.length).toBe(2));
  expect(lastPayload(tauri)?.selectedIndex).toBe(-1);

  enterKey();
  await new Promise((resolve) => setTimeout(resolve, 100));
  expect(sideEffects(tauri)).toHaveLength(0);

  retry.resolve([file, file2, chosen]);
  await waitFor(() => expect(lastPayload(tauri)?.results.length).toBe(3));
  await waitFor(() => expect(lastPayload(tauri)?.selectedIndex).toBe(2));
});

it('a final payload that lacks the chosen row falls back to the implicit choice', async () => {
  const chosen = webSearch('foo');
  const { tauri } = await mountSearch([file, chosen], 'foo');
  select(chosen, 1);
  await waitFor(() => expect(lastPayload(tauri)?.selectedIndex).toBe(1));
  const retry = await startRetry(tauri, 200);
  interim(retry.request, [file, file2]);
  await waitFor(() => expect(lastPayload(tauri)?.selectedIndex).toBe(-1));
  retry.resolve([file, file2]);
  await waitFor(() => expect(lastPayload(tauri)?.selectedIndex).toBe(0));
});

it('arrowing while nothing is selected is a new choice that the final payload follows', async () => {
  const chosen = webSearch('foo');
  const { tauri } = await mountSearch([file, chosen], 'foo');
  select(chosen, 1);
  const retry = await startRetry(tauri, 300);
  interim(retry.request, [file, file2]);
  await waitFor(() => expect(lastPayload(tauri)?.selectedIndex).toBe(-1));
  select(file2, 1);
  await waitFor(() => expect(lastPayload(tauri)?.selectedIndex).toBe(1));
  retry.resolve([file, chosen, file2]);
  await waitFor(() => expect(lastPayload(tauri)?.results.length).toBe(3));
  await waitFor(() => expect(lastPayload(tauri)?.selectedIndex).toBe(2));
});

it('a new query resets the choice even when the same row identity appears again', async () => {
  const chosen = weakQuick;
  const { tauri } = await mountSearch([file, chosen], 'foo');
  select(chosen, 1);
  await waitFor(() => expect(lastPayload(tauri)?.selectedIndex).toBe(1));
  handlers.get('search-panel:query')!({ payload: { query: 'bar', inputSequence: 2 } });
  await waitFor(() => expect(lastPayload(tauri)?.query).toBe('bar'));
  await waitFor(() => expect(lastPayload(tauri)?.phase).toBe('complete'));
  expect(lastPayload(tauri)?.selectedIndex).toBe(0);
});
