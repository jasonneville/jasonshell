import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { IPC_COMMANDS, IPC_EVENTS } from '../ipc';
import type { Rect } from './snipSelection';

export interface SnipToken { generation: string; captureId: string }
export interface SnipContext extends SnipToken {
  phase: 'selecting' | 'preview'; monitorId: string; width: number; height: number;
  scaleFactor?: number; originX?: number; originY?: number;
}
export interface SnipAcknowledgement extends SnipToken { ready?: boolean; accepted?: boolean }
export interface CopyOutcome extends SnipToken {
  status: 'committed' | 'committed-warning' | 'rejected' | 'publication-unknown';
  committed: boolean | null; durable: boolean | null; code?: string;
}
export interface SaveOutcome extends SnipToken { status: 'saved' | 'cancelled' | 'error'; code?: string }
export const tokenArgs = ({ generation, captureId }: SnipToken) => ({ generation, captureId });
export const imageArgs = (context: SnipContext) => ({ ...tokenArgs(context), monitorId: context.monitorId });
export const sameSnip = (a: SnipToken | null, b: SnipToken | null) => !!a && !!b
  && a.generation === b.generation && a.captureId === b.captureId;
export const startSnip = () => invoke<SnipToken>(IPC_COMMANDS.startSnip);
export const snipBarReady = (token: SnipToken) => invoke<SnipAcknowledgement>(IPC_COMMANDS.snipBarReady, tokenArgs(token));
export function isSnipToken(value: unknown): value is SnipToken {
  if (!value || typeof value !== 'object') return false;
  const token = value as SnipToken;
  return typeof token.generation === 'string' && /^[1-9][0-9]{0,19}$/.test(token.generation)
    && BigInt(token.generation) <= 18446744073709551615n
    && typeof token.captureId === 'string' && /^[\x21-\x7e]{1,64}$/.test(token.captureId);
}
export const readySnip = (context: SnipContext) => invoke<SnipAcknowledgement>(IPC_COMMANDS.snipReady, imageArgs(context));
export const beginSnip = (context: SnipContext) => invoke<SnipAcknowledgement>(IPC_COMMANDS.snipBeginSelection, imageArgs(context));
export const completeSnip = (context: SnipContext, rect: Rect) => invoke<SnipToken>(IPC_COMMANDS.completeSnip, { ...imageArgs(context), rect });
export const cancelSnip = (context: SnipToken) => invoke<SnipToken>(IPC_COMMANDS.cancelSnip, tokenArgs(context));
export const copySnip = (context: SnipToken) => invoke<CopyOutcome>(IPC_COMMANDS.copySnip, tokenArgs(context));
export const saveSnip = (context: SnipToken) => invoke<SaveOutcome>(IPC_COMMANDS.saveSnip, tokenArgs(context));
export const dismissSnip = (context: SnipToken) => invoke<SnipToken>(IPC_COMMANDS.dismissSnip, tokenArgs(context));

function validContext(value: SnipContext): boolean {
  return !!value && typeof value.generation === 'string' && /^[1-9][0-9]{0,19}$/.test(value.generation)
    && BigInt(value.generation) <= 18446744073709551615n
    && typeof value.captureId === 'string' && /^[\x21-\x7e]{1,64}$/.test(value.captureId)
    && typeof value.monitorId === 'string' && /^[\x21-\x7e]{1,64}$/.test(value.monitorId)
    && Number.isInteger(value.width) && value.width > 0 && value.width <= 16384
    && Number.isInteger(value.height) && value.height > 0 && value.height <= 16384;
}

/** Own-window metadata handshake. Subscribe first; native lookup buffers early startup context. */
export function observeSnipContext(phase: SnipContext['phase'], update: (context: SnipContext) => void,
  armed: (token: SnipToken) => void = () => undefined, failed: () => void = () => undefined): () => void {
  let disposed = false;
  let revision = 0;
  let latest: SnipContext | null = null;
  const unlisteners: Array<() => void> = [];
  const accept = (context: SnipContext) => {
    if (disposed || !validContext(context) || context.phase !== phase) return;
    if (latest && (BigInt(context.generation) < BigInt(latest.generation)
      || (context.generation === latest.generation && !sameSnip(context, latest)))) return;
    latest = context;
    update(context);
  };
  const registration = async () => {
    for (const [name, callback] of [
      [IPC_EVENTS.snipContext, (event: { payload: SnipContext }) => { revision += 1; accept(event.payload); }],
      [IPC_EVENTS.snipArmed, (event: { payload: SnipContext }) => { if (!disposed) armed(event.payload); }]
    ] as const) {
      const unlisten = await listen<SnipContext>(name, callback);
      if (disposed) { unlisten(); return; }
      unlisteners.push(unlisten);
    }
    const requestedRevision = revision;
    const context = await invoke<SnipContext>(IPC_COMMANDS.getSnipContext);
    if (revision === requestedRevision) accept(context);
  };
  void registration().catch(() => { if (!disposed) failed(); });
  return () => { disposed = true; for (const unlisten of unlisteners) unlisten(); };
}

export async function snipImageUrl(context: SnipContext, isCurrent: () => boolean = () => true): Promise<string> {
  const bytes = await invoke<Uint8Array | ArrayBuffer>(IPC_COMMANDS.getSnipImage, imageArgs(context));
  if (!isCurrent()) throw new Error('stale');
  if (!(bytes instanceof Uint8Array) && !(bytes instanceof ArrayBuffer)) throw new Error('image-failed');
  const buffer = bytes instanceof Uint8Array ? bytes.slice().buffer : bytes;
  if (!buffer.byteLength || buffer.byteLength > 512 * 1024 * 1024) throw new Error('image-failed');
  return URL.createObjectURL(new Blob([buffer], { type: 'image/png' }));
}

export function decodeSnipImage(image: HTMLImageElement): Promise<void> {
  if (typeof image.decode === 'function') return image.decode();
  if (image.complete && image.naturalWidth > 0) return Promise.resolve();
  return new Promise((resolve, reject) => {
    const cleanup = () => { image.removeEventListener('load', loaded); image.removeEventListener('error', error); };
    const loaded = () => { cleanup(); resolve(); };
    const error = () => { cleanup(); reject(new Error('image-failed')); };
    image.addEventListener('load', loaded, { once: true });
    image.addEventListener('error', error, { once: true });
  });
}
