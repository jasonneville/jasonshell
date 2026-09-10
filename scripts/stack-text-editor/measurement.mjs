import { mkdir, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';

export const MEASUREMENT_SCHEMA_VERSION = 'stack-text-measurement.v1';
export const MEASUREMENT_EVENTS = Object.freeze([
  'intent',
  'firstPaint',
  'localMutation',
  'backendAck',
  'durable',
  'eof',
  'indexComplete',
  'savePhase'
]);
export const MEASUREMENT_EVENT_ALIASES = Object.freeze({
  glyphPaint: 'firstPaint',
  localInput: 'localMutation',
  ack: 'backendAck',
  acknowledgement: 'backendAck',
  durability: 'durable',
  eofComplete: 'eof',
  indexCompletion: 'indexComplete'
});
export const T01_03_PASSING_CRITERIA = Object.freeze([
  'intent, firstPaint, localMutation, backendAck, durable, eof, indexComplete, and savePhase remain separate events',
  'deliberate delay/stall detection reports event, elapsed time, threshold, and deliberate flag',
  'native private bytes/process/queue/I/O/disk observations are separated from control runs',
  'missing native paint evidence is blocked rather than inferred from browser or promise timing',
  'event artifacts contain no source text, clipboard payload, or secret fields'
]);

const MAX_U64 = 18446744073709551615n;
const TOKEN_PATTERN = /^[A-Za-z0-9._:-]{1,128}$/u;
const PROOF_PATTERN = /^(?:sha256:[0-9a-f]{64}|trace:[A-Za-z0-9._:-]{1,96})$/u;
const SOURCES = Object.freeze(['native', 'browser', 'synthetic', 'control']);
const PAINT_EVIDENCE = Object.freeze(['range-read-only', 'host-callback', 'browser-only', 'awaiting-independent-native-review']);
const SAVE_PHASES = Object.freeze(['beforeStage', 'afterStage', 'writeBatch', 'flush', 'revalidate', 'backup', 'publish', 'published', 'inspect', 'journalCommit', 'cleanup']);
const EVENT_METADATA_SCHEMAS = Object.freeze({
  intent: Object.freeze({
    experiment: (value) => assertEnum(value, ['controlled-editor'], 'experiment')
  }),
  firstPaint: Object.freeze({
    nativePaintObserved: assertBoolean,
    nativeProof: assertProof,
    nativeProofProcessId: (value) => assertSafeInteger(value, 'nativeProofProcessId'),
    paintEvidence: (value) => assertEnum(value, PAINT_EVIDENCE, 'paintEvidence'),
    glyphPaintEvidence: (value) => assertEnum(value, PAINT_EVIDENCE, 'glyphPaintEvidence'),
    queueDepth: (value) => assertSafeInteger(value, 'queueDepth'),
    inFlightPayloads: (value) => assertSafeInteger(value, 'inFlightPayloads'),
    bytesRead: (value) => assertSafeInteger(value, 'bytesRead'),
    bytesWritten: (value) => assertSafeInteger(value, 'bytesWritten'),
    diskBytes: (value) => assertSafeInteger(value, 'diskBytes'),
    width: (value) => assertFiniteNumber(value, 'width'),
    height: (value) => assertFiniteNumber(value, 'height'),
    requestId: (value) => assertToken(value, 'requestId'),
    schemaRevision: (value) => assertEnum(value, ['stack-text-editor.v1'], 'schemaRevision')
  }),
  localMutation: Object.freeze({
    inputToPaintMs: (value) => assertFiniteNumber(value, 'inputToPaintMs'),
    queueDepth: (value) => assertSafeInteger(value, 'queueDepth'),
    inFlightPayloads: (value) => assertSafeInteger(value, 'inFlightPayloads'),
    bytesRead: (value) => assertSafeInteger(value, 'bytesRead'),
    bytesWritten: (value) => assertSafeInteger(value, 'bytesWritten'),
    diskBytes: (value) => assertSafeInteger(value, 'diskBytes'),
    revision: assertRevision,
    requestId: (value) => assertToken(value, 'requestId'),
    schemaRevision: (value) => assertEnum(value, ['stack-text-editor.v1'], 'schemaRevision')
  }),
  backendAck: Object.freeze({
    revision: assertRevision,
    deliberate: assertBoolean
  }),
  durable: Object.freeze({
    revision: assertRevision
  }),
  eof: Object.freeze({}),
  indexComplete: Object.freeze({
    known: assertBoolean
  }),
  savePhase: Object.freeze({
    phase: (value) => assertEnum(value, SAVE_PHASES, 'phase'),
    revision: assertRevision,
    batchBytes: (value) => value === null ? undefined : assertSafeInteger(value, 'batchBytes')
  })
});
const DEFAULT_THRESHOLDS = Object.freeze({ firstPaintMs: 250, localMutationMs: 32, backendAckMs: 50, durableMs: 1000 });

function assertEnum(value, allowed, fieldName) {
  if (typeof value !== 'string' || !allowed.includes(value)) throw new Error(`unsupported measurement ${fieldName} value`);
}

function assertToken(value, fieldName) {
  if (typeof value !== 'string' || !TOKEN_PATTERN.test(value)) throw new Error(`unsupported measurement ${fieldName} value`);
}

function assertBoolean(value, fieldName = 'boolean') {
  if (typeof value !== 'boolean') throw new Error(`measurement ${fieldName} must be boolean`);
}

function assertProof(value) {
  if (typeof value !== 'string' || !PROOF_PATTERN.test(value)) throw new Error('measurement nativeProof must be a typed verifier digest');
}

function assertFiniteNumber(value, fieldName) {
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0) throw new Error(`measurement ${fieldName} must be a finite non-negative number`);
}

function assertSafeInteger(value, fieldName) {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0) throw new Error(`measurement ${fieldName} must be a non-negative safe integer`);
}

function assertRevision(value, fieldName = 'revision') {
  if (typeof value !== 'string' || !/^(?:0|[1-9]\d{0,19})$/u.test(value) || BigInt(value) > MAX_U64) {
    throw new Error(`measurement ${fieldName} must be a canonical u64 string`);
  }
}

function assertMeasurementObject(value) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) throw new TypeError('measurement metadata must be a flat object');
}

function validateEventFields(eventName, fields) {
  assertMeasurementObject(fields);
  const schema = EVENT_METADATA_SCHEMAS[eventName];
  if (!schema) throw new Error(`unknown measurement event ${eventName}`);
  for (const [key, value] of Object.entries(fields)) {
    if (key === 'source') {
      if (typeof value !== 'string' || !SOURCES.includes(value)) throw new Error('unsupported measurement source');
      continue;
    }
    if (key === 'atMs') {
      assertFiniteNumber(value, 'atMs');
      continue;
    }
    if (!Object.hasOwn(schema, key)) throw new Error(`unknown measurement field ${key} for ${eventName}`);
    schema[key](value, key);
  }
}

function validateMeasurementEvent(event, nativePaintVerifier = () => false) {
  assertMeasurementObject(event);
  if (typeof event.event !== 'string' || !MEASUREMENT_EVENTS.includes(event.event)) throw new Error('events must use the scoped measurement schema');
  if (event.sequence !== undefined) assertSafeInteger(event.sequence, 'sequence');
  if (event.atMs === undefined) throw new Error('measurement event timestamp is required');
  assertFiniteNumber(event.atMs, 'atMs');
  if (event.source === undefined) throw new Error('measurement event source is required');
  if (typeof event.source !== 'string' || !SOURCES.includes(event.source)) throw new Error('unsupported measurement source');
  const metadata = Object.create(null);
  for (const [key, value] of Object.entries(event)) {
    if (!['event', 'sequence', 'atMs', 'source'].includes(key)) metadata[key] = value;
  }
  validateEventFields(event.event, metadata);
  if (event.event === 'firstPaint' && event.source === 'native') {
    if (event.nativePaintObserved !== true || !event.nativeProof || !nativePaintVerifier(event)) {
      throw new Error('native firstPaint requires independently verified native paint evidence');
    }
  }
}

export function validateMeasurementEvents(events, options = {}) {
  if (!Array.isArray(events)) throw new TypeError('events must be an array');
  for (const event of events) validateMeasurementEvent(event, options.nativePaintVerifier);
  return events;
}

function normalizeSource(source) {
  if (typeof source !== 'string' || !SOURCES.includes(source)) throw new Error(`unsupported measurement source ${String(source)}`);
  return source;
}

export function detectMeasurementStalls(events, thresholds = DEFAULT_THRESHOLDS) {
  const firstByEvent = new Map();
  for (const event of events) if (!firstByEvent.has(event.event)) firstByEvent.set(event.event, event);
  const intentAt = firstByEvent.get('intent')?.atMs;
  if (intentAt === undefined) return { detected: false, stalls: [], reason: 'intent event missing' };
  const pairs = [
    ['firstPaint', 'firstPaintMs'],
    ['localMutation', 'localMutationMs'],
    ['backendAck', 'backendAckMs'],
    ['durable', 'durableMs']
  ];
  const stalls = [];
  for (const [eventName, thresholdKey] of pairs) {
    const event = firstByEvent.get(eventName);
    if (!event) continue;
    const baseline = eventName === 'backendAck' ? firstByEvent.get('localMutation')?.atMs
      : eventName === 'durable' ? firstByEvent.get('backendAck')?.atMs : intentAt;
    const elapsedMs = eventName === 'localMutation' ? event.inputToPaintMs
      : baseline === undefined ? undefined : Math.max(0, event.atMs - baseline);
    if (typeof elapsedMs !== 'number' || !Number.isFinite(elapsedMs)) continue;
    const thresholdMs = Number(thresholds[thresholdKey] ?? DEFAULT_THRESHOLDS[thresholdKey]);
    if (elapsedMs > thresholdMs) stalls.push({ event: eventName, elapsedMs, thresholdMs, deliberate: Boolean(event.deliberate) });
  }
  return { detected: stalls.length > 0, stalls };
}

export function summarizeMeasurement(events, options = {}) {
  validateMeasurementEvents(events, options);
  const thresholds = { ...DEFAULT_THRESHOLDS, ...(options.thresholds ?? {}) };
  const firstByEvent = new Map();
  for (const event of events) if (!firstByEvent.has(event.event)) firstByEvent.set(event.event, event);
  const intentAt = firstByEvent.get('intent')?.atMs ?? null;
  const durations = {};
  for (const name of MEASUREMENT_EVENTS) {
    const event = firstByEvent.get(name);
    durations[`${name}FromIntentMs`] = intentAt === null || !event ? null : Math.max(0, event.atMs - intentAt);
  }
  durations.inputToPaintMs = firstByEvent.get('localMutation')?.inputToPaintMs ?? null;
  const nativePaint = firstByEvent.get('firstPaint')?.source === 'native'
    ? { status: 'observed', event: firstByEvent.get('firstPaint') }
    : { status: 'blocked', reason: 'native paint evidence was not observed; promise/browser timing is not a native paint claim' };
  return {
    schemaVersion: MEASUREMENT_SCHEMA_VERSION,
    mode: options.mode ?? 'editor',
    eventCount: events.length,
    durations,
    nativePaint,
    stalls: detectMeasurementStalls(events, thresholds),
    queue: options.queue ?? null,
    io: options.io ?? null,
    nativeProcess: options.nativeProcess ?? null,
    disk: options.disk ?? null
  };
}

export function createMeasurementRun(options = {}) {
  const mode = options.mode ?? 'editor';
  if (!['editor', 'control'].includes(mode)) throw new Error('measurement mode must be editor or control');
  const clock = options.clock ?? (() => performance.now());
  const nativePaintVerifier = typeof options.nativePaintVerifier === 'function' ? options.nativePaintVerifier : () => false;
  const startedAt = clock();
  const events = [];
  let sequence = 0;
  const queue = { maxDepth: 0, samples: [] };
  const io = { bytesRead: 0, bytesWritten: 0, reads: 0, writes: 0 };
  const disk = { bytesAllocated: null, bytesFreed: null };

  function record(event, fields = {}) {
    const canonicalEvent = MEASUREMENT_EVENT_ALIASES[event] ?? event;
    if (!MEASUREMENT_EVENTS.includes(canonicalEvent)) throw new Error(`unknown measurement event ${event}`);
    assertMeasurementObject(fields);
    const source = normalizeSource(fields.source === undefined ? (mode === 'control' ? 'control' : 'synthetic') : fields.source);
    const atMs = fields.atMs === undefined ? clock() : fields.atMs;
    validateEventFields(canonicalEvent, { ...fields, atMs, source });
    if (canonicalEvent === 'firstPaint' && source === 'native') {
      if (fields.nativePaintObserved !== true || !fields.nativeProof || !nativePaintVerifier({ ...fields, source, atMs })) {
        throw new Error('native firstPaint requires independently verified native paint evidence');
      }
    }
    if (atMs < startedAt) throw new Error('measurement timestamps must be finite and monotonic from run start');
    const entry = { sequence: sequence++, event: canonicalEvent, atMs, source };
    for (const [key, value] of Object.entries(fields)) if (!['atMs', 'source'].includes(key)) entry[key] = value;
    if (events.length > 0 && atMs < events.at(-1).atMs) throw new Error('measurement events must be monotonic');
    events.push(entry);
    if (canonicalEvent === 'localMutation' && Number.isFinite(fields.queueDepth)) {
      queue.maxDepth = Math.max(queue.maxDepth, Number(fields.queueDepth));
      queue.samples.push({ atMs, depth: Number(fields.queueDepth) });
    }
    return entry;
  }

  function recordIo(values = {}) {
    for (const key of ['bytesRead', 'bytesWritten', 'reads', 'writes']) {
      if (values[key] !== undefined) io[key] = Math.max(0, Number(values[key]));
    }
    return { ...io };
  }

  function recordDisk(values = {}) {
    for (const key of ['bytesAllocated', 'bytesFreed']) {
      if (values[key] !== undefined && values[key] !== null) disk[key] = Math.max(0, Number(values[key]));
    }
    return { ...disk };
  }

  function complete(options = {}) {
    const finishedAt = Number(options.finishedAt ?? clock());
    const summary = summarizeMeasurement(events, { ...options, mode, queue, io, disk, nativePaintVerifier });
    return {
      schemaVersion: MEASUREMENT_SCHEMA_VERSION,
      mode,
      startedAt,
      finishedAt,
      events: events.map((event) => ({ ...event })),
      summary,
      control: options.control ?? null,
      experiment: options.experiment ?? null
    };
  }

  return { mode, record, recordIo, recordDisk, complete, events, queue, io, disk };
}

export function compareControlRun(editorArtifact, controlArtifact) {
  const editorSummary = editorArtifact?.summary;
  const controlSummary = controlArtifact?.summary;
  if (!editorSummary || !controlSummary) throw new Error('editor and control artifacts must include summaries');
  return {
    schemaVersion: MEASUREMENT_SCHEMA_VERSION,
    editorMode: editorArtifact.mode,
    controlMode: controlArtifact.mode,
    firstPaintDeltaMs: editorSummary.durations.firstPaintFromIntentMs === null || controlSummary.durations.firstPaintFromIntentMs === null
      ? null
      : editorSummary.durations.firstPaintFromIntentMs - controlSummary.durations.firstPaintFromIntentMs,
    nativeEvidence: editorSummary.nativePaint.status,
    editorStalls: editorSummary.stalls,
    controlStalls: controlArtifact.controlProbe ?? controlSummary.stalls,
    ioDelta: {
      bytesRead: (editorSummary.io?.bytesRead ?? 0) - (controlSummary.io?.bytesRead ?? 0),
      bytesWritten: (editorSummary.io?.bytesWritten ?? 0) - (controlSummary.io?.bytesWritten ?? 0)
    },
    note: 'Control subtraction is diagnostic only; it does not turn browser timing into native paint evidence.'
  };
}
export async function writeMeasurementArtifact(path, artifact, options = {}) {
  if (artifact === null || typeof artifact !== 'object' || Array.isArray(artifact)) throw new TypeError('measurement artifact must be an object');
  validateMeasurementEvents(artifact.events, options);
  const summaryEvent = artifact.summary?.nativePaint?.event;
  if (summaryEvent !== undefined) validateMeasurementEvent(summaryEvent, options.nativePaintVerifier);
  if (artifact.summary?.nativePaint?.status === 'observed' && summaryEvent?.source !== 'native') {
    throw new Error('native summary requires independently verified native paint evidence');
  }
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(artifact, null, 2)}\n`, 'utf8');
  return path;
}

export async function writeMeasurementEvents(path, events, options = {}) {
  validateMeasurementEvents(events, options);
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${events.map((event) => JSON.stringify(event)).join('\n')}${events.length ? '\n' : ''}`, 'utf8');
  return path;
}
export const createMeasurementHarness = createMeasurementRun;
