import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import { child, ledger, overrides, resetBridge, status } from '../browser/stack-mouse/bridge';

vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-mouse/bridge'));
import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';

beforeEach(() => { resetBridge(); vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} }); });
afterEach(() => vi.unstubAllGlobals());

function press(target: Element, button = 3, type = 'pointerdown') {
  const event = new MouseEvent(type, { button, bubbles: true, cancelable: true });
  Object.defineProperty(event, 'pointerType', { value: 'mouse' });
  target.dispatchEvent(event);
  return event;
}
async function mountPanel() {
  const view = render(StackPopupSurface);
  await fireEvent.dblClick(await screen.findByText('repo'));
  await screen.findByText('draft.txt');
  const selectedPaths = [...document.querySelectorAll('[data-stack-entry-path][aria-selected="true"]')].map((row) => row.getAttribute('data-stack-entry-path'));
  await fireEvent.click(await screen.findByRole('button', { name: 'main', exact: true }));
  await waitFor(() => expect(screen.getByRole('button', { name: 'Branch main' })).toBeTruthy());
  return { view, selectedPaths };
}
function assertReadOnly() {
  expect(ledger.filter(({ command }) => /stack_git_(?:revert|add|unstage|ignore|delete|checkout|create|stash_(?:drop|pop|apply))/.test(command))).toEqual([]);
  expect(ledger.filter(({ command }) => command === 'read_stack_folder')).toHaveLength(2);
  expect(ledger.some(({ command }) => command === 'hide_stack_popup')).toBe(false);
  expect((screen.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value).toBe(child);
  expect((screen.getByRole('button', { name: 'Back', exact: true }) as HTMLButtonElement).disabled).toBe(false);
  expect((screen.getByRole('button', { name: 'Forward', exact: true }) as HTMLButtonElement).disabled).toBe(true);
}

it.each(['staged', 'unstaged'] as const)('collapsed %s Changes group removes visible diff; Forward preserves panel and Back closes panel without folder navigation', async (group) => {
  const groupStatus = { ...status, entries: status.entries.map((entry) => ({ ...entry, staged: group === 'staged', unstaged: group === 'unstaged' })) };
  overrides.set('get_stack_git_status', ({ path }) => path === child ? groupStatus : null);
  const { selectedPaths } = await mountPanel();
  const groupLabel = group === 'staged' ? 'Staged' : 'Unstaged';
  const section = screen.getByRole('listitem', { name: `${groupLabel} changes` });
  await fireEvent.click(within(section).getByRole('button', { name: /draft.txt/ }));
  await screen.findByRole('region', { name: 'Diff for draft.txt' });
  const collapse = within(section).getByRole('button', { name: new RegExp(`^${groupLabel}\\b`) });
  expect(collapse.getAttribute('aria-expanded')).toBe('true');
  await fireEvent.click(collapse);
  expect(collapse.getAttribute('aria-expanded')).toBe('false');
  expect(screen.queryByRole('region', { name: 'Diff for draft.txt' })).toBeNull();
  expect(document.querySelector('.stack-git-change-diff-drawer')).toBeNull();
  const popup = screen.getByRole('region', { name: 'Stack browser' });
  expect(press(popup, 4).defaultPrevented).toBe(true);
  expect(press(popup, 4, 'mousedown').defaultPrevented).toBe(true);
  expect(press(popup, 4, 'auxclick').defaultPrevented).toBe(true);
  await tick();
  expect(screen.getByRole('region', { name: 'Git panel' })).toBeTruthy();
  expect(collapse.getAttribute('aria-expanded')).toBe('false');
  assertReadOnly();
  expect(press(popup).defaultPrevented).toBe(true);
  expect(press(popup, 3, 'mousedown').defaultPrevented).toBe(true);
  expect(press(popup, 3, 'auxclick').defaultPrevented).toBe(true);
  await tick();
  assertReadOnly();
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Git panel' })).toBeNull());
  expect(screen.getByRole('grid', { name: 'Folder details' })).toBeTruthy();
  expect([...document.querySelectorAll('[data-stack-entry-path][aria-selected="true"]')].map((row) => row.getAttribute('data-stack-entry-path'))).toEqual(selectedPaths);
});

it('branch menu atop Changes diff owns Back before outside-pointer dismissal, with one action per press', async () => {
  await mountPanel();
  await fireEvent.click(screen.getByRole('button', { name: /draft.txt/ }));
  await screen.findByRole('region', { name: 'Diff for draft.txt' });
  await fireEvent.click(screen.getByRole('button', { name: 'Branch main' }));
  await screen.findByRole('region', { name: 'Branch picker' });
  const panel = screen.getByRole('region', { name: 'Git panel' });
  // Target outside the branch picker: an existing bubbling pointer handler closes it first today.
  expect(press(panel).defaultPrevented).toBe(true);
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Branch picker' })).toBeNull());
  press(panel, 3, 'mousedown'); press(panel, 3, 'auxclick');
  expect(screen.getByRole('region', { name: 'Diff for draft.txt' })).toBeTruthy();
  expect(screen.getByRole('region', { name: 'Git panel' })).toBeTruthy();
  expect(press(panel, 4).defaultPrevented).toBe(true);
  expect(screen.getByRole('region', { name: 'Diff for draft.txt' })).toBeTruthy();
  press(panel);
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Diff for draft.txt' })).toBeNull());
  expect(screen.getByRole('region', { name: 'Git panel' })).toBeTruthy();
  press(panel);
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Git panel' })).toBeNull());
  assertReadOnly();
});

it('Git confirmation Back cancels only confirmation, preserves branch menu, never deletes branch', async () => {
  await mountPanel();
  await fireEvent.click(screen.getByRole('button', { name: 'Branch main' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Delete local branch topic' }));
  const dialog = screen.getByRole('alertdialog');
  expect(press(dialog, 4).defaultPrevented).toBe(true);
  expect(screen.getByRole('alertdialog')).toBe(dialog);
  expect(press(dialog).defaultPrevented).toBe(true);
  await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
  expect(screen.getByRole('region', { name: 'Branch picker' })).toBeTruthy();
  expect(screen.getByRole('region', { name: 'Git panel' })).toBeTruthy();
  assertReadOnly();
});

it('History Back unwinds file diff then commit detail then panel; stale file response cannot reopen', async () => {
  await mountPanel();
  let resolveDiff!: (value: unknown) => void;
  overrides.set('stack_git_commit_file_diff', () => new Promise((resolve) => { resolveDiff = resolve; }));
  await fireEvent.click(screen.getByRole('tab', { name: 'History' }));
  await fireEvent.click(await screen.findByRole('button', { name: /Fixture commit/ }));
  await fireEvent.click(await screen.findByRole('button', { name: /draft.txt/ }));
  await screen.findByRole('region', { name: 'Diff for draft.txt' });
  const panel = screen.getByRole('region', { name: 'Git panel' });
  press(panel);
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Diff for draft.txt' })).toBeNull());
  expect(screen.getByRole('list', { name: 'Files in abc123' })).toBeTruthy();
  resolveDiff({ content: '+stale' });
  await Promise.resolve();
  expect(screen.queryByRole('region', { name: 'Diff for draft.txt' })).toBeNull();
  press(panel);
  await waitFor(() => expect(screen.queryByRole('list', { name: 'Files in abc123' })).toBeNull());
  expect(screen.getByRole('region', { name: 'Git panel' })).toBeTruthy();
  press(panel);
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Git panel' })).toBeNull());
  assertReadOnly();
});

it('hidden History detail does not outrank visible Changes list', async () => {
  await mountPanel();
  await fireEvent.click(screen.getByRole('tab', { name: 'History' }));
  await fireEvent.click(await screen.findByRole('button', { name: /Fixture commit/ }));
  await screen.findByRole('list', { name: 'Files in abc123' });
  await fireEvent.click(screen.getByRole('tab', { name: 'Changes' }));
  expect(screen.queryByRole('list', { name: 'Files in abc123' })).toBeNull();
  expect(press(screen.getByRole('region', { name: 'Git panel' })).defaultPrevented).toBe(true);
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Git panel' })).toBeNull());
  assertReadOnly();
});

it('Stashes Back unwinds diff then stash detail then panel without apply/pop/drop', async () => {
  await mountPanel();
  await fireEvent.click(screen.getByRole('tab', { name: 'Stashes' }));
  await fireEvent.click(await screen.findByRole('button', { name: /draft.txt/ }));
  await screen.findByRole('region', { name: 'Diff for draft.txt' });
  const panel = screen.getByRole('region', { name: 'Git panel' });
  press(panel);
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Diff for draft.txt' })).toBeNull());
  expect(screen.getByRole('list', { name: /Files in.*Fixture stash/ })).toBeTruthy();
  press(panel);
  await waitFor(() => expect(screen.queryByRole('list', { name: /Files in.*Fixture stash/ })).toBeNull());
  expect(screen.getByRole('region', { name: 'Git panel' })).toBeTruthy();
  press(panel);
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Git panel' })).toBeNull());
  assertReadOnly();
});

it('ignore menu consumes Forward and Back closes only menu above open Changes diff', async () => {
  overrides.set('get_stack_git_status', () => ({ ...status, modified: 0, untracked: 1, entries: status.entries.map((entry) => ({ ...entry, status: 'untracked' })) }));
  await mountPanel();
  await screen.findByLabelText('Untracked');
  const row = screen.getByRole('button', { name: /draft.txt/ });
  await fireEvent.click(row);
  await fireEvent.contextMenu(row);
  const menu = await screen.findByRole('menu', { name: 'Git file actions' });
  expect(press(menu, 4).defaultPrevented).toBe(true);
  expect(screen.getByRole('menu', { name: 'Git file actions' })).toBeTruthy();
  press(menu);
  await waitFor(() => expect(screen.queryByRole('menu', { name: 'Git file actions' })).toBeNull());
  expect(screen.getByRole('region', { name: 'Diff for draft.txt' })).toBeTruthy();
  expect(screen.getByRole('region', { name: 'Git panel' })).toBeTruthy();
  assertReadOnly();
});

it('rapid physical Back presses unwind branch picker, diff, panel once each; compatibility cannot steal next layer', async () => {
  await mountPanel();
  await fireEvent.click(screen.getByRole('button', { name: /draft.txt/ }));
  await fireEvent.click(screen.getByRole('button', { name: 'Branch main' }));
  await screen.findByRole('region', { name: 'Branch picker' });
  const popup = screen.getByRole('region', { name: 'Stack browser' });
  for (const layer of ['Branch picker', 'Diff for draft.txt', 'Git panel']) {
    expect(press(popup).defaultPrevented).toBe(true);
    await tick();
    expect(press(popup, 3, 'mousedown').defaultPrevented).toBe(true);
    expect(press(popup, 3, 'auxclick').defaultPrevented).toBe(true);
    await tick();
    expect(screen.queryByRole('region', { name: layer })).toBeNull();
    if (layer === 'Branch picker') expect(screen.getByRole('region', { name: 'Diff for draft.txt' })).toBeTruthy();
    if (layer === 'Diff for draft.txt') expect(screen.getByRole('region', { name: 'Git panel' })).toBeTruthy();
    assertReadOnly();
  }
});

it.each(['files', 'diff'])('delayed Stash %s response cannot reopen dismissed visible layers', async (kind) => {
  await mountPanel();
  let resolveResponse!: (value: unknown) => void;
  overrides.set(kind === 'files' ? 'stack_git_stash_files' : 'stack_git_stash_file_diff', () => new Promise((resolve) => { resolveResponse = resolve; }));
  await fireEvent.click(screen.getByRole('tab', { name: 'Stashes' }));
  await screen.findByRole('list', { name: /Files in.*Fixture stash/ });
  const panel = screen.getByRole('region', { name: 'Git panel' });
  if (kind === 'diff') {
    await fireEvent.click(await screen.findByRole('button', { name: /draft.txt/ }));
    await screen.findByRole('region', { name: 'Diff for draft.txt' });
    press(panel);
    await tick();
    expect(screen.queryByRole('region', { name: 'Diff for draft.txt' })).toBeNull();
  }
  press(panel);
  await tick();
  expect(screen.queryByRole('list', { name: /Files in.*Fixture stash/ })).toBeNull();
  resolveResponse(kind === 'files'
    ? { files: [{ path: 'stale.txt', relativePath: 'stale.txt', status: 'M' }] }
    : { content: '+stale-stash-content' });
  await tick(); await tick();
  expect(screen.queryByText(/stale/)).toBeNull();
  expect(screen.queryByRole('region', { name: 'Diff for draft.txt' })).toBeNull();
  expect(screen.queryByRole('list', { name: /Files in.*Fixture stash/ })).toBeNull();
  expect(screen.getByRole('region', { name: 'Git panel' })).toBeTruthy();
  assertReadOnly();
});

it('busy Git ignore confirmation consumes rapid side presses without close, duplicate mutation, navigation or destructive calls', async () => {
  overrides.set('get_stack_git_status', () => ({ ...status, modified: 0, untracked: 1, entries: status.entries.map((entry) => ({ ...entry, status: 'untracked' })) }));
  let resolveIgnore!: (value: unknown) => void;
  overrides.set('stack_git_ignore_path', () => new Promise((resolve) => { resolveIgnore = resolve; }));
  await mountPanel();
  await screen.findByLabelText('Untracked');
  await fireEvent.contextMenu(screen.getByRole('button', { name: /draft.txt/ }));
  await fireEvent.click(await screen.findByRole('menuitem', { name: /gitignore/ }));
  const dialog = await screen.findByRole('alertdialog');
  await fireEvent.click(screen.getByRole('button', { name: 'Add rule' }));
  await waitFor(() => expect(dialog.getAttribute('aria-busy')).toBe('true'));
  const calls = ledger.filter(({ command }) => command !== 'get_stack_popup_request').slice();
  for (const button of [3, 4, 3, 4]) {
    expect(press(dialog, button).defaultPrevented).toBe(true);
    expect(press(dialog, button, 'mousedown').defaultPrevented).toBe(true);
    expect(press(dialog, button, 'auxclick').defaultPrevented).toBe(true);
    await tick();
    expect(screen.getByRole('alertdialog')).toBe(dialog);
  }
  expect(ledger.filter(({ command }) => command !== 'get_stack_popup_request')).toEqual(calls);
  expect(ledger.filter(({ command }) => command === 'stack_git_ignore_path')).toHaveLength(1);
  expect(ledger.filter(({ command }) => /hide|delete|revert|stash_(?:drop|pop|apply)/.test(command))).toEqual([]);
  expect((screen.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value).toBe(child);
  resolveIgnore({ summary: 'Fixture ignore complete', output: '', repositoryRoot: child });
  await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
});

it('Back cancellation restores Git confirmation focus via existing branch-picker helper and deletes nothing', async () => {
  await mountPanel();
  const picker = screen.getByRole('button', { name: 'Branch main' });
  await fireEvent.click(picker);
  const remove = await screen.findByRole('button', { name: 'Delete local branch topic' });
  remove.focus();
  await fireEvent.click(remove);
  await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Cancel' })));
  press(screen.getByRole('alertdialog'));
  await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
  await waitFor(() => expect(document.activeElement).toBe(picker));
  assertReadOnly();
});

it('ordinary outside pointer and keyboard Escape retain existing branch-menu dismissal without mouse history action', async () => {
  await mountPanel();
  await fireEvent.click(screen.getByRole('button', { name: /draft.txt/ }));
  await fireEvent.click(screen.getByRole('button', { name: 'Branch main' }));
  const panel = screen.getByRole('region', { name: 'Git panel' });
  expect(press(panel, 0).defaultPrevented).toBe(false);
  await tick();
  expect(screen.queryByRole('region', { name: 'Branch picker' })).toBeNull();
  expect(screen.getByRole('region', { name: 'Diff for draft.txt' })).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Branch main' }));
  await fireEvent.keyDown(panel, { key: 'Escape' });
  expect(screen.queryByRole('region', { name: 'Branch picker' })).toBeNull();
  expect(screen.getByRole('region', { name: 'Diff for draft.txt' })).toBeTruthy();
  assertReadOnly();
});
