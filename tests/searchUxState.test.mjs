import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  buildVisibleSearchGroupOverflows,
  buildVisibleSearchRows,
  DEFAULT_VISIBLE_GROUP_LIMIT,
  configuredSearchOpenAction,
  createLatestSearchExecutionQueue,
  createLatestSearchQueryController,
  nextSearchPanelFallbackDelay,
  nextVisibleRowIndex,
  resolveVisibleSearchRowResultIndex,
  nextSearchResultRefreshRequest,
  searchModeFromSettings,
  searchPanelKeyboardAction,
  searchVisibleRowIdentity,
  searchResultActionHints,
  selectedVisibleRowIndex,
  nextProgressiveSearchResultSet,
  shouldRetrySearchAfterProviderCacheWarm,
  shouldApplySearchEngineResponse,
  shouldApplySearchResultRefresh,
  shouldContinueSearchPanelFallbackPolling
} from '../dist-tests/features/search/searchUxState.js';
import {
  rankSearchResults,
  rankSearchResultsWithUsage,
  recordSearchUsage,
  setSearchUsageForTest
} from '../dist-tests/lib/searchRanking.js';

const results = [
  { id: 'command:refresh-search', kind: 'command', title: 'Refresh', subtitle: 'Reload', terms: 'refresh', priority: 40 },
  { id: 'window:10', kind: 'window', title: 'Editor', subtitle: 'Code', terms: 'editor code', priority: 90 },
  { id: 'folder:shell:Downloads', kind: 'folder', title: 'Downloads', subtitle: 'Folder', terms: 'downloads folder', priority: 60, path: 'shell:Downloads' },
  { id: 'file:C:/notes.txt', kind: 'file', title: 'notes.txt', subtitle: 'File', terms: 'notes file', priority: 50 },
  { id: 'app:C:/Code.lnk', kind: 'app', title: 'Code', subtitle: 'Pinned app', terms: 'code app', priority: 100 }
];

test('builds flat visible rows with Best match first and grouped remaining rows after it', () => {
  const visibleRows = buildVisibleSearchRows(results);

  assert.deepEqual(
    visibleRows.map((row) => [row.groupLabel, row.result.title, row.showGroupLabel]),
    [
      ['Best match', 'Refresh', false],
      ['Best match', 'Editor', false],
      ['Best match', 'Downloads', false],
      ['Best match', 'notes.txt', false],
      ['Best match', 'Code', false]
    ]
  );
  assert.deepEqual(visibleRows.map((row) => row.resultIndex), [0, 1, 2, 3, 4]);
  assert.deepEqual(visibleRows.map((row) => row.domId), [
    'search-result-0',
    'search-result-1',
    'search-result-2',
    'search-result-3',
    'search-result-4'
  ]);
});

test('keeps backend canonical order across every visible row without grouped tail', () => {
  const visibleRows = buildVisibleSearchRows(results);

  assert.deepEqual(
    visibleRows.map((row) => row.result.title),
    ['Refresh', 'Editor', 'Downloads', 'notes.txt', 'Code']
  );
  assert.deepEqual(visibleRows.map((row) => row.resultIndex), [0, 1, 2, 3, 4]);
  assert.deepEqual(visibleRows.map((row) => row.showGroupLabel), [false, false, false, false, false]);
  assert.deepEqual(buildVisibleSearchGroupOverflows(results), []);
});

test('best match removes top backend rows from later groups while preserving group order inside remainder', () => {
  const visibleRows = buildVisibleSearchRows([
    { id: 'setting:display', kind: 'setting', title: 'Display Settings', subtitle: 'System display preferences', terms: 'display settings', priority: 980 },
    { id: 'folder:c-dev', kind: 'folder', title: 'C:\\dev', subtitle: 'Folder', terms: 'dev folder', priority: 920, path: 'C:\\dev' },
    { id: 'folder:downloads', kind: 'folder', title: 'Downloads', subtitle: 'Folder', terms: 'downloads folder', priority: 780, path: 'shell:Downloads' },
    { id: 'file:notes', kind: 'file', title: 'notes.txt', subtitle: 'File', terms: 'notes file', priority: 760 },
    { id: 'window:editor', kind: 'window', title: 'Editor', subtitle: 'Code', terms: 'editor code', priority: 740 },
    { id: 'command:refresh-search', kind: 'command', title: 'Refresh', subtitle: 'Reload', terms: 'refresh', priority: 720 },
    { id: 'app:spotify', kind: 'app', title: 'Spotify', subtitle: 'Application', terms: 'spotify app', priority: 700 },
    { id: 'app:code', kind: 'app', title: 'Code', subtitle: 'Application', terms: 'code app', priority: 690 }
  ]);

  assert.deepEqual(
    visibleRows.map((row) => row.result.title),
    ['Display Settings', 'C:\\dev', 'Downloads', 'notes.txt', 'Editor', 'Refresh', 'Spotify', 'Code']
  );
  assert.deepEqual(
    visibleRows.map((row) => row.groupLabel),
    ['Best match', 'Best match', 'Best match', 'Best match', 'Best match', 'Best match', 'Best match', 'Best match']
  );
  assert.equal(new Set(visibleRows.map((row) => row.rowKey)).size, visibleRows.length);
});

test('caps only leading consecutive apps at four and preserves tail order with interleaved apps', () => {
  const visibleRows = buildVisibleSearchRows([
    { id: 'app:1', kind: 'app', title: 'App 1', subtitle: 'App', terms: 'app 1', priority: 1000 },
    { id: 'app:2', kind: 'app', title: 'App 2', subtitle: 'App', terms: 'app 2', priority: 999 },
    { id: 'app:3', kind: 'app', title: 'App 3', subtitle: 'App', terms: 'app 3', priority: 998 },
    { id: 'app:4', kind: 'app', title: 'App 4', subtitle: 'App', terms: 'app 4', priority: 997 },
    { id: 'app:5', kind: 'app', title: 'App 5', subtitle: 'App', terms: 'app 5', priority: 996 },
    { id: 'folder:docs', kind: 'folder', title: 'Docs', subtitle: 'Folder', terms: 'docs', priority: 995, path: 'C:\\Docs' },
    { id: 'app:tail', kind: 'app', title: 'Tail App', subtitle: 'App', terms: 'tail app', priority: 994 },
    { id: 'file:notes', kind: 'file', title: 'notes.txt', subtitle: 'File', terms: 'notes', priority: 993 },
    { id: 'window:editor', kind: 'window', title: 'Editor', subtitle: 'Window', terms: 'editor', priority: 992 }
  ]);

  assert.deepEqual(
    visibleRows.map((row) => row.result.title),
    ['App 1', 'App 2', 'App 3', 'App 4', 'Docs', 'Tail App', 'notes.txt', 'Editor', 'App 5']
  );
  assert.deepEqual(visibleRows.map((row) => row.resultIndex), [0, 1, 2, 3, 5, 6, 7, 8, 4]);
  assert.deepEqual(visibleRows.map((row) => row.rowKey), [
    '0:app:1',
    '1:app:2',
    '2:app:3',
    '3:app:4',
    '5:folder:docs',
    '6:app:tail',
    '7:file:notes',
    '8:window:editor',
    '4:app:5'
  ]);
  assert.deepEqual(visibleRows.map((row) => row.domId), [
    'search-result-0',
    'search-result-1',
    'search-result-2',
    'search-result-3',
    'search-result-5',
    'search-result-6',
    'search-result-7',
    'search-result-8',
    'search-result-4'
  ]);
});

test('keeps all app rows visible when results are app-only', () => {
  const visibleRows = buildVisibleSearchRows([
    { id: 'app:1', kind: 'app', title: 'App 1', subtitle: 'App', terms: 'app 1', priority: 1000 },
    { id: 'app:2', kind: 'app', title: 'App 2', subtitle: 'App', terms: 'app 2', priority: 999 },
    { id: 'app:3', kind: 'app', title: 'App 3', subtitle: 'App', terms: 'app 3', priority: 998 },
    { id: 'app:4', kind: 'app', title: 'App 4', subtitle: 'App', terms: 'app 4', priority: 997 },
    { id: 'app:5', kind: 'app', title: 'App 5', subtitle: 'App', terms: 'app 5', priority: 996 }
  ]);

  assert.deepEqual(visibleRows.map((row) => row.result.title), ['App 1', 'App 2', 'App 3', 'App 4', 'App 5']);
  assert.deepEqual(visibleRows.map((row) => row.resultIndex), [0, 1, 2, 3, 4]);
});

test('labels primary and secondary actions by result kind', () => {
  assert.deepEqual(searchResultActionHints(results[4]), { primary: 'Launch', secondary: null });
  assert.deepEqual(searchResultActionHints(results[1]), { primary: 'Focus', secondary: null });
  assert.deepEqual(searchResultActionHints(results[2]), { primary: 'Open', secondary: 'Pin' });
  assert.deepEqual(searchResultActionHints(results[0]), { primary: 'Run', secondary: null });
  assert.deepEqual(
    searchResultActionHints({
      id: 'calculator:2+2',
      kind: 'calculator',
      title: '4',
      subtitle: 'Calculator',
      terms: '2+2',
      priority: 80
    }),
    { primary: 'Copy', secondary: null }
  );
});

test('category rows default to seven visible items and advertise remaining rows', () => {
  const appResults = Array.from({ length: 12 }, (_, index) => ({
    id: `app:${index}`,
    kind: 'app',
    title: `App ${index + 1}`,
    subtitle: 'Application',
    terms: `app ${index + 1}`,
    priority: 900 - index
  }));
  const rows = buildVisibleSearchRows(appResults);
  const overflows = buildVisibleSearchGroupOverflows(appResults);

  assert.equal(DEFAULT_VISIBLE_GROUP_LIMIT, 7);
  assert.equal(rows.length, 12);
  assert.equal(rows.filter((row) => row.groupId === 'bestMatch').length, 12);
  assert.deepEqual(overflows, []);
});

test('expanded groups reveal hidden rows and clear overflow state for that category', () => {
  const appResults = Array.from({ length: 12 }, (_, index) => ({
    id: `app:${index}`,
    kind: 'app',
    title: `App ${index + 1}`,
    subtitle: 'Application',
    terms: `app ${index + 1}`,
    priority: 900 - index
  }));
  const expanded = new Set(['apps']);
  const rows = buildVisibleSearchRows(appResults, { expandedGroups: expanded });
  const overflows = buildVisibleSearchGroupOverflows(appResults, { expandedGroups: expanded });

  assert.equal(rows.length, 12);
  assert.equal(rows.filter((row) => row.groupId === 'bestMatch').length, 12);
  assert.deepEqual(overflows, []);
});

test('groups new Flow-like result kinds into settings and commands visible sections', () => {
  const visibleRows = buildVisibleSearchRows([
    { id: 'setting:display', kind: 'setting', title: 'Display', subtitle: 'Setting', terms: 'display', priority: 70 },
    { id: 'calculator:2+2', kind: 'calculator', title: '4', subtitle: 'Calculator', terms: '2+2', priority: 70 },
    { id: 'web:g', kind: 'web', title: 'Search web', subtitle: 'Google', terms: 'g cats', priority: 70 },
    { id: 'bookmark:docs', kind: 'bookmark', title: 'Docs', subtitle: 'Bookmark', terms: 'docs', priority: 70 }
  ]);

  assert.deepEqual(
    visibleRows.map((row) => row.groupLabel),
    ['Best match', 'Best match', 'Best match', 'Best match']
  );
  assert.deepEqual(visibleRows.map((row) => row.result.kind), ['setting', 'calculator', 'web', 'bookmark']);
});

test('visible row helpers map backend selection into visual order', () => {
  const visibleRows = buildVisibleSearchRows([
    { id: 'app:alpha', kind: 'app', title: 'Alpha', subtitle: 'App', terms: 'alpha', priority: 100 },
    { id: 'file:beta', kind: 'file', title: 'beta.txt', subtitle: 'File', terms: 'beta', priority: 90 },
    { id: 'folder:gamma', kind: 'folder', title: 'Gamma', subtitle: 'Folder', terms: 'gamma', priority: 80, path: 'C:\\Gamma' },
    { id: 'app:delta', kind: 'app', title: 'Delta', subtitle: 'App', terms: 'delta', priority: 70 },
    { id: 'command:echo', kind: 'command', title: 'Echo', subtitle: 'Command', terms: 'echo', priority: 60 }
  ]);

  assert.equal(selectedVisibleRowIndex(visibleRows, 3), 3);
  assert.equal(selectedVisibleRowIndex(visibleRows, 4), 4);
  assert.equal(selectedVisibleRowIndex(visibleRows, 99), -1);
  assert.equal(nextVisibleRowIndex(visibleRows, -1, 1), 0);
  assert.equal(nextVisibleRowIndex(visibleRows, -1, -1), visibleRows.length - 1);
  assert.equal(nextVisibleRowIndex(visibleRows, 3, -1), 2);
  assert.equal(nextVisibleRowIndex(visibleRows, 3, 1), 4);
});

test('phase 1 query-gate ordering keeps first visible row equal to backend rank 1 across representative intents', () => {
  const queryCases = [
    {
      query: 'display settings',
      results: [
        { id: 'setting:display', kind: 'setting', title: 'Display Settings', subtitle: 'Setting', terms: 'display settings', priority: 1000 },
        { id: 'folder:display-docs', kind: 'folder', title: 'Display', subtitle: 'Folder', terms: 'display folder', priority: 999, path: 'C:\\Docs\\Display' },
        { id: 'app:displayfusion', kind: 'app', title: 'DisplayFusion', subtitle: 'App', terms: 'displayfusion', priority: 998 },
        { id: 'setting:sound', kind: 'setting', title: 'Sound Settings', subtitle: 'Setting', terms: 'sound settings', priority: 997 }
      ],
      expectedTop: 'Display Settings'
    },
    {
      query: 'sound settings',
      results: [
        { id: 'setting:sound', kind: 'setting', title: 'Sound Settings', subtitle: 'Setting', terms: 'sound settings', priority: 1000 },
        { id: 'file:sound', kind: 'file', title: 'sound.txt', subtitle: 'File', terms: 'sound file', priority: 999 },
        { id: 'app:sound-recorder', kind: 'app', title: 'Sound Recorder', subtitle: 'App', terms: 'sound recorder', priority: 998 },
        { id: 'setting:display', kind: 'setting', title: 'Display Settings', subtitle: 'Setting', terms: 'display settings', priority: 997 }
      ],
      expectedTop: 'Sound Settings'
    },
    {
      query: 'control panel',
      results: [
        { id: 'setting:control-panel', kind: 'setting', title: 'Control Panel', subtitle: 'Windows setting', terms: 'control panel settings', priority: 1000 },
        { id: 'command:open-control-plane', kind: 'command', title: 'Control Plane', subtitle: 'JasonShell command', terms: 'control plane', priority: 999 },
        { id: 'folder:control-panel', kind: 'folder', title: 'Control Panel', subtitle: 'Folder', terms: 'control panel folder', priority: 998, path: 'C:\\Docs\\Control Panel' }
      ],
      expectedTop: 'Control Panel'
    },
    {
      query: 'spotify',
      results: [
        { id: 'app:spotify', kind: 'app', title: 'Spotify', subtitle: 'Installed app', terms: 'spotify app', priority: 1000 },
        { id: 'file:spotify', kind: 'file', title: 'spotify notes.txt', subtitle: 'File', terms: 'spotify notes', priority: 999 },
        { id: 'folder:spotify', kind: 'folder', title: 'Spotify', subtitle: 'Folder', terms: 'spotify folder', priority: 998, path: 'C:\\Spotify' }
      ],
      expectedTop: 'Spotify'
    },
    {
      query: 'C:\\dev',
      results: [
        { id: 'folder:c-dev', kind: 'folder', title: 'C:\\dev', subtitle: 'Folder', terms: 'c dev folder', priority: 1000, path: 'C:\\dev' },
        { id: 'app:devhome', kind: 'app', title: 'Dev Home', subtitle: 'App', terms: 'dev home', priority: 999 },
        { id: 'file:dev-notes', kind: 'file', title: 'dev-notes.txt', subtitle: 'File', terms: 'dev notes', priority: 998 }
      ],
      expectedTop: 'C:\\dev'
    },
    {
      query: 'dev',
      results: [
        { id: 'folder:c-dev', kind: 'folder', title: 'C:\\dev', subtitle: 'Folder', terms: 'c dev folder', priority: 1000, path: 'C:\\dev' },
        { id: 'app:devhome', kind: 'app', title: 'Dev Home', subtitle: 'App', terms: 'dev home', priority: 999 },
        { id: 'command:open-control-plane', kind: 'command', title: 'Open developer dashboard', subtitle: 'Command', terms: 'developer dashboard', priority: 998 },
        { id: 'file:devlog', kind: 'file', title: 'dev.log', subtitle: 'File', terms: 'dev log', priority: 997 }
      ],
      expectedTop: 'C:\\dev'
    }
  ];

  for (const queryCase of queryCases) {
    const visibleRows = buildVisibleSearchRows(queryCase.results);
    assert.equal(visibleRows[0]?.result.title, queryCase.expectedTop, queryCase.query);
    assert.equal(visibleRows[0]?.result, queryCase.results[0], queryCase.query);
  }
});

test('visible rows keep duplicate backend ids uniquely keyable while preserving raw activation ids', () => {
  const visibleRows = buildVisibleSearchRows([
    { id: 'duplicate:id', kind: 'app', title: 'First Duplicate', subtitle: 'App', terms: 'dup first', priority: 100 },
    { id: 'duplicate:id', kind: 'folder', title: 'Second Duplicate', subtitle: 'Folder', terms: 'dup second', priority: 99, path: 'C:\\dup' },
    { id: 'duplicate:id', kind: 'file', title: 'Third Duplicate', subtitle: 'File', terms: 'dup third', priority: 98 }
  ]);

  assert.deepEqual(visibleRows.map((row) => row.id), ['duplicate:id', 'duplicate:id', 'duplicate:id']);
  assert.deepEqual(visibleRows.map((row) => row.rowKey), [
    '0:duplicate:id',
    '1:duplicate:id',
    '2:duplicate:id'
  ]);
  assert.deepEqual(visibleRows.map((row) => row.domId), [
    'search-result-0',
    'search-result-1',
    'search-result-2'
  ]);
  assert.equal(new Set(visibleRows.map((row) => row.rowKey)).size, 3);
});

test('visible row identity resolves second duplicate backend id to the clicked visible row', () => {
  const duplicateResults = [
    { id: 'duplicate:id', recordKey: 'app:first', kind: 'app', title: 'First Duplicate', subtitle: 'App', terms: 'dup first', priority: 100 },
    { id: 'duplicate:id', recordKey: 'folder:second', kind: 'folder', title: 'Second Duplicate', subtitle: 'Folder', terms: 'dup second', priority: 99, path: 'C:\\dup' },
    { id: 'duplicate:id', recordKey: 'file:third', kind: 'file', title: 'Third Duplicate', subtitle: 'File', terms: 'dup third', priority: 98 }
  ];
  const visibleRows = buildVisibleSearchRows(duplicateResults);
  const secondIdentity = searchVisibleRowIdentity(visibleRows[1]);

  assert.deepEqual(secondIdentity, {
    id: 'duplicate:id',
    rowKey: '1:duplicate:id',
    recordKey: 'folder:second',
    resultIndex: 1
  });
  assert.equal(resolveVisibleSearchRowResultIndex(duplicateResults, secondIdentity), 1);
  assert.equal(resolveVisibleSearchRowResultIndex(duplicateResults, { id: 'duplicate:id' }), 0);
});

test('progressive new-query local rows replace stale prior-query working set before complete', () => {
  const spotifyRows = [
    { id: 'app:spotify', recordKey: 'app:spotify', kind: 'app', title: 'Spotify', subtitle: 'App', terms: 'spotify', priority: 100 },
    { id: 'file:spotify', recordKey: 'file:spotify', kind: 'file', title: 'spotify notes.txt', subtitle: 'File', terms: 'spotify file', priority: 90 }
  ];
  const displayRows = [
    { id: 'setting:display', recordKey: 'setting:display', kind: 'setting', title: 'Display Settings', subtitle: 'Setting', terms: 'display settings', priority: 1000 }
  ];
  const typing = nextProgressiveSearchResultSet(
    { query: 'spotify', results: spotifyRows },
    { query: 'display settings', phase: 'typing', results: [] }
  );
  const local = nextProgressiveSearchResultSet(
    typing,
    { query: 'display settings', phase: 'local', results: displayRows }
  );

  assert.deepEqual(typing.results.map((result) => result.title), ['Spotify', 'spotify notes.txt']);
  assert.equal(typing.query, 'spotify');
  assert.deepEqual(local.results.map((result) => result.title), ['Display Settings']);
  assert.equal(local.query, 'display settings');
  assert.equal(buildVisibleSearchRows(local.results)[0]?.result.title, 'Display Settings');
});

test('progressive same-normalized-query snapshots replace stale best match without needing trailing-space rerun', () => {
  const staleRows = [
    {
      id: 'file:spotify-readme',
      recordKey: 'file:c:\\notes\\spotify-readme.txt',
      kind: 'file',
      title: 'spotify-readme.txt',
      subtitle: 'Old file hit',
      terms: 'spotify readme',
      priority: 700
    },
    {
      id: 'app:spotify-windowsapps',
      recordKey: 'app:c:\\users\\jnev1\\appdata\\local\\microsoft\\windowsapps\\spotify.exe',
      kind: 'app',
      title: 'Spotify',
      subtitle: 'Stale WindowsApps launcher',
      terms: 'spotify',
      priority: 650
    }
  ];
  const freshRankedRows = [
    {
      id: 'app:spotify',
      recordKey: 'app:c:\\program files\\spotify\\spotify.exe',
      kind: 'app',
      title: 'Spotify',
      subtitle: 'Installed app',
      terms: 'spotify',
      priority: 2400
    },
    {
      id: 'file:spotify-readme',
      recordKey: 'file:c:\\notes\\spotify-readme.txt',
      kind: 'file',
      title: 'spotify-readme.txt',
      subtitle: 'Old file hit',
      terms: 'spotify readme',
      priority: 700
    }
  ];
  const next = nextProgressiveSearchResultSet(
    { query: 'spotify', results: staleRows },
    { query: 'spotify', phase: 'provider', results: freshRankedRows }
  );

  assert.deepEqual(
    next.results.map((result) => result.recordKey),
    ['app:c:\\program files\\spotify\\spotify.exe', 'file:c:\\notes\\spotify-readme.txt']
  );
  assert.equal(buildVisibleSearchRows(next.results)[0]?.result.recordKey, 'app:c:\\program files\\spotify\\spotify.exe');
});

test('visible rows preserve fuzzy highlight span data for panel rendering', () => {
  const visibleRows = buildVisibleSearchRows([
    {
      id: 'app:spotify',
      kind: 'app',
      title: 'Spotify',
      subtitle: 'Application',
      terms: 'spotify app',
      priority: 100,
      titleHighlightData: [0, 2, 3, 3],
      subtitleHighlightData: []
    }
  ]);

  assert.deepEqual(visibleRows[0].result.titleHighlightData, [0, 2, 3, 3]);
});

test('top-bar keyboard traversal can map visible-row movement back to backend result indices', () => {
  const visibleRows = buildVisibleSearchRows([
    { id: 'app:alpha', kind: 'app', title: 'Alpha', subtitle: 'App', terms: 'alpha', priority: 100 },
    { id: 'app:beta', kind: 'app', title: 'Beta', subtitle: 'App', terms: 'beta', priority: 99 },
    { id: 'app:gamma', kind: 'app', title: 'Gamma', subtitle: 'App', terms: 'gamma', priority: 98 },
    { id: 'app:delta', kind: 'app', title: 'Delta', subtitle: 'App', terms: 'delta', priority: 97 },
    { id: 'app:overflow', kind: 'app', title: 'Overflow', subtitle: 'App', terms: 'overflow', priority: 96 },
    { id: 'setting:display', kind: 'setting', title: 'Display Settings', subtitle: 'Setting', terms: 'display settings', priority: 100 },
    { id: 'folder:dev', kind: 'folder', title: 'C:\\dev', subtitle: 'Folder', terms: 'dev folder', priority: 99, path: 'C:\\dev' },
    { id: 'file:notes', kind: 'file', title: 'notes.txt', subtitle: 'File', terms: 'notes file', priority: 95 },
    { id: 'setting:sound', kind: 'setting', title: 'Sound Settings', subtitle: 'Setting', terms: 'sound settings', priority: 94 },
    { id: 'app:spotify', kind: 'app', title: 'Spotify', subtitle: 'App', terms: 'spotify app', priority: 1 }
  ]);

  let selectedIndex = 4;
  let selectedVisibleIndex = selectedVisibleRowIndex(visibleRows, selectedIndex);
  assert.equal(selectedVisibleIndex, 9);
  assert.equal(visibleRows[selectedVisibleIndex].result.title, 'Overflow');

  selectedIndex = visibleRows[nextVisibleRowIndex(visibleRows, selectedVisibleIndex, -1)].resultIndex;
  selectedVisibleIndex = selectedVisibleRowIndex(visibleRows, selectedIndex);
  assert.equal(selectedVisibleIndex, 8);
  assert.equal(visibleRows[selectedVisibleIndex].result.title, 'Spotify');
  assert.equal(selectedIndex, 9);

  selectedIndex = visibleRows[nextVisibleRowIndex(visibleRows, selectedVisibleIndex, -1)].resultIndex;
  selectedVisibleIndex = selectedVisibleRowIndex(visibleRows, selectedIndex);
  assert.equal(selectedVisibleIndex, 7);
  assert.equal(visibleRows[selectedVisibleIndex].result.title, 'Sound Settings');
  assert.equal(selectedIndex, 8);
});

test('ranking accepts injected usage so frequent results can outrank equal matches', () => {
  const ranked = rankSearchResultsWithUsage([
    { id: 'app:alpha', kind: 'app', title: 'Alpha Tool', subtitle: 'App', terms: 'tool', priority: 100 },
    { id: 'app:beta', kind: 'app', title: 'Beta Tool', subtitle: 'App', terms: 'tool', priority: 100 }
  ], 'tool', { 'app:beta': 24 });

  assert.deepEqual(ranked.map((result) => result.id), ['app:beta', 'app:alpha']);
});

test('ranking caches usage map instead of reading storage for every rank', () => {
  let reads = 0;
  let writes = [];
  global.window = {
    localStorage: {
      getItem() {
        reads += 1;
        return JSON.stringify({ 'app:beta': 24 });
      },
      setItem(_key, value) {
        writes.push(value);
      }
    }
  };
  setSearchUsageForTest(null);

  const sample = [
    { id: 'app:alpha', kind: 'app', title: 'Alpha Tool', subtitle: 'App', terms: 'tool', priority: 100 },
    { id: 'app:beta', kind: 'app', title: 'Beta Tool', subtitle: 'App', terms: 'tool', priority: 100 }
  ];
  assert.equal(rankSearchResults(sample, 'tool')[0].id, 'app:beta');
  assert.equal(rankSearchResults(sample, 'tool')[0].id, 'app:beta');
  assert.equal(reads, 1);

  recordSearchUsage('app:alpha');
  assert.equal(writes.length, 1);
  delete global.window;
  setSearchUsageForTest(null);
});

test('search panel fallback polling is bounded with backoff', () => {
  assert.deepEqual(
    [0, 1, 2, 3, 4, 5].map(nextSearchPanelFallbackDelay),
    [120, 240, 500, 1000, 2000, 2000]
  );
  assert.equal(shouldContinueSearchPanelFallbackPolling(0, true, false), false);
  assert.equal(shouldContinueSearchPanelFallbackPolling(0, true, true), true);
  assert.equal(shouldContinueSearchPanelFallbackPolling(0, false, false), true);
});

test('search input state can advance while expensive result refresh is deferred latest-only', () => {
  let displayedQuery = '';
  let sequence = 0;
  const requests = [];

  for (const query of ['s', 'sp', 'spo', 'spot']) {
    displayedQuery = query;
    const request = nextSearchResultRefreshRequest(sequence, query);
    sequence = request.sequence;
    requests.push(request);
  }

  assert.equal(displayedQuery, 'spot');
  assert.equal(shouldApplySearchResultRefresh(requests[0], displayedQuery, sequence), false);
  assert.equal(shouldApplySearchResultRefresh(requests[2], displayedQuery, sequence), false);
  assert.equal(shouldApplySearchResultRefresh(requests[3], displayedQuery, sequence), true);
});

test('latest-only search execution queue keeps rapid Firefox draft immediate and runs final provider request only', () => {
  const executed = [];
  const queue = createLatestSearchExecutionQueue((request) => {
    executed.push(request);
  });
  let draft = '';

  for (const query of ['F', 'Fi', 'Fir', 'Fire', 'Firef', 'Firefo', 'Firefox']) {
    draft = query;
    queue.enqueue({ query, sequence: executed.length + query.length });
    assert.equal(draft, query);
    assert.deepEqual(executed, []);
  }

  queue.flush();

  assert.equal(draft, 'Firefox');
  assert.deepEqual(executed, [{ query: 'Firefox', sequence: 7 }]);
});

test('search engine controller ignores out-of-order stale provider responses', () => {
  const controller = createLatestSearchQueryController();
  const first = controller.next('d');
  const second = controller.next('di');
  const third = controller.next('display');

  assert.equal(shouldApplySearchEngineResponse(first, 'display', controller.currentSequence()), false);
  assert.equal(controller.shouldApply(second, 'display'), false);
  assert.equal(controller.shouldApply(third, 'display'), true);
});

test('provider cache warm retry allows stale refresh payloads with nonzero app rows', () => {
  const request = { query: 'spotify', sequence: 7 };

  assert.equal(
    shouldRetrySearchAfterProviderCacheWarm(request, 'spotify', 7, [
      { providerId: 'apps', cache: 'refresh', resultCount: 3 }
    ]),
    true
  );
});

test('provider cache warm retry ignores non-app cache timings and cache hits', () => {
  const request = { query: 'spotify', sequence: 8 };

  assert.equal(
    shouldRetrySearchAfterProviderCacheWarm(request, 'spotify', 8, [
      { providerId: 'everything', cache: 'refresh', resultCount: 50 }
    ]),
    false
  );
  assert.equal(
    shouldRetrySearchAfterProviderCacheWarm(request, 'spotify', 8, [
      { providerId: 'apps', cache: 'hit', resultCount: 3 }
    ]),
    false
  );
});

test('search mode defaults to centered and preserves explicit top-right routing', () => {
  assert.equal(searchModeFromSettings(undefined), 'centeredHotkey');
  assert.equal(searchModeFromSettings('topRight'), 'topRight');
  assert.equal(searchModeFromSettings('centeredHotkey'), 'centeredHotkey');
  assert.equal(configuredSearchOpenAction('topRight'), 'openTopRight');
  assert.equal(configuredSearchOpenAction('centeredHotkey'), 'openCentered');
});

test('search keyboard actions are shared by top-right and centered modes', () => {
  assert.equal(searchPanelKeyboardAction('ArrowDown'), 'selectNext');
  assert.equal(searchPanelKeyboardAction('ArrowUp'), 'selectPrevious');
  assert.equal(searchPanelKeyboardAction('Enter'), 'activate');
  assert.equal(searchPanelKeyboardAction('Escape'), 'close');
  assert.equal(searchPanelKeyboardAction('Tab'), 'none');
});
