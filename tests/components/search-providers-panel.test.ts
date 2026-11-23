// Tier 1 (PRD-01/02/03) component evidence for the centered search panel with mocked IPC:
// scope membership of the new provider rows and the Shift+Enter "reveal in Quick Commands" intent.
// Live native behaviour is out of scope (needs human consent).
import { fireEvent, render, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import SearchPanelSurface from '../../src/components/SearchPanelSurface.svelte';
import { tauriMocks } from './setup';
import type { SearchPanelPayload, SearchPanelResult } from '../../src/lib/searchPanel';

const popoverDescriptors = ['showPopover', 'hidePopover'].map(name => [name, Object.getOwnPropertyDescriptor(HTMLElement.prototype, name)] as const);
beforeEach(() => {
  if (typeof HTMLElement.prototype.showPopover === 'function') return;
  for (const [name, display, state] of [['showPopover', 'block', 'open'], ['hidePopover', 'none', 'closed']] as const) {
    Object.defineProperty(HTMLElement.prototype, name, { configurable: true, value(this: HTMLElement) {
      this.style.display = display;
      this.dispatchEvent(Object.assign(new Event('toggle'), { newState: state }));
    } });
  }
});
afterEach(() => {
  for (const [name, descriptor] of popoverDescriptors) {
    if (descriptor) Object.defineProperty(HTMLElement.prototype, name, descriptor);
    else Reflect.deleteProperty(HTMLElement.prototype, name);
  }
});

const providerResults: SearchPanelResult[] = [
  { id: 'calculator:result', kind: 'calculator', title: '42', providerId: 'calculator', actionId: 'copyCalculatorResult', copyText: '42', recordKey: 'calculator:result' },
  { id: 'quick:build', kind: 'command', title: 'Build project', providerId: 'commands', actionId: 'runCommand', recordKey: 'quick:build' },
  { id: 'command:open-control-plane', kind: 'command', title: 'Open Control Plane', providerId: 'commands', actionId: 'runCommand', recordKey: 'command:open-control-plane' },
  { id: 'bookmark:https://svelte.dev/', kind: 'bookmark', title: 'Svelte docs', providerId: 'bookmarks', actionId: 'openBookmark', url: 'https://svelte.dev/', recordKey: 'bookmark:https://svelte.dev/' },
  { id: 'web:open-url', kind: 'web', title: 'Open https://svelte.dev', providerId: 'web', actionId: 'openWebUrl', url: 'https://svelte.dev/', recordKey: 'web:open-url' },
  { id: 'web:search', kind: 'web', title: 'Search the web for svelte', providerId: 'web', actionId: 'openWebUrl', url: 'https://www.google.com/search?q=svelte', recordKey: 'web:search' }
].map((result) => ({ subtitle: '', terms: '', priority: 0, ...result })) as SearchPanelResult[];

async function mount(results: SearchPanelResult[], selectedIndex: number, presentation: 'centered' | 'anchored' = 'centered', phase?: SearchPanelPayload['phase']) {
  const tauri = tauriMocks();
  const payload: SearchPanelPayload = { query: 'svelte', results, selectedIndex, statusMessage: '', presentation, ...(phase ? { phase } : {}) };
  tauri.listen.mockImplementation(async () => vi.fn());
  tauri.invoke.mockImplementation(async (command) => command === 'get_search_panel_payload' ? payload : null);
  HTMLElement.prototype.scrollIntoView = vi.fn();
  const view = render(SearchPanelSurface);
  await waitFor(() => expect(within(view.getByRole('listbox', { name: 'Search results' })).getAllByRole('option').length).toBe(results.length));
  return { view, tauri };
}

async function chooseScope(view: ReturnType<typeof render>, label: string) {
  const trigger = view.getByRole('combobox', { name: 'Search scope' });
  await fireEvent.click(trigger);
  await waitFor(() => expect(trigger.getAttribute('aria-expanded')).toBe('true'));
  const popup = document.getElementById(trigger.getAttribute('aria-controls')!)!;
  await fireEvent.click(within(popup).getByRole('option', { name: label, exact: true }));
  await waitFor(() => expect(trigger.getAttribute('aria-expanded')).toBe('false'));
}

function optionTitles(view: ReturnType<typeof render>) {
  return within(view.getByRole('listbox', { name: 'Search results' })).getAllByRole('option').map((row) => row.textContent ?? '');
}

it('Commands scope lists only command rows: calculator, web and bookmark rows appear only under All', async () => {
  const { view } = await mount(providerResults, 0);
  await chooseScope(view, 'Commands');
  const commands = optionTitles(view);
  expect(commands).toHaveLength(2);
  expect(commands.some((text) => text.includes('Build project'))).toBe(true);
  expect(commands.some((text) => text.includes('Open Control Plane'))).toBe(true);
  for (const excluded of ['42', 'Svelte docs', 'Open https://svelte.dev', 'Search the web for svelte']) {
    expect(commands.some((text) => text.includes(excluded)), excluded).toBe(false);
  }
  await chooseScope(view, 'All');
  expect(optionTitles(view)).toHaveLength(providerResults.length);
});

it('every non-All scope hides the web fallback, calculator and bookmark rows', async () => {
  const { view } = await mount(providerResults, 0);
  for (const scope of ['Apps', 'Windows', 'Settings', 'Folders', 'Everything']) {
    await chooseScope(view, scope);
    const rows = within(view.getByRole('listbox', { name: 'Search results' })).queryAllByRole('option').map((row) => row.textContent ?? '');
    for (const excluded of ['Search the web for svelte', 'Svelte docs', 'Open https://svelte.dev']) {
      expect(rows.some((text) => text.includes(excluded)), `${scope}: ${excluded}`).toBe(false);
    }
  }
});

it('Enter on the preselected first row activates it, never the trailing web fallback, when other rows exist', async () => {
  const { view, tauri } = await mount(providerResults, 0);
  const input = view.getByRole('combobox', { name: 'Search Everything' });
  await fireEvent.keyDown(input, { key: 'Enter' });
  const activation = tauri.emitTo.mock.calls.find((args) => args[1] === 'search-panel:activate')?.[2];
  expect(activation).toMatchObject({ id: 'calculator:result', recordKey: 'calculator:result' });
});

it('plain Enter on a quick command emits a run activation with no reveal intent', async () => {
  const { view, tauri } = await mount(providerResults, 1);
  await fireEvent.keyDown(view.getByRole('combobox', { name: 'Search Everything' }), { key: 'Enter' });
  const activation = tauri.emitTo.mock.calls.find((args) => args[1] === 'search-panel:activate')?.[2];
  expect(activation).toMatchObject({ id: 'quick:build', resultIndex: 1 });
  expect((activation as { intent?: string }).intent).toBeUndefined();
});

it('Shift+Enter on a quick command emits a reveal activation (opens Quick Commands, never runs)', async () => {
  const { view, tauri } = await mount(providerResults, 1);
  await fireEvent.keyDown(view.getByRole('combobox', { name: 'Search Everything' }), { key: 'Enter', shiftKey: true });
  const activations = tauri.emitTo.mock.calls.filter((args) => args[1] === 'search-panel:activate');
  expect(activations).toHaveLength(1);
  expect(activations[0][2]).toMatchObject({ id: 'quick:build', resultIndex: 1, recordKey: 'quick:build', intent: 'reveal' });
  expect(tauri.emitTo.mock.calls.some((args) => args[1] === 'search-panel:key' && args[2] === 'Enter')).toBe(false);
});

it('Shift+Enter on a non-quick-command row does not emit a reveal intent', async () => {
  const { view, tauri } = await mount(providerResults, 0);
  await fireEvent.keyDown(view.getByRole('combobox', { name: 'Search Everything' }), { key: 'Enter', shiftKey: true });
  const activation = tauri.emitTo.mock.calls.find((args) => args[1] === 'search-panel:activate')?.[2] as { intent?: string } | undefined;
  expect(activation?.intent).not.toBe('reveal');
});

it('Shift+Enter on a quick command is inert when the scope hides that row', async () => {
  const { view, tauri } = await mount(providerResults, 1);
  await chooseScope(view, 'Apps');
  await fireEvent.keyDown(view.getByRole('combobox', { name: 'Search Everything' }), { key: 'Enter', shiftKey: true });
  expect(tauri.emitTo.mock.calls.some((args) => args[1] === 'search-panel:activate')).toBe(false);
});

it('anchored presentation keeps showing every provider row unfiltered with Enter activation intact', async () => {
  const { view, tauri } = await mount(providerResults, 4, 'anchored');
  expect(view.queryByRole('combobox', { name: 'Search scope' })).toBeNull();
  expect(optionTitles(view)).toHaveLength(providerResults.length);
  await fireEvent.click(within(view.getByRole('listbox', { name: 'Search results' })).getAllByRole('option')[3]);
  await waitFor(() => expect(tauri.emitTo.mock.calls.some((args) => args[1] === 'search-panel:select')).toBe(true));
});

it('selectedIndex -1 means nothing is selected: Enter activates nothing, even with strong rows present', async () => {
  const weak = { id: 'quick:w', kind: 'command', title: 'Weekly report', providerId: 'commands', actionId: 'runCommand', recordKey: 'quick:w', matchReason: 'wordPrefix', subtitle: '', terms: '', priority: 0 } as SearchPanelResult;
  const strong = { id: 'everything:a', kind: 'file', title: 'report.docx', providerId: 'everything', recordKey: 'everything:a', matchReason: 'prefix', subtitle: '', terms: '', priority: 0 } as SearchPanelResult;
  const mounted = await mount([weak, strong], -1, 'centered', 'local');
  await fireEvent.keyDown(mounted.view.getByRole('combobox', { name: 'Search Everything' }), { key: 'Enter' });
  expect(mounted.tauri.emitTo.mock.calls.some((args) => args[1] === 'search-panel:activate')).toBe(false);
});

it('a scope that hides the selected row falls back to the implicit choice, never to a weak row', async () => {
  const mk = (id: string, kind: string, providerId: string, matchReason: string) =>
    ({ id, kind, title: id, providerId, recordKey: id, matchReason, subtitle: '', terms: '', priority: 0 }) as SearchPanelResult;
  const rows = [mk('app:a', 'app', 'apps', 'exact'), mk('quick:w', 'command', 'commands', 'wordPrefix'), mk('quick:p', 'command', 'commands', 'prefix')];
  const { view, tauri } = await mount(rows, 0);
  await chooseScope(view, 'Commands');
  await fireEvent.keyDown(view.getByRole('combobox', { name: 'Search Everything' }), { key: 'Enter' });
  expect(tauri.emitTo.mock.calls.find((args) => args[1] === 'search-panel:activate')?.[2]).toMatchObject({ id: 'quick:p' });
});
