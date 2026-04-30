import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  SEARCH_ENGINE_COMMAND,
  isSafeControlPanelAction,
  isSafeMsSettingsUri,
  isSearchEngineResponse,
  isSearchProgressPayload,
  isSearchQueryRequest,
  isSearchResult,
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
    resultCount: 1,
    applied: true,
    discardedAsStale: false,
    ...overrides
  };
}

test('search engine request and response contracts match phase 1 shape', () => {
  assert.equal(SEARCH_ENGINE_COMMAND, 'search_engine');
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

test.todo('phase 0 red: rust search engine still lacks progressive typing/local/provider batches');

test.todo('phase 0 red: rust app cache and fuzzy scoring gaps are still unimplemented');

test('new search engine wrapper is isolated from legacy catalog and ranking hot paths', () => {
  const source = readFileSync(new URL('../src/lib/searchEngine.ts', import.meta.url), 'utf8');
  assert.doesNotMatch(source, /searchCatalog/);
  assert.doesNotMatch(source, /searchRanking/);
  assert.doesNotMatch(source, /systemSearch/);
  assert.doesNotMatch(source, /search_sources|windowsSearch|warmedCache/);
});
