import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const css = readFileSync(new URL('../src/components/StackPopupSurface.css', import.meta.url), 'utf8');
const stackPopupApi = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');

test('row context menu exposes Open with picker plus suggested developer apps', () => {
  assert.equal(surface.includes('Open width'), false);
  assert.equal(surface.includes('Default width'), false);
  assert.match(surface, />Open with ▸<\/MeltActionButton>/);
  assert.match(surface, /openWithSuggestions/);
  assert.match(surface, /openSelectedWithSuggestedApp\(app\)/);
  assert.match(surface, />\{app\.label\}<\/MeltActionButton>/);
  assert.match(stackPopupApi, /listStackOpenWithCandidates\(path: string\): Promise<StackOpenWithCandidate\[]>/);
  assert.match(stackPopupApi, /openStackItemWithApp\(path: string, appId: string\): Promise<void>/);
  assert.match(surface, />Choose app\.\.\.<\/MeltActionButton>/);
  assert.match(surface, /openSelectedWithPicker\(\)/);
});

test('Open with flyout has a bridge so rightward mouse movement stays inside submenu zone', () => {
  assert.match(css, /\.context-submenu::after/);
  assert.match(css, /left: 100%;/);
  assert.match(css, /\.context-submenu:hover \.context-submenu-panel/);
  assert.doesNotMatch(css, /left: calc\(100% \+ 0\.25rem\)/);
});

test('Open with picker is backed by a Tauri command wrapper', () => {
  assert.match(stackPopupApi, /openStackItemWithPicker\(path: string\): Promise<void>/);
  assert.match(stackPopupApi, /invoke\(IPC_COMMANDS\.openStackItemWithPicker, \{ path \}\)/);
});

test('background context menu is available off rows and keeps selection actions', () => {
  assert.match(surface, /on:contextmenu=\{handleBackgroundContextMenu\}/);
  assert.match(surface, /function shouldIgnoreBackgroundContextMenu/);
  assert.match(surface, /STACK_BROWSER_BACKGROUND_CONTEXT_MENU_IGNORE_SELECTORS/);

  const backgroundMenu = surface.slice(
    surface.indexOf('{#if backgroundMenu}'),
    surface.indexOf('{#if deleteConfirmation}')
  );
  assert.match(backgroundMenu, />Copy<\/MeltActionButton>/);
  assert.match(backgroundMenu, />Cut<\/MeltActionButton>/);
  assert.match(backgroundMenu, />Rename<\/MeltActionButton>/);
  assert.match(backgroundMenu, />Delete<\/MeltActionButton>/);
  assert.match(backgroundMenu, />Reveal<\/MeltActionButton>/);
  assert.match(backgroundMenu, />Paste<\/MeltActionButton>/);
  assert.match(backgroundMenu, />New Folder<\/MeltActionButton>/);
  assert.match(backgroundMenu, />New Text File<\/MeltActionButton>/);
  assert.match(backgroundMenu, />Copy Folder Path<\/MeltActionButton>/);
  assert.match(backgroundMenu, />Open Terminal Here<\/MeltActionButton>/);
  assert.match(stackPopupApi, /newStackTextFile\(parent: string\): Promise<StackEntry>/);
  assert.match(stackPopupApi, /openStackTerminalHere\(path: string\): Promise<void>/);
});

test('stack browser publishes native-friendly file drag payloads as copy operations', () => {
  assert.match(surface, /folderPathToUri/);
  assert.match(surface, /prepareStackFileDrag\(paths\)/);
  assert.match(surface, /event\.dataTransfer\.effectAllowed = 'copy'/);
  assert.match(surface, /setData\('text\/uri-list'/);
  assert.match(surface, /setData\('DownloadURL'/);
});
