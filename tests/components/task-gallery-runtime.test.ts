import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, expect, it, vi } from 'vitest';

// Production DOM + mocked IPC only. Never starts Tauri or owns native HWNDs.
const bridge = vi.hoisted(() => ({
  handlers: new Map<string, (event: { payload: unknown }) => unknown>(),
  invoke: vi.fn(), emit: vi.fn()
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: bridge.invoke }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string, handler: (event: { payload: unknown }) => unknown) => {
    bridge.handlers.set(name, handler);
    return () => bridge.handlers.delete(name);
  }, emit: bridge.emit, emitTo: bridge.emit
}));
import TaskGallerySurface from '../../src/components/TaskGallerySurface.svelte';

const rows = Array.from({ length: 30 }, (_, index) => ({
  hwnd: String(101 + index), title: `Window ${index + 1}`, processName: 'Editor', processId: 42,
  iconDataUrl: '', isActive: index === 0, isMinimized: false
}));
// Authoritative native payload contract; dimensions are logical CSS pixels.
function layout(count: number, columns = Math.min(count, 3)) {
  const ribbon = count <= 3 && columns === count;
  return { mode: ribbon ? 'ribbon' : 'matrix', columns,
    thumbnailWidth: 172, thumbnailHeight: 96, cardWidth: 180, cardHeight: 136, footerHeight: 32,
    viewportWidth: columns * 180 + (columns - 1) * 8 + (ribbon ? 0 : 12),
    viewportHeight: Math.min(Math.ceil(count / columns), 2) * 136 + (Math.ceil(count / columns) > 1 ? 8 : 0),
    panelWidth: columns * 180 + (columns - 1) * 8 + (ribbon ? 16 : 28),
    panelHeight: Math.ceil(count / columns) > 1 ? 328 : 184 };
}
async function publish(count = 6, revision = 1, columns = Math.min(count, 3), nonce = 'session-A', windows = rows.slice(0, count)) {
  await bridge.handlers.get('task-gallery:open')?.({ payload: {
    nonce, revision, groupKey: 'editor', label: 'Editor', focusGallery: false,
    windows, layout: layout(count, columns)
  } });
  await tick();
}
async function open(count = 6, columns = Math.min(count, 3)) {
  const view = render(TaskGallerySurface);
  await waitFor(() => expect(bridge.handlers.has('task-gallery:open')).toBe(true));
  await publish(count, 1, columns);
  return view;
}
function activate(index: number) {
  return screen.getByRole('button', { name: new RegExp(`^Activate Window ${index + 1},`) });
}
function close(index: number) { return screen.getByRole('button', { name: `Close Window ${index + 1}` }); }
function calls(name: string) { return bridge.invoke.mock.calls.filter(([command]) => command === name); }
function dimensions(element: Element) {
  const panel = element.closest('.task-gallery-panel') as HTMLElement | null;
  const value = (name: string, fallback: number) => Number.parseFloat(panel?.style.getPropertyValue(name) ?? '') || fallback;
  return { columns: value('--gallery-columns', 3), width: value('--gallery-viewport-width', 568), height: value('--gallery-viewport-height', 280) };
}
function deferredSync() {
  let resolve!: (value: unknown) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function syncReply(args: any, imageDataUrl?: string) {
  return { nonce: args.nonce, revision: args.revision, requestId: args.requestId,
    thumbnails: args.frames.map((frame: any) => ({ hwnd: frame.hwnd,
      source: imageDataUrl ? 'captured' : 'unavailable', imageDataUrl: imageDataUrl ?? null, error: imageDataUrl ? null : 'Mock unavailable' })) };
}
const capture = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=';

beforeEach(() => {
  // Controlled logical geometry, not layout evidence. Browser tests measure real CSS.
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(function (this: HTMLElement) {
    return this.matches('.task-gallery-viewport') ? dimensions(this).width : 584;
  });
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockImplementation(function (this: HTMLElement) {
    return this.matches('.task-gallery-viewport') ? dimensions(this).height : 328;
  });
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
    if (this.matches('.task-gallery-viewport')) {
      const size = dimensions(this); return new DOMRect(8, 40, size.width, size.height);
    }
    const tile = this.closest('[data-gallery-hwnd]');
    if (tile) {
      const siblings = [...tile.parentElement!.querySelectorAll('[data-gallery-hwnd]')];
      const index = siblings.indexOf(tile), columns = dimensions(this).columns;
      return new DOMRect(12 + index % columns * 188, 44 + Math.floor(index / columns) * 144, 172, 96);
    }
    return new DOMRect(0, 0, 584, 328);
  });
  bridge.handlers.clear(); bridge.invoke.mockReset(); bridge.emit.mockReset();
  bridge.emit.mockResolvedValue(undefined);
  bridge.invoke.mockImplementation(async (name: string, request: { args?: Record<string, unknown> }) => {
    if (name === 'allocate_task_preview_request_id') return 1;
    if (name === 'sync_task_gallery_thumbnails') return syncReply(request.args);
    return undefined;
  });
});

it('hover-open gallery does not steal existing DOM focus', async () => {
  const external = document.createElement('button'); external.textContent = 'Foreground app';
  document.body.append(external); external.focus();
  try {
    await open(3);
    expect(document.activeElement).toBe(external);
    await publish(4, 2);
    expect(document.activeElement).toBe(external);
  } finally { external.remove(); }
});

it.each([2, 3])('Ribbon %i windows retains exact snapshot order and does not open separate preview on hover/focus', async (count) => {
  await open(count);
  expect(screen.getAllByRole('button', { name: /^Close Window / }).map((button) => button.getAttribute('aria-label')))
    .toEqual(rows.slice(0, count).map((row) => `Close ${row.title}`));
  await fireEvent.mouseEnter(activate(0));
  activate(0).focus();
  await tick(); await Promise.resolve(); await tick();
  expect(calls('show_task_gallery_window_preview')).toHaveLength(0);
});

it.each([4, 6, 30])('Matrix %i uses authoritative columns for vertical spatial navigation', async (count) => {
  await open(count);
  activate(0).focus(); await tick();
  await fireEvent.keyDown(activate(0), { key: 'ArrowDown' });
  await waitFor(() => expect(document.activeElement).toBe(activate(3)));
  await fireEvent.keyDown(activate(3), { key: 'ArrowUp' });
  await waitFor(() => expect(document.activeElement).toBe(activate(0)));
});

it('narrow authoritative two-column Matrix moves one row, not one item or fixed three columns', async () => {
  await open(6, 2);
  activate(1).focus(); await tick();
  await fireEvent.keyDown(activate(1), { key: 'ArrowDown' });
  await waitFor(() => expect(document.activeElement).toBe(activate(3)));
});

it('3→4 snapshot preserves surviving selected Close DOM focus while switching navigation mode', async () => {
  await open(3);
  const surviving = close(1);
  surviving.focus(); await tick();
  await publish(4, 2);
  expect(document.activeElement).toBe(surviving);
  expect(close(1).tabIndex).toBe(0);
  await fireEvent.keyDown(surviving, { key: 'ArrowDown' });
  await waitFor(() => expect(document.activeElement).toBe(activate(3)));
});

it('inline sync carries current nonce/revision and strictly advancing requestId; closed sessions cannot resync', async () => {
  await open(3);
  await waitFor(() => expect(calls('sync_task_gallery_thumbnails').length).toBeGreaterThan(0));
  const first = calls('sync_task_gallery_thumbnails').at(-1)![1].args;
  expect(first).toMatchObject({ nonce: 'session-A', revision: 1 });
  expect(Number.isSafeInteger(first.requestId)).toBe(true);
  expect(first.requestId).toBeGreaterThan(0);
  await publish(4, 2);
  await waitFor(() => expect(calls('sync_task_gallery_thumbnails').at(-1)![1].args.revision).toBe(2));
  expect(calls('sync_task_gallery_thumbnails').at(-1)![1].args.requestId).toBeGreaterThan(first.requestId);
  await bridge.handlers.get('task-gallery:closed')?.({ payload: { nonce: 'session-A' } });
  await tick();
  const count = calls('sync_task_gallery_thumbnails').length;
  await fireEvent.scroll(window); await tick();
  expect(calls('sync_task_gallery_thumbnails')).toHaveLength(count);
  expect(screen.queryByRole('dialog')).toBeNull();
});

it('older same-session revision cannot roll back authoritative cards', async () => {
  await open(3);
  await publish(4, 3);
  await publish(2, 2);
  expect(screen.getAllByRole('button', { name: /^Close Window / })).toHaveLength(4);
});

it.each(['revision', 'request', 'replacement', 'closed', 'unmount'] as const)('late captured sync after %s cannot repaint', async (boundary) => {
  const pending: { args: any; deferred: ReturnType<typeof deferredSync> }[] = [];
  bridge.invoke.mockImplementation((name: string, request: any) => {
    if (name !== 'sync_task_gallery_thumbnails') return Promise.resolve(undefined);
    if (request.args.frames.length === 0) return Promise.resolve(syncReply(request.args));
    const deferred = deferredSync(); pending.push({ args: request.args, deferred }); return deferred.promise;
  });
  const view = await open(3);
  await waitFor(() => expect(pending.length).toBeGreaterThan(0));
  const stale = pending.splice(0);
  bridge.invoke.mockImplementation(async (name: string, request: any) => name === 'sync_task_gallery_thumbnails' ? syncReply(request.args) : undefined);
  if (boundary === 'revision') await publish(4, 2);
  else if (boundary === 'replacement') await publish(3, 1, 3, 'session-B');
  else if (boundary === 'request') {
    await fireEvent(window, new Event('resize'));
    await waitFor(() => expect(calls('sync_task_gallery_thumbnails').at(-1)![1].args.requestId).toBeGreaterThan(stale.at(-1)!.args.requestId));
  } else if (boundary === 'closed') {
    await bridge.handlers.get('task-gallery:closed')?.({ payload: { nonce: 'session-A' } }); await tick();
  } else view.unmount();
  stale.forEach(({ args, deferred }) => deferred.resolve(syncReply(args, capture)));
  await tick(); await Promise.resolve(); await tick();
  expect(document.querySelectorAll('.task-gallery-captured-image')).toHaveLength(0);
  expect(calls('activate_task_gallery_window')).toHaveLength(0);
});

it('late rejected sync after replacement cannot show stale diagnostics', async () => {
  const deferred = deferredSync();
  bridge.invoke.mockImplementation((name: string) => name === 'sync_task_gallery_thumbnails' ? deferred.promise : Promise.resolve(undefined));
  await open(3);
  await waitFor(() => expect(calls('sync_task_gallery_thumbnails').length).toBeGreaterThan(0));
  bridge.invoke.mockImplementation(async (name: string, request: any) => name === 'sync_task_gallery_thumbnails' ? syncReply(request.args) : undefined);
  await publish(3, 1, 3, 'session-B');
  const errors = vi.spyOn(console, 'error').mockImplementation(() => {});
  deferred.reject(new Error('old capture failed')); await tick(); await Promise.resolve(); await tick();
  expect(errors).not.toHaveBeenCalled();
  expect(screen.queryByRole('status')).toBeNull();
});

it.each(['scroll', 'resize'] as const)('%s clears native destinations before newer visible frame synchronization', async (event) => {
  await open(6);
  await waitFor(() => expect(calls('sync_task_gallery_thumbnails').some(([, request]) => request.args.frames.length > 0)).toBe(true));
  const start = calls('sync_task_gallery_thumbnails').at(-1)![1].args.requestId;
  if (event === 'scroll') await fireEvent.scroll(document.querySelector('.task-gallery-viewport')!);
  else await fireEvent(window, new Event('resize'));
  await waitFor(() => expect(calls('sync_task_gallery_thumbnails').at(-1)![1].args.requestId).toBeGreaterThan(start + 1));
  const newer = calls('sync_task_gallery_thumbnails').filter(([, request]) => request.args.requestId > start);
  expect(newer[0][1].args.frames).toEqual([]);
  expect(newer.at(-1)![1].args.frames.length).toBeGreaterThan(0);
});

it.each(['Escape', 'unmount'] as const)('%s submits empty native registration set for cleanup', async (boundary) => {
  const view = await open(3);
  await waitFor(() => expect(calls('sync_task_gallery_thumbnails').some(([, request]) => request.args.frames.length > 0)).toBe(true));
  if (boundary === 'Escape') await fireEvent.keyDown(window, { key: 'Escape' });
  else view.unmount();
  expect(calls('sync_task_gallery_thumbnails').at(-1)![1].args.frames).toEqual([]);
  expect(calls('hide_task_gallery').length).toBeGreaterThan(0);
});

it('captured fallback and unavailable states retain independent actions without fabricated image', async () => {
  bridge.invoke.mockImplementation(async (name: string, request: any) => {
    if (name !== 'sync_task_gallery_thumbnails') return undefined;
    return { ...syncReply(request.args), thumbnails: request.args.frames.map((frame: any, index: number) => ({
      hwnd: frame.hwnd, source: index === 0 ? 'captured' : 'unavailable',
      imageDataUrl: index === 0 ? capture : null, error: index === 0 ? null : 'Access denied'
    })) };
  });
  await open(3);
  await waitFor(() => expect(document.querySelectorAll('.task-gallery-captured-image')).toHaveLength(1));
  expect(document.querySelector('.task-gallery-captured-image')?.getAttribute('src')).toBe(capture);
  expect(screen.getAllByText(/Preview unavailable/i)).toHaveLength(2);
  expect(screen.getAllByRole('button', { name: /^Close Window / })).toHaveLength(3);
});

it('failed captured image degrades to truthful unavailable state, never broken or fabricated thumbnail', async () => {
  bridge.invoke.mockImplementation(async (name: string, request: any) => name === 'sync_task_gallery_thumbnails' ? syncReply(request.args, capture) : undefined);
  await open(3);
  await waitFor(() => expect(document.querySelectorAll('.task-gallery-captured-image')).toHaveLength(3));
  const first = document.querySelector('.task-gallery-captured-image')!;
  await fireEvent.error(first); await tick();
  const card = activate(0).closest('[data-gallery-hwnd]')!;
  expect(card.textContent).toMatch(/preview unavailable/i);
  const image = card.querySelector<HTMLImageElement>('.task-gallery-captured-image');
  expect(image === null || image.hidden || image.style.visibility === 'hidden' || image.style.display === 'none').toBe(true);
  expect(close(0)).toBeTruthy();
});

it.each(['new request', 'new revision', 'replacement session'] as const)('old image error after %s cannot clear newer identical-URL capture', async (boundary) => {
  bridge.invoke.mockImplementation(async (name: string, request: any) => name === 'sync_task_gallery_thumbnails' ? syncReply(request.args, capture) : undefined);
  await open(3);
  await waitFor(() => expect(document.querySelectorAll('.task-gallery-captured-image')).toHaveLength(3));
  const oldImage = activate(0).querySelector('.task-gallery-captured-image')!;
  if (boundary === 'new request') await fireEvent(window, new Event('resize'));
  else if (boundary === 'new revision') await publish(3, 2);
  else await publish(3, 1, 3, 'session-B');
  await waitFor(() => {
    const image = activate(0).querySelector('.task-gallery-captured-image');
    expect(image).not.toBeNull(); expect(image).not.toBe(oldImage);
  });
  const currentImage = activate(0).querySelector('.task-gallery-captured-image')!;
  expect(currentImage.getAttribute('src')).toBe(capture);
  await fireEvent.error(oldImage); await tick();
  expect(activate(0).querySelector('.task-gallery-captured-image')).toBe(currentImage);
  expect(activate(0).textContent).toMatch(/Captured/);
  expect(calls('close_task_gallery_window')).toHaveLength(0);
});

it('fresh accepted capture recovers unavailable state after image error, including identical URL', async () => {
  bridge.invoke.mockImplementation(async (name: string, request: any) => name === 'sync_task_gallery_thumbnails' ? syncReply(request.args, capture) : undefined);
  await open(3);
  await waitFor(() => expect(activate(0).querySelector('.task-gallery-captured-image')).not.toBeNull());
  await fireEvent.error(activate(0).querySelector('.task-gallery-captured-image')!);
  expect(activate(0).textContent).toMatch(/Preview unavailable/);
  expect(activate(0).textContent).not.toMatch(/Captured/);
  await fireEvent(window, new Event('resize'));
  await waitFor(() => expect(activate(0).querySelector('.task-gallery-captured-image')?.getAttribute('src')).toBe(capture));
  expect(activate(0).textContent).toMatch(/Captured/);
  expect(activate(0).textContent).not.toMatch(/Preview unavailable/);
});

// Review gate: IPC receipt is not native cleanup acknowledgement.
async function pendingContextClear() {
  const view = await open(6);
  await waitFor(() => expect(calls('sync_task_gallery_thumbnails').some(([, request]) => request.args.frames.length > 0)).toBe(true));
  const clears: { args: any; deferred: ReturnType<typeof deferredSync> }[] = [];
  bridge.invoke.mockImplementation(async (name: string, request: any) => {
    if (name !== 'sync_task_gallery_thumbnails') return undefined;
    if (request.args.frames.length) return syncReply(request.args);
    const deferred = deferredSync(); clears.push({ args: request.args, deferred });
    return deferred.promise;
  });
  await fireEvent.contextMenu(activate(5), { clientX: 560, clientY: 310 });
  await waitFor(() => expect(clears.length).toBeGreaterThan(0));
  return { view, clears, first: clears[0] };
}

it('context menu waits for successful current empty-frame cleanup acknowledgement, not IPC receipt', async () => {
  const { first } = await pendingContextClear();
  expect(screen.queryByRole('menu')).toBeNull();
  first.deferred.resolve(syncReply(first.args));
  await waitFor(() => expect(screen.getByRole('menu', { name: 'Window 6 actions' })).toBeTruthy());
});

it('rejected context cleanup never paints a menu and reports accessible actionable failure', async () => {
  const { first } = await pendingContextClear();
  first.deferred.reject(new Error('Native thumbnail cleanup denied'));
  await tick();
  expect(screen.queryByRole('menu')).toBeNull();
  await waitFor(() => expect(screen.getByRole('alert').textContent).toMatch(/(?:clear|cleanup|preview|thumbnail).*(?:retry|try again)/i));
  expect(calls('activate_task_gallery_window')).toHaveLength(0);
});

it.each(['Escape', 'native dismissal', 'replacement session', 'removed target', 'replacement context', 'unmount'] as const)(
  'late context cleanup acknowledgement after %s cannot resurrect the old menu', async (boundary) => {
    const { view, first } = await pendingContextClear();
    if (boundary === 'Escape') await fireEvent.keyDown(window, { key: 'Escape' });
    else if (boundary === 'native dismissal') await bridge.handlers.get('task-gallery:closed')?.({ payload: { nonce: 'session-A' } });
    else if (boundary === 'replacement session') await publish(6, 1, 3, 'session-B');
    else if (boundary === 'removed target') await publish(5, 2);
    else if (boundary === 'replacement context') await fireEvent.contextMenu(activate(1), { clientX: 220, clientY: 90 });
    else view.unmount();
    first.deferred.resolve(syncReply(first.args));
    await tick(); await tick();
    expect(screen.queryByRole('menu')).toBeNull();
    expect(calls('activate_task_gallery_window')).toHaveLength(0);
  }
);

it('same-session unchanged refresh preserves an open menu, its focus, and native suspension', async () => {
  await open(6);
  await fireEvent.contextMenu(activate(5), { clientX: 560, clientY: 310 });
  await waitFor(() => expect(screen.getByRole('menu', { name: 'Window 6 actions' })).toBeTruthy());
  const menu = screen.getByRole('menu');
  const focused = document.activeElement;
  const start = calls('sync_task_gallery_thumbnails').length;
  await publish(6, 2);
  await tick(); await new Promise(resolve => setTimeout(resolve, 40));
  expect(screen.queryByRole('menu', { name: 'Window 6 actions' })).toBe(menu);
  expect(document.activeElement).toBe(focused);
  expect(calls('sync_task_gallery_thumbnails').slice(start).every(([, request]) => request.args.frames.length === 0)).toBe(true);
  await fireEvent.click(screen.getByRole('menuitem', { name: 'Minimize' }));
  expect(calls('activate_task_gallery_window').at(-1)![1].args).toMatchObject({ hwnd: '106', nonce: 'session-A', minimizeIfActive: true });
});

it('same-session unchanged refresh never sends an empty registration set or removes accepted captures', async () => {
  bridge.invoke.mockImplementation(async (name: string, request: any) => name === 'sync_task_gallery_thumbnails' ? syncReply(request.args, capture) : undefined);
  await open(3);
  await waitFor(() => expect(document.querySelectorAll('.task-gallery-captured-image')).toHaveLength(3));
  const start = calls('sync_task_gallery_thumbnails').length;
  await publish(3, 2);
  expect(document.querySelectorAll('.task-gallery-captured-image')).toHaveLength(3);
  await tick(); await new Promise(resolve => setTimeout(resolve, 40));
  expect(calls('sync_task_gallery_thumbnails').slice(start).filter(([, request]) => request.args.frames.length === 0)).toHaveLength(0);
});

it('same-session refresh while cleanup pending preserves right-click intent until a current clear acknowledges', async () => {
  const { clears } = await pendingContextClear();
  await publish(6, 2);
  expect(screen.queryByRole('menu')).toBeNull();
  // Old acknowledgement is insufficient; implementation may reissue cleanup for
  // current revision, but must not discard the user's surviving target intent.
  for (let round = 0; round < 4; round++) {
    clears.splice(0).forEach(({ args, deferred }) => deferred.resolve(syncReply(args)));
    await tick(); await new Promise(resolve => setTimeout(resolve, 20));
  }
  expect(screen.queryByRole('menu', { name: 'Window 6 actions' })).not.toBeNull();
  expect(calls('sync_task_gallery_thumbnails').at(-1)![1].args.frames).toEqual([]);
});

it.each(['Escape', 'removed target', 'replacement session'] as const)('%s dismisses open context; obsolete actions cannot dispatch', async boundary => {
  await open(6);
  await fireEvent.contextMenu(activate(5), { clientX: 560, clientY: 310 });
  await waitFor(() => expect(screen.getByRole('menu')).toBeTruthy());
  if (boundary === 'Escape') await fireEvent.keyDown(window, { key: 'Escape' });
  else if (boundary === 'removed target') await publish(5, 2);
  else await publish(6, 1, 3, 'session-B');
  expect(screen.queryByRole('menu')).toBeNull();
  expect(calls('activate_task_gallery_window')).toHaveLength(0);
  expect(screen.getByRole('dialog')).toBeTruthy();
});

it('live thumbnails retain real captured backups while acknowledged menu cleanup clears every native destination', async () => {
  bridge.invoke.mockImplementation(async (name: string, request: any) => {
    if (name !== 'sync_task_gallery_thumbnails') return undefined;
    return { ...syncReply(request.args), thumbnails: request.args.frames.map((frame: any) => ({
      hwnd: frame.hwnd, source: 'live', imageDataUrl: capture, error: null
    })) };
  });
  await open(6);
  await waitFor(() => expect(calls('sync_task_gallery_thumbnails').some(([, request]) => request.args.frames.length === 6)).toBe(true));
  await tick();
  await fireEvent.contextMenu(activate(5), { clientX: 560, clientY: 310 });
  await waitFor(() => expect(screen.getByRole('menu')).toBeTruthy());
  expect(calls('sync_task_gallery_thumbnails').at(-1)![1].args.frames).toEqual([]);
  expect(document.querySelectorAll('.task-gallery-captured-image')).toHaveLength(6);
  expect([...document.querySelectorAll('.task-gallery-captured-image')].every(image => image.getAttribute('src') === capture)).toBe(true);
  expect(screen.queryAllByText(/Preview unavailable/)).toHaveLength(0);
  await publish(6, 2);
  expect(document.querySelectorAll('.task-gallery-captured-image')).toHaveLength(6);
});

it.each(['nonce', 'revision', 'requestId'] as const)('mismatched cleanup acknowledgement %s never authorizes context paint', async (field) => {
  const { first } = await pendingContextClear();
  const reply = syncReply(first.args);
  first.deferred.resolve({ ...reply, [field]: field === 'nonce' ? 'obsolete-session' : reply[field] + 1 });
  await tick(); await tick();
  expect(screen.queryByRole('menu')).toBeNull();
  expect(calls('activate_task_gallery_window')).toHaveLength(0);
});

it('same HWND with changed PID cannot inherit an accepted captured image', async () => {
  bridge.invoke.mockImplementation(async (name: string, request: any) => name === 'sync_task_gallery_thumbnails' ? syncReply(request.args, capture) : undefined);
  await open(3);
  await waitFor(() => expect(activate(0).querySelector('.task-gallery-captured-image')).not.toBeNull());
  const oldImage = activate(0).querySelector('.task-gallery-captured-image');
  const pending = deferredSync();
  bridge.invoke.mockImplementation((name: string, request: any) => {
    if (name !== 'sync_task_gallery_thumbnails') return Promise.resolve(undefined);
    return request.args.frames.length ? pending.promise : Promise.resolve(syncReply(request.args));
  });
  await publish(3, 2, 3, 'session-A', rows.slice(0, 3).map((row, index) => index === 0 ? { ...row, processId: 99 } : row));
  expect(activate(0).querySelector('.task-gallery-captured-image')).toBeNull();
  expect(oldImage?.isConnected).toBe(false);
  expect(activate(1).querySelector('.task-gallery-captured-image')?.getAttribute('src')).toBe(capture);
  expect(calls('activate_task_gallery_window')).toHaveLength(0);
});

it('same HWND with changed PID dismisses open menu instead of inheriting old target actions', async () => {
  await open(6);
  await fireEvent.contextMenu(activate(0), { clientX: 20, clientY: 60 });
  await waitFor(() => expect(screen.getByRole('menu', { name: 'Window 1 actions' })).toBeTruthy());
  await publish(6, 2, 3, 'session-A', rows.slice(0, 6).map((row, index) => index === 0 ? { ...row, processId: 99 } : row));
  expect(screen.queryByRole('menu')).toBeNull();
  expect(calls('activate_task_gallery_window')).toHaveLength(0);
  expect(screen.getAllByRole('button', { name: /^Close Window / })).toHaveLength(6);
});

it('same HWND with changed PID never dispatches an inherited menu action to the replacement process', async () => {
  await open(6);
  await fireEvent.contextMenu(activate(0), { clientX: 20, clientY: 60 });
  await waitFor(() => expect(screen.getByRole('menu', { name: 'Window 1 actions' })).toBeTruthy());
  await publish(6, 2, 3, 'session-A', rows.slice(0, 6).map((row, index) => index === 0 ? { ...row, processId: 99 } : row));
  const inherited = screen.queryByRole('menuitem', { name: 'Minimize' });
  if (inherited) await fireEvent.click(inherited);
  expect(calls('activate_task_gallery_window')).toHaveLength(0);
});

it('same HWND with changed PID invalidates pending menu intent despite current cleanup acknowledgement', async () => {
  const { clears } = await pendingContextClear();
  await publish(6, 2, 3, 'session-A', rows.slice(0, 6).map((row, index) => index === 5 ? { ...row, processId: 99 } : row));
  for (let round = 0; round < 4; round++) {
    clears.splice(0).forEach(({ args, deferred }) => deferred.resolve(syncReply(args)));
    await tick(); await new Promise(resolve => setTimeout(resolve, 20));
  }
  expect(screen.queryByRole('menu')).toBeNull();
  expect(calls('activate_task_gallery_window')).toHaveLength(0);
});
