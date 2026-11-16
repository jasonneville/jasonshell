import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import { root, child, other, context, summary, mutate, setVisible, notify, deferred, ledger, overrides, emit, resetBridge } from '../browser/stack-external-refresh/bridge';
vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-external-refresh/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-external-refresh/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-external-refresh/bridge'));
vi.mock('../../src/features/stack-browser/stackTextEditorAdapter', () => ({ createStackTextEditorAdapter: ({ parent, content, onChange }: {parent:HTMLElement;content:string;onChange:(draft:string)=>void}) => {
  const input = document.createElement('textarea'); input.setAttribute('aria-label', 'External fixture draft'); input.value = content; input.oninput = () => onChange(input.value); parent.append(input);
  return { destroy: () => input.remove(), focusAtStart: () => input.focus(), setFont: () => {} };
} }));
import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';
const calls = (command: string) => ledger.filter((entry) => entry.command === command);
const scans = () => calls('get_stack_git_summary').length;
const settle = () => new Promise((resolve) => setTimeout(resolve, 330));
async function open() {
  const view = render(StackPopupSurface); await screen.findByText('draft.txt');
  await waitFor(() => expect(calls('start_stack_folder_watch')).toHaveLength(1));
  await settle(); return view;
}
async function navigate(path: string) {
  const input = screen.getByRole('textbox', { name: 'Current folder path' });
  await fireEvent.input(input, { target: { value: path } }); await fireEvent.submit(input.closest('form')!);
  await waitFor(() => expect(document.querySelector('.stack-path')?.getAttribute('title')).toBe(path)); await settle();
}
beforeEach(() => { resetBridge(); vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} }); });
afterEach(() => { vi.unstubAllGlobals(); vi.restoreAllMocks(); });

it('external change updates production-mounted rows, summary and badges without manual Refresh', async () => {
  await open(); const before = scans(); mutate(); await notify();
  await screen.findByText('external-1.txt'); await waitFor(() => expect(scans()).toBe(before + 1));
  expect(screen.getByText('draft.txt').closest('[data-stack-entry-path]')?.getAttribute('data-git-status')).toBe('modified');
  expect(document.querySelector('.stack-git-count.git-status-modified')?.textContent).toContain('1');
  expect(calls('get_stack_git_status')).toHaveLength(0);
});

it('idle and ordinary same-repo navigation never scan Git without a change event', async () => {
  await open(); const before = scans(); const metadata = calls('get_stack_git_metadata').length;
  await settle(); expect(scans()).toBe(before);
  await navigate(child); await fireEvent.click(screen.getByRole('button', { name: 'Back', exact: true })); await settle();
  expect(scans()).toBe(before); expect(calls('get_stack_git_metadata')).toHaveLength(metadata);
  expect(calls('start_stack_folder_watch')).toHaveLength(1); expect(calls('stop_stack_folder_watch')).toHaveLength(0);
});

it('a burst of external events coalesces into one bounded folder/summary refresh', async () => {
  await open(); const before = scans(); const reads = calls('read_stack_folder').length;
  mutate(3); await notify(); await notify(); await notify();
  await screen.findByText('external-3.txt'); await settle();
  expect(scans()).toBe(before + 1); expect(calls('read_stack_folder')).toHaveLength(reads + 1);
});

it('three events during held refresh queue exactly one followup and publish latest external mutation', async () => {
  await open(); const before = scans(); const held = deferred<ReturnType<typeof summary>>(); let requests = 0;
  overrides.set('get_stack_git_summary', ({ path }) => ++requests === 1 ? held.promise : summary(String(path)));
  mutate(1); await notify(); await waitFor(() => expect(scans()).toBe(before + 1));
  mutate(4); await notify(); await notify(); await notify(); await settle();
  expect(scans()).toBe(before + 1); held.resolve({ ...summary(root), modified: 1 });
  await waitFor(() => expect(scans()).toBe(before + 2)); await screen.findByText('external-4.txt'); await settle();
  expect(scans()).toBe(before + 2); expect(document.querySelector('.stack-git-count.git-status-modified')?.textContent).toContain('4');
});

it('old watch token after path change cannot refresh the new folder or repository', async () => {
  await open(); await navigate(other); const before = scans(); const reads = calls('read_stack_folder').length;
  await notify('watch-1'); await settle(); expect(scans()).toBe(before); expect(calls('read_stack_folder')).toHaveLength(reads);
  mutate(2); await notify(); await screen.findByText('external-2.txt');
});

it('start acknowledgement catches a mutation that happened before native registration completed', async () => {
  const ack = deferred<{watchId:string;canonicalPath:string}>(); overrides.set('start_stack_folder_watch', () => ack.promise);
  render(StackPopupSurface); await screen.findByText('draft.txt');
  await waitFor(() => expect(calls('start_stack_folder_watch')).toHaveLength(1)); mutate(2);
  ack.resolve({ watchId: 'watch-1', canonicalPath: root });
  await screen.findByText('external-2.txt'); expect(scans()).toBeGreaterThan(0);
});

it('late start acknowledgement after unmount is stopped and never refreshes disposed UI', async () => {
  const ack = deferred<{watchId:string;canonicalPath:string}>(); overrides.set('start_stack_folder_watch', () => ack.promise);
  const view = render(StackPopupSurface); await screen.findByText('draft.txt');
  await waitFor(() => expect(calls('start_stack_folder_watch')).toHaveLength(1)); view.unmount(); const before = scans();
  ack.resolve({ watchId: 'late-watch', canonicalPath: root });
  await waitFor(() => expect(calls('stop_stack_folder_watch').some((call) => call.args.watchId === 'late-watch')).toBe(true));
  await notify('late-watch'); await settle(); expect(scans()).toBe(before);
});

it('navigation serializes watch starts and retires a late old-path acknowledgement before replacement', async () => {
  const ack = deferred<{watchId:string;canonicalPath:string}>(); let attempts = 0;
  overrides.set('start_stack_folder_watch', ({ path }) => ++attempts === 1 ? ack.promise : { watchId: 'new-watch', canonicalPath: String(path) });
  render(StackPopupSurface); await screen.findByText('draft.txt'); await waitFor(() => expect(calls('start_stack_folder_watch')).toHaveLength(1));
  await navigate(other); expect(calls('start_stack_folder_watch')).toHaveLength(1);
  ack.resolve({ watchId: 'late-root', canonicalPath: root });
  await waitFor(() => expect(calls('stop_stack_folder_watch').some((call) => call.args.watchId === 'late-root')).toBe(true));
  await waitFor(() => expect(calls('start_stack_folder_watch')).toHaveLength(2)); await settle(); const before = scans();
  await notify('late-root'); await settle(); expect(scans()).toBe(before);
  mutate(2); await notify('new-watch'); await screen.findByText('external-2.txt');
});

it('visible but unfocused remains eligible for native external-change delivery', async () => {
  await open(); await fireEvent(window, new Event('blur')); await emit('tauri://focus-changed', false);
  mutate(1); await notify(); await screen.findByText('external-1.txt'); expect(calls('stop_stack_folder_watch')).toHaveLength(0);
});

it('hidden stops watching and reopening catches missed mutations without trusting an old event', async () => {
  await open(); setVisible(false); await emit('stack-popup:closed');
  await waitFor(() => expect(calls('stop_stack_folder_watch')).toHaveLength(1)); const before = scans();
  mutate(3); await notify('watch-1'); await settle(); expect(scans()).toBe(before);
  setVisible(true); await emit('stack-popup:open', { path: root, requestId: 'external-reopen' });
  await screen.findByText('external-3.txt'); await waitFor(() => expect(calls('start_stack_folder_watch')).toHaveLength(2));
});

it('native popup-targeted closed immediately followed by reopen cannot retain the retired watch token', async () => {
  await open(); setVisible(false); await emit('stack-popup:closed');
  mutate(2); setVisible(true); await emit('stack-popup:open', { path: root, requestId: 'rapid-native-reopen' });
  await waitFor(() => expect(calls('start_stack_folder_watch')).toHaveLength(2)); await screen.findByText('external-2.txt');
  await settle(); const before = scans(); await notify('watch-1'); await settle(); expect(scans()).toBe(before);
  mutate(3); await notify('watch-2'); await screen.findByText('external-3.txt');
});

it('watch failure shows manual Refresh fallback while preserving the last good summary', async () => {
  overrides.set('start_stack_folder_watch', () => Promise.reject(new Error('fixture watcher unavailable')));
  render(StackPopupSurface); await screen.findByText('draft.txt');
  await screen.findByText(/fixture watcher unavailable/);
  expect(document.querySelector('.stack-git-branch')?.textContent).toBe('main');
  mutate(2); await fireEvent.click(screen.getByRole('button', { name: 'Refresh', exact: true })); await screen.findByText('external-2.txt');
});

it('failed external summary retains known Git snapshot, reports failure and does not silently retry', async () => {
  await open(); const before = scans(); const branch = document.querySelector('.stack-git-branch');
  overrides.set('get_stack_git_summary', () => Promise.reject(new Error('external summary unavailable')));
  mutate(1); await notify(); await screen.findByText('external-1.txt'); await screen.findByText(/external summary unavailable/);
  expect(document.querySelector('.stack-git-branch')).toBe(branch); await settle(); expect(scans()).toBe(before + 1);
  overrides.set('get_stack_git_summary', ({ path }) => summary(String(path)));
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh', exact: true }));
  await waitFor(() => expect(document.querySelector('.stack-git-count.git-status-modified')?.textContent).toContain('1'));
});

it('current asynchronous native watch error preserves the snapshot and exposes manual fallback', async () => {
  await open(); const before = scans(); const branch = document.querySelector('.stack-git-branch');
  await emit('stack-popup:folder-watch-error', { watchId: 'watch-1', message: 'Live folder updates unavailable. Use Refresh.' });
  await screen.findByText(/Live folder updates unavailable/); expect(document.querySelector('.stack-git-branch')).toBe(branch);
  await notify('watch-1'); await settle(); expect(scans()).toBe(before);
  mutate(2); await fireEvent.click(screen.getByRole('button', { name: 'Refresh', exact: true })); await screen.findByText('external-2.txt');
});

it('stale asynchronous watch error cannot mark a newer repository subscription unavailable', async () => {
  await open(); await navigate(other);
  await emit('stack-popup:folder-watch-error', { watchId: 'watch-1', message: 'Stale watcher failed' }); await tick();
  expect(screen.queryByText(/Stale watcher failed/)).toBeNull(); mutate(1); await notify(); await screen.findByText('external-1.txt');
});

it('external refresh preserves selection, search focus and folder scroll instead of stealing focus', async () => {
  await open(); await fireEvent.click(screen.getByText('draft.txt'));
  const search = screen.getByRole('textbox', { name: 'Search current folder' }); await fireEvent.input(search, { target: { value: 'draft' } }); search.focus();
  const body = document.querySelector('.details-body') as HTMLElement; body.scrollTop = 80;
  const before = scans(); mutate(1); await notify(); await waitFor(() => expect(scans()).toBe(before + 1));
  expect(document.activeElement).toBe(search); expect((search as HTMLInputElement).value).toBe('draft');
  expect(document.querySelector('[data-stack-entry-path][aria-selected="true"]')?.getAttribute('data-stack-entry-path')).toBe(`${root}\\draft.txt`);
  expect(body.scrollTop).toBe(80);
});

it('external refresh never reloads or discards an unsaved editor draft and does not steal editor focus', async () => {
  await open(); await fireEvent.dblClick(screen.getByText('draft.txt'));
  const draft = await screen.findByRole('textbox', { name: 'External fixture draft' });
  await fireEvent.input(draft, { target: { value: 'unsaved external draft' } }); draft.focus();
  const reads = calls('read_stack_basic_text_file').length; const before = scans(); mutate(1); await notify();
  await waitFor(() => expect(scans()).toBe(before + 1));
  expect((draft as HTMLTextAreaElement).value).toBe('unsaved external draft'); expect(document.activeElement).toBe(draft);
  expect(calls('read_stack_basic_text_file')).toHaveLength(reads); expect(calls('save_stack_basic_text_file')).toHaveLength(0);
});

it('external invalidation survives same-repository navigation while its context revalidation is held', async () => {
  await open(); const before = scans(); const contexts = calls('get_stack_git_context').length;
  const held = deferred<ReturnType<typeof context>>(); let requests = 0;
  overrides.set('get_stack_git_context', ({ path }) => ++requests === 1 ? held.promise : context(String(path)));
  mutate(2); await notify();
  await waitFor(() => expect(calls('get_stack_git_context')).toHaveLength(contexts + 1));
  await navigate(child); expect(scans()).toBe(before);
  // Return a deliberately obsolete identity: it must not overwrite the newly
  // verified child context, but its consumed invalidation still needs a followup.
  held.resolve(context(other));
  await waitFor(() => expect(scans() - before, 'one dirty summary followup after stale context').toBe(1));
  await settle(); expect(scans()).toBe(before + 1);
  expect(calls('start_stack_folder_watch')).toHaveLength(1); expect(calls('stop_stack_folder_watch')).toHaveLength(0);
  expect(document.querySelector('.stack-path')?.getAttribute('title')).toBe(child);
  expect(document.querySelector('.stack-git-count.git-status-modified')?.textContent).toContain('2');
  expect(screen.getByText('external-2.txt').closest('[data-stack-entry-path]')?.getAttribute('data-stack-entry-path')).toBe(`${child}\\external-2.txt`);
  expect(calls('get_stack_git_summary').at(-1)?.args.path).not.toBe(other);
});

it('missing trusted Git still admits native folder watching and refreshes plain rows without Git badges', async () => {
  overrides.set('get_stack_git_context', () => Promise.reject(new Error('Trusted Git executable unavailable')));
  render(StackPopupSurface); await screen.findByText('draft.txt');
  await waitFor(() => expect(calls('start_stack_folder_watch'), 'plain-folder watch despite rejected Git context').toHaveLength(1));
  expect(calls('start_stack_folder_watch')[0].args.path).toBe(root); await settle();
  expect(document.querySelector('.stack-git-branch')).toBeNull(); expect(scans()).toBe(0);
  mutate(3); await notify(); await screen.findByText('external-3.txt');
  expect(document.querySelector('[data-git-status]')).toBeNull(); expect(scans()).toBe(0);
  expect(calls('start_stack_folder_watch')).toHaveLength(1);
});

it('Git context failure after repository navigation retires old identity while the new folder remains watched', async () => {
  await open(); mutate(1); await notify(); await screen.findByText('external-1.txt');
  await waitFor(() => expect(document.querySelector('[data-git-status="modified"]')).not.toBeNull());
  overrides.set('get_stack_git_context', () => Promise.reject(new Error('Trusted Git executable unavailable')));
  await navigate(other);
  await waitFor(() => expect(calls('start_stack_folder_watch'), 'replacement plain-folder watch after Git context failure').toHaveLength(2));
  expect(calls('start_stack_folder_watch')[1].args.path).toBe(other); expect(document.querySelector('.stack-git-branch')).toBeNull();
  expect(document.querySelector('[data-git-status]')).toBeNull(); const before = scans();
  mutate(4); await notify('watch-1'); await settle(); expect(scans()).toBe(before);
  await notify('watch-2'); await screen.findByText('external-4.txt'); expect(scans()).toBe(before);
  expect(screen.getByText('external-4.txt').closest('[data-stack-entry-path]')?.getAttribute('data-stack-entry-path')).toBe(`${other}\\external-4.txt`);
});
