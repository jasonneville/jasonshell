import { cleanup, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import SpeechIndicatorSurface from '../../src/components/SpeechIndicatorSurface.svelte';
import micArtwork from '../../src/assets/icons/mic_24dp_E3E3E3_FILL1_wght300_GRAD0_opsz24.svg?raw';
import * as orbRenderer from '../../src/lib/speechIndicatorOrb';
import { createSpeechIndicatorFill } from '../../src/lib/speechIndicatorFill';
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
let statusUnlisten: ReturnType<typeof vi.fn<() => void>>;
let levelUnlisten: ReturnType<typeof vi.fn<() => void>>;

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
function shell(view: { container: HTMLElement }) {
  const element = view.container.querySelector<HTMLElement>('.mic-shell');
  expect(element).not.toBeNull();
  return element!;
}
function speaking(view: { container: HTMLElement }) {
  // The meter's audible/hold state is now observable as nonzero mic fill,
  // never as a speaking ring class. Missing presentation binding fails hard.
  return micLevel(view) > 0;
}
function micLevel(view: { container: HTMLElement }) {
  const glyph = shell(view).querySelector<HTMLElement>('.mic-glyph');
  expect(glyph).not.toBeNull();
  const scalar = glyph!.style.getPropertyValue('--mic-level').trim();
  expect(scalar, 'mic must expose its rendered normalized fill scalar').not.toBe('');
  const level = Number(scalar);
  expect(Number.isFinite(level)).toBe(true);
  expect(level).toBeGreaterThanOrEqual(0);
  expect(level).toBeLessThanOrEqual(1);
  return level;
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance', 'requestAnimationFrame', 'cancelAnimationFrame'] });
  vi.spyOn(document, 'hidden', 'get').mockReturnValue(false);
  // jsdom has no GPU. Explicitly model the supported opaque fallback path,
  // rather than relying on jsdom's noisy unimplemented canvas API.
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
  vi.stubGlobal('matchMedia', vi.fn((query: string) => ({
    // Existing exact meter/hold assertions run with interpolation bypassed.
    // Normal-motion rendered-value acceptance is isolated below.
    matches: query === '(prefers-reduced-motion: reduce)', media: query, onchange: null,
    addEventListener: vi.fn(), removeEventListener: vi.fn(),
    addListener: vi.fn(), removeListener: vi.fn(), dispatchEvent: vi.fn()
  })));
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
afterEach(() => { cleanup(); vi.clearAllTimers(); vi.useRealTimers(); vi.unstubAllGlobals(); });

describe('decorative orb background and safe fallback', () => {
  it('tries WebGL and retains the original mic without an animation loop when unavailable', async () => {
    const frame = vi.spyOn(window, 'requestAnimationFrame');
    const view = await mount();
    await status('recording');
    const canvas = shell(view).querySelector('canvas');
    const glyph = shell(view).querySelector<HTMLElement>('.mic-glyph');
    expect(HTMLCanvasElement.prototype.getContext).toHaveBeenCalled();
    expect(glyph).not.toBeNull();
    // Failed initialization may remove the canvas or leave it above its static
    // fallback. Either is valid; any retained canvas stays behind the mic.
    if (canvas) expect(canvas.compareDocumentPosition(glyph!)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    const computed = getComputedStyle(glyph!);
    const mask = glyph!.style.maskImage || glyph!.style.mask || computed.maskImage || computed.mask;
    const src = mask.match(/url\(["']?(.+?)["']?\)/)?.[1];
    expect(src, 'rendered mask must reference original packaged SVG').toBeTruthy();
    if (src!.startsWith('data:')) {
      const comma = src!.indexOf(',');
      expect(src!.slice(0, comma)).toMatch(/^data:image\/svg\+xml(?:;[^,]+)?$/);
      const decoded = src!.slice(0, comma).includes(';base64')
        ? atob(src!.slice(comma + 1)) : decodeURIComponent(src!.slice(comma + 1));
      const parse = (svg: string) => new DOMParser().parseFromString(svg, 'image/svg+xml').documentElement;
      expect(parse(decoded).isEqualNode(parse(micArtwork))).toBe(true);
    } else {
      expect(src).toContain('mic_24dp_E3E3E3_FILL1_wght300_GRAD0_opsz24.svg');
    }
    expect(shell(view).querySelector('img')).toBeNull();
    expect(view.container.querySelector('main')?.getAttribute('aria-hidden')).toBe('true');
    expect(frame).not.toHaveBeenCalled();
    // Fallback must not disable recording feedback or change the hold contract.
    await voice(0.035);
    expect(speaking(view)).toBe(true);
    expect(micLevel(view)).toBe(0.07);
    await advance(300);
    expect(speaking(view)).toBe(false);
    view.unmount();
    expect(vi.getTimerCount()).toBe(0);
  });

  it('recording status alone activates the orb; scalar voice traffic never reaches the renderer', async () => {
    const controller = { setActive: vi.fn(), dispose: vi.fn() };
    vi.spyOn(orbRenderer, 'createSpeechIndicatorOrb').mockReturnValue(controller);
    const view = await mount();
    expect(controller.setActive).toHaveBeenLastCalledWith(false);
    await status('recording');
    expect(controller.setActive).toHaveBeenLastCalledWith(true);
    const activations = controller.setActive.mock.calls.length;
    for (const level of [0, 0.035, 0.5, 1]) await voice(level);
    await voice(1, 99);
    expect(controller.setActive).toHaveBeenCalledTimes(activations);
    await status('transcribing', 99);
    expect(controller.setActive).toHaveBeenLastCalledWith(false);
    expect(controller.setActive.mock.calls.every((args) => args.length === 1 && typeof args[0] === 'boolean')).toBe(true);
    view.unmount();
    expect(controller.dispose).toHaveBeenCalledOnce();
  });

  it.each(['(prefers-reduced-motion: reduce)', '(forced-colors: active)'])('%s preserves the mic without starting an animation loop', async (preference) => {
    vi.mocked(window.matchMedia).mockImplementation((query: string) => ({
      matches: query === preference, media: query, onchange: null,
      addEventListener: vi.fn(), removeEventListener: vi.fn(),
      addListener: vi.fn(), removeListener: vi.fn(), dispatchEvent: vi.fn()
    }));
    const frame = vi.spyOn(window, 'requestAnimationFrame');
    const view = await mount();
    await status('recording');
    expect(shell(view).querySelector('.mic-glyph')).not.toBeNull();
    expect(frame).not.toHaveBeenCalled();
    expect(view.container.querySelector('button, a[href], input, [tabindex]:not([tabindex="-1"])')).toBeNull();
    await voice(0.8);
    if (preference === '(forced-colors: active)') await advance(208);
    expect(speaking(view)).toBe(true);
    expect(micLevel(view)).toBe(1);
    view.unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});

describe('rendered mic fill smoothing (meter threshold and hold remain unchanged)', () => {
  let reduced: MediaQueryList;
  beforeEach(() => {
    reduced = Object.assign(new EventTarget(), {
      matches: false, media: '(prefers-reduced-motion: reduce)', onchange: null,
      addListener: vi.fn(), removeListener: vi.fn()
    }) as unknown as MediaQueryList;
    const other = Object.assign(new EventTarget(), {
      matches: false, media: '(forced-colors: active)', onchange: null,
      addListener: vi.fn(), removeListener: vi.fn()
    }) as unknown as MediaQueryList;
    vi.mocked(window.matchMedia).mockImplementation((query) => query === reduced.media ? reduced : other);
  });
  async function reduceMotion(value: boolean) {
    Object.defineProperty(reduced, 'matches', { configurable: true, value });
    reduced.dispatchEvent(new Event('change'));
    await settle();
  }
  async function visibility(hidden: boolean) {
    vi.spyOn(document, 'hidden', 'get').mockReturnValue(hidden);
    document.dispatchEvent(new Event('visibilitychange'));
    await settle();
  }

  it('rises through displayed intermediate values instead of jumping, then reaches exactly one within 200ms plus one frame', async () => {
    const view = await mount();
    await status('recording');
    await voice(1);
    expect(micLevel(view)).toBe(0);
    await advance(32);
    const early = micLevel(view);
    expect(early).toBeGreaterThan(0);
    expect(early).toBeLessThan(1);
    await advance(32);
    expect(micLevel(view)).toBeGreaterThan(early);
    // Leave easing/duration tuning free, bounded by 200ms + RAF quantization.
    await advance(144);
    expect(micLevel(view)).toBe(1);
  });

  it('raw 0.25 approaches display 0.5 continuously in 100ms; raw 0.1 releases to display 0.2 in 180ms', async () => {
    const view = await mount();
    await status('recording');
    await voice(0.25);
    expect(micLevel(view)).toBe(0);
    await advance(32);
    const partial = micLevel(view);
    expect(partial).toBeGreaterThan(0);
    expect(partial).toBeLessThan(0.5);
    await advance(64);
    expect(micLevel(view)).toBeGreaterThan(partial);
    expect(micLevel(view)).toBeLessThan(0.5);
    await advance(16);
    expect(micLevel(view)).toBe(0.5);
    await voice(0.1);
    expect(micLevel(view)).toBe(0.5);
    await advance(160);
    expect(micLevel(view)).toBeGreaterThan(0.2);
    expect(micLevel(view)).toBeLessThan(0.5);
    await advance(32);
    expect(micLevel(view)).toBe(0.2);
  });

  it('audible fall is gentler than rise, monotonic and settles without overshooting', async () => {
    const view = await mount();
    await status('recording');
    await voice(1);
    await advance(48);
    const riseProgress = micLevel(view);
    await advance(160);
    expect(micLevel(view)).toBe(1);
    await voice(0.1); // raw 0.1 maps to display 0.2; easing contract unchanged.
    expect(micLevel(view)).toBe(1);
    await advance(48);
    const fall = micLevel(view);
    expect(fall).toBeGreaterThan(0.2);
    expect(fall).toBeLessThan(1);
    expect((1 - fall) / 0.8).toBeLessThan(riseProgress);
    let previous = fall;
    for (let step = 0; step < 10; step += 1) {
      await advance(16);
      const current = micLevel(view);
      expect(current).toBeLessThanOrEqual(previous);
      expect(current).toBeGreaterThanOrEqual(0.2);
      previous = current;
    }
    expect(micLevel(view)).toBe(0.2);
  });

  it('interruption retargets from displayed value, not previous target or zero', async () => {
    const view = await mount();
    await status('recording');
    await voice(1);
    await advance(48);
    const displayed = micLevel(view);
    expect(displayed).toBeGreaterThan(0);
    expect(displayed).toBeLessThan(1);
    const target = displayed / 2;
    // Ensure an audible target even with a conservative easing curve.
    expect(target / 2).toBeGreaterThanOrEqual(0.035);
    await voice(target / 2); // gain applies before generic interpolation.
    expect(micLevel(view)).toBe(displayed);
    await advance(32);
    const falling = micLevel(view);
    expect(falling).toBeLessThan(displayed);
    expect(falling).toBeGreaterThan(target);
    await voice(0.45);
    expect(micLevel(view)).toBe(falling);
    await advance(32);
    expect(micLevel(view)).toBeGreaterThan(falling);
    expect(micLevel(view)).toBeLessThan(0.9);
    await advance(176);
    expect(micLevel(view)).toBe(0.9);
  });

  it('quiet/invalid/stale traffic retains held target; exactly 300ms expiry begins a bounded release to exact zero', async () => {
    const view = await mount();
    await status('recording');
    await voice(4);
    await advance(208);
    expect(micLevel(view)).toBe(1);
    for (const level of [NaN, Infinity, -Infinity, -1, 0, 0.034]) await voice(level);
    await voice(0.4, 6);
    await advance(91);
    expect(micLevel(view)).toBe(1);
    await advance(1);
    // Meter target is now zero, but normal-motion paint releases rather than snaps.
    expect(micLevel(view)).toBe(1);
    await advance(32);
    expect(micLevel(view)).toBeGreaterThan(0);
    expect(micLevel(view)).toBeLessThan(1);
    await advance(176);
    expect(micLevel(view)).toBe(0);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('repeated identical packets do not restart animation or prevent exact settlement', async () => {
    const view = await mount();
    await status('recording');
    await voice(1);
    for (let step = 0; step < 13; step += 1) {
      await advance(16);
      await voice(1);
    }
    expect(micLevel(view)).toBe(1);
    // Only the unchanged meter hold remains, not an endless presentation loop.
    expect(vi.getTimerCount()).toBe(1);
  });

  it('raw levels at or above 0.5 share a full display target and cannot restart rise on each packet', async () => {
    const view = await mount();
    await status('recording');
    await voice(0.5);
    expect(micLevel(view)).toBe(0);
    let previous = 0;
    for (const raw of [0.75, 1, 4, 0.5, 0.9, 0.6, 0.5]) {
      await advance(16);
      const displayed = micLevel(view);
      expect(displayed).toBeGreaterThanOrEqual(previous);
      await voice(raw);
      expect(micLevel(view)).toBe(displayed);
      previous = displayed;
    }
    expect(micLevel(view)).toBe(1);
    expect(vi.getTimerCount()).toBe(1);
  });

  it.each(['idle', 'transcribing', 'copied', 'error', 'new nonce'] as const)('%s resets displayed fill immediately during interpolation and cancels pending work', async (end) => {
    const view = await mount();
    await status('recording');
    await voice(1);
    await advance(48);
    expect(micLevel(view)).toBeGreaterThan(0);
    if (end === 'new nonce') await status('recording', 8);
    else await status(end, 99);
    expect(micLevel(view)).toBe(0);
    expect(vi.getTimerCount()).toBe(0);
    await voice(1, 7);
    await advance(208);
    expect(micLevel(view)).toBe(0);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('initial reduced motion skips interpolation and retains exact threshold/hold behavior', async () => {
    await reduceMotion(true);
    const view = await mount();
    await status('recording');
    await voice(0.035);
    expect(micLevel(view)).toBe(0.07);
    await advance(299);
    expect(micLevel(view)).toBe(0.07);
    await advance(1);
    expect(micLevel(view)).toBe(0);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('dynamic reduced motion snaps to current target and cancels interpolation, then disabling it restores smoothing', async () => {
    const view = await mount();
    await status('recording');
    await voice(1);
    await advance(32);
    expect(micLevel(view)).toBeLessThan(1);
    await reduceMotion(true);
    expect(micLevel(view)).toBe(1);
    expect(vi.getTimerCount()).toBe(1);
    await voice(0.1);
    expect(micLevel(view)).toBe(0.2);
    await reduceMotion(false);
    await voice(0.4);
    expect(micLevel(view)).toBe(0.2);
    await advance(32);
    expect(micLevel(view)).toBeGreaterThan(0.2);
    expect(micLevel(view)).toBeLessThan(0.8);
    await advance(176);
    expect(micLevel(view)).toBe(0.8);
  });

  it('hidden documents cancel presentation work, do not schedule it for new packets, and resume current target safely', async () => {
    const view = await mount();
    await status('recording');
    await voice(1);
    await advance(32);
    await visibility(true);
    expect(vi.getTimerCount()).toBe(1); // Meter hold only; GPU unavailable here.
    const frames = vi.spyOn(window, 'requestAnimationFrame');
    await voice(0.25);
    expect(frames).not.toHaveBeenCalled();
    expect(vi.getTimerCount()).toBe(1);
    await visibility(false);
    await advance(208);
    expect(micLevel(view)).toBe(0.5);
    await visibility(true);
    await advance(500);
    expect(vi.getTimerCount()).toBe(0);
    await visibility(false);
    await advance(208);
    expect(micLevel(view)).toBe(0);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('unmount cancels interpolation, hold and preference/visibility listeners; subsequent events cannot restart work', async () => {
    const removeMedia = vi.spyOn(reduced, 'removeEventListener');
    const removeDocument = vi.spyOn(document, 'removeEventListener');
    const view = await mount();
    await status('recording');
    await voice(1);
    await advance(32);
    const glyph = shell(view).querySelector<HTMLElement>('.mic-glyph')!;
    view.unmount();
    const detached = glyph.style.getPropertyValue('--mic-level');
    expect(vi.getTimerCount()).toBe(0);
    expect(removeMedia.mock.calls.some(([event]) => event === 'change')).toBe(true);
    expect(removeDocument.mock.calls.some(([event]) => event === 'visibilitychange')).toBe(true);
    await voice(1);
    await reduceMotion(true);
    await visibility(false);
    await advance(500);
    expect(glyph.style.getPropertyValue('--mic-level')).toBe(detached);
    expect(vi.getTimerCount()).toBe(0);
    expect(statusUnlisten).toHaveBeenCalledOnce();
    expect(levelUnlisten).toHaveBeenCalledOnce();
  });
});

describe('fixed mic presentation', () => {
  it.each([
    [0.035, 0.07], [0.1, 0.2], [0.25, 0.5], [0.499, 0.998],
    [0.5, 1], [0.75, 1], [1, 1], [4, 1]
  ])('raw %s maps to display %s without changing speech detection', async (raw, displayed) => {
    const view = await mount();
    await status('recording');
    await voice(raw);
    expect(micLevel(view)).toBe(displayed);
    expect(vi.getTimerCount()).toBe(1);
    await advance(299);
    expect(micLevel(view)).toBe(displayed);
    await advance(1);
    expect(micLevel(view)).toBe(0);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('presentation gain never promotes raw subthreshold noise or invalid packets to speech or extends the hold', async () => {
    const view = await mount();
    await status('recording');
    const noise = [NaN, Infinity, -Infinity, -1, 0, 0.02, 0.034, 0.035 - Number.EPSILON];
    for (const raw of noise) {
      await voice(raw);
      expect(micLevel(view)).toBe(0);
      expect(vi.getTimerCount()).toBe(0);
    }
    await voice(0.25);
    expect(micLevel(view)).toBe(0.5);
    await advance(299);
    for (const raw of noise) await voice(raw);
    expect(micLevel(view)).toBe(0.5);
    await advance(1);
    expect(micLevel(view)).toBe(0);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('generic fill helper accepts display targets unchanged, not raw speech sensitivity', () => {
    const paint = vi.fn();
    const fill = createSpeechIndicatorFill(paint);
    fill.setTarget(0.25);
    expect(paint).toHaveBeenLastCalledWith(0.25);
    fill.setTarget(0.5);
    expect(paint).toHaveBeenLastCalledWith(0.5);
    fill.reset();
    expect(paint).toHaveBeenLastCalledWith(0);
    fill.dispose();
    expect(vi.getTimerCount()).toBe(0);
  });

  it('recording starts with zero fill; matching speech changes only the masked icon, never the shell ring', async () => {
    const view = await mount();
    await status('recording');
    expect(mocks.show).toHaveBeenCalledOnce();
    expect(speaking(view)).toBe(false);
    const quietClass = shell(view).className;
    const quietStyle = shell(view).getAttribute('style');
    await voice(0.035);
    expect(speaking(view)).toBe(true);
    expect(micLevel(view)).toBe(0.07);
    expect(shell(view).className).toBe(quietClass);
    expect(shell(view).getAttribute('style')).toBe(quietStyle);
    expect(shell(view).classList.contains('mic-shell--speaking')).toBe(false);
    expect(view.container.querySelector('.waveform, .bar')).toBeNull();
    expect(shell(view).querySelector('img')).toBeNull();
  });

  it('fill follows increasing and decreasing audible levels up to full; shell geometry and styling remain fixed', async () => {
    const view = await mount();
    await status('recording');
    const className = shell(view).className;
    const style = shell(view).getAttribute('style');
    for (const level of [0.035, 0.25, 0.5, 0.8, 1, 0.7, 0.2]) {
      await voice(level);
      expect(micLevel(view)).toBe(Math.min(1, level * 2));
      expect(shell(view).className).toBe(className);
      expect(shell(view).getAttribute('style')).toBe(style);
    }
    expect(shell(view).querySelector('[style*="bar-scale"]')).toBeNull();
  });

  it('zero/subthreshold/invalid packets do not extend held fill; finite loud values clamp to full', async () => {
    const view = await mount();
    await status('recording');
    for (const level of [NaN, Infinity, -Infinity, -1, 0, 0.035 - Number.EPSILON]) {
      await voice(level);
      expect(micLevel(view)).toBe(0);
      expect(vi.getTimerCount()).toBe(0);
    }
    await voice(4);
    expect(micLevel(view)).toBe(1);
    await advance(100);
    await voice(0.2);
    expect(micLevel(view)).toBe(0.4);
    await advance(299);
    for (const level of [NaN, Infinity, -Infinity, -1, 0, 0.01]) await voice(level);
    expect(micLevel(view)).toBe(0.4);
    await advance(1);
    expect(micLevel(view)).toBe(0);
    expect(vi.getTimerCount()).toBe(0);
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
    expect(micLevel(view)).toBe(1);
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
    expect(micLevel(view)).toBe(0.07);
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
    expect(micLevel(view)).toBe(1);
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
    expect(micLevel(view)).toBe(1);
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
