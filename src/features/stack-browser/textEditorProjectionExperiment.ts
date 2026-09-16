/** P03 test-only bounded projection. Not imported by product runtime. */
import { open, type FileHandle } from 'node:fs/promises';
import { LIMITS, SCHEMA, createSelection, type Encoding, type Lease } from './textEditorProtocol.js';

export const PROJECTION_LIMITS = Object.freeze({
  units: LIMITS.projectionUnits, segments: LIMITS.segments, rows: LIMITS.rows,
  activeReads: LIMITS.reads, totalCredits: LIMITS.payloads, pendingSeeks: LIMITS.pendingSeeks,
  bytes: LIMITS.pendingBytes, operations: LIMITS.pendingEdits, leases: LIMITS.leases,
  selections: LIMITS.selections,
});

type LineState = 'exact' | 'unknown' | 'indexing';
type RangeProvider = { read(start: number, length: number): Promise<Uint8Array>; close?(): Promise<void> };
type Boundary = { local: number; byte: number };
type Decoded = { text: string; newlines: { localOffset: number; kind: 'lf' | 'cr' | 'crlf' }[]; boundaries: Boundary[] };

function fail(code: string): never { throw new Error(code); }

export function decodeSegment(bytes: Uint8Array, encoding: Encoding, bomBytes = 0): Decoded {
  if (![0, 2, 3].includes(bomBytes) || bomBytes > bytes.length) fail('InvalidTextBoundary');
  const decoderEncoding = encoding === 'utf8' ? 'utf-8' : encoding === 'utf16le' ? 'utf-16le' : 'utf-16be';
  let raw: string;
  try { raw = new TextDecoder(decoderEncoding, { fatal: true, ignoreBOM: true }).decode(bytes.subarray(bomBytes)); }
  catch { return fail('InvalidTextBoundary'); }
  let text = '', byte = bomBytes;
  const boundaries: Boundary[] = [{ local: 0, byte }], newlines: Decoded['newlines'] = [];
  for (let index = 0; index < raw.length;) {
    const scalar = String.fromCodePoint(raw.codePointAt(index)!);
    const next = raw[index + scalar.length];
    if (scalar === '\r') {
      const crlf = next === '\n';
      text += '\n';
      newlines.push({ localOffset: text.length - 1, kind: crlf ? 'crlf' : 'cr' });
      byte += encoding === 'utf8' ? (crlf ? 2 : 1) : (crlf ? 4 : 2);
      index += crlf ? 2 : 1;
      boundaries.push({ local: text.length, byte });
      continue;
    }
    text += scalar;
    if (scalar === '\n') newlines.push({ localOffset: text.length - 1, kind: 'lf' });
    byte += encoding === 'utf8' ? new TextEncoder().encode(scalar).length : scalar.length * 2;
    index += scalar.length;
    boundaries.push({ local: text.length, byte });
  }
  if (text.length > PROJECTION_LIMITS.units || newlines.length + 1 > PROJECTION_LIMITS.rows) fail('ResourceLimit');
  return { text, newlines, boundaries };
}

/** Decode transport fragments as one logical segment, including CRLF split between fragments. */
export function decodeSplitSegments(parts: readonly Uint8Array[], encoding: Encoding, bomBytes = 0): Decoded {
  const bytes = Buffer.concat(parts.map(part => Buffer.from(part)));
  return decodeSegment(bytes, encoding, bomBytes);
}

export function localAtByte(decoded: Decoded, byte: number): number {
  const point = decoded.boundaries.find(boundary => boundary.byte === byte);
  if (!point) fail('InvalidTextBoundary');
  return point.local;
}

export class FileRangeProvider implements RangeProvider {
  #handle?: FileHandle;
  constructor(private readonly path: string) {}
  async read(start: number, length: number): Promise<Uint8Array> {
    this.#handle ??= await open(this.path, 'r');
    const buffer = Buffer.alloc(Math.min(length, LIMITS.block));
    const { bytesRead } = await this.#handle.read(buffer, 0, buffer.length, start);
    return buffer.subarray(0, bytesRead);
  }
  async close(): Promise<void> { await this.#handle?.close(); this.#handle = undefined; }
}

class MemoryRangeProvider implements RangeProvider {
  constructor(private readonly bytes: Uint8Array, private readonly withholdAfter = Infinity) {}
  async read(start: number, length: number): Promise<Uint8Array> {
    if (start >= this.withholdAfter) fail('WithheldRead');
    const end = Math.min(this.bytes.length, start + length, this.withholdAfter);
    return this.bytes.slice(start, end);
  }
}

type LeasePoint = { lease: Lease; segment: number; offset: number; affinity: 'before' | 'after' };
type SelectionState = { from: number; to: number; owners: string[]; state: 'active' | 'released' | 'expired' };
type Snapshot = { before: string; after: string };
type QueuedSeek = { byte: number; generation: number; resolve(value: Lease): void; reject(error: Error): void };
type Operation = { id: string; bytes: number };

export class ProjectionSession {
  visibleText = '';
  line: { state: LineState; count: string | null } = { state: 'unknown', count: null };
  lease!: Lease;
  readonly metrics = { bytesRead: 0, maxResidentBytes: 0, activeReads: 0, maxActiveReads: 0, totalCredits: 0, maxTotalCredits: 0, pendingSeeks: 0 };
  #generation = 0;
  #lineState: LineState;
  #leases = new Map<string, Lease>();
  #selections = new Map<string, SelectionState>();
  #pending: Operation[] = [];
  #inflight: Operation | null = null;
  #pendingBytes = 0;
  #pendingSeek: QueuedSeek | null = null;
  #undo: Snapshot[] = [];
  #redo: Snapshot[] = [];
  #document = '';

  constructor(private readonly provider: RangeProvider, options: { lineState?: LineState } = {}) {
    this.#lineState = options.lineState ?? 'unknown';
  }
  static fromBytes(bytes: Uint8Array, options: { lineState?: LineState; withholdAfter?: number } = {}): ProjectionSession {
    return new ProjectionSession(new MemoryRangeProvider(bytes, options.withholdAfter), options);
  }
  async seek(byte: number): Promise<Lease> {
    const generation = ++this.#generation;
    return new Promise<Lease>((resolve, reject) => {
      const request = { byte, generation, resolve, reject };
      if (this.metrics.activeReads < PROJECTION_LIMITS.activeReads && this.metrics.totalCredits < PROJECTION_LIMITS.totalCredits) this.#startSeek(request);
      else {
        this.#pendingSeek?.reject(new Error('Cancelled'));
        this.#pendingSeek = request;
        this.metrics.pendingSeeks = 1;
      }
    });
  }
  async #startSeek(request: QueuedSeek): Promise<void> {
    this.metrics.activeReads++; this.metrics.totalCredits++;
    this.metrics.maxActiveReads = Math.max(this.metrics.maxActiveReads, this.metrics.activeReads);
    this.metrics.maxTotalCredits = Math.max(this.metrics.maxTotalCredits, this.metrics.totalCredits);
    try {
      const bytes = await this.provider.read(request.byte, Math.min(LIMITS.block, PROJECTION_LIMITS.units * 4));
      if (request.generation !== this.#generation) throw new Error('StaleRevision');
      this.metrics.bytesRead += bytes.length;
      this.metrics.maxResidentBytes = Math.max(this.metrics.maxResidentBytes, bytes.length);
      const decoded = decodeSegment(bytes, 'utf8');
      this.visibleText = decoded.text;
      if (!this.#document) this.#document = decoded.text;
      this.line = { state: this.#lineState, count: null };
      const lease: Lease = { schema: SCHEMA, sessionId: 'p03', sourceGeneration: 'source1', documentRevision: '0',
        viewGeneration: String(request.generation), leaseId: `view${request.generation}`,
        segments: [{ encoding: 'utf8', byteStart: String(request.byte), text: decoded.text, newlines: decoded.newlines }],
        line: this.line, context: { before: request.byte ? 'continued' : 'complete', after: bytes.length === LIMITS.block ? 'continued' : 'complete', continuationId: request.byte || bytes.length === LIMITS.block ? `continuation${request.generation}` : null } };
      this.lease = lease; this.#leases.set(lease.leaseId, lease);
      request.resolve(lease);
    } catch (error) {
      this.metrics.totalCredits--;
      request.reject(error as Error);
    } finally {
      this.metrics.activeReads--;
      this.#drainSeek();
    }
  }
  #drainSeek(): void {
    if (!this.#pendingSeek || this.metrics.activeReads >= PROJECTION_LIMITS.activeReads || this.metrics.totalCredits >= PROJECTION_LIMITS.totalCredits) return;
    const next = this.#pendingSeek; this.#pendingSeek = null; this.metrics.pendingSeeks = 0; void this.#startSeek(next);
  }
  acceptLease(lease: Lease): void { if (lease.viewGeneration !== String(this.#generation)) fail('StaleRevision'); this.hydrate(lease); }
  hydrate(lease: Lease): void { this.lease = lease; this.visibleText = lease.segments.map((segment) => segment.text).join(''); }
  pinLease(lease: Lease): void { if (!this.#leases.has(lease.leaseId) && this.#leases.size >= PROJECTION_LIMITS.leases) fail('ResourceLimit'); this.#leases.set(lease.leaseId, lease); }
  releaseLease(id: string): void { if (this.#leases.delete(id)) this.metrics.totalCredits--; this.#drainSeek(); }
  edit(from: number, to: number, insertion: string): void {
    if (from < 0 || to < from || to > this.visibleText.length) fail('InvalidTextBoundary');
    const insertionBytes = new TextEncoder().encode(insertion).length;
    if (this.#pending.length + Number(this.#inflight !== null) >= PROJECTION_LIMITS.operations || this.#pendingBytes + insertionBytes > PROJECTION_LIMITS.bytes) fail('ResourceLimit');
    const before = this.#document || this.visibleText;
    this.visibleText = `${this.visibleText.slice(0, from)}${insertion}${this.visibleText.slice(to)}`;
    this.#document = `${before.slice(0, from)}${insertion}${before.slice(to)}`;
    this.#undo.push({ before, after: this.#document }); this.#redo = [];
    const operation = { id: `operation${this.#undo.length}`, bytes: insertionBytes };
    this.#pendingBytes += insertionBytes;
    if (this.#inflight === null) this.#inflight = operation; else this.#pending.push(operation);
  }
  ack(): void { if (!this.#inflight) fail('InvalidRequest'); this.#pendingBytes -= this.#inflight.bytes; this.#inflight = this.#pending.shift() ?? null; }
  get inFlightCount(): number { return Number(this.#inflight !== null); }
  get pendingOperationCount(): number { return this.#pending.length; }
  get historyDepth(): number { return this.#undo.length; }
  get pendingInputBytes(): number { return this.#pendingBytes; }
  select(id: string, anchor: LeasePoint, head: LeasePoint): void {
    if (!this.#selections.has(id) && this.#selections.size >= PROJECTION_LIMITS.selections) fail('ResourceLimit');
    for (const point of [anchor, head]) if (this.#leases.get(point.lease.leaseId) !== point.lease) fail('Unauthorized');
    createSelection({ sessionId: 'p03', sourceGeneration: 'source1', documentRevision: '0', inputSequence: '0' }, id, anchor, head);
    const localPosition = (point: LeasePoint) => Number(BigInt(point.lease.segments[point.segment].byteStart)) + point.offset;
    const from = localPosition(anchor), to = localPosition(head);
    if (from < 0 || to < from || to > this.#document.length) fail('InvalidTextBoundary');
    this.#selections.set(id, { from, to, owners: [anchor.lease.leaseId, head.lease.leaseId], state: 'active' });
  }
  #selection(id: string): SelectionState {
    const selection = this.#selections.get(id); if (!selection || selection.state === 'released' || selection.owners.some(owner => !this.#leases.has(owner))) fail('Unauthorized');
    if (selection.state === 'expired') fail('StaleRevision'); return selection;
  }
  exportSelection(id: string): string { const s = this.#selection(id); return this.#document.slice(s.from, s.to); }
  replaceSelection(id: string, text: string): void { const s = this.#selection(id); this.edit(s.from, s.to, text); }
  releaseSelection(id: string): void { const s = this.#selection(id); s.state = 'released'; }
  expireSelections(): void { for (const selection of this.#selections.values()) if (selection.state === 'active') selection.state = 'expired'; }
  undo(): void { const item = this.#undo.pop(); if (!item) return; this.#redo.push(item); this.#document = item.before; this.visibleText = item.before; }
  redo(): void { const item = this.#redo.pop(); if (!item) return; this.#undo.push(item); this.#document = item.after; this.visibleText = item.after; }
  documentText(): string { return this.#document; }
}
