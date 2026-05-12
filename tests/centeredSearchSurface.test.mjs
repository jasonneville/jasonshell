import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  IPC_COMMANDS
} from '../dist-tests/ipc/commands.js';
import {
  isCenteredSearchSurfaceContract
} from '../dist-tests/lib/searchPanel.js';

test('centered search command is registered in the frontend IPC map', () => {
  assert.equal(IPC_COMMANDS.showCenteredSearchPanel, 'show_centered_search_panel');
  assert.equal(IPC_COMMANDS.resizeSearchPanel, 'resize_search_panel');
});

test('top bar opens configured centered search from focus and typing without Ctrl+K reliance', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  assert.match(source, /loadShellSettings/);
  assert.match(source, /searchModeFromSettings\(settings\.ui\.searchMode\)/);
  assert.match(source, /function openConfiguredPanel/);
  assert.match(source, /openCenteredPanel/);
  assert.match(source, /function publishImmediateSearchInputState\(nextQuery: string\)/);
  assert.match(source, /function startImmediateSearchQueryExecution/);
  assert.match(source, /const nextQuery = \(event\.currentTarget as HTMLInputElement\)\.value;/);
  assert.match(source, /const request = publishImmediateSearchInputState\(nextQuery\)/);
  assert.match(source, /startImmediateSearchQueryExecution\(request\)/);
  assert.match(source, /queueSearchPanelPublish\(\{[\s\S]*phase: request\.query \? 'typing' : 'complete'/);
  assert.match(source, /scheduleSearchEngine\(searchQuery\)/);
  assert.match(source, /function handleSearchFocus\(\) \{[\s\S]*openConfiguredPanel\(\);/);
  assert.match(source, /on:focus=\{handleSearchFocus\}/);
  assert.doesNotMatch(source, /Ctrl K/);
});

test('top-bar direct input path publishes lightweight pending state and queues deferred search work', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const inputHandler = source.match(/function handleSearchInput\(event: Event\) \{[\s\S]*?\n  \}/)?.[0] ?? '';

  assert.match(inputHandler, /const nextQuery = \(event\.currentTarget as HTMLInputElement\)\.value;/);
  assert.match(inputHandler, /const request = publishImmediateSearchInputState\(nextQuery\)/);
  assert.match(inputHandler, /startImmediateSearchQueryExecution\(request\)/);
  assert.doesNotMatch(inputHandler, /queueSearchQueryProcessing/);
  assert.doesNotMatch(inputHandler, /applySearchQuery\(/);
  assert.doesNotMatch(inputHandler, /queueSearchPanelPublish\(/);
  assert.doesNotMatch(inputHandler, /openConfiguredPanel\(/);
  assert.doesNotMatch(inputHandler, /showSearchPanel\(/);
  assert.doesNotMatch(inputHandler, /showCenteredSearchPanel\(/);
  assert.doesNotMatch(inputHandler, /searchEngine\(/);
  assert.doesNotMatch(inputHandler, /buildVisibleSearchRows\(/);
  assert.doesNotMatch(inputHandler, /nextProgressiveSearchResultSet\(/);
  assert.doesNotMatch(inputHandler, /mergeSearchPanelResultsByStableKey\(/);

  assert.doesNotMatch(source, /let queuedSearchQueryTimer: number \| null = null;/);
  assert.doesNotMatch(source, /const SEARCH_QUERY_PROCESSING_DELAY_MS = 16;/);
  assert.doesNotMatch(source, /queueSearchQueryProcessing|flushQueuedSearchQuery|searchQueryProcessingQueue/);
});

test('centered search surface contract accepts screen-center combobox payloads', () => {
  assert.equal(isCenteredSearchSurfaceContract({
    label: 'search-panel',
    mode: 'centeredHotkey',
    requestId: 'req-1',
    query: 'plan',
    sequence: 10,
    anchor: 'screenCenter',
    closeReasons: ['escape', 'outsideClick', 'focusLoss', 'activation'],
    accessibility: {
      role: 'combobox',
      listboxId: 'search-results',
      activeOptionId: 'search-result-0'
    }
  }), true);
});

test('search panel surface owns centered input and resize grip wiring', () => {
  const source = readFileSync(new URL('../src/components/SearchPanelSurface.svelte', import.meta.url), 'utf8');
  assert.match(source, /SEARCH_PANEL_QUERY_EVENT/);
  assert.match(source, /SEARCH_PANEL_KEY_EVENT/);
  assert.match(source, /class="search-panel-query"/);
  assert.match(source, /class="search-resize-grip"/);
  assert.match(source, /resizeSearchPanel\(size\)/);
  assert.match(source, /writeCenteredSearchPanelSize/);
});

test('centered search surface targets top-bar explicitly for cross-window search intents', () => {
  const source = readFileSync(new URL('../src/components/SearchPanelSurface.svelte', import.meta.url), 'utf8');
  const queryEmit = source.match(/function queueQueryEmit\(value: string\) \{[\s\S]*?\n  \}/)?.[0] ?? '';
  assert.match(source, /type SearchPanelQueryPayload/);
  assert.match(source, /let queryInputSequence = 0;/);
  assert.match(source, /import \{ emitTo \} from '@tauri-apps\/api\/event';/);
  assert.match(source, /topBarWebviewWindowEventTarget/);
  assert.match(source, /const topBarTarget = topBarWebviewWindowEventTarget\(\);/);
  assert.match(source, /queueQueryEmit\(value\)/);
  assert.match(queryEmit, /queryInputSequence \+= 1;/);
  assert.match(queryEmit, /const payload: SearchPanelQueryPayload = \{/);
  assert.match(queryEmit, /query: value/);
  assert.match(queryEmit, /inputSequence: queryInputSequence/);
  assert.match(queryEmit, /emitTo\(topBarTarget, SEARCH_PANEL_QUERY_EVENT, payload\)/);
  assert.match(source, /emitTo\(topBarTarget, SEARCH_PANEL_KEY_EVENT, event\.key\)/);
  assert.match(source, /emitTo\(topBarTarget, SEARCH_PANEL_SELECT_EVENT, searchVisibleRowIdentity\(row\)\)/);
  assert.match(source, /emitTo\(topBarTarget, SEARCH_PANEL_ACTIVATE_EVENT, searchVisibleRowIdentity\(row\)\)/);
  assert.match(source, /emitTo\(topBarTarget, SEARCH_PANEL_INTERACTION_EVENT, null\)/);
});

test('centered search surface keeps a local optimistic query draft while backend search catches up', () => {
  const source = readFileSync(new URL('../src/components/SearchPanelSurface.svelte', import.meta.url), 'utf8');
  assert.match(source, /let optimisticQueryDraft: string \| null = null;/);
  assert.doesNotMatch(source, /let pendingQueryEmitValue: string \| null = null;/);
  assert.doesNotMatch(source, /let queryEmitTimer: number \| null = null;/);
  assert.match(source, /\$: displayedQuery = optimisticQueryDraft \?\? query;/);
  assert.match(source, /optimisticQueryDraft = value;/);
  assert.match(source, /value=\{displayedQuery\}/);
  assert.match(source, /if \(panelState\.query === '' \|\| optimisticQueryDraft === panelState\.query\) \{/);
  const queryEmit = source.match(/function queueQueryEmit\(value: string\) \{[\s\S]*?\n  \}/)?.[0] ?? '';
  assert.match(queryEmit, /function queueQueryEmit\(value: string\) \{/);
  assert.match(queryEmit, /emitTo\(topBarTarget, SEARCH_PANEL_QUERY_EVENT, payload\)/);
  assert.doesNotMatch(queryEmit, /window\.setTimeout|pendingQueryEmitValue|queryEmitTimer|queuedValue/);
});

test('centered search surface hides immediately on escape and only refocuses when panel focus is elsewhere', () => {
  const source = readFileSync(new URL('../src/components/SearchPanelSurface.svelte', import.meta.url), 'utf8');
  assert.match(source, /import \{\s*getSearchPanelPayload,\s*hideSearchPanel,/);
  assert.match(source, /function hideCenteredPanelImmediately\(\) \{/);
  assert.match(source, /queryInput\?\.blur\(\);/);
  assert.match(source, /void hideSearchPanel\(\)\.catch\(\(\) => undefined\);/);
  assert.match(source, /emitTo\(topBarTarget, SEARCH_PANEL_KEY_EVENT, 'Escape'\)/);
  assert.match(source, /if \(event\.key === 'Escape' && presentation === 'centered'\) \{/);
  assert.match(source, /if \(presentation === 'centered'\) \{\s*hideCenteredPanelImmediately\(\);/);
  assert.match(source, /function shouldFocusCenteredQueryInput\(\) \{/);
  assert.match(source, /document\.activeElement === queryInput/);
  assert.match(source, /document\.activeElement instanceof HTMLElement && document\.activeElement\.closest\('\.search-panel'\)/);
  assert.doesNotMatch(source, /if \(event\.payload\.presentation === 'centered'\) \{\s*void focusQueryInput\(\);/);
});
