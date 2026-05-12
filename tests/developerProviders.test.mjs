import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import {
  SAVED_SEARCH_PERSISTENCE_CONTRACT,
  buildDeveloperSearchProviders,
  filterSavedSearchesForScope
} from '../dist-tests/features/search/developerProviders.js';
import { buildSearchCatalog } from '../dist-tests/lib/searchCatalog.js';

const activeContext = {
  activeWorkspaceId: 'workspace-a',
  activeWorkspaceRoot: 'C:\\dev\\jasonshell'
};

test('bounds developer providers per source and across the merged result set', () => {
  const workspaceFiles = Array.from({ length: 20 }, (_, index) => ({
    path: `C:\\dev\\jasonshell\\src\\file-${index}.ts`,
    workspaceId: 'workspace-a',
    workspaceRoot: 'C:\\dev\\jasonshell',
    title: `file-${index}.ts`,
    terms: 'provider bounded'
  }));

  const response = buildDeveloperSearchProviders(
    {
      workspaceFiles,
      commands: Array.from({ length: 20 }, (_, index) => ({
        id: `command-${index}`,
        title: `Provider command ${index}`,
        terms: 'provider bounded'
      }))
    },
    activeContext,
    'provider',
    { perProviderLimit: 3, totalLimit: 5 }
  );

  assert.equal(response.results.length, 5);
  assert.ok(response.groups.every((group) => group.results.length <= 3));
});

test('ranks active workspace matches above external files while preserving provider groups', () => {
  const response = buildDeveloperSearchProviders(
    {
      workspaceFiles: [
        {
          path: 'D:\\other\\src\\settings.ts',
          workspaceId: 'workspace-b',
          workspaceRoot: 'D:\\other',
          title: 'settings.ts'
        },
        {
          path: 'C:\\dev\\jasonshell\\src\\settings.ts',
          workspaceId: 'workspace-a',
          workspaceRoot: 'C:\\dev\\jasonshell',
          title: 'settings.ts'
        }
      ],
      gitChanges: [
        {
          path: 'C:\\dev\\jasonshell\\src\\settings.ts',
          status: 'modified',
          workspaceId: 'workspace-a',
          workspaceRoot: 'C:\\dev\\jasonshell'
        }
      ]
    },
    activeContext,
    'settings'
  );

  assert.equal(response.groups.map((group) => group.providerId).includes('workspace-files'), true);
  assert.equal(response.groups.map((group) => group.providerId).includes('git-changes'), true);
  assert.equal(response.results[0].workspaceMatch, true);
  assert.equal(response.results.every((result) => result.terms.toLowerCase().includes('settings')), true);
});

test('filters scoped providers to the active workspace when requested', () => {
  const response = buildDeveloperSearchProviders(
    {
      recentFiles: [
        {
          path: 'C:\\dev\\jasonshell\\README.md',
          workspaceId: 'workspace-a',
          workspaceRoot: 'C:\\dev\\jasonshell'
        },
        {
          path: 'D:\\external\\README.md',
          workspaceId: 'workspace-b',
          workspaceRoot: 'D:\\external'
        }
      ],
      processes: [
        {
          pid: 40,
          name: 'node.exe',
          status: 'Running',
          isKillable: true,
          workspaceId: 'workspace-a',
          cwd: 'C:\\dev\\jasonshell'
        },
        {
          pid: 41,
          name: 'node.exe',
          status: 'Running',
          isKillable: true,
          workspaceId: 'workspace-b',
          cwd: 'D:\\external'
        }
      ]
    },
    activeContext,
    '',
    { workspacePolicy: 'active-only' }
  );

  assert.deepEqual(
    response.results.map((result) => result.workspaceId),
    ['workspace-a', 'workspace-a']
  );
});

test('exposes saved-search persistence and scope contracts', () => {
  assert.equal(SAVED_SEARCH_PERSISTENCE_CONTRACT.settingsSchema, 'jasonshell.settings');
  assert.equal(SAVED_SEARCH_PERSISTENCE_CONTRACT.settingsVersion, 1);
  assert.equal(SAVED_SEARCH_PERSISTENCE_CONTRACT.secretStorageAllowed, false);

  const savedSearches = [
    { id: 'global-errors', name: 'Errors', query: 'error', scope: 'global' },
    {
      id: 'active-tests',
      name: 'Tests',
      query: 'test',
      scope: 'workspace',
      workspaceId: 'workspace-a'
    },
    {
      id: 'other-builds',
      name: 'Builds',
      query: 'build',
      scope: 'workspace',
      workspaceId: 'workspace-b'
    }
  ];

  assert.deepEqual(
    filterSavedSearchesForScope(savedSearches, activeContext, 'active-only').map((search) => search.id),
    ['global-errors', 'active-tests']
  );

  const response = buildDeveloperSearchProviders(
    { savedSearches },
    activeContext,
    'test',
    { workspacePolicy: 'active-only' }
  );

  assert.deepEqual(response.results.map((result) => result.id), ['developer:saved-searches:active-tests']);
  assert.equal(response.results[0].persistedScope, 'workspace');
});

test('static developer dashboard command is active through top-bar activation', () => {
  const catalog = buildSearchCatalog([], [], []);
  const command = catalog.find((result) => result.id === 'command:open-control-plane');
  const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');

  assert.equal(command?.kind, 'command');
  assert.match(command?.terms ?? '', /developer dashboard settings control plane git changes task history/);
  assert.match(topBarSource, /showControlPlane/);
  assert.match(topBarSource, /result\.id === 'command:open-control-plane'/);
});
