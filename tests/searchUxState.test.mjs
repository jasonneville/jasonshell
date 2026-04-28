import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  groupSearchResults,
  searchResultActionHints
} from '../dist-tests/features/search/searchUxState.js';
import {
  rankSearchResultsWithUsage
} from '../dist-tests/lib/searchRanking.js';

const results = [
  { id: 'command:refresh-search', kind: 'command', title: 'Refresh', subtitle: 'Reload', terms: 'refresh', priority: 40 },
  { id: 'window:10', kind: 'window', title: 'Editor', subtitle: 'Code', terms: 'editor code', priority: 90 },
  { id: 'folder:shell:Downloads', kind: 'folder', title: 'Downloads', subtitle: 'Folder', terms: 'downloads folder', priority: 60, path: 'shell:Downloads' },
  { id: 'file:C:/notes.txt', kind: 'file', title: 'notes.txt', subtitle: 'File', terms: 'notes file', priority: 50 },
  { id: 'app:C:/Code.lnk', kind: 'app', title: 'Code', subtitle: 'Pinned app', terms: 'code app', priority: 100 }
];

test('groups search results into keyboard-scan sections without changing result order inside groups', () => {
  const groups = groupSearchResults(results);

  assert.deepEqual(groups.map((group) => group.id), ['apps', 'windows', 'places', 'files', 'commands']);
  assert.deepEqual(groups.map((group) => group.items[0].index), [4, 1, 2, 3, 0]);
});

test('labels primary and secondary actions by result kind', () => {
  assert.deepEqual(searchResultActionHints(results[4]), { primary: 'Launch', secondary: null });
  assert.deepEqual(searchResultActionHints(results[1]), { primary: 'Focus', secondary: null });
  assert.deepEqual(searchResultActionHints(results[2]), { primary: 'Open', secondary: 'Pin' });
  assert.deepEqual(searchResultActionHints(results[0]), { primary: 'Run', secondary: null });
});

test('ranking accepts injected usage so frequent results can outrank equal matches', () => {
  const ranked = rankSearchResultsWithUsage([
    { id: 'app:alpha', kind: 'app', title: 'Alpha Tool', subtitle: 'App', terms: 'tool', priority: 100 },
    { id: 'app:beta', kind: 'app', title: 'Beta Tool', subtitle: 'App', terms: 'tool', priority: 100 }
  ], 'tool', { 'app:beta': 24 });

  assert.deepEqual(ranked.map((result) => result.id), ['app:beta', 'app:alpha']);
});
