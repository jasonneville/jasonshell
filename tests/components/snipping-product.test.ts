import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { defaultShellSettings } from '../../src/lib/settings';
import { tauriMocks } from './setup';

const native = vi.hoisted(() => ({ label: 'snip-overlay' }));
vi.mock('@tauri-apps/api/window', async () => {
  const { tauriMocks } = await import('./setup');
  return { getCurrentWindow: () => ({ label: native.label, listen: tauriMocks().listen,
    onDragDropEvent: vi.fn().mockResolvedValue(vi.fn()) }) };
});

type Context = { generation: string; captureId: string; phase: 'selecting' | 'preview'; monitorId: string;
  width: number; height: number; scaleFactor?: number; originX?: number; originY?: number };
const selecting: Context = { generation: '7', captureId: '77777777777777777777777777777777', phase: 'selecting', monitorId: 'm0',
  width: 1000, height: 750, scaleFactor: 1.25, originX: -1000, originY: 0 };
const preview: Context = { ...selecting, phase: 'preview', width: 126, height: 75 };
// Transport-only PNG stub; jsdom decode is controlled. Actual PNG pixels are validated
// independently by native clipboard tests, not claimed from these placeholder bytes.
const png = new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]);
let context: Context;
let copyResult: Record<string, unknown>;
let saveResult: Record<string, unknown>;
let imageResponse: () => Promise<Uint8Array>;
type TokenPair = Pick<Context, 'generation' | 'captureId'>;
const handlers = new Map<string, (event: { payload: Context | TokenPair }) => void>();
let autoArm: boolean;
const token = () => ({ generation: context.generation, captureId: context.captureId });
const calls = (name: string) => tauriMocks().invoke.mock.calls.filter(([command]) => command === name);
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
async function component(name: string) {
  const path = `../../src/components/${name}.svelte`;
  // Missing components fail individual runnable RED cases rather than abort collection.
  return (await import(/* @vite-ignore */ path)).default;
}

beforeEach(() => {
  native.label = 'snip-overlay';
  context = { ...selecting };
  copyResult = { status: 'rejected', committed: false, durable: false, code: 'clipboard-busy' };
  saveResult = { status: 'cancelled' };
  imageResponse = async () => png;
  autoArm = true;
  handlers.clear(); localStorage.clear();
  vi.stubGlobal('URL', Object.assign(class extends URL {}, {
    createObjectURL: vi.fn(() => `blob:synthetic-${Math.random()}`), revokeObjectURL: vi.fn()
  }));
  Object.defineProperty(HTMLImageElement.prototype, 'decode', { configurable: true, value: vi.fn().mockResolvedValue(undefined) });
  Object.defineProperty(Element.prototype, 'setPointerCapture', { configurable: true, value: vi.fn() });
  Object.defineProperty(Element.prototype, 'releasePointerCapture', { configurable: true, value: vi.fn() });
  Object.defineProperty(Element.prototype, 'hasPointerCapture', { configurable: true, value: vi.fn(() => true) });
  tauriMocks().listen.mockImplementation(async (name, callback) => { handlers.set(name, callback); return vi.fn(); });
  tauriMocks().invoke.mockImplementation(async (command: string) => {
    switch (command) {
      case 'load_shell_settings': return defaultShellSettings();
      case 'get_speech_model_status': return { state: 'missing', source: null, error: null };
      case 'start_snip': return token();
      case 'get_snip_context': return { ...context };
      case 'get_snip_image': return imageResponse();
      case 'snip_ready': {
        const acknowledged = token();
        // This renderer may decode before the other monitor. False is NOT permission
        // to select; native shows all then emits a targeted, token-bound armed event.
        if (autoArm) queueMicrotask(() => handlers.get('snip:armed')?.({ payload: acknowledged }));
        return { ...acknowledged, ready: false };
      }
      case 'snip_begin_selection': return { ...token(), accepted: true };
      case 'complete_snip': return { ...token(), completed: true };
      case 'cancel_snip': case 'dismiss_snip': return { ...token(), closed: true };
      case 'copy_snip': return { ...token(), ...copyResult };
      case 'save_snip': return { ...token(), ...saveResult };
      default: return []; // Existing TopBar unrelated bootstrap calls, not snip effects.
    }
  });
});

afterEach(() => { vi.unstubAllGlobals(); });

async function overlay() {
  native.label = 'snip-overlay-7-m0'; context = { ...selecting };
  const view = render(await component('SnipOverlaySurface'));
  await waitFor(() => expect(calls('get_snip_image')).toHaveLength(1));
  for (const image of view.container.querySelectorAll('img')) await fireEvent.load(image);
  const region = await view.findByRole('region', { name: 'Screen snip selection' });
  vi.spyOn(region, 'getBoundingClientRect').mockReturnValue({ left: 0, top: 0, right: 800, bottom: 600,
    width: 800, height: 600, x: 0, y: 0, toJSON: () => ({}) });
  await waitFor(() => expect(calls('snip_ready')).toHaveLength(1));
  return { view, region };
}
async function mountPreview() {
  native.label = 'snip-preview-7'; context = { ...preview };
  const urlsBefore = vi.mocked(URL.createObjectURL).mock.calls.length;
  const view = render(await component('SnipPreviewSurface'));
  await waitFor(() => expect(vi.mocked(URL.createObjectURL).mock.calls.length).toBeGreaterThan(urlsBefore));
  for (const image of view.container.querySelectorAll('img')) await fireEvent.load(image);
  await view.findByRole('img', { name: 'Snip preview' });
  await view.findByRole('button', { name: 'Copy', exact: true });
  return view;
}
async function pointer(target: Element, type: string, x: number, y: number, pointerId = 1, isPrimary = true) {
  const event = new MouseEvent(type, { bubbles: true, clientX: x, clientY: y, button: 0,
    buttons: type === 'pointerup' ? 0 : 1 });
  Object.defineProperties(event, { pointerId: { value: pointerId }, isPrimary: { value: isPrimary } });
  await fireEvent(target, event);
}
async function selectReverse(region: Element) {
  await pointer(region, 'pointerdown', 110, 80);
  await waitFor(() => expect(calls('snip_begin_selection')).toHaveLength(1));
  await pointer(region, 'pointermove', 10, 20);
  await pointer(region, 'pointerup', 10, 20);
  await waitFor(() => expect(calls('complete_snip')).toHaveLength(1));
}

it('routes and lazy loads both actual snip surface components', async () => {
  const { resolveSurfaceFromLabel } = await import('../../src/lib/shellSurface');
  const { loadSurfaceComponent } = await import('../../src/lib/surfaceLoader');
  for (const label of ['snip-overlay', 'snip-preview']) {
    const surface = resolveSurfaceFromLabel(label);
    expect(surface).toBe(label);
    expect((await loadSurfaceComponent(surface))?.default).toBeTruthy();
  }
});

it('waits for frozen PNG decode/readiness and revokes its blob on unmount', async () => {
  const pending = deferred<Uint8Array>(); imageResponse = () => pending.promise;
  const view = render(await component('SnipOverlaySurface'));
  await waitFor(() => expect(calls('get_snip_image')).toHaveLength(1));
  expect(calls('snip_ready')).toHaveLength(0);
  expect(view.queryByRole('img', { name: 'Frozen desktop' })).toBeNull();
  pending.resolve(png);
  await waitFor(() => expect(URL.createObjectURL).toHaveBeenCalled());
  for (const image of view.container.querySelectorAll('img')) await fireEvent.load(image);
  await view.findByRole('img', { name: 'Frozen desktop' });
  await waitFor(() => expect(calls('snip_ready')).toHaveLength(1));
  const url = vi.mocked(URL.createObjectURL).mock.results[0].value;
  view.unmount(); expect(URL.revokeObjectURL).toHaveBeenCalledWith(url);
});

it('binds first primary pointer, normalizes reverse CSS drag, completes once without renderer DPI authority', async () => {
  const { view, region } = await overlay();
  await selectReverse(region);
  expect(calls('snip_begin_selection')[0][1]).toEqual({ ...token(), monitorId: 'm0' });
  expect(calls('complete_snip')[0][1]).toEqual({ ...token(), monitorId: 'm0',
    rect: { x: 10, y: 20, width: 100, height: 60 } });
  await pointer(region, 'pointerup', 10, 20);
  await fireEvent(region, new Event('lostpointercapture', { bubbles: true }));
  expect(calls('complete_snip')).toHaveLength(1); expect(calls('cancel_snip')).toHaveLength(0);
  view.unmount();
});

it('clamps held drag to one monitor and ignores duplicate/secondary/foreign pointer', async () => {
  const { region } = await overlay();
  await pointer(region, 'pointerdown', 100, 100);
  await pointer(region, 'pointerdown', 400, 400, 2, false);
  await pointer(region, 'pointerdown', 400, 400, 3, true);
  await pointer(region, 'pointerup', 400, 400, 2, false);
  expect(calls('complete_snip')).toHaveLength(0);
  await pointer(region, 'pointermove', 1200, -200);
  await pointer(region, 'pointerup', 1200, -200);
  await waitFor(() => expect(calls('complete_snip')).toHaveLength(1));
  expect(calls('snip_begin_selection')).toHaveLength(1);
  expect(calls('complete_snip')[0][1]?.rect).toEqual({ x: 100, y: 0, width: 700, height: 100 });
});

it.each([
  [[10, 20], [110, 80]], [[110, 20], [10, 80]], [[10, 80], [110, 20]]
])('normalizes remaining drag direction %j -> %j', async (start, end) => {
  const { region } = await overlay();
  await pointer(region, 'pointerdown', start[0], start[1]);
  await pointer(region, 'pointerup', end[0], end[1]);
  await waitFor(() => expect(calls('complete_snip')).toHaveLength(1));
  expect(calls('complete_snip')[0][1]?.rect).toEqual({ x: 10, y: 20, width: 100, height: 60 });
});

it('native monitor-lock refusal prevents completion from another overlay', async () => {
  const { region } = await overlay();
  const invoke = tauriMocks().invoke.getMockImplementation()!;
  tauriMocks().invoke.mockImplementation((command, args) => command === 'snip_begin_selection'
    ? Promise.resolve({ ...token(), accepted: false }) : invoke(command, args));
  await pointer(region, 'pointerdown', 10, 20);
  await waitFor(() => expect(calls('snip_begin_selection')).toHaveLength(1));
  await pointer(region, 'pointerup', 110, 80);
  expect(calls('complete_snip')).toHaveLength(0);
});

it.each(['pointercancel', 'lostpointercapture', 'Escape'])('cancels %s without completing selection', async (reason) => {
  const { region } = await overlay();
  await pointer(region, 'pointerdown', 10, 20);
  if (reason === 'Escape') await fireEvent.keyDown(window, { key: 'Escape' });
  else await pointer(region, reason, 100, 80);
  await pointer(region, 'pointerup', 100, 80);
  await waitFor(() => expect(calls('cancel_snip')).toHaveLength(1));
  expect(calls('complete_snip')).toHaveLength(0);
});

it('zero-area release cannot complete or publish a crop', async () => {
  const { region } = await overlay();
  await pointer(region, 'pointerdown', 10, 20); await pointer(region, 'pointerup', 10, 20);
  expect(calls('complete_snip')).toHaveLength(0); expect(calls('copy_snip')).toHaveLength(0);
});

it('unmounted pending image cannot publish readiness and stale blobs are revoked', async () => {
  const pending = deferred<Uint8Array>(); imageResponse = () => pending.promise;
  const view = render(await component('SnipOverlaySurface'));
  await waitFor(() => expect(calls('get_snip_image')).toHaveLength(1));
  view.unmount(); pending.resolve(png); await Promise.resolve(); await Promise.resolve();
  expect(calls('snip_ready')).toHaveLength(0);
  expect(vi.mocked(URL.createObjectURL).mock.calls.length).toBe(vi.mocked(URL.revokeObjectURL).mock.calls.length);
});

it('replacement overlay context ignores an older image settlement and readies only current capture', async () => {
  const oldImage = deferred<Uint8Array>(); let reads = 0;
  imageResponse = () => ++reads === 1 ? oldImage.promise : Promise.resolve(png);
  const view = render(await component('SnipOverlaySurface'));
  await waitFor(() => expect(calls('get_snip_image')).toHaveLength(1));
  await waitFor(() => expect(handlers.has('snip:context')).toBe(true));
  context = { ...selecting, generation: '8', captureId: '88888888888888888888888888888888' };
  handlers.get('snip:context')!({ payload: context });
  await waitFor(() => expect(calls('get_snip_image')).toHaveLength(2));
  oldImage.resolve(png);
  await waitFor(() => expect(URL.createObjectURL).toHaveBeenCalled());
  for (const image of view.container.querySelectorAll('img')) await fireEvent.load(image);
  await waitFor(() => expect(calls('snip_ready')).toHaveLength(1));
  expect(calls('snip_ready')[0][1]).toEqual({ ...token(), monitorId: 'm0' });
});

it('rendered fake-native journey starts, selects, retries Copy, cancels/errors/succeeds Save, then dismisses', async () => {
  native.label = 'top-bar';
  const bar = render(await component('TopBar'));
  await fireEvent.click(await bar.findByRole('button', { name: 'Capture screen region' }));
  await waitFor(() => expect(calls('start_snip')).toHaveLength(1)); bar.unmount();
  const selected = await overlay(); await selectReverse(selected.region); selected.view.unmount();
  const view = await mountPreview();
  // Automatic clipboard publication belongs to native completion, never a second
  // renderer Copy on preview mount (coordinator suite verifies native ordering).
  expect(calls('copy_snip')).toHaveLength(0);
  await fireEvent.click(view.getByRole('button', { name: 'Copy', exact: true }));
  await waitFor(() => expect(view.container.textContent).toMatch(/clipboard.*busy|try.*again/i));
  copyResult = { status: 'committed', committed: true, durable: true };
  await fireEvent.click(view.getByRole('button', { name: 'Copy', exact: true }));
  await waitFor(() => expect(view.container.textContent).toMatch(/copied/i));
  await fireEvent.click(view.getByRole('button', { name: 'Save', exact: true }));
  await waitFor(() => expect(calls('save_snip')).toHaveLength(1));
  expect(view.queryByRole('alert')).toBeNull();
  saveResult = { status: 'error', code: 'save-access-denied' };
  await fireEvent.click(view.getByRole('button', { name: 'Save', exact: true }));
  await waitFor(() => expect(view.container.textContent).toMatch(/could not save|unable to save|access.*denied/i));
  saveResult = { status: 'saved' };
  await fireEvent.click(view.getByRole('button', { name: 'Save', exact: true }));
  await waitFor(() => expect(view.container.textContent).toMatch(/saved/i));
  for (const [, args] of calls('save_snip')) expect(args).toEqual(token());
  await fireEvent.click(view.getByRole('button', { name: 'Dismiss', exact: true }));
  await waitFor(() => expect(calls('dismiss_snip')).toHaveLength(1));
});

it.each([
  [{ status: 'publication-unknown', committed: null, durable: null, code: 'clipboard-publication-unknown' }, /unknown|could not confirm/i],
  [{ status: 'committed-warning', committed: true, durable: false, code: 'clipboard-durability-lost' }, /warning|not durable|durability.*not.*guaranteed|may.*lost|could not.*keep/i]
])('preview truthfully renders native Copy outcome %j without claiming durable success', async (result, message) => {
  copyResult = result; const view = await mountPreview();
  await fireEvent.click(view.getByRole('button', { name: 'Copy', exact: true }));
  await waitFor(() => expect(view.container.textContent).toMatch(message));
  expect(view.container.textContent).not.toMatch(/copied successfully|saved successfully/i);
});

it('preview does not dismiss on blur and serializes Copy/Save/Dismiss while picker is pending', async () => {
  const pending = deferred<unknown>(); const view = await mountPreview();
  const invoke = tauriMocks().invoke.getMockImplementation()!;
  tauriMocks().invoke.mockImplementation((command, args) => command === 'save_snip' ? pending.promise : invoke(command, args));
  await fireEvent.blur(window); expect(calls('dismiss_snip')).toHaveLength(0);
  await fireEvent.click(view.getByRole('button', { name: 'Save', exact: true }));
  for (const name of ['Copy', 'Save', 'Dismiss', 'Close screen snip']) {
    const button = view.getByRole('button', { name, exact: true }) as HTMLButtonElement;
    await waitFor(() => expect(button.disabled).toBe(true)); await fireEvent.click(button);
  }
  expect(calls('save_snip')).toHaveLength(1); expect(calls('copy_snip')).toHaveLength(0); expect(calls('dismiss_snip')).toHaveLength(0);
  pending.resolve({ ...token(), status: 'cancelled' });
  await waitFor(() => expect((view.getByRole('button', { name: 'Copy', exact: true }) as HTMLButtonElement).disabled).toBe(false));
});

it('preview exposes decorative SVG action icons below the image and a separate header close control', async () => {
  const view = await mountPreview();
  const image = view.getByRole('img', { name: 'Snip preview' });
  const actions = ['Copy', 'Save', 'Dismiss'].map(name => view.getByRole('button', { name, exact: true }));
  for (const button of actions) {
    expect(button.getAttribute('aria-label')).toBeTruthy();
    expect(button.querySelector('svg[aria-hidden="true"]')).not.toBeNull();
    expect(image.compareDocumentPosition(button) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  }
  const close = view.getByRole('button', { name: 'Close screen snip', exact: true });
  expect(close.closest('header')).not.toBeNull();
  expect(close.querySelector('svg[aria-hidden="true"]')).not.toBeNull();
  expect(actions).not.toContain(close);
  expect(calls('copy_snip')).toHaveLength(0);
});

it.each(['Dismiss', 'Close screen snip'])('%s dismisses the exact preview token once and releases its image', async name => {
  const view = await mountPreview();
  const url = (view.getByRole('img', { name: 'Snip preview' }) as HTMLImageElement).src;
  await fireEvent.click(view.getByRole('button', { name, exact: true }));
  await waitFor(() => expect(calls('dismiss_snip')).toHaveLength(1));
  expect(calls('dismiss_snip')[0][1]).toEqual(token());
  await waitFor(() => expect(view.queryByRole('img', { name: 'Snip preview' })).toBeNull());
  expect(URL.revokeObjectURL).toHaveBeenCalledWith(url);
  await fireEvent.click(view.getByRole('button', { name, exact: true }));
  expect(calls('dismiss_snip')).toHaveLength(1);
  expect(calls('copy_snip')).toHaveLength(0);
});

it('preview loading disables Copy/Save but permits token-bound header Close without clipboard publication', async () => {
  native.label = 'snip-preview-7'; context = { ...preview };
  const pending = deferred<Uint8Array>(); imageResponse = () => pending.promise;
  const view = render(await component('SnipPreviewSurface'));
  await waitFor(() => expect(calls('get_snip_image')).toHaveLength(1));
  for (const name of ['Copy', 'Save']) {
    const button = view.getByRole('button', { name, exact: true }) as HTMLButtonElement;
    expect(button.disabled).toBe(true); await fireEvent.click(button);
  }
  const close = view.getByRole('button', { name: 'Close screen snip', exact: true }) as HTMLButtonElement;
  expect(close.disabled).toBe(false); await fireEvent.click(close);
  await waitFor(() => expect(calls('dismiss_snip')).toHaveLength(1));
  pending.resolve(png); await Promise.resolve(); await Promise.resolve();
  expect(calls('copy_snip')).toHaveLength(0); expect(calls('save_snip')).toHaveLength(0);
  expect(view.queryByRole('img', { name: 'Snip preview' })).toBeNull();
});

it.each(['copy_snip', 'dismiss_snip'])('pending %s disables all action controls including header Close', async command => {
  const view = await mountPreview(); const pending = deferred<unknown>();
  const invoke = tauriMocks().invoke.getMockImplementation()!;
  tauriMocks().invoke.mockImplementation((name, args) => name === command ? pending.promise : invoke(name, args));
  await fireEvent.click(view.getByRole('button', { name: command === 'copy_snip' ? 'Copy' : 'Close screen snip', exact: true }));
  for (const name of ['Copy', 'Save', 'Dismiss', 'Close screen snip']) {
    const button = view.getByRole('button', { name, exact: true }) as HTMLButtonElement;
    await waitFor(() => expect(button.disabled).toBe(true)); await fireEvent.click(button);
  }
  expect(calls(command)).toHaveLength(1); expect(calls('save_snip')).toHaveLength(0);
  expect(calls(command === 'copy_snip' ? 'dismiss_snip' : 'copy_snip')).toHaveLength(0);
  pending.resolve({ ...token(), status: 'committed', committed: true, durable: true, closed: true });
  await waitFor(() => expect(view.getByRole('region', { name: 'Screen snip preview' }).getAttribute('aria-busy')).toBe('false'));
});

it('stale header Close settlement cannot dismiss a replacement preview or revoke its image', async () => {
  const view = await mountPreview(); const pending = deferred<unknown>();
  const oldToken = token(); const invoke = tauriMocks().invoke.getMockImplementation()!;
  tauriMocks().invoke.mockImplementation((name, args) => name === 'dismiss_snip' ? pending.promise : invoke(name, args));
  await fireEvent.click(view.getByRole('button', { name: 'Close screen snip', exact: true }));
  context = { ...preview, generation: '8', captureId: '88888888888888888888888888888888' };
  handlers.get('snip:context')!({ payload: context });
  await waitFor(() => expect(calls('get_snip_image')).toHaveLength(2));
  await waitFor(() => expect(vi.mocked(URL.createObjectURL).mock.calls).toHaveLength(2));
  const replacementUrl = vi.mocked(URL.createObjectURL).mock.results[1].value;
  pending.resolve({ ...oldToken, closed: true });
  await waitFor(() => expect((view.getByRole('button', { name: 'Copy', exact: true }) as HTMLButtonElement).disabled).toBe(false));
  expect((view.getByRole('img', { name: 'Snip preview' }) as HTMLImageElement).src).toBe(replacementUrl);
  expect(URL.revokeObjectURL).not.toHaveBeenCalledWith(replacementUrl);
  expect(calls('dismiss_snip')[0][1]).toEqual(oldToken);
});

it('new context ignores stale pending Copy success and releases previous image blob', async () => {
  const pending = deferred<unknown>(); const view = await mountPreview();
  const invoke = tauriMocks().invoke.getMockImplementation()!;
  tauriMocks().invoke.mockImplementation((command, args) => command === 'copy_snip' ? pending.promise : invoke(command, args));
  await fireEvent.click(view.getByRole('button', { name: 'Copy', exact: true }));
  await waitFor(() => expect(handlers.has('snip:context')).toBe(true));
  context = { ...preview, generation: '8', captureId: '88888888888888888888888888888888' };
  handlers.get('snip:context')!({ payload: context });
  pending.resolve({ generation: selecting.generation, captureId: selecting.captureId, status: 'committed', committed: true, durable: true });
  await waitFor(() => expect(calls('get_snip_image').length).toBeGreaterThan(1));
  expect(view.container.textContent).not.toMatch(/copied/i);
  expect(URL.revokeObjectURL).toHaveBeenCalled(); expect(calls('dismiss_snip')).toHaveLength(0);
});

it('TopBar snipping launcher renders the supplied crop SVG with theme-inherited color', async () => {
  native.label = 'top-bar'; const view = render(await component('TopBar'));
  const button = await view.findByRole('button', { name: 'Capture screen region' });
  const icon = button.querySelector('svg');
  expect(icon).not.toBeNull();
  expect(icon!.getAttribute('viewBox')).toBe('0 -960 960 960');
  expect(icon!.getAttribute('fill')).toBe('currentColor');
  expect(icon!.getAttribute('aria-hidden')).toBe('true');
  expect(icon!.getAttribute('focusable')).toBe('false');
  // Independent fixture: the user-supplied crop_24dp ... wght300 SVG path,
  // not the shared registry under test (which would allow a wrong asset to pass).
  expect(icon!.querySelector('path')?.getAttribute('d')).toBe(
    'M690-50v-160H282.31Q252-210 231-231q-21-21-21-51.31V-690H50v-60h160v-160h60v627.69q0 4.62 3.85 8.46 3.84 3.85 8.46 3.85H910v60H750v160h-60Zm0-280v-347.69q0-4.62-3.85-8.46-3.84-3.85-8.46-3.85H330v-60h347.69Q708-750 729-729q21 21 21 51.31V-330h-60Z'
  );
});

it('TopBar snipping launcher starts capture once through pointer click with no clipboard side effect', async () => {
  native.label = 'top-bar'; const view = render(await component('TopBar'));
  const button = await view.findByRole('button', { name: 'Capture screen region' });
  await fireEvent.click(button, { detail: 1 });
  await waitFor(() => expect(calls('start_snip')).toHaveLength(1));
  expect(calls('copy_snip')).toHaveLength(0);
});

it('TopBar renders snipping control after sound and a 4px reorder never starts capture', async () => {
  native.label = 'top-bar'; const view = render(await component('TopBar'));
  const button = await view.findByRole('button', { name: 'Capture screen region' });
  const ids = Array.from(view.container.querySelectorAll('[data-top-bar-control]')).map((item) => item.getAttribute('data-top-bar-control'));
  expect(ids).toEqual(['terminal', 'command', 'tray', 'mic', 'sound', 'snip']);
  await pointer(button, 'pointerdown', 100, 10); await pointer(button, 'pointermove', 104, 10);
  await pointer(button, 'pointerup', 104, 10); await fireEvent.click(button, { detail: 1 });
  expect(calls('start_snip')).toHaveLength(0);
  await fireEvent.click(button, { detail: 0 }); await waitFor(() => expect(calls('start_snip')).toHaveLength(1));
});

it('Settings exposes Alt+S and reports legacy duplicate conflict without silently changing prior chords', async () => {
  native.label = 'settings-panel'; const view = render(await component('SettingsPanelSurface'));
  const snip = await view.findByRole('button', { name: 'Capture Screen snipping shortcut' });
  expect(snip.textContent).toContain('Alt+S');
  const settings = defaultShellSettings(); settings.hotkeys.terminal = 'Alt+S';
  const invoke = tauriMocks().invoke.getMockImplementation()!;
  tauriMocks().invoke.mockImplementation((command, args) => command === 'load_shell_settings' ? Promise.resolve(settings) : invoke(command, args));
  view.unmount(); const legacy = render(await component('SettingsPanelSurface'));
  await waitFor(() => expect(legacy.container.textContent).toMatch(/conflict|already assigned|duplicate/i));
  expect(legacy.getByRole('alert').textContent).toMatch(/Alt\+S is already assigned to Terminal\. Change either shortcut to enable Screen snipping\./);
  expect((await legacy.findByRole('button', { name: 'Capture Terminal shortcut' })).textContent).toContain('Alt+S');
  expect(calls('save_shell_settings')).toHaveLength(0);
});

it('decoded overlay cannot select before all-monitor armed; stale armed tokens do not enable it', async () => {
  autoArm = false;
  const { region } = await overlay();
  await pointer(region, 'pointerdown', 10, 20); await pointer(region, 'pointerup', 110, 80);
  expect(calls('snip_begin_selection')).toHaveLength(0);
  handlers.get('snip:armed')!({ payload: { generation: '6', captureId: selecting.captureId } });
  await pointer(region, 'pointerdown', 10, 20); expect(calls('snip_begin_selection')).toHaveLength(0);
  handlers.get('snip:armed')!({ payload: token() });
  await selectReverse(region);
  expect(calls('complete_snip')).toHaveLength(1);
});

it('subscribes before buffered context fetch and accepts already-armed acknowledgement without event replay', async () => {
  const invoke = tauriMocks().invoke.getMockImplementation()!;
  tauriMocks().invoke.mockImplementation((command, args) => {
    if (command === 'get_snip_context') {
      expect(handlers.has('snip:context')).toBe(true); expect(handlers.has('snip:armed')).toBe(true);
    }
    if (command === 'snip_ready') return Promise.resolve({ ...token(), ready: true });
    return invoke(command, args);
  });
  const { region } = await overlay(); await selectReverse(region);
});

it.each(['image', 'ready'])('native %s failure cancels without readiness/selection publication and releases blob', async (failure) => {
  const invoke = tauriMocks().invoke.getMockImplementation()!;
  tauriMocks().invoke.mockImplementation((command, args) => command === (failure === 'image' ? 'get_snip_image' : 'snip_ready')
    ? Promise.reject({ code: failure === 'image' ? 'image-failed' : 'window-failed' }) : invoke(command, args));
  const view = render(await component('SnipOverlaySurface'));
  await waitFor(() => expect(calls('cancel_snip')).toHaveLength(1));
  expect(calls('snip_begin_selection')).toHaveLength(0); expect(calls('complete_snip')).toHaveLength(0);
  view.unmount();
  expect(vi.mocked(URL.revokeObjectURL).mock.calls.length).toBe(vi.mocked(URL.createObjectURL).mock.calls.length);
});

it('TopBar keyboard activation preserves migrated user control order in rendered DOM', async () => {
  const old = ['sound', 'tray', 'terminal', 'mic', 'command'];
  localStorage.setItem('jasonshell:top-bar:control-order:v1', JSON.stringify(old));
  native.label = 'top-bar'; const view = render(await component('TopBar'));
  const button = await view.findByRole('button', { name: 'Capture screen region' });
  expect(Array.from(view.container.querySelectorAll('[data-top-bar-control]')).map(e => e.getAttribute('data-top-bar-control')))
    .toEqual([...old, 'snip']);
  await fireEvent.keyDown(button, { key: 'Enter' }); await fireEvent.keyUp(button, { key: 'Enter' });
  // jsdom does not generate the browser's keyboard click; deliver its detail=0 event.
  await fireEvent.click(button, { detail: 0 });
  await waitFor(() => expect(calls('start_snip')).toHaveLength(1));
});

it('pending Save plus concurrent TopBar start returns busy; cancellation retains original preview without stale success', async () => {
  const pending = deferred<unknown>(); const view = await mountPreview();
  const invoke = tauriMocks().invoke.getMockImplementation()!;
  tauriMocks().invoke.mockImplementation((command, args) => {
    if (command === 'save_snip') return pending.promise;
    if (command === 'start_snip') return Promise.reject({ code: 'busy' });
    return invoke(command, args);
  });
  await fireEvent.click(view.getByRole('button', { name: 'Save', exact: true }));
  native.label = 'top-bar'; const bar = render(await component('TopBar'));
  await fireEvent.click(await bar.findByRole('button', { name: 'Capture screen region' }));
  await waitFor(() => expect(calls('start_snip')).toHaveLength(1));
  expect(calls('complete_snip')).toHaveLength(0); expect(calls('dismiss_snip')).toHaveLength(0);
  pending.resolve({ ...token(), status: 'cancelled' });
  await waitFor(() => expect((view.getByRole('button', { name: 'Copy', exact: true }) as HTMLButtonElement).disabled).toBe(false));
  expect(view.getByRole('img', { name: 'Snip preview' })).toBeTruthy();
  expect(view.container.textContent).not.toMatch(/saved|copied/i);
});
