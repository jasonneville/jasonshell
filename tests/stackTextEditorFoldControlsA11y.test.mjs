import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const adapter = readFileSync(new URL('../src/features/stack-browser/stackTextEditorAdapter.ts', import.meta.url), 'utf8');
const foldControl = readFileSync(new URL('../src/features/stack-browser/stackFoldControl.ts', import.meta.url), 'utf8');
const fixture = readFileSync(new URL('../scripts/stack-text-editor/modern-editor-fixture.js', import.meta.url), 'utf8');

test('product fold controls restore the same semantic replacement only after one toggle', () => {
  assert.match(adapter, /foldGutter\(\{[\s\S]*markerDOM/);
  assert.match(foldControl, /document\.createElement\('button'\)/);
  assert.match(foldControl, /aria-label/);
  assert.match(foldControl, /aria-expanded/);
  assert.match(adapter, /foldKeymap/);
  assert.match(adapter, /languageCompartment\.reconfigure\(structured \? configuredLanguage : language\)/);
  assert.match(foldControl, /marker\.addEventListener\('click'/);
  assert.doesNotMatch(foldControl, /restoreAfterToggle\(marker\);\s*marker\.click\(\)/);
  assert.match(adapter, /replacement\.dataset\.foldFrom === foldIdentity/);
});

test('browser fixture source declares guarded CodeMirror keyboard, fold, focus, and document checks', () => {
  // Node checks fixture source only; without a DOM, the guarded browser path does not execute.
  assert.match(fixture, /if \(typeof document !== 'undefined'\) \{/);
  assert.doesNotMatch(fixture, /\b(?:invoke|listen|emit|localStorage|fetch)\s*\(/);
  for (const token of ['new EditorView', 'foldedRanges(view.state)', "KeyboardEvent('keydown'", 'dataset.foldFrom', 'beforeDocument', 'afterDocument']) {
    assert.ok(fixture.includes(token), `fixture missing ${token}`);
  }
  assert.match(fixture, /createStackTextEditorAdapter/);
  assert.match(fixture, /document\.execCommand\('insertText', false, '>'\)/);
  assert.match(fixture, /dirtyCalls === 1/);
  assert.match(fixture, /xmlView\.state\.doc\.toString\(\) === xmlBefore/);
});
