import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const stackPopupApi = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');

test('row context menu exposes working Open with picker labels only', () => {
  assert.equal(surface.includes('Open width'), false);
  assert.equal(surface.includes('Default width'), false);
  assert.match(surface, />Open with ▸<\/button>/);
  assert.match(surface, />Choose app\.\.\.<\/button>/);
  assert.match(surface, /openSelectedWithPicker\(\)/);
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
  assert.match(backgroundMenu, />Copy<\/button>/);
  assert.match(backgroundMenu, />Cut<\/button>/);
  assert.match(backgroundMenu, />Rename<\/button>/);
  assert.match(backgroundMenu, />Delete<\/button>/);
  assert.match(backgroundMenu, />Reveal<\/button>/);
  assert.match(backgroundMenu, />Paste<\/button>/);
  assert.match(backgroundMenu, />New Folder<\/button>/);
});
