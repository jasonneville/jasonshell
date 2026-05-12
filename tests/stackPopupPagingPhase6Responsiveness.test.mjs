import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const rustStackPopup = readFileSync(new URL('../src-tauri/src/stack_popup.rs', import.meta.url), 'utf8');
const rustIcons = readFileSync(new URL('../src-tauri/src/stack_popup/icons.rs', import.meta.url), 'utf8');

test('stack icon command is async and moves shell icon extraction off the command thread', () => {
  assert.match(rustStackPopup, /pub\s+async\s+fn\s+resolve_stack_item_icons\(/);
  assert.match(rustStackPopup, /resolve_stack_item_icons_for_paths_async\(paths\)\.await/);
  assert.match(rustIcons, /pub\(crate\)\s+async\s+fn\s+resolve_stack_item_icons_for_paths_async/);
  assert.match(rustIcons, /tauri::async_runtime::spawn_blocking/);
});

test('icon hydration prioritizes visible rows without restarting the whole folder queue', () => {
  assert.match(surface, /let\s+iconHydrationVisiblePriority:\s*string\[\]\s*=\s*\[\]/);
  assert.match(surface, /queueVisibleIconHydrationPriority\(folderPath,\s*loadSequence\)/);
  assert.match(surface, /emitVisibleRowsWindowChanged[\s\S]*queueVisibleIconHydrationPriority/);
  assert.match(surface, /const batchPaths = nextIconHydrationBatch\(\)/);
  assert.match(surface, /iconHydrationPending = mergeIconHydrationPending/);
});

test('visible-row priority and window telemetry use filtered visible entries', () => {
  const prioritySource = surface.slice(
    surface.indexOf('function queueVisibleIconHydrationPriority'),
    surface.indexOf('function mergeIconHydrationPending')
  );
  const emitSource = surface.slice(
    surface.indexOf('function emitVisibleRowsWindowChanged'),
    surface.indexOf('function sortBy')
  );
  assert.match(prioritySource, /stackBrowserVirtualWindow\(visibleEntries,\s*detailsBodyScrollTop,\s*detailsBodyHeight\)/);
  assert.doesNotMatch(prioritySource, /stackBrowserVirtualWindow\(entries,/);
  assert.match(emitSource, /stackBrowserVirtualWindow\(visibleEntries,\s*detailsBodyScrollTop,\s*detailsBodyHeight\)/);
  assert.match(emitSource, /totalRows:\s*visibleEntries\.length/);
});

test('folder search input refreshes filtered visible-row icon priority without scroll events', () => {
  const handlerSource = surface.slice(
    surface.indexOf('async function handleStackSearchInput'),
    surface.indexOf('function emitVisibleRowsWindowChanged')
  );
  assert.match(surface, /on:input=\{handleStackSearchInput\}/);
  assert.match(handlerSource, /searchQuery = input\.value/);
  assert.match(handlerSource, /detailsBodyScrollTop = 0/);
  assert.match(handlerSource, /detailsBody\.scrollTop = 0/);
  assert.match(handlerSource, /await tick\(\)/);
  assert.match(handlerSource, /emitVisibleRowsWindowChanged\(\)/);
});

test('icon progress counts queued visible-priority rows as unresolved', () => {
  assert.match(surface, /function unresolvedIconHydrationCount\(\)/);
  assert.match(surface, /let\s+iconHydrationInFlightPathCount\s*=\s*0/);
  assert.match(surface, /iconHydrationInFlightPathCount \+= batchPaths\.length/);
  assert.match(surface, /iconHydrationInFlightPathCount = Math\.max\(0,\s*iconHydrationInFlightPathCount - paths\.length\)/);
  assert.match(surface, /iconHydrationPending\.length \+ iconHydrationVisiblePriority\.length \+ iconHydrationInFlightPathCount/);
  assert.match(surface, /iconHydrationResolvedCount = resolvedIconHydrationCount\(\)/);
  assert.doesNotMatch(surface, /iconHydrationPending\.length \+ iconHydrationVisiblePriority\.length \+ iconHydrationInFlight\b/);
  assert.doesNotMatch(surface, /targetCount - iconHydrationPending\.length\)/);
});

test('icon hydration applies one merged state update per resolved batch', () => {
  const resolveIconBatch = surface.slice(surface.indexOf('async function resolveIconBatch'), surface.indexOf('function stackIconUpdatesFromBatch'));
  assert.match(resolveIconBatch, /const updates = stackIconUpdatesFromBatch\(batch\.items\)/);
  assert.equal((resolveIconBatch.match(/applyStackEntryIconUpdates/g) ?? []).length, 1);
  assert.doesNotMatch(resolveIconBatch, /for\s*\([^)]*updates[^)]*\)[\s\S]*applyStackEntryIconUpdates/);
});

test('icon cache miss resolves shell icon outside the cache mutex', () => {
  assert.match(rustIcons, /fn cached_stack_icon_lookup/);
  assert.match(rustIcons, /fn store_stack_icon_cache_result/);
  const resolverSource = rustIcons.slice(
    rustIcons.indexOf('fn resolve_stack_item_icon'),
    rustIcons.indexOf('fn cached_stack_icon_lookup')
  );
  assert.match(resolverSource, /if let Some\(cached\) = cached_stack_icon_lookup\(&cache_key\)/);
  assert.match(resolverSource, /let icon_data_url = resolve_shell_icon_data_url\(&trimmed_path\)/);
  assert.match(resolverSource, /store_stack_icon_cache_result\(cache_key,\s*icon_data_url\.clone\(\)\)/);
  assert.doesNotMatch(resolverSource, /cache\.lock\(\)[\s\S]*resolve_shell_icon_data_url/);
});
