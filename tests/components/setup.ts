import { afterEach, vi } from 'vitest';
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

afterEach(() => { cleanup(); vi.useRealTimers(); tauri.listen.mockReset(); tauri.emitTo.mockReset(); tauri.invoke.mockReset(); });
