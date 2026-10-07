import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { emit, handlers, ledger, overrides, resetBridge, root } from '../browser/stack-mouse/bridge';

vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-mouse/bridge'));

import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';

beforeEach(() => {
  resetBridge();
  vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} });
});
afterEach(() => vi.unstubAllGlobals());

function status() { return document.querySelector<HTMLElement>('.stack-status')!; }
function expectInlineStatus() {
  const live = status();
  const pin = screen.getByRole('button', { name: 'Pin to quick bar' });
  const search = screen.getByRole('textbox', { name: 'Search current folder' });
  const toolbar = pin.closest('.stack-actions')!;
  expect(live).not.toBeNull();
  expect(document.querySelectorAll('.stack-status')).toHaveLength(1);
  expect(toolbar.contains(live), 'entire status belongs to toolbar, not footer').toBe(true);
  expect(pin.compareDocumentPosition(live) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(live.compareDocumentPosition(search) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(pin.parentElement, 'Pin/status share a wrapping unit').toBe(live.parentElement);
  expect(live.parentElement).not.toBe(toolbar);
  expect(live.getAttribute('role')).toBe('status');
  expect(live.getAttribute('aria-live')).toBe('polite');
  expect(live.classList.contains('surface-state')).toBe(false);
}
async function expectText(text: string) {
  await waitFor(() => expect(status().textContent).toContain(text));
  expect(within(status()).getByTitle(text)).toBeTruthy();
}
async function openFolder() {
  render(StackPopupSurface);
  await screen.findByText('repo');
  await expectText('1 of 1 items');
}
function page(warnings: string[] = [], empty = false) {
  return { path: root, offset: 0, limit: 100, total: empty ? 0 : 1, hasMore: false, warnings,
    items: empty ? [] : [{ path: `${root}\\repo`, name: 'repo', kind: 'folder', typeLabel: 'folder', sizeBytes: 8, modifiedAt: null }] };
}

it('ordinary status has full count/title; Pin and Search actions retain semantics', async () => {
  overrides.set('pin_stack_folder', () => undefined);
  await openFolder();
  await fireEvent.click(screen.getByRole('button', { name: 'Pin to quick bar' }));
  expect(ledger.find(({ command }) => command === 'pin_stack_folder')?.args).toEqual({ path: root });
  const search = screen.getByRole('textbox', { name: 'Search current folder' });
  await fireEvent.input(search, { target: { value: 'missing' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Clear search' }));
  expect((search as HTMLInputElement).value).toBe('');
  expect(document.activeElement).toBe(search);
  expectInlineStatus();
});

it.each([0, 1, 2])('empty/partial listing preserves existing wording (%s warnings)', async (count) => {
  overrides.set('read_stack_folder', () => page(Array.from({ length: count }, () => 'unreadable'), count === 0));
  render(StackPopupSurface);
  await expectText(count ? `1 of 1 items - partial listing: ${count} warning${count === 1 ? '' : 's'}` : 'This folder is empty');
  expectInlineStatus();
});

it('loading keeps both primary and secondary full text/title in live region', async () => {
  overrides.set('read_stack_folder', () => new Promise(() => {}));
  render(StackPopupSurface);
  await expectText('Loading folder...');
  await expectText('Loading...');
  expectInlineStatus();
});

it('icon hydration remains secondary to listing count', async () => {
  overrides.set('resolve_stack_item_icons', () => new Promise(() => {}));
  await openFolder();
  await expectText('Loading icons 0 of 1');
  expectInlineStatus();
});

it('long error remains complete in live text/title with error accent', async () => {
  const message = `Cannot enumerate folder: ${'very long diagnostic detail '.repeat(20)}end`;
  vi.spyOn(console, 'error').mockImplementation(() => {});
  overrides.set('read_stack_folder', () => Promise.reject(new Error(message)));
  render(StackPopupSurface);
  await expectText(message);
  expect(status().classList.contains('error')).toBe(true);
  expectInlineStatus();
});

it.each([false, true])('clipboard %s preserves immediate singular count feedback', async (cut) => {
  overrides.set(cut ? 'cut_stack_items' : 'copy_stack_items', () => undefined);
  await openFolder();
  await fireEvent.click(screen.getByText('repo'));
  await fireEvent.click(screen.getByRole('button', { name: cut ? 'Cut selected item' : 'Copy folder' }));
  await expectText(`${cut ? 'Cut' : 'Copied'} 1 item`);
  expectInlineStatus();
});

it('no folder retains initial guidance and disables Pin/Paste', async () => {
  overrides.set('get_stack_popup_request', () => null);
  render(StackPopupSurface);
  await expectText('Choose a pinned folder');
  expect((screen.getByRole('button', { name: 'Pin to quick bar' }) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByRole('button', { name: 'Paste into current folder' }) as HTMLButtonElement).disabled).toBe(true);
  expectInlineStatus();
});

it.each([false, true])('clipboard %s preserves plural multi-selection feedback', async (cut) => {
  overrides.set(cut ? 'cut_stack_items' : 'copy_stack_items', () => undefined);
  render(StackPopupSurface);
  await fireEvent.dblClick(await screen.findByText('repo'));
  await screen.findByText('draft.txt');
  await fireEvent.click(screen.getByText('draft.txt'));
  await fireEvent.click(screen.getByText('readme.md'), { ctrlKey: true });
  await fireEvent.click(screen.getByRole('button', { name: cut ? 'Cut selected item' : 'Copy selected item' }));
  await expectText(`${cut ? 'Cut' : 'Copied'} 2 items`);
  expectInlineStatus();
});

it.each(['determinate', 'indeterminate', 'completed', 'failed'])('operation %s takes priority over listing/icons and retains progress semantics', async (mode) => {
  let resolve!: (value: unknown) => void;
  let reject!: (error: Error) => void;
  overrides.set('resolve_stack_item_icons', () => new Promise(() => {}));
  overrides.set('paste_stack_items', () => new Promise((yes, no) => { resolve = yes; reject = no; }));
  vi.spyOn(console, 'error').mockImplementation(() => {});
  await openFolder();
  await waitFor(() => expect(handlers.get('stack-operation:progress')?.size).toBe(1));
  await fireEvent.click(screen.getByRole('button', { name: 'Paste into current folder' }));
  await waitFor(() => expect(ledger.some(({ command }) => command === 'paste_stack_items')).toBe(true));
  const operationId = ledger.find(({ command }) => command === 'paste_stack_items')!.args.operationId;
  if (mode === 'determinate') {
    await emit('stack-operation:progress', { operationId, operation: 'paste', phase: 'copying', determinate: true,
      completedFiles: 1, totalFiles: 4, completedBytes: 256, totalBytes: 1024, currentPath: `${root}\\long-file.txt`, elapsedMs: 2000 });
    await waitFor(() => expect(status().textContent).toContain('Copying · 1 / 4 files · 256 B / 1.0 KB · long-file.txt · 00:02'));
    const progress = within(status()).getByRole('progressbar') as HTMLProgressElement;
    expect(progress.max).toBe(1);
    expect(progress.value).toBe(0.25);
    expect(progress.getAttribute('aria-label')).toBe(status().textContent?.trim());
  } else if (mode === 'completed') {
    resolve({ failures: [] });
    await waitFor(() => expect(status().textContent).toContain('Paste complete ·'));
  } else if (mode === 'failed') {
    reject(new Error('Fixture paste denied'));
    await waitFor(() => expect(status().textContent).toContain('Paste failed ·'));
    expect(status().classList.contains('error')).toBe(true);
    expect(status().textContent).not.toContain('Fixture paste denied'); // Existing operation priority, not fallback error text.
  } else {
    await waitFor(() => expect(status().textContent).toContain('Preparing ·'));
    expect(status().querySelector('.stack-operation-spinner')?.getAttribute('aria-hidden')).toBe('true');
  }
  if (mode !== 'determinate') expect(within(status()).queryByRole('progressbar')).toBeNull();
  if (mode === 'completed' || mode === 'failed') expect(status().querySelector('.stack-operation-spinner')).toBeNull();
  const fullText = status().textContent!.trim();
  expect(within(status()).getByTitle(fullText)).toBeTruthy();
  expect(fullText).not.toContain('of 1 items');
  expect(fullText).not.toContain('Loading icons');
  expectInlineStatus();
});
