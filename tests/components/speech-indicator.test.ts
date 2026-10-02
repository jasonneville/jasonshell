import { cleanup, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import SpeechIndicatorSurface from '../../src/components/SpeechIndicatorSurface.svelte';
import type { SpeechStatusEvent, SpeechStatusResponse, SpeechVoiceLevelEvent } from '../../src/ipc/events';

// Local mocks: the shared setup deliberately has no speech or show/hide API.
const mocks = vi.hoisted(() => ({
  status: vi.fn(), level: vi.fn(), query: vi.fn(), show: vi.fn(), hide: vi.fn()
}));
vi.mock('../../src/lib/speech', () => ({
  listenSpeechStatus: mocks.status,
  listenSpeechVoiceLevel: mocks.level,
  getSpeechStatus: mocks.query
}));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ label: 'speech-indicator', show: mocks.show, hide: mocks.hide })
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

let statusHandler: (event: SpeechStatusEvent) => void;
let levelHandler: (event: SpeechVoiceLevelEvent) => void;
let statusUnlisten: ReturnType<typeof vi.fn>;
let levelUnlisten: ReturnType<typeof vi.fn>;

// Flush promise registration, query/compensation and Svelte DOM updates without
// advancing the clock. Both setTimeout and performance.now use the fake clock.
async function settle() {
  for (let turn = 0; turn < 5; turn += 1) { await Promise.resolve(); await tick(); }
}
async function advance(ms: number) {
  await vi.advanceTimersByTimeAsync(ms);
  await settle();
}
async function status(kind: SpeechStatusEvent['status'], nonce: number | null = 7) {
  statusHandler({ status: kind, nonce });
  await settle();
}
async function voice(level: number, nonce = 7) {
  levelHandler({ level, nonce });
  await settle();
}
async function mount() {
  const view = render(SpeechIndicatorSurface);
  await settle();
  return view;
}
function shell(view: ReturnType<typeof render>) {
  const element = view.container.querySelector<HTMLElement>('.mic-shell');
  expect(element).not.toBeNull();
  return element!;
}
function speaking(view: ReturnType<typeof render>) {
  return shell(view).classList.contains('mic-shell--speaking');
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });
  Object.values(mocks).forEach((mock) => mock.mockReset());
  statusUnlisten = vi.fn();
  levelUnlisten = vi.fn();
  mocks.status.mockImplementation((handler: typeof statusHandler) => {
    statusHandler = handler;
    return Promise.resolve(statusUnlisten);
  });
  mocks.level.mockImplementation((handler: typeof levelHandler) => {
    levelHandler = handler;
    return Promise.resolve(levelUnlisten);
  });
  mocks.query.mockResolvedValue({ status: 'idle', nonce: null });
  mocks.show.mockResolvedValue(undefined);
  mocks.hide.mockResolvedValue(undefined);
});
afterEach(() => { cleanup(); vi.clearAllTimers(); vi.useRealTimers(); });

describe('fixed mic presentation', () => {
  it('recording starts quiet; matching speech only adds the speaking edge', async () => {
    const view = await mount();
    await status('recording');
    expect(mocks.show).toHaveBeenCalledOnce();
    expect(speaking(view)).toBe(false);
    const quietArt = shell(view).innerHTML;
    await voice(0.035);
    expect(speaking(view)).toBe(true);
    expect(shell(view).innerHTML).toBe(quietArt);
    expect(view.container.querySelector('.waveform, .bar')).toBeNull();
    expect(shell(view).querySelector('img.mic-glyph')?.getAttribute('alt')).toBe('');
  });

  it('low and high audible inputs have identical markup, classes and inline styles', async () => {
    const view = await mount();
    await status('recording');
    await voice(0.035);
    const low = shell(view).outerHTML;
    await voice(1);
    expect(shell(view).outerHTML).toBe(low);
    expect(shell(view).querySelector('[style*="bar-scale"]')).toBeNull();
  });

  it('is aria-hidden with no focusable or interactive controls', async () => {
    const view = await mount();
    await status('recording');
    await voice(0.8);
    expect(view.container.querySelector('main')?.getAttribute('aria-hidden')).toBe('true');
    expect(view.container.querySelector('button, a[href], input, select, textarea, [contenteditable="true"], [tabindex]:not([tabindex="-1"]), [autofocus]')).toBeNull();
  });
});

describe('nonce-scoped 300ms speaking hold', () => {
  it('voice before recording and subthreshold voice while recording stay quiet', async () => {
    const view = await mount();
    await voice(1);
    expect(speaking(view)).toBe(false);
    await status('recording');
    await voice(0.035 - Number.EPSILON);
    expect(speaking(view)).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('quiet packets never extend 299/300ms expiry; timer clears speech without another event', async () => {
    const view = await mount();
    await status('recording');
    await voice(0.6);
    for (let gap = 0; gap < 5; gap += 1) { await advance(50); await voice(0); }
    await advance(49);
    await voice(0.035 - Number.EPSILON);
    expect(speaking(view)).toBe(true);
    await advance(1);
    expect(speaking(view)).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('fresh threshold speech extends the hold by exactly 300ms', async () => {
    const view = await mount();
    await status('recording');
    await voice(0.5);
    await advance(250);
    await voice(0.035);
    await advance(299);
    expect(speaking(view)).toBe(true);
    await advance(1);
    expect(speaking(view)).toBe(false);
  });

  it('stale nonce cannot activate or extend speech', async () => {
    const view = await mount();
    await status('recording');
    await voice(1, 6);
    expect(speaking(view)).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
    await voice(0.5);
    await advance(299);
    await voice(1, 6);
    expect(speaking(view)).toBe(true);
    await advance(1);
    expect(speaking(view)).toBe(false);
  });

  it('new session clears the previous meter and timer; old traffic stays ignored', async () => {
    const view = await mount();
    await status('recording');
    await voice(0.8);
    await advance(100);
    await status('recording', 8);
    expect(speaking(view)).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
    await voice(1, 7);
    expect(speaking(view)).toBe(false);
    await voice(0.035, 8);
    await advance(299);
    expect(speaking(view)).toBe(true);
    await advance(1);
    expect(speaking(view)).toBe(false);
  });

  it('repeated recording status retains speech and its original expiry', async () => {
    const view = await mount();
    await status('recording');
    await voice(0.8);
    await advance(200);
    await status('recording');
    expect(speaking(view)).toBe(true);
    expect(vi.getTimerCount()).toBe(1);
    await advance(99);
    expect(speaking(view)).toBe(true);
    await advance(1);
    expect(speaking(view)).toBe(false);
  });

  it.each(['idle', 'transcribing', 'copied', 'error'] as const)('%s with mismatched nonce hides and resets immediately', async (kind) => {
    const view = await mount();
    await status('recording');
    await voice(0.6);
    const hides = mocks.hide.mock.calls.length;
    await status(kind, 99);
    expect(mocks.hide).toHaveBeenCalledTimes(hides + 1);
    expect(speaking(view)).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
    await voice(1);
    expect(speaking(view)).toBe(false);
    await status('recording', 8);
    expect(speaking(view)).toBe(false);
  });

  it('recording without a nonce fails closed', async () => {
    const view = await mount();
    await status('recording');
    await voice(0.6);
    const hides = mocks.hide.mock.calls.length;
    await status('recording', null);
    expect(mocks.hide).toHaveBeenCalledTimes(hides + 1);
    expect(speaking(view)).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });
});

describe('hydration and async lifecycle', () => {
  it('subscribes before querying and restores an already-recording session quietly', async () => {
    mocks.query.mockResolvedValue({ status: 'recording', nonce: 42 });
    const view = await mount();
    expect(mocks.status.mock.invocationCallOrder[0]).toBeLessThan(mocks.query.mock.invocationCallOrder[0]);
    expect(mocks.show).toHaveBeenCalledOnce();
    expect(speaking(view)).toBe(false);
    await voice(0.5, 7);
    expect(speaking(view)).toBe(false);
    await voice(0.5, 42);
    expect(speaking(view)).toBe(true);
  });

  it.each(['idle', 'transcribing', 'copied', 'error'] as const)('hydrated %s stays hidden', async (kind) => {
    mocks.query.mockResolvedValue({ status: kind, nonce: 42 });
    const view = await mount();
    expect(mocks.show).not.toHaveBeenCalled();
    expect(mocks.hide).toHaveBeenCalledOnce();
    expect(speaking(view)).toBe(false);
  });

  it('new terminal event supersedes a deferred recording snapshot', async () => {
    const query = deferred<SpeechStatusResponse>();
    mocks.query.mockReturnValue(query.promise);
    const view = await mount();
    await status('idle', 99);
    query.resolve({ status: 'recording', nonce: 7 });
    await settle();
    expect(mocks.show).not.toHaveBeenCalled();
    expect(speaking(view)).toBe(false);
  });

  it('new recording event supersedes a deferred terminal snapshot', async () => {
    const query = deferred<SpeechStatusResponse>();
    mocks.query.mockReturnValue(query.promise);
    const view = await mount();
    await status('recording', 8);
    await voice(0.5, 8);
    query.resolve({ status: 'idle', nonce: null });
    await settle();
    expect(mocks.hide).not.toHaveBeenCalled();
    expect(speaking(view)).toBe(true);
  });

  it('query failure without a newer event fails closed', async () => {
    const query = deferred<SpeechStatusResponse>();
    mocks.query.mockReturnValue(query.promise);
    const view = await mount();
    query.reject(new Error('query unavailable'));
    await settle();
    expect(mocks.hide).toHaveBeenCalledOnce();
    expect(mocks.show).not.toHaveBeenCalled();
    expect(speaking(view)).toBe(false);
  });

  it('query failure cannot hide a newer recording event', async () => {
    const query = deferred<SpeechStatusResponse>();
    mocks.query.mockReturnValue(query.promise);
    const view = await mount();
    await status('recording');
    await voice(0.5);
    query.reject(new Error('stale query unavailable'));
    await settle();
    expect(mocks.hide).not.toHaveBeenCalled();
    expect(speaking(view)).toBe(true);
  });

  it.each(['stop', 'unmount'] as const)('late show after %s is compensated with hide', async (end) => {
    const show = deferred<void>();
    mocks.show.mockReturnValue(show.promise);
    const view = await mount();
    await status('recording');
    if (end === 'stop') await status('idle');
    else view.unmount();
    const hides = mocks.hide.mock.calls.length;
    show.resolve(undefined);
    await settle();
    expect(mocks.hide).toHaveBeenCalledTimes(hides + 1);
  });

  it('unmount removes both listeners and cancels the active release timer', async () => {
    const view = await mount();
    await status('recording');
    await voice(0.5);
    expect(vi.getTimerCount()).toBe(1);
    view.unmount();
    expect(statusUnlisten).toHaveBeenCalledOnce();
    expect(levelUnlisten).toHaveBeenCalledOnce();
    expect(vi.getTimerCount()).toBe(0);
    await voice(1);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('deferred listener registrations clean up when they resolve after unmount', async () => {
    const registration = deferred<() => void>();
    const levels = deferred<() => void>();
    mocks.status.mockImplementation((handler: typeof statusHandler) => { statusHandler = handler; return registration.promise; });
    mocks.level.mockImplementation((handler: typeof levelHandler) => { levelHandler = handler; return levels.promise; });
    const view = await mount();
    expect(mocks.query).not.toHaveBeenCalled();
    view.unmount();
    registration.resolve(statusUnlisten);
    levels.resolve(levelUnlisten);
    await settle();
    expect(statusUnlisten).toHaveBeenCalledOnce();
    expect(levelUnlisten).toHaveBeenCalledOnce();
    expect(mocks.query).not.toHaveBeenCalled();
    expect(vi.getTimerCount()).toBe(0);
  });

  it.each(['resolve', 'reject'] as const)('hydration settling via %s after unmount cannot show or hide the window', async (outcome) => {
    const query = deferred<SpeechStatusResponse>();
    mocks.query.mockReturnValue(query.promise);
    const view = await mount();
    view.unmount();
    if (outcome === 'resolve') query.resolve({ status: 'recording', nonce: 7 });
    else query.reject(new Error('disposed query unavailable'));
    await settle();
    expect(mocks.show).not.toHaveBeenCalled();
    expect(mocks.hide).not.toHaveBeenCalled();
  });
});
