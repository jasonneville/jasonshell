import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  CENTERED_SEARCH_CLOSE_REASONS,
  SEARCH_ACTIVATION_KINDS,
  SEARCH_PANEL_LABEL,
  SEARCH_PROVIDER_HEALTH_REASON_CODES,
  SEARCH_PROVIDER_HEALTH_STATES,
  SEARCH_PROVIDER_IDS,
  SEARCH_RESULT_KINDS,
  isCenteredSearchSurfaceContract,
  isEverythingSetupConsentAllowed,
  isProviderHealthContract,
  isSearchActivationRequest,
  searchResultActionId
} from '../dist-tests/lib/searchPanel.js';
import { IPC_COMMANDS } from '../dist-tests/ipc/commands.js';

test('search result kind union expands without breaking current result callers', () => {
  assert.deepEqual(SEARCH_RESULT_KINDS, [
    'app',
    'window',
    'folder',
    'file',
    'command',
    'setting',
    'calculator',
    'web',
    'bookmark'
  ]);

  const legacy = {
    id: 'system:file:C:\\Users\\me\\notes.txt',
    kind: 'file',
    title: 'notes.txt',
    subtitle: 'File',
    terms: 'notes file',
    priority: 120,
    path: 'C:\\Users\\me\\notes.txt'
  };
  assert.equal(searchResultActionId(legacy), 'openFile');
});

test('provider health contract reports safe Everything setup states', () => {
  assert.deepEqual(SEARCH_PROVIDER_IDS, [
    'apps',
    'openWindows',
    'everything',
    'windowsSearch',
    'warmedCache',
    'commands',
    'calculator',
    'web',
    'bookmarks'
  ]);
  assert.deepEqual(SEARCH_PROVIDER_HEALTH_STATES, [
    'ready',
    'degraded',
    'unavailable',
    'indexing',
    'adminRequired',
    'disabled'
  ]);
  assert.ok(SEARCH_PROVIDER_HEALTH_REASON_CODES.includes('sdkMissing'));
  assert.ok(SEARCH_PROVIDER_HEALTH_REASON_CODES.includes('checksumBlocked'));

  assert.equal(
    isProviderHealthContract({
      providerId: 'everything',
      state: 'degraded',
      reasonCode: 'sdkMissing',
      message: 'Everything SDK unavailable; search unavailable',
      canRequestSetup: true,
      checkedAtIso: '2026-04-29T20:00:00.000Z'
    }),
    true
  );
  assert.equal(
    isProviderHealthContract({
      providerId: 'everything',
      state: 'degraded',
      reasonCode: 'apiToken',
      message: 'bad',
      canRequestSetup: true,
      checkedAtIso: 'not-a-date'
    }),
    false
  );
});

test('install consent and result contracts enforce no silent launch or unsafe artifact path', () => {
  assert.equal(
    isEverythingSetupConsentAllowed({
      action: 'runBundledInstaller',
      consent: true,
      officialUrl: 'https://www.voidtools.com/downloads/',
      artifactName: 'Everything-1.4.1.1032.x64-Setup.exe',
      version: '1.4.1.1032',
      sha256: 'b'.repeat(64),
      licenseApproved: true,
      provenanceApproved: true,
      requiresAdmin: true,
      explainsFilenameExposure: true
    }),
    true
  );
  assert.equal(
    isEverythingSetupConsentAllowed({
      action: 'runBundledInstaller',
      consent: true,
      officialUrl: 'https://www.voidtools.com/downloads/',
      artifactName: 'Everything-1.4.1.1032.x64-Setup.exe',
      version: '1.4.1.1032',
      sha256: 'b'.repeat(64),
      licenseApproved: false,
      provenanceApproved: true,
      requiresAdmin: true,
      explainsFilenameExposure: true
    }),
    false
  );
  assert.equal(
    isEverythingSetupConsentAllowed({
      action: 'openOfficialDownload',
      consent: true,
      officialUrl: 'https://www.voidtools.com/downloads/',
      licenseApproved: false,
      provenanceApproved: false,
      requiresAdmin: false,
      explainsFilenameExposure: true
    }),
    true
  );
});

test('activation contract covers Flow-like result actions', () => {
  assert.deepEqual(SEARCH_ACTIVATION_KINDS, [
    'openApp',
    'focusWindow',
    'openFile',
    'openFolder',
    'runCommand',
    'openSetting',
    'copyCalculatorResult',
    'openWebUrl',
    'openBookmark'
  ]);
  assert.equal(
    isSearchActivationRequest({
      resultId: 'calculator:2+2',
      providerId: 'calculator',
      actionId: 'copyCalculatorResult',
      kind: 'copyCalculatorResult',
      recordKey: 'calculator:2+2',
      payload: { value: 4 },
      requiresConfirmation: false
    }),
    true
  );
  assert.equal(
    isSearchActivationRequest({
      resultId: 'web:g',
      providerId: 'web',
      actionId: 'openWebUrl',
      kind: 'openWebUrl',
      recordKey: 'web:g',
      payload: { url: ['bad'] },
      requiresConfirmation: false
    }),
    false
  );
});

test('centered surface contract exists without changing current search-panel label', () => {
  assert.equal(SEARCH_PANEL_LABEL, 'search-panel');
  assert.deepEqual(CENTERED_SEARCH_CLOSE_REASONS, [
    'escape',
    'outsideClick',
    'focusLoss',
    'activation',
    'settingsChanged'
  ]);
  assert.equal(
    isCenteredSearchSurfaceContract({
      label: 'search-panel',
      mode: 'centeredHotkey',
      requestId: 'search-1',
      query: 'terminal',
      sequence: 42,
      anchor: 'screenCenter',
      closeReasons: CENTERED_SEARCH_CLOSE_REASONS,
      accessibility: {
        role: 'combobox',
        listboxId: 'search-results',
        activeOptionId: 'search-option-0'
      }
    }),
    true
  );
});

test('IPC command constants include registered provider health and setup names', () => {
  assert.equal(IPC_COMMANDS.getSearchProviderHealth, 'get_search_provider_health');
  assert.equal(IPC_COMMANDS.requestEverythingSetup, 'request_everything_setup');
  assert.equal(IPC_COMMANDS.showCenteredSearchPanel, 'show_centered_search_panel');
  assert.equal('activateSearchResult' in IPC_COMMANDS, false);
  assert.equal('showCenteredSearch' in IPC_COMMANDS, false);
  assert.equal('hideCenteredSearch' in IPC_COMMANDS, false);
});
