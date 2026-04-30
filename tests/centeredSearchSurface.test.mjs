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
