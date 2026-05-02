import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const stackPopupSource = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');
const stackPopupSurfaceSource = readFileSync(
  new URL('../src/components/StackPopupSurface.svelte', import.meta.url),
  'utf8'
);

test('stack folder diagnostics include phase-5 timing milestones and icon-cache counters', () => {
  assert.match(stackPopupSource, /firstPaintDurationMs: number;/);
  assert.match(stackPopupSource, /metadataListingCompleteDurationMs: number;/);
  assert.match(stackPopupSource, /iconQueueCompleteDurationMs: number;/);
  assert.match(stackPopupSource, /iconCacheHits: number;/);
  assert.match(stackPopupSource, /iconCacheMisses: number;/);
});

test('stack folder listing emits explicit metadata first-paint and metadata-complete diagnostics phases', () => {
  assert.match(stackPopupSource, /phase: 'first-paint'/);
  assert.match(stackPopupSource, /phase: 'metadata-complete'/);
});

test('stack popup icon hydration tracks icon-cache hits and misses for completion diagnostics', () => {
  assert.match(stackPopupSurfaceSource, /iconHydrationCacheHits/);
  assert.match(stackPopupSurfaceSource, /iconHydrationCacheMisses/);
  assert.match(stackPopupSurfaceSource, /iconQueueCompleteDurationMs/);
  assert.match(stackPopupSurfaceSource, /phase: 'icon-queue-complete'/);
});
