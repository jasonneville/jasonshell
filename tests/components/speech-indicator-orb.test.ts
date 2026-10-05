import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createSpeechIndicatorOrb } from '../../src/lib/speechIndicatorOrb';

// A deterministic WebGL boundary fixture. This tests controller decisions and
// resource ownership, not shader appearance or real-driver compatibility.
function gpu() {
  const shaders: object[] = [];
  const program = { resource: 'program' };
  const buffer = { resource: 'buffer' };
  const gl = {
    VERTEX_SHADER: 35633, FRAGMENT_SHADER: 35632, COMPILE_STATUS: 35713,
    LINK_STATUS: 35714, ARRAY_BUFFER: 34962, STATIC_DRAW: 35044,
    FLOAT: 5126, TRIANGLES: 4, NO_ERROR: 0,
    createShader: vi.fn(() => { const shader = { resource: 'shader', index: shaders.length }; shaders.push(shader); return shader; }),
    shaderSource: vi.fn(), compileShader: vi.fn(), getShaderParameter: vi.fn(() => true),
    createProgram: vi.fn(() => program), attachShader: vi.fn(), linkProgram: vi.fn(),
    getProgramParameter: vi.fn(() => true), useProgram: vi.fn(),
    createBuffer: vi.fn(() => buffer), bindBuffer: vi.fn(), bufferData: vi.fn(),
    getAttribLocation: vi.fn(() => 0), enableVertexAttribArray: vi.fn(), vertexAttribPointer: vi.fn(),
    getUniformLocation: vi.fn((_program: unknown, name: string) => ({ name })),
    uniform1f: vi.fn(), uniform2f: vi.fn(), uniform3f: vi.fn(), viewport: vi.fn(),
    drawArrays: vi.fn(), isContextLost: vi.fn(() => false), getError: vi.fn(() => 0),
    deleteShader: vi.fn(), deleteProgram: vi.fn(), deleteBuffer: vi.fn()
  };
  return { gl, shaders, program, buffer };
}

function preference(query: string) {
  const target = new EventTarget();
  return Object.assign(target, {
    media: query, matches: false,
    addEventListener: vi.fn(target.addEventListener.bind(target)),
    removeEventListener: vi.fn(target.removeEventListener.bind(target)),
    change(matches: boolean) { this.matches = matches; target.dispatchEvent(new Event('change')); }
  });
}

let canvas: HTMLCanvasElement;
let device: ReturnType<typeof gpu>;
let reduce: ReturnType<typeof preference>;
let forced: ReturnType<typeof preference>;
let hidden: boolean;
let pending: Map<number, FrameRequestCallback>;
let request: ReturnType<typeof vi.fn>;
let cancel: ReturnType<typeof vi.fn>;
const controllers: ReturnType<typeof createSpeechIndicatorOrb>[] = [];

function mount() {
  const controller = createSpeechIndicatorOrb(canvas);
  controllers.push(controller);
  return controller;
}
function frame(now: number) {
  expect(pending.size).toBe(1);
  const [id, callback] = [...pending][0];
  pending.delete(id);
  callback(now);
}
function visibility(value: boolean) {
  hidden = value;
  document.dispatchEvent(new Event('visibilitychange'));
}
function expectReleased() {
  expect(device.gl.deleteProgram).toHaveBeenCalledWith(device.program);
  expect(device.gl.deleteBuffer).toHaveBeenCalledWith(device.buffer);
  for (const shader of device.shaders) expect(device.gl.deleteShader).toHaveBeenCalledWith(shader);
  expect(pending.size).toBe(0);
}

beforeEach(() => {
  document.documentElement.style.setProperty('--js-color-accent', '#a6e22e');
  document.documentElement.dataset.theme = 'monokai';
  canvas = document.createElement('canvas');
  device = gpu();
  vi.spyOn(canvas, 'getContext').mockReturnValue(device.gl as unknown as WebGLRenderingContext);
  reduce = preference('(prefers-reduced-motion: reduce)');
  forced = preference('(forced-colors: active)');
  vi.stubGlobal('matchMedia', vi.fn((query: string) => query === reduce.media ? reduce : forced));
  hidden = false;
  vi.spyOn(document, 'hidden', 'get').mockImplementation(() => hidden);
  vi.stubGlobal('devicePixelRatio', 1);
  pending = new Map();
  let next = 1;
  request = vi.fn((callback: FrameRequestCallback) => { const id = next++; pending.set(id, callback); return id; });
  cancel = vi.fn((id: number) => { pending.delete(id); });
  vi.stubGlobal('requestAnimationFrame', request);
  vi.stubGlobal('cancelAnimationFrame', cancel);
});
afterEach(() => {
  controllers.splice(0).forEach((controller) => controller.dispose());
  document.documentElement.style.removeProperty('--js-color-accent');
  delete document.documentElement.dataset.theme;
  vi.unstubAllGlobals();
});

async function theme(token: string, id = 'dracula') {
  document.documentElement.style.setProperty('--js-color-accent', token);
  document.documentElement.dataset.theme = id;
  // Deliver real MutationObserver callbacks without advancing RAF or mic input.
  for (let turn = 0; turn < 5; turn += 1) await Promise.resolve();
}
function expectAccent(gl: ReturnType<typeof gpu>['gl'], rgb: number[]) {
  const calls = gl.uniform3f.mock.calls.filter(([location]) => location?.name === 'u_color');
  expect(calls.length).toBeGreaterThan(0);
  const [, ...actual] = calls.at(-1)!;
  rgb.forEach((channel, index) => expect(actual[index]).toBeCloseTo(channel / 255, 8));
}

describe('existing shell theme accent drives the orb', () => {
  it.each([
    ['#a6e22e', [166, 226, 46]], ['#04d9c4', [4, 217, 196]],
    ['#bd93f9', [189, 147, 249]], ['rgb(38, 139, 210)', [38, 139, 210]]
  ])('reads initial resolved accent %s before its first draw', (token, rgb) => {
    document.documentElement.style.setProperty('--js-color-accent', token as string);
    mount();
    expectAccent(device.gl, rgb as number[]);
    expect(device.gl.uniform3f.mock.invocationCallOrder[0]).toBeLessThan(device.gl.drawArrays.mock.invocationCallOrder[0]);
  });

  it.each(['recording', 'paused', 'reduced motion'] as const)('live theme attribute updates %s color without starting duplicate animation chains', async (mode) => {
    if (mode === 'reduced motion') reduce.matches = true;
    const controller = mount();
    if (mode !== 'paused') controller.setActive(true);
    const initializations = device.gl.createProgram.mock.calls.length;
    const draws = device.gl.drawArrays.mock.calls.length;
    await theme('#bd93f9');
    expectAccent(device.gl, [189, 147, 249]);
    expect(device.gl.drawArrays.mock.calls.length).toBeGreaterThan(draws);
    expect(device.gl.createProgram).toHaveBeenCalledTimes(initializations);
    expect(pending.size).toBe(mode === 'recording' ? 1 : 0);
    if (mode === 'reduced motion') expect(device.gl.uniform1f).toHaveBeenLastCalledWith(expect.anything(), 0);
    await theme('#04d9c4', 'andromeda');
    expectAccent(device.gl, [4, 217, 196]);
    expect(pending.size).toBe(mode === 'recording' ? 1 : 0);
  });

  it('theme changes while hidden/forced colors are adopted on resume without forbidden draws', async () => {
    const controller = mount();
    controller.setActive(true);
    visibility(true);
    const draws = device.gl.drawArrays.mock.calls.length;
    await theme('#bd93f9');
    expect(device.gl.drawArrays).toHaveBeenCalledTimes(draws);
    expect(pending.size).toBe(0);
    visibility(false);
    expectAccent(device.gl, [189, 147, 249]);
    forced.change(true);
    const forcedDraws = device.gl.drawArrays.mock.calls.length;
    await theme('#04d9c4', 'andromeda');
    expect(device.gl.drawArrays).toHaveBeenCalledTimes(forcedDraws);
    forced.change(false);
    expectAccent(device.gl, [4, 217, 196]);
    expect(pending.size).toBe(1);
  });

  it('empty or invalid accent safely uses one deterministic finite RGB fallback', async () => {
    document.documentElement.style.removeProperty('--js-color-accent');
    const controller = mount();
    controller.setActive(true);
    const fallback = device.gl.uniform3f.mock.calls.at(-1)!.slice(1) as number[];
    expect(fallback).toHaveLength(3);
    expect(fallback.every((value) => Number.isFinite(value) && value >= 0 && value <= 1)).toBe(true);
    for (const token of ['', 'not-a-color', 'var(--missing-accent)', '#gggggg']) {
      await theme(token, `invalid-${token}`);
      expect(device.gl.uniform3f.mock.calls.at(-1)!.slice(1)).toEqual(fallback);
      expect(canvas.style.visibility).toBe('visible');
      expect(pending.size).toBe(1);
    }
  });

  it('theme observer disconnects on dispose and subsequent theme changes perform no GPU I/O', async () => {
    const disconnect = vi.spyOn(MutationObserver.prototype, 'disconnect');
    const controller = mount();
    controller.setActive(true);
    controller.dispose();
    expect(disconnect).toHaveBeenCalled();
    const colors = device.gl.uniform3f.mock.calls.length;
    const draws = device.gl.drawArrays.mock.calls.length;
    await theme('#bd93f9');
    expect(device.gl.uniform3f).toHaveBeenCalledTimes(colors);
    expect(device.gl.drawArrays).toHaveBeenCalledTimes(draws);
    expectReleased();
  });

  it('latest theme survives context loss and is used by the restored renderer', async () => {
    const controller = mount();
    controller.setActive(true);
    canvas.dispatchEvent(new Event('webglcontextlost', { cancelable: true }));
    await theme('#bd93f9');
    const replacement = gpu();
    vi.mocked(canvas.getContext).mockReturnValue(replacement.gl as unknown as WebGLRenderingContext);
    canvas.dispatchEvent(new Event('webglcontextrestored'));
    expectAccent(replacement.gl, [189, 147, 249]);
    expect(pending.size).toBe(1);
  });
});

describe('healthy GPU orb lifecycle', () => {
  it('initializes opaque WebGL with one static frame; recording owns a single animation chain', () => {
    const controller = mount();
    expect(canvas.getContext).toHaveBeenCalledWith('webgl', { alpha: false, antialias: false });
    expect(canvas.style.visibility).toBe('visible');
    expect(device.gl.drawArrays).toHaveBeenCalledOnce();
    expect(request).not.toHaveBeenCalled();
    controller.setActive(true);
    expect(pending.size).toBe(1);
    frame(1000);
    frame(1050);
    expect(device.gl.uniform1f).toHaveBeenLastCalledWith(expect.anything(), 0.05);
    frame(5050);
    expect(device.gl.uniform1f).toHaveBeenLastCalledWith(expect.anything(), 0.15);
    controller.setActive(true);
    expect(pending.size).toBe(1);
    controller.setActive(false);
    expect(pending.size).toBe(0);
    expect(cancel).toHaveBeenCalled();
  });

  it('hidden documents cancel RAF, skip drawing, and resume only while recording', () => {
    const controller = mount();
    controller.setActive(true);
    const draws = device.gl.drawArrays.mock.calls.length;
    visibility(true);
    expect(pending.size).toBe(0);
    expect(device.gl.drawArrays).toHaveBeenCalledTimes(draws);
    visibility(false);
    expect(pending.size).toBe(1);
    visibility(true);
    controller.setActive(false);
    visibility(false);
    expect(pending.size).toBe(0);
  });

  it('reduced motion draws time zero without RAF, including live preference changes', () => {
    reduce.matches = true;
    const controller = mount();
    controller.setActive(true);
    expect(device.gl.drawArrays).toHaveBeenCalled();
    expect(device.gl.uniform1f.mock.calls.every(([, time]) => time === 0)).toBe(true);
    expect(request).not.toHaveBeenCalled();
    reduce.change(false);
    frame(1000);
    frame(1050);
    reduce.change(true);
    expect(pending.size).toBe(0);
    expect(device.gl.uniform1f).toHaveBeenLastCalledWith(expect.anything(), 0);
  });

  it('forced colors suppress draws and RAF; live changes restore recording decoration', () => {
    forced.matches = true;
    const controller = mount();
    controller.setActive(true);
    expect(device.gl.drawArrays).not.toHaveBeenCalled();
    expect(request).not.toHaveBeenCalled();
    forced.change(false);
    expect(pending.size).toBe(1);
    const draws = device.gl.drawArrays.mock.calls.length;
    forced.change(true);
    expect(pending.size).toBe(0);
    expect(device.gl.drawArrays).toHaveBeenCalledTimes(draws);
  });

  it.each([[1, 40], [1.5, 60], [2, 80], [4, 80], [0, 40]])('DPR %s produces bounded %spx backing resolution', (ratio, pixels) => {
    vi.stubGlobal('devicePixelRatio', ratio);
    mount();
    expect(canvas.width).toBe(pixels);
    expect(canvas.height).toBe(pixels);
    expect(device.gl.viewport).toHaveBeenCalledWith(0, 0, pixels, pixels);
    expect(device.gl.uniform2f).toHaveBeenCalledWith(expect.anything(), pixels, pixels);
  });

  it('context loss exposes fallback, releases resources; restoration reinitializes and resumes', () => {
    const controller = mount();
    controller.setActive(true);
    const loss = new Event('webglcontextlost', { cancelable: true });
    canvas.dispatchEvent(loss);
    expect(loss.defaultPrevented).toBe(true);
    expect(canvas.style.visibility).toBe('hidden');
    expectReleased();
    const replacement = gpu();
    vi.mocked(canvas.getContext).mockReturnValue(replacement.gl as unknown as WebGLRenderingContext);
    canvas.dispatchEvent(new Event('webglcontextrestored'));
    expect(canvas.style.visibility).toBe('visible');
    expect(replacement.gl.drawArrays).toHaveBeenCalledOnce();
    expect(pending.size).toBe(1);
    controller.dispose();
    expect(replacement.gl.deleteProgram).toHaveBeenCalledWith(replacement.program);
    expect(replacement.gl.deleteBuffer).toHaveBeenCalledWith(replacement.buffer);
    expect(replacement.gl.deleteShader).toHaveBeenCalledTimes(2);
  });

  it('restoration during reduced motion or stopped recording does not start RAF', () => {
    const controller = mount();
    controller.setActive(true);
    canvas.dispatchEvent(new Event('webglcontextlost', { cancelable: true }));
    reduce.change(true);
    canvas.dispatchEvent(new Event('webglcontextrestored'));
    expect(pending.size).toBe(0);
    expect(device.gl.uniform1f).toHaveBeenLastCalledWith(expect.anything(), 0);
    controller.setActive(false);
    reduce.change(false);
    expect(pending.size).toBe(0);
  });

  it('dispose cancels RAF, deletes every GL object once, and detaches all listeners', () => {
    const canvasRemove = vi.spyOn(canvas, 'removeEventListener');
    const documentRemove = vi.spyOn(document, 'removeEventListener');
    const controller = mount();
    controller.setActive(true);
    controller.dispose();
    expectReleased();
    expect(device.gl.deleteShader).toHaveBeenCalledTimes(2);
    expect(device.gl.deleteProgram).toHaveBeenCalledOnce();
    expect(device.gl.deleteBuffer).toHaveBeenCalledOnce();
    expect(canvasRemove).toHaveBeenCalledWith('webglcontextlost', expect.any(Function));
    expect(canvasRemove).toHaveBeenCalledWith('webglcontextrestored', expect.any(Function));
    expect(documentRemove).toHaveBeenCalledWith('visibilitychange', expect.any(Function));
    expect(reduce.removeEventListener).toHaveBeenCalledWith('change', expect.any(Function));
    expect(forced.removeEventListener).toHaveBeenCalledWith('change', expect.any(Function));
    const draws = device.gl.drawArrays.mock.calls.length;
    const initializations = device.gl.createProgram.mock.calls.length;
    controller.dispose();
    controller.setActive(true);
    visibility(false);
    reduce.change(false);
    forced.change(false);
    canvas.dispatchEvent(new Event('webglcontextrestored'));
    expect(device.gl.drawArrays).toHaveBeenCalledTimes(draws);
    expect(device.gl.createProgram).toHaveBeenCalledTimes(initializations);
    expect(device.gl.deleteShader).toHaveBeenCalledTimes(2);
    expect(pending.size).toBe(0);
  });
});

describe('GPU failure falls back without crashing', () => {
  it.each(['unavailable', 'context throws', 'shader unavailable', 'compilation', 'program unavailable', 'linking', 'buffer unavailable', 'attribute unavailable'] as const)('%s initialization fails closed and cleans allocated objects', (failure) => {
    const gl = device.gl;
    if (failure === 'unavailable') vi.mocked(canvas.getContext).mockReturnValue(null);
    if (failure === 'context throws') vi.mocked(canvas.getContext).mockImplementation(() => { throw new Error('GPU disabled'); });
    if (failure === 'shader unavailable') gl.createShader.mockReturnValue(null as never);
    if (failure === 'compilation') gl.getShaderParameter.mockReturnValue(false);
    if (failure === 'program unavailable') gl.createProgram.mockReturnValue(null as never);
    if (failure === 'linking') gl.getProgramParameter.mockReturnValue(false);
    if (failure === 'buffer unavailable') gl.createBuffer.mockReturnValue(null as never);
    if (failure === 'attribute unavailable') gl.getAttribLocation.mockReturnValue(-1);
    let controller!: ReturnType<typeof createSpeechIndicatorOrb>;
    expect(() => { controller = mount(); controller.setActive(true); }).not.toThrow();
    expect(canvas.style.visibility).toBe('hidden');
    expect(pending.size).toBe(0);
    expect(gl.drawArrays).not.toHaveBeenCalled();
    if (!['unavailable', 'context throws', 'program unavailable'].includes(failure)) {
      expect(gl.deleteProgram).toHaveBeenCalledWith(device.program);
    }
    for (const shader of device.shaders) expect(gl.deleteShader).toHaveBeenCalledWith(shader);
    if (failure === 'attribute unavailable') expect(gl.deleteBuffer).toHaveBeenCalledWith(device.buffer);
    expect(() => controller.dispose()).not.toThrow();
  });

  it.each(['draw throws', 'GL error', 'context lost'] as const)('%s while animating hides canvas, cancels loop and releases GL objects', (failure) => {
    const controller = mount();
    controller.setActive(true);
    if (failure === 'draw throws') device.gl.drawArrays.mockImplementation(() => { throw new Error('Driver failed'); });
    if (failure === 'GL error') device.gl.getError.mockReturnValue(1282);
    if (failure === 'context lost') device.gl.isContextLost.mockReturnValue(true);
    expect(() => frame(1000)).not.toThrow();
    expect(canvas.style.visibility).toBe('hidden');
    expectReleased();
    const draws = device.gl.drawArrays.mock.calls.length;
    visibility(false);
    controller.setActive(true);
    expect(device.gl.drawArrays).toHaveBeenCalledTimes(draws);
    expect(pending.size).toBe(0);
  });
});
