import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  applySearchPanelPayload,
  defaultSearchPanelViewState,
  shouldRevealSelectedResult
} from '../dist-tests/lib/searchPanelState.js';
import { buildSearchCatalog } from '../dist-tests/lib/searchCatalog.js';
import {
  shouldRefreshSystemSearchAfterIndexUpdate,
  shouldApplySystemSearchResponse,
  shouldRetryIndexedSearch
} from '../dist-tests/lib/systemSearchState.js';

test('applies a typed search payload with visible results and selection', () => {
  const next = applySearchPanelPayload(defaultSearchPanelViewState, {
    query: 'firefox',
    results: [
      {
        id: 'app:C:\\Pins\\Firefox.lnk',
        kind: 'app',
        title: 'Firefox',
        subtitle: 'Pinned app',
        terms: 'firefox browser',
        priority: 100
      }
    ],
    selectedIndex: 0,
    statusMessage: 'Type to search apps, windows, folders, and commands'
  });

  assert.equal(next.query, 'firefox');
  assert.equal(next.results.length, 1);
  assert.equal(next.results[0].title, 'Firefox');
  assert.equal(next.selectedIndex, 0);
});

test('applies filesystem search results with launch paths', () => {
  const next = applySearchPanelPayload(defaultSearchPanelViewState, {
    query: 'spotify',
    results: [
      {
        id: 'system:file:C:\\Users\\me\\Documents\\spotify notes.txt',
        kind: 'file',
        path: 'C:\\Users\\me\\Documents\\spotify notes.txt',
        title: 'spotify notes',
        subtitle: 'File',
        terms: 'C:\\Users\\me\\Documents\\spotify notes.txt file local filesystem',
        priority: 120
      }
    ],
    selectedIndex: 0,
    statusMessage: 'Showing apps, windows, files, folders, and commands'
  });

  assert.equal(next.query, 'spotify');
  assert.equal(next.results[0].kind, 'file');
  assert.equal(next.results[0].path, 'C:\\Users\\me\\Documents\\spotify notes.txt');
});

test('includes backend app and file results in the visible catalog', () => {
  const catalog = buildSearchCatalog([], [], [
    {
      id: 'system:app:C:\\Users\\me\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Spotify.lnk',
      kind: 'app',
      path: 'C:\\Users\\me\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Spotify.lnk',
      title: 'Spotify',
      subtitle: 'Installed app - Start Menu',
      terms: 'spotify installed program',
      priority: 210
    },
    {
      id: 'system:file:C:\\Users\\me\\Downloads\\spotify receipt.pdf',
      kind: 'file',
      path: 'C:\\Users\\me\\Downloads\\spotify receipt.pdf',
      title: 'spotify receipt',
      subtitle: 'File - Downloads',
      terms: 'spotify receipt file',
      priority: 160
    }
  ]);

  assert.equal(catalog.some((result) => result.title === 'Spotify'), true);
  assert.equal(catalog.some((result) => result.kind === 'file'), true);
});

test('ignores stale system search responses while keeping latest query live', () => {
  assert.equal(shouldApplySystemSearchResponse('spot', 3, 'spotify', 4), false);
  assert.equal(shouldApplySystemSearchResponse('spotify', 4, 'spotify', 4), true);
});

test('retries empty indexed search while the cache is warming', () => {
  assert.equal(shouldRetryIndexedSearch(0, 0), true);
  assert.equal(shouldRetryIndexedSearch(0, 2), false);
  assert.equal(shouldRetryIndexedSearch(1, 0), false);
});

test('reveals only valid selected search rows', () => {
  assert.equal(shouldRevealSelectedResult(7, 12), true);
  assert.equal(shouldRevealSelectedResult(-1, 12), false);
  assert.equal(shouldRevealSelectedResult(12, 12), false);
});

test('refreshes current system search after an index event', () => {
  assert.equal(shouldRefreshSystemSearchAfterIndexUpdate(true, 'spotify'), true);
  assert.equal(shouldRefreshSystemSearchAfterIndexUpdate(true, 's'), false);
  assert.equal(shouldRefreshSystemSearchAfterIndexUpdate(false, 'spotify'), false);
});
