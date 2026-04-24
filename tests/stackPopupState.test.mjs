import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  applyStackEntries,
  canNavigateStackBack,
  canNavigateStackForward,
  defaultStackPopupViewState,
  formatStackSize,
  navigateStackHistory,
  openStackFolder,
  selectStackEntry,
  selectedStackEntry,
  stackItemNameFromPath
} from '../dist-tests/stackPopupState.js';

const documents = 'C:\\Users\\me\\Documents';
const downloads = 'C:\\Users\\me\\Downloads';

test('keeps stack navigation history across folder switching', () => {
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = openStackFolder(state, downloads);

  assert.equal(state.currentPath, downloads);
  assert.deepEqual(state.history, [documents, downloads]);
  assert.equal(canNavigateStackBack(state), true);
  assert.equal(canNavigateStackForward(state), false);

  state = navigateStackHistory(state, -1);
  assert.equal(state.currentPath, documents);
  assert.equal(canNavigateStackForward(state), true);
});

test('does not duplicate current folder when reopened after hide/show', () => {
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = openStackFolder(state, documents);

  assert.deepEqual(state.history, [documents]);
  assert.equal(state.historyIndex, 0);
});

test('drops forward history after branching to a different folder', () => {
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = openStackFolder(state, downloads);
  state = navigateStackHistory(state, -1);
  state = openStackFolder(state, 'C:\\Users\\me\\Desktop');

  assert.deepEqual(state.history, [documents, 'C:\\Users\\me\\Desktop']);
  assert.equal(canNavigateStackForward(state), false);
});

test('clears stale entries when switching folders', () => {
  const entries = [
    {
      id: 'old',
      name: 'old.txt',
      path: 'C:\\Users\\me\\Documents\\old.txt',
      entryType: 'File',
      size: 1,
      modifiedMs: null
    }
  ];
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackEntries(state, documents, entries);
  state = openStackFolder(state, downloads);

  assert.equal(state.currentPath, downloads);
  assert.deepEqual(state.entries, []);
  assert.equal(state.selectedPath, null);
});

test('clears stale entries when navigating history', () => {
  const entries = [
    {
      id: 'old',
      name: 'old.txt',
      path: 'C:\\Users\\me\\Downloads\\old.txt',
      entryType: 'File',
      size: 1,
      modifiedMs: null
    }
  ];
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = openStackFolder(state, downloads);
  state = applyStackEntries(state, downloads, entries);
  state = navigateStackHistory(state, -1);

  assert.equal(state.currentPath, documents);
  assert.deepEqual(state.entries, []);
});

test('applies entries and preserves valid selection', () => {
  const entries = [
    {
      id: 'c:\\users\\me\\documents\\notes.txt',
      name: 'notes.txt',
      path: 'C:\\Users\\me\\Documents\\notes.txt',
      entryType: 'File',
      size: 1536,
      modifiedMs: 1700000000000
    }
  ];
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackEntries(state, documents, entries);
  state = selectStackEntry(state, entries[0].path);

  assert.equal(selectedStackEntry(state)?.name, 'notes.txt');
  assert.equal(formatStackSize(selectedStackEntry(state)?.size), '1.5 KB');
});

test('ignores stale folder entry payloads', () => {
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackEntries(state, downloads, [
    {
      id: 'stale',
      name: 'stale.txt',
      path: 'C:\\Users\\me\\Downloads\\stale.txt',
      entryType: 'File',
      size: 1,
      modifiedMs: null
    }
  ]);

  assert.equal(state.entries.length, 0);
  assert.equal(state.currentPath, documents);
});

test('extracts stack item names from windows paths', () => {
  assert.equal(stackItemNameFromPath('C:\\Users\\me\\Documents'), 'Documents');
  assert.equal(stackItemNameFromPath('/home/me/Documents'), 'Documents');
});
