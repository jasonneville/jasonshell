import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  createLatestSearchQueryController,
  clearLatestSearchQuery
} from '../dist-tests/features/search/searchUxState.js';

function readSource(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
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

test('top-bar search renders icon-only button that opens centered search', () => {
  const source = readSource('../src/components/TopBar.svelte');
  const styles = readSource('../src/components/TopBar.css');

  assert.match(source, /<div class="search-control" bind:this=\{searchControl\}>/);
  assert.match(source, /class="search-button"/);
  assert.match(source, /ariaLabel="Open search"/);
  assert.match(source, /ariaHaspopup="dialog"/);
  assert.match(source, /tooltip="Search"/);
  assert.match(source, /<MaterialSymbolIcon name="search" \/>/);
  assert.match(source, /onClick=\{\(\) => void \(searchOpen \? closePanel\(\) : openCenteredPanel\(\{ publishCurrentPayload: true \}\)\)\}/);
  assert.match(source, /let searchInputDraft = '';/);
  assert.doesNotMatch(source, /search-clear-button/);
  assert.doesNotMatch(source, /handleSearchPointerDown|handleSearchFocus|handleSearchKeydown|clearSearch|hasSearchClearValue/);
  assert.match(styles, /\.top-bar \.search-control \{[\s\S]*?flex:\s*0 0 auto;/);
});

test('centered search panel renders independent clear button for non-empty displayed query', () => {
  const source = readSource('../src/components/SearchPanelSurface.svelte');

  assert.match(source, /function hasCenteredSearchClearValue\(\)/);
  assert.match(source, /\{#if presentation === 'centered' && hasCenteredSearchClearValue\(\)\}/);
  assert.match(source, /ariaLabel="Clear search"/);
  assert.match(source, /class="search-panel-clear-button"/);
  assert.match(source, /onClick=\{\(\) => void clearCenteredSearch\(\)\}/);
});

test('clearing advances latest search sequence so stale provider response is rejected', () => {
  const controller = createLatestSearchQueryController();
  const oldRequest = controller.next('firefox');
  const clearRequest = clearLatestSearchQuery(controller);

  assert.equal(clearRequest.query, '');
  assert.equal(clearRequest.sequence, oldRequest.sequence + 1);
  assert.equal(controller.shouldApply(oldRequest, ''), false);
  assert.equal(controller.shouldApply(clearRequest, ''), true);
});

test('centered panel clear routes through query reset path and does not call provider directly', () => {
  const source = readSource('../src/components/TopBar.svelte');
  assert.match(source, /function handleSearchInput\(event: Event\)/);
  assert.match(source, /const request = publishImmediateSearchInputState\(nextQuery\)/);
  assert.match(source, /startImmediateSearchQueryExecution\(request\)/);
  assert.doesNotMatch(source, /searchInput\?\.focus|searchInput\?\.blur|handleSearchPointerDown|clearSearch/);
});

test('centered clear emits empty query with newer input sequence and keeps focus', () => {
  const source = readSource('../src/components/SearchPanelSurface.svelte');
  const clearSearch = extractFunction(source, 'clearCenteredSearch');

  assert.match(clearSearch, /optimisticQueryDraft = '';/);
  assert.match(clearSearch, /queueQueryEmit\(''\)/);
  assert.match(clearSearch, /await tick\(\)/);
  assert.match(clearSearch, /queryInput\?\.focus\(\{ preventScroll: true \}\)/);
  assert.doesNotMatch(clearSearch, /searchEngine\(|publishSearchPanel\(|getSearchPanelPayload\(/);
});
