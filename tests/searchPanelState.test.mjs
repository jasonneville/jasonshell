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
  assert.equal(next.sequence, 0);
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

test('catalog exposes system settings intents before incidental filesystem matches', () => {
  const catalog = buildSearchCatalog([], [], [
    {
      id: 'system:folder:C:\\Docs\\Control Panel',
      providerId: 'everything',
      kind: 'folder',
      path: 'C:\\Docs\\Control Panel',
      title: 'Control Panel',
      subtitle: 'Folder',
      terms: 'control panel folder',
      priority: 999
    }
  ]);

  assert.equal(catalog.some((result) => result.id === 'setting:windows-settings'), true);
  assert.equal(catalog.some((result) => result.id === 'setting:control-panel'), true);
  assert.equal(catalog.find((result) => result.id === 'setting:windows-settings')?.path, 'ms-settings:');
  assert.equal(catalog.find((result) => result.id === 'setting:control-panel')?.path, 'control.exe');
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
  assert.equal(shouldRefreshSystemSearchAfterIndexUpdate(true, 's'), true);
  assert.equal(shouldRefreshSystemSearchAfterIndexUpdate(true, '   '), false);
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

test('search panel payload signature includes presentation and ranking metadata', () => {
  const basePayload = {
    query: 'spotify',
    presentation: 'centered',
    results: [{
      id: 'system:app:C:\\Spotify.exe',
      providerId: 'everything',
      kind: 'app',
      title: 'Spotify',
      subtitle: 'Application',
      terms: 'spotify',
      priority: 100,
      path: 'C:\\Spotify.exe',
      recordKey: 'app:c:\\spotify.exe',
      runCount: 2,
      topMost: false
    }],
    selectedIndex: 0,
    statusMessage: 'Showing ranked search results'
  };
  const changedPayload = {
    ...basePayload,
    presentation: 'anchored',
    results: [{ ...basePayload.results[0], topMost: true, runCount: 5 }]
  };

  assert.notEqual(searchPanelPayloadSignature(basePayload), searchPanelPayloadSignature(changedPayload));
});

test('search panel state rejects stale query and phase regressions for the same sequence', () => {
  const local = applySearchPanelPayload(defaultSearchPanelViewState, {
    query: 'spotify',
    results: [{
      id: 'app:C:\\Spotify.exe',
      providerId: 'apps',
      kind: 'app',
      title: 'Spotify',
      subtitle: 'App',
      terms: 'spotify',
      priority: 100,
      recordKey: 'app:c:\\spotify.exe'
    }],
    selectedIndex: 0,
    statusMessage: '1 local result',
    phase: 'local',
    sequence: 5
  });
  const staleQuery = applySearchPanelPayload(local, {
    query: 'spot',
    results: [],
    selectedIndex: 0,
    statusMessage: 'stale',
    phase: 'provider',
    sequence: 5
  });
  const regressedPhase = applySearchPanelPayload(local, {
    query: 'spotify',
    results: [],
    selectedIndex: 0,
    statusMessage: 'older local',
    phase: 'typing',
    sequence: 5
  });

  assert.equal(staleQuery.query, 'spotify');
  assert.equal(staleQuery.statusMessage, '1 local result');
  assert.equal(regressedPhase.phase, 'local');
});

test('search panel state keeps useful rows for typing and empty error payloads', () => {
  const local = applySearchPanelPayload(defaultSearchPanelViewState, {
    query: 'spotify',
    results: [{
      id: 'app:C:\\Spotify.exe',
      providerId: 'apps',
      kind: 'app',
      title: 'Spotify',
      subtitle: 'App',
      terms: 'spotify',
      priority: 100,
      recordKey: 'app:c:\\spotify.exe'
    }],
    selectedIndex: 0,
    statusMessage: '1 local result',
    phase: 'local',
    sequence: 6
  });
  const typing = applySearchPanelPayload(local, {
    query: 'spotify',
    results: [],
    selectedIndex: 0,
    statusMessage: 'Searching...',
    phase: 'typing',
    sequence: 6
  });
  const errored = applySearchPanelPayload(local, {
    query: 'spotify',
    results: [],
    selectedIndex: 0,
    statusMessage: 'Provider error',
    phase: 'error',
    sequence: 6
  });

  assert.equal(typing.results.length, 1);
  assert.equal(errored.results.length, 1);
  assert.equal(errored.statusMessage, 'Provider error');
});

test('search panel state allows complete payload after recoverable provider error', () => {
  const local = applySearchPanelPayload(defaultSearchPanelViewState, {
    query: 'spotify',
    results: [{
      id: 'app:C:\\Spotify.exe',
      providerId: 'apps',
      kind: 'app',
      title: 'Spotify',
      subtitle: 'App',
      terms: 'spotify',
      priority: 100,
      recordKey: 'app:c:\\spotify.exe'
    }],
    selectedIndex: 0,
    statusMessage: '1 local result',
    phase: 'local',
    sequence: 9
  });
  const errored = applySearchPanelPayload(local, {
    query: 'spotify',
    results: [],
    selectedIndex: 0,
    statusMessage: 'Everything unavailable',
    phase: 'error',
    sequence: 9
  });
  const completed = applySearchPanelPayload(errored, {
    query: 'spotify',
    results: local.results,
    selectedIndex: 0,
    statusMessage: 'Showing search results',
    phase: 'complete',
    sequence: 9
  });

  assert.equal(completed.phase, 'complete');
  assert.equal(completed.statusMessage, 'Showing search results');
  assert.equal(completed.results.length, 1);
});

test('top-bar defers expensive search render work out of the input handler', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const inputHandler = source.match(/function handleSearchInput\(event: Event\) \{[\s\S]*?\n  \}/)?.[0] ?? '';
  assert.match(inputHandler, /applySearchQuery\(\(event\.currentTarget as HTMLInputElement\)\.value\)/);
  assert.match(source, /function applySearchQuery\(nextQuery: string\)[\s\S]*scheduleSearchEngine\(searchQuery, request\)/);
  assert.match(source, /function publishPendingSearchPayload/);
  assert.match(source, /function scheduleSearchEngine\(query: string, existingRequest\?: \{ query: string; sequence: number \} \| null\)/);
  assert.doesNotMatch(source, /queuedSearchEngineRequest/);
  assert.match(source, /searchEngine\(\{/);
  assert.match(source, /SEARCH_ENGINE_PROGRESS_EVENT/);
  assert.match(source, /applySearchEngineProgress\(event\.payload\)/);
  assert.match(source, /mergeSearchPanelResultsByStableKey/);
  assert.match(source, /shouldApplySearchEngineResponse\(/);
  assert.match(source, /searchEngineResponseToPanelPayload\(response, selectedIndex, searchPresentation\)/);
  assert.doesNotMatch(inputHandler, /rankSearchResults\(/);
  assert.match(source, /window\.setTimeout\(\(\) => \{[\s\S]*loadSearchEngineResults\(request\)/);
  assert.doesNotMatch(source, /buildSearchCatalog\(launchers, openWindows, systemResults\)/);
  assert.doesNotMatch(source, /rankSearchResults\(allResults, query\)/);
  assert.doesNotMatch(source, /\$:\s*searchResults\s*=\s*rankSearchResults\(allResults, searchQuery\)/);
  assert.match(source, /function queueSearchPanelPublish/);
  assert.match(source, /lastSearchPanelPayloadSignature = signature/);
  assert.match(source, /payload\.sequence === undefined/);
  assert.doesNotMatch(source, /await publishSearchPanel\(sequencedPayload\)/);
  assert.match(source, /result\.kind === 'setting' && result\.path/);
  assert.match(source, /await openShellPath\(result\.path\)/);
  assert.match(source, /const needsNativeShow = !searchOpen \|\| searchPresentation !== 'centered'/);
  assert.match(source, /if \(needsNativeShow\) \{[\s\S]*showCenteredSearchPanel\(readCenteredSearchPanelSize\(\)\)/);
  assert.match(source, /visibleRows = buildVisibleSearchRows\(searchResults,\s*\{/);
  assert.match(source, /selectedVisibleIndex = selectedVisibleRowIndex\(visibleRows, selectedIndex\)/);
  assert.match(source, /const nextIndex = nextVisibleRowIndex\(visibleRows, selectedVisibleIndex, 1\)/);
  assert.match(source, /const nextIndex = nextVisibleRowIndex\(visibleRows, selectedVisibleIndex, -1\)/);
  assert.match(source, /void activateResult\(selectedVisibleResult\)/);
});

test('search-panel fallback fetches cannot overwrite newer event payloads', () => {
  const source = readFileSync(new URL('../src/components/SearchPanelSurface.svelte', import.meta.url), 'utf8');
  assert.match(source, /let fallbackGeneration = 0/);
  assert.match(source, /SEARCH_PANEL_UPDATE_EVENT[\s\S]*fallbackGeneration \+= 1/);
  assert.match(source, /const generation = fallbackGeneration[\s\S]*getSearchPanelPayload\(\)\.then/);
  assert.match(source, /if \(generation !== fallbackGeneration\) \{[\s\S]*return;[\s\S]*\}[\s\S]*applyPayload\(payload\)/);
});

test('centered search surface keeps local typing immediate without unconditional payload refocus', () => {
  const source = readFileSync(new URL('../src/components/SearchPanelSurface.svelte', import.meta.url), 'utf8');

  assert.match(source, /let optimisticQueryDraft: string \| null = null;/);
  assert.match(source, /\$: displayedQuery = optimisticQueryDraft \?\? query;/);
  assert.match(source, /optimisticQueryDraft = value;/);
  assert.match(source, /if \(shouldFocusCenteredQueryInput\(\)\) \{\s*void focusQueryInput\(\);/);
  assert.doesNotMatch(source, /if \(event\.payload\.presentation === 'centered'\) \{\s*void focusQueryInput\(\);/);
});

test('search panel renders a flat visibleRows model instead of grouped buckets', () => {
  const source = readFileSync(new URL('../src/components/SearchPanelSurface.svelte', import.meta.url), 'utf8');

  assert.match(source, /visibleRows = buildVisibleSearchRows\(results,\s*\{/);
  assert.match(source, /\{#each visibleRows as row, index/);
  assert.doesNotMatch(source, /resultGroups = groupSearchResults\(results\)/);
  assert.doesNotMatch(source, /\{#each resultGroups as group/);
});

test('search panel keyboard and aria state follow visibleRows order', () => {
  const source = readFileSync(new URL('../src/components/SearchPanelSurface.svelte', import.meta.url), 'utf8');

  assert.match(source, /selectedRowIndex = selectedVisibleRowIndex\(visibleRows, selectedIndex\)/);
  assert.match(source, /aria-activedescendant=\{selectedRow\?\.domId\}/);
  assert.match(source, /\{#each visibleRows as row, index \(row\.rowKey\)\}/);
  assert.match(source, /id=\{row\.domId\}/);
  assert.match(source, /use:trackVisibleRow=\{index\}/);
  assert.match(source, /selectVisibleOffset\(1\)/);
  assert.match(source, /selectVisibleOffset\(-1\)/);
  assert.match(source, /activateRow\(selectedRow\)/);
  assert.match(source, /searchVisibleRowIdentity\(row\)/);
  assert.doesNotMatch(source, /use:trackResultRow=\{index\}/);
});
