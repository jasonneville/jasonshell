import assert from 'node:assert/strict';
import { test } from 'node:test';

// Dynamic import keeps each acceptance case visible in RED while the module is absent.
const selection = () => import('../dist-tests/lib/snipSelection.js');
const bounds = { width: 800, height: 600 };
const pointer = (pointerId, x, y, overrides = {}) => ({ pointerId, x, y, button: 0, isPrimary: true, ...overrides });

test('snip normalizes all four drag directions without mutating input', async () => {
  const { normalizeSelection } = await selection();
  for (const [start, end] of [
    [{ x: 10, y: 20 }, { x: 100, y: 80 }],
    [{ x: 100, y: 80 }, { x: 10, y: 20 }],
    [{ x: 100, y: 20 }, { x: 10, y: 80 }],
    [{ x: 10, y: 80 }, { x: 100, y: 20 }]
  ]) {
    const before = structuredClone({ start, end, bounds });
    assert.deepEqual(normalizeSelection(start, end, bounds), { x: 10, y: 20, width: 90, height: 60 });
    assert.deepEqual({ start, end, bounds }, before);
  }
});

test('snip clamps local CSS coordinates, preserves fractional edges, and rejects empty/nonfinite input', async () => {
  const { normalizeSelection } = await selection();
  assert.deepEqual(normalizeSelection({ x: -20, y: 20.25 }, { x: 900, y: 700 }, bounds),
    { x: 0, y: 20.25, width: 800, height: 579.75 });
  for (const end of [{ x: 10, y: 20 }, { x: 10, y: 30 }, { x: 30, y: 20 }, { x: NaN, y: 30 }, { x: Infinity, y: 30 }]) {
    assert.equal(normalizeSelection({ x: 10, y: 20 }, end, bounds), null);
  }
  assert.equal(normalizeSelection({ x: -10, y: -10 }, { x: -1, y: -1 }, bounds), null);
  for (const invalid of [{ width: 0, height: 600 }, { width: -1, height: 600 }, { width: 800, height: NaN }]) {
    assert.equal(normalizeSelection({ x: 1, y: 1 }, { x: 20, y: 20 }, invalid), null);
  }
});

test('snip ignores secondary/right pointers and commits only the original held pointer', async () => {
  const { beginSelection, moveSelection, finishSelection } = await selection();
  assert.equal(beginSelection(pointer(1, 10, 20, { button: 2 }), bounds), null);
  assert.equal(beginSelection(pointer(1, 10, 20, { isPrimary: false }), bounds), null);
  const started = beginSelection(pointer(1, 10, 20), bounds);
  assert.ok(started);
  assert.deepEqual(moveSelection(started, pointer(2, 300, 400), bounds), started);
  assert.deepEqual(finishSelection(started, pointer(2, 300, 400), bounds), { state: started, selection: null });
  const moved = moveSelection(started, pointer(1, 900, 700), bounds);
  assert.deepEqual(finishSelection(moved, pointer(1, 100, 80), bounds),
    { state: null, selection: { x: 10, y: 20, width: 90, height: 60 } });
  assert.deepEqual(finishSelection(started, pointer(1, 10, 20), bounds), { state: null, selection: null });
});

test('snip pointer cancel ignores foreign pointer; Escape/lost capture discard without completion', async () => {
  const { beginSelection, cancelSelection, finishSelection } = await selection();
  const started = beginSelection(pointer(7, 10, 20), bounds);
  assert.ok(started);
  assert.deepEqual(cancelSelection(started, 8), started);
  assert.equal(cancelSelection(started, 7), null);
  assert.equal(cancelSelection(started), null);
  assert.deepEqual(finishSelection(null, pointer(7, 100, 80), bounds), { state: null, selection: null });
});
