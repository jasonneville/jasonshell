import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const stackPopupSource = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const materialSymbolRegistrySource = readFileSync(
  new URL('../src/components/icons/materialSymbolIcons.ts', import.meta.url),
  'utf8'
);
const stackPopupStyles = readFileSync(new URL('../src/components/StackPopupSurface.css', import.meta.url), 'utf8');

function sourceBetween(source, startNeedle, endNeedle) {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `${startNeedle} exists`);
  const end = source.indexOf(endNeedle, start);
  assert.notEqual(end, -1, `${endNeedle} exists after ${startNeedle}`);
  return source.slice(start, end);
}

function assertToolbarIcon({ iconName, label, title = label }) {
  const materialIconPattern = new RegExp(
    `<MeltActionButton\\s+class="stack-action-icon-button"[\\s\\S]*ariaLabel="${label}"[\\s\\S]*tooltip="${title}"[\\s\\S]*<MaterialSymbolIcon\\s+name="${iconName}"\\s*/>[\\s\\S]*<\\/MeltActionButton>`
  );
  assert.match(stackToolbarSource, materialIconPattern, `${label} toolbar control uses accessible ${iconName} icon`);
}

const stackToolbarSource = sourceBetween(stackPopupSource, '<div class="stack-actions">', '{#if createFolderDraft');
const rowContextMenuSource = sourceBetween(stackPopupSource, '{#if rowMenu}', '{#if backgroundMenu}');
const backgroundContextMenuSource = sourceBetween(stackPopupSource, '{#if backgroundMenu}', '{#if deleteConfirmation}');

test('shared Material Symbols registry includes every stack browser toolbar icon', () => {
  for (const iconName of [
    'arrow_back',
    'arrow_forward',
    'refresh',
    'file_copy',
    'folder_copy',
    'content_cut',
    'content_paste',
    'drive_file_rename',
    'delete',
    'create_new_folder',
    'preview',
    'search'
  ]) {
    assert.match(materialSymbolRegistrySource, new RegExp(`['"]${iconName}['"]`), `${iconName} registered`);
  }
});

test('stack browser uses official outlined Material Symbol paths', () => {
  const officialPathHashes = {
    arrow_back: '3f525a2f9a67d03788b3ea8427a14a6f0b87803449b6997dd320f876f17439d7',
    arrow_forward: '674681b269599f0e40e545460a6a3223095e9c86135beeee541b1cfb1266694c',
    refresh: 'e28819bd0619154f239865bc62a3e93d4745c61f9e6497b88d3e8a5762e886c6',
    file_copy: '9fb0bb463ca46cdcf050c40380793e217d2b4be82a2b7a167024f51e91dadaee',
    folder_copy: 'ba731b7d613ef6f30fd6017528a21375d75c7f6a6856cbf2344191cff71e5ce6',
    content_cut: 'e5b5b2edbc7552ef5e692fe76978c6fcdf31b49220cd2302c4d789abccff8176',
    content_paste: 'b909f383232550fdf8c394b50866a9bd049aff7413cdd429791f95f03f6ef396',
    drive_file_rename: 'b309b0f6d5a68db21283af9bb2286c22d9d44e835cec7e42879fc5b0cc3ab5fa',
    delete: 'eecc33e20fd234261ab77b3ff520bbf40c565367c473db76b14ab0d3794d4df6',
    create_new_folder: '84a67975f945bc893ce48e0afdbc5c3319ab3698b2b9e56d1c5ac9c3b252861b',
    preview: '38eefe2fcb5408235a9777ebe087b891cd6c0d6ed91b4d4b583069d37f423500'
  };

  for (const [iconName, expectedHash] of Object.entries(officialPathHashes)) {
    const pathMatch = materialSymbolRegistrySource.match(new RegExp(`^  ${iconName}: '([^']+)'`, 'm'));
    assert.ok(pathMatch, `${iconName} path exists`);
    assert.equal(createHash('sha256').update(pathMatch[1]).digest('hex'), expectedHash, `${iconName} path matches Google source`);
  }
});

test('stack browser icon buttons have no background or border', () => {
  const buttonStyles = sourceBetween(stackPopupStyles, '.stack-actions button {', '.stack-actions button:disabled');
  const hoverStyles = sourceBetween(stackPopupStyles, '.stack-actions button:not(:disabled):hover,', '.stack-search {');
  assert.match(buttonStyles, /background:\s*transparent/);
  assert.match(buttonStyles, /border:\s*0/);
  assert.doesNotMatch(hoverStyles, /background:|border(?:-color)?:/);
});

test('stack browser toolbar text buttons are Material Symbol icon buttons with accessible labels', () => {
  assertToolbarIcon({ iconName: 'arrow_back', label: 'Back' });
  assertToolbarIcon({ iconName: 'arrow_forward', label: 'Forward' });
  assertToolbarIcon({ iconName: 'refresh', label: 'Refresh' });
  assertToolbarIcon({ iconName: 'content_cut', label: 'Cut selected item', title: 'Cut selected item' });
  assertToolbarIcon({ iconName: 'content_paste', label: 'Paste into current folder', title: 'Paste into current folder' });
  assertToolbarIcon({ iconName: 'drive_file_rename', label: 'Rename selected item', title: 'Rename selected item' });
  assertToolbarIcon({ iconName: 'delete', label: 'Delete selected item', title: 'Delete selected item' });
  assertToolbarIcon({ iconName: 'create_new_folder', label: 'New folder' });
  assertToolbarIcon({ iconName: 'preview', label: 'Reveal selected item', title: 'Reveal selected item' });

  assert.match(
    stackToolbarSource,
    /copySelectionIsFolder \? 'folder_copy' : 'file_copy'/,
    'Copy toolbar icon branches selected folders to folder_copy and files to file_copy'
  );
  assert.match(
    stackToolbarSource,
    /<MeltActionButton\s+class="stack-action-icon-button"[\s\S]*ariaLabel=\{copySelectionLabel\}[\s\S]*tooltip=\{copySelectionLabel\}[\s\S]*<MaterialSymbolIcon\s+name=\{copySelectionIsFolder \? 'folder_copy' : 'file_copy'\}\s*\/>[\s\S]*<\/MeltActionButton>/,
    'Copy toolbar control uses accessible file/folder copy icon branch'
  );

  for (const text of ['Back', 'Forward', 'Refresh', 'Cut selected item', 'Paste into current folder', 'Rename selected item', 'Delete selected item', 'New folder', 'Reveal selected item']) {
    assert.doesNotMatch(stackToolbarSource, new RegExp(`>${text}<`), `${text} toolbar text is removed`);
  }
});

test('stack browser search label uses icon-only chrome while context menus keep text labels', () => {
  const searchLabelSource = sourceBetween(stackToolbarSource, '<label class="stack-search"', '</label>');

  assert.match(
    searchLabelSource,
    /<MaterialSymbolIcon\s+name="search"\s*\/>/,
    'Search label uses accessible search icon'
  );
  assert.doesNotMatch(searchLabelSource, /<span>\s*Search\s*<\/span>/, 'Search text span is removed next to input');

  for (const menuText of ['Copy', 'Cut', 'Paste', 'Rename', 'Delete']) {
    const pattern = new RegExp(`>${menuText}<`);
    if (menuText === 'Paste') {
      assert.doesNotMatch(rowContextMenuSource, pattern, 'Paste removed from row context menu');
    } else {
      assert.match(rowContextMenuSource, pattern, `${menuText} row context menu text remains`);
    }
    assert.match(backgroundContextMenuSource, pattern, `${menuText} background context menu text remains`);
  }
});
