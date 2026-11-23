// Tier 1 (PRD-01/02/03): renderer contract behaviour for the Calculator, Web, Bookmarks and
// Quick Commands search providers. Behaviour tests against dist-tests; the only source read is the
// Rust IPC constant inventory via the shared helper.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { IPC_COMMANDS } from '../dist-tests/ipc/commands.js';
import {
  isSearchAction,
  isSearchEngineResponse,
  isSearchResult,
  mergeSearchPanelResultsByStableKey,
  searchEngineResultToPanelResult,
  validateSearchResultActionSafety
} from '../dist-tests/lib/searchEngine.js';
import { searchResultActionId } from '../dist-tests/lib/searchPanel.js';
import { readRust } from './support/rustSource.mjs';

const generatedAt = '2026-11-23T10:00:00.000Z';
// Canonical wire shapes shared with the Rust serializer test (tests/fixtures/search-actions.json).
const A = JSON.parse(readFileSync(new URL('./fixtures/search-actions.json', import.meta.url), 'utf8')).actions;

function result(overrides) {
  return {
    id: 'x',
    providerId: 'web',
    kind: 'web',
    title: 'Title',
    action: A.openWebUrl,
    terms: [],
    aliases: [],
    score: 100,
    matchReason: 'token',
    recordKey: 'x',
    ...overrides
  };
}

test('web and bookmark actions accept clean http(s) URLs only', () => {
  for (const kind of ['openWebUrl', 'openBookmark']) {
    assert.equal(isSearchAction({ kind, url: 'https://github.com/rust-lang' }), true);
    assert.equal(isSearchAction({ kind, url: 'http://example.org/a?b=c' }), true);
    for (const url of [
      'javascript:alert(1)',
      'JaVaScRiPt:alert(1)',
      'file:///C:/Windows/notepad.exe',
      'data:text/html,hi',
      'ftp://example.com/',
      'ms-settings:display',
      'https://user:pass@example.com/',
      'https://user@example.com/',
      'https://',
      'http://',
      ' https://example.com/',
      'https://example.com/a b',
      'https://example.com/\n',
      'https://example.com/\u0007',
      'https://example.com\\evil',
      'example.com',
      ''
    ]) {
      assert.equal(isSearchAction({ kind, url }), false, `${kind} ${JSON.stringify(url)}`);
    }
  }
});

test('one unsafe web action invalidates the whole search response', () => {
  const good = result({ id: 'web:search', recordKey: 'web:search' });
  const bad = result({ id: 'web:evil', action: { kind: 'openWebUrl', url: 'https://u:p@example.com/' } });
  const response = (results) => ({
    query: 'q', sequence: 1, results, providerTimings: [], health: [], generatedAt
  });
  assert.equal(isSearchEngineResponse(response([good])), true);
  assert.equal(isSearchEngineResponse(response([good, bad])), false);
  assert.equal(isSearchResult(bad), false);
  assert.equal(validateSearchResultActionSafety(bad), false);
});

test('calculator rows normalise to copyCalculatorResult carrying exactly the plain result text', () => {
  const panel = searchEngineResultToPanelResult(result({
    id: 'calculator:result',
    providerId: 'calculator',
    kind: 'calculator',
    title: '42',
    subtitle: '0x2A - 0b101010',
    action: A.copyText,
    recordKey: 'calculator:result'
  }));
  assert.equal(panel.kind, 'calculator');
  assert.equal(panel.providerId, 'calculator');
  assert.equal(panel.actionId, 'copyCalculatorResult');
  assert.equal(panel.copyText, '42');
  assert.equal(panel.url, undefined);
  assert.equal(searchResultActionId(panel), 'copyCalculatorResult');
});

test('web, bookmark and quick-command rows normalise to the actions the activation handler dispatches on', () => {
  const web = searchEngineResultToPanelResult(result({
    id: 'web:search', recordKey: 'web:search',
    action: A.openWebSearch
  }));
  assert.equal(web.actionId, 'openWebUrl');
  assert.equal(web.url, A.openWebSearch.url);
  assert.equal(web.copyText, undefined);

  const bookmark = searchEngineResultToPanelResult(result({
    id: 'bookmark:https://svelte.dev/', providerId: 'bookmarks', kind: 'bookmark',
    action: A.openBookmark, recordKey: 'bookmark:https://svelte.dev/'
  }));
  assert.equal(bookmark.actionId, 'openBookmark');
  assert.equal(bookmark.url, A.openBookmark.url);

  const quick = searchEngineResultToPanelResult(result({
    id: 'quick:build', providerId: 'commands', kind: 'command', title: 'Build project',
    subtitle: 'Quick command', action: A.runCommandQuick,
    recordKey: 'quick:build'
  }));
  assert.equal(quick.actionId, 'runCommand');
  assert.equal(quick.providerId, 'commands');
  assert.equal(quick.id, 'quick:build');
  // The renderer row never carries command text, args, cwd or any path for a quick command.
  assert.equal(quick.path, undefined);
  assert.equal(quick.copyText, undefined);
});

test('the web fallback keeps its identity when progress payloads re-merge', () => {
  const fallback = searchEngineResultToPanelResult(result({ id: 'web:search', recordKey: 'web:search' }));
  const app = searchEngineResultToPanelResult(result({
    id: 'app:a', providerId: 'apps', kind: 'app', title: 'A', action: A.openApp, recordKey: 'app:a'
  }));
  const merged = mergeSearchPanelResultsByStableKey([app, fallback], [fallback, app]);
  assert.deepEqual(merged.map((row) => row.id), ['app:a', 'web:search']);
});

test('new search activation IPC commands are named in the renderer registry and Rust contract', () => {
  assert.equal(IPC_COMMANDS.openSearchUrl, 'open_search_url');
  assert.equal(IPC_COMMANDS.copySearchText, 'copy_search_text');
  assert.equal(IPC_COMMANDS.runSearchQuickCommand, 'run_search_quick_command');
  const contracts = readRust('src-tauri/src/contracts.rs');
  assert.match(contracts, /pub const OPEN_SEARCH_URL: &str = "open_search_url";/u);
  assert.match(contracts, /pub const RUN_SEARCH_QUICK_COMMAND: &str = "run_search_quick_command";/u);
  assert.match(contracts, /pub const COPY_SEARCH_TEXT: &str = "copy_search_text";/u);
  const main = readRust('src-tauri/src/main.rs');
  assert.match(main, /\bopen_search_url\b/u);
  assert.match(main, /\brun_search_quick_command\b/u);
  assert.match(main, /\bcopy_search_text\b/u);
});

// Visible-row reordering moves apps beyond the fourth leading one behind the tail. That must never
// push the web-search fallback (always supplied last by the backend) above any other row.
import { buildVisibleSearchRows } from '../dist-tests/features/search/searchUxState.js';

function panelRow(id, kind, providerId, extra = {}) {
  return { id, kind, providerId, title: id, subtitle: '', terms: '', priority: 0, recordKey: id, ...extra };
}

test('web fallback stays the last visible row even when more than four apps lead the result list', () => {
  const apps = Array.from({ length: 7 }, (_, i) => panelRow(`app:${i}`, 'app', 'apps'));
  const rows = buildVisibleSearchRows([...apps, panelRow('web:search', 'web', 'web')]);
  assert.equal(rows.at(-1).id, 'web:search', rows.map((row) => row.id).join(','));
  assert.equal(rows.length, 8);
});

test('web fallback stays last behind apps plus a non-app tail (files, bookmarks)', () => {
  const apps = Array.from({ length: 6 }, (_, i) => panelRow(`app:${i}`, 'app', 'apps'));
  const tail = [panelRow('file:a', 'file', 'everything'), panelRow('bookmark:b', 'bookmark', 'bookmarks')];
  const rows = buildVisibleSearchRows([...apps, ...tail, panelRow('web:search', 'web', 'web')]);
  assert.equal(rows.at(-1).id, 'web:search', rows.map((row) => row.id).join(','));
});

test('a leading calculator row stays first and keeps its original result index', () => {
  const rows = buildVisibleSearchRows([
    panelRow('calculator:result', 'calculator', 'calculator'),
    ...Array.from({ length: 6 }, (_, i) => panelRow(`app:${i}`, 'app', 'apps'))
  ]);
  assert.equal(rows[0].id, 'calculator:result');
  assert.equal(rows[0].resultIndex, 0);
});

test('every canonical fixture action is accepted by isSearchAction and survives a response round trip', () => {
  assert.equal(Object.keys(A).length >= 10, true);
  for (const [name, action] of Object.entries(A)) {
    assert.equal(isSearchAction(action), true, name);
    const row = result({ id: `fixture:${name}`, recordKey: `fixture:${name}`, providerId: 'apps', kind: action.kind === 'openApp' ? 'app' : 'command', action });
    if (action.kind === 'openSetting' || action.kind === 'runControlPanel') row.kind = 'setting';
    const response = { query: 'q', sequence: 1, results: [row], providerTimings: [], health: [], generatedAt };
    assert.equal(isSearchEngineResponse(response), true, name);
    assert.deepEqual(JSON.parse(JSON.stringify(response)).results[0].action, action, name);
  }
  assert.equal('args' in A.runControlPanelNoArgs, false, 'no args key when absent');
});

test('panel normalisation carries matchReason so the renderer can tell weak rows apart', () => {
  const panel = searchEngineResultToPanelResult(result({ id: 'quick:x', providerId: 'commands', kind: 'command', action: A.runCommandQuick, matchReason: 'wordPrefix' }));
  assert.equal(panel.matchReason, 'wordPrefix');
});
