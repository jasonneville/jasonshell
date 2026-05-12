import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  STACK_BROWSER_BACKGROUND_CONTEXT_MENU_IGNORE_SELECTORS,
  stackBrowserMarqueeRect,
  stackBrowserMarqueeSelectedVirtualPaths,
  stackBrowserMarqueeSelectedPaths,
  stackBrowserBreadcrumbOverflow,
  stackBrowserDeletePrompt,
  stackBrowserSearchEntries,
  stackBrowserScrollTopForIndex,
  stackBrowserVirtualWindow
} from '../dist-tests/lib/stackPopupViewModel.js';

test('declares background context menu ignore selectors for interactive stack chrome', () => {
  assert.deepEqual([...STACK_BROWSER_BACKGROUND_CONTEXT_MENU_IGNORE_SELECTORS], [
    '[role="row"]',
    '.context-menu',
    '.delete-confirm-dialog',
    '.inline-editor',
    '.stack-toolbar',
    '.stack-resize-grip'
  ]);
});

function entry(name, entryType = 'File') {
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

test('calculates a bounded virtual row window for large folders', () => {
  const rows = Array.from({ length: 1000 }, (_, index) => `row-${index}`);
  const window = stackBrowserVirtualWindow(rows, 3000, 300, {
    rowHeight: 30,
    overscan: 5,
    minRows: 100
  });

  assert.equal(window.enabled, true);
  assert.equal(window.startIndex, 95);
  assert.equal(window.endIndex, 115);
  assert.equal(window.rows.length, 20);
  assert.equal(window.rows[0].item, 'row-95');
  assert.equal(window.rows.at(-1).item, 'row-114');
  assert.equal(window.beforeHeight, 2850);
  assert.equal(window.afterHeight, 26550);
});

test('keeps small folders unvirtualized to preserve simple row semantics', () => {
  const rows = ['alpha', 'bravo'];
  const window = stackBrowserVirtualWindow(rows, 500, 300, { minRows: 100 });

  assert.equal(window.enabled, false);
  assert.deepEqual(window.rows.map((row) => row.item), rows);
  assert.equal(window.beforeHeight, 0);
  assert.equal(window.afterHeight, 0);
});

test('calculates scroll positions that keep keyboard-selected virtual rows mounted', () => {
  assert.equal(
    stackBrowserScrollTopForIndex(150, 0, 300, 1000, { rowHeight: 30 }),
    4230
  );
  assert.equal(
    stackBrowserScrollTopForIndex(2, 3000, 300, 1000, { rowHeight: 30 }),
    60
  );
  assert.equal(
    stackBrowserScrollTopForIndex(105, 3000, 300, 1000, { rowHeight: 30 }),
    3000
  );
});

test('normalizes stack browser marquee rectangles from either drag direction', () => {
  assert.deepEqual(stackBrowserMarqueeRect({ x: 180, y: 90 }, { x: 40, y: 160 }), {
    left: 40,
    top: 90,
    right: 180,
    bottom: 160,
    width: 140,
    height: 70
  });
});

test('selects visible stack rows whose bounds intersect the marquee rectangle', () => {
  const rows = [
    { path: 'C:\\Users\\me\\Documents\\alpha.txt', rect: { left: 12, top: 10, right: 360, bottom: 40 } },
    { path: 'C:\\Users\\me\\Documents\\bravo.txt', rect: { left: 12, top: 40, right: 360, bottom: 70 } },
    { path: 'C:\\Users\\me\\Documents\\charlie.txt', rect: { left: 12, top: 70, right: 360, bottom: 100 } },
    { path: 'C:\\Users\\me\\Documents\\delta.txt', rect: { left: 12, top: 100, right: 360, bottom: 130 } }
  ];

  assert.deepEqual(
    stackBrowserMarqueeSelectedPaths(
      rows,
      { left: 0, top: 35, right: 220, bottom: 106, width: 220, height: 71 },
      ['C:\\Users\\me\\Documents\\preselected.txt'],
      false
    ),
    [
      'C:\\Users\\me\\Documents\\alpha.txt',
      'C:\\Users\\me\\Documents\\bravo.txt',
      'C:\\Users\\me\\Documents\\charlie.txt',
      'C:\\Users\\me\\Documents\\delta.txt'
    ]
  );
});

test('additive stack browser marquee keeps existing selection and appends intersecting rows once', () => {
  const rows = [
    { path: 'C:\\Users\\me\\Documents\\alpha.txt', rect: { left: 20, top: 20, right: 300, bottom: 50 } },
    { path: 'C:\\Users\\me\\Documents\\bravo.txt', rect: { left: 20, top: 50, right: 300, bottom: 80 } }
  ];

  assert.deepEqual(
    stackBrowserMarqueeSelectedPaths(
      rows,
      { left: 0, top: 45, right: 100, bottom: 65, width: 100, height: 20 },
      ['C:\\Users\\me\\Documents\\alpha.txt', 'C:\\Users\\me\\Documents\\zulu.txt'],
      true
    ),
    [
      'C:\\Users\\me\\Documents\\alpha.txt',
      'C:\\Users\\me\\Documents\\zulu.txt',
      'C:\\Users\\me\\Documents\\bravo.txt'
    ]
  );
});

test('virtual stack browser marquee selects unmounted rows by row geometry during autoscroll', () => {
  const paths = Array.from({ length: 20 }, (_, index) => `C:\\Users\\me\\Documents\\row-${index}.txt`);

  assert.deepEqual(
    stackBrowserMarqueeSelectedVirtualPaths(
      paths,
      { left: 0, top: 45, right: 300, bottom: 155, width: 300, height: 110 },
      {
        rowHeight: 30,
        rowLeft: 12,
        rowRight: 360,
        viewportTop: 100,
        scrollTop: 90,
        existingSelection: ['C:\\Users\\me\\Documents\\old.txt'],
        additive: false
      }
    ),
    paths.slice(1, 5)
  );
});

test('virtual stack browser additive marquee preserves drag-start and prior drag selections once rows unmount', () => {
  const paths = Array.from({ length: 20 }, (_, index) => `C:\\Users\\me\\Documents\\row-${index}.txt`);

  assert.deepEqual(
    stackBrowserMarqueeSelectedVirtualPaths(
      paths,
      { left: 0, top: 185, right: 300, bottom: 246, width: 300, height: 61 },
      {
        rowHeight: 30,
        rowLeft: 12,
        rowRight: 360,
        viewportTop: 100,
        scrollTop: 90,
        existingSelection: [paths[0], paths[1]],
        additive: true
      }
    ),
    [paths[0], paths[1], paths[5], paths[6], paths[7]]
  );
});

test('collapses middle breadcrumbs while preserving root and current folder', () => {
  const segments = [
    { name: 'C:', path: 'C:\\' },
    { name: 'Users', path: 'C:\\Users' },
    { name: 'me', path: 'C:\\Users\\me' },
    { name: 'Documents', path: 'C:\\Users\\me\\Documents' },
    { name: 'Projects', path: 'C:\\Users\\me\\Documents\\Projects' },
    { name: 'JasonShell', path: 'C:\\Users\\me\\Documents\\Projects\\JasonShell' }
  ];

  const overflow = stackBrowserBreadcrumbOverflow(segments, 4);

  assert.deepEqual(
    overflow.visibleSegments.map((segment) => segment.name),
    ['C:', 'Documents', 'Projects', 'JasonShell']
  );
  assert.deepEqual(
    overflow.hiddenSegments.map((segment) => segment.name),
    ['Users', 'me']
  );
  assert.equal(overflow.hiddenCount, 2);
  assert.equal(overflow.hiddenTitle, 'Users / me');
});

test('builds explicit delete prompt state from visible selection only', () => {
  const entries = [entry('alpha.txt'), entry('bravo.txt'), entry('charlie.txt')];
  const prompt = stackBrowserDeletePrompt(
    entries,
    [entries[0].path, 'C:\\Users\\me\\Documents\\stale.txt', entries[2].path],
    null
  );

  assert.equal(prompt.canDelete, true);
  assert.deepEqual(prompt.paths, [entries[0].path, entries[2].path]);
  assert.equal(prompt.itemCount, 2);
  assert.equal(prompt.label, '2 items');
  assert.equal(prompt.message, 'Delete 2 selected items? This cannot be undone.');
});

test('delete prompt falls back to selectedPath and rejects empty selections', () => {
  const entries = [entry('alpha.txt')];

  assert.deepEqual(stackBrowserDeletePrompt(entries, [], entries[0].path), {
    canDelete: true,
    paths: [entries[0].path],
    itemCount: 1,
    label: 'alpha.txt',
    title: 'Delete alpha.txt?',
    message: 'Delete "alpha.txt"? This cannot be undone.'
  });

  assert.equal(stackBrowserDeletePrompt(entries, [], null).canDelete, false);
});

test('filters stack browser entries by name and path search text', () => {
  const entries = [
    entry('Alpha Notes.txt'),
    entry('budget.xlsx'),
    { ...entry('src', 'Folder'), path: 'C:\\Users\\me\\Documents\\Project\\src' }
  ];

  assert.deepEqual(
    stackBrowserSearchEntries(entries, 'notes').map((item) => item.name),
    ['Alpha Notes.txt']
  );
  assert.deepEqual(
    stackBrowserSearchEntries(entries, 'project').map((item) => item.name),
    ['src']
  );
  assert.deepEqual(stackBrowserSearchEntries(entries, '   '), entries);
});
