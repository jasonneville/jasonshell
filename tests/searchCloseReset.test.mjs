import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

function readTopBar() {
  return readFileSync('src/components/TopBar.svelte', 'utf8');
}

function extractFunction(source, name) {
  const start = source.indexOf(`function ${name}`);
  assert.notEqual(start, -1, `${name} should exist`);
  const bodyStart = source.indexOf('{', start);
  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0) return source.slice(start, index + 1);
  }
  throw new Error(`could not extract ${name}`);
}

function extractListenHandler(source, eventName) {
  const eventIndex = source.indexOf(`listen(${eventName}`);
  assert.notEqual(eventIndex, -1, `${eventName} listener should exist`);
  const arrowIndex = source.indexOf('=>', eventIndex);
  const bodyStart = source.indexOf('{', arrowIndex);
  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0) return source.slice(bodyStart, index + 1);
  }
  throw new Error(`could not extract ${eventName} listener`);
}

test('shared active-search reset clears local query, results, selection, and stale-response gates', () => {
  const source = readTopBar();
  const resetActiveSearchState = extractFunction(source, 'resetActiveSearchState');

  assert.match(resetActiveSearchState, /cleanupSearchWorkAfterClose\(\)/);
  assert.match(resetActiveSearchState, /searchQuery = '';/);
  assert.match(resetActiveSearchState, /searchInputDraft = '';/);
  assert.match(resetActiveSearchState, /searchResults = \[\];/);
  assert.match(resetActiveSearchState, /searchResultsQuery = '';/);
  assert.match(resetActiveSearchState, /selectedIndex = 0;/);
  assert.match(resetActiveSearchState, /searchStatus = 'Search is ready';/);
  assert.match(resetActiveSearchState, /expandedVisibleGroups = new Set<SearchExpandableGroupId>\(\);/);
  assert.match(resetActiveSearchState, /invalidateSearchEngineResponses\(\)/);
});

test('explicit close uses shared reset and publishes blank payload for centered and top-bar sync', () => {
  const closePanel = extractFunction(readTopBar(), 'closePanel');

  assert.match(closePanel, /resetActiveSearchState\(\)/);
  assert.match(closePanel, /query: ''/);
  assert.match(closePanel, /results: \[\]/);
  assert.match(closePanel, /selectedIndex: 0/);
  assert.match(closePanel, /statusMessage: 'Search is ready'/);
  assert.match(closePanel, /searchInput\?\.blur\(\)/);
});

test('native search-panel closed event uses the same reset path', () => {
  const source = readTopBar();
  const handler = extractListenHandler(source, 'SEARCH_PANEL_CLOSED_EVENT');

  assert.match(handler, /resetActiveSearchState\(\)/);
  assert.doesNotMatch(handler, /cleanupSearchWorkAfterClose\(\)/);
});

test('successful result activation routes through close reset instead of manual partial clears', () => {
  const activateResult = extractFunction(readTopBar(), 'activateResult');

  assert.match(activateResult, /await closePanel\(\)/);
  assert.doesNotMatch(activateResult, /searchQuery = '';/);
  assert.doesNotMatch(activateResult, /searchInputDraft = '';/);
  assert.doesNotMatch(activateResult, /selectedIndex = 0;/);
});
