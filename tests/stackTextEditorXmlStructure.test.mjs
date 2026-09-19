import assert from 'node:assert/strict';
import test from 'node:test';
import { xml } from '@codemirror/lang-xml';
import { codeFolding, foldEffect, foldable, foldedRanges, syntaxTree } from '@codemirror/language';
import { EditorState } from '@codemirror/state';

const stateFor = (doc) => EditorState.create({ doc, extensions: [xml({ autoCloseTags: true }), codeFolding()] });
const foldAt = (state, lineNo = 1) => { const line = state.doc.line(lineNo); return foldable(state, line.from, line.to); };
const nodeNames = (state) => { const names = []; syntaxTree(state).iterate({ enter: n => names.push(n.name) }); return names; };

test('XML parser recognizes namespace structures and ignores non-element pseudo-tags', () => {
  const doc = '<?xml version="1.0"?>\n<root xmlns="urn:d" xmlns:x="urn:x" x:id="1">\n<!-- <fake> -->\n<?work <fake>?>\n<![CDATA[<fake> &amp;]]>\n<x:item entity="&quot;"><x:item/></x:item>\n</root>';
  const state = stateFor(doc);
  const names = nodeNames(state);
  assert.ok(names.includes('Element') && names.includes('ProcessingInst') && names.includes('Cdata'));
  assert.ok(foldAt(state, 2), 'root element folds');
  assert.equal(foldAt(state, 3), null, 'comment creates no fold');
  assert.equal(foldAt(state, 4), null, 'PI creates no fold');
  assert.equal(foldAt(state, 5), null, 'CDATA creates no fold');
  assert.equal(foldAt(state, 6), null, 'single-line/self-closing body creates no line fold');
});

test('XML fold state maps before/inside/after edits without changing document', () => {
  let state = stateFor('<r>\n  <x>text</x>\n</r>');
  const fold = foldAt(state);
  const doc = state.doc.toString();
  state = state.update({ effects: foldEffect.of(fold) }).state;
  assert.equal(state.doc.toString(), doc);
  state = state.update({ changes: { from: 0, insert: ' ' } }).state;
  const mapped = []; foldedRanges(state).between(0, state.doc.length, (from, to) => mapped.push({ from, to }));
  assert.deepEqual(mapped, [{ from: fold.from + 1, to: fold.to + 1 }]);
  state = state.update({ changes: { from: mapped[0].from + 1, insert: 'z' } }).state;
  state = state.update({ changes: { from: state.doc.length, insert: '\n' } }).state;
  assert.equal(state.doc.toString(), ` ${doc.slice(0, fold.from + 1)}z${doc.slice(fold.from + 1)}\n`);
});

test('crossed, missing, and partial XML remain editable without fabricated folds', () => {
  for (const doc of ['<a><b></a></b>', '<a><b></b>', '<a><']) {
    let state = stateFor(doc);
    assert.equal(foldAt(state), null);
    state = state.update({ changes: { from: state.doc.length, insert: 'x' } }).state;
    assert.equal(state.doc.toString(), `${doc}x`);
  }
});
