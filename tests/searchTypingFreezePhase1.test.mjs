import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  createLatestSearchQueryController,
  shouldRetrySearchFreshness,
  shouldApplySearchEngineResponse
} from '../dist-tests/features/search/searchUxState.js';

function extractFunction(source, name) {
  const start = source.indexOf(`function ${name}`);
  assert.notEqual(start, -1, `${name} should exist`);
  const bodyStart = source.indexOf('{', start);
  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') {
      depth += 1;
    } else if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        return source.slice(start, index + 1);
      }
    }
  }
  throw new Error(`could not extract ${name}`);
}

test('phase 1 top-bar input handler publishes draft and starts search for every input event', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const handler = extractFunction(source, 'handleSearchInput');
  const immediate = extractFunction(source, 'publishImmediateSearchInputState');
  const start = extractFunction(source, 'startImmediateSearchQueryExecution');

  assert.match(source, /let searchInputDraft = '';/);
  assert.match(source, /function updateSearchInputDraft\(nextQuery: string\)/);
  assert.match(source, /function publishImmediateSearchInputState\(nextQuery: string\)/);
  assert.match(source, /function startImmediateSearchQueryExecution\(request: SearchEngineQueryRequestState\)/);
  assert.match(handler, /const nextQuery = \(event\.currentTarget as HTMLInputElement\)\.value;/);
  assert.match(handler, /const request = publishImmediateSearchInputState\(nextQuery\)/);
  assert.match(handler, /startImmediateSearchQueryExecution\(request\)/);
  assert.equal(
    handler.indexOf('publishImmediateSearchInputState(nextQuery)') <
      handler.indexOf('startImmediateSearchQueryExecution(request)'),
    true,
    'input must publish visible pending state before async provider execution starts'
  );
  assert.doesNotMatch(handler, /applySearchQuery/);
  assert.doesNotMatch(handler, /publishSearchPanel|showSearchPanel|showCenteredSearchPanel|searchEngine|buildVisibleSearchRows|nextProgressiveSearchResultSet|mergeSearchPanelResultsByStableKey/);
  assert.doesNotMatch(immediate, /searchEngine|loadSearchEngineResults|buildVisibleSearchRows|nextProgressiveSearchResultSet|mergeSearchPanelResultsByStableKey/);
  assert.match(start, /void loadSearchEngineResults\(request\)/);
  assert.doesNotMatch(start, /setTimeout|searchQueryProcessingQueue|queueSearchQueryProcessing/);
});

test('phase 1 search query execution is not debounced or coalesced after input draft advances', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const start = extractFunction(source, 'startImmediateSearchQueryExecution');

  assert.doesNotMatch(source, /queuedSearchQueryTimer/);
  assert.doesNotMatch(source, /SEARCH_QUERY_PROCESSING_DELAY_MS/);
  assert.doesNotMatch(source, /createLatestSearchExecutionQueue/);
  assert.doesNotMatch(source, /queueSearchQueryProcessing|flushQueuedSearchQuery|searchQueryProcessingQueue/);
  assert.match(start, /publishPendingSearchPayload\(request\.sequence, searchResults\)/);
  assert.match(start, /void loadSearchEngineResults\(request\)/);
  assert.match(start, /scheduleSearchFreshnessRetry\(request\)/);
});

test('draft query change cancels old search work before deferred apply window', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const immediate = extractFunction(source, 'publishImmediateSearchInputState');
  const supersede = extractFunction(source, 'cancelSupersededSearchWorkForDraft');

  assert.match(supersede, /function cancelSupersededSearchWorkForDraft\(nextQuery: string\)/);
  assert.match(supersede, /if \(nextQuery === searchQuery\) \{[\s\S]*?return;/);
  assert.match(supersede, /cancelSearchEngineTimer\(\)/);
  assert.match(supersede, /invalidateSearchEngineResponses\(\)/);
  assert.equal(
    immediate.indexOf('cancelSupersededSearchWorkForDraft(nextQuery)') <
      immediate.indexOf('searchQuery = nextQuery'),
    true,
    'old search work must be canceled before the visible query changes'
  );
  assert.doesNotMatch(immediate, /searchEngine|loadSearchEngineResults/);
});

test('rapid Windows settings typing publishes pending visible state for every key before provider flush', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const immediate = extractFunction(source, 'publishImmediateSearchInputState');
  const start = extractFunction(source, 'startImmediateSearchQueryExecution');

  assert.match(immediate, /const request = searchQueryController\.next\(nextQuery\)/);
  assert.match(immediate, /const normalizedChanged = request\.query !== previousNormalizedQuery/);
  assert.match(immediate, /if \(!request\.query \|\| normalizedChanged\) \{[\s\S]*searchResults = \[\];[\s\S]*searchResultsQuery = '';/);
  assert.match(immediate, /openConfiguredPanel\(\{ publishCurrentPayload: false \}\)/);
  assert.match(immediate, /queueSearchPanelPublish\(\{/);
  assert.match(immediate, /query: request\.query/);
  assert.match(immediate, /results: request\.query && !normalizedChanged \? searchResults : \[\]/);
  assert.match(immediate, /phase: request\.query \? 'typing' : 'complete'/);
  assert.match(immediate, /sequence: request\.sequence/);

  const typedQueries = ['W', 'Wi', 'Win', 'Wind', 'Windo', 'Window', 'Windows', 'Windows ', 'Windows s', 'Windows se', 'Windows set', 'Windows sett', 'Windows setti', 'Windows settin', 'Windows setting', 'Windows settings'];
  const controller = createLatestSearchQueryController();
  const executed = [];
  const visiblePayloads = typedQueries.map((query) => {
    const request = controller.next(query);
    executed.push(request.query);
    return {
      query: request.query,
      phase: request.query ? 'typing' : 'complete',
      results: [],
      sequence: request.sequence
    };
  });

  assert.deepEqual(visiblePayloads.map((payload) => payload.query), typedQueries.map((query) => query.trim()));
  assert.deepEqual(visiblePayloads.map((payload) => payload.sequence), typedQueries.map((_, index) => index + 1));
  assert.deepEqual(executed, typedQueries.map((query) => query.trim()));
  assert.match(start, /void loadSearchEngineResults\(request\)/);
});

test('top-bar input events enqueue exact current value instead of prior draft state', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const inputHandler = extractFunction(source, 'handleSearchInput');

  assert.match(inputHandler, /const nextQuery = \(event\.currentTarget as HTMLInputElement\)\.value;/);
  assert.match(inputHandler, /const request = publishImmediateSearchInputState\(nextQuery\)/);
  assert.match(inputHandler, /startImmediateSearchQueryExecution\(request\)/);
  assert.doesNotMatch(inputHandler, /queueSearchQueryProcessing/);
});

test('centered search query events reject stale source-order payloads and use exact query', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const listener =
    source.match(/void listen<SearchPanelQueryPayload>\(SEARCH_PANEL_QUERY_EVENT, \(event\) => \{[\s\S]*?\n    \}\)\.then/)?.[0] ?? '';

  assert.match(source, /let lastSearchPanelInputSequence = 0;/);
  assert.match(listener, /isSearchPanelQueryPayload\(event\.payload\)/);
  assert.match(listener, /event\.payload\.inputSequence <= lastSearchPanelInputSequence/);
  assert.match(listener, /lastSearchPanelInputSequence = event\.payload\.inputSequence/);
  assert.match(listener, /const nextQuery = event\.payload\.query;/);
  assert.match(listener, /const request = publishImmediateSearchInputState\(nextQuery\)/);
  assert.match(listener, /startImmediateSearchQueryExecution\(request\)/);
  assert.doesNotMatch(listener, /queueSearchQueryProcessing/);
});

test('centered search out-of-order Wi event cannot overwrite later Windows settings query', () => {
  const events = [
    { query: 'Windows settings', inputSequence: 16 },
    { query: 'Wi', inputSequence: 2 }
  ];
  let lastInputSequence = 0;
  const accepted = [];

  for (const event of events) {
    if (event.inputSequence <= lastInputSequence) {
      continue;
    }
    lastInputSequence = event.inputSequence;
    accepted.push(event.query);
  }

  assert.deepEqual(accepted, ['Windows settings']);
});

test('app-index refresh uses current draft query during deferred apply window', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const refreshHandler =
    source.match(/void listen<SearchIndexRefreshedPayload>\(SEARCH_INDEX_REFRESHED_EVENT, \(event\) => \{[\s\S]*?\n    \}\)\.then/)?.[0] ?? '';

  assert.match(refreshHandler, /const refreshQuery = searchInputDraft !== searchQuery \? searchInputDraft : searchQuery;/);
  assert.match(refreshHandler, /if \(!searchOpen \|\| !refreshQuery\.trim\(\)\) \{/);
  assert.match(refreshHandler, /if \(refreshQuery !== searchQuery\) \{[\s\S]*const request = publishImmediateSearchInputState\(refreshQuery\);[\s\S]*startImmediateSearchQueryExecution\(request\)[\s\S]*return;/);
  assert.match(refreshHandler, /scheduleSearchEngine\(refreshQuery\)/);
  assert.doesNotMatch(refreshHandler, /scheduleSearchEngine\(searchQuery\)/);
});

test('rapid Brave typing creates exact current requests for every prefix', () => {
  const controller = createLatestSearchQueryController();
  const executed = [];

  for (const query of ['b', 'br', 'bra', 'brav', 'brave']) {
    const request = controller.next(query);
    executed.push(request.query);
  }

  assert.deepEqual(executed, ['b', 'br', 'bra', 'brav', 'brave']);
});

test('phase 1 close cancels queued and scheduled search work and invalidates stale responses', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const cleanupSearchWorkAfterClose = extractFunction(source, 'cleanupSearchWorkAfterClose');

  assert.match(source, /function cancelSearchEngineTimer\(\)/);
  assert.match(source, /function invalidateSearchEngineResponses\(\)/);
  assert.match(source, /function closePanel\(\) \{[\s\S]*cleanupSearchWorkAfterClose\(\)/);
  assert.doesNotMatch(cleanupSearchWorkAfterClose, /cancelSearchQueryProcessing/);
  assert.match(cleanupSearchWorkAfterClose, /cancelSearchEngineTimer\(\)/);
  assert.match(cleanupSearchWorkAfterClose, /invalidateSearchEngineResponses\(\)/);
});

test('phase 1 rapid Firefox input keeps final draft current and only latest provider response applies', () => {
  const controller = createLatestSearchQueryController();
  let visibleDraft = '';
  const requests = [];

  for (const query of ['F', 'Fi', 'Fir', 'Fire', 'Firef', 'Firefo', 'Firefox']) {
    visibleDraft = query;
    requests.push(controller.next(query));
  }

  assert.equal(visibleDraft, 'Firefox');
  for (const request of requests.slice(0, -1)) {
    assert.equal(shouldApplySearchEngineResponse(request, visibleDraft, controller.currentSequence()), false);
  }
  assert.equal(
    shouldApplySearchEngineResponse(requests.at(-1), visibleDraft, controller.currentSequence()),
    true
  );
});

test('phase 1 Rust search command runs coordinator behind async blocking boundary', () => {
  const source = readFileSync(new URL('../src-tauri/src/search/mod.rs', import.meta.url), 'utf8');

  assert.match(source, /pub\(crate\) async fn search_engine\(/);
  assert.match(source, /tauri::async_runtime::spawn_blocking\(move \|\| \{/);
  assert.match(source, /run_search_engine\(request, \|payload\| \{/);
  assert.match(source, /\.await\s*\.map_err\(/);
});

test('spacebar freshness workaround is replaced by automatic same-query retry', () => {
  assert.equal(shouldRetrySearchFreshness('firefox', '', 'Searching...', 7, 7), true);
  assert.equal(shouldRetrySearchFreshness('firefox', 'firefox', 'Searching local providers...', 7, 7), true);
  assert.equal(shouldRetrySearchFreshness('firefox ', 'firefox', 'Showing search results', 8, 8), false);
  assert.equal(shouldRetrySearchFreshness('firefox', 'firefox', 'Showing search results', 8, 8), false);
  assert.equal(shouldRetrySearchFreshness('firefox', '', 'Searching...', 7, 8), false);

  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  assert.match(source, /let searchFreshnessRetryTimer: number \| null = null;/);
  assert.match(source, /function scheduleSearchFreshnessRetry\(request: \{ query: string; sequence: number \}\)/);
  assert.match(source, /shouldRetrySearchFreshness\(/);
  assert.match(source, /scheduleSearchEngine\(searchQuery\)/);
});
