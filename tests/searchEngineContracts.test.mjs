import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  SEARCH_ENGINE_COMMAND,
  SEARCH_ENGINE_PROGRESS_EVENT,
  isSafeControlPanelAction,
  isSafeMsSettingsUri,
  isSearchEngineResponse,
  isSearchProgressPayload,
  isSearchQueryRequest,
  mergeSearchPanelResultsByStableKey,
  isSearchResult,
  searchEngineProgressToPanelPayload,
  searchEngineResultToPanelResult,
  validateSearchResultActionSafety
} from '../dist-tests/lib/searchEngine.js';

const generatedAt = '2026-04-29T19:20:00.000Z';

function displaySettingsResult(overrides = {}) {
  return {
    id: 'setting:display',
    providerId: 'settings',
    kind: 'setting',
    title: 'Display Settings',
    subtitle: 'System display preferences',
    path: 'ms-settings:display',
    action: { kind: 'openSetting', uri: 'ms-settings:display' },
    terms: ['display', 'screen', 'monitor', 'settings'],
    aliases: ['screen settings', 'monitor settings'],
    score: 980,
    matchReason: 'settings-alias',
    recordKey: 'setting:display',
    ...overrides
  };
}

function providerTiming(overrides = {}) {
  return {
    providerId: 'settings',
    startedAt: generatedAt,
    endedAt: generatedAt,
    durationMs: 2.5,
    cache: 'hit',
    cacheAgeMs: 25,
    resultCount: 1,
    applied: true,
    discardedAsStale: false,
    ...overrides
  };
}

test('search engine request and response contracts match phase 1 shape', () => {
  assert.equal(SEARCH_ENGINE_COMMAND, 'search_engine');
  assert.equal(SEARCH_ENGINE_PROGRESS_EVENT, 'search-engine:progress');
  assert.equal(
    isSearchQueryRequest({
      query: 'display settings',
      sequence: 17,
      limit: 25,
      presentation: 'centered',
      context: {
        openWindows: [{ id: 'hwnd:1', title: 'Terminal', appName: 'Windows Terminal' }],
        workspaceRoots: ['C:\\dev\\jasonshell']
      }
    }),
    true
  );

  assert.equal(
    isSearchEngineResponse({
      query: 'display settings',
      sequence: 17,
      results: [displaySettingsResult()],
      providerTimings: [providerTiming()],
      health: [{ providerId: 'settings', state: 'ready', message: 'ready' }],
      generatedAt,
      diagnostics: {
        coordinator: 'search_engine.phase2.settings_only',
        legacyHotPathUsed: false,
        notes: ['settings provider only']
      }
    }),
    true
  );
});

test('search result validator requires explicit safe action contracts', () => {
  assert.equal(isSearchResult(displaySettingsResult()), true);
  assert.equal(
    isSearchResult(
      displaySettingsResult({
        action: { kind: 'openSetting', uri: 'https://example.com/settings' }
      })
    ),
    false
  );
  assert.equal(
    isSearchResult(
      displaySettingsResult({
        action: { kind: 'runControlPanel', executable: 'powershell.exe' }
      })
    ),
    false
  );
});

test('settings action safety allows ms-settings pages and control.exe only', () => {
  assert.equal(isSafeMsSettingsUri('ms-settings:'), true);
  assert.equal(isSafeMsSettingsUri('ms-settings:display'), true);
  assert.equal(isSafeMsSettingsUri('ms-settings:sound'), true);
  assert.equal(isSafeMsSettingsUri('ms-settings:windowsupdate'), true);
  assert.equal(isSafeMsSettingsUri('ms-settings:display;calc.exe'), false);
  assert.equal(isSafeMsSettingsUri('shell:AppsFolder'), false);

  assert.equal(
    isSafeControlPanelAction({ kind: 'runControlPanel', executable: 'control.exe' }),
    true
  );
  assert.equal(
    isSafeControlPanelAction({
      kind: 'runControlPanel',
      executable: 'control.exe',
      args: ['Microsoft.Sound']
    }),
    true
  );
  assert.equal(
    isSafeControlPanelAction({
      kind: 'runControlPanel',
      executable: 'control.exe',
      args: ['Microsoft.Sound', '& calc.exe']
    }),
    false
  );

  assert.equal(validateSearchResultActionSafety(displaySettingsResult()), true);
  assert.equal(
    validateSearchResultActionSafety(
      displaySettingsResult({
        action: { kind: 'runControlPanel', executable: 'control.exe' },
        path: 'control.exe',
        recordKey: 'settings:control-panel'
      })
    ),
    true
  );
});

test('control panel panel result keeps executable path separate from safe args', () => {
  const panelResult = searchEngineResultToPanelResult(
    displaySettingsResult({
      id: 'setting:control-panel-sound',
      title: 'Sound Control Panel',
      path: 'control.exe',
      action: {
        kind: 'runControlPanel',
        executable: 'control.exe',
        args: ['Microsoft.Sound']
      },
      recordKey: 'setting:control-panel-sound'
    })
  );

  assert.equal(panelResult.actionId, 'runControlPanel');
  assert.equal(panelResult.path, 'control.exe');
  assert.deepEqual(panelResult.actionArgs, ['Microsoft.Sound']);
});

test('progress payload supports immediate local rows and stale provider batches', () => {
  assert.equal(
    isSearchProgressPayload({
      query: 'sound settings',
      sequence: 22,
      phase: 'local',
      results: [
        displaySettingsResult({
          id: 'setting:sound',
          title: 'Sound Settings',
          path: 'ms-settings:sound',
          action: { kind: 'openSetting', uri: 'ms-settings:sound' },
          recordKey: 'setting:sound'
        })
      ],
      providerTimings: [providerTiming({ providerId: 'settings', resultCount: 1 })],
      statusMessage: '1 local result',
      generatedAt
    }),
    true
  );

  assert.equal(
    isSearchProgressPayload({
      query: 'disp',
      sequence: 21,
      phase: 'provider',
      results: [],
      providerTimings: [
        providerTiming({
          providerId: 'everything',
          resultCount: 20,
          applied: false,
          discardedAsStale: true
        })
      ],
      statusMessage: 'Discarded stale provider batch',
      generatedAt,
      stale: true
    }),
    true
  );
});

test('progress payload converts to panel payload and merges by stable record key', () => {
  const localPayload = searchEngineProgressToPanelPayload({
    query: 'sound settings',
    sequence: 22,
    phase: 'local',
    results: [
      displaySettingsResult({
        id: 'setting:sound',
        title: 'Sound Settings',
        titleHighlightData: [0, 5],
        path: 'ms-settings:sound',
        action: { kind: 'openSetting', uri: 'ms-settings:sound' },
        recordKey: 'setting:sound'
      })
    ],
    providerTimings: [providerTiming({ providerId: 'settings', resultCount: 1 })],
    statusMessage: '1 local result',
    generatedAt
  });
  const providerPayload = searchEngineProgressToPanelPayload({
    query: 'sound settings',
    sequence: 22,
    phase: 'provider',
    results: [
      displaySettingsResult({
        id: 'everything:file:c:/docs/sound.txt',
        providerId: 'everything',
        kind: 'file',
        title: 'sound.txt',
        subtitle: 'File',
        path: 'C:\\docs\\sound.txt',
        action: { kind: 'openFile', path: 'C:\\docs\\sound.txt' },
        terms: ['sound'],
        aliases: [],
        score: 400,
        matchReason: 'token',
        recordKey: 'file:c:\\docs\\sound.txt'
      }),
      displaySettingsResult({
        id: 'setting:sound-duplicate-id',
        title: 'Sound Settings',
        path: 'ms-settings:sound',
        action: { kind: 'openSetting', uri: 'ms-settings:sound' },
        recordKey: 'setting:sound'
      })
    ],
    providerTimings: [
      providerTiming({ providerId: 'settings', resultCount: 1 }),
      providerTiming({ providerId: 'everything', resultCount: 1 })
    ],
    statusMessage: 'Merged Everything results',
    generatedAt
  });

  const merged = mergeSearchPanelResultsByStableKey(localPayload.results, providerPayload.results);
  assert.equal(merged.length, 2);
  assert.equal(merged[0].recordKey, 'setting:sound');
  assert.deepEqual(merged[0].titleHighlightData, [0, 5]);
  assert.equal(merged[1].recordKey, 'file:c:\\docs\\sound.txt');
});

test('provider timing accepts persistent app-cache indexing state and cache age', () => {
  assert.equal(
    isSearchEngineResponse({
      query: 'spotify',
      sequence: 30,
      results: [],
      providerTimings: [
        providerTiming({
          providerId: 'apps',
          cache: 'indexing',
          cacheAgeMs: 120_000,
          resultCount: 0
        })
      ],
      health: [{ providerId: 'apps', state: 'indexing', message: 'building app index' }],
      generatedAt
    }),
    true
  );
});

test('search result contract accepts highlight spans for fuzzy matches', () => {
  const result = displaySettingsResult({
    id: 'app:spotify',
    providerId: 'apps',
    kind: 'app',
    title: 'Spotify',
    subtitle: 'Application',
    path: 'C:\\Apps\\Spotify.lnk',
    action: { kind: 'openApp', path: 'C:\\Apps\\Spotify.lnk' },
    terms: ['spotify'],
    aliases: ['Spotify'],
    score: 2520,
    matchReason: 'subsequence',
    recordKey: 'app:c:\\apps\\spotify.lnk',
    titleHighlightData: [0, 2, 3, 3],
    subtitleHighlightData: []
  });

  assert.equal(isSearchResult(result), true);
  assert.deepEqual(searchEngineResultToPanelResult(result).titleHighlightData, [0, 2, 3, 3]);
});

test('search result contract rejects malformed highlight span arrays', () => {
  assert.equal(
    isSearchResult(
      displaySettingsResult({
        titleHighlightData: [0, 2, 3]
      })
    ),
    false
  );
});

test('new search engine wrapper is isolated from legacy catalog and ranking hot paths', () => {
  const source = readFileSync(new URL('../src/lib/searchEngine.ts', import.meta.url), 'utf8');
  assert.doesNotMatch(source, /searchCatalog/);
  assert.doesNotMatch(source, /searchRanking/);
  assert.doesNotMatch(source, /systemSearch/);
  assert.doesNotMatch(source, /search_sources|windowsSearch|warmedCache/);
});
