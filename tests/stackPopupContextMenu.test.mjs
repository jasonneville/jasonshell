import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const css = readFileSync(new URL('../src/components/StackPopupSurface.css', import.meta.url), 'utf8');
const stackPopupApi = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');

test('row context menu exposes Open with picker plus suggested developer apps', () => {
  assert.equal(surface.includes('Open width'), false);
  assert.equal(surface.includes('Default width'), false);
  assert.match(surface, />Open with ▸<\/ContextMenuItem>/);
  assert.match(surface, /openWithSuggestions/);
  assert.match(surface, /openSelectedWithSuggestedApp\(app\)/);
  assert.match(surface, />\{app\.label\}<\/ContextMenuItem>/);
  assert.match(stackPopupApi, /listStackOpenWithCandidates\(path: string\): Promise<StackOpenWithCandidate\[]>/);
  assert.match(stackPopupApi, /openStackItemWithApp\(path: string, appId: string\): Promise<void>/);
  assert.match(surface, />Choose app\.\.\.<\/ContextMenuItem>/);
  assert.match(surface, /openSelectedWithPicker\(\)/);
});

test('Open with flyout has a bridge so rightward mouse movement stays inside submenu zone', () => {
  assert.match(css, /\.context-submenu-panel::before/);
  assert.match(css, /right: 100%;/);
  assert.match(css, /\.context-menu-shell:has\(\.context-submenu:hover\) > \.context-submenu-panel/);
  assert.doesNotMatch(css, /left: calc\(100% \+ 0\.25rem\)/);
});

test('Open with flyout is a sibling of the independently scrolling root menu', () => {
  const rowMenu = surface.slice(
    surface.indexOf('{#if rowMenu}'),
    surface.indexOf('{#if backgroundMenu}')
  );

  assert.match(rowMenu, /class="context-menu-shell"/);
  assert.match(rowMenu, /<ContextMenu bind:element=\{rowMenuElement\} className="context-menu context-menu-scroll"[\s\S]*?<\/ContextMenu>\s*<ContextMenu bind:element=\{rowSubmenuPanelElement\} className="context-menu context-submenu-panel"/);
  assert.match(rowMenu, /on:keydown=\{\(event\) => void handleRowMenuKeydown\(event\)\}/);
  assert.match(surface, /event\.key !== 'ArrowRight'[\s\S]*?rowSubmenuPanelElement\?\.querySelector<HTMLElement>\('button:not\(:disabled\)'\)\?\.focus\(\)/);
  assert.match(css, /\.context-menu-scroll\s*\{[\s\S]*?overflow-y:\s*auto;/);
  assert.match(css, /\.context-menu-shell\s*\{[\s\S]*?overflow:\s*visible;/);
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
  for (const label of ['Copy', 'Cut', 'Rename', 'Delete', 'Reveal', 'New Folder', 'New Text File', 'Copy Folder Path', 'Open Terminal Here']) {
    assert.ok(backgroundMenu.includes(`>${label}</ContextMenuItem>`), `${label} must remain a background menu action`);
  }
  assert.doesNotMatch(backgroundMenu, />Paste<\/ContextMenuItem>/, 'background menu intentionally omits Paste');
  const toolbar = surface.slice(surface.indexOf('<div class="stack-actions">'), surface.indexOf('<div class="stack-search">'));
  assert.match(toolbar, /ariaLabel="Paste into current folder"[^>]*disabled=\{!currentPath\}[^>]*onClick=\{\(\) => void pasteIntoCurrentFolder\(\)\}/, 'toolbar retains Paste ownership');
  assert.match(stackPopupApi, /newStackTextFile\(parent: string\): Promise<StackEntry>/);
  assert.match(stackPopupApi, /openStackTerminalHere\(path: string\): Promise<void>/);
});

test('stack browser starts outbound native drag via row pointer handlers, not HTML dragstart', () => {
  const row = surface.slice(surface.indexOf('{#each virtualEntries.rows'));
  const rowButton = row.slice(row.indexOf('<button'), row.indexOf('on:drop=', row.indexOf('<button')));
  assert.match(rowButton, /on:pointerdown=\{\(event\) => beginRowDrag\(event, entry\)\}/);
  assert.match(rowButton, /on:pointermove=\{\(event\) => moveRowDrag\(event, entry\)\}/);
  assert.doesNotMatch(rowButton, /on:dragstart=|\bdraggable=/);
  assert.match(stackPopupApi, /invoke<StackNativeDragOutcome>\(IPC_COMMANDS\.startStackFileDrag, \{ paths \}\)/);
});
