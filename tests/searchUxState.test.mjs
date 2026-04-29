import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  groupSearchResults,
  ctrlKSearchAction,
  nextSearchPanelFallbackDelay,
  nextSearchResultRefreshRequest,
  searchModeFromSettings,
  searchPanelKeyboardAction,
  searchResultActionHints,
  shouldApplySearchResultRefresh,
  shouldContinueSearchPanelFallbackPolling
} from '../dist-tests/features/search/searchUxState.js';
import {
  rankSearchResults,
  rankSearchResultsWithUsage,
  recordSearchUsage,
  setSearchUsageForTest
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
  assert.deepEqual(
    searchResultActionHints({
      id: 'calculator:2+2',
      kind: 'calculator',
      title: '4',
      subtitle: 'Calculator',
      terms: '2+2',
      priority: 80
    }),
    { primary: 'Copy', secondary: null }
  );
});

test('groups new Flow-like result kinds with command-style actions', () => {
  const groups = groupSearchResults([
    { id: 'setting:display', kind: 'setting', title: 'Display', subtitle: 'Setting', terms: 'display', priority: 70 },
    { id: 'calculator:2+2', kind: 'calculator', title: '4', subtitle: 'Calculator', terms: '2+2', priority: 70 },
    { id: 'web:g', kind: 'web', title: 'Search web', subtitle: 'Google', terms: 'g cats', priority: 70 },
    { id: 'bookmark:docs', kind: 'bookmark', title: 'Docs', subtitle: 'Bookmark', terms: 'docs', priority: 70 }
  ]);

  assert.deepEqual(groups.map((group) => group.id), ['commands']);
  assert.deepEqual(groups[0].items.map((item) => item.result.kind), ['setting', 'calculator', 'web', 'bookmark']);
});

test('ranking accepts injected usage so frequent results can outrank equal matches', () => {
  const ranked = rankSearchResultsWithUsage([
    { id: 'app:alpha', kind: 'app', title: 'Alpha Tool', subtitle: 'App', terms: 'tool', priority: 100 },
    { id: 'app:beta', kind: 'app', title: 'Beta Tool', subtitle: 'App', terms: 'tool', priority: 100 }
  ], 'tool', { 'app:beta': 24 });

  assert.deepEqual(ranked.map((result) => result.id), ['app:beta', 'app:alpha']);
});

test('ranking caches usage map instead of reading storage for every rank', () => {
  let reads = 0;
  let writes = [];
  global.window = {
    localStorage: {
      getItem() {
        reads += 1;
        return JSON.stringify({ 'app:beta': 24 });
      },
      setItem(_key, value) {
        writes.push(value);
      }
    }
  };
  setSearchUsageForTest(null);

  const sample = [
    { id: 'app:alpha', kind: 'app', title: 'Alpha Tool', subtitle: 'App', terms: 'tool', priority: 100 },
    { id: 'app:beta', kind: 'app', title: 'Beta Tool', subtitle: 'App', terms: 'tool', priority: 100 }
  ];
  assert.equal(rankSearchResults(sample, 'tool')[0].id, 'app:beta');
  assert.equal(rankSearchResults(sample, 'tool')[0].id, 'app:beta');
  assert.equal(reads, 1);

  recordSearchUsage('app:alpha');
  assert.equal(writes.length, 1);
  delete global.window;
  setSearchUsageForTest(null);
});

test('search panel fallback polling is bounded with backoff', () => {
  assert.deepEqual(
    [0, 1, 2, 3, 4, 5].map(nextSearchPanelFallbackDelay),
    [120, 240, 500, 1000, 2000, 2000]
  );
  assert.equal(shouldContinueSearchPanelFallbackPolling(0, true, false), false);
  assert.equal(shouldContinueSearchPanelFallbackPolling(0, true, true), true);
  assert.equal(shouldContinueSearchPanelFallbackPolling(0, false, false), true);
});

test('search input state can advance while expensive result refresh is deferred latest-only', () => {
  let displayedQuery = '';
  let sequence = 0;
  const requests = [];

  for (const query of ['s', 'sp', 'spo', 'spot']) {
    displayedQuery = query;
    const request = nextSearchResultRefreshRequest(sequence, query);
    sequence = request.sequence;
    requests.push(request);
  }

  assert.equal(displayedQuery, 'spot');
  assert.equal(shouldApplySearchResultRefresh(requests[0], displayedQuery, sequence), false);
  assert.equal(shouldApplySearchResultRefresh(requests[2], displayedQuery, sequence), false);
  assert.equal(shouldApplySearchResultRefresh(requests[3], displayedQuery, sequence), true);
});

test('search mode routes Ctrl+K while preserving top-right default', () => {
  assert.equal(searchModeFromSettings(undefined), 'topRight');
  assert.equal(searchModeFromSettings('topRight'), 'topRight');
  assert.equal(searchModeFromSettings('centeredHotkey'), 'centeredHotkey');
  assert.equal(ctrlKSearchAction('topRight'), 'openTopRight');
  assert.equal(ctrlKSearchAction('centeredHotkey'), 'openCentered');
});

test('search keyboard actions are shared by top-right and centered modes', () => {
  assert.equal(searchPanelKeyboardAction('ArrowDown'), 'selectNext');
  assert.equal(searchPanelKeyboardAction('ArrowUp'), 'selectPrevious');
  assert.equal(searchPanelKeyboardAction('Enter'), 'activate');
  assert.equal(searchPanelKeyboardAction('Escape'), 'close');
  assert.equal(searchPanelKeyboardAction('Tab'), 'none');
});
