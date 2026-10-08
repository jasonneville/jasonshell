import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Host from './overflow-ticker-host.svelte';

// Controlled layout + WAAPI contract evidence. Actual interpolated transforms
// belong to the browser smoke of BottomBar and TaskGallerySurface.
let width = 100;
let contentWidth = 180;
let reduced = false;
let resize: ResizeObserverCallback;
const cancel = vi.fn();
const animate = vi.fn((_frames: Keyframe[] | PropertyIndexedKeyframes, _options?: number | KeyframeAnimationOptions) => ({ cancel, play: vi.fn(), finish: vi.fn() }));
const mediaListeners = new Set<(event: MediaQueryListEvent) => void>();

beforeEach(() => {
  vi.useFakeTimers();
  width = 100; contentWidth = 180; reduced = false;
  animate.mockClear(); cancel.mockClear(); mediaListeners.clear();
  vi.stubGlobal('ResizeObserver', class {
    constructor(callback: ResizeObserverCallback) { resize = callback; }
    observe() {} unobserve() {} disconnect() {}
  });
  vi.stubGlobal('matchMedia', vi.fn(() => ({
    get matches() { return reduced; }, media: '(prefers-reduced-motion: reduce)',
    addEventListener: (_: string, listener: (event: MediaQueryListEvent) => void) => mediaListeners.add(listener),
    removeEventListener: (_: string, listener: (event: MediaQueryListEvent) => void) => mediaListeners.delete(listener)
  })));
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(() => width);
  vi.spyOn(HTMLElement.prototype, 'scrollWidth', 'get').mockImplementation(() => contentWidth);
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
    return new DOMRect(0, 0, this.textContent === 'Long document title' && !this.children.length ? contentWidth : width, 20);
  });
  vi.stubGlobal('Animation', class {});
  Object.defineProperty(HTMLElement.prototype, 'animate', { configurable: true, value: animate });
});
afterEach(() => { vi.unstubAllGlobals(); delete (HTMLElement.prototype as any).animate; });
async function mount() {
  const view = render(Host, { text: 'Long document title' });
  await tick();
  return { ...view, button: screen.getByRole('button', { name: 'Long document title' }) };
}
async function hover(button: HTMLElement) { await fireEvent.mouseEnter(button); await tick(); }
function contract() {
  const [frames, options] = animate.mock.calls.at(-1)! as unknown as [Keyframe[], KeyframeAnimationOptions];
  return { frames, options };
}

it('uses measured 80px overflow, 600ms engagement delay, 40px/sec travel and exact endpoint/origin holds', async () => {
  const { button } = await mount();
  expect(animate).not.toHaveBeenCalled();
  await hover(button);
  expect(animate).toHaveBeenCalled();
  const { frames, options } = contract();
  expect(options).toMatchObject({ delay: 600, duration: 9000, iterations: Infinity, easing: 'linear' });
  // 2000ms outward, 2000ms end hold, 2000ms return, 3000ms origin hold.
  expect(frames.map(frame => frame.transform)).toEqual([
    'translateX(0px)', 'translateX(-80px)', 'translateX(-80px)', 'translateX(0px)', 'translateX(0px)'
  ]);
  expect(frames.map(frame => frame.offset)).toEqual([0, 2 / 9, 4 / 9, 6 / 9, 1]);
  expect(button.textContent).toBe('Application iconLong document title');
  expect(button.querySelector('[tabindex]')).toBeNull();
});

it('fractional intrinsic width stays stable when inline-block scrollWidth rounds upward', async () => {
  contentWidth = 180.4;
  vi.spyOn(HTMLElement.prototype, 'scrollWidth', 'get').mockImplementation(function (this: HTMLElement) {
    return this.parentElement?.classList.contains('moving') ? 181 : 0;
  });
  const { button } = await mount(); await hover(button);
  expect(animate).toHaveBeenCalledOnce();
  await vi.advanceTimersByTimeAsync(600);
  resize([], {} as ResizeObserver); await tick();
  expect(animate).toHaveBeenCalledOnce();
  expect(cancel).not.toHaveBeenCalled();
});

it.each([80, 100])('does not animate text fitting %ipx viewport', async (size) => {
  contentWidth = size;
  const { button } = await mount(); await hover(button);
  expect(animate).not.toHaveBeenCalled();
});

it('leave cancels pending delay or running animation and reentry starts fresh', async () => {
  const { button } = await mount(); await hover(button);
  await fireEvent.mouseLeave(button); await tick();
  expect(cancel).toHaveBeenCalled();
  animate.mockClear(); await hover(button);
  expect(contract().options.delay).toBe(600);
});

it('focus provides equivalent motion; leaving hover retains motion until focus also leaves', async () => {
  const { button } = await mount();
  button.focus(); await tick();
  expect(animate).toHaveBeenCalled();
  await hover(button); cancel.mockClear();
  await fireEvent.mouseLeave(button); await tick();
  expect(cancel).not.toHaveBeenCalled();
  button.blur(); await tick();
  expect(cancel).toHaveBeenCalled();
});

it('blur keeps hovered title moving; cancellation during initial delay prevents later reveal', async () => {
  const { button } = await mount(); await hover(button);
  button.focus(); await tick(); cancel.mockClear();
  button.blur(); await tick();
  expect(cancel).not.toHaveBeenCalled();
  await fireEvent.mouseLeave(button); await tick();
  expect(cancel).toHaveBeenCalled();
  await vi.advanceTimersByTimeAsync(600);
  expect(button.querySelector('.moving')).toBeNull();
});

it('remeasures resize and text replacement; newly fitting title becomes inert', async () => {
  const { button, rerender } = await mount(); await hover(button);
  width = 140;
  resize([], {} as ResizeObserver); await tick();
  expect(cancel).toHaveBeenCalled();
  expect(contract().frames.some(frame => frame.transform === 'translateX(-40px)')).toBe(true);
  contentWidth = 90;
  await rerender({ text: 'Short' }); await tick();
  animate.mockClear();
  resize([], {} as ResizeObserver); await tick();
  expect(animate).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Short' })).toBe(button);
});

it('reduced motion disables engagement and cancels motion when preference changes', async () => {
  reduced = true;
  const { button } = await mount(); await hover(button);
  expect(animate).not.toHaveBeenCalled();
  reduced = false;
  for (const listener of mediaListeners) listener({ matches: false } as MediaQueryListEvent);
  await tick();
  expect(animate).toHaveBeenCalled();
  reduced = true; cancel.mockClear();
  for (const listener of mediaListeners) listener({ matches: true } as MediaQueryListEvent);
  await tick();
  expect(cancel).toHaveBeenCalled();
});

it('unmount cancels motion so detached titles cannot keep running', async () => {
  const { button, unmount } = await mount(); await hover(button);
  cancel.mockClear(); unmount();
  expect(cancel).toHaveBeenCalled();
});
