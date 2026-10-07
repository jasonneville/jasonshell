import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const stackPopupSource = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const materialSymbolRegistrySource = readFileSync(
  new URL('../src/components/icons/materialSymbolIcons.ts', import.meta.url),
  'utf8'
);
const materialSymbolIconSource = readFileSync(
  new URL('../src/components/icons/MaterialSymbolIcon.svelte', import.meta.url),
  'utf8'
);
const stackPopupStyles = readFileSync(new URL('../src/components/StackPopupSurface.css', import.meta.url), 'utf8');
const quickBarPinIcon = readFileSync(
  new URL('../src/assets/icons/add_location_24dp_E3E3E3_FILL0_wght300_GRAD0_opsz24.svg', import.meta.url),
  'utf8'
);

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
    'add_location',
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
    preview: '38eefe2fcb5408235a9777ebe087b891cd6c0d6ed91b4d4b583069d37f423500',
    add_location: '5d82cb3b4359490bb7963e115e92917bfac97712cd395c27818ea9c64086522b'
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

test('stack browser toolbar styling never depends on generated child order', () => {
  assert.doesNotMatch(
    stackPopupStyles,
    /\.stack-actions\s+button:nth-child\(/,
    'Melt tooltip siblings make positional toolbar selectors unstable'
  );
});

test('stack browser entire status sits inline after Pin before Search without footer reservation', () => {
  const popupStyles = sourceBetween(stackPopupStyles, '.stack-popup {', '.stack-popup.resizing');
  const statusStyles = sourceBetween(stackPopupStyles, '.stack-status {', '.inline-editor');
  const statusSource = sourceBetween(stackToolbarSource, '<div class="stack-status"', '<div class="stack-search">');

  assert.doesNotMatch(popupStyles, /padding-bottom:/, 'popup no longer reserves footer space');
  assert.doesNotMatch(stackPopupStyles, /--stack-status-height/, 'desktop and mobile footer reservation removed');
  assert.doesNotMatch(statusStyles, /position:\s*absolute|bottom:/, 'status stays in toolbar flow');
  assert.doesNotMatch(statusSource, /surface-state/, 'shared boxed/tinted state styling is not used');
  assert.match(statusStyles, /overflow:\s*hidden/, 'long status stays inside its reserved line');
  assert.match(statusStyles, /white-space:\s*nowrap/, 'narrow status does not wrap over file controls');
  assert.match(statusStyles, /text-overflow:\s*ellipsis/, 'long status has a visible truncation cue');
  assert.match(statusSource, /title=\{errorMessage \|\| stackState\.statusMessage\}/, 'full long error or status remains available on hover');
  assert.match(statusSource, /title=\{iconHydrationStatusMessage\}/, 'full secondary status remains available on hover');
  assert.match(statusSource, /title=\{operationStatusText\(activeOperation\)\}/, 'full operation text remains available');
  assert.match(statusSource, /role="status"\s+aria-live="polite"/, 'entire inline status remains a live region');
  assert.ok(stackToolbarSource.indexOf('ariaLabel="Pin to quick bar"') < stackToolbarSource.indexOf('<div class="stack-status"'));
  assert.equal((stackPopupSource.match(/class="stack-status(?:\s|")/g) ?? []).length, 1, 'no duplicate footer status');
  assert.match(stackPopupStyles, /\.inline-editor\s*\{[\s\S]*?grid-row:\s*2;/, 'inline editor keeps its own row above content');
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
    /<MeltActionButton class="stack-action-icon-button" ariaLabel=\{selectedEntry\?\.entryType === 'Folder' \? 'Copy folder' : 'Copy selected item'\} tooltip=\{selectedEntry\?\.entryType === 'Folder' \? 'Copy folder' : 'Copy selected item'\} disabled=\{!hasSelection\} onClick=\{\(\) => void copySelected\(false\)\}><MaterialSymbolIcon name=\{selectedEntry\?\.entryType === 'Folder' \? 'folder_copy' : 'file_copy'\} \/><\/MeltActionButton>/,
    'Copy toolbar control binds accessible label, tooltip, disabled state, and icon to selected entry'
  );
  assert.match(materialSymbolIconSource, /export let decorative = true;/, 'Copy icon is decorative by default');
  assert.match(materialSymbolIconSource, /aria-hidden=\{decorative \? 'true' : undefined\}/, 'Copy icon cannot override button accessible label');
  assert.match(readFileSync(new URL('../src/components/melt/MeltActionButton.svelte', import.meta.url), 'utf8'), /<button[\s\S]*?aria-label=\{ariaLabel\}[\s\S]*?disabled=\{disabled\}/, 'Melt control forwards accessible label and disabled state');

  for (const text of ['Back', 'Forward', 'Refresh', 'Cut selected item', 'Paste into current folder', 'Rename selected item', 'Delete selected item', 'New folder', 'Reveal selected item']) {
    assert.doesNotMatch(stackToolbarSource, new RegExp(`>${text}<`), `${text} toolbar text is removed`);
  }
});

test('stack browser pins the current folder from a shared 1rem Material Symbol immediately after Reveal', () => {
  assert.equal(
    createHash('sha256').update(quickBarPinIcon).digest('hex'),
    '6842216e981258d074fa733c3bbaaf692c301eafea7b45d8743059bf79d7a895',
    'quick-bar pin icon exactly matches supplied SVG'
  );
  assert.match(materialSymbolRegistrySource, /['"]add_location['"]/, 'add_location is registered');
  assert.doesNotMatch(stackPopupSource, /quickBarPinIcon/, 'standalone quick-bar icon URL is removed');
  assert.doesNotMatch(stackToolbarSource, /<img\b/, 'toolbar avoids native-size standalone image rendering');
  assert.match(materialSymbolIconSource, /width:\s*1rem;/, 'shared icon width is exactly 1rem');
  assert.match(materialSymbolIconSource, /height:\s*1rem;/, 'shared icon height is exactly 1rem');
  assert.match(
    stackPopupSource,
    /async function pinCurrentFolderToQuickBar\(\)[\s\S]*if \(!currentPath\) \{[\s\S]*await pinStackFolder\(currentPath\);/
  );
  assert.match(
    stackToolbarSource,
    /ariaLabel="Reveal selected item"[\s\S]*?<\/MeltActionButton>\s*(?:<div\b[^>]*>\s*)?<MeltActionButton\s+class="stack-action-icon-button"\s+ariaLabel="Pin to quick bar"\s+tooltip="Pin to quick bar"\s+disabled=\{!currentPath\}\s+onClick=\{\(\) => void pinCurrentFolderToQuickBar\(\)\}>\s*<MaterialSymbolIcon\s+name="add_location"\s*\/\>/
  );
});

test('stack browser search wrapper uses icon-only chrome while context menus keep text labels', () => {
  const searchWrapperSource = sourceBetween(stackPopupSource, '<div class="stack-search">', '{#if createFolderDraft');

  assert.match(
    searchWrapperSource,
    /<MaterialSymbolIcon\s+name="search"\s*\/>/,
    'Search wrapper uses search icon'
  );
  assert.match(searchWrapperSource, /aria-label="Search current folder"/, 'Search input keeps accessible name');
  assert.doesNotMatch(searchWrapperSource, /<label\b/, 'Search wrapper does not contain interactive controls inside a label');
  assert.doesNotMatch(searchWrapperSource, /<span>\s*Search\s*<\/span>/, 'Search text span is removed next to input');

  for (const menuText of ['Copy', 'Cut', 'Paste', 'Rename', 'Delete']) {
    const pattern = new RegExp(`>${menuText}<`);
    if (menuText === 'Paste') {
      assert.doesNotMatch(rowContextMenuSource, pattern, 'Paste removed from row context menu');
      assert.doesNotMatch(backgroundContextMenuSource, pattern, 'Paste removed from background context menu');
    } else {
      assert.match(rowContextMenuSource, pattern, `${menuText} row context menu text remains`);
      assert.match(backgroundContextMenuSource, pattern, `${menuText} background context menu text remains`);
    }
  }
});

test('stack browser search icon sits inside the input wrapper with room to its left', () => {
  const inputWrapperSource = sourceBetween(
    stackPopupSource,
    '<div class="stack-search-input-wrapper">',
    '</div>'
  );
  assert.match(
    inputWrapperSource,
    /<MaterialSymbolIcon\s+name="search"\s*\/>[\s\S]*?<input\b[^>]*aria-label="Search current folder"/,
    'search icon is inside the wrapper before the folder-search input'
  );
  const iconStyles = sourceBetween(stackPopupStyles, '.stack-search-input-wrapper .material-symbol-icon {', '.stack-search input {');
  const inputStyles = sourceBetween(stackPopupStyles, '.stack-search input {', '.stack-search-clear-button {');
  assert.match(iconStyles, /position:\s*absolute/, 'search icon is positioned within the input wrapper');
  assert.match(iconStyles, /left:\s*[^;]+;/, 'search icon is inset from the left edge');
  assert.match(inputStyles, /padding:\s*[^;]*\s(?:[1-9]\d*(?:\.\d+)?|0?\.\d*[1-9]\d*)rem\s*;/, 'input reserves nonzero left space for the icon');
});

test('stack browser search exposes a clear button only for a non-empty query', () => {
  const searchWrapperSource = sourceBetween(stackPopupSource, '<div class="stack-search">', '{#if createFolderDraft');
  const searchInputWrapperStyles = sourceBetween(stackPopupStyles, '.stack-search-input-wrapper {', '.stack-search-input-wrapper .material-symbol-icon');
  const clearButtonStyles = sourceBetween(stackPopupStyles, '.stack-search-clear-button {', '.stack-search-clear-button .material-symbol-icon');

  assert.match(searchWrapperSource, /<div class="stack-search-input-wrapper">/, 'input and clear button share stable wrapper');
  assert.match(searchWrapperSource, /bind:this=\{stackSearchInput\}/, 'search input can regain focus after clear');
  assert.match(searchWrapperSource, /\{#if searchQuery\}[\s\S]*ariaLabel="Clear search"/);
  assert.match(searchWrapperSource, /tooltip="Clear search"/);
  assert.match(searchWrapperSource, /onClick=\{clearStackSearch\}/);
  assert.match(searchWrapperSource, /<MaterialSymbolIcon\s+name="close"\s*\/>/);
  assert.match(searchInputWrapperStyles, /position:\s*relative/, 'search input wrapper anchors clear button');
  assert.match(clearButtonStyles, /position:\s*absolute/, 'clear button uses stable absolute positioning');
  assert.match(clearButtonStyles, /right:\s*0\.18rem/, 'clear button stays inset from input edge');
  assert.doesNotMatch(clearButtonStyles, /margin-left:\s*-/, 'clear button avoids brittle negative margin');
  assert.match(stackPopupSource, /function clearStackSearch\(\)[\s\S]*searchQuery = '';/);
});

test('Escape unfocuses stack browser search without closing or clearing it', () => {
  const searchWrapperSource = sourceBetween(stackPopupSource, '<div class="stack-search">', '{#if createFolderDraft');
  const searchKeydownSource = sourceBetween(
    stackPopupSource,
    'function handleStackSearchKeydown',
    'async function clearStackSearch'
  );

  assert.match(searchWrapperSource, /on:keydown=\{handleStackSearchKeydown\}/);
  assert.match(
    searchKeydownSource,
    /if \(event\.key === 'Escape'\) \{\s*event\.preventDefault\(\);\s*input\.blur\(\);\s*\}/,
    'Escape branch prevents default and blurs input'
  );
  assert.match(searchKeydownSource, /event\.stopPropagation\(\)/);
  assert.doesNotMatch(searchKeydownSource, /searchQuery\s*=/);
  assert.doesNotMatch(searchKeydownSource, /closeStackPopupFromSurface|hideStackPopup/);
});
