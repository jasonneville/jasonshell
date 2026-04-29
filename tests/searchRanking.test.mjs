import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  collapseDuplicateResults,
  rankSearchResultsWithUsage,
  scoreSearchResult,
  searchResultRecordKey
} from '../dist-tests/lib/searchRanking.js';

test('selected-count boost moves frequent result up without hiding exact match', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'app:exact',
      providerId: 'apps',
      kind: 'app',
      title: 'Code',
      subtitle: 'Pinned app',
      terms: 'code editor',
      priority: 100
    },
    {
      id: 'app:frequent',
      providerId: 'apps',
      kind: 'app',
      title: 'Code Helper',
      subtitle: 'Pinned app',
      terms: 'code helper',
      priority: 100
    }
  ], 'code', { 'app:frequent': 500 });

  assert.deepEqual(ranked.map((result) => result.id), ['app:exact', 'app:frequent']);
});

test('Everything-only result sets are ranked instead of left in provider order', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'system:file:C:\\Docs\\zeta.txt',
      providerId: 'everything',
      kind: 'file',
      title: 'zeta',
      subtitle: 'Everything',
      terms: 'zeta',
      priority: 10,
      path: 'C:\\Docs\\zeta.txt'
    },
    {
      id: 'system:folder:C:\\Docs\\Alpha',
      providerId: 'everything',
      kind: 'folder',
      title: 'Alpha',
      subtitle: 'Everything',
      terms: 'alpha',
      priority: 999,
      path: 'C:\\Docs\\Alpha',
      topMost: true
    }
  ], 'a', { 'folder:c:\\docs\\alpha': 80 });

  assert.deepEqual(ranked.map((result) => result.title), ['Alpha', 'zeta']);
});

test('top-most override wins deterministically for equal query matches', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'app:alpha',
      providerId: 'apps',
      kind: 'app',
      title: 'Alpha Tool',
      subtitle: 'Pinned app',
      terms: 'tool',
      priority: 100
    },
    {
      id: 'app:beta',
      providerId: 'apps',
      kind: 'app',
      title: 'Beta Tool',
      subtitle: 'Pinned app',
      terms: 'tool',
      priority: 100,
      topMost: true
    }
  ], 'tool', {});

  assert.equal(ranked[0].id, 'app:beta');
});

test('provider and result type priority prefer Everything file results over Windows fallback duplicates', () => {
  const duplicatePath = 'C:\\Docs\\Plan.txt';
  const collapsed = collapseDuplicateResults([
    {
      id: `system:file:${duplicatePath}`,
      providerId: 'windowsSearch',
      kind: 'file',
      title: 'Plan',
      subtitle: 'Windows Search',
      terms: 'plan windows search',
      priority: 140,
      path: duplicatePath
    },
    {
      id: `system:file:${duplicatePath}`,
      providerId: 'everything',
      kind: 'file',
      title: 'Plan',
      subtitle: 'Everything',
      terms: 'plan everything voidtools',
      priority: 90,
      path: duplicatePath,
      runCount: 12
    }
  ]);

  assert.equal(collapsed.length, 1);
  assert.equal(collapsed[0].providerId, 'everything');
  assert.equal(searchResultRecordKey(collapsed[0]), 'file:c:\\docs\\plan.txt');
});

test('score math is capped and deterministic', () => {
  const score = scoreSearchResult({
    id: 'file:huge',
    providerId: 'everything',
    kind: 'file',
    title: 'Huge',
    subtitle: 'Everything',
    terms: 'huge everything',
    priority: Number.MAX_SAFE_INTEGER,
    runCount: Number.MAX_SAFE_INTEGER
  }, ['huge'], { 'file:huge': Number.MAX_SAFE_INTEGER });

  assert.equal(score, 1_000_000);
});

test('exact app matches outrank Everything folders for launcher-style queries', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'system:folder:C:\\Users\\me\\AppData\\Roaming\\Spotify',
      providerId: 'everything',
      kind: 'folder',
      title: 'Spotify',
      subtitle: 'Folder - AppData',
      terms: 'spotify folder everything',
      priority: 96,
      path: 'C:\\Users\\me\\AppData\\Roaming\\Spotify'
    },
    {
      id: 'system:app:C:\\Users\\me\\AppData\\Roaming\\Spotify\\Spotify.exe',
      providerId: 'everything',
      kind: 'app',
      title: 'Spotify',
      subtitle: 'Application - AppData',
      terms: 'spotify app everything',
      priority: 170,
      path: 'C:\\Users\\me\\AppData\\Roaming\\Spotify\\Spotify.exe'
    }
  ], 'spotify', {});

  assert.equal(ranked[0].kind, 'app');
});

test('exact app intents outrank high priority Everything folders', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'system:folder:C:\\Users\\me\\Music\\Spotify',
      providerId: 'everything',
      kind: 'folder',
      title: 'Spotify',
      subtitle: 'Folder - Music',
      terms: 'spotify folder everything',
      priority: 999,
      path: 'C:\\Users\\me\\Music\\Spotify'
    },
    {
      id: 'app:spotify',
      providerId: 'apps',
      kind: 'app',
      title: 'Spotify',
      subtitle: 'Installed app',
      terms: 'spotify music application launch',
      priority: 100,
      path: 'C:\\Users\\me\\AppData\\Roaming\\Spotify\\Spotify.exe'
    }
  ], 'spotify', {});

  assert.equal(ranked[0].id, 'app:spotify');
});

test('fuzzy app token matches Spotify-style launcher intent', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'system:folder:C:\\Users\\me\\Music\\Spotify',
      providerId: 'everything',
      kind: 'folder',
      title: 'Spotify',
      subtitle: 'Folder - Music',
      terms: 'spotify folder everything',
      priority: 999,
      path: 'C:\\Users\\me\\Music\\Spotify'
    },
    {
      id: 'app:spotify',
      providerId: 'apps',
      kind: 'app',
      title: 'Spotify',
      subtitle: 'Installed app',
      terms: 'spotify music application launch',
      priority: 100,
      path: 'C:\\Users\\me\\AppData\\Roaming\\Spotify\\Spotify.exe'
    }
  ], 'sptfy', {});

  assert.equal(ranked[0].id, 'app:spotify');
});

test('Windows Settings and Control Panel intents outrank incidental folders', () => {
  const settingsRanked = rankSearchResultsWithUsage([
    {
      id: 'system:folder:C:\\Docs\\Windows Settings',
      providerId: 'everything',
      kind: 'folder',
      title: 'Windows Settings',
      subtitle: 'Folder',
      terms: 'windows settings folder everything',
      priority: 999,
      path: 'C:\\Docs\\Windows Settings'
    },
    {
      id: 'setting:windows-settings',
      providerId: 'commands',
      kind: 'setting',
      title: 'Windows Settings',
      subtitle: 'Open Windows Settings',
      terms: 'windows settings control panel system',
      priority: 118,
      path: 'ms-settings:'
    }
  ], 'windows settings', {});
  const controlPanelRanked = rankSearchResultsWithUsage([
    {
      id: 'system:folder:C:\\Docs\\Control Panel',
      providerId: 'everything',
      kind: 'folder',
      title: 'Control Panel',
      subtitle: 'Folder',
      terms: 'control panel folder everything',
      priority: 999,
      path: 'C:\\Docs\\Control Panel'
    },
    {
      id: 'setting:control-panel',
      providerId: 'commands',
      kind: 'setting',
      title: 'Control Panel',
      subtitle: 'Open classic Control Panel',
      terms: 'control panel classic settings',
      priority: 116,
      path: 'control.exe'
    }
  ], 'control panel', {});

  assert.equal(settingsRanked[0].id, 'setting:windows-settings');
  assert.equal(controlPanelRanked[0].id, 'setting:control-panel');
});

test('bare settings query outranks exact incidental Everything folder', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'system:folder:C:\\Docs\\Settings',
      providerId: 'everything',
      kind: 'folder',
      title: 'Settings',
      subtitle: 'Folder',
      terms: 'settings folder everything',
      priority: 999,
      path: 'C:\\Docs\\Settings',
      runCount: 100
    },
    {
      id: 'setting:windows-settings',
      providerId: 'commands',
      kind: 'setting',
      title: 'Windows Settings',
      subtitle: 'Open Windows Settings',
      terms: 'windows settings system settings control panel',
      priority: 118,
      path: 'ms-settings:'
    }
  ], 'settings', {});

  assert.equal(ranked[0].id, 'setting:windows-settings');
});

test('nonmatching results do not pass through on provider or type boosts alone', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'command:open-control-plane',
      providerId: 'commands',
      kind: 'command',
      title: 'Open developer dashboard',
      subtitle: 'Open settings and developer dashboard',
      terms: 'developer dashboard settings control plane git changes task history providers diagnostics',
      priority: 92
    }
  ], 'spotify', {});

  assert.deepEqual(ranked, []);
});

test('control panel query can match control-plane command alias before files', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'system:file:C:\\Docs\\Control Panel Notes.txt',
      providerId: 'everything',
      kind: 'file',
      title: 'Control Panel Notes',
      subtitle: 'File',
      terms: 'control panel notes everything',
      priority: 999,
      path: 'C:\\Docs\\Control Panel Notes.txt',
      runCount: 100
    },
    {
      id: 'command:open-control-plane',
      providerId: 'commands',
      kind: 'command',
      title: 'Open developer dashboard',
      subtitle: 'Open settings and developer dashboard',
      terms: 'developer dashboard settings control plane git changes task history control panel providers diagnostics',
      priority: 92
    }
  ], 'control panel', {});

  assert.equal(ranked[0].id, 'command:open-control-plane');
});

test('exact filename matches outrank weak substring matches', () => {
  const ranked = rankSearchResultsWithUsage([
    {
      id: 'file:weak',
      providerId: 'windowsSearch',
      kind: 'file',
      title: 'Annual Plan Archive',
      subtitle: 'File',
      terms: 'plan archive',
      priority: 120
    },
    {
      id: 'file:exact',
      providerId: 'windowsSearch',
      kind: 'file',
      title: 'Plan.txt',
      subtitle: 'File',
      terms: 'plan',
      priority: 90
    }
  ], 'plan', {});

  assert.equal(ranked[0].id, 'file:exact');
});
