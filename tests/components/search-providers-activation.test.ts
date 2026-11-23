// Tier 1 (PRD-01/02/03) component evidence: how the top-bar webview (which receives
// search-panel:activate) dispatches Calculator, Web, Bookmark and Quick Command rows.
// Mocked IPC only; no native shell.
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

const calculatorRow: Row = {
  id: 'calculator:result', providerId: 'calculator', kind: 'calculator', title: '42', subtitle: '0x2A - 0b101010',
  action: A.copyText, terms: [], aliases: [], score: 9000, matchReason: 'calculator', recordKey: 'calculator:result'
};
const webRow: Row = {
  id: 'web:open-url', providerId: 'web', kind: 'web', title: 'Open https://github.com', subtitle: 'Open URL',
  action: A.openWebUrl, terms: [], aliases: [], score: 500, matchReason: 'url', recordKey: 'web:open-url'
};
const bookmarkRow: Row = {
  id: 'bookmark:https://svelte.dev/', providerId: 'bookmarks', kind: 'bookmark', title: 'Svelte docs', subtitle: 'https://svelte.dev/',
  action: A.openBookmark, terms: [], aliases: [], score: 400, matchReason: 'token', recordKey: 'bookmark:https://svelte.dev/'
};
const quickRow: Row = {
  id: 'quick:build', providerId: 'commands', kind: 'command', title: 'Build project', subtitle: 'Quick command',
  action: A.runCommandQuick, terms: [], aliases: [], score: 800, matchReason: 'prefix', recordKey: 'quick:build'
};

let handlers: Map<string, Handler>;
let copyImpl: () => Promise<unknown>;
let rowsForQuery: Row[];
let quickRunImpl: () => Promise<unknown>;

beforeEach(() => {
  handlers = new Map();
  rowsForQuery = [];
  copyImpl = async () => null;
  quickRunImpl = async () => ({ outcome: 'started', runId: 'run-1', processId: 1 });
});
afterEach(() => { vi.unstubAllGlobals(); });

async function openSearchWith(rows: Row[], query: string) {
  rowsForQuery = rows;
  const tauri = tauriMocks();
  tauri.listen.mockImplementation(async (name: string, callback: Handler) => { handlers.set(name, callback); return vi.fn(); });
  tauri.invoke.mockImplementation(async (command: string, args?: { request?: { query?: string; sequence?: number } }) => {
    if (command === 'load_shell_settings') {
      return { ui: { searchMode: 'centeredHotkey', lockTopBarHeight: true }, search: {}, quickCommands: { entries: [], history: [] } };
    }
    if (command === 'search_engine') {
      return { query: args?.request?.query ?? '', sequence: args?.request?.sequence ?? 0, results: rowsForQuery,
        providerTimings: [], health: [], generatedAt };
    }
    if (command === 'run_search_quick_command') return quickRunImpl();
    if (command === 'copy_search_text') return copyImpl();
    return [];
  });
  const view = render(TopBar);
  const button = await view.findByRole('button', { name: 'Open search' });
  await fireEvent.click(button);
  await waitFor(() => expect(handlers.has('search-panel:query')).toBe(true));
  await waitFor(() => expect(handlers.has('search-panel:activate')).toBe(true));
  handlers.get('search-panel:query')!({ payload: { query, inputSequence: 1 } });
  await waitFor(() => expect(tauri.invoke.mock.calls.some(([name]) => name === 'search_engine')).toBe(true));
  await waitFor(() => {
    const published = tauri.invoke.mock.calls.filter(([name]) => name === 'publish_search_panel').map(([, args]) => args?.payload);
    expect(published.some((payload) => payload?.results?.length === rows.length)).toBe(true);
  });
  return { view, tauri, activate: (payload: unknown) => handlers.get('search-panel:activate')!({ payload }) };
}

function calls(tauri: ReturnType<typeof tauriMocks>, command: string) {
  return tauri.invoke.mock.calls.filter(([name]) => name === command);
}

it('activating a calculator row copies exactly the plain result text and closes the panel', async () => {
  const { tauri, activate } = await openSearchWith([calculatorRow], '6*7');
  activate({ id: 'calculator:result' });
  await waitFor(() => expect(calls(tauri, 'copy_search_text')).toHaveLength(1));
  expect(calls(tauri, 'copy_search_text')[0][1]).toEqual({ text: '42' });
  await waitFor(() => expect(calls(tauri, 'hide_search_panel').length).toBeGreaterThan(0));
  for (const unrelated of ['open_shell_path', 'launch_app_path', 'run_quick_command', 'run_search_quick_command', 'open_search_url']) {
    expect(calls(tauri, unrelated), unrelated).toHaveLength(0);
  }
});

it('a failed clipboard write keeps the panel open and reports the failure truthfully', async () => {
  copyImpl = async () => { throw new Error('clipboard busy'); };
  const { tauri, activate } = await openSearchWith([calculatorRow], '6*7');
  const hidesBefore = calls(tauri, 'hide_search_panel').length;
  activate({ id: 'calculator:result' });
  await waitFor(() => expect(calls(tauri, 'copy_search_text')).toHaveLength(1));
  await waitFor(() => {
    const statuses = calls(tauri, 'publish_search_panel').map(([, args]) => String(args?.payload?.statusMessage ?? ''));
    expect(statuses.some((status) => /clipboard busy|could not copy|copy failed|not copied/i.test(status))).toBe(true);
  });
  expect(calls(tauri, 'hide_search_panel')).toHaveLength(hidesBefore);
});

it.each([
  ['web row', webRow, 'web:open-url', 'https://github.com/'],
  ['bookmark row', bookmarkRow, 'bookmark:https://svelte.dev/', 'https://svelte.dev/']
])('activating a %s asks the backend to open exactly that http(s) URL and nothing else', async (_name, row, id, url) => {
  const { tauri, activate } = await openSearchWith([row], 'q');
  activate({ id });
  await waitFor(() => expect(calls(tauri, 'open_search_url')).toHaveLength(1));
  expect(calls(tauri, 'open_search_url')[0][1]).toEqual({ url });
  expect(calls(tauri, 'open_shell_path')).toHaveLength(0);
  expect(calls(tauri, 'copy_search_text')).toHaveLength(0);
  await waitFor(() => expect(calls(tauri, 'hide_search_panel').length).toBeGreaterThan(0));
});

it('a backend refusal to open a URL is surfaced and the panel stays open', async () => {
  const { tauri, activate } = await openSearchWith([webRow], 'github.com');
  const original = tauri.invoke.getMockImplementation()!;
  tauri.invoke.mockImplementation(async (command: string, args?: unknown) => {
    if (command === 'open_search_url') throw new Error('blocked url');
    return original(command, args);
  });
  const hidesBefore = calls(tauri, 'hide_search_panel').length;
  activate({ id: 'web:open-url' });
  await waitFor(() => expect(calls(tauri, 'open_search_url')).toHaveLength(1));
  await waitFor(() => {
    const statuses = calls(tauri, 'publish_search_panel').map(([, args]) => String(args?.payload?.statusMessage ?? ''));
    expect(statuses.some((status) => /could not open|blocked url/i.test(status))).toBe(true);
  });
  expect(calls(tauri, 'hide_search_panel')).toHaveLength(hidesBefore);
});

it('activating a quick command runs it by command ID only through the search-owned command', async () => {
  const { tauri, activate } = await openSearchWith([quickRow], 'build');
  activate({ id: 'quick:build' });
  await waitFor(() => expect(calls(tauri, 'run_search_quick_command')).toHaveLength(1));
  expect(calls(tauri, 'run_search_quick_command')[0][1]).toEqual({ request: { id: 'build' } });
  expect(calls(tauri, 'run_quick_command')).toHaveLength(0);
  expect(calls(tauri, 'show_command_panel')).toHaveLength(0);
  await waitFor(() => expect(calls(tauri, 'hide_search_panel').length).toBeGreaterThan(0));
});

it('double activation while a launch is in flight issues a single launch request', async () => {
  let release!: () => void;
  quickRunImpl = () => new Promise((resolve) => { release = () => resolve({ outcome: 'started', runId: 'run-1', processId: 1 }); });
  const { tauri, activate } = await openSearchWith([quickRow], 'build');
  activate({ id: 'quick:build' });
  activate({ id: 'quick:build' });
  activate({ id: 'quick:build', rowKey: 'quick:build:0', recordKey: 'quick:build', resultIndex: 0 });
  await waitFor(() => expect(calls(tauri, 'run_search_quick_command')).toHaveLength(1));
  release();
  await waitFor(() => expect(calls(tauri, 'hide_search_panel').length).toBeGreaterThan(0));
  expect(calls(tauri, 'run_search_quick_command')).toHaveLength(1);
});

it('a stale or deleted quick command reports the backend error truthfully and keeps the panel open', async () => {
  quickRunImpl = async () => { throw new Error("quick command 'build' is not configured"); };
  const { tauri, activate } = await openSearchWith([quickRow], 'build');
  const hidesBefore = calls(tauri, 'hide_search_panel').length;
  activate({ id: 'quick:build' });
  await waitFor(() => expect(calls(tauri, 'run_search_quick_command')).toHaveLength(1));
  await waitFor(() => {
    const statuses = calls(tauri, 'publish_search_panel').map(([, args]) => String(args?.payload?.statusMessage ?? ''));
    expect(statuses.some((status) => status.includes('not configured'))).toBe(true);
  });
  expect(calls(tauri, 'hide_search_panel')).toHaveLength(hidesBefore);
  expect(calls(tauri, 'show_command_panel')).toHaveLength(0);
});

it('Shift+Enter reveal opens the Quick Commands panel at that command without running it', async () => {
  const { tauri, activate } = await openSearchWith([quickRow], 'build');
  activate({ id: 'quick:build', recordKey: 'quick:build', resultIndex: 0, intent: 'reveal' });
  await waitFor(() => expect(calls(tauri, 'show_command_panel')).toHaveLength(1));
  const request = (calls(tauri, 'show_command_panel')[0][1] as { request: Record<string, unknown> }).request;
  expect(request.commandId).toBe('build');
  expect(typeof request.anchorLeft).toBe('number');
  expect(typeof request.anchorWidth).toBe('number');
  expect(calls(tauri, 'run_search_quick_command')).toHaveLength(0);
  expect(calls(tauri, 'run_quick_command')).toHaveLength(0);
});

it('a reveal intent on a non-quick-command row never opens the Quick Commands panel', async () => {
  const { tauri, activate } = await openSearchWith([calculatorRow], '6*7');
  activate({ id: 'calculator:result', recordKey: 'calculator:result', resultIndex: 0, intent: 'reveal' });
  await waitFor(() => expect(calls(tauri, 'copy_search_text')).toHaveLength(1));
  expect(calls(tauri, 'show_command_panel')).toHaveLength(0);
});

it('built-in non-quick command rows keep their existing activation (no search-owned quick launch)', async () => {
  const control: Row = {
    id: 'command:open-control-plane', providerId: 'commands', kind: 'command', title: 'Open Control Plane',
    action: A.runCommand, terms: [], aliases: [], score: 700,
    matchReason: 'prefix', recordKey: 'command:open-control-plane'
  };
  const { tauri, activate } = await openSearchWith([control], 'control');
  activate({ id: 'command:open-control-plane' });
  await waitFor(() => expect(calls(tauri, 'hide_search_panel').length).toBeGreaterThan(0));
  expect(calls(tauri, 'run_search_quick_command')).toHaveLength(0);
  expect(calls(tauri, 'run_quick_command')).toHaveLength(0);
});

// Review L4: the backend returns a typed outcome; the renderer never parses error text.
it('an openPanel outcome opens the Quick Commands panel at that command and does not report a launch', async () => {
  quickRunImpl = async () => ({ outcome: 'openPanel', commandId: 'build' });
  const { tauri, activate } = await openSearchWith([quickRow], 'build');
  activate({ id: 'quick:build', recordKey: 'quick:build', resultIndex: 0 });
  await waitFor(() => expect(calls(tauri, 'show_command_panel')).toHaveLength(1));
  expect((calls(tauri, 'show_command_panel')[0][1] as { request: Record<string, unknown> }).request.commandId).toBe('build');
});

it('a started outcome closes the panel and never opens the Quick Commands panel', async () => {
  const { tauri, activate } = await openSearchWith([quickRow], 'build');
  activate({ id: 'quick:build', recordKey: 'quick:build', resultIndex: 0 });
  await waitFor(() => expect(calls(tauri, 'hide_search_panel').length).toBeGreaterThan(0));
  expect(calls(tauri, 'show_command_panel')).toHaveLength(0);
});

it('an error message that merely mentions the panel is an error, not an openPanel outcome', async () => {
  quickRunImpl = async () => { throw new Error('please open the Quick Commands panel'); };
  const { tauri, activate } = await openSearchWith([quickRow], 'build');
  activate({ id: 'quick:build', recordKey: 'quick:build', resultIndex: 0 });
  await waitFor(() => expect(calls(tauri, 'run_search_quick_command')).toHaveLength(1));
  await waitFor(() => {
    const statuses = calls(tauri, 'publish_search_panel').map(([, args]) => String(args?.payload?.statusMessage ?? ''));
    expect(statuses.some((status) => status.includes('Quick Commands panel'))).toBe(true);
  });
  expect(calls(tauri, 'show_command_panel')).toHaveLength(0);
});

// Review L1: identity is authoritative. A stale recordKey (the row was replaced while streaming)
// must not activate whatever now sits at that id or index.
it.each([
  ['web search fallback', { ...webRow, id: 'web:search', recordKey: 'web:search:new query' }, { id: 'web:search', recordKey: 'web:search:old query', resultIndex: 0 }, 'open_search_url'],
  ['calculator', calculatorRow, { id: 'calculator:result', recordKey: 'calculator:result:6*8', resultIndex: 0 }, 'copy_search_text'],
  ['bookmark', bookmarkRow, { id: 'bookmark:https://svelte.dev/', recordKey: 'bookmark:https://old.example/', resultIndex: 0 }, 'open_search_url'],
])('activating a %s with a stale recordKey does nothing', async (_name, row, identity, command) => {
  const { tauri, activate } = await openSearchWith([row], 'q');
  const hidesBefore = calls(tauri, 'hide_search_panel').length;
  activate(identity);
  await new Promise((resolve) => setTimeout(resolve, 150));
  expect(calls(tauri, command)).toHaveLength(0);
  expect(calls(tauri, 'hide_search_panel')).toHaveLength(hidesBefore);
});

// Review H2: Enter runs only the highlighted identity, even if streaming reordered the rows.
it('activation resolves by id and recordKey, not by a stale result index', async () => {
  const { tauri, activate } = await openSearchWith([quickRow, calculatorRow], 'q');
  activate({ id: 'calculator:result', recordKey: 'calculator:result', resultIndex: 0 });
  await waitFor(() => expect(calls(tauri, 'copy_search_text')).toHaveLength(1));
  expect(calls(tauri, 'run_search_quick_command')).toHaveLength(0);
});

it('an identity whose id no longer exists activates nothing, even when its index is valid', async () => {
  const { tauri, activate } = await openSearchWith([quickRow], 'q');
  activate({ id: 'quick:deleted', recordKey: 'quick:deleted', resultIndex: 0 });
  await new Promise((resolve) => setTimeout(resolve, 150));
  expect(calls(tauri, 'run_search_quick_command')).toHaveLength(0);
});
