// P01 test-only boundaries. Never imported by application code.
import { open, statfs, unlink } from 'node:fs/promises';
import { dirname } from 'node:path';
import { createHash } from 'node:crypto';

const CHUNK = 65536;
const kinds = ['utf8', 'utf16le', 'utf16be', 'minified', 'giant-line', 'many-lines', 'combining', 'invalid-tail'];
export function* corpus({ kind, seed, bytes }) {
  if (!kinds.includes(kind) || !Number.isSafeInteger(seed) || seed < 0 || seed > 0xffffffff
    || !Number.isSafeInteger(bytes) || bytes < 1) throw Error('InvalidFixture');
  const value = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
  const text = kind === 'giant-line' ? 'x'.repeat(1024) : kind === 'many-lines' ? 'x\n'
    : kind === 'combining' ? `a${'\u0301'.repeat(20000)}` : kind === 'minified' ? `${value},`
      : `seed=${value} A\u{1f600}\r\nB\rC\n`;
  let pattern = Buffer.from(text, kind.startsWith('utf16') ? 'utf16le' : 'utf8');
  if (kind === 'utf16be') pattern = pattern.swap16();
  const start = kind === 'utf16le' ? Buffer.from([0xff, 0xfe]) : kind === 'utf16be' ? Buffer.from([0xfe, 0xff])
    : kind === 'minified' ? Buffer.from('[') : Buffer.alloc(0);
  if (start.length) yield start;
  // Whole encoded patterns prevent accidental invalid tails and CRLF truncation.
  const repeats = Math.max(1, Math.ceil(bytes / pattern.length));
  const perChunk = Math.max(1, Math.floor(CHUNK / pattern.length));
  for (let left = repeats; left > 0;) {
    const count = Math.min(left, perChunk), chunk = Buffer.allocUnsafe(count * pattern.length);
    for (let n = 0; n < count; n++) pattern.copy(chunk, n * pattern.length);
    yield chunk; left -= count;
  }
  if (kind === 'minified') yield Buffer.from('0]');
  if (kind === 'invalid-tail') yield Buffer.from([0xff]);
}
export async function writeFixture(path, options, sample = () => {}) {
  const fs = await statfs(dirname(path), { bigint: true });
  if (!Number.isSafeInteger(options.bytes) || options.bytes < 1 || BigInt(options.bytes) + 1048576n > fs.bavail * fs.bsize) throw Error('ResourceLimit');
  const handle = await open(path, 'wx');
  let bytes = 0, maxChunk = 0;
  const hash = createHash('sha256');
  try {
    for (const chunk of corpus(options)) {
      let written = 0;
      while (written < chunk.length) {
        const result = await handle.write(chunk, written, chunk.length - written);
        if (!result.bytesWritten) throw Error('IoFailure');
        written += result.bytesWritten;
      }
      hash.update(chunk); bytes += chunk.length; maxChunk = Math.max(maxChunk, chunk.length);
      sample();
    }
    await handle.sync();
  } catch (error) {
    try { await handle.close(); } finally { await unlink(path); }
    throw error;
  }
  await handle.close();
  return { ...options, actualBytes: String(bytes), maxChunk, sha256: hash.digest('hex') };
}

export class ByteOracle {
  #bytes; #undo = []; #redo = []; #revision = 0n; #bomBytes;
  constructor(bytes, encoding = 'utf-8') {
    if (bytes.length > 1048576) throw Error('OracleLimit');
    if (!['utf-8', 'utf-16le', 'utf-16be'].includes(encoding)) throw Error('InvalidEncoding');
    const bom = Buffer.from(encoding === 'utf-8' ? [239, 187, 191] : encoding === 'utf-16le' ? [255, 254] : [254, 255]);
    this.#bomBytes = bytes.subarray(0, bom.length).equals(bom) ? bom.length : 0;
    this.encoding = encoding; this.#bytes = Buffer.from(bytes); this.#boundaries();
  }
  #boundaries() {
    const decoder = new TextDecoder(this.encoding, { fatal: true, ignoreBOM: true });
    // ignoreBOM=true includes U+FEFF; strip only the original file header explicitly.
    const text = decoder.decode(this.#bytes.subarray(this.#bomBytes)), boundaries = new Set([this.#bomBytes]);
    let position = this.#bomBytes, previousCr = false;
    for (const scalar of text) {
      if (previousCr && scalar === '\n') boundaries.delete(position);
      position += this.encoding === 'utf-8' ? Buffer.byteLength(scalar) : scalar.length * 2;
      boundaries.add(position); previousCr = scalar === '\r';
    }
    return boundaries;
  }
  replace(from, to, insert) {
    const boundaries = this.#boundaries();
    if (!boundaries.has(from) || !boundaries.has(to) || to < from) throw Error('InvalidTextBoundary');
    new TextDecoder(this.encoding, { fatal: true }).decode(insert);
    if (this.#bytes.length - (to - from) + insert.length > 1048576 || this.#undo.length >= 256) throw Error('OracleLimit');
    this.#undo.push(this.#bytes); this.#redo = [];
    this.#bytes = Buffer.concat([this.#bytes.subarray(0, from), insert, this.#bytes.subarray(to)]);
    this.#revision++;
  }
  undo() { if (this.#undo.length) { this.#redo.push(this.#bytes); this.#bytes = this.#undo.pop(); this.#revision++; } }
  redo() { if (this.#redo.length) { this.#undo.push(this.#bytes); this.#bytes = this.#redo.pop(); this.#revision++; } }
  get revision() { return this.#revision; }
  bytes() { return Buffer.from(this.#bytes); }
}
export class ControlledReads {
  #source; #pending = new Map();
  eof = false; order = []; bytesRead = 0;
  constructor(bytes) { if (bytes.length > 1048576) throw Error('OracleLimit'); this.#source = Buffer.from(bytes); }
  read(id, start, length) {
    if (this.#pending.size >= 2 || this.#pending.has(id)) throw Error('ResourceLimit');
    if (!Number.isSafeInteger(start) || start < 0 || !Number.isSafeInteger(length) || length < 0 || length > 262144) throw Error('InvalidRange');
    return new Promise((resolve, reject) => this.#pending.set(id, { start, length, resolve, reject }));
  }
  release(id) {
    const request = this.#pending.get(id); if (!request) throw Error('UnknownRequest');
    this.#pending.delete(id); const bytes = Buffer.from(this.#source.subarray(request.start, request.start + request.length));
    this.bytesRead += bytes.length; this.eof ||= request.start + bytes.length >= this.#source.length;
    this.order.push(id); request.resolve(bytes);
  }
  fail(id, code = 'Cancelled') {
    const request = this.#pending.get(id); if (!request) throw Error('UnknownRequest');
    this.#pending.delete(id); this.order.push(id); request.reject(Error(code));
  }
  close() { for (const id of this.#pending.keys()) this.fail(id); }
}
export async function publish(sink, bytes, failpoint = () => {}) {
  let published = false;
  try {
    for (const phase of ['write', 'flush', 'publish', 'inspect', 'journalCommit', 'cleanup']) {
      if (await failpoint(phase) === 'cancel') return { state: published ? 'published' : 'cancelled', durable: false, cancellation: published ? 'already-published' : 'acknowledged' };
      if (phase === 'publish') { sink.bytes = Buffer.from(bytes); published = true; }
    }
    return { state: 'durable', durable: true, cancellation: 'none' };
  } catch {
    return { state: published ? 'ambiguous' : 'failed', durable: false, cancellation: published ? 'ambiguous' : 'none', error: published ? 'PublicationAmbiguous' : 'IoFailure' };
  }
}

export class Credits {
  active = new Set(); held = new Set(); pending = null; closed = false;
  constructor(workerAvailable = true) { this.workerAvailable = workerAvailable; }
  request(id) {
    if (this.closed) throw Error('Cancelled'); if (!this.workerAvailable) throw Error('WorkerUnavailable');
    if (this.active.has(id) || this.held.has(id) || this.pending === id) throw Error('DuplicateRequest');
    if (this.active.size < 2 && this.active.size + this.held.size < 4) { this.active.add(id); return 'started'; }
    this.pending = id; return 'queued';
  }
  #pump() {
    if (!this.closed && this.pending && this.active.size < 2 && this.active.size + this.held.size < 4) {
      this.active.add(this.pending); this.pending = null;
    }
  }
  complete(id) { if (!this.active.delete(id)) throw Error('UnknownRequest'); this.held.add(id); this.#pump(); }
  release(id) { this.held.delete(id); this.#pump(); }
  cancel(id) { this.active.delete(id); this.held.delete(id); if (this.pending === id) this.pending = null; this.#pump(); }
  close() { this.closed = true; this.active.clear(); this.held.clear(); this.pending = null; }
}

// Admission model, not a filesystem reservation. Production must additionally reserve free space.
export function requiredQuota(parts) {
  const names = ['source', 'addStore', 'staging', 'backup', 'overhead'];
  if (!parts || Object.keys(parts).length !== names.length) throw Error('ResourceLimit');
  let total = 0n;
  for (const name of names) {
    const value = parts[name];
    if (typeof value !== 'string' || !/^(0|[1-9][0-9]{0,19})$/.test(value)) throw Error('ResourceLimit');
    total += BigInt(value); if (total > 18446744073709551615n) throw Error('ResourceLimit');
  }
  return total;
}
export class SpoolQuota {
  #sessions = new Map(); #total = 0;
  reserve(session, bytes) {
    if (typeof session !== 'string' || !/^[A-Za-z0-9_-]{1,96}$/.test(session) || !Number.isSafeInteger(bytes) || bytes <= 0) throw Error('InvalidRequest');
    const current = this.#sessions.get(session) ?? 0;
    if (current + bytes > 67108864 || this.#total + bytes > 268435456 || (!this.#sessions.has(session) && this.#sessions.size >= 32)) throw Error('ResourceLimit');
    this.#sessions.set(session, current + bytes); this.#total += bytes;
  }
  release(session) { const bytes = this.#sessions.get(session) ?? 0; this.#total -= bytes; this.#sessions.delete(session); }
  get total() { return this.#total; }
}
export class InputQueue {
  #pending = []; #inflight = null; #bytes = 0;
  admit(id, text) {
    if (typeof id !== 'string' || !/^[A-Za-z0-9_-]{1,96}$/.test(id) || typeof text !== 'string') throw Error('InvalidRequest');
    if (this.#inflight?.id === id || this.#pending.some(e => e.id === id)) throw Error('DuplicateRequest');
    const bytes = text.length * 2;
    if (bytes > 1048576) throw Error('SpoolRequired');
    if (this.#bytes + bytes > 1048576 || this.#pending.length + Number(this.#inflight !== null) >= 64) throw Error('ResourceLimit');
    this.#pending.push({ id, text, bytes }); this.#bytes += bytes;
  }
  next() { if (this.#inflight) return null; this.#inflight = this.#pending.shift() ?? null; return this.#inflight && { ...this.#inflight }; }
  settle(id, accepted) {
    if (this.#inflight?.id !== id || typeof accepted !== 'boolean') throw Error('InvalidRequest');
    if (!accepted) return 'retained';
    this.#bytes -= this.#inflight.bytes; this.#inflight = null; return 'accepted';
  }
  get bytes() { return this.#bytes; }
  get drained() { return !this.#inflight && !this.#pending.length; }
}
export class Scheduler {
  #jobs = [];
  enqueue(id, priority) {
    if (!['interactive', 'demand', 'background'].includes(priority) || !/^[A-Za-z0-9_-]{1,96}$/.test(id)) throw Error('InvalidRequest');
    if (this.#jobs.some(j => j.id === id)) throw Error('DuplicateRequest');
    if (this.#jobs.length >= 32) throw Error('ResourceLimit');
    this.#jobs.push({ id, priority });
  }
  next() {
    for (const priority of ['interactive', 'demand', 'background']) {
      const at = this.#jobs.findIndex(j => j.priority === priority);
      if (at !== -1) return this.#jobs.splice(at, 1)[0];
    }
    return null;
  }
  cancel(id) { this.#jobs = this.#jobs.filter(j => j.id !== id); }
  close() { this.#jobs = []; }
}
export async function transferSelection(oracle, from, to, clipboard, cut = false) {
  const bytes = oracle.bytes(), revision = oracle.revision;
  // Validate range before exposing any bytes. Separate oracle leaves original unchanged.
  new ByteOracle(bytes, oracle.encoding).replace(from, to, Buffer.alloc(0));
  await clipboard.write(Buffer.from(bytes.subarray(from, to)));
  if (cut) {
    if (oracle.revision !== revision) throw Error('StaleRevision');
    oracle.replace(from, to, Buffer.alloc(0));
  }
}

const eventKeys = { intent: [], workerReady: [], firstPaintCallback: ['bytesRead'], localMutation: [], inputPaintCallback: [], ack: ['revision'], durable: ['revision'], eof: ['bytesRead'], indexComplete: ['bytesRead'], savePhase: ['bytesWritten', 'phase'], controlStall: [] };
const counters = ['bytesRead', 'bytesWritten'];
function validateEvent(event) {
  if (Object.keys(event).sort().join() !== 'kind,metadata,timeMs' || !Object.hasOwn(eventKeys, event.kind)
    || !Number.isFinite(event.timeMs) || event.timeMs < 0 || !event.metadata || typeof event.metadata !== 'object' || Array.isArray(event.metadata)) throw Error('InvalidMeasurement');
  if (Object.keys(event.metadata).sort().join() !== eventKeys[event.kind].join()) throw Error('InvalidMeasurement');
  for (const [key, value] of Object.entries(event.metadata)) {
    if (counters.includes(key)) { if (!Number.isSafeInteger(value) || value < 0) throw Error('InvalidMeasurement'); }
    else if (key === 'revision') { if (typeof value !== 'string' || !/^(0|[1-9][0-9]{0,19})$/.test(value) || BigInt(value) > 18446744073709551615n) throw Error('InvalidMeasurement'); }
    else if (key === 'requestId') { if (typeof value !== 'string' || !/^[A-Za-z0-9_-]{1,96}$/.test(value)) throw Error('InvalidMeasurement'); }
    else if (key === 'phase') { if (!['queued', 'write', 'flush', 'publish', 'inspect', 'journalCommit', 'cleanup', 'failed', 'cancelled', 'ambiguous'].includes(value)) throw Error('InvalidMeasurement'); }
    else throw Error('InvalidMeasurement');
  }
}
export class Measurements {
  #events = [];
  record(kind, timeMs, metadata = {}) {
    const event = { kind, timeMs, metadata: { ...metadata } }; validateEvent(event);
    if (this.#events.length >= 256 || timeMs < (this.#events.at(-1)?.timeMs ?? 0)) throw Error('InvalidMeasurement');
    this.#events.push(event);
  }
  json() { this.#events.forEach(validateEvent); return JSON.stringify(this.#events); }
  summary() {
    const local = this.#events.find(e => e.kind === 'localMutation'), ack = this.#events.find(e => e.kind === 'ack');
    return { nativeEvidence: 'blocked', eofObserved: this.#events.some(e => e.kind === 'eof'),
      ackDelayMs: local && ack ? ack.timeMs - local.timeMs : null,
      timingBasis: 'producer monotonic callback clock, not native presentation' };
  }
}
