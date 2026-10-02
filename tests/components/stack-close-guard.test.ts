import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, expect, it, vi } from 'vitest';

const bridge = vi.hoisted(() => {
  const handlers = new Map<string, Set<() => void>>();
  return {
    handlers,
    hide: vi.fn(async () => { handlers.get('stack-popup:closed')?.forEach((handler) => handler()); }),
    request: vi.fn(async () => { handlers.get('stack-popup:close-requested')?.forEach((handler) => handler()); return true; }),
    on(event: string, handler: () => void) {
      const subscribers = handlers.get(event) ?? new Set();
      subscribers.add(handler);
      handlers.set(event, subscribers);
      return () => { subscribers.delete(handler); };
    }
  };
});

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    listen: async (event: string, handler: () => void) => bridge.on(event, handler),
    onDragDropEvent: async () => () => {},
    label: 'stack-popup'
  })
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (event: string, handler: () => void) => bridge.on(event, handler),
  emit: vi.fn(async () => {}), emitTo: vi.fn(async () => {})
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => null) }));
vi.mock('../../src/lib/stackPopup', async (importOriginal) => {
  const original = await importOriginal<typeof import('../../src/lib/stackPopup')>();
  return {
    ...original,
    getStackPopupRequest: vi.fn(async () => ({ path: 'C:\\drafts', requestId: 'initial' })),
    listStackFolder: vi.fn(async () => ({ path: 'C:\\drafts', entries: [{ path: 'C:\\drafts\\draft.txt', name: 'draft.txt', entryType: 'File', sizeBytes: 2, modifiedAt: null }], warnings: [], offset: 0, total: 1 })),
    getStackGitStatus: vi.fn(async () => null),
    emitStackFolderListingDiagnostics: vi.fn(),
    resolveStackItemIcons: vi.fn(async () => ({ cacheHits: 0, cacheMisses: 0, items: [] })),
    hideStackPopup: bridge.hide,
    toggleStackPopup: bridge.request
  };
});
vi.mock('../../src/lib/settings', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../src/lib/settings')>()),
  loadShellSettings: vi.fn(async () => ({ stackBrowser: { terminalProfile: 'windowsTerminal' } }))
}));
vi.mock('../../src/components/StackTextEditor.svelte', async () => ({
  default: (await import('./stack-close-guard-editor.svelte')).default
}));

import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';

beforeEach(() => {
  bridge.handlers.clear();
  bridge.hide.mockClear();
  bridge.request.mockClear();
});

it('mock-native close request cannot discard a dirty editor until confirmation', async () => {
  const view = render(StackPopupSurface);
  await waitFor(() => expect(screen.getByText('draft.txt')).toBeTruthy());
  await fireEvent.dblClick(screen.getByText('draft.txt'));
  await fireEvent.click(screen.getByRole('button', { name: /Make draft dirty:/ }));
  await bridge.request();
  expect(bridge.hide).not.toHaveBeenCalled();
  expect(screen.getByRole('alertdialog')).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(bridge.hide).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: /Make draft dirty:/ })).toBeTruthy();
  await bridge.request();
  await bridge.request();
  expect(screen.getAllByRole('alertdialog')).toHaveLength(1);
  await fireEvent.click(screen.getByRole('button', { name: 'Discard' }));
  await waitFor(() => expect(bridge.hide).toHaveBeenCalledTimes(1));
  view.unmount();
  expect(bridge.handlers.get('stack-popup:close-requested')?.size ?? 0).toBe(0);
});

it('mock-native close request closes clean popup without discard prompt', async () => {
  const view = render(StackPopupSurface);
  await waitFor(() => expect(bridge.handlers.get('stack-popup:close-requested')?.size).toBe(1));
  await waitFor(() => expect(screen.getByText('draft.txt')).toBeTruthy());
  await fireEvent.dblClick(screen.getByText('draft.txt'));
  expect(screen.getByRole('button', { name: /Make draft dirty:/ })).toBeTruthy();
  await bridge.request();
  await waitFor(() => expect(bridge.hide).toHaveBeenCalledTimes(1));
  expect(screen.queryByRole('alertdialog')).toBeNull();
  view.unmount();
  expect(bridge.handlers.get('stack-popup:close-requested')?.size ?? 0).toBe(0);
});

it('Back cancels existing native-close dirty confirmation without dismissing draft or hiding popup', async () => {
  render(StackPopupSurface);
  await fireEvent.dblClick(await screen.findByText('draft.txt'));
  await fireEvent.click(screen.getByRole('button', { name: /Make draft dirty:/ }));
  await bridge.request();
  const dialog = await screen.findByRole('alertdialog');
  const event = new MouseEvent('pointerdown', { button: 3, bubbles: true, cancelable: true });
  Object.defineProperty(event, 'pointerType', { value: 'mouse' });
  dialog.dispatchEvent(event);
  expect(event.defaultPrevented).toBe(true);
  await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
  expect(screen.getByRole('button', { name: /Make draft dirty:/ })).toBeTruthy();
  expect(bridge.hide).not.toHaveBeenCalled();
});
