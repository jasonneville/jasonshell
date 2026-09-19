import assert from 'node:assert/strict';
import test from 'node:test';
import { json } from '@codemirror/lang-json';
import { codeFolding, foldEffect, foldable, foldedRanges } from '@codemirror/language';
import { EditorState } from '@codemirror/state';

const stateFor = (doc) => EditorState.create({ doc, extensions: [json(), codeFolding()] });
const rangeOnLine = (state, number = 1) => {
  const line = state.doc.line(number);
  return foldable(state, line.from, line.to);
};
const ranges = (state) => {
  const found = [];
  foldedRanges(state).between(0, state.doc.length, (from, to) => found.push({ from, to }));
  return found;
};

test('JSON folds are parser-derived for nested containers and exclude scalar/empty bodies', () => {
  const nested = stateFor('{\n  "items": [\n    {"escaped": "} ]"}\n  ]\n}');
  assert.deepEqual(nested.sliceDoc(rangeOnLine(nested).from, rangeOnLine(nested).to), '\n  "items": [\n    {"escaped": "} ]"}\n  ]\n');
  assert.ok(rangeOnLine(nested, 2), 'nested array folds');
  for (const doc of ['42', '"scalar"', '{}', '[]']) assert.equal(rangeOnLine(stateFor(doc)), null);
});

test('JSON folded ranges map edits and folding is document-neutral', () => {
  let state = stateFor('{\n  "a": {"b": 1}\n}');
  const fold = rangeOnLine(state);
  const original = state.doc.toString();
  state = state.update({ effects: foldEffect.of(fold) }).state;
  assert.equal(state.doc.toString(), original);
  assert.deepEqual(ranges(state), [fold]);
  state = state.update({ changes: { from: 0, insert: ' ' } }).state;
  assert.deepEqual(ranges(state), [{ from: fold.from + 1, to: fold.to + 1 }]);
  state = state.update({ changes: { from: state.doc.length, insert: '\n' } }).state;
  assert.equal(ranges(state).length, 1);
  state = state.update({ changes: { from: ranges(state)[0].from + 1, insert: ' ' } }).state;
  assert.equal(state.doc.length, original.length + 3);
});

test('malformed JSON stays editable and never receives fabricated top-level structure', () => {
  for (const doc of ['{"a":', '{"a": [1, 2}', '{]']) {
    let state = stateFor(doc);
    assert.equal(rangeOnLine(state), null);
    state = state.update({ changes: { from: state.doc.length, insert: 'x' } }).state;
    assert.equal(state.doc.toString(), `${doc}x`);
  }
});
