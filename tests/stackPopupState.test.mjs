import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  applyStackEntries,
  applyStackFolderListing,
  canNavigateStackBack,
  canNavigateStackForward,
  defaultStackPopupViewState,
  findTypeToSelectPath,
  formatStackSize,
  mergeStackFolderListings,
  navigateStackHistory,
  normalizeStackDisplayPath,
  openStackFolder,
  parentStackPath,
  selectAllStackEntries,
  selectStackEntry,
  selectedStackEntry,
  selectedStackPaths,
  sortStackEntries,
  stackBreadcrumbSegments,
  stackPopupHasRetainedRows,
  stackListingStatus,
  stackPopupOpenPath,
  stackPopupRequestKey,
  stackItemNameFromPath,
  updateStackSort
} from '../dist-tests/stackPopupState.js';
import { stackFileIconForEntry } from '../dist-tests/stackFileIcons.js';

const documents = 'C:\\Users\\me\\Documents';
const downloads = 'C:\\Users\\me\\Downloads';

function stackEntry(name, entryType = 'File') {
  return {
    id: `C:\\Users\\me\\Documents\\${name}`,
    name,
    path: `C:\\Users\\me\\Documents\\${name}`,
    entryType,
    typeLabel: entryType === 'Folder' ? 'Folder' : 'TXT File',
    size: entryType === 'Folder' ? null : 1,
    modifiedMs: null,
    isHidden: false,
    isReadonly: false,
    isSystem: false,
    isSymlink: false,
    isReparsePoint: false
  };
}

test('keeps stack navigation history across folder switching', () => {
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = openStackFolder(state, downloads);

  assert.equal(state.currentPath, downloads);
  assert.deepEqual(state.history, [documents, downloads]);
  assert.equal(canNavigateStackBack(state), true);
  assert.equal(canNavigateStackForward(state), false);

  state = navigateStackHistory(state, -1);
  assert.equal(state.currentPath, documents);
  assert.equal(canNavigateStackForward(state), true);
});

test('does not duplicate current folder when reopened after hide/show', () => {
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = openStackFolder(state, documents);

  assert.deepEqual(state.history, [documents]);
  assert.equal(state.historyIndex, 0);
});

test('normalizes stack popup open request payloads', () => {
  assert.equal(stackPopupOpenPath(documents), documents);
  assert.equal(stackPopupOpenPath({ path: downloads }), downloads);
  assert.equal(stackPopupOpenPath({ folderPath: downloads }), downloads);
  assert.equal(stackPopupOpenPath({ path: '   ' }), null);
  assert.equal(stackPopupOpenPath(null), null);
});

test('keys stack popup open requests by request id when available', () => {
  assert.equal(stackPopupRequestKey({ path: documents, requestId: 'open-1' }), 'request:open-1');
  assert.equal(stackPopupRequestKey({ path: documents, requestId: 'open-2' }), 'request:open-2');
  assert.equal(stackPopupRequestKey({ path: documents }), `legacy:${documents}`);
  assert.equal(stackPopupRequestKey(documents), `legacy:${documents}`);
  assert.equal(stackPopupRequestKey({ path: '   ', requestId: 'open-3' }), 'request:open-3');
  assert.equal(stackPopupRequestKey(null), null);
});

test('normalizes extended windows paths for display and navigation', () => {
  assert.equal(normalizeStackDisplayPath('\\\\?\\C:\\Users\\me\\Documents'), documents);
  assert.equal(
    normalizeStackDisplayPath('\\\\?\\UNC\\server\\share\\Folder'),
    '\\\\server\\share\\Folder'
  );
  assert.equal(parentStackPath('\\\\?\\C:\\Users\\me\\Documents'), 'C:\\Users\\me');
});

test('builds valid breadcrumb paths for drive and unc folders', () => {
  assert.deepEqual(
    stackBreadcrumbSegments(documents).map((crumb) => crumb.path),
    ['C:\\', 'C:\\Users', 'C:\\Users\\me', documents]
  );
  assert.deepEqual(
    stackBreadcrumbSegments('\\\\?\\UNC\\server\\share\\Folder').map((crumb) => crumb.path),
    ['\\\\server\\share', '\\\\server\\share\\Folder']
  );
});

test('drops forward history after branching to a different folder', () => {
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = openStackFolder(state, downloads);
  state = navigateStackHistory(state, -1);
  state = openStackFolder(state, 'C:\\Users\\me\\Desktop');

  assert.deepEqual(state.history, [documents, 'C:\\Users\\me\\Desktop']);
  assert.equal(canNavigateStackForward(state), false);
});

test('retains previous entries while a different folder is loading', () => {
  const entries = [
    {
      id: 'old',
      name: 'old.txt',
      path: 'C:\\Users\\me\\Documents\\old.txt',
      entryType: 'File',
      size: 1,
      modifiedMs: null
    }
  ];
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackEntries(state, documents, entries);
  state = openStackFolder(state, downloads);

  assert.equal(state.currentPath, downloads);
  assert.equal(state.entriesPath, documents);
  assert.deepEqual(state.entries, entries);
  assert.equal(state.selectedPath, null);
  assert.equal(state.statusMessage, 'Loading folder...');
  assert.equal(stackPopupHasRetainedRows(state), true);
});

test('retains previous entries while navigating history loads another folder', () => {
  const entries = [
    {
      id: 'old',
      name: 'old.txt',
      path: 'C:\\Users\\me\\Downloads\\old.txt',
      entryType: 'File',
      size: 1,
      modifiedMs: null
    }
  ];
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = openStackFolder(state, downloads);
  state = applyStackEntries(state, downloads, entries);
  state = navigateStackHistory(state, -1);

  assert.equal(state.currentPath, documents);
  assert.equal(state.entriesPath, downloads);
  assert.deepEqual(state.entries, entries);
  assert.equal(stackPopupHasRetainedRows(state), true);
});

test('clears retained-row state once the requested folder page arrives', () => {
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackEntries(state, documents, [stackEntry('alpha.txt')]);
  state = openStackFolder(state, downloads);

  assert.equal(stackPopupHasRetainedRows(state), true);

  state = applyStackEntries(state, downloads, [
    {
      ...stackEntry('fresh.txt'),
      id: 'C:\\Users\\me\\Downloads\\fresh.txt',
      path: 'C:\\Users\\me\\Downloads\\fresh.txt'
    }
  ]);

  assert.equal(state.entriesPath, downloads);
  assert.equal(stackPopupHasRetainedRows(state), false);
});

test('applies entries and preserves valid selection', () => {
  const entries = [
    {
      id: 'c:\\users\\me\\documents\\notes.txt',
      name: 'notes.txt',
      path: 'C:\\Users\\me\\Documents\\notes.txt',
      entryType: 'File',
      size: 1536,
      modifiedMs: 1700000000000
    }
  ];
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackEntries(state, documents, entries);
  state = selectStackEntry(state, entries[0].path);

  assert.equal(selectedStackEntry(state)?.name, 'notes.txt');
  assert.equal(formatStackSize(selectedStackEntry(state)?.size), '1.5 KB');
});

test('supports toggle, range, and select-all stack selection', () => {
  const entries = [stackEntry('alpha.txt'), stackEntry('bravo.txt'), stackEntry('charlie.txt')];
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackEntries(state, documents, entries);

  state = selectStackEntry(state, entries[0].path);
  state = selectStackEntry(state, entries[2].path, 'toggle');
  assert.deepEqual(selectedStackPaths(state), [entries[0].path, entries[2].path]);
  assert.equal(state.selectionAnchorPath, entries[2].path);

  state = selectStackEntry(state, entries[2].path, 'toggle');
  assert.deepEqual(selectedStackPaths(state), [entries[0].path]);

  state = selectStackEntry(state, entries[0].path);
  state = selectStackEntry(state, entries[2].path, 'range');
  assert.deepEqual(selectedStackPaths(state), entries.map((entry) => entry.path));
  assert.equal(state.selectedPath, entries[2].path);

  state = selectAllStackEntries(state);
  assert.deepEqual(selectedStackPaths(state), entries.map((entry) => entry.path));
});

test('preserves visible selections and drops stale selections after refresh', () => {
  const entries = [stackEntry('alpha.txt'), stackEntry('bravo.txt'), stackEntry('charlie.txt')];
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackEntries(state, documents, entries);
  state = selectAllStackEntries(state);

  state = applyStackEntries(state, documents, [entries[0], entries[2]]);

  assert.deepEqual(selectedStackPaths(state), [entries[0].path, entries[2].path]);
  assert.equal(state.selectedPath, entries[0].path);
});

test('finds the next type-to-select match from current selection', () => {
  const entries = [stackEntry('alpha.txt'), stackEntry('bravo.txt'), stackEntry('beta.txt')];

  assert.equal(findTypeToSelectPath(entries, 'b'), entries[1].path);
  assert.equal(findTypeToSelectPath(entries, 'b', entries[1].path), entries[2].path);
  assert.equal(findTypeToSelectPath(entries, 'z', entries[1].path), null);
});

test('sorts stack entries deterministically while preserving folders first', () => {
  const entries = [
    { ...stackEntry('zeta.txt'), size: 2, modifiedMs: 20, typeLabel: 'TXT File' },
    { ...stackEntry('beta', 'Folder'), size: null, modifiedMs: 30, typeLabel: 'Folder' },
    { ...stackEntry('alpha.txt'), size: 10, modifiedMs: 10, typeLabel: 'TXT File' },
    { ...stackEntry('alpha', 'Folder'), size: null, modifiedMs: 40, typeLabel: 'Folder' }
  ];

  assert.deepEqual(
    sortStackEntries(entries, 'name', 'asc').map((entry) => entry.name),
    ['alpha', 'beta', 'alpha.txt', 'zeta.txt']
  );
  assert.deepEqual(
    sortStackEntries(entries, 'size', 'desc').map((entry) => entry.name),
    ['beta', 'alpha', 'alpha.txt', 'zeta.txt']
  );
  assert.deepEqual(
    sortStackEntries(entries, 'modified', 'asc').map((entry) => entry.name),
    ['beta', 'alpha', 'alpha.txt', 'zeta.txt']
  );
});

test('updates stack sort column and toggles direction', () => {
  const entries = [stackEntry('bravo.txt'), stackEntry('alpha.txt')];
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackEntries(state, documents, entries);

  state = updateStackSort(state, 'name');
  assert.equal(state.sortDirection, 'desc');
  assert.deepEqual(state.entries.map((entry) => entry.name), ['bravo.txt', 'alpha.txt']);

  state = updateStackSort(state, 'type');
  assert.equal(state.sortColumn, 'type');
  assert.equal(state.sortDirection, 'asc');
});

test('ignores stale folder entry payloads', () => {
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackEntries(state, downloads, [
    {
      id: 'stale',
      name: 'stale.txt',
      path: 'C:\\Users\\me\\Downloads\\stale.txt',
      entryType: 'File',
      size: 1,
      modifiedMs: null
    }
  ]);

  assert.equal(state.entries.length, 0);
  assert.equal(state.currentPath, documents);
});

test('applies complete stack folder listings with partial warning status', () => {
  const listing = {
    path: documents,
    entries: [
      {
        id: 'notes',
        name: 'notes.txt',
        path: 'C:\\Users\\me\\Documents\\notes.txt',
        entryType: 'File',
        typeLabel: 'TXT File',
        size: 1,
        modifiedMs: null,
        isSymlink: false,
        isReparsePoint: false
      }
    ],
    total: 3,
    warnings: [
      {
        path: 'C:\\Users\\me\\Documents\\secret.txt',
        message: 'Access denied'
      }
    ]
  };
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackFolderListing(state, documents, listing);

  assert.equal(state.entries.length, 1);
  assert.equal(state.statusMessage, '1 of 3 items - partial listing: 1 warning');
  assert.equal(stackListingStatus(listing), state.statusMessage);
});

test('merges incremental stack folder pages for immediate first render', () => {
  const firstPage = {
    path: documents,
    entries: [stackEntry('Alpha', 'Folder')],
    total: 3,
    warnings: [{ path: null, message: 'first warning' }],
    offset: 0,
    limit: 1,
    hasMore: true
  };
  const secondPage = {
    path: documents,
    entries: [stackEntry('bravo.txt')],
    total: 3,
    warnings: [{ path: null, message: 'second warning' }],
    offset: 1,
    limit: 1,
    hasMore: true
  };

  const merged = mergeStackFolderListings(
    mergeStackFolderListings(null, firstPage),
    secondPage
  );

  assert.deepEqual(merged.entries.map((entry) => entry.name), ['Alpha', 'bravo.txt']);
  assert.equal(merged.total, 3);
  assert.deepEqual(merged.warnings.map((warning) => warning.message), ['first warning', 'second warning']);
});

test('ignores stale folder listing payloads', () => {
  let state = openStackFolder(defaultStackPopupViewState, documents);
  state = applyStackFolderListing(state, downloads, {
    path: downloads,
    entries: [
      {
        id: 'stale',
        name: 'stale.txt',
        path: 'C:\\Users\\me\\Downloads\\stale.txt',
        entryType: 'File',
        typeLabel: 'TXT File',
        size: 1,
        modifiedMs: null,
        isSymlink: false,
        isReparsePoint: false
      }
    ],
    total: 1,
    warnings: []
  });

  assert.equal(state.entries.length, 0);
  assert.equal(state.currentPath, documents);
});

test('extracts stack item names from windows paths', () => {
  assert.equal(stackItemNameFromPath('C:\\Users\\me\\Documents'), 'Documents');
  assert.equal(stackItemNameFromPath('/home/me/Documents'), 'Documents');
});

test('classifies stack browser row icons by folder and file extension', () => {
  assert.deepEqual(stackFileIconForEntry(stackEntry('Projects', 'Folder')), {
    kind: 'folder',
    label: 'Folder'
  });
  assert.deepEqual(stackFileIconForEntry(stackEntry('app.EXE')), {
    kind: 'app',
    label: 'Application'
  });
  assert.deepEqual(stackFileIconForEntry(stackEntry('photo.png')).kind, 'image');
  assert.deepEqual(stackFileIconForEntry({ ...stackEntry('unknown.custom'), typeLabel: 'CUSTOM File' }), {
    kind: 'file',
    label: 'CUSTOM File'
  });
});

test('stack browser fallback icon metadata does not use text abbreviations', () => {
  for (const entry of [
    stackEntry('Projects', 'Folder'),
    stackEntry('app.exe'),
    stackEntry('notes.txt'),
    { ...stackEntry('unknown.custom'), typeLabel: 'CUSTOM File' }
  ]) {
    assert.equal(Object.hasOwn(stackFileIconForEntry(entry), 'glyph'), false);
  }
});
