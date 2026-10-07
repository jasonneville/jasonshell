import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { beforeEach, expect, it, vi } from 'vitest';

// Bounded rendered cross-surface journey. Real components and IPC wrappers; all
// Tauri commands/events mocked. Never starts the shell or touches real HWNDs.
const bridge = vi.hoisted(() => ({
  handlers: new Map<string, Set<(event: { payload: any }) => unknown>>(),
  invoke: vi.fn(), emitTo: vi.fn(), emit: vi.fn(), sequence: 0
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: bridge.invoke }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string, handler: (event: { payload: any }) => unknown) => {
    const handlers = bridge.handlers.get(name) ?? new Set();
    handlers.add(handler); bridge.handlers.set(name, handlers);
    return () => handlers.delete(handler);
  },
  emitTo: bridge.emitTo, emit: bridge.emit
}));
import BottomBar from '../../src/components/BottomBar.svelte';
import ContextMenuOverlaySurface from '../../src/components/ContextMenuOverlaySurface.svelte';
import type { TaskbarWindow } from '../../src/lib/taskbarWindows';

const OPEN = 'context-menu-overlay:open';
const SELECT = 'context-menu-overlay:select';
const row = (hwnd: string, overrides: Partial<TaskbarWindow> = {}): TaskbarWindow => ({
  hwnd, title: `Editor ${hwnd}`, processName: 'Editor', processId: Number(hwnd) + 1000,
  iconDataUrl: '', isActive: false, isMinimized: false,
  activityState: 'idle', attentionState: 'idle', toastCount: 0, ...overrides
});
const initial = () => [row('101'), row('102', { isActive: true }), row('201', { processName: 'Other' })];
async function publish(name: string, payload: unknown) {
  for (const handler of bridge.handlers.get(name) ?? []) await handler({ payload });
  await tick();
}
function calls(command: string) { return bridge.invoke.mock.calls.filter(([name]) => name === command); }
function actions() { return calls('run_task_window_action').map(([, args]) => args.request); }
async function settle() { await tick(); await Promise.resolve(); await tick(); }
async function snapshot(windows: TaskbarWindow[]) {
  await publish('taskbar:windows-snapshot', { sequence: ++bridge.sequence, windows });
}
async function mount(windows = initial()) {
  bridge.invoke.mockImplementation(async (name: string) => {
    if (name === 'list_open_task_windows') return windows;
    if (name === 'list_pinned_taskbar_apps') return [];
    if (name === 'load_shell_settings') return { ui: { lockBottomBarHeight: true, bottomBarHeightLogical: 32.4 } };
    if (name === 'allocate_task_preview_request_id') return ++bridge.sequence;
    return undefined;
  });
  render(BottomBar);
  render(ContextMenuOverlaySurface);
  await waitFor(() => expect(document.querySelector('.task-capsule')).not.toBeNull());
  await waitFor(() => expect(bridge.handlers.has(SELECT) && bridge.handlers.has(OPEN)).toBe(true));
  return document.querySelector<HTMLButtonElement>('.task-capsule')!;
}
async function openGroup(capsule: HTMLButtonElement) {
  const count = calls('show_context_menu_overlay').length;
  await fireEvent.contextMenu(capsule, { clientX: 140, clientY: 720 });
  await waitFor(() => expect(calls('show_context_menu_overlay'), 'capsule right-click must request shared overlay').toHaveLength(count + 1), { timeout: 200 });
  const request = calls('show_context_menu_overlay').at(-1)![1].request;
  expect(request).toMatchObject({ kind: 'task-group', source: 'bottom-bar' });
  expect(typeof request.token).toBe('string');
  expect(request.token.length).toBeGreaterThan(0);
  await waitFor(() => expect(screen.getByRole('menu', { name: 'Context menu' })).toBeTruthy());
  return request;
}
function menuButton(name: string) {
  return within(screen.getByRole('menu', { name: 'Context menu' })).getByRole('menuitem', { name });
}
async function choose(name: string) { await fireEvent.click(menuButton(name)); await settle(); }
function selection(request: any, overrides: Record<string, unknown> = {}) {
  return { source: request.source, kind: request.kind, token: request.token, action: 'closeAll', ...overrides };
}
function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((yes) => { resolve = yes; });
  return { promise, resolve };
}
beforeEach(() => {
  bridge.handlers.clear(); bridge.invoke.mockReset(); bridge.emitTo.mockReset(); bridge.emit.mockReset(); bridge.sequence = 0;
  // Narrow geometry fixture induces the existing auto-policy 2-window capsule.
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(80);
  bridge.emitTo.mockImplementation(async (_target: string, name: string, payload: unknown) => publish(name, payload));
  bridge.emit.mockResolvedValue(undefined);
});

it('capsule badge right-click opens exactly the three group actions, with presentation-only PID', async () => {
  const capsule = await mount();
  expect(within(capsule).getByLabelText('2 windows')).toBeTruthy();
  const request = await openGroup(capsule);
  expect(request.processId).toBe(1102);
  expect(Object.keys(request).sort()).toEqual(['kind', 'processId', 'source', 'token', 'x', 'y']);
  expect(bridge.emitTo).toHaveBeenCalledWith('context-menu-overlay', OPEN, request);
  expect(within(screen.getByRole('menu')).getAllByRole('menuitem').map((item) => item.textContent?.trim())).toEqual([
    'Open in Process Manager', 'Pin to taskbar', 'Close all windows'
  ]);
  expect(menuButton('Close all windows').className).toMatch(/destructive/);
  expect(actions()).toEqual([]);
  expect(calls('activate_task_window')).toHaveLength(0);
});

it('overlay task-group branch alone renders shared actions rather than launcher fallback', async () => {
  render(ContextMenuOverlaySurface);
  await waitFor(() => expect(bridge.handlers.has(OPEN)).toBe(true));
  await publish(OPEN, { source: 'bottom-bar', kind: 'task-group', token: 'presentation-only', x: 0, y: 0, processId: 42 });
  expect(menuButton('Open in Process Manager')).toBeTruthy();
  expect(menuButton('Pin to taskbar')).toBeTruthy();
  expect(menuButton('Close all windows')).toBeTruthy();
  expect(screen.queryByText('Unpin from taskbar')).toBeNull();
});

it.each(['Open in Process Manager', 'Pin to taskbar'])( '%s journeys through overlay token to active captured representative', async (name) => {
  const request = await openGroup(await mount());
  await choose(name);
  expect(bridge.emitTo.mock.calls.some(([target, event, payload]) => target === 'bottom-bar' && event === SELECT && payload.token === request.token && payload.kind === 'task-group')).toBe(true);
  expect(actions()).toEqual([{ hwnd: '102', action: name === 'Pin to taskbar' ? 'pin' : 'process', processId: name === 'Pin to taskbar' ? null : 1102 }]);
  expect(screen.queryByRole('menu')).toBeNull();
  expect(calls('hide_context_menu_overlay').length).toBeGreaterThan(0);
});

it.each(['Open in Process Manager', 'Pin to taskbar'])('%s falls back to first live captured member when active member vanished', async (name) => {
  await openGroup(await mount());
  await snapshot([row('101'), row('103'), row('201', { processName: 'Other' })]);
  await choose(name);
  expect(actions()).toEqual([{ hwnd: '101', action: name === 'Pin to taskbar' ? 'pin' : 'process', processId: name === 'Pin to taskbar' ? null : 1101 }]);
});

it('process skips active member without valid PID but pin still uses active member', async () => {
  const capsule = await mount([row('101'), row('102', { isActive: true, processId: null })]);
  expect((await openGroup(capsule)).processId).toBe(1101);
  await choose('Open in Process Manager');
  expect(actions()).toEqual([{ hwnd: '101', action: 'process', processId: 1101 }]);
  await openGroup(capsule);
  await choose('Pin to taskbar');
  expect(actions().at(-1)).toEqual({ hwnd: '102', action: 'pin', processId: null });
});

it('no valid PID disables process presentation and forged process selection cannot invoke it', async () => {
  const request = await openGroup(await mount([row('101', { processId: null }), row('102', { processId: 0 })]));
  expect((menuButton('Open in Process Manager') as HTMLButtonElement).disabled).toBe(true);
  await publish(SELECT, selection(request, { action: 'process', processId: 9999, hwnd: '201' }));
  expect(actions()).toEqual([]);
});

it('close-all invokes normal close once per captured member; duplicate selection is inert; never kills a process', async () => {
  const request = await openGroup(await mount());
  await choose('Close all windows');
  await waitFor(() => expect(actions()).toHaveLength(2));
  expect(actions()).toEqual([{ hwnd: '101', action: 'request-close', processId: 1101 }, { hwnd: '102', action: 'request-close', processId: 1102 }]);
  await publish(SELECT, selection(request));
  expect(actions()).toHaveLength(2);
  expect(bridge.invoke.mock.calls.some(([name]) => /kill|terminate/.test(name))).toBe(false);
});

it('close-all excludes vanished, newly joined and unrelated members', async () => {
  await openGroup(await mount());
  await snapshot([row('101'), row('103'), row('201', { processName: 'Other' })]);
  await choose('Close all windows');
  expect(actions()).toEqual([{ hwnd: '101', action: 'request-close', processId: 1101 }]);
});

it.each(['PID changed', 'group changed'])('close-all refuses captured HWND whose %s', async (change) => {
  await openGroup(await mount());
  await snapshot([row('101', change === 'PID changed' ? { processId: 9001 } : { processName: 'Other' }), row('102'), row('103')]);
  await choose('Close all windows');
  expect(actions()).toEqual([{ hwnd: '102', action: 'request-close', processId: 1102 }]);
});

it.each(['Open in Process Manager', 'Pin to taskbar'])('%s rejects reused captured active HWND/PID and uses surviving captured member', async (name) => {
  await openGroup(await mount());
  await snapshot([row('101'), row('102', { isActive: true, processId: 9999 }), row('103')]);
  await choose(name);
  expect(actions()).toEqual([{ hwnd: '101', action: name === 'Pin to taskbar' ? 'pin' : 'process', processId: name === 'Pin to taskbar' ? null : 1101 }]);
});

it('selection-supplied HWND/PID cannot override originating captured representative', async () => {
  const request = await openGroup(await mount());
  await publish(SELECT, selection(request, { action: 'process', hwnd: '201', processId: 9999, windows: [row('201')] }));
  expect(actions()).toEqual([{ hwnd: '102', action: 'process', processId: 1102 }]);
});

it('close-all continues after individual normal-close rejection', async () => {
  await openGroup(await mount());
  vi.spyOn(console, 'error').mockImplementation(() => {});
  bridge.invoke.mockImplementation(async (name: string, args: any) => {
    if (name === 'run_task_window_action' && args.request.hwnd === '101') throw new Error('WM_CLOSE denied');
  });
  await choose('Close all windows');
  await waitFor(() => expect(actions()).toHaveLength(2));
  expect(actions().map((action) => action.hwnd)).toEqual(['101', '102']);
  expect(actions().every((action) => action.action === 'request-close' && action.processId > 0)).toBe(true);
});

it('rechecks membership after pending first close before dispatching next captured HWND', async () => {
  await openGroup(await mount());
  const pending = deferred();
  bridge.invoke.mockImplementation((name: string, args: any) => name === 'run_task_window_action' && args.request.hwnd === '101' ? pending.promise : Promise.resolve());
  await choose('Close all windows');
  await snapshot([row('101'), row('102', { processId: 9999 }), row('103')]);
  pending.resolve(); await settle();
  expect(actions()).toEqual([{ hwnd: '101', action: 'request-close', processId: 1101 }]);
});

it.each(['process', 'pin', 'closeAll'])('%s rejects vanished captured group even if a new group has the same key', async (action) => {
  const request = await openGroup(await mount());
  await snapshot([row('301'), row('302'), row('201', { processName: 'Other' })]);
  await publish(SELECT, selection(request, { action }));
  expect(actions()).toEqual([]);
});

it.each([
  { token: 'wrong-token' }, { source: 'top-bar' }, { kind: 'task-window' },
  { action: 'unpin' }, { action: 'close' }, { action: 'kill' }
])('ignores invalid group selection %j without consuming valid current menu', async (invalid) => {
  const request = await openGroup(await mount());
  await publish(SELECT, selection(request, invalid));
  expect(actions()).toEqual([]);
  await choose('Pin to taskbar');
  expect(actions()).toEqual([{ hwnd: '102', action: 'pin', processId: null }]);
});

it('second group menu invalidates first token', async () => {
  const capsule = await mount();
  const old = await openGroup(capsule);
  const current = await openGroup(capsule);
  expect(current.token).not.toBe(old.token);
  await publish(SELECT, selection(old));
  expect(actions()).toEqual([]);
  await choose('Pin to taskbar');
  expect(actions()).toHaveLength(1);
});

it.each(['direct-to-group', 'group-to-direct'])('switching menu kind invalidates previous origin context: %s', async (direction) => {
  const capsule = await mount();
  const direct = document.querySelector<HTMLButtonElement>('.task-group-direct .task-button')!;
  async function openDirect() {
    await fireEvent.contextMenu(direct);
    await settle();
    return calls('show_context_menu_overlay').at(-1)![1].request;
  }
  const old = direction === 'direct-to-group' ? await openDirect() : await openGroup(capsule);
  if (direction === 'direct-to-group') await openGroup(capsule); else await openDirect();
  await publish(SELECT, selection(old, { action: direction === 'direct-to-group' ? 'close' : 'closeAll' }));
  expect(actions()).toEqual([]);
  await choose('Pin to taskbar');
  expect(actions()[0].hwnd).toBe(direction === 'direct-to-group' ? '102' : '201');
});

it.each(['direct', 'group'])('async menu-opening race: pending first group hide cannot supersede latest %s menu', async (latestKind) => {
  const firstCapsule = await mount([...initial(), row('301', { processName: 'Second' }), row('302', { processName: 'Second', isActive: true })]);
  const pending = deferred();
  const originalInvoke = bridge.invoke.getMockImplementation()!;
  let heldFirstHide = false;
  bridge.invoke.mockImplementation((name: string, ...args: unknown[]) => {
    if (name === 'hide_task_window_preview' && !heldFirstHide) {
      heldFirstHide = true;
      return pending.promise;
    }
    return originalInvoke(name, ...args);
  });
  try {
    await fireEvent.contextMenu(firstCapsule);
    await waitFor(() => expect(heldFirstHide, 'first group opening must reach the held preview hide').toBe(true), { timeout: 200 });
    expect(calls('show_context_menu_overlay')).toHaveLength(0);
    const latestTarget = latestKind === 'direct'
      ? document.querySelector<HTMLButtonElement>('.task-group-direct .task-button')!
      : document.querySelectorAll<HTMLButtonElement>('.task-capsule')[1];
    await fireEvent.contextMenu(latestTarget);
    await waitFor(() => expect(calls('show_context_menu_overlay')).toHaveLength(1), { timeout: 200 });
    const latest = calls('show_context_menu_overlay')[0][1].request;
    expect(latest.kind).toBe(latestKind === 'direct' ? 'task-window' : 'task-group');
    pending.resolve(); await settle(); await settle();
    expect(calls('show_context_menu_overlay')).toHaveLength(1);
    expect(bridge.emitTo.mock.calls.filter(([, event]) => event === OPEN).map(([, , request]) => request)).toEqual([latest]);
    await choose('Pin to taskbar');
    expect(actions()).toEqual([{ hwnd: latestKind === 'direct' ? '201' : '302', action: 'pin', processId: null }]);
    const selected = bridge.emitTo.mock.calls.filter(([, event]) => event === SELECT).at(-1)![2];
    expect(selected.token).toBe(latest.token);
    expect(selected.kind).toBe(latest.kind);
  } finally {
    pending.resolve(); await settle();
  }
});

it.each([
  ['missing', undefined], ['null', null], ['zero', 0]
] as const)('close-all skips %s PID member and requests only valid captured PID', async (_label, invalidPid) => {
  await openGroup(await mount([row('101', { processId: invalidPid }), row('102', { isActive: true })]));
  await choose('Close all windows');
  await settle();
  expect(actions()).toEqual([{ hwnd: '102', action: 'request-close', processId: 1102 }]);
});

it('close-all with only missing/zero PIDs issues no task action, including forged expected PID', async () => {
  const request = await openGroup(await mount([row('101', { processId: undefined }), row('102', { processId: 0 })]));
  await choose('Close all windows');
  await publish(SELECT, selection(request, { processId: 9999 }));
  expect(actions()).toEqual([]);
});

it('group overlay keyboard navigates three rows and Escape dismisses without selecting', async () => {
  await openGroup(await mount());
  const process = menuButton('Open in Process Manager');
  const pin = menuButton('Pin to taskbar');
  const close = menuButton('Close all windows');
  await waitFor(() => expect(document.activeElement).toBe(process));
  const user = userEvent.setup();
  await user.keyboard('[ArrowDown]'); expect(document.activeElement).toBe(pin);
  await user.keyboard('[ArrowDown]'); expect(document.activeElement).toBe(close);
  await user.keyboard('[ArrowDown]'); expect(document.activeElement).toBe(process);
  await user.keyboard('[ArrowUp]'); expect(document.activeElement).toBe(close);
  await user.keyboard('[Home]'); expect(document.activeElement).toBe(process);
  await user.keyboard('[End]'); expect(document.activeElement).toBe(close);
  await user.keyboard('[Escape]'); await settle();
  expect(screen.queryByRole('menu')).toBeNull();
  expect(calls('hide_context_menu_overlay').at(-1)).toEqual(['hide_context_menu_overlay', { request: { source: 'bottom-bar', restoreOriginFocus: true } }]);
  expect(bridge.emitTo.mock.calls.filter(([, event]) => event === SELECT)).toHaveLength(0);
  expect(actions()).toEqual([]);
});

it('group overlay keyboard skips disabled process row and Enter selects pin with origin token', async () => {
  const request = await openGroup(await mount([row('101', { processId: null }), row('102', { processId: 0 })]));
  const pin = menuButton('Pin to taskbar');
  const close = menuButton('Close all windows');
  expect((menuButton('Open in Process Manager') as HTMLButtonElement).disabled).toBe(true);
  await waitFor(() => expect(document.activeElement).toBe(pin));
  const user = userEvent.setup();
  await user.keyboard('[ArrowDown]'); expect(document.activeElement).toBe(close);
  await user.keyboard('[ArrowDown]'); expect(document.activeElement).toBe(pin);
  await user.keyboard('[ArrowUp]'); expect(document.activeElement).toBe(close);
  await user.keyboard('[Home]'); expect(document.activeElement).toBe(pin);
  await user.keyboard('[End]'); expect(document.activeElement).toBe(close);
  await user.keyboard('[Home][Enter]'); await settle();
  expect(bridge.emitTo).toHaveBeenCalledWith('bottom-bar', SELECT, { source: 'bottom-bar', kind: 'task-group', token: request.token, action: 'pin' });
  expect(actions()).toEqual([{ hwnd: '101', action: 'pin', processId: null }]);
});

it('right-click cancels pending hover gallery before it can open', async () => {
  const capsule = await mount();
  vi.useFakeTimers();
  await fireEvent.mouseEnter(capsule);
  await fireEvent.contextMenu(capsule);
  await settle();
  await vi.advanceTimersByTimeAsync(350);
  expect(calls('show_task_gallery')).toHaveLength(0);
  expect(calls('show_context_menu_overlay')).toHaveLength(1);
});

it('right-click hides already-open gallery before shared menu is shown', async () => {
  const capsule = await mount();
  await fireEvent.click(capsule);
  await waitFor(() => expect(calls('show_task_gallery')).toHaveLength(1));
  const nonce = calls('show_task_gallery')[0][1].args.nonce;
  await openGroup(capsule);
  expect(calls('hide_task_gallery').some(([, args]) => args.nonce === nonce)).toBe(true);
  const hideIndex = bridge.invoke.mock.calls.findIndex(([name, args]) => name === 'hide_task_gallery' && args.nonce === nonce);
  const showIndex = bridge.invoke.mock.calls.findIndex(([name]) => name === 'show_context_menu_overlay');
  expect(hideIndex).toBeLessThan(showIndex);
  expect(capsule.getAttribute('aria-expanded')).toBe('false');
});

it('existing direct tile context menu retains single-window close transport', async () => {
  await mount();
  const direct = document.querySelector<HTMLButtonElement>('.task-group-direct .task-button')!;
  await fireEvent.contextMenu(direct);
  await waitFor(() => expect(screen.getByText('Close window')).toBeTruthy());
  expect(calls('show_context_menu_overlay')[0][1].request.kind).toBe('task-window');
  await choose('Close window');
  expect(actions()).toEqual([{ hwnd: '201', action: 'close', processId: null }]);
});

it('existing capsule left-click opens gallery and drag suppresses generated click without native actions', async () => {
  const capsule = await mount();
  await fireEvent.click(capsule);
  await waitFor(() => expect(calls('show_task_gallery')).toHaveLength(1));
  await fireEvent.click(capsule);
  await settle();
  const group = capsule.closest('[data-task-group-key]')!;
  const pointer = (name: string, x: number) => {
    const event = new MouseEvent(name, { bubbles: true, button: 0, clientX: x });
    Object.defineProperty(event, 'pointerId', { value: 7 });
    return fireEvent(group, event);
  };
  await pointer('pointerdown', 10); await pointer('pointermove', 40); await pointer('pointerup', 40);
  await fireEvent.click(capsule, { detail: 1 }); await settle();
  expect(calls('show_task_gallery')).toHaveLength(1);
  expect(calls('show_context_menu_overlay')).toHaveLength(0);
  expect(calls('activate_task_window')).toHaveLength(0);
  expect(actions()).toEqual([]);
});
