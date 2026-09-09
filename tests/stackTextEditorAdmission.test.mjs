import test from 'node:test';
import assert from 'node:assert/strict';
import { requiredQuota, SpoolQuota, InputQueue, Scheduler, ByteOracle, transferSelection } from '../scripts/stack-text-editor/harness.mjs';
test('checked quota includes every retained asset; unknown and overflow refuse', () => {
  const parts = { source: '9007199254740993', addStore: '1', staging: '2', backup: '3', overhead: '4' };
  assert.equal(requiredQuota(parts), 9007199254741003n);
  assert.throws(() => requiredQuota({ ...parts, source: '18446744073709551615' }));
  assert.throws(() => requiredQuota({ ...parts, overhead: null }));
  const quota = new SpoolQuota();
  for (let n = 0; n < 4; n++) quota.reserve(`s${n}`, 67108864);
  assert.throws(() => quota.reserve('s4', 1)); assert.equal(quota.total, 268435456);
  quota.release('s0'); quota.release('s0'); assert.equal(quota.total, 201326592);
});
test('input cap refuses new admission, preserves rejected text, requires spool for bulk', () => {
  const queue = new InputQueue(); queue.admit('o1', 'x'.repeat(524288));
  assert.throws(() => queue.admit('o2', 'y')); assert.throws(() => queue.admit('big', 'x'.repeat(524289)), /SpoolRequired/);
  const next = queue.next(); assert.equal(next.text.length, 524288); assert.equal(queue.next(), null);
  assert.equal(queue.settle('o1', false), 'retained'); assert.equal(queue.bytes, 1048576); assert.equal(queue.drained, false);
  queue.settle('o1', true); assert.equal(queue.drained, true);
  for (let n = 0; n < 64; n++) queue.admit(`n${n}`, '');
  assert.throws(() => queue.admit('full', ''));
});
test('interactive and demand outrank background; bounded jobs cancel and close', () => {
  const scheduler = new Scheduler(); scheduler.enqueue('background', 'background'); scheduler.enqueue('seek', 'demand'); scheduler.enqueue('edit', 'interactive');
  assert.equal(scheduler.next().id, 'edit'); assert.equal(scheduler.next().id, 'seek');
  scheduler.cancel('background'); assert.equal(scheduler.next(), null);
  for (let n = 0; n < 32; n++) scheduler.enqueue(`j${n}`, 'background');
  assert.throws(() => scheduler.enqueue('full', 'interactive')); scheduler.close(); assert.equal(scheduler.next(), null);
});
test('denied clipboard never cuts; invalid range never reaches clipboard', async () => {
  const oracle = new ByteOracle(Buffer.from('A\u{1f600}Z')); let writes = 0;
  const denied = { write: async () => { writes++; throw Error('ClipboardUnavailable'); } };
  await assert.rejects(transferSelection(oracle, 1, 5, denied, true), /ClipboardUnavailable/);
  assert.equal(oracle.bytes().toString(), 'A\u{1f600}Z');
  await assert.rejects(transferSelection(oracle, 2, 5, denied, true), /InvalidTextBoundary/); assert.equal(writes, 1);
  await transferSelection(oracle, 1, 5, { write: async bytes => assert.equal(bytes.toString(), '\u{1f600}') }, true);
  assert.equal(oracle.bytes().toString(), 'AZ');
});
test('clipboard completion cannot cut after intervening edit or undo', async () => {
  const oracle = new ByteOracle(Buffer.from('abc'));
  let release;
  const pending = transferSelection(oracle, 0, 1, { write: () => new Promise(resolve => { release = resolve; }) }, true);
  oracle.replace(0, 0, Buffer.from('x')); oracle.undo();
  release();
  await assert.rejects(pending, /StaleRevision/);
  assert.equal(oracle.bytes().toString(), 'abc');
});
