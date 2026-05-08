import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export type ShellBarResizeEdge = 'top' | 'bottom';

export interface ResizeShellBarRequest {
  edge: ShellBarResizeEdge;
  heightLogical: number;
}

export interface ResizeShellBarResponse {
  edge: ShellBarResizeEdge;
  heightLogical: number;
}

export const MIN_TOP_BAR_HEIGHT_LOGICAL = 18;
export const MIN_BOTTOM_BAR_HEIGHT_LOGICAL = 24;
export const MAX_SHELL_BAR_HEIGHT_LOGICAL = 120;

export function clampShellBarHeight(edge: ShellBarResizeEdge, heightLogical: number): number {
  const minimum = edge === 'top' ? MIN_TOP_BAR_HEIGHT_LOGICAL : MIN_BOTTOM_BAR_HEIGHT_LOGICAL;
  if (!Number.isFinite(heightLogical)) {
    return minimum;
  }
  return Math.min(MAX_SHELL_BAR_HEIGHT_LOGICAL, Math.max(minimum, heightLogical));
}

export function shellBarHeightFromDrag(
  edge: ShellBarResizeEdge,
  startHeightLogical: number,
  startClientY: number,
  currentClientY: number
): number {
  const delta = edge === 'top'
    ? currentClientY - startClientY
    : startClientY - currentClientY;
  return clampShellBarHeight(edge, startHeightLogical + delta);
}

export function resizeShellBar(request: ResizeShellBarRequest): Promise<ResizeShellBarResponse> {
  return invoke<ResizeShellBarResponse>(IPC_COMMANDS.resizeShellBar, {
    request: {
      edge: request.edge,
      heightLogical: clampShellBarHeight(request.edge, request.heightLogical)
    }
  });
}

export interface ShellBarResizeScheduler {
  schedule(heightLogical: number): void;
  flush(heightLogical?: number): Promise<void>;
}

export function createShellBarResizeScheduler(
  edge: ShellBarResizeEdge,
  onError: (error: unknown) => void = console.error
): ShellBarResizeScheduler {
  let frameId: number | null = null;
  let inFlight = false;
  let queuedHeight: number | null = null;
  let idleResolvers: Array<() => void> = [];

  const requestFrame = (callback: () => void): number => {
    if (typeof globalThis.requestAnimationFrame === 'function') {
      return globalThis.requestAnimationFrame(callback);
    }
    return globalThis.setTimeout(callback, 16) as unknown as number;
  };

  const cancelFrame = (id: number) => {
    if (typeof globalThis.cancelAnimationFrame === 'function') {
      globalThis.cancelAnimationFrame(id);
      return;
    }
    globalThis.clearTimeout(id);
  };

  const resolveIdleIfReady = () => {
    if (inFlight || queuedHeight !== null || frameId !== null) {
      return;
    }
    const resolvers = idleResolvers;
    idleResolvers = [];
    for (const resolve of resolvers) {
      resolve();
    }
  };

  const dispatchLatest = () => {
    if (inFlight || queuedHeight === null) {
      resolveIdleIfReady();
      return;
    }
    const heightLogical = queuedHeight;
    queuedHeight = null;
    inFlight = true;
    void resizeShellBar({ edge, heightLogical })
      .catch(onError)
      .finally(() => {
        inFlight = false;
        dispatchLatest();
      });
  };

  const scheduleFrame = () => {
    if (frameId !== null) {
      return;
    }
    frameId = requestFrame(() => {
      frameId = null;
      dispatchLatest();
    });
  };

  return {
    schedule(heightLogical: number) {
      queuedHeight = clampShellBarHeight(edge, heightLogical);
      scheduleFrame();
    },
    flush(heightLogical?: number) {
      if (heightLogical !== undefined) {
        queuedHeight = clampShellBarHeight(edge, heightLogical);
      }
      if (frameId !== null) {
        cancelFrame(frameId);
        frameId = null;
      }
      dispatchLatest();
      if (!inFlight && queuedHeight === null) {
        return Promise.resolve();
      }
      return new Promise((resolve) => {
        idleResolvers.push(resolve);
      });
    }
  };
}
