import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import { child, deep, nested, submodule, linked, outside, sibling, junction, context, summary, deferred, ledger, overrides, resetBridge } from '../browser/stack-git-refresh/bridge';
vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-git-refresh/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-git-refresh/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-git-refresh/bridge'));
vi.mock('../../src/features/stack-browser/stackTextEditorAdapter', () => ({
  createStackTextEditorAdapter: ({ parent, content, onChange }: {parent:HTMLElement;content:string;onChange:(draft:string)=>void}) => {
    const input = document.createElement('textarea'); input.setAttribute('aria-label', 'Fixture draft'); input.value = content;
    input.oninput = () => onChange(input.value); parent.append(input);
    return { destroy: () => input.remove(), focusAtStart: () => input.focus(), setFont: () => {} };
  }
}));
import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';
const calls = (command: string) => ledger.filter((call) => call.command === command);
const branch = () => document.querySelector('.stack-git-branch');
async function open() { render(StackPopupSurface); await screen.findByText('src'); await waitFor(() => expect(branch()?.textContent).toBe('main')); }
async function navigate(path: string) {
  const input = screen.getByRole('textbox', { name: 'Current folder path' });
  await fireEvent.input(input, { target: { value: path } }); await fireEvent.submit(input.closest('form')!);
  await waitFor(() => expect(calls('read_stack_folder').some((call) => call.args.path === path)).toBe(true)); await tick();
}
beforeEach(() => { resetBridge(); vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} }); });
afterEach(() => vi.unstubAllGlobals());

it('initial folder uses lightweight summary, never workbench status; metadata cannot delay branch paint', async () => {
  const metadata = deferred<unknown>(); overrides.set('get_stack_git_metadata', () => metadata.promise);
  await open(); expect(calls('get_stack_git_summary')).toHaveLength(1); expect(calls('get_stack_git_status')).toHaveLength(0);
  metadata.resolve({ ...context(child), remoteRepositoryUrl: null });
});
it('same-repo child, Back, Forward retain branch DOM and badges without another summary or enrichment', async () => {
  await open(); await waitFor(() => expect(calls('get_stack_git_metadata')).toHaveLength(1));
  const original = branch(); const counts = ['get_stack_git_summary', 'get_stack_git_status', 'get_stack_git_metadata'].map((name) => calls(name).length);
  await fireEvent.dblClick(screen.getByText('src')); await screen.findByText('draft.txt');
  expect(branch()).toBe(original); await waitFor(() => expect(screen.getByText('draft.txt').closest('[data-stack-entry-path]')?.getAttribute('data-git-status')).toBe('modified'));
  await fireEvent.click(screen.getByRole('button', { name: 'Back', exact: true })); await screen.findByText('src');
  await fireEvent.click(screen.getByRole('button', { name: 'Forward', exact: true })); await screen.findByText('draft.txt');
  expect(['get_stack_git_summary', 'get_stack_git_status', 'get_stack_git_metadata'].map((name) => calls(name).length)).toEqual(counts);
});
it.each([nested, submodule, linked, sibling, junction])('verified boundary %s never reuses enclosing repository status', async (path) => {
  await open(); const next = deferred<ReturnType<typeof summary>>(); overrides.set('get_stack_git_summary', () => next.promise);
  await navigate(path); await waitFor(() => expect(calls('get_stack_git_context').some((call) => call.args.path === path)).toBe(true));
  await waitFor(() => expect(branch()?.textContent).not.toBe('main')); next.resolve(summary(path));
  await waitFor(() => expect(branch()?.textContent).toBe('other'));
});
it('leaving repository clears snapshot; returning fetches fresh rather than using multi-repo cache', async () => {
  await open(); await navigate(outside); await waitFor(() => expect(branch()).toBeNull());
  await navigate(child); await waitFor(() => expect(calls('get_stack_git_summary')).toHaveLength(2));
});
it('slow same-repo context preserves snapshot across committed folder listing without status work', async () => {
  await open(); const original = branch(); const next = deferred<unknown>(); overrides.set('get_stack_git_context', () => next.promise);
  await fireEvent.dblClick(screen.getByText('src')); await screen.findByText('draft.txt');
  expect(branch()).toBe(original); next.resolve(context(deep)); await tick();
  expect(calls('get_stack_git_summary')).toHaveLength(1);
});
it('explicit Refresh rereads summary, retains successful snapshot on failure, then can retry', async () => {
  await open(); overrides.set('get_stack_git_summary', () => Promise.reject(new Error('summary refresh failed')));
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh', exact: true }));
  await waitFor(() => expect(calls('get_stack_git_summary')).toHaveLength(2)); expect(branch()?.textContent).toBe('main');
  overrides.set('get_stack_git_summary', () => summary(child, 'refreshed'));
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh', exact: true }));
  await waitFor(() => expect(branch()?.textContent).toBe('refreshed'));
});
it('late old summary cannot overwrite newly entered nested repository', async () => {
  const old = deferred<ReturnType<typeof summary>>(); overrides.set('get_stack_git_summary', ({ path }) => path === child ? old.promise : summary(String(path)));
  render(StackPopupSurface); await screen.findByText('nested'); await navigate(nested);
  await waitFor(() => expect(branch()?.textContent).toBe('other')); old.resolve(summary(child)); await tick();
  expect(branch()?.textContent).toBe('other');
});
it('late old metadata cannot decorate a new repository', async () => {
  const old = deferred<unknown>(); overrides.set('get_stack_git_metadata', ({ path }) => path === child ? old.promise : { ...context(String(path)), remoteRepositoryUrl: null });
  await open(); await waitFor(() => expect(calls('get_stack_git_metadata')).toHaveLength(1));
  await navigate(nested); await waitFor(() => expect(branch()?.textContent).toBe('other'));
  old.resolve({ ...context(child), remoteRepositoryUrl: 'https://wrong.invalid', ahead: 99, behind: 99 }); await tick();
  expect(document.querySelector('.stack-git-remote-link')).toBeNull();
});
it('opening workbench still requests full enriched status', async () => {
  await open(); await fireEvent.click(branch()!); await screen.findByRole('region', { name: 'Git panel' });
  await waitFor(() => expect(calls('get_stack_git_status').length).toBeGreaterThan(0));
});
it('successful editor save invalidates summary without needing folder navigation', async () => {
  await open(); await fireEvent.dblClick(screen.getByText('draft.txt'));
  await fireEvent.input(await screen.findByRole('textbox', { name: 'Fixture draft' }), { target: { value: 'saved' } });
  const save = screen.getByRole('button', { name: /^Save/ }); await fireEvent.click(save);
  await waitFor(() => expect(calls('save_stack_basic_text_file')).toHaveLength(1));
  await waitFor(() => expect(calls('get_stack_git_summary')).toHaveLength(2));
});

it.each(['folder', 'text', 'rename', 'paste', 'delete', 'extract'])('successful local %s mutation refreshes current snapshot once', async (operation) => {
  await open();
  if (operation === 'folder') {
    await fireEvent.click(screen.getByRole('button', { name: 'New folder', exact: true }));
    const input = screen.getByRole('textbox', { name: 'New folder name' });
    await fireEvent.input(input, { target: { value: 'created' } }); await fireEvent.submit(input.closest('form')!);
  } else if (operation === 'text') {
    await fireEvent.contextMenu(screen.getByRole('region', { name: 'Stack browser' }));
    await fireEvent.click(await screen.findByRole('menuitem', { name: 'New Text File', exact: true }));
  } else if (operation === 'rename') {
    await fireEvent.click(screen.getByText('draft.txt')); await fireEvent.click(screen.getByRole('button', { name: 'Rename selected item' }));
    const input = screen.getByRole('textbox', { name: 'Rename item' });
    await fireEvent.input(input, { target: { value: 'renamed.txt' } }); await fireEvent.submit(input.closest('form')!);
  } else if (operation === 'paste') {
    await fireEvent.click(screen.getByRole('button', { name: 'Paste into current folder' }));
  } else if (operation === 'delete') {
    const visibility = vi.spyOn(document, 'hidden', 'get').mockReturnValue(false);
    overrides.set('delete_stack_items', () => ({ deleted: 1, failures: [] }));
    try {
      await fireEvent.click(screen.getByText('draft.txt')); await fireEvent.click(screen.getByRole('button', { name: 'Delete selected item' }));
      await fireEvent.click(await screen.findByRole('button', { name: 'Delete', exact: true }));
      await waitFor(() => expect(calls('delete_stack_items')).toHaveLength(1));
    } finally { visibility.mockRestore(); }
  } else {
    const archive = screen.getByText('archive.zip'); await fireEvent.click(archive); await fireEvent.contextMenu(archive);
    await fireEvent.click(await screen.findByRole('menuitem', { name: 'Extract here', exact: true }));
  }
  const command = { folder: 'new_stack_folder', text: 'new_stack_text_file', rename: 'rename_stack_item', paste: 'paste_stack_items', delete: 'delete_stack_items', extract: 'extract_stack_archive' }[operation]!;
  await waitFor(() => expect(calls(command)).toHaveLength(1));
  await waitFor(() => expect(calls('get_stack_git_summary')).toHaveLength(2));
});

it('failed local mutation and failed editor save do not invalidate a valid summary', async () => {
  await open(); overrides.set('new_stack_folder', () => Promise.reject(new Error('mutation failed')));
  await fireEvent.click(screen.getByRole('button', { name: 'New folder', exact: true }));
  const input = screen.getByRole('textbox', { name: 'New folder name' }); await fireEvent.submit(input.closest('form')!);
  await screen.findByText('mutation failed'); expect(calls('get_stack_git_summary')).toHaveLength(1);
  await fireEvent.keyDown(input, { key: 'Escape' });
  overrides.set('save_stack_basic_text_file', () => Promise.reject(new Error('save failed')));
  await fireEvent.dblClick(screen.getByText('draft.txt')); await fireEvent.input(await screen.findByRole('textbox', { name: 'Fixture draft' }), { target: { value: 'failed draft' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save file' })); await screen.findByText('save failed');
  expect(calls('get_stack_git_summary')).toHaveLength(1);
});

it('stale context from abandoned navigation cannot clear or replace the current repository', async () => {
  await open(); const old = deferred<unknown>(); overrides.set('get_stack_git_context', ({ path }) => path === nested ? old.promise : context(String(path)));
  await navigate(nested); await navigate(deep); await waitFor(() => expect(branch()?.textContent).toBe('main'));
  old.resolve(context(nested)); await tick(); expect(branch()?.textContent).toBe('main'); expect(calls('get_stack_git_summary')).toHaveLength(1);
});

it('same-repo navigation joins pending initial summary instead of discarding or duplicating it', async () => {
  const pending = deferred<ReturnType<typeof summary>>(); overrides.set('get_stack_git_summary', () => pending.promise);
  render(StackPopupSurface); await screen.findByText('src');
  await fireEvent.dblClick(screen.getByText('src')); await screen.findByText('draft.txt');
  pending.resolve(summary(child)); await waitFor(() => expect(branch()?.textContent).toBe('main'));
  expect(calls('get_stack_git_summary')).toHaveLength(1);
});

it('two explicit refreshes reject older same-repo summary completion', async () => {
  await open(); const old = deferred<ReturnType<typeof summary>>(); let refreshes = 0;
  overrides.set('get_stack_git_summary', () => ++refreshes === 1 ? old.promise : summary(child, 'newest'));
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh', exact: true }));
  await waitFor(() => expect(calls('get_stack_git_summary')).toHaveLength(2));
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh', exact: true }));
  await waitFor(() => expect(branch()?.textContent).toBe('newest'));
  old.resolve(summary(child, 'stale')); await tick(); expect(branch()?.textContent).toBe('newest');
});

it.each([true, false])('Git stage action success=%s invalidates summary only after success', async (success) => {
  await open(); overrides.set('stack_git_add_paths', () => success ? { repositoryRoot: child, summary: 'Added', output: '' } : Promise.reject(new Error('stage failed')));
  await fireEvent.click(branch()!); await screen.findByRole('region', { name: 'Git panel' });
  await fireEvent.click(await screen.findByRole('button', { name: 'Stage all unstaged files' }));
  await waitFor(() => expect(calls('stack_git_add_paths')).toHaveLength(1));
  if (success) await waitFor(() => expect(calls('get_stack_git_summary')).toHaveLength(2));
  else { await screen.findAllByText(/stage failed/i); expect(calls('get_stack_git_summary')).toHaveLength(1); }
});

it.each([false, true])('successful incoming drop move=%s invalidates current snapshot', async (move) => {
  await open(); const grid = screen.getByRole('grid', { name: 'Folder details' });
  await fireEvent.drop(grid, { shiftKey: move, dataTransfer: { files: [], types: ['application/x-jasonshell-stack-paths'], getData: (type: string) => type === 'application/x-jasonshell-stack-paths' ? JSON.stringify([`${outside}\\incoming.txt`]) : '' } });
  await waitFor(() => expect(calls('paste_stack_items')).toHaveLength(1));
  await waitFor(() => expect(calls('get_stack_git_summary')).toHaveLength(2));
});

it('failed typed path does not retire the still-current initial pending Git context', async () => {
  const pending = deferred<unknown>(); overrides.set('get_stack_git_context', () => pending.promise);
  const readFolder = overrides.get('read_stack_folder')!;
  overrides.set('read_stack_folder', (args) => args.path === outside ? Promise.reject(new Error('typed destination unavailable')) : readFolder(args));
  render(StackPopupSurface); await screen.findByText('src');
  await waitFor(() => expect(calls('get_stack_git_context')).toHaveLength(1));
  await navigate(outside); await screen.findByText('typed destination unavailable');
  pending.resolve(context(child)); await waitFor(() => expect(branch()?.textContent).toBe('main'));
  expect(calls('get_stack_git_context')).toHaveLength(1);
  expect(calls('get_stack_git_summary')).toHaveLength(1);
  expect(screen.getByText('src')).toBeTruthy();
});

it('unverified destination rows never inherit old badges even when their lexical paths match the snapshot', async () => {
  await open(); const pending = deferred<unknown>(); overrides.set('get_stack_git_context', () => pending.promise);
  await fireEvent.dblClick(screen.getByText('src'));
  const destination = () => [...document.querySelectorAll('[data-stack-entry-path]')].find((row) => row.getAttribute('data-stack-entry-path') === `${deep}\\draft.txt`);
  await waitFor(() => expect(destination()).toBeTruthy());
  expect(branch()?.textContent).toBe('main');
  expect(destination()?.getAttribute('data-git-status')).toBeNull();
  expect(destination()?.querySelector('.git-status-badge')).toBeNull();
  pending.resolve(context(deep));
  await waitFor(() => expect(destination()?.getAttribute('data-git-status')).toBe('modified'));
  expect(calls('get_stack_git_summary')).toHaveLength(1);
});

it('failed same-folder Refresh summary does not auto-retry when its delayed context completes', async () => {
  await open(); const pending = deferred<unknown>(); overrides.set('get_stack_git_context', () => pending.promise);
  overrides.set('get_stack_git_summary', () => Promise.reject(new Error('single refresh failure')));
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh', exact: true }));
  await screen.findByText(/single refresh failure/);
  expect(calls('get_stack_git_summary')).toHaveLength(2);
  pending.resolve(context(child)); await tick(); await tick();
  expect(calls('get_stack_git_summary')).toHaveLength(2);
  expect(branch()?.textContent).toBe('main');
});

it('successful commit invalidates summary even when its follow-up push fails', async () => {
  await open();
  overrides.set('get_stack_git_status', () => ({ ...summary(child), remoteRepositoryUrl: 'https://example.invalid/repo', entries: summary(child).entries.map((entry) => ({ ...entry, staged: true, unstaged: false })) }));
  overrides.set('stack_git_commit', () => ({ repositoryRoot: child, summary: 'Commit created', output: 'fixture commit' }));
  overrides.set('stack_git_push', () => Promise.reject(new Error('follow-up push failed')));
  await fireEvent.click(branch()!); await screen.findByRole('region', { name: 'Git panel' });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Commit message' }), { target: { value: 'fixture commit' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Commit & Push', exact: true }));
  await waitFor(() => expect(calls('stack_git_commit')).toHaveLength(1));
  await waitFor(() => expect(calls('stack_git_push')).toHaveLength(1));
  await screen.findAllByText(/follow-up push failed/);
  await waitFor(() => expect(calls('get_stack_git_summary')).toHaveLength(2));
});

async function triggerPaste(route: string) {
  if (route === 'toolbar') await fireEvent.click(screen.getByRole('button', { name: 'Paste into current folder' }));
  else await fireEvent.drop(screen.getByRole('grid', { name: 'Folder details' }), { dataTransfer: { files: [], types: ['application/x-jasonshell-stack-paths'], getData: (type: string) => type === 'application/x-jasonshell-stack-paths' ? JSON.stringify([`${outside}\\incoming.txt`]) : '' } });
}

it.each(['toolbar', 'drop'])('successful %s paste invalidates summary even when its follow-up folder read fails', async (route) => {
  await open(); const readFolder = overrides.get('read_stack_folder')!;
  let rejectListing = true;
  overrides.set('read_stack_folder', (args) => rejectListing ? Promise.reject(new Error('post-paste listing failed')) : readFolder(args));
  await triggerPaste(route);
  await waitFor(() => expect(calls('paste_stack_items')).toHaveLength(1));
  await waitFor(() => expect(calls('get_stack_git_summary')).toHaveLength(2));
  console.info('paste-boundary-evidence', { route, outcome: 'success-then-listing-error', summaries: calls('get_stack_git_summary').length });
  await screen.findAllByText(/post-paste listing failed/);
  rejectListing = false;
  await navigate(deep);
  expect(calls('get_stack_git_summary')).toHaveLength(2);
});

it.each(['toolbar', 'drop'])('all-failed %s paste does not invalidate the current Git snapshot', async (route) => {
  await open();
  overrides.set('paste_stack_items', () => ({ pasted: [], failures: [{ path: `${outside}\\incoming.txt`, message: 'fixture paste denied' }] }));
  await triggerPaste(route);
  await waitFor(() => expect(calls('paste_stack_items')).toHaveLength(1));
  await waitFor(() => expect(calls('read_stack_folder').length).toBeGreaterThan(1));
  expect(calls('get_stack_git_summary')).toHaveLength(1);
  console.info('paste-boundary-evidence', { route, outcome: 'all-failed', summaries: calls('get_stack_git_summary').length });
  await screen.findAllByText(/fixture paste denied/);
});

it('failed metadata probe on Refresh preserves the last known remote link and exposes a retry diagnostic', async () => {
  await open(); await screen.findByRole('button', { name: 'Open remote repository in browser' });
  const knownTitle = screen.getByRole('button', { name: 'Open remote repository in browser' }).getAttribute('title');
  overrides.set('get_stack_git_metadata', () => Promise.reject(new Error('fixture metadata probe timeout')));
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh', exact: true }));
  await screen.findAllByText(/fixture metadata probe timeout/);
  expect(calls('get_stack_git_metadata')).toHaveLength(2);
  expect(screen.queryByRole('button', { name: 'Open remote repository in browser' })?.getAttribute('title')).toBe(knownTitle);
  expect(branch()?.textContent).toBe('main');
});
