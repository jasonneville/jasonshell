import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  collapseDuplicateResults,
  rankSearchResultsWithUsage,
  scoreSearchResult,
  searchResultRecordKey
} from '../dist-tests/lib/searchRanking.js';

test('selected-count boost moves frequent result up without hiding exact match', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'app:exact',
      providerId: 'apps',
      kind: 'app',
      title: 'Code',
      subtitle: 'Pinned app',
      terms: 'code editor',
      priority: 100
    },
    {
      id: 'app:frequent',
      providerId: 'apps',
      kind: 'app',
      title: 'Code Helper',
      subtitle: 'Pinned app',
      terms: 'code helper',
      priority: 100
    }
  ], 'code', { 'app:frequent': 500 });

  assert.deepEqual(ranked.map((result) => result.id), ['app:exact', 'app:frequent']);
});

test('top-most override wins deterministically for equal query matches', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'app:alpha',
      providerId: 'apps',
      kind: 'app',
      title: 'Alpha Tool',
      subtitle: 'Pinned app',
      terms: 'tool',
      priority: 100
    },
    {
      id: 'app:beta',
      providerId: 'apps',
      kind: 'app',
      title: 'Beta Tool',
      subtitle: 'Pinned app',
      terms: 'tool',
      priority: 100,
      topMost: true
    }
  ], 'tool', {});

  assert.equal(ranked[0].id, 'app:beta');
});

test('provider and result type priority prefer Everything file results over Windows fallback duplicates', () => {
  const duplicatePath = 'C:\\Docs\\Plan.txt';
  const collapsed = collapseDuplicateResults([
    {
      id: `system:file:${duplicatePath}`,
      providerId: 'windowsSearch',
      kind: 'file',
      title: 'Plan',
      subtitle: 'Windows Search',
      terms: 'plan windows search',
      priority: 140,
      path: duplicatePath
    },
    {
      id: `system:file:${duplicatePath}`,
      providerId: 'everything',
      kind: 'file',
      title: 'Plan',
      subtitle: 'Everything',
      terms: 'plan everything voidtools',
      priority: 90,
      path: duplicatePath,
      runCount: 12
    }
  ]);

  assert.equal(collapsed.length, 1);
  assert.equal(collapsed[0].providerId, 'everything');
  assert.equal(searchResultRecordKey(collapsed[0]), 'file:c:\\docs\\plan.txt');
});

test('score math is capped and deterministic', () => {
  const score = scoreSearchResult({
    id: 'file:huge',
    providerId: 'everything',
    kind: 'file',
    title: 'Huge',
    subtitle: 'Everything',
    terms: 'huge everything',
    priority: Number.MAX_SAFE_INTEGER,
    runCount: Number.MAX_SAFE_INTEGER
  }, ['huge'], { 'file:huge': Number.MAX_SAFE_INTEGER });

  assert.equal(score, 1_000_000);
});

test('exact filename matches outrank weak substring matches', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'file:weak',
      providerId: 'everything',
      kind: 'file',
      title: 'Annual Plan Archive',
      subtitle: 'File',
      terms: 'plan archive',
      priority: 120
    },
    {
      id: 'file:exact',
      providerId: 'everything',
      kind: 'file',
      title: 'Plan.txt',
      subtitle: 'File',
      terms: 'plan',
      priority: 90
    }
  ], 'plan', {});

  assert.equal(ranked[0].id, 'file:exact');
});
