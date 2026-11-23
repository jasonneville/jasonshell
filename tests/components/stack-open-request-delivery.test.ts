import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import { root, other, resetBridge, deferred, ledger, handlers, overrides, emit, setVisible } from '../browser/stack-external-refresh/bridge';
vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-external-refresh/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-external-refresh/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-external-refresh/bridge'));
import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';

// FE-18 acceptance: Stack open requests are event-driven. No interval polling of
// get_stack_popup_request; exactly one pull after the open listener registers
// (event-before-listener race), plus recovery pulls on focus gain and on the
// document becoming visible. Mocked IPC only; live native delivery is not covered.
const PULL = 'get_stack_popup_request';
const OPEN = 'stack-popup:open';
const FOCUS = 'tauri://focus-changed';
const pulls = () => ledger.filter((entry) => entry.command === PULL).length;
const folderReads = (path?: string) => ledger.filter((entry) => entry.command === 'read_stack_folder' && (!path || entry.args.path === path)).length;
const shownPath = () => document.querySelector('.stack-path')?.getAttribute('title');
const quiet = () => new Promise((resolve) => setTimeout(resolve, 40));
function setDocumentVisibility(hidden: boolean) {
  Object.defineProperty(document, 'hidden', { configurable: true, get: () => hidden });
  Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => (hidden ? 'hidden' : 'visible') });
  document.dispatchEvent(new Event('visibilitychange'));
}
async function open() {
  const view = render(StackPopupSurface);
  await screen.findByText('draft.txt');
  await waitFor(() => expect(shownPath()).toBe(root));
  return view;
}

beforeEach(() => {
  resetBridge();
  vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} });
});
afterEach(() => {
  setDocumentVisibility(false);
  delete (document as unknown as Record<string, unknown>).hidden;
  delete (document as unknown as Record<string, unknown>).visibilityState;
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

it('registers the open listener before its single initial pull and never polls while idle', async () => {
  // Only interval timers are faked so testing-library waits keep working.
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
  let listenerReadyAtPull: boolean | null = null;
  overrides.set(PULL, () => { listenerReadyAtPull ??= (handlers.get(OPEN)?.size ?? 0) > 0; return { path: root, requestId: 'external-initial' }; });
  await open();
  expect(pulls()).toBe(1);
  expect(listenerReadyAtPull).toBe(true);
  vi.advanceTimersByTime(5000); await tick(); await quiet();
  expect(pulls(), 'hidden/idle popup issues zero get_stack_popup_request IPC over 5 s').toBe(1);
});

it('an open event navigates without any extra pull and applies one request key once', async () => {
  await open(); const before = folderReads(other);
  await emit(OPEN, { path: other, requestId: 'evt-1' });
  await waitFor(() => expect(shownPath()).toBe(other));
  await waitFor(() => expect(folderReads(other)).toBe(before + 1));
  await emit(OPEN, { path: other, requestId: 'evt-1' });
  await quiet();
  expect(folderReads(other)).toBe(before + 1);
  expect(pulls()).toBe(1);
});

it('window focus gain recovers a missed open event; focus loss does nothing', async () => {
  await open();
  overrides.set(PULL, () => ({ path: other, requestId: 'missed-1' }));
  await emit(FOCUS, false); await quiet();
  expect(pulls()).toBe(1);
  expect(shownPath()).toBe(root);
  await emit(FOCUS, true);
  await waitFor(() => expect(shownPath()).toBe(other));
  expect(pulls()).toBe(2);
});

it('document becoming visible recovers a missed open event; becoming hidden does not pull', async () => {
  await open();
  overrides.set(PULL, () => ({ path: other, requestId: 'missed-2' }));
  setDocumentVisibility(true); await quiet();
  expect(pulls()).toBe(1);
  setDocumentVisibility(false);
  await waitFor(() => expect(shownPath()).toBe(other));
  expect(pulls()).toBe(2);
});

it('a pull returning the already-applied key is a no-op (no folder reload)', async () => {
  await open(); const reads = folderReads();
  await emit(FOCUS, true); await quiet();
  expect(pulls()).toBe(2);
  expect(folderReads()).toBe(reads);
  expect(shownPath()).toBe(root);
});

it('a stale pull that resolves after a newer open event cannot override it', async () => {
  await open();
  const held = deferred<{ path: string; requestId: string }>();
  overrides.set(PULL, () => held.promise);
  await emit(FOCUS, true);
  await waitFor(() => expect(pulls()).toBe(2));
  await emit(OPEN, { path: other, requestId: 'newer-event' });
  await waitFor(() => expect(shownPath()).toBe(other));
  const otherReads = folderReads(other);
  held.resolve({ path: `${root}\\src`, requestId: 'stale-pull' });
  await quiet();
  expect(shownPath()).toBe(other);
  expect(folderReads(`${root}\\src`)).toBe(0);
  expect(folderReads(other)).toBe(otherReads);
});

it('a hidden popup ignores recovery pulls until shown, and unmount removes every listener and timer', async () => {
  const view = await open();
  setVisible(false);
  overrides.set(PULL, () => ({ path: other, requestId: 'while-hidden' }));
  await emit(FOCUS, true); await quiet();
  expect(shownPath()).toBe(root);
  view.unmount();
  expect(handlers.get(OPEN)?.size ?? 0).toBe(0);
  expect(handlers.get(FOCUS)?.size ?? 0).toBe(0);
  const settled = pulls();
  await emit(FOCUS, true); setDocumentVisibility(true); setDocumentVisibility(false); await quiet();
  expect(pulls(), 'no pull after disposal').toBe(settled);
});
