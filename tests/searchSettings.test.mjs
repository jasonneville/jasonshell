import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  DEFAULT_SEARCH_SETTINGS,
  EVERYTHING_INSTALL_MODES,
  EVERYTHING_SDK_SOURCES,
  EVERYTHING_SORT_MODES,
  SEARCH_MODES,
  WEB_SEARCH_ENGINES,
  coerceSearchSettings,
  defaultSearchSettings,
  isEverythingSetupConsentAllowed
} from '../dist-tests/lib/searchSettings.js';
import { assertNoSecretSettingKeys, defaultShellSettings } from '../dist-tests/lib/settings.js';

test('search settings defaults match JSON-owned Everything safe defaults', () => {
  assert.deepEqual(defaultSearchSettings(), {
    ui: {
      searchMode: 'centeredHotkey'
    },
    search: {
      resultLimit: 50,
      webEngine: 'google',
      everything: {
        enabled: true,
        installMode: 'ask',
        sdkSource: 'system',
        maxResults: 100,
        fullPathSearch: true,
        sort: 'nameAsc',
        contentSearchEnabled: false
      }
    }
  });
  assert.deepEqual(DEFAULT_SEARCH_SETTINGS, defaultSearchSettings());
});

test('shell settings include search settings as behavior source of truth', () => {
  const settings = defaultShellSettings();

  assert.deepEqual(settings.ui.searchMode, 'centeredHotkey');
  assert.deepEqual(settings.search, defaultSearchSettings().search);
});

test('v1 settings without search fields coerce to search defaults', () => {
  const legacy = {
    schema: 'jasonshell.settings',
    version: 1,
    ui: {
      activeWorkspaceId: 'workspace-a',
      enableDiagnosticsExport: true
    },
    workspaces: [],
    taskHistory: []
  };

  const normalized = coerceSearchSettings(legacy);

  assert.equal(normalized.ui.searchMode, 'centeredHotkey');
  assert.equal(normalized.search.everything.enabled, true);
  assert.equal(normalized.search.everything.installMode, 'ask');
  assert.equal(normalized.search.everything.contentSearchEnabled, false);
});

test('invalid search setting enums and bounds normalize to documented safe defaults', () => {
  const normalized = coerceSearchSettings({
    ui: {
      searchMode: 'popup'
    },
    search: {
      resultLimit: 10_000,
      everything: {
        enabled: true,
        installMode: 'silent',
        sdkSource: 'remote',
        maxResults: -1,
        fullPathSearch: true,
        sort: 'random',
        contentSearchEnabled: true
      }
    }
  });

  assert.equal(normalized.ui.searchMode, 'centeredHotkey');
  assert.equal(normalized.search.resultLimit, 50);
  assert.equal(normalized.search.everything.installMode, 'ask');
  assert.equal(normalized.search.everything.sdkSource, 'system');
  assert.equal(normalized.search.everything.maxResults, 100);
  assert.equal(normalized.search.everything.sort, 'nameAsc');
});

test('search settings bounds match Rust normalization limits', () => {
  const normalized = coerceSearchSettings({
    search: {
      resultLimit: 100,
      everything: {
        maxResults: 200
      }
    }
  });

  assert.equal(normalized.search.resultLimit, 100);
  assert.equal(normalized.search.everything.maxResults, 200);
});

test('search settings reject secret-like keys before persistence', () => {
  assert.doesNotThrow(() => assertNoSecretSettingKeys(defaultShellSettings()));
  assert.throws(
    () =>
      assertNoSecretSettingKeys({
        search: {
          everything: {
            apiToken: 'not-allowed'
          }
        }
      }),
    /search\.everything\.apiToken/
  );
});

test('everything setup consent blocks unsafe download, bundle, or execution paths', () => {
  assert.deepEqual(SEARCH_MODES, ['topRight', 'centeredHotkey']);
  assert.deepEqual(EVERYTHING_INSTALL_MODES, ['ask', 'disabled', 'managed']);
  assert.deepEqual(EVERYTHING_SDK_SOURCES, ['bundled', 'system']);
  assert.deepEqual(EVERYTHING_SORT_MODES, ['nameAsc', 'pathAsc', 'dateModifiedDesc', 'runCountDesc']);

  assert.equal(
    isEverythingSetupConsentAllowed({
      action: 'downloadInstaller',
      consent: true,
      officialUrl: 'https://www.voidtools.com/downloads/',
      artifactName: 'Everything-1.4.1.1032.x64-Setup.exe',
      version: '1.4.1.1032',
      sha256: 'a'.repeat(64),
      licenseApproved: true,
      provenanceApproved: true,
      requiresAdmin: true,
      explainsFilenameExposure: true
    }),
    true
  );
  assert.equal(
    isEverythingSetupConsentAllowed({
      action: 'downloadInstaller',
      consent: true,
      officialUrl: 'https://www.voidtools.com/downloads/',
      artifactName: 'Everything-1.4.1.1032.x64-Setup.exe',
      version: '1.4.1.1032',
      sha256: undefined,
      licenseApproved: true,
      provenanceApproved: true,
      requiresAdmin: true,
      explainsFilenameExposure: true
    }),
    false
  );
  assert.equal(
    isEverythingSetupConsentAllowed({
      action: 'launchInstalled',
      consent: true,
      officialUrl: 'https://www.voidtools.com/downloads/',
      artifactName: 'Everything-1.4.1.1032.x64-Setup.exe',
      version: '1.4.1.1032',
      sha256: 'a'.repeat(64),
      licenseApproved: true,
      provenanceApproved: true,
      requiresAdmin: false,
      explainsFilenameExposure: true
    }),
    true
  );
  assert.equal(
    isEverythingSetupConsentAllowed({
      action: 'launchInstalled',
      consent: false,
      officialUrl: 'https://www.voidtools.com/downloads/',
      artifactName: 'Everything-1.4.1.1032.x64-Setup.exe',
      version: '1.4.1.1032',
      sha256: 'a'.repeat(64),
      licenseApproved: true,
      provenanceApproved: true,
      requiresAdmin: false,
      explainsFilenameExposure: true
    }),
    false
  );
});

// Tier 1 PRD-02b: configurable web-search engine (fixed allowlist, default google).
test('web search engine allowlist is fixed and google is the default', () => {
  assert.deepEqual(WEB_SEARCH_ENGINES, ['google', 'duckduckgo', 'bing', 'brave']);
  assert.equal(defaultSearchSettings().search.webEngine, 'google');
});

test('legacy settings without webEngine migrate to google and valid engines persist', () => {
  assert.equal(coerceSearchSettings({ search: { resultLimit: 10 } }).search.webEngine, 'google');
  assert.equal(coerceSearchSettings({}).search.webEngine, 'google');
  for (const engine of WEB_SEARCH_ENGINES) {
    assert.equal(coerceSearchSettings({ search: { webEngine: engine } }).search.webEngine, engine);
  }
});

test('invalid or hostile webEngine values fall back to google', () => {
  for (const value of ['yahoo', 'Google', '', 'https://evil.example/?q=', 7, null, {}, ['bing']]) {
    assert.equal(coerceSearchSettings({ search: { webEngine: value } }).search.webEngine, 'google', String(value));
  }
});

test('shell settings carry webEngine inside search and default settings stay clone-isolated', () => {
  const settings = defaultShellSettings();
  assert.equal(settings.search.webEngine, 'google');
  settings.search.webEngine = 'brave';
  assert.equal(defaultSearchSettings().search.webEngine, 'google');
});
