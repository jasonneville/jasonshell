import { render, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, expect, it, vi } from 'vitest';
import { emit, ledger, overrides, resetBridge, root } from '../browser/stack-mouse/bridge';
vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-mouse/bridge'));
import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';

const MAX_BYTES = 2 * 1024 * 1024;
const MAX_ENTRIES = 256;
const icon = `data:image/png;base64,${'A'.repeat(8192)}`;
const probe = (view: ReturnType<typeof render>) => (view.component as any).__ramProbe();
let request = 0;
let negative = false;
const calls = () => ledger.filter(({ command }) => command === 'resolve_stack_item_icons');
beforeEach(() => {
  resetBridge(); request = 0; negative = false;
  overrides.set('get_stack_git_status', () => null);
  overrides.set('hide_stack_popup', () => undefined);
  overrides.set('read_stack_folder', ({ path }) => ({ path, offset: 0, limit: 48, total: 48, hasMore: false, warnings: [],
    items: Array.from({ length: 48 }, (_, i) => ({ path: `${path}\\item-${i}.exe`, name: `item-${i}.exe`, kind: 'file', typeLabel: 'Application', sizeBytes: 10, modifiedAt: null })) }));
  overrides.set('resolve_stack_item_icons', ({ paths }) => ({ items: (paths as string[]).map((path) => ({ path, iconDataUrl: negative ? null : icon })), cacheHits: 0, cacheMisses: (paths as string[]).length }));
});
async function open(view: ReturnType<typeof render>, path: string) {
  await emit('stack-popup:open', { path, requestId: `ram-${++request}` });
  for (let i = 0; i < 30; i++) { await Promise.resolve(); await tick(); }
  await waitFor(() => expect(probe(view).currentPath).toBe(path));
  await waitFor(() => expect((view.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value).toBe(path));
  await waitFor(() => {
    const rows = [...view.container.querySelectorAll('[data-stack-entry-path]')];
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.every(row => row.getAttribute('data-stack-entry-path')?.startsWith(path + '\\'))).toBe(true);
  });
  for (let i = 0; i < 30; i++) { await Promise.resolve(); await tick(); }
}
async function mount() {
  const view = render(StackPopupSurface);
  await waitFor(() => expect(calls().length).toBeGreaterThan(0));
  for (let i = 0; i < 30; i++) { await Promise.resolve(); await tick(); }
  return view;
}

it.each([false, true])('folder churn bounds cache bytes and negative entries (negative=%s), including hide/reopen', async (isNegative) => {
  negative = isNegative;
  const view = await mount();
  const before = probe(view);
  console.info('ram-stack-before', JSON.stringify(before));
  for (let i = 0; i < 12; i++) await open(view, `${root}\\churn-${i}`);
  await emit('stack-popup:close-requested');
  await tick();
  const after = probe(view);
  console.info('ram-stack-icon-churn', JSON.stringify({ isNegative, folders: 13, before, after, budgetBytes: MAX_BYTES, maxEntries: MAX_ENTRIES }));
  expect.soft(after.entries).toBeLessThanOrEqual(MAX_ENTRIES);
  expect.soft(after.estimatedStringBytes).toBeLessThanOrEqual(MAX_BYTES);
  if (after.negativeEntries !== null) expect.soft(after.negativeEntries).toBeLessThanOrEqual(MAX_ENTRIES);
  await open(view, `${root}\\churn-11`);
  if (!isNegative) expect(probe(view).visibleIcons).toBe(48);
});

it('recent folder cache hits preserve all current row icons without duplicate native requests', async () => {
  const view = await mount();
  await open(view, `${root}\\recent`);
  await waitFor(() => expect(probe(view).visibleIcons).toBe(48));
  const count = calls().length;
  await open(view, root);
  await open(view, `${root}\\recent`);
  expect(calls()).toHaveLength(count);
  expect(probe(view).visibleIcons).toBe(48);
});

it('late icon result from retired folder never overwrites current row presentation', async () => {
  const view = await mount();
  let finish!: (value: unknown) => void;
  const original = overrides.get('resolve_stack_item_icons')!;
  overrides.set('resolve_stack_item_icons', ({ paths }) => (paths as string[]).some((path) => path.includes('delayed'))
    ? new Promise((resolve) => { finish = resolve; }) : original({ paths }));
  await open(view, `${root}\\delayed`);
  await waitFor(() => expect(finish).toBeTypeOf('function'));
  await open(view, `${root}\\current`);
  await waitFor(() => expect(probe(view).visibleIcons).toBe(48));
  // Let the resolver finish its cache write before capturing the stale-response oracle.
  for (let i = 0; i < 40; i++) { await Promise.resolve(); await tick(); }
  const current = probe(view);
  finish({ items: [{ path: `${root}\\delayed\\item-0.exe`, iconDataUrl: 'data:image/png;base64,STALE' }], cacheHits: 0, cacheMisses: 1 });
  await tick(); await tick();
  expect(probe(view).currentPath).toBe(current.currentPath);
  expect(probe(view).visibleIcons).toBe(48);
  expect(view.container.querySelector('img[src*="STALE"]')).toBeNull();
  expect(probe(view).entries).toBe(current.entries);
  expect(probe(view).estimatedStringBytes).toBe(current.estimatedStringBytes);
});

it('oversized icons remain visible without exceeding retained-cache budget', async () => {
  const hugeIcon = `data:image/png;base64,${'A'.repeat(MAX_BYTES)}`;
  overrides.set('resolve_stack_item_icons', ({ paths }) => ({ items: (paths as string[]).map((path) => ({ path, iconDataUrl: hugeIcon })), cacheHits: 0, cacheMisses: (paths as string[]).length }));
  const view = await mount();
  await waitFor(() => expect(probe(view).visibleIcons).toBe(48));
  expect(probe(view).estimatedStringBytes).toBeLessThanOrEqual(MAX_BYTES);
  expect(probe(view).entries).toBe(0);
});

it('a folder listing settled after unmount cannot spawn icon IPC or repopulate the disposed cache', async () => {
  const view = await mount();
  const delayedPath = `${root}\\listing-after-dispose`;
  const page = overrides.get('read_stack_folder')!({ path: delayedPath });
  let finishListing!: (value: unknown) => void;
  overrides.set('read_stack_folder', ({ path }) => path === delayedPath
    ? new Promise((resolve) => { finishListing = resolve; }) : page);
  await emit('stack-popup:open', { path: delayedPath, requestId: 'delayed-listing-disposal' });
  await waitFor(() => expect(finishListing).toBeTypeOf('function'));
  const requestsBeforeUnmount = calls().length;
  view.unmount();
  const afterUnmount = probe(view);
  expect(afterUnmount.entries).toBe(0);
  finishListing(page);
  for (let i = 0; i < 80; i++) { await Promise.resolve(); await tick(); }
  const afterLateListing = probe(view);
  console.info('ram-stack-disposed-listing', JSON.stringify({ requestsBeforeUnmount, requestsAfterLateListing: calls().length, afterUnmount, afterLateListing }));
  expect.soft(calls()).toHaveLength(requestsBeforeUnmount);
  expect.soft(afterLateListing.entries).toBe(0);
  expect.soft(afterLateListing.estimatedStringBytes).toBe(0);
});
