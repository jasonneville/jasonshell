import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { canonicalRequest, OperationLedger } from '../dist-tests/features/stack-browser/textEditorProtocol.js';
const fixture = JSON.parse(readFileSync(new URL('./fixtures/stack-text-request-v2.json', import.meta.url)));
test('canonical request bytes match Rust and changed Unicode payload rejects same ID', () => {
  assert.equal(canonicalRequest(fixture.request), fixture.canonical);
  const digest = request => createHash('sha256').update(canonicalRequest(request)).digest('hex');
  const reordered = Object.fromEntries(Object.entries(fixture.request).reverse());
  assert.equal(digest(reordered), digest(fixture.request));
  const ledger = new OperationLedger();
  ledger.begin('o1', digest(fixture.request));
  assert.throws(() => ledger.begin('o1', digest({ ...fixture.request, insertion: { kind: 'inline', text: 'changed' } })));
  assert.throws(() => ledger.finish('o1', { state: 'rejected', code: 'unknown' }));
  assert.throws(() => ledger.finish('o1', { state: 'accepted', documentRevision: '1', text: 'extra' }));
});
