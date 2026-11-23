// Review H2/L1: implicit selection never lands on a weak or fallback row while stronger rows exist,
// and identity resolution is strict about stale record keys. Behaviour tests on dist-tests.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  implicitSearchSelectionIndex,
  isImplicitlySelectable,
  isWebSearchFallback,
  resolveVisibleSearchRowResultIndex,
  streamedSearchSelection
} from '../dist-tests/features/search/searchUxState.js';
import { searchEngineResponseToPanelPayload } from '../dist-tests/lib/searchEngine.js';

function panel(id, kind, providerId, matchReason, extra = {}) {
  return { id, kind, providerId, title: id, subtitle: '', terms: '', priority: 0, recordKey: id, matchReason, ...extra };
}

const strongQuick = panel('quick:a', 'command', 'commands', 'prefix');
const exactQuick = panel('quick:e', 'command', 'commands', 'exact');
const wordQuick = panel('quick:w', 'command', 'commands', 'wordPrefix');
const substringQuick = panel('quick:s', 'command', 'commands', 'substring');
const unknownQuick = panel('quick:u', 'command', 'commands', undefined);
const webSearch = panel('web:search', 'web', 'web', 'webSearch', { recordKey: 'web:search:q' });
const bareUrl = panel('web:open-url', 'web', 'web', 'bareDomain', { recordKey: 'web:open-url:https://logo.ai/' });
const explicitUrl = panel('web:open-url', 'web', 'web', 'url', { recordKey: 'web:open-url:https://github.com/' });
const file = panel('everything:a', 'file', 'everything', 'prefix');
const app = panel('app:a', 'app', 'apps', 'exact');
const builtin = panel('command:open-control-plane', 'command', 'commands', 'prefix');

test('web fallback and weak rows are not implicitly selectable; strong rows are', () => {
  assert.equal(isWebSearchFallback(webSearch), true);
  assert.equal(isWebSearchFallback(bareUrl), false);
  for (const weak of [webSearch, bareUrl, wordQuick, substringQuick, unknownQuick]) {
    assert.equal(isImplicitlySelectable(weak), false, weak.id + ':' + weak.matchReason);
  }
  for (const strong of [strongQuick, exactQuick, explicitUrl, file, app, builtin]) {
    assert.equal(isImplicitlySelectable(strong), true, strong.id + ':' + strong.matchReason);
  }
});

test('implicit selection skips weak and fallback rows while a strong row exists', () => {
  assert.equal(implicitSearchSelectionIndex([wordQuick, file]), 1);
  assert.equal(implicitSearchSelectionIndex([bareUrl, app, webSearch]), 1);
  assert.equal(implicitSearchSelectionIndex([webSearch, file]), 1);
  assert.equal(implicitSearchSelectionIndex([file, wordQuick, webSearch]), 0);
  assert.equal(implicitSearchSelectionIndex([explicitUrl, webSearch]), 0);
});

test('a lone weak quick command is never implicitly selected, so Enter cannot run it', () => {
  assert.equal(implicitSearchSelectionIndex([wordQuick]), -1);
  assert.equal(implicitSearchSelectionIndex([substringQuick]), -1);
  assert.equal(implicitSearchSelectionIndex([unknownQuick]), -1);
  assert.equal(implicitSearchSelectionIndex([wordQuick, webSearch]), -1);
});

test('when every row is a web row the first one is selected; nothing means -1', () => {
  assert.equal(implicitSearchSelectionIndex([webSearch]), 0);
  assert.equal(implicitSearchSelectionIndex([bareUrl, webSearch]), 0);
  assert.equal(implicitSearchSelectionIndex([]), -1);
});

test('the response-to-panel payload applies implicit selection when no explicit index is supplied', () => {
  const response = (rows) => ({
    query: 'report', sequence: 1, providerTimings: [], health: [], generatedAt: '2026-11-23T10:00:00.000Z',
    results: rows.map((row) => ({
      id: row.id, providerId: row.providerId, kind: row.kind, title: row.title, terms: [], aliases: [], score: 1,
      matchReason: row.matchReason ?? '', recordKey: row.recordKey,
      action: row.kind === 'web' ? { kind: 'openWebUrl', url: 'https://example.com/' }
        : row.kind === 'file' ? { kind: 'openFile', path: 'C:\\a.txt' }
          : { kind: 'runCommand', commandId: row.id }
    }))
  });
  assert.equal(searchEngineResponseToPanelPayload(response([wordQuick, file])).selectedIndex, 1);
  assert.equal(searchEngineResponseToPanelPayload(response([wordQuick])).selectedIndex, -1);
  assert.equal(searchEngineResponseToPanelPayload(response([file, webSearch])).selectedIndex, 0);
});

test('identity resolution requires the recordKey to match when one is supplied', () => {
  const rows = [webSearch, panel('calculator:result', 'calculator', 'calculator', 'calculator', { recordKey: 'calculator:result:6*7' })];
  assert.equal(resolveVisibleSearchRowResultIndex(rows, { id: 'web:search', recordKey: 'web:search:q', resultIndex: 0 }), 0);
  assert.equal(resolveVisibleSearchRowResultIndex(rows, { id: 'web:search', recordKey: 'web:search:old', resultIndex: 0 }), -1);
  assert.equal(resolveVisibleSearchRowResultIndex(rows, { id: 'calculator:result', recordKey: 'calculator:result:6*8', resultIndex: 1 }), -1);
  assert.equal(resolveVisibleSearchRowResultIndex(rows, { id: 'gone', recordKey: 'gone', resultIndex: 0 }), -1);
  // Legacy string identity (id only) keeps working.
  assert.equal(resolveVisibleSearchRowResultIndex(rows, 'web:search'), 0);
});

test('streamed selection follows the chosen row by id and recordKey, never by position', () => {
  const rows = [file, app, webSearch];
  assert.deepEqual(streamedSearchSelection(rows, { id: 'app:a', recordKey: 'app:a' }), { index: 1, chosen: true });
  const reordered = [app, file, webSearch];
  assert.deepEqual(streamedSearchSelection(reordered, { id: 'app:a', recordKey: 'app:a' }), { index: 0, chosen: true });
  assert.deepEqual(streamedSearchSelection(reordered, { id: 'everything:a', recordKey: 'everything:a' }), { index: 1, chosen: true });
});

test('streamed selection falls back to the implicit choice when nothing was chosen or the chosen row vanished', () => {
  assert.deepEqual(streamedSearchSelection([wordQuick, file, webSearch], null), { index: 1, chosen: false });
  assert.deepEqual(streamedSearchSelection([wordQuick, file], { id: 'gone', recordKey: 'gone' }), { index: 1, chosen: false });
  assert.deepEqual(streamedSearchSelection([file], { id: 'everything:a', recordKey: 'stale-key' }), { index: 0, chosen: false });
  assert.deepEqual(streamedSearchSelection([wordQuick], null), { index: -1, chosen: false });
  assert.deepEqual(streamedSearchSelection([], null), { index: 0, chosen: false });
});

test('a streamed update never moves a chosen weak row off its identity or onto a new top row', () => {
  const first = streamedSearchSelection([file, wordQuick], { id: 'quick:w', recordKey: 'quick:w' });
  assert.equal(first.index, 1);
  const after = streamedSearchSelection([app, file, wordQuick], { id: 'quick:w', recordKey: 'quick:w' });
  assert.deepEqual(after, { index: 2, chosen: true });
});

// Review M1/M2: third argument is `interim` (default false = final payload).
test('an interim payload that lacks the chosen row selects nothing but remembers the choice', () => {
  const chosen = { id: 'web:search', recordKey: 'web:search:q' };
  assert.deepEqual(streamedSearchSelection([file, app], chosen, true), { index: -1, chosen: true });
  assert.deepEqual(streamedSearchSelection([], chosen, true), { index: -1, chosen: true });
});

test('an interim payload that still has the chosen row follows it by identity', () => {
  assert.deepEqual(streamedSearchSelection([file, webSearch], { id: 'web:search', recordKey: 'web:search:q' }, true), { index: 1, chosen: true });
});

test('an interim payload with no choice uses the implicit selection', () => {
  assert.deepEqual(streamedSearchSelection([wordQuick, file], null, true), { index: 1, chosen: false });
  assert.deepEqual(streamedSearchSelection([wordQuick], null, true), { index: -1, chosen: false });
});

test('the final payload reselects the chosen row, or falls back to implicit and forgets the choice', () => {
  const chosen = { id: 'web:search', recordKey: 'web:search:q' };
  assert.deepEqual(streamedSearchSelection([file, webSearch], chosen, false), { index: 1, chosen: true });
  assert.deepEqual(streamedSearchSelection([file, app], chosen, false), { index: 0, chosen: false });
  assert.deepEqual(streamedSearchSelection([file, app], chosen), { index: 0, chosen: false });
});

const bookmark = (reason) => panel('bookmark:https://domain.com/', 'bookmark', 'bookmarks', reason);

test('interim payloads only let exact/prefix bookmarks be implicitly selected; final payloads allow any', () => {
  for (const reason of ['exact', 'prefix']) {
    assert.equal(isImplicitlySelectable(bookmark(reason), true), true, reason);
  }
  for (const reason of ['tokenPrefix', 'substring', undefined]) {
    assert.equal(isImplicitlySelectable(bookmark(reason), true), false, String(reason));
    assert.equal(isImplicitlySelectable(bookmark(reason), false), true, 'final ' + String(reason));
  }
  assert.equal(implicitSearchSelectionIndex([bookmark('substring'), file], true), 1);
  assert.equal(implicitSearchSelectionIndex([bookmark('substring')], true), -1);
  assert.equal(implicitSearchSelectionIndex([bookmark('substring'), file], false), 0);
  assert.equal(implicitSearchSelectionIndex([bookmark('prefix'), file], true), 0);
});
