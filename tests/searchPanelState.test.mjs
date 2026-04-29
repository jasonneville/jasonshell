import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import {
  applySearchPanelPayload,
  defaultSearchPanelViewState,
  shouldRevealSelectedResult
} from '../dist-tests/lib/searchPanelState.js';
import { buildSearchCatalog } from '../dist-tests/lib/searchCatalog.js';
import {
  shouldRefreshSystemSearchAfterIndexUpdate,
  shouldApplySystemSearchResponse,
  shouldRetryIndexedSearch,
  searchPanelAnchorState,
  searchPanelPayloadSignature,
  shouldPublishSearchPanelPayload,
  shouldShowSearchPanelForAnchor
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

test('keeps search panel show and publish idempotent for realtime typing', () => {
  const anchor = searchPanelAnchorState({ left: 10.4, width: 220.2 });
  assert.deepEqual(anchor, { left: 10, width: 220 });
  assert.equal(shouldShowSearchPanelForAnchor(false, null, anchor), true);
  assert.equal(shouldShowSearchPanelForAnchor(true, anchor, anchor), false);
  assert.equal(shouldShowSearchPanelForAnchor(true, anchor, { left: 11, width: 220 }), true);

  const payload = {
    query: 'dev',
    results: [{ id: 'command:open-control-plane', kind: 'command', title: 'Open developer dashboard', subtitle: 'Developer dashboard', terms: 'dev', priority: 92 }],
    selectedIndex: 0,
    statusMessage: 'Showing apps, windows, files, folders, and commands'
  };
  const signature = searchPanelPayloadSignature(payload);
  assert.equal(shouldPublishSearchPanelPayload(null, payload), true);
  assert.equal(shouldPublishSearchPanelPayload(signature, payload), false);
});

test('top-bar defers expensive search render work out of the input handler', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const inputHandler = source.match(/function handleSearchInput\(event: Event\) \{[\s\S]*?\n  \}/)?.[0] ?? '';
  assert.match(inputHandler, /searchQuery = \(event\.currentTarget as HTMLInputElement\)\.value/);
  assert.match(inputHandler, /scheduleSearchRender\(searchQuery\)/);
  assert.doesNotMatch(inputHandler, /rankSearchResults\(/);
  assert.match(source, /window\.setTimeout\(\(\) => \{[\s\S]*rankSearchResults\(allResults, query\)/);
  assert.doesNotMatch(source, /\$:\s*searchResults\s*=\s*rankSearchResults\(allResults, searchQuery\)/);
  assert.match(source, /function queueSearchPanelPublish/);
  assert.match(source, /lastSearchPanelPayloadSignature = signature/);
  assert.match(source, /sequence: \+\+searchPanelPayloadSequence/);
  assert.doesNotMatch(source, /await publishSearchPanel\(sequencedPayload\)/);
});

test('search-panel fallback fetches cannot overwrite newer event payloads', () => {
  const source = readFileSync(new URL('../src/components/SearchPanelSurface.svelte', import.meta.url), 'utf8');
  assert.match(source, /let fallbackGeneration = 0/);
  assert.match(source, /SEARCH_PANEL_UPDATE_EVENT[\s\S]*fallbackGeneration \+= 1/);
  assert.match(source, /const generation = fallbackGeneration[\s\S]*getSearchPanelPayload\(\)\.then/);
  assert.match(source, /if \(generation !== fallbackGeneration\) \{[\s\S]*return;[\s\S]*\}[\s\S]*applyPayload\(payload\)/);
});
