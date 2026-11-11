import assert from 'node:assert/strict';
import test from 'node:test';
import { sequenceTracker, setOracle } from './fixtures/terminalRetention.fixture.mjs';

test('lossless tracker agrees with string-key Set over sparse, reverse, interleaved and duplicate arrivals', async () => {
  const tracker = await sequenceTracker();
  const oracle = setOracle();
  let state = 117;
  for (let i = 0; i < 20000; i += 1) {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    const session = `session-${state % 3}`;
    const stream = [undefined, 'stdout', 'stderr', 'system'][(state >>> 5) % 4];
    const sequence = (state >>> 8) % 2000;
    assert.equal(tracker.accept(session, stream, sequence), oracle(session, stream, sequence), `${session}/${stream}/${sequence}`);
  }
});

test('range bridging never loses holes, reverse arrivals, or stream/session independence', async () => {
  const tracker = await sequenceTracker();
  for (const sequence of [10, 12, 14, 13, 11, 9, 0, -1, 8, 7]) assert.equal(tracker.accept('a', 'stdout', sequence), true);
  for (const sequence of [10, 12, 14, 13, 11, 9, 0, -1, 8, 7]) assert.equal(tracker.accept('a', undefined, sequence), false);
  assert.equal(tracker.accept('a', 'stdout', 6), true);
  assert.equal(tracker.accept('a', 'stderr', 10), true);
  assert.equal(tracker.accept('b', 'stdout', 10), true);
});

test('exceptional numeric and unsequenced values preserve existing exact string-key behavior', async () => {
  const tracker = await sequenceTracker();
  const oracle = setOracle();
  const values = [NaN, Infinity, -Infinity, 1.5, -1.5, -0, 0, Number.MIN_SAFE_INTEGER - 1, Number.MIN_SAFE_INTEGER, Number.MIN_SAFE_INTEGER + 1, Number.MAX_SAFE_INTEGER - 1, Number.MAX_SAFE_INTEGER, Number.MAX_SAFE_INTEGER + 1, undefined, null, '1'];
  for (let repeat = 0; repeat < 3; repeat += 1) for (const sequence of values) {
    assert.equal(tracker.accept('a', undefined, sequence), oracle('a', undefined, sequence), String(sequence));
  }
});

test('increasing contiguous workloads retain one interval with zero fallback keys', async (t) => {
  for (const chunks of [100, 1000, 10000]) {
    const tracker = await sequenceTracker();
    for (let sequence = 0; sequence < chunks; sequence += 1) assert.equal(tracker.accept('a', 'stdout', sequence), true);
    const stats = tracker.stats();
    assert.equal(stats.rangeCount, 1);
    assert.equal(stats.fallbackKeyCount, 0);
    t.diagnostic(JSON.stringify({ chunks, ...stats }));
    for (let sequence = chunks - 1; sequence >= 0; sequence -= 1) assert.equal(tracker.accept('a', 'stdout', sequence), false);
  }
});

test('fragmentation remains lossless rather than silently evicting old sparse keys', async () => {
  const tracker = await sequenceTracker();
  for (let sequence = 0; sequence < 10000; sequence += 2) tracker.accept('a', 'stdout', sequence);
  assert.equal(tracker.stats().rangeCount, 5000);
  for (let sequence = 1; sequence < 10000; sequence += 2) assert.equal(tracker.accept('a', 'stdout', sequence), true);
  assert.equal(tracker.stats().rangeCount, 1);
  assert.equal(tracker.accept('a', 'stdout', 0), false);
});
