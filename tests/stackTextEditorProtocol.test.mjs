import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  decimal, segmentBoundaries, validateLease, leaseByte, createSelection,
  remapEndpoint, requireCompleteContext, OperationLedger, assertBarrier,
  validateRequest, LIMITS
} from '../dist-tests/features/stack-browser/textEditorProtocol.js';

const vectors = JSON.parse(readFileSync(new URL('./fixtures/stack-text-v2.json', import.meta.url)));
const segment = (overrides = {}) => ({ encoding: 'utf8', byteStart: '0', text: 'abc', newlines: [], ...overrides });
const lease = (overrides = {}) => ({ schema: vectors.schema, sessionId: 's1', sourceGeneration: 'g1',
  documentRevision: '0', viewGeneration: '1', leaseId: 'l1', segments: [segment()],
  line: { state: 'unknown', count: null }, context: { before: 'complete', after: 'complete', continuationId: null }, ...overrides });
const state = { sessionId: 's1', sourceGeneration: 'g1', documentRevision: '0', inputSequence: '0' };
const barrier = { documentRevision: '0', inputSequence: '0', operationIds: [] };

test('lossless wire integers reject noncanonical and numeric inputs', () => {
  for (const value of vectors.integers.valid) assert.equal(decimal(value), BigInt(value));
  for (const value of [...vectors.integers.invalid, NaN, Infinity, -1, 0.5, 1]) assert.throws(() => decimal(value));
});
test('shared UTF-8/UTF-16 scalar/CRLF vectors map exact bytes above 2^53', () => {
  for (const { offsets, bytes, ...input } of vectors.segments) {
    assert.deepEqual([...segmentBoundaries(input)], offsets.map((n, i) => [n, BigInt(bytes[i])]));
    assert.throws(() => leaseByte(lease({ segments: [input] }), 0, 2));
  }
});
test('reject malformed scalars, forged newline maps, source overflow, unknown fields', () => {
  for (const input of [segment({ text: '\ud800' }), segment({ text: '\r' }), segment({ text: '\n' }),
    segment({ newlines: [{ localOffset: 0, kind: 'crlf' }] }), segment({ byteStart: '18446744073709551615' }),
    segment({ sourceText: 'secret' }), segment({ encoding: 'latin1' })]) assert.throws(() => segmentBoundaries(input));
  assert.throws(() => segmentBoundaries(segment({ text: '\n', newlines: [{ localOffset: 0, kind: 'lf' }, { localOffset: 0, kind: 'lf' }] })));
});
test('lease bounds count rows across all segments including base row', () => {
  const lines = n => segment({ text: '\n'.repeat(n), newlines: Array.from({ length: n }, (_, localOffset) => ({ localOffset, kind: 'lf' })) });
  validateLease(lease({ segments: [lines(4095)] }));
  assert.throws(() => validateLease(lease({ segments: [lines(4096)] })));
  assert.throws(() => validateLease(lease({ segments: [lines(3000), { ...lines(3000), byteStart: '3000' }] })));
  assert.throws(() => validateLease(lease({ segments: [segment({ text: 'a'.repeat(LIMITS.projectionUnits + 1) })] })));
  assert.throws(() => validateLease(lease({ line: { state: 'unknown', count: '0' } })));
  assert.throws(() => validateLease(lease({ line: { state: 'exact', count: '0' } })));
});
test('one backend selection combines two independent leases and retains direction', () => {
  const a = lease();
  const b = lease({ leaseId: 'l2', viewGeneration: '2', segments: [segment({ byteStart: '9007199254740993' })] });
  const selection = createSelection(state, 'r1', { lease: b, segment: 0, offset: 2, affinity: 'after' }, { lease: a, segment: 0, offset: 1, affinity: 'before' });
  assert.equal(selection.anchor.byte, '9007199254740995');
  assert.equal(selection.head.byte, '1');
  assert.equal(selection.direction, 'backward');
  assert.throws(() => createSelection(state, 'r1', { lease: lease({ sessionId: 'forged' }), segment: 0, offset: 0, affinity: 'before' }, { lease: a, segment: 0, offset: 0, affinity: 'before' }));
  assert.throws(() => createSelection(state, 'r1', { lease: lease({ documentRevision: '1' }), segment: 0, offset: 0, affinity: 'before' }, { lease: a, segment: 0, offset: 0, affinity: 'before' }));
});
test('anchor affinity remaps insertion, deletion and overflow without float conversion', () => {
  assert.deepEqual(remapEndpoint({ byte: '5', affinity: 'before' }, '5', '5', '3'), { byte: '5', affinity: 'before' });
  assert.deepEqual(remapEndpoint({ byte: '5', affinity: 'after' }, '5', '5', '3'), { byte: '8', affinity: 'after' });
  assert.equal(remapEndpoint({ byte: '7', affinity: 'after' }, '5', '10', '2').byte, '7');
  assert.equal(remapEndpoint({ byte: '9007199254740993', affinity: 'before' }, '5', '10', '2').byte, '9007199254740990');
  assert.throws(() => remapEndpoint({ byte: '18446744073709551615', affinity: 'after' }, '0', '0', '1'));
});
test('truncated grapheme context is never certified complete', () => {
  const incomplete = lease({ context: { before: 'complete', after: 'continued', continuationId: 'c1' } });
  validateLease(incomplete);
  assert.throws(() => requireCompleteContext(incomplete));
  assert.throws(() => validateLease(lease({ context: { before: 'complete', after: 'continued', continuationId: null } })));
  requireCompleteContext(lease());
});
test('ledger exact-once, changed payload, rejected retention and safe retirement', () => {
  const ledger = new OperationLedger(2);
  assert.equal(ledger.begin('o1', 'a'.repeat(64)), null);
  assert.deepEqual(ledger.begin('o1', 'a'.repeat(64)), { state: 'pending' });
  assert.throws(() => ledger.begin('o1', 'b'.repeat(64)));
  ledger.finish('o1', { state: 'accepted', documentRevision: '1' });
  assert.deepEqual(ledger.begin('o1', 'a'.repeat(64)), { state: 'accepted', documentRevision: '1' });
  assert.throws(() => ledger.retire('o1', '0'));
  ledger.retire('o1', '1');
  assert.throws(() => ledger.begin('o1', 'a'.repeat(64)));
  ledger.begin('o2', 'b'.repeat(64)); ledger.finish('o2', { state: 'rejected', code: 'StaleRevision' });
  assert.deepEqual(ledger.begin('o2', 'b'.repeat(64)), { state: 'rejected', code: 'StaleRevision' });
});
test('snapshot barrier rejects pending, missing and stale visible input', () => {
  assertBarrier(state, barrier, new OperationLedger());
  assert.throws(() => assertBarrier(state, undefined, new OperationLedger()));
  assert.throws(() => assertBarrier(state, { ...barrier, inputSequence: '1' }, new OperationLedger()));
  assert.throws(() => assertBarrier(state, { ...barrier, operationIds: ['o1'] }, new OperationLedger()));
});
test('all snapshot-dependent requests require barrier; strict insertion variants', () => {
  const base = { schema: vectors.schema, sessionId: 's1', sourceGeneration: 'g1', requestId: 'q1' };
  const request = { ...base, command: 'close_stack_text_document', barrier, disposition: 'cancel' };
  validateRequest(request);
  const { barrier: _, ...missing } = request;
  assert.throws(() => validateRequest(missing));
  assert.throws(() => validateRequest({ ...request, owner: 'stack-popup' }));
  assert.throws(() => validateRequest({ ...request, schema: 'stack-text-editor.v1' }));
});
