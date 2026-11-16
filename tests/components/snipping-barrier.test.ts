// Actual TopBar + unchanged Melt buttons; transport and animation clock only are injected.
// DOM paint acknowledgement is not evidence of DWM composition or native foreground identity.
import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import TopBar from '../../src/components/TopBar.svelte';
import css from '../../src/components/TopBar.css?raw';
import { defaultShellSettings } from '../../src/lib/settings';
import { IPC_COMMANDS } from '../../src/ipc/commands';
import { IPC_EVENTS } from '../../src/ipc/events';
import { tauriMocks } from './setup';

vi.mock('@tauri-apps/api/window', async () => {
  const { tauriMocks } = await import('./setup');
  return { getCurrentWindow: () => ({ label: 'top-bar', listen: tauriMocks().listen,
    onDragDropEvent: vi.fn().mockResolvedValue(vi.fn()) }) };
});
vi.mock('@tauri-apps/api/webviewWindow', async () => {
  const { tauriMocks } = await import('./setup');
  return { getCurrentWebviewWindow: () => ({ label: 'top-bar',
    listen: (name: string, callback: (event: { payload: unknown }) => void) =>
      tauriMocks().listen(name, callback, { target: { kind: 'WebviewWindow', label: 'top-bar' } }) }) };
});
type Token = { generation: string; captureId: string };
const first: Token = { generation: '7', captureId: '77777777777777777777777777777777' };
const second: Token = { generation: '8', captureId: '88888888888888888888888888888888' };
const listeners = new Map<string, (event: { payload: unknown }) => void>();
const unlisteners: ReturnType<typeof vi.fn>[] = [];
let frames: Map<number, FrameRequestCallback>;
let style: HTMLStyleElement;
const calls = (command: string) => tauriMocks().invoke.mock.calls.filter(([name]) => name === command);
beforeEach(() => {
  listeners.clear(); unlisteners.length = 0; localStorage.clear(); frames = new Map(); let id = 0;
  vi.stubGlobal('requestAnimationFrame', vi.fn((callback: FrameRequestCallback) => { frames.set(++id, callback); return id; }));
  vi.stubGlobal('cancelAnimationFrame', vi.fn((id: number) => frames.delete(id)));
  // Vite's jsdom CSS imports are not a browser stylesheet. Install unmodified actual
  // production CSS, not a test-only suppression rule that could manufacture GREEN.
  style = document.createElement('style'); style.textContent = css; document.head.append(style);
  tauriMocks().listen.mockImplementation(async (name, callback) => {
    listeners.set(name, callback); const unlisten = vi.fn(() => listeners.delete(name)); unlisteners.push(unlisten); return unlisten;
  });
  tauriMocks().invoke.mockImplementation(async (command, args) => {
    if (command === 'load_shell_settings') return defaultShellSettings();
    if (command === 'get_speech_model_status') return { state: 'missing', source: null, error: null };
    if (command === 'start_snip') return first;
    if (command === 'snip_bar_ready') return { ...args, accepted: true };
    return [];
  });
});
afterEach(() => { style.remove(); vi.unstubAllGlobals(); });
async function bar() {
  const view = render(TopBar);
  const button = await view.findByRole('button', { name: 'Capture screen region' });
  await waitFor(() => expect(listeners.has('snip:prepare_bar')).toBe(true));
  await waitFor(() => expect(listeners.has('snip:release_bar')).toBe(true));
  const root = view.container.querySelector('.top-bar') as HTMLElement;
  expect(root).toBeTruthy(); return { view, button, root };
}
async function event(name: string, token: Token) { listeners.get(name)!({ payload: token }); await tick(); }
async function frame() {
  const pending = [...frames.entries()]; frames.clear();
  for (const [, callback] of pending) callback(performance.now());
  await tick(); await Promise.resolve();
}
function visible(root: HTMLElement, button: HTMLElement) {
  expect(root.hidden).toBe(false); expect(getComputedStyle(root).display).not.toBe('none');
  expect(getComputedStyle(root).visibility).not.toBe('hidden');
  expect(getComputedStyle(button).display).not.toBe('none'); expect(button.closest('[hidden]')).toBeNull();
}

it('barrier suppresses only real nested Melt tooltips, keeps icons visible, and acks after tick plus frame exactly once', async () => {
  const { root, button } = await bar();
  await fireEvent.focus(button); // Real Melt builder may reopen tooltip during preparation.
  await event('snip:prepare_bar', first);
  expect(root.classList.contains('snip-preparing')).toBe(true); visible(root, button);
  for (const control of root.querySelectorAll('button')) visible(root, control);
  const tooltips = root.querySelectorAll('.melt-action-button-tooltip'); expect(tooltips.length).toBeGreaterThan(0);
  for (const tooltip of tooltips) expect(getComputedStyle(tooltip).display).toBe('none');
  expect(calls('snip_bar_ready')).toHaveLength(0); await frame();
  await waitFor(() => expect(calls('snip_bar_ready')).toHaveLength(1));
  expect(calls('snip_bar_ready')[0][1]).toEqual(first);
  await event('snip:prepare_bar', first); await frame(); expect(calls('snip_bar_ready')).toHaveLength(1);
  expect(calls('start_snip')).toHaveLength(0); // Native Alt+S uses same event, no synthesized start click.
});

it('new prepare invalidates old frame and stale release cannot unsuppress current token', async () => {
  const { root } = await bar();
  await event('snip:prepare_bar', first); await event('snip:prepare_bar', second);
  await event('snip:release_bar', first); expect(root.classList.contains('snip-preparing')).toBe(true);
  await frame(); await waitFor(() => expect(calls('snip_bar_ready')).toHaveLength(1));
  expect(calls('snip_bar_ready')[0][1]).toEqual(second);
  await event('snip:release_bar', second); expect(root.classList.contains('snip-preparing')).toBe(false);
});

it('matching release before animation callback cancels acknowledgement, not just CSS', async () => {
  const { root } = await bar();
  await event('snip:prepare_bar', first); await event('snip:release_bar', first); await frame();
  expect(root.classList.contains('snip-preparing')).toBe(false); expect(calls('snip_bar_ready')).toHaveLength(0);
});

it('unmount invalidates queued frame and unsubscribes prepare/release listeners', async () => {
  const { view } = await bar(); await event('snip:prepare_bar', first); view.unmount(); await frame();
  expect(calls('snip_bar_ready')).toHaveLength(0);
  expect(listeners.has('snip:prepare_bar')).toBe(false); expect(listeners.has('snip:release_bar')).toBe(false);
  expect(unlisteners.filter(fn => fn.mock.calls.length > 0).length).toBeGreaterThanOrEqual(2);
});

it('computed tooltip visibility failure refuses acknowledgement instead of trusting class alone', async () => {
  const { root } = await bar(); const original = window.getComputedStyle.bind(window);
  vi.spyOn(window, 'getComputedStyle').mockImplementation((element, pseudo) => {
    if (element.matches('.melt-action-button-tooltip')) return { display: 'block' } as CSSStyleDeclaration;
    return original(element, pseudo);
  });
  await event('snip:prepare_bar', first); await frame();
  expect(root.classList.contains('snip-preparing')).toBe(true); expect(calls('snip_bar_ready')).toHaveLength(0);
});

it('keyboard click without pointerdown starts zero-argument request but does not suppress before native prepare', async () => {
  const { root, button } = await bar();
  await fireEvent.keyDown(button, { key: 'Enter' }); await fireEvent.keyUp(button, { key: 'Enter' });
  await fireEvent.click(button, { detail: 0 }); // jsdom browser-default keyboard click.
  await waitFor(() => expect(calls('start_snip')).toHaveLength(1));
  expect(calls('start_snip')[0][1] ?? {}).toEqual({});
  expect(root.classList.contains('snip-preparing')).toBe(false); expect(calls('snip_bar_ready')).toHaveLength(0);
  await event('snip:prepare_bar', first); await frame();
  await waitFor(() => expect(calls('snip_bar_ready')).toHaveLength(1)); visible(root, button);
});

// Match SnipPreviewSurface's accessible failure/retry contract. These exercise
// production TopBar and Melt controls, not native capture or the IPC ACL boundary.
function startResponse(response: () => Promise<Token>) {
  const fallback = tauriMocks().invoke.getMockImplementation()!;
  tauriMocks().invoke.mockImplementation((command, args) => command === 'start_snip'
    ? response() : fallback(command, args));
}

it('rejected snip start announces a useful timeout and retry guidance instead of silently doing nothing', async () => {
  startResponse(() => Promise.reject('readiness-timeout'));
  const { view, button, root } = await bar();
  await fireEvent.click(button);
  await waitFor(() => expect(calls('start_snip')).toHaveLength(1));
  const failure = await view.findByRole('alert');
  expect(failure.textContent).toMatch(/snip|capture|screenshot/i);
  expect(failure.textContent).toMatch(/timed?\s*out|timeout|time.*out/i);
  expect(failure.textContent).toMatch(/try again|retry/i);
  expect(getComputedStyle(failure).display).not.toBe('none');
  expect(getComputedStyle(failure).visibility).not.toBe('hidden');
  expect(button.hasAttribute('disabled')).toBe(false);
  expect(root.classList.contains('snip-preparing')).toBe(false);
  expect(calls('snip_bar_ready')).toHaveLength(0);
});

it('retry clears the old snip error immediately and a successful new start does not restore it', async () => {
  let resolveRetry!: (token: Token) => void;
  const retry = new Promise<Token>((resolve) => { resolveRetry = resolve; });
  let attempts = 0;
  startResponse(() => ++attempts === 1 ? Promise.reject('capture-failed') : retry);
  const { view, button } = await bar();
  try {
    await fireEvent.click(button);
    const failure = await view.findByRole('alert');
    expect(failure.textContent).toMatch(/snip|capture|screenshot/i);
    await fireEvent.click(button);
    await waitFor(() => expect(calls('start_snip')).toHaveLength(2));
    expect(view.queryByRole('alert')?.textContent ?? '').toBe('');
    resolveRetry(second);
    await tick();
    await waitFor(() => expect(button.hasAttribute('disabled')).toBe(false));
    expect(view.queryByRole('alert')?.textContent ?? '').toBe('');
    expect(calls('start_snip')).toHaveLength(2);
    for (const [, args] of calls('start_snip')) expect(args ?? {}).toEqual({});
  } finally { resolveRetry(second); }
});

it('pending snip start suppresses repeated clicks but completion permits another request', async () => {
  let resolveStart!: (token: Token) => void;
  const pending = new Promise<Token>((resolve) => { resolveStart = resolve; });
  startResponse(() => pending);
  const { button } = await bar();
  try {
    await fireEvent.click(button);
    await fireEvent.click(button);
    await fireEvent.click(button, { detail: 0 });
    expect(calls('start_snip')).toHaveLength(1);
    resolveStart(first);
    await tick();
    await waitFor(() => expect(button.hasAttribute('disabled')).toBe(false));
    await fireEvent.click(button);
    await waitFor(() => expect(calls('start_snip')).toHaveLength(2));
  } finally { resolveStart(first); }
});

it('IPC registry exposes exact barrier command/events without extra renderer foreground-intent command', () => {
  expect(Object.values(IPC_COMMANDS)).toContain('snip_bar_ready');
  expect(Object.values(IPC_EVENTS)).toContain('snip:prepare_bar');
  expect(Object.values(IPC_EVENTS)).toContain('snip:release_bar');
  expect(Object.values(IPC_COMMANDS)).not.toContain('prepare_snip_intent');
});

const hotkeyFeedback = 'snip:hotkey_feedback';
async function feedback(payload: unknown) {
  expect(listeners.has(hotkeyFeedback), 'TopBar must subscribe to native hotkey feedback').toBe(true);
  listeners.get(hotkeyFeedback)!({ payload });
  await tick();
}
const started = (attempt: string) => ({ attempt, status: 'started', code: null });
const failed = (attempt: string, code = 'readiness-timeout') => ({ attempt, status: 'error', code });

it('hotkey feedback listener targets exact current topbar WebviewWindow and disposes', async () => {
  const { view } = await bar();
  await waitFor(() => expect(listeners.has(hotkeyFeedback)).toBe(true));
  const registrations = tauriMocks().listen.mock.calls.filter(([name]) => name === hotkeyFeedback);
  expect(registrations).toHaveLength(1);
  expect(registrations[0][2]).toEqual({ target: { kind: 'WebviewWindow', label: 'top-bar' } });
  view.unmount();
  expect(listeners.has(hotkeyFeedback)).toBe(false);
});

it('hotkey feedback shows sanitized accessible timeout retry through existing snip error presentation without invoking start', async () => {
  const { view, root, button } = await bar();
  await feedback(started('1')); await feedback(failed('1'));
  const alert = await view.findByRole('alert');
  expect(alert.classList.contains('snip-start-error')).toBe(true);
  expect(alert.textContent).toMatch(/snip|capture|screenshot/i);
  expect(alert.textContent).toMatch(/timed?\s*out|timeout|time.*out/i);
  expect(alert.textContent).toMatch(/try again|retry/i);
  expect(view.queryAllByRole('alert')).toHaveLength(1);
  expect(calls('start_snip')).toHaveLength(0);expect(calls('snip_bar_ready')).toHaveLength(0);
  expect(root.classList.contains('snip-preparing')).toBe(false);
  expect(button.hasAttribute('disabled')).toBe(false);visible(root,button);
});

it('newer hotkey started clears previous click failure before prepare and stale failure cannot restore it', async () => {
  startResponse(() => Promise.reject('capture-failed'));
  const { view, button } = await bar();await fireEvent.click(button);await view.findByRole('alert');
  await feedback(started('2'));
  expect(view.queryByRole('alert')).toBeNull();
  await feedback(failed('1'));expect(view.queryByRole('alert')).toBeNull();
  expect(calls('start_snip')).toHaveLength(1);
});

it('hotkey feedback orders decimal attempts beyond Number precision and ignores stale started or error events', async () => {
  const { view } = await bar();
  await feedback(started('9007199254740992'));await feedback(failed('9007199254740992'));
  await view.findByRole('alert');
  await feedback(started('9007199254740993'));expect(view.queryByRole('alert')).toBeNull();
  await feedback(failed('9007199254740992'));expect(view.queryByRole('alert')).toBeNull();
  await feedback(failed('9007199254740993','capture-failed'));
  const text=(await view.findByRole('alert')).textContent;
  await feedback(started('9007199254740992'));
  expect(view.getByRole('alert').textContent).toBe(text);
});

it('duplicate hotkey error or same-attempt started cannot replace or clear its first announced failure', async () => {
  const { view } = await bar();await feedback(started('3'));await feedback(failed('3'));
  const original=await view.findByRole('alert');const text=original.textContent;
  await feedback(failed('3','capture-failed'));await feedback(started('3'));await feedback(failed('3'));
  expect(view.queryAllByRole('alert')).toHaveLength(1);
  expect(view.getByRole('alert')).toBe(original);expect(original.textContent).toBe(text);
});

it('malformed hotkey feedback cannot mutate failure state or poison the next valid attempt', async () => {
  const { view } = await bar();await feedback(started('4'));await feedback(failed('4'));
  const text=view.getByRole('alert').textContent;
  const invalid: unknown[]=[null,[],{},'error',
    ...['','0','05','-1','1e3','18446744073709551616','9'.repeat(1000)].map(attempt=>started(attempt)),
    {attempt:5,status:'started',code:null},{attempt:'5',status:'success',code:null},
    {attempt:'5',status:'started',code:'capture-failed'},{attempt:'5',status:'error',code:null},
    failed('5','C:\\private\\token=secret'),failed('5','unknown-code')];
  for(const payload of invalid){await feedback(payload);expect(view.getByRole('alert').textContent).toBe(text);}
  await feedback(started('5'));expect(view.queryByRole('alert')).toBeNull();
  await feedback(failed('5','window-failed'));
  expect((await view.findByRole('alert')).textContent).toMatch(/try again|retry/i);
  expect(view.container.textContent).not.toContain('secret');
});
