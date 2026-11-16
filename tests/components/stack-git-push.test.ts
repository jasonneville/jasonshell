import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { child, installPushFixture, ledger, overrides, resetBridge, status } from '../browser/stack-mouse/bridge';

vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-mouse/bridge'));
import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';
import StackGitPanel from '../../src/components/StackGitPanel.svelte';

beforeEach(() => {
  resetBridge();
  vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} });
});
afterEach(() => vi.unstubAllGlobals());

describe.each([undefined, 'https://example.invalid/repo'])('push runtime (browser URL %s)', (browserUrl) => {
async function mount(options: Parameters<typeof installPushFixture>[0] = {}) {
  installPushFixture({ browserUrl, ...options });
  render(StackPopupSurface);
  await fireEvent.dblClick(await screen.findByText('repo'));
  await screen.findByText('draft.txt');
  await fireEvent.click(await screen.findByRole('button', { name: 'main', exact: true }));
  await screen.findByRole('button', { name: 'Branch main' });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Commit message' }), { target: { value: 'explicit origin fixture' } });
}
const mutations = () => ledger.filter(({ command }) => ['stack_git_commit', 'stack_git_push'].includes(command));
const button = (name: string) => screen.getByRole('button', { name, exact: true }) as HTMLButtonElement;
const deferred = () => {
  let resolve!: (value: unknown) => void;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
};

it.each(['Push', 'Commit & Push'])('%s works without browser URL, exactly once, refreshes after success', async (action) => {
  await mount();
  const statusReads = ledger.filter(({ command }) => command === 'get_stack_git_status').length;
  expect(button(action).disabled).toBe(false);
  await fireEvent.click(button(action));
  await screen.findByText(action === 'Push' ? 'Pushed' : 'Committed and pushed', { exact: true });
  await waitFor(() => expect(button('Push').disabled).toBe(false));
  expect(mutations().map(({ command }) => command)).toEqual(action === 'Push' ? ['stack_git_push'] : ['stack_git_commit', 'stack_git_push']);
  expect(mutations().at(-1)?.args).toEqual({ folderPath: child });
  if (action !== 'Push') expect(mutations()[0].args).toEqual({ request: { folderPath: child, message: 'explicit origin fixture', paths: [`${child}\\draft.txt`] } });
  expect(ledger.filter(({ command }) => command === 'get_stack_git_status').length).toBeGreaterThan(statusReads);
  const browserLink = screen.queryByRole('button', { name: 'Open remote repository in browser', exact: true });
  if (browserUrl) expect(browserLink).not.toBeNull();
  else expect(browserLink).toBeNull();
});

it.each(['Push', 'Commit & Push'])('%s suppresses repeated actions while commit/push is pending', async (action) => {
  await mount();
  const pendingCommit = deferred(); const pendingPush = deferred();
  overrides.set('stack_git_commit', () => pendingCommit.promise);
  overrides.set('stack_git_push', () => pendingPush.promise);
  expect(button(action).disabled).toBe(false);
  await fireEvent.click(button(action));
  await waitFor(() => expect(mutations()).toHaveLength(1));
  expect(button('Push').disabled).toBe(true);
  expect(button('Commit & Push').disabled).toBe(true);
  await fireEvent.click(button(action)); await fireEvent.click(button('Push'));
  expect(mutations()).toHaveLength(1);
  if (action !== 'Push') {
    expect(mutations()[0].command).toBe('stack_git_commit');
    pendingCommit.resolve({ summary: 'Commit created', output: '', repositoryRoot: child });
    await waitFor(() => expect(mutations()).toHaveLength(2));
    expect(button('Push').disabled).toBe(true);
    await fireEvent.click(button('Push'));
    expect(mutations()).toHaveLength(2);
  }
  pendingPush.resolve({ summary: 'Pushed', output: '', repositoryRoot: child });
  await waitFor(() => expect(button('Push').disabled).toBe(false));
  expect(mutations().map(({ command }) => command)).toEqual(action === 'Push' ? ['stack_git_push'] : ['stack_git_commit', 'stack_git_push']);
});

it('failed commit shows native detail, never pushes, and preserves message for correction', async () => {
  await mount({ commitError: 'fixture commit rejected' });
  expect(button('Commit & Push').disabled).toBe(false);
  await fireEvent.click(button('Commit & Push'));
  await screen.findByText(/Committing failed: fixture commit rejected/);
  expect(mutations().map(({ command }) => command)).toEqual(['stack_git_commit']);
  expect((screen.getByRole('textbox', { name: 'Commit message' }) as HTMLTextAreaElement).value).toBe('explicit origin fixture');
});

it('failed push after commit preserves committed result; retry Push never recommits', async () => {
  await mount({ pushError: 'fixture non-fast-forward' });
  expect(button('Commit & Push').disabled).toBe(false);
  await fireEvent.click(button('Commit & Push'));
  await screen.findByText(/Pushing failed: fixture non-fast-forward/);
  expect(mutations().map(({ command }) => command)).toEqual(['stack_git_commit', 'stack_git_push']);
  expect((screen.getByRole('textbox', { name: 'Commit message' }) as HTMLTextAreaElement).value).toBe('');
  expect(button('Commit & Push').disabled).toBe(true);
  expect(screen.getByText(/fixture commit SHA/)).toBeTruthy();
  overrides.set('stack_git_push', () => ({ summary: 'Pushed', output: 'retry succeeded', repositoryRoot: child }));
  await fireEvent.click(button('Push'));
  await screen.findByText('Pushed', { exact: true });
  expect(mutations().map(({ command }) => command)).toEqual(['stack_git_commit', 'stack_git_push', 'stack_git_push']);
});

it('standalone push surfaces backend failure without committing', async () => {
  await mount({ pushError: 'configured origin required' });
  expect(button('Push').disabled).toBe(false);
  await fireEvent.click(button('Push'));
  await screen.findByText(/Pushing failed: configured origin required/);
  expect(mutations().map(({ command }) => command)).toEqual(['stack_git_push']);
});

it.each(['Push', 'Commit & Push'])('%s remains successful when follow-up refresh warns', async (action) => {
  installPushFixture({ browserUrl });
  render(StackGitPanel, { folderPath: child, initialStatus: { ...status, remoteRepositoryUrl: browserUrl, entries: status.entries.map((entry) => ({ ...entry, staged: true, unstaged: false })) }, onRefresh: () => { throw new Error('fixture status unavailable'); } });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Commit message' }), { target: { value: 'explicit origin fixture' } });
  expect(button(action).disabled).toBe(false);
  await fireEvent.click(button(action));
  await screen.findByText(action === 'Push' ? 'Pushed' : 'Committed and pushed', { exact: true });
  await screen.findByText(/fixture status unavailable/);
  expect(screen.queryByText(/Pushing failed:/)).toBeNull();
  expect(mutations().filter(({ command }) => command === 'stack_git_push')).toHaveLength(1);
});
});
