import { render, waitFor, within } from '@testing-library/svelte';
import { beforeEach, expect, it, vi } from 'vitest';
import { tauriMocks } from './setup';
import BottomBar from '../../src/components/BottomBar.svelte';

// Real BottomBar and shared button; mocked IPC only. jsdom proves DOM contracts,
// not CSS layout. Browser cell geometry lives in bottom-bar-alignment-checks.js.
const bridge = tauriMocks();
const icon = 'data:image/svg+xml,%3Csvg xmlns="http://www.w3.org/2000/svg"/%3E';

beforeEach(() => {
  bridge.listen.mockResolvedValue(() => {});
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(800);
});

async function mountCapsule(count = 4) {
  const windows = Array.from({ length: count }, (_, index) => ({
    hwnd: String(index + 1), title: `Editor ${index + 1}`, processName: 'Editor',
    processId: 100 + index, iconDataUrl: icon, isActive: false, isMinimized: false,
    activityState: 'idle', attentionState: 'idle', toastCount: 0
  }));
  bridge.invoke.mockImplementation(async (name: string) => {
    if (name === 'list_open_task_windows') return windows;
    if (name === 'list_pinned_taskbar_apps') return [];
    if (name === 'load_shell_settings') return { ui: { lockBottomBarHeight: true, bottomBarHeightLogical: 32.4 } };
    return undefined;
  });
  const mounted = render(BottomBar);
  const capsule = await waitFor(() => {
    const button = document.querySelector<HTMLButtonElement>('.task-capsule');
    expect(button).not.toBeNull();
    return button!;
  });
  return { ...mounted, capsule };
}

it.each([2, 3, 4, 6, 30])('%i-window capsule retains app identity, exact count and decorative mode', async (count) => {
  const { capsule } = await mountCapsule(count);
  expect(capsule.querySelector('.task-icon')?.getAttribute('src')).toBe(icon);
  expect(capsule.querySelector('.task-label')?.textContent).toBe('Editor');
  expect(within(capsule).getByLabelText(`${count} windows`).textContent).toBe(String(count));
  const matrix = count >= 4;
  const marks = capsule.querySelector(matrix ? '.task-group-glyph' : '.task-ribbon-marks');
  expect(marks?.getAttribute('aria-hidden')).toBe('true');
  expect(marks?.querySelectorAll(matrix ? 'rect' : 'i')).toHaveLength(matrix ? 4 : count);
  expect(capsule.querySelector(matrix ? '.task-ribbon-marks' : '.task-group-glyph')).toBeNull();
  const label = capsule.querySelector('.task-label');
  expect(capsule.querySelector('.task-icon')?.nextElementSibling,
    'every capsule must keep the app icon immediately beside its title').toBe(label);
  if (matrix) {
    expect(label?.nextElementSibling, 'Matrix identity belongs after the title').toBe(marks);
    expect(marks?.nextElementSibling, 'Matrix glyph must immediately precede the count').toBe(capsule.querySelector('.task-count'));
  }
});

it('keeps equal whole-device-pixel sides through initial scale, observer and window resize; cleans up', async () => {
  let scale = 1.25;
  Object.defineProperty(SVGElement.prototype, 'getScreenCTM', { configurable: true, value: () => ({ a: scale }) });
  vi.spyOn(window, 'devicePixelRatio', 'get').mockReturnValue(1.5);
  const observers: { callback: () => void; observe: ReturnType<typeof vi.fn>; disconnect: ReturnType<typeof vi.fn> }[] = [];
  vi.stubGlobal('ResizeObserver', class {
    observe = vi.fn(); disconnect = vi.fn();
    constructor(public callback: () => void) { observers.push(this); }
  });
  const add = vi.spyOn(window, 'addEventListener'), remove = vi.spyOn(window, 'removeEventListener');
  try {
    const { capsule, unmount } = await mountCapsule();
    const glyph = capsule.querySelector('svg')!;
    const observer = observers.find(o => o.observe.mock.calls.some(([node]) => node === glyph))!;
    expect(observer).toBeTruthy();
    const observation = observer.observe.mock.calls.find(([node]) => node === glyph)!;
    expect(observation[1]).toEqual({ box: 'device-pixel-content-box' });
    const assertSide = () => {
      const physicalScale = scale * window.devicePixelRatio;
      const expected = Math.max(1, Math.round(3 * physicalScale));
      for (const rect of glyph.querySelectorAll('rect')) {
        expect(Number(rect.getAttribute('width')) * physicalScale).toBeCloseTo(expected, 10);
        expect(rect.getAttribute('height')).toBe(rect.getAttribute('width'));
      }
    };
    assertSide();
    scale = 1.5; observer.callback(); assertSide();
    scale = 0.1; window.dispatchEvent(new Event('resize')); assertSide();
    const resize = add.mock.calls.find(([name, callback]) => name === 'resize' && callback === observer.callback);
    expect(resize).toBeTruthy();
    const finalSide = glyph.querySelector('rect')!.getAttribute('width');
    unmount();
    expect(observer.disconnect).toHaveBeenCalledOnce();
    expect(remove.mock.calls.some(([name, callback]) => name === 'resize' && callback === observer.callback)).toBe(true);
    scale = 2; window.dispatchEvent(new Event('resize'));
    expect(glyph.querySelector('rect')!.getAttribute('width')).toBe(finalSide);
  } finally { delete (SVGElement.prototype as any).getScreenCTM; }
});

it.each([undefined, null, NaN, Infinity, 0, -1])('handles missing/null or invalid screen scale %s without corrupting cells', async (value) => {
  if (value !== undefined) Object.defineProperty(SVGElement.prototype, 'getScreenCTM', { configurable: true, value: () => value === null ? null : ({ a: value }) });
  vi.spyOn(window, 'devicePixelRatio', 'get').mockReturnValue(1.25);
  try {
    const { capsule } = await mountCapsule();
    for (const rect of capsule.querySelectorAll('.task-group-glyph rect')) {
      const side = Number(rect.getAttribute('width'));
      expect(side).toBeGreaterThan(0);
      expect(Number.isFinite(side)).toBe(true);
      expect(rect.getAttribute('height')).toBe(rect.getAttribute('width'));
      expect(side).toBe(value === undefined || value === null ? 3.2 : 3);
    }
  } finally { delete (SVGElement.prototype as any).getScreenCTM; }
});
