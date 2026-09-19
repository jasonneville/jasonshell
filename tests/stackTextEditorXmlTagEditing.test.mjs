import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { autoCloseTags, xml, xmlLanguage } from '@codemirror/lang-xml';
import { history, undo } from '@codemirror/commands';
import { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { syntaxTree } from '@codemirror/language';

const adapter = readFileSync(new URL('../src/features/stack-browser/stackTextEditorAdapter.ts', import.meta.url), 'utf8');

const tagPair = (doc, position) => {
  const state = EditorState.create({ doc, extensions: [xmlLanguage] });
  let malformed = false;
  syntaxTree(state).iterate({ enter: node => { if (node.type.isError || node.name === 'MismatchedCloseTag') malformed = true; } });
  if (malformed) return null;
  let node = syntaxTree(state).resolveInner(position, 1);
  while (node && !['OpenTag', 'CloseTag'].includes(node.name)) node = node.parent;
  const element = node?.parent;
  if (!element || element.name !== 'Element') return null;
  const open = element.firstChild, close = element.lastChild;
  if (open?.name !== 'OpenTag' || close?.name !== 'CloseTag') return null;
  const name = n => n.getChild('TagName');
  const a = name(open), b = name(close);
  if (!a || !b || state.sliceDoc(a.from, a.to) !== state.sliceDoc(b.from, b.to)) return null;
  return [{ from: a.from, to: a.to }, { from: b.from, to: b.to }];
};

test('XML parser pairs exact namespace-qualified and nested same-name tags', () => {
  const doc = '<x:r a=">"><x:r>v &amp; z</x:r></x:r>';
  assert.deepEqual(tagPair(doc, 2), [{ from: 1, to: 4 }, { from: 33, to: 36 }]);
  assert.deepEqual(tagPair(doc, 15), [{ from: 12, to: 15 }, { from: 27, to: 30 }]);
});

test('XML passive matching rejects declarations, PI, comments, CDATA, self-closing and malformed tags', () => {
  for (const [doc, pos] of [['<?xml?>', 2], ['<?pi?>', 2], ['<!--x-->', 2], ['<![CDATA[x]]>', 4], ['<x:any/>', 2], ['<a><b></a></b>', 2], ['<partial', 2]]) {
    assert.equal(tagPair(doc, pos), null, doc);
  }
});

test('XML install retains package autoclose, parser bracket matching, one update listener, and no generic completion', () => {
  assert.match(adapter, /bracketMatching\(\)/);
  assert.match(adapter, /EditorView\.updateListener\.of/);
  assert.doesNotMatch(adapter, /autocompletion\(|@codemirror\/autocomplete/);
  assert.match(adapter, /const atomicXmlCloseTag = EditorView\.inputHandler/);
  assert.match(adapter, /insert: `><\/\$\{name\}>`/);
});

function typeXml(doc, text = '>', { selection = null, composing = false } = {}) {
  let dirtyCalls = 0;
  let state = EditorState.create({
    doc,
    selection: selection ?? { anchor: doc.length },
    extensions: [xml(), history()]
  });
  assert.ok(autoCloseTags, 'package autoclose extension exported');
  const handler = state.facet(EditorView.inputHandler)[0];
  assert.ok(handler, 'XML autoclose input handler installed');
  const from = state.selection.main.from;
  const to = state.selection.main.to;
  const view = {
    composing,
    get state() { return state; },
    dispatch(transactions) {
      const list = Array.isArray(transactions) ? transactions : [transactions];
      state = list.at(-1).state;
      if (list.some(transaction => transaction.docChanged)) dirtyCalls++;
    }
  };
  const handled = handler(view, from, to, text, () => state.update({
    changes: { from, to, insert: text },
    selection: { anchor: from + text.length },
    userEvent: 'input.type'
  }));
  return { handled, dirtyCalls, get state() { return state; }, view };
}

test('pinned XML package exposes two-step undo defect that production atomic handler fixes', () => {
  const original = '<x:item id="1"';
  const edit = typeXml(original);
  assert.equal(edit.handled, true);
  assert.equal(edit.state.doc.toString(), '<x:item id="1"></x:item>');
  assert.equal(edit.dirtyCalls, 1);
  assert.equal(undo(edit.view), true);
  assert.equal(edit.state.doc.toString(), `${original}>`, 'pinned package alone leaves typed >; production atomic handler is required');
});

test('XML package handler excludes paste-like multi-text, selection, deletion, and composition simulations', () => {
  assert.equal(typeXml('<a', '></a>').handled, false, 'paste');
  assert.equal(typeXml('<a>', '>', { selection: { anchor: 0, head: 2 } }).handled, false, 'selection');
  assert.equal(typeXml('<a', '').handled, false, 'backspace/undo');
  assert.equal(typeXml('<a', '>', { composing: true }).handled, false, 'IME');
  for (const doc of ['<?xml version="1.0"?', '<?pi?', '<!--x--', '<![CDATA[x]]', '<a/']) {
    assert.equal(typeXml(doc).handled, false, doc);
  }
  const closing = typeXml('<root><child><', '/');
  assert.equal(closing.handled, true);
  assert.equal(closing.state.doc.toString(), '<root><child></child>');
});
