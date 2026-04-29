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
});

test('top bar branches Ctrl+K through JSON search mode without changing input echo path', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  assert.match(source, /loadShellSettings/);
  assert.match(source, /searchModeFromSettings\(settings\.ui\.searchMode\)/);
  assert.match(source, /ctrlKSearchAction\(searchMode\)/);
  assert.match(source, /openCenteredPanel/);
  assert.match(source, /searchQuery = \(event\.currentTarget as HTMLInputElement\)\.value/);
  assert.match(source, /queueSearchPanelPublish\(\)/);
  assert.match(source, /scheduleSearchRender\(searchQuery\)/);
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
