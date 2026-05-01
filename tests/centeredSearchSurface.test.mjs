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
  assert.match(source, /function applySearchQuery\(nextQuery: string\)/);
  assert.match(source, /applySearchQuery\(\(event\.currentTarget as HTMLInputElement\)\.value\)/);
  assert.match(source, /queueSearchPanelPublish\(\{[\s\S]*phase: searchQuery\.trim\(\) \? 'typing' : 'complete'/);
  assert.match(source, /scheduleSearchEngine\(searchQuery\)/);
  assert.match(source, /on:focus=\{openConfiguredPanel\}/);
  assert.doesNotMatch(source, /Ctrl K/);
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
  assert.match(source, /import \{ emitTo \} from '@tauri-apps\/api\/event';/);
  assert.match(source, /topBarWebviewWindowEventTarget/);
  assert.match(source, /const topBarTarget = topBarWebviewWindowEventTarget\(\);/);
  assert.match(source, /queueQueryEmit\(value\)/);
  assert.match(source, /emitTo\(topBarTarget, SEARCH_PANEL_QUERY_EVENT, queuedValue\)/);
  assert.match(source, /emitTo\(topBarTarget, SEARCH_PANEL_KEY_EVENT, event\.key\)/);
  assert.match(source, /emitTo\(topBarTarget, SEARCH_PANEL_SELECT_EVENT, searchVisibleRowIdentity\(row\)\)/);
  assert.match(source, /emitTo\(topBarTarget, SEARCH_PANEL_ACTIVATE_EVENT, searchVisibleRowIdentity\(row\)\)/);
  assert.match(source, /emitTo\(topBarTarget, SEARCH_PANEL_INTERACTION_EVENT, null\)/);
});

test('centered search surface keeps a local optimistic query draft while backend search catches up', () => {
  const source = readFileSync(new URL('../src/components/SearchPanelSurface.svelte', import.meta.url), 'utf8');
  assert.match(source, /let optimisticQueryDraft: string \| null = null;/);
  assert.match(source, /let pendingQueryEmitValue: string \| null = null;/);
  assert.match(source, /let queryEmitTimer: number \| null = null;/);
  assert.match(source, /\$: displayedQuery = optimisticQueryDraft \?\? query;/);
  assert.match(source, /optimisticQueryDraft = value;/);
  assert.match(source, /value=\{displayedQuery\}/);
  assert.match(source, /if \(panelState\.query === '' \|\| optimisticQueryDraft === panelState\.query\) \{/);
  assert.match(source, /function queueQueryEmit\(value: string\) \{/);
  assert.match(source, /pendingQueryEmitValue = value;/);
  assert.match(source, /queryEmitTimer = window\.setTimeout\(\(\) => \{/);
  assert.match(source, /emitTo\(topBarTarget, SEARCH_PANEL_QUERY_EVENT, queuedValue\)/);
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
