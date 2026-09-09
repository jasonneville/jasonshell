/** P01 executable contract only. No filesystem provider or production IPC routes. */
export const SCHEMA = 'stack-text-editor.v2' as const;
export const LIMITS = Object.freeze({ firstRead: 65536, block: 262144, projectionUnits: 131072,
  segments: 2048, rows: 4096, reads: 2, payloads: 4, pendingSeeks: 1,
  pendingBytes: 1048576, pendingEdits: 64, leases: 8, selections: 64,
  jobs: 32, events: 256, operations: 1024, results: 256,
  hotBytes: 33554432, indexBytes: 16777216, spoolBytes: 67108864, globalSpoolBytes: 268435456 });
const MAX = 18446744073709551615n;
export type Decimal = string;
export type Encoding = 'utf8' | 'utf16le' | 'utf16be';
export type Affinity = 'before' | 'after';
export type ErrorCode = 'Unauthorized' | 'UnsupportedTarget' | 'EncodingRequired' | 'InvalidTextBoundary'
  | 'StaleRevision' | 'SourceChanged' | 'SharingViolation' | 'Readonly' | 'ResourceLimit'
  | 'Cancelled' | 'IoFailure' | 'PublicationAmbiguous' | 'InvalidRequest' | 'RetryRetired' | 'ContextRequired';
export class ProtocolError extends Error {
  constructor(public readonly code: ErrorCode) { super(code); }
}
function fail(code: ErrorCode = 'InvalidRequest'): never { throw new ProtocolError(code); }
export function decimal(value: unknown): bigint {
  if (typeof value !== 'string' || !/^(0|[1-9][0-9]{0,19})$/.test(value)) fail();
  const result = BigInt(value);
  if (result > MAX) fail();
  return result;
}
function checked(value: bigint): string { if (value < 0n || value > MAX) fail(); return String(value); }
function id(value: unknown): asserts value is string {
  if (typeof value !== 'string' || !/^[A-Za-z0-9_-]{1,96}$/.test(value)) fail();
}
function integer(value: unknown, max: number): asserts value is number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0 || value > max) fail();
}
function object(value: unknown, keys: string[]): asserts value is Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)
    || Object.keys(value).length !== keys.length || keys.some(key => !Object.hasOwn(value, key))
    || Object.keys(value).some(key => !keys.includes(key))) fail();
}
function oneOf(value: unknown, values: readonly string[]) { if (!values.includes(value as string)) fail(); }
export interface Segment { encoding: Encoding; byteStart: Decimal; text: string; newlines: { localOffset: number; kind: 'lf' | 'cr' | 'crlf' }[] }
export interface Lease {
  schema: typeof SCHEMA; sessionId: string; sourceGeneration: string; documentRevision: Decimal;
  viewGeneration: Decimal; leaseId: string; segments: Segment[];
  line: { state: 'exact' | 'unknown' | 'indexing'; count: Decimal | null };
  context: { before: 'complete' | 'continued'; after: 'complete' | 'continued'; continuationId: string | null };
}
export interface SessionVersion { sessionId: string; sourceGeneration: string; documentRevision: Decimal; inputSequence: Decimal }
export interface Endpoint { byte: Decimal; affinity: Affinity }
export interface Selection extends SessionVersion {
  selectionId: string; anchor: Endpoint; head: Endpoint; direction: 'forward' | 'backward';
  state: 'active' | 'invalidated'; expiresAfterRevision: Decimal;
}
export interface Barrier { documentRevision: Decimal; inputSequence: Decimal; operationIds: string[] }
export type Receipt = { state: 'pending' } | { state: 'accepted'; documentRevision: Decimal } | { state: 'rejected'; code: ErrorCode };
export interface Job { jobId: string; sessionId: string; sourceGeneration: string; documentRevision: Decimal;
  phase: 'queued' | 'reading' | 'staging' | 'flushing' | 'publishing' | 'published' | 'durable' | 'failed' | 'cancelled' | 'ambiguous';
  completedBytes: Decimal; totalBytes: Decimal | null; cancellation: 'none' | 'requested' | 'removed' | 'acknowledged' | 'already-published' | 'ambiguous' }

/** Reconstruct boundaries from scalar widths + explicit original EOLs, never offset addition. */
export function segmentBoundaries(input: Segment): Map<number, bigint> {
  object(input, ['encoding', 'byteStart', 'text', 'newlines']);
  oneOf(input.encoding, ['utf8', 'utf16le', 'utf16be']);
  if (typeof input.text !== 'string' || input.text.length > LIMITS.projectionUnits || !Array.isArray(input.newlines)
    || input.newlines.length >= LIMITS.rows) fail('ResourceLimit');
  const eols = new Map<number, string>();
  let previous = -1;
  for (const eol of input.newlines) {
    object(eol, ['localOffset', 'kind']); integer(eol.localOffset, input.text.length); oneOf(eol.kind, ['lf', 'cr', 'crlf']);
    if (eol.localOffset <= previous || input.text[eol.localOffset] !== '\n') fail('InvalidTextBoundary');
    eols.set(eol.localOffset, eol.kind); previous = eol.localOffset;
  }
  let byte = decimal(input.byteStart), local = 0;
  const boundaries = new Map<number, bigint>([[0, byte]]);
  for (const scalar of input.text) {
    const cp = scalar.codePointAt(0)!;
    if ((cp >= 0xd800 && cp <= 0xdfff) || scalar === '\r') fail('InvalidTextBoundary');
    const eol = eols.get(local);
    if (scalar === '\n' && !eol) fail('InvalidTextBoundary');
    const width = input.encoding === 'utf8' ? (cp <= 0x7f ? 1 : cp <= 0x7ff ? 2 : cp <= 0xffff ? 3 : 4) : scalar.length * 2;
    byte += BigInt(width * (eol === 'crlf' ? 2 : 1));
    checked(byte); local += scalar.length; boundaries.set(local, byte);
  }
  return boundaries;
}
export function validateLease(lease: Lease): void {
  object(lease, ['schema', 'sessionId', 'sourceGeneration', 'documentRevision', 'viewGeneration', 'leaseId', 'segments', 'line', 'context']);
  if (lease.schema !== SCHEMA) fail();
  id(lease.sessionId); id(lease.sourceGeneration); id(lease.leaseId); decimal(lease.documentRevision); decimal(lease.viewGeneration);
  if (!Array.isArray(lease.segments) || !lease.segments.length || lease.segments.length > LIMITS.segments) fail('ResourceLimit');
  let units = 0, rows = 1, end: bigint | undefined;
  for (const segment of lease.segments) {
    const boundaries = segmentBoundaries(segment);
    if (end !== undefined && end !== decimal(segment.byteStart)) fail('InvalidTextBoundary');
    end = boundaries.get(segment.text.length); units += segment.text.length; rows += segment.newlines.length;
    if (units > LIMITS.projectionUnits || rows > LIMITS.rows) fail('ResourceLimit');
  }
  object(lease.line, ['state', 'count']); oneOf(lease.line.state, ['exact', 'unknown', 'indexing']);
  if (lease.line.state === 'exact' ? lease.line.count === null || decimal(lease.line.count) === 0n : lease.line.count !== null) fail();
  object(lease.context, ['before', 'after', 'continuationId']);
  oneOf(lease.context.before, ['complete', 'continued']); oneOf(lease.context.after, ['complete', 'continued']);
  if (lease.context.before === 'continued' || lease.context.after === 'continued') id(lease.context.continuationId);
  else if (lease.context.continuationId !== null) fail();
}
export function leaseByte(lease: Lease, segment: number, offset: number): string {
  validateLease(lease); integer(segment, lease.segments.length - 1); integer(offset, LIMITS.projectionUnits);
  const byte = segmentBoundaries(lease.segments[segment]).get(offset);
  if (byte === undefined) fail('InvalidTextBoundary');
  return String(byte);
}
export function requireCompleteContext(lease: Lease): void {
  validateLease(lease);
  if (lease.context.before !== 'complete' || lease.context.after !== 'complete') fail('ContextRequired');
}
type LeasePoint = { lease: Lease; segment: number; offset: number; affinity: Affinity };
/** Caller passes authoritative pinned leases, not mappings supplied by a mutation request. */
export function createSelection(state: SessionVersion, selectionId: string, a: LeasePoint, h: LeasePoint): Selection {
  id(selectionId);
  const endpoint = (point: LeasePoint): Endpoint => {
    if (point.lease.sessionId !== state.sessionId) fail('Unauthorized');
    if (point.lease.sourceGeneration !== state.sourceGeneration) fail('SourceChanged');
    if (point.lease.documentRevision !== state.documentRevision) fail('StaleRevision');
    oneOf(point.affinity, ['before', 'after']);
    return { byte: leaseByte(point.lease, point.segment, point.offset), affinity: point.affinity };
  };
  const anchor = endpoint(a), head = endpoint(h);
  return { ...state, selectionId, anchor, head, direction: decimal(anchor.byte) > decimal(head.byte) ? 'backward' : 'forward',
    state: 'active', expiresAfterRevision: String(decimal(state.documentRevision) > MAX - 256n ? MAX : decimal(state.documentRevision) + 256n) };
}
/** Arithmetic only, not a wire validator. Caller owns the document and must first
 * validate endpoints/edit boundaries against its encoding and CRLF map. */
export function remapEndpoint(endpoint: Endpoint, from: Decimal, to: Decimal, insertedBytes: Decimal): Endpoint {
  object(endpoint, ['byte', 'affinity']); oneOf(endpoint.affinity, ['before', 'after']);
  const p = decimal(endpoint.byte), start = decimal(from), end = decimal(to), inserted = decimal(insertedBytes);
  if (end < start) fail();
  const next = p < start ? p : p > end ? p - (end - start) + inserted : start + (endpoint.affinity === 'after' ? inserted : 0n);
  return { byte: checked(next), affinity: endpoint.affinity };
}
export function resolvePoint(point: WirePoint, lease: Lease): LeasePoint {
  if (point.leaseId !== lease.leaseId || point.viewGeneration !== lease.viewGeneration) fail('StaleRevision');
  leaseByte(lease, point.segment, point.offset); oneOf(point.affinity, ['before', 'after']);
  return { lease, segment: point.segment, offset: point.offset, affinity: point.affinity };
}
export function remapSelection(selection: Selection, state: SessionVersion, from: Decimal, to: Decimal, insertedBytes: Decimal): Selection {
  if (selection.sessionId !== state.sessionId) fail('Unauthorized');
  if (selection.sourceGeneration !== state.sourceGeneration) fail('SourceChanged');
  if (decimal(state.documentRevision) <= decimal(selection.documentRevision)) fail('StaleRevision');
  if (selection.state === 'invalidated' || decimal(state.documentRevision) > decimal(selection.expiresAfterRevision)) return { ...selection, ...state, state: 'invalidated' };
  if (decimal(state.documentRevision) !== decimal(selection.documentRevision) + 1n) fail('StaleRevision');
  const anchor = remapEndpoint(selection.anchor, from, to, insertedBytes), head = remapEndpoint(selection.head, from, to, insertedBytes);
  return { ...selection, ...state, anchor, head, direction: decimal(anchor.byte) > decimal(head.byte) ? 'backward' : 'forward' };
}

/** Bounded protocol oracle. Production persistence/retirement is P05, never eviction. */
export class OperationLedger {
  private active = new Map<string, { digest: string; receipt: Receipt }>();
  private retired = new Set<string>();
  constructor(private readonly capacity = LIMITS.operations) { integer(capacity, LIMITS.operations); if (!capacity) fail(); }
  begin(operationId: string, digest: string): Receipt | null {
    id(operationId); if (!/^[0-9a-f]{64}$/.test(digest)) fail();
    if (this.retired.has(operationId)) fail('RetryRetired');
    const existing = this.active.get(operationId);
    if (existing) { if (existing.digest !== digest) fail(); return { ...existing.receipt }; }
    if (this.active.size >= this.capacity) fail('ResourceLimit');
    this.active.set(operationId, { digest, receipt: { state: 'pending' } }); return null;
  }
  finish(operationId: string, receipt: Exclude<Receipt, { state: 'pending' }>): void {
    const record = this.active.get(operationId);
    if (!record || record.receipt.state !== 'pending') fail();
    if (receipt.state === 'accepted') { object(receipt, ['state', 'documentRevision']); decimal(receipt.documentRevision); }
    else if (receipt.state === 'rejected') { object(receipt, ['state', 'code']); oneOf(receipt.code, ERRORS); }
    else fail();
    record.receipt = { ...receipt };
  }
  accepted(operationId: string, revision: Decimal): boolean {
    const receipt = this.active.get(operationId)?.receipt;
    return receipt?.state === 'accepted' && decimal(receipt.documentRevision) <= decimal(revision);
  }
  retire(operationId: string, durableRevision: Decimal): void {
    if (!this.accepted(operationId, durableRevision)) fail('StaleRevision');
    if (this.retired.size >= this.capacity) fail('ResourceLimit');
    this.retired.add(operationId); this.active.delete(operationId);
  }
}
export function assertBarrier(state: SessionVersion, barrier: Barrier, ledger: OperationLedger): void {
  validateBarrier(barrier);
  if (barrier.documentRevision !== state.documentRevision || barrier.inputSequence !== state.inputSequence
    || barrier.operationIds.some(operationId => !ledger.accepted(operationId, state.documentRevision))) fail('StaleRevision');
}
function validateBarrier(barrier: Barrier) {
  object(barrier, ['documentRevision', 'inputSequence', 'operationIds']); decimal(barrier.documentRevision); decimal(barrier.inputSequence);
  if (!Array.isArray(barrier.operationIds) || barrier.operationIds.length > LIMITS.pendingEdits) fail('ResourceLimit');
  barrier.operationIds.forEach(id); if (new Set(barrier.operationIds).size !== barrier.operationIds.length) fail();
}

export type Insertion = { kind: 'inline'; text: string } | { kind: 'spool'; spoolId: string };
export type Request = { schema: typeof SCHEMA; sessionId: string; sourceGeneration: string; requestId: string } & (
  | { command: 'get_stack_text_session' }
  | { command: 'read_stack_text_window'; byte: Decimal; viewGeneration: Decimal }
  | { command: 'cancel_stack_text_job'; jobId: string }
  | { command: 'release_stack_text_selection'; selectionId: string }
  | { command: 'continue_stack_text_context'; continuationId: string; viewGeneration: Decimal }
  | { command: 'close_stack_text_document'; barrier: Barrier; disposition: 'discard' | 'cancel' }
  | { command: 'reload_stack_text_document'; barrier: Barrier; disposition: 'discard' | 'cancel'; encoding: Encoding }
  | { command: 'save_stack_text_document' | 'undo_stack_text_edit' | 'redo_stack_text_edit'; barrier: Barrier; operationId: string }
  | { command: 'save_stack_text_document_as'; barrier: Barrier; operationId: string; destination: string }
  | { command: 'search_stack_text_document'; barrier: Barrier; query: string; cursor: string | null }
  | { command: 'create_stack_text_selection'; barrier: Barrier; anchor: WirePoint; head: WirePoint }
  | { command: 'export_stack_text_selection'; barrier: Barrier; selectionId: string; operationId: string; mode: 'copy' | 'cut' | 'export' }
  | { command: 'apply_stack_text_edits' | 'replace_stack_text_matches' | 'import_stack_text_clipboard'; barrier: Barrier; operationId: string; selectionId: string; insertion: Insertion }
);
export interface WirePoint { leaseId: string; viewGeneration: Decimal; segment: number; offset: number; affinity: Affinity }
export interface OpenRequest { schema: typeof SCHEMA; requestId: string; path: string; encoding: Encoding | null }
export function validateOpen(input: OpenRequest): void {
  object(input, ['schema', 'requestId', 'path', 'encoding']);
  if (input.schema !== SCHEMA) fail(); id(input.requestId); scalarText(input.path, 32767);
  if (!input.path || input.path.includes('\0')) fail();
  if (input.encoding !== null) oneOf(input.encoding, ['utf8', 'utf16le', 'utf16be']);
}

export interface Session extends SessionVersion {
  sourceBytes: Decimal; encoding: Encoding; bomBytes: 0 | 2 | 3; insertedEol: 'lf' | 'cr' | 'crlf';
  dirty: boolean; savedRevision: Decimal; durableRevision: Decimal; readOnlyReason: ErrorCode | null;
}
export interface Failure { code: ErrorCode; retry: 'never' | 'same-request' | 'new-request';
  disposition: 'unchanged' | 'retained' | 'ambiguous'; operationId: string | null; jobId: string | null }
export type WireResult = { schema: typeof SCHEMA; requestId: string; sessionId: string | null; sourceGeneration: string | null } & (
  | { kind: 'session'; data: Session } | { kind: 'lease'; data: Lease } | { kind: 'selection'; data: Selection }
  | { kind: 'receipt'; data: Receipt } | { kind: 'job'; data: Job } | { kind: 'error'; data: Failure }
  | { kind: 'search'; data: { selectionIds: string[]; cursor: string | null; complete: boolean } }
  | { kind: 'transfer'; data: { spoolId: string; bytes: Decimal; sha256: string; state: 'staged' | 'ready' } }
  | { kind: 'released' | 'closed'; data: null }
);
const ERRORS: readonly ErrorCode[] = ['Unauthorized', 'UnsupportedTarget', 'EncodingRequired', 'InvalidTextBoundary', 'StaleRevision', 'SourceChanged', 'SharingViolation', 'Readonly', 'ResourceLimit', 'Cancelled', 'IoFailure', 'PublicationAmbiguous', 'InvalidRequest', 'RetryRetired', 'ContextRequired'];
export function validateResult(result: WireResult): void {
  object(result, ['schema', 'requestId', 'sessionId', 'sourceGeneration', 'kind', 'data']);
  if (result.schema !== SCHEMA) fail(); id(result.requestId);
  if (result.sessionId !== null) id(result.sessionId); if (result.sourceGeneration !== null) id(result.sourceGeneration);
  if ((result.sessionId === null) !== (result.sourceGeneration === null) || (result.kind !== 'error' && result.sessionId === null)) fail();
  const d = result.data;
  const owner = (value: { sessionId: string; sourceGeneration: string }) => {
    if (value.sessionId !== result.sessionId || value.sourceGeneration !== result.sourceGeneration) fail('Unauthorized');
  };
  switch (result.kind) {
    case 'lease': validateLease(result.data); owner(result.data); break;
    case 'session': {
      const s = result.data;
      object(s, ['sessionId', 'sourceGeneration', 'documentRevision', 'inputSequence', 'sourceBytes', 'encoding', 'bomBytes', 'insertedEol', 'dirty', 'savedRevision', 'durableRevision', 'readOnlyReason']); owner(s);
      decimal(s.inputSequence); decimal(s.sourceBytes); const revision = decimal(s.documentRevision);
      if (decimal(s.savedRevision) > revision || decimal(s.durableRevision) > revision || typeof s.dirty !== 'boolean') fail();
      oneOf(s.encoding, ['utf8', 'utf16le', 'utf16be']); oneOf(s.insertedEol, ['lf', 'cr', 'crlf']);
      if (s.bomBytes !== 0 && s.bomBytes !== (s.encoding === 'utf8' ? 3 : 2)) fail();
      if (s.readOnlyReason !== null) oneOf(s.readOnlyReason, ERRORS); break;
    }
    case 'selection': {
      const s = result.data;
      object(s, ['sessionId', 'sourceGeneration', 'documentRevision', 'inputSequence', 'selectionId', 'anchor', 'head', 'direction', 'state', 'expiresAfterRevision']); owner(s);
      id(s.selectionId); decimal(s.documentRevision); decimal(s.inputSequence); decimal(s.expiresAfterRevision);
      for (const point of [s.anchor, s.head]) { object(point, ['byte', 'affinity']); decimal(point.byte); oneOf(point.affinity, ['before', 'after']); }
      oneOf(s.state, ['active', 'invalidated']); oneOf(s.direction, ['forward', 'backward']);
      if (s.state === 'active' && (decimal(s.expiresAfterRevision) < decimal(s.documentRevision) || s.direction !== (decimal(s.anchor.byte) > decimal(s.head.byte) ? 'backward' : 'forward'))) fail(); break;
    }
    case 'receipt': {
      const r = result.data;
      if (r.state === 'accepted') { object(r, ['state', 'documentRevision']); decimal(r.documentRevision); }
      else if (r.state === 'rejected') { object(r, ['state', 'code']); oneOf(r.code, ERRORS); }
      else { object(r, ['state']); if (r.state !== 'pending') fail(); } break;
    }
    case 'job': {
      const j = result.data;
      object(j, ['jobId', 'sessionId', 'sourceGeneration', 'documentRevision', 'phase', 'completedBytes', 'totalBytes', 'cancellation']); owner(j); id(j.jobId); decimal(j.documentRevision);
      const completed = decimal(j.completedBytes); if (j.totalBytes !== null && completed > decimal(j.totalBytes)) fail();
      oneOf(j.phase, ['queued', 'reading', 'staging', 'flushing', 'publishing', 'published', 'durable', 'failed', 'cancelled', 'ambiguous']);
      oneOf(j.cancellation, ['none', 'requested', 'removed', 'acknowledged', 'already-published', 'ambiguous']);
      if (['published', 'durable'].includes(j.phase) && !['none', 'already-published'].includes(j.cancellation)) fail();
      if (['removed', 'acknowledged'].includes(j.cancellation) && j.phase !== 'cancelled') fail();
      if ((j.phase === 'ambiguous') !== (j.cancellation === 'ambiguous')) fail(); break;
    }
    case 'error': {
      const e = result.data; object(e, ['code', 'retry', 'disposition', 'operationId', 'jobId']);
      oneOf(e.code, ERRORS); oneOf(e.retry, ['never', 'same-request', 'new-request']); oneOf(e.disposition, ['unchanged', 'retained', 'ambiguous']);
      if (e.operationId !== null) id(e.operationId); if (e.jobId !== null) id(e.jobId);
      if ((e.code === 'PublicationAmbiguous') !== (e.disposition === 'ambiguous') || (e.code === 'PublicationAmbiguous' && e.retry !== 'never')) fail(); break;
    }
    case 'search': {
      const s = result.data; object(s, ['selectionIds', 'cursor', 'complete']);
      if (!Array.isArray(s.selectionIds) || s.selectionIds.length > LIMITS.results) fail('ResourceLimit'); s.selectionIds.forEach(id);
      if (new Set(s.selectionIds).size !== s.selectionIds.length || typeof s.complete !== 'boolean' || s.complete !== (s.cursor === null)) fail();
      if (s.cursor !== null) id(s.cursor); break;
    }
    case 'transfer': {
      const t = result.data; object(t, ['spoolId', 'bytes', 'sha256', 'state']); id(t.spoolId); decimal(t.bytes);
      if (!/^[0-9a-f]{64}$/.test(t.sha256)) fail(); oneOf(t.state, ['staged', 'ready']); break;
    }
    case 'released': case 'closed': if (d !== null) fail(); break;
    default: fail();
  }
}
const COMMAND_FIELDS: Record<Request['command'], string[]> = {
  get_stack_text_session: [], read_stack_text_window: ['byte', 'viewGeneration'], cancel_stack_text_job: ['jobId'],
  release_stack_text_selection: ['selectionId'], continue_stack_text_context: ['continuationId', 'viewGeneration'],
  close_stack_text_document: ['barrier', 'disposition'], reload_stack_text_document: ['barrier', 'disposition', 'encoding'],
  save_stack_text_document: ['barrier', 'operationId'], save_stack_text_document_as: ['barrier', 'operationId', 'destination'],
  undo_stack_text_edit: ['barrier', 'operationId'], redo_stack_text_edit: ['barrier', 'operationId'],
  search_stack_text_document: ['barrier', 'query', 'cursor'], create_stack_text_selection: ['barrier', 'anchor', 'head'],
  export_stack_text_selection: ['barrier', 'selectionId', 'operationId', 'mode'],
  apply_stack_text_edits: ['barrier', 'operationId', 'selectionId', 'insertion'],
  replace_stack_text_matches: ['barrier', 'operationId', 'selectionId', 'insertion'],
  import_stack_text_clipboard: ['barrier', 'operationId', 'selectionId', 'insertion']
};
function scalarText(value: unknown, max: number) {
  if (typeof value !== 'string' || value.length > max) fail('ResourceLimit');
  for (const scalar of value) { const cp = scalar.codePointAt(0)!; if (cp >= 0xd800 && cp <= 0xdfff) fail('InvalidTextBoundary'); }
}
export function validateRequest(input: Request): void {
  if (!input || !Object.hasOwn(COMMAND_FIELDS, input.command)) fail();
  const fields = COMMAND_FIELDS[input.command];
  object(input, ['schema', 'sessionId', 'sourceGeneration', 'requestId', 'command', ...fields]);
  if (input.schema !== SCHEMA) fail(); id(input.sessionId); id(input.sourceGeneration); id(input.requestId);
  for (const field of fields) {
    const value = (input as unknown as Record<string, unknown>)[field];
    if (field === 'barrier') validateBarrier(value as Barrier);
    else if (['byte', 'viewGeneration'].includes(field)) decimal(value);
    else if (['jobId', 'operationId', 'selectionId', 'continuationId'].includes(field)) id(value);
    else if (field === 'disposition') oneOf(value, ['discard', 'cancel']);
    else if (field === 'encoding') oneOf(value, ['utf8', 'utf16le', 'utf16be']);
    else if (field === 'mode') oneOf(value, ['copy', 'cut', 'export']);
    else if (field === 'cursor') { if (value !== null) id(value); }
    else if (field === 'destination') { scalarText(value, 32767); if (!value || (value as string).includes('\0')) fail(); }
    else if (field === 'query') { scalarText(value, 4096); if (!value) fail(); }
    else if (field === 'anchor' || field === 'head') {
      const point = value as WirePoint; object(point, ['leaseId', 'viewGeneration', 'segment', 'offset', 'affinity']);
      id(point.leaseId); decimal(point.viewGeneration); integer(point.segment, LIMITS.segments - 1); integer(point.offset, LIMITS.projectionUnits); oneOf(point.affinity, ['before', 'after']);
    } else if (field === 'insertion') {
      const insertion = value as Insertion;
      if (insertion?.kind === 'inline') { object(insertion, ['kind', 'text']); scalarText(insertion.text, LIMITS.pendingBytes / 2); }
      else { object(insertion, ['kind', 'spoolId']); if (insertion.kind !== 'spool') fail(); id(insertion.spoolId); }
    }
  }
}

/** Canonical ASCII JSON bytes; SHA-256 computed by the authorized recipient, never trusted from caller. */
export function canonicalRequest(input: Request): string {
  validateRequest(input);
  const sorted = (value: unknown): unknown => Array.isArray(value) ? value.map(sorted)
    : value && typeof value === 'object' ? Object.fromEntries(Object.keys(value).sort().map(key => [key, sorted((value as Record<string, unknown>)[key])])) : value;
  return JSON.stringify(sorted(input)).replace(/[\u007f-\uffff]/g, c => `\\u${c.charCodeAt(0).toString(16).padStart(4, '0')}`);
}
