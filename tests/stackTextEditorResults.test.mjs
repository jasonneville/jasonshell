import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { SCHEMA, validateOpen, validateResult, remapSelection, resolvePoint } from '../dist-tests/features/stack-browser/textEditorProtocol.js';

test('shared result wire vectors agree with Rust including explicit nulls', () => {
  const vectors = JSON.parse(readFileSync(new URL('./fixtures/stack-text-results-v2.json', import.meta.url)));
  const envelope = { schema: SCHEMA, requestId: 'q1', sessionId: 's1', sourceGeneration: 'g1' };
  for (const body of vectors.valid) validateResult({ ...envelope, ...body });
  for (const body of vectors.invalid) assert.throws(() => validateResult({ ...envelope, ...body }), JSON.stringify(body));
});

test('open has exact explicit encoding, bounded path and no caller authority', () => {
  const request = { schema: SCHEMA, requestId: 'q1', path: 'C:\\test.txt', encoding: null };
  validateOpen(request);
  for (const bad of [{ ...request, windowLabel: 'stack-popup' }, { ...request, path: 'a\0b' }, { ...request, encoding: 'auto' }]) assert.throws(() => validateOpen(bad));
  delete request.encoding; assert.throws(() => validateOpen(request));
});
test('result error and job schemas reject ambiguity and wrong nullable fields', () => {
  const envelope = { schema: SCHEMA, requestId: 'q1', sessionId: 's1', sourceGeneration: 'g1' };
  const error = { ...envelope, kind: 'error', data: { code: 'IoFailure', retry: 'same-request', disposition: 'retained', operationId: null, jobId: null } };
  validateResult(error);
  assert.throws(() => validateResult({ ...error, data: { ...error.data, text: 'private' } }));
  const job = { ...envelope, kind: 'job', data: { jobId: 'j1', sessionId: 's1', sourceGeneration: 'g1', documentRevision: '0', phase: 'published', completedBytes: '10', totalBytes: '10', cancellation: 'already-published' } };
  validateResult(job);
  assert.throws(() => validateResult({ ...job, data: { ...job.data, cancellation: 'acknowledged' } }));
  assert.throws(() => validateResult({ ...job, data: { ...job.data, sessionId: 'forged' } }));
  assert.throws(() => validateResult({ ...job, data: { ...job.data, completedBytes: '11' } }));
});
test('selection remaps both endpoints and expires rather than reviving invalidated handles', () => {
  const selection = { sessionId: 's1', sourceGeneration: 'g1', documentRevision: '0', inputSequence: '0', selectionId: 'r1', anchor: { byte: '9', affinity: 'after' }, head: { byte: '2', affinity: 'before' }, direction: 'backward', state: 'active', expiresAfterRevision: '2' };
  const state = { sessionId: 's1', sourceGeneration: 'g1', documentRevision: '1', inputSequence: '1' };
  const mapped = remapSelection(selection, state, '3', '5', '1');
  assert.equal(mapped.anchor.byte, '8'); assert.equal(mapped.head.byte, '2'); assert.equal(mapped.direction, 'backward');
  assert.equal(remapSelection(mapped, { ...state, documentRevision: '3' }, '0', '0', '0').state, 'invalidated');
  assert.throws(() => remapSelection(selection, { ...state, sourceGeneration: 'g2' }, '0', '0', '0'));
});
test('wire endpoint resolves only through matching authoritative lease view', () => {
  const lease = { schema: SCHEMA, sessionId: 's1', sourceGeneration: 'g1', documentRevision: '0', viewGeneration: '2', leaseId: 'l1', segments: [{ encoding: 'utf8', byteStart: '0', text: 'a', newlines: [] }], line: { state: 'exact', count: '1' }, context: { before: 'complete', after: 'complete', continuationId: null } };
  const point = { leaseId: 'l1', viewGeneration: '2', segment: 0, offset: 1, affinity: 'after' };
  assert.equal(resolvePoint(point, lease).offset, 1);
  assert.throws(() => resolvePoint({ ...point, viewGeneration: '1' }, lease));
  assert.throws(() => resolvePoint({ ...point, leaseId: 'other' }, lease));
});
