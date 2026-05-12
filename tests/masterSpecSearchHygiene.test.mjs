import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const masterSpec = readFileSync(new URL('../master_spec.md', import.meta.url), 'utf8');

function currentSpecBeforeLedger() {
  return masterSpec.split('## Change Ledger')[0] ?? masterSpec;
}

test('current search spec bans stale deferred/coalesced visible query scheduling', () => {
  const currentSpec = currentSpecBeforeLedger();
  assert.match(currentSpec, /For every non-empty query[\s\S]*this is not debounce-coalesced/);
  assert.doesNotMatch(currentSpec, /queueSearchQueryProcessing/);
  assert.doesNotMatch(currentSpec, /zero-delay latest-only/);
  assert.doesNotMatch(currentSpec, /schedules provider work with a zero-delay/);
  assert.doesNotMatch(currentSpec, /schedules `searchEngine` on a zero-delay/);
});
