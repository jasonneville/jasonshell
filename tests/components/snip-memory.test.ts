import { beforeEach, expect, it, vi } from 'vitest';
import { snipImageUrl, type SnipContext } from '../../src/lib/snipping';
import { tauriMocks } from './setup';

const context: SnipContext = { generation: '1', captureId: 'capture', monitorId: 'monitor', phase: 'preview', width: 1, height: 1 };
beforeEach(() => {
  vi.stubGlobal('URL', Object.assign(class extends URL {}, { createObjectURL: vi.fn(() => 'blob:test'), revokeObjectURL: vi.fn() }));
});
async function blobBytes(blob: Blob): Promise<number[]> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(Array.from(new Uint8Array(reader.result as ArrayBuffer)));
    reader.onerror = reject;
    reader.readAsArrayBuffer(blob);
  });
}

it.each(['full', 'subview', 'shared-subview'] as const)('%s preserves exact payload and removes explicit typed-array slice copy', async shape => {
  const backing = shape === 'shared-subview' ? new SharedArrayBuffer(10) : new ArrayBuffer(10);
  const all = new Uint8Array(backing); all.set([99, 98, 137, 80, 78, 71, 13, 10, 97, 96]);
  const bytes = shape === 'full' ? all : new Uint8Array(backing, 2, 6);
  const expected = Array.from(bytes);
  const slice = vi.spyOn(bytes, 'slice');
  tauriMocks().invoke.mockResolvedValue(bytes);
  expect(await snipImageUrl(context)).toBe('blob:test');
  const blob = vi.mocked(URL.createObjectURL).mock.calls[0][0] as Blob;
  expect(blob.type).toBe('image/png');
  expect(await blobBytes(blob)).toEqual(expected);
  process.stdout.write(`snip explicit-copy fixture ${JSON.stringify({ shape, payloadBytes: bytes.byteLength,
    explicitSliceCalls: slice.mock.calls.length, explicitSliceBytes: slice.mock.calls.length * bytes.byteLength,
    compatibilityCopyBytes: shape === 'shared-subview' ? bytes.byteLength : 0 })}\n`);
  // Blob's platform snapshot is required and deliberately NOT counted as removable allocation.
  expect(slice).not.toHaveBeenCalled();
});

it('ArrayBuffer remains accepted byte-for-byte', async () => {
  tauriMocks().invoke.mockResolvedValue(new Uint8Array([137, 80, 78, 71]).buffer);
  await snipImageUrl(context);
  expect(await blobBytes(vi.mocked(URL.createObjectURL).mock.calls[0][0] as Blob)).toEqual([137, 80, 78, 71]);
});

it.each([[], [137, 80], new Uint8Array(), new ArrayBuffer(0)])('invalid/empty transport %j creates no URL', async bytes => {
  tauriMocks().invoke.mockResolvedValue(bytes);
  await expect(snipImageUrl(context)).rejects.toThrow('image-failed');
  expect(URL.createObjectURL).not.toHaveBeenCalled();
});

it('stale response allocates neither explicit slice nor object URL', async () => {
  const bytes = new Uint8Array([137, 80]); const slice = vi.spyOn(bytes, 'slice');
  tauriMocks().invoke.mockResolvedValue(bytes);
  await expect(snipImageUrl(context, () => false)).rejects.toThrow('stale');
  expect(slice).not.toHaveBeenCalled(); expect(URL.createObjectURL).not.toHaveBeenCalled();
});

it.each([false, true])('Blob boundary shared=%s uses compatibility copy only for shared backing', async shared => {
  const backing = shared ? new SharedArrayBuffer(8) : new ArrayBuffer(8);
  const bytes = new Uint8Array(backing, 2, 4); bytes.set([137, 80, 78, 71]);
  const NativeBlob = Blob;
  let captured: BlobPart[] = [];
  vi.stubGlobal('Blob', class extends NativeBlob {
    constructor(parts: BlobPart[], options?: BlobPropertyBag) { captured = parts; super(parts, options); }
  });
  tauriMocks().invoke.mockResolvedValue(bytes);
  await snipImageUrl(context);
  const part = captured[0] as Uint8Array;
  expect(part).toBeInstanceOf(Uint8Array);
  expect(part.buffer).toBeInstanceOf(ArrayBuffer);
  expect(part.byteLength).toBe(4);
  expect(Array.from(part)).toEqual([137, 80, 78, 71]);
  if (shared) expect(part.buffer).not.toBe(backing);
  else { expect(part.buffer).toBe(backing); expect(part.byteOffset).toBe(2); }
});
