import { afterEach, beforeEach, vi } from 'vitest';
import { cleanup } from '@testing-library/svelte';

const tauri = vi.hoisted(() => ({
  listen: vi.fn(),
  emitTo: vi.fn(),
  invoke: vi.fn()
}));
export function tauriMocks() { return tauri; }

vi.mock('@tauri-apps/api/event', () => ({ listen: tauri.listen, emitTo: tauri.emitTo, emit: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: tauri.invoke }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ listen: tauri.listen, label: 'test-window' }) }));

// jsdom has no layout observer or media-query evaluator. Surface tests that
// need changing geometry/preferences override these controlled inert defaults.
beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class {
    observe() {}
    unobserve() {}
    disconnect() {}
  });
  vi.stubGlobal('matchMedia', (media: string) => Object.assign(new EventTarget(), {
    matches: false, media, onchange: null,
    addListener() {}, removeListener() {},
    dispatchEvent: EventTarget.prototype.dispatchEvent
  }));
});

afterEach(() => {
  cleanup(); vi.useRealTimers(); vi.unstubAllGlobals();
  tauri.listen.mockReset(); tauri.emitTo.mockReset(); tauri.invoke.mockReset();
});
