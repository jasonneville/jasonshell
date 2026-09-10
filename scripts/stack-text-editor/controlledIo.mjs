import { deterministicRandom, assertU64Decimal } from './byteTextOracle.mjs';

export const RANGE_IO_SCHEMA_VERSION = 'stack-text-range-io.v1';
export const PUBLICATION_FAILPOINTS = Object.freeze([
  'beforeStage',
  'afterStage',
  'writeBatch',
  'flush',
  'revalidate',
  'backup',
  'publish',
  'inspect',
  'journalCommit',
  'cleanup'
]);

export class ControlledIoError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'ControlledIoError';
    this.code = code;
    this.details = details;
  }
}

function asByteArray(value) {
  if (value instanceof Uint8Array) return value;
  if (value instanceof ArrayBuffer) return new Uint8Array(value);
  if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
  throw new TypeError('range source must be bytes');
}

function delay(milliseconds) {
  if (!milliseconds) return Promise.resolve();
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function normalizeReadArgs(offsetOrRequest, length, requestId) {
  if (typeof offsetOrRequest === 'object' && offsetOrRequest !== null) {
    return {
      offset: offsetOrRequest.offset,
      length: offsetOrRequest.length,
      requestId: offsetOrRequest.requestId ?? offsetOrRequest.operationId,
      priority: offsetOrRequest.priority ?? 'demand'
    };
  }
  return { offset: offsetOrRequest, length, requestId, priority: 'demand' };
}

export class DeterministicRangeIo {
  constructor(source, options = {}) {
    this.source = asByteArray(source);
    this.blockSize = Math.min(256 * 1024, Math.max(1, Number(options.blockSize ?? 64 * 1024)));
    this.delayMs = Math.max(0, Number(options.delayMs ?? 0));
    this.withholdAfterOffset = options.withholdAfterFirstBlock ? this.blockSize : options.withholdAfterOffset === undefined ? null : Number(options.withholdAfterOffset);
    this.withholdEof = Boolean(options.withholdEof);
    this.withholdAll = Boolean(options.withholdAll);
    this.failReads = new Map();
    this.pending = new Map();
    this.completed = [];
    this.readLog = [];
    this.bytesRead = 0;
    this.bytesRequested = 0;
    this.maxPending = 0;
    this.requestCounter = 0;
    this.reorderMode = options.reorderMode ?? 'auto';
    this.random = deterministicRandom(options.seed ?? 1);
  }

  get size() { return this.source.length; }
  get pendingCount() { return this.pending.size; }

  shouldWithhold(offset, length) {
    const end = offset + length;
    return this.withholdAll || (this.withholdAfterOffset !== null && offset >= this.withholdAfterOffset) || (this.withholdEof && end >= this.source.length);
  }

  configure(options = {}) {
    if (options.withholdAfterFirstBlock !== undefined) this.withholdAfterOffset = options.withholdAfterFirstBlock ? this.blockSize : null;
    if (options.withholdAfterOffset !== undefined) this.withholdAfterOffset = options.withholdAfterOffset === null ? null : Number(options.withholdAfterOffset);
    if (options.withholdEof !== undefined) this.withholdEof = Boolean(options.withholdEof);
    if (options.withholdAll !== undefined) this.withholdAll = Boolean(options.withholdAll);
    if (options.reorderMode !== undefined) this.reorderMode = options.reorderMode;
    if (options.delayMs !== undefined) this.delayMs = Math.max(0, Number(options.delayMs));
  }

  failRequest(requestId, code = 'IoFailure', message = 'controlled range read failed') {
    this.failReads.set(String(requestId), { code, message });
  }

  read(offsetOrRequest, length, requestId) {
    const request = normalizeReadArgs(offsetOrRequest, length, requestId);
    const id = String(request.requestId ?? `range-${++this.requestCounter}`);
    let normalizedOffset;
    let normalizedLength;
    try {
      normalizedOffset = Number(assertU64Decimal(request.offset, 'offset'));
      normalizedLength = Number(assertU64Decimal(request.length, 'length'));
    } catch (error) {
      return Promise.reject(error);
    }
    if (!Number.isSafeInteger(normalizedOffset) || !Number.isSafeInteger(normalizedLength) || normalizedOffset < 0 || normalizedLength < 0 || normalizedOffset > this.source.length || normalizedLength > this.source.length - normalizedOffset) {
      return Promise.reject(new ControlledIoError('InvalidRange', 'range lies outside the source')); 
    }
    if (normalizedLength > this.blockSize || this.pending.size >= 4) return Promise.reject(new ControlledIoError('ResourceLimit', 'range exceeds bounded I/O admission'));
    if (this.pending.has(id)) return Promise.reject(new ControlledIoError('DuplicateRequest', `range request ${id} is already pending`));
    const shouldWithhold = this.shouldWithhold(normalizedOffset, normalizedLength);
    this.bytesRequested += normalizedLength;
    const entry = {
      requestId: id,
      offset: normalizedOffset,
      length: normalizedLength,
      priority: request.priority,
      sequence: this.readLog.length,
      withheld: shouldWithhold,
      resolve: null,
      reject: null
    };
    this.readLog.push({ requestId: id, offset: normalizedOffset, length: normalizedLength, priority: request.priority, withheld: shouldWithhold });
    const promise = new Promise((resolve, reject) => {
      entry.resolve = resolve;
      entry.reject = reject;
    });
    this.pending.set(id, entry);
    this.maxPending = Math.max(this.maxPending, this.pending.size);
    if (!shouldWithhold) this.scheduleRelease(id);
    return promise;
  }

  readRange(request) { return this.read(request); }

  async scheduleRelease(requestId) {
    await delay(this.delayMs);
    if (!this.pending.has(requestId)) return;
    if (this.reorderMode === 'manual' || this.pending.get(requestId).withheld) return;
    if (this.reorderMode === 'reverse' && requestId !== [...this.pending.keys()].at(-1)) return;
    if (this.reorderMode === 'random' && this.random() < 0.5) return;
    this.release(requestId);
  }

  release(requestId) {
    const id = String(requestId);
    const entry = this.pending.get(id);
    if (!entry) throw new ControlledIoError('UnknownRequest', `no pending range request ${id}`);
    this.pending.delete(id);
    const failure = this.failReads.get(id);
    this.failReads.delete(id);
    if (failure) {
      const error = new ControlledIoError(failure.code, failure.message, { requestId: id, offset: entry.offset, length: entry.length });
      entry.reject(error);
      this.completed.push({ requestId: id, status: 'failed', code: failure.code });
      for (const pending of this.pending.values()) if (!pending.withheld) this.scheduleRelease(pending.requestId);
      return;
    }
    const value = this.source.slice(entry.offset, entry.offset + entry.length);
    this.bytesRead += value.length;
    const result = { requestId: id, offset: String(entry.offset), length: String(value.length), bytes: value };
    entry.resolve(result);
    this.completed.push({ requestId: id, status: 'completed', length: value.length });
    for (const pending of this.pending.values()) if (!pending.withheld) this.scheduleRelease(pending.requestId);
  }

  fail(requestId, code = 'IoFailure', message = 'controlled range read failed') {
    this.failRequest(requestId, code, message);
    if (this.pending.has(String(requestId))) this.release(requestId);
  }

  releaseNext(order = this.reorderMode) {
    const entries = [...this.pending.values()];
    if (entries.length === 0) return null;
    let entry;
    if (order === 'reverse') entry = entries.at(-1);
    else if (order === 'random') entry = entries[Math.floor(this.random() * entries.length)];
    else entry = entries[0];
    this.release(entry.requestId);
    return entry.requestId;
  }

  releaseAll(order = this.reorderMode) {
    const released = [];
    while (this.pending.size > 0) released.push(this.releaseNext(order));
    return released;
  }

  snapshot() {
    return {
      schemaVersion: RANGE_IO_SCHEMA_VERSION,
      sourceBytes: this.source.length,
      pending: this.pending.size,
      maxPending: this.maxPending,
      bytesRequested: this.bytesRequested,
      bytesRead: this.bytesRead,
      reads: this.readLog.length,
      completed: this.completed.length,
      pendingRequestIds: [...this.pending.keys()]
    };
  }
}

/** Test-only demand transport: two reads, four held payload credits, one latest seek. */
export class BoundedRangeTransport {
  constructor(io) {
    this.io = io;
    this.available = true;
    this.closed = false;
    this.activeReads = 0;
    this.entries = new Map();
    this.latest = null;
  }

  request(request) {
    if (this.closed) return Promise.reject(new ControlledIoError('Cancelled', 'transport closed'));
    if (this.entries.has(request.requestId) || this.latest?.request.requestId === request.requestId) return Promise.reject(new ControlledIoError('DuplicateRequest', 'request is already admitted'));
    const promise = new Promise((resolve, reject) => {
      if (this.latest) this.latest.reject(new ControlledIoError('Cancelled', 'superseded by latest demand'));
      this.latest = { request, resolve, reject, active: false, released: false };
    });
    this.drain();
    return promise;
  }

  drain() {
    if (this.closed || !this.available || !this.latest || this.activeReads >= 2 || this.entries.size >= 4) return;
    const entry = this.latest;
    this.latest = null;
    entry.active = true;
    this.activeReads++;
    this.entries.set(entry.request.requestId, entry);
    this.io.read(entry.request).then((payload) => {
      if (entry.released) return;
      entry.active = false;
      this.activeReads--;
      entry.resolve({ ...payload, release: () => this.release(entry) });
      this.drain();
    }, (error) => {
      entry.reject(error);
      this.release(entry);
    });
  }

  release(entry) {
    if (entry.released) return;
    entry.released = true;
    if (entry.active) { entry.active = false; this.activeReads--; }
    this.entries.delete(entry.request.requestId);
    this.drain();
  }

  cancel(requestId) {
    if (this.latest?.request.requestId === requestId) {
      this.latest.reject(new ControlledIoError('Cancelled', 'queued request removed'));
      this.latest = null;
      return 'removed';
    }
    const entry = this.entries.get(requestId);
    if (!entry) return 'notFound';
    if (entry.active) this.io.fail(requestId, 'Cancelled');
    entry.reject(new ControlledIoError('Cancelled', 'controlled read cancelled'));
    this.release(entry);
    return 'cancelled';
  }

  setAvailable(available) { this.available = Boolean(available); this.drain(); }
  close() {
    this.closed = true;
    if (this.latest) this.cancel(this.latest.request.requestId);
    for (const id of this.entries.keys()) this.cancel(id);
  }
  snapshot() {
    return { activeReads: this.activeReads, inFlightPayloads: this.entries.size, queued: this.latest ? 1 : 0, available: this.available, closed: this.closed };
  }
}

export class PublicationFailpointController {
  constructor(options = {}) {
    this.actions = new Map();
    this.pending = new Map();
    this.events = [];
    for (const [phase, action] of Object.entries(options)) this.configure(phase, action);
  }

  configure(phase, action = 'none') {
    if (!PUBLICATION_FAILPOINTS.includes(phase)) throw new ControlledIoError('InvalidFailpoint', `unknown publication phase ${phase}`);
    if (typeof action === 'string') this.actions.set(phase, { type: action });
    else this.actions.set(phase, { ...action });
    return this;
  }

  clear(phase) { this.actions.delete(phase); }

  async hit(phase, metadata = {}) {
    if (!PUBLICATION_FAILPOINTS.includes(phase)) throw new ControlledIoError('InvalidFailpoint', `unknown publication phase ${phase}`);
    const action = this.actions.get(phase) ?? { type: 'none' };
    this.events.push({ phase, action: action.type ?? 'none', sequence: this.events.length });
    if (action.type === 'delay') await delay(Number(action.ms ?? 1));
    if (action.type === 'withhold') {
      const token = `${phase}-${this.events.length}`;
      await new Promise((resolve, reject) => this.pending.set(token, { resolve, reject, metadata }));
    }
    if (action.type === 'cancel') throw new ControlledIoError('Cancelled', `publication interrupted at ${phase}`);
    if (action.type === 'error' || action.type === 'deny') {
      throw new ControlledIoError(action.code ?? 'IoFailure', `publication failed at ${phase}`);
    }
    if (action.type === 'ambiguous') throw new ControlledIoError('PublicationAmbiguous', `publication outcome is ambiguous at ${phase}`);
  }

  release(token) {
    const waiter = this.pending.get(token);
    if (!waiter) throw new ControlledIoError('UnknownFailpoint', `no pending failpoint ${token}`);
    this.pending.delete(token);
    waiter.resolve();
  }

  fail(token, code = 'IoFailure') {
    const waiter = this.pending.get(token);
    if (!waiter) throw new ControlledIoError('UnknownFailpoint', `no pending failpoint ${token}`);
    this.pending.delete(token);
    waiter.reject(new ControlledIoError(code, `publication failpoint ${token} failed`));
  }

  snapshot() {
    return { events: this.events.map(({ phase, action, sequence }) => ({ phase, action, sequence })), pending: [...this.pending.keys()] };
  }
}

export class TestOnlyClipboard {
  constructor(options = {}) {
    this.available = options.available !== false;
    this.delayMs = Math.max(0, Number(options.delayMs ?? 0));
    this.failCode = options.failCode ?? null;
    this.events = [];
  }

  configure(options = {}) {
    if (options.available !== undefined) this.available = Boolean(options.available);
    if (options.delayMs !== undefined) this.delayMs = Math.max(0, Number(options.delayMs));
    if (options.failCode !== undefined) this.failCode = options.failCode;
  }

  async publish(value) {
    const bytes = value instanceof Uint8Array ? value : new TextEncoder().encode(String(value));
    this.events.push({ operation: 'publish', bytes: bytes.length, sequence: this.events.length });
    await delay(this.delayMs);
    if (!this.available) throw new ControlledIoError('ClipboardUnavailable', 'clipboard publication was denied by the test boundary');
    if (this.failCode) throw new ControlledIoError(this.failCode, 'clipboard publication failed by the test boundary');
    return { published: true, bytes: bytes.length };
  }

  snapshot() { return { available: this.available, events: [...this.events] }; }
}

export async function executePublication({ bytes, revision, failpoints = new PublicationFailpointController(), chunkSize = 64 * 1024, sink = null, onPhase = null } = {}) {
  const payload = asByteArray(bytes);
  const writes = [];
  const target = sink ?? { staged: [], published: null };
  const boundedChunkSize = Math.min(256 * 1024, Math.max(1, Number(chunkSize)));
  let phase = 'not-started';
  let failurePhase = 'not-started';
  let publishedByOperation = false;
  const bytesWritten = () => writes.reduce((sum, value) => sum + value, 0);
  try {
    failurePhase = 'beforeStage';
    await onPhase?.({ phase: 'beforeStage', revision: String(revision) });
    await failpoints.hit('beforeStage', { revision });
    phase = 'staged';
    failurePhase = 'afterStage';
    await onPhase?.({ phase: 'afterStage', revision: String(revision) });
    await failpoints.hit('afterStage', { revision });
    for (let offset = 0; offset < payload.length; offset += boundedChunkSize) {
      const chunk = payload.slice(offset, Math.min(payload.length, offset + boundedChunkSize));
      failurePhase = 'writeBatch';
      await onPhase?.({ phase: 'writeBatch', revision: String(revision), offset: String(offset), length: chunk.length });
      await failpoints.hit('writeBatch', { revision, offset: String(offset), length: String(chunk.length) });
      target.staged.push(chunk);
      writes.push(chunk.length);
    }
    for (const publicationPhase of ['flush', 'revalidate', 'backup', 'publish']) {
      failurePhase = publicationPhase;
      await onPhase?.({ phase: publicationPhase, revision: String(revision) });
      await failpoints.hit(publicationPhase, { revision });
      if (publicationPhase === 'publish') {
        target.published = Uint8Array.from(target.staged.flatMap((chunk) => [...chunk]));
        publishedByOperation = true;
        phase = 'published';
      }
    }
    for (const publicationPhase of ['inspect', 'journalCommit', 'cleanup']) {
      failurePhase = publicationPhase;
      await onPhase?.({ phase: publicationPhase, revision: String(revision) });
      await failpoints.hit(publicationPhase, { revision });
    }
    return {
      status: 'published',
      outcome: 'published',
      revision: String(revision),
      bytesWritten: bytesWritten(),
      phase,
      writes
    };
  } catch (error) {
    const code = error?.code ?? 'IoFailure';
    const base = {
      revision: String(revision),
      bytesWritten: bytesWritten(),
      phase,
      writes
    };
    if (publishedByOperation) {
      return {
        status: 'published',
        outcome: 'already-published',
        ...base,
        postPublicationFailure: { code, phase: failurePhase, message: error.message }
      };
    }
    const status = code === 'Cancelled' ? 'cancelled' : code === 'PublicationAmbiguous' ? 'ambiguous' : 'failed';
    return { status, outcome: status, ...base, error: { code, message: error.message } };
  }
}
export const createControlledRangeIo = (source, options = {}) => new DeterministicRangeIo(source, options);
export const createPublicationFailpoints = (options = {}) => new PublicationFailpointController(options);
