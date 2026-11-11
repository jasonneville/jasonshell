import assert from 'node:assert/strict';
import test from 'node:test';
import { classifyProcessGroup, sortProcesses, orderProcessRefresh } from '../dist-tests/lib/processManagerState.js';
import { countedPids, legacyOrdering, processSortFixture } from './fixtures/processManagerSort.fixture.mjs';

const legacy = legacyOrdering(classifyProcessGroup);
const sort = { column: 'cpuPercent', direction: 'desc' };
const pids = (rows) => rows.map((row) => row.pid);

test('sort prepares taskbar PID membership at most once, not per comparison', () => {
  const rows = processSortFixture();
  const observed = countedPids([1, 5, 7, 13]);
  const result = sortProcesses(rows, sort, { taskbarActivePids: observed.iterable });
  assert.deepEqual(result, legacy.sortProcesses(rows, sort, { taskbarActivePids: [1, 5, 7, 13] }));
  assert.ok(observed.counts.iterations <= 1, `membership rebuilt ${observed.counts.iterations} times`);
});

test('refresh shares one taskbar membership preparation across both ordering passes', () => {
  const rows = processSortFixture();
  const previous = rows.slice(10).reverse();
  const observed = countedPids([1, 5, 7, 13]);
  const result = orderProcessRefresh(previous, rows, sort, {
    preserveExistingOrder: true, taskbarActivePids: observed.iterable
  });
  assert.deepEqual(result, legacy.orderProcessRefresh(previous, rows, sort, {
    preserveExistingOrder: true, taskbarActivePids: [1, 5, 7, 13]
  }));
  assert.ok(observed.counts.iterations <= 1, `membership rebuilt ${observed.counts.iterations} times`);
});

test('all columns/directions preserve legacy groups, ties, missing metrics and refresh order', () => {
  const rows = processSortFixture(64);
  const previous = rows.filter((_, index) => index % 3 !== 0).reverse();
  for (const column of ['name', 'pid', 'cpuPercent', 'memoryBytes', 'gpuPercent', 'threadCount', 'startTimeMs']) {
    for (const direction of ['asc', 'desc']) {
      for (const active of [[], [1, 7, 13, 13], new Set([2, 4, 6])]) {
        const options = { taskbarActivePids: active };
        const state = { column, direction };
        assert.deepEqual(sortProcesses(rows, state, options), legacy.sortProcesses(rows, state, options));
        for (const preserveExistingOrder of [false, true]) {
          const refreshOptions = { ...options, preserveExistingOrder };
          assert.deepEqual(orderProcessRefresh(previous, rows, state, refreshOptions),
            legacy.orderProcessRefresh(previous, rows, state, refreshOptions));
        }
      }
    }
  }
});

test('sort and refresh never mutate caller arrays or process rows', () => {
  const rows = Object.freeze(processSortFixture(32).map(Object.freeze));
  const previous = Object.freeze(rows.slice().reverse());
  const options = Object.freeze({ preserveExistingOrder: true, taskbarActivePids: Object.freeze([1, 3, 7]) });
  const before = pids(rows);
  const oldBefore = pids(previous);
  for (const result of [sortProcesses(rows, sort, options), orderProcessRefresh(previous, rows, sort, options)]) {
    assert.notStrictEqual(result, rows);
    assert.notStrictEqual(result, previous);
    for (const row of result) assert.strictEqual(row, rows.find((item) => item.pid === row.pid));
  }
  assert.deepEqual(pids(rows), before);
  assert.deepEqual(pids(previous), oldBefore);
});

test('empty, singleton and first refresh retain value and ownership semantics', () => {
  const singleton = processSortFixture(1);
  assert.deepEqual(sortProcesses([], sort), []);
  assert.deepEqual(orderProcessRefresh(singleton, [], sort, { preserveExistingOrder: true }), []);
  assert.deepEqual(orderProcessRefresh([], singleton, sort, { preserveExistingOrder: true }), singleton);
  assert.notStrictEqual(sortProcesses(singleton, sort), singleton);
});

test('taskbar options changed between calls are never cached across invocations', () => {
  const rows = [{ ...processSortFixture(1)[0], pid: 1, name: 'svchost.exe', taskbarActive: false },
    { ...processSortFixture(1)[0], pid: 2, name: 'tool', taskbarActive: false }];
  const active = [1];
  const options = { taskbarActivePids: active, preserveExistingOrder: true };
  assert.equal(classifyProcessGroup(rows[0], options), 'applications');
  assert.deepEqual(pids(sortProcesses(rows, sort, options)), [1, 2]);
  active.splice(0, 1, 2);
  assert.equal(classifyProcessGroup(rows[0], options), 'windows');
  assert.deepEqual(pids(sortProcesses(rows, sort, options)), [2, 1]);
  assert.deepEqual(pids(orderProcessRefresh(rows, rows, sort, options)), [2, 1]);
  options.taskbarActivePids = [1];
  assert.deepEqual(pids(orderProcessRefresh(rows, rows, sort, options)), [1, 2]);
});

test('active overrides Windows classification; equal known metrics retain PID tie break', () => {
  const base = processSortFixture(1)[0];
  const rows = [9, 3, 7].map((pid) => ({ ...base, pid, name: 'svchost.exe', taskbarActive: true, cpuPercent: 1 }));
  assert.deepEqual(pids(sortProcesses(rows, sort)), [3, 7, 9]);
  assert.equal(classifyProcessGroup(rows[0]), 'applications');
  assert.equal(classifyProcessGroup({ ...rows[0], taskbarActive: false }), 'windows');
});
