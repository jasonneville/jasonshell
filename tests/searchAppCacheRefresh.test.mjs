import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { shouldRetrySearchAfterProviderCacheWarm } from '../dist-tests/features/search/searchUxState.js';

function appTiming(cache, resultCount = 0) {
  return {
    providerId: 'apps',
    startedAt: '2026-05-02T00:00:00Z',
    durationMs: 4,
    cache,
    resultCount,
    applied: true,
    discardedAsStale: false
  };
}

test('app cache miss/indexing retries the latest trimmed query without needing a trailing space', () => {
  const request = { query: 'spotify', sequence: 42 };

  assert.equal(
    shouldRetrySearchAfterProviderCacheWarm(request, 'spotify', 42, [appTiming('miss')]),
    true
  );
  assert.equal(
    shouldRetrySearchAfterProviderCacheWarm(request, 'spotify ', 42, [appTiming('indexing')]),
    true
  );
  assert.equal(
    shouldRetrySearchAfterProviderCacheWarm(request, 'spotify', 43, [appTiming('indexing')]),
    false
  );
  assert.equal(
    shouldRetrySearchAfterProviderCacheWarm(request, 'firefox', 42, [appTiming('indexing')]),
    false
  );
});

test('app cache retry stops when app cache is hit or another provider is warming', () => {
  const request = { query: 'brave', sequence: 9 };

  assert.equal(
    shouldRetrySearchAfterProviderCacheWarm(request, 'brave', 9, [appTiming('hit', 1)]),
    false
  );
  assert.equal(
    shouldRetrySearchAfterProviderCacheWarm(request, 'brave', 9, [appTiming('refresh', 2)]),
    true
  );
  assert.equal(
    shouldRetrySearchAfterProviderCacheWarm(request, 'brave', 9, [
      { ...appTiming('indexing'), providerId: 'everything' }
    ]),
    false
  );
});

test('top bar schedules app-cache warm retries from progress and complete provider timings', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');

  assert.match(source, /let searchProviderCacheRetryTimer: number \| null = null;/);
  assert.match(source, /function scheduleSearchProviderCacheRetry\(/);
  assert.match(source, /shouldRetrySearchAfterProviderCacheWarm\(/);
  assert.match(source, /payload\.providerTimings/);
  assert.match(source, /response\.providerTimings/);
  assert.match(source, /SEARCH_PROVIDER_CACHE_RETRY_LIMIT/);
});
