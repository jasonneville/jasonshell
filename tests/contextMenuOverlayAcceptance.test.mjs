import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const components = readdirSync(new URL('../src/components/', import.meta.url));
const overlayName = components.find((name) => /contextmenu.*(?:overlay|popup).*surface\.svelte$/i.test(name));

test('shared context-menu overlay is its own native surface rather than a thin-bar child', () => {
  assert.ok(overlayName, 'dedicated shared ContextMenu overlay/popup surface must exist');
  const overlay = read(`src/components/${overlayName}`);
  assert.match(overlay, /<ContextMenu\b/, 'overlay hosts shared visual menu');
  assert.match(read('src/lib/surfaceLoader.ts'), /context.menu.*(?:overlay|popup)|(?:overlay|popup).*context.menu/i, 'surface loader must route overlay window');
  assert.match(read('src-tauri/src/shell_windows.rs'), /context.menu.*(?:overlay|popup)|(?:overlay|popup).*context.menu/i, 'native window must own overlay');
});

test('top and bottom bars request the same external overlay; neither renders an inline menu', () => {
  const top = read('src/components/TopBar.svelte');
  const bottom = read('src/components/BottomBar.svelte');
  for (const [name, bar] of [['TopBar', top], ['BottomBar', bottom]]) {
    assert.doesNotMatch(bar, /<ContextMenu\b/, `${name} must not render a menu inside the thin WebView`);
    assert.match(bar, /(?:show|open|publish)\w*(?:ContextMenu|MenuOverlay|MenuPopup)\s*\(/, `${name} must request the popup on right click`);
  }
});

test('overlay clamps multirow menus within its own viewport and closes on dismiss or selection', () => {
  assert.ok(overlayName, 'shared overlay surface required');
  const overlay = read(`src/components/${overlayName}`);
  assert.match(overlay, /positionScrollableContextMenuInViewport|positionContextMenuInViewport/, 'use existing viewport placement helper');
  assert.match(overlay, /(?:close|hide|dismiss)/i, 'overlay must provide close lifecycle');
  assert.match(overlay, /(?:Escape|focus|blur|pointerdown)/, 'dismiss on escape or focus/outside interaction');
});

test('overlay action transport retains authorized pin, task and launcher operations without trusting arbitrary requests', () => {
  assert.ok(overlayName, 'shared overlay surface required');
  const overlay = read(`src/components/${overlayName}`);
  const top = read('src/components/TopBar.svelte');
  const bottom = read('src/components/BottomBar.svelte');
  const native = read('src-tauri/src/taskbar_menu.rs');
  assert.match(overlay, /(?:runTaskWindowAction|runTaskbarLauncherAction|action|dispatch)/, 'overlay must dispatch selected actions');
  assert.match(top + overlay, /openStackFolderInVscode|openInVscode/, 'pin VS Code action remains available');
  assert.match(top + overlay, /unpinFromMenu|unpin/, 'pin removal remains available');
  assert.match(bottom + overlay, /(?:focusPid|processId)/, 'PID-focused Process Manager remains available');
  assert.match(bottom + overlay, /runTaskWindowAction|run_task_window_action/, 'task actions remain available');
  assert.match(bottom + overlay, /runTaskbarLauncherAction|run_taskbar_launcher_action/, 'launcher actions remain available');
  assert.match(overlay, /token:\s*request\.token/, 'selection echoes opaque request token');
  assert.match(top, /selection\.token !== pinContextMenu\?\.token/, 'top bar rejects stale pin selections');
  assert.match(bottom, /selection\.kind === 'task-window' && selection\.token === taskContextMenu\?\.token/, 'bottom bar rejects stale task selections');
  assert.match(bottom, /selection\.kind === 'launcher' && selection\.token === launcherContextMenu\?\.token/, 'bottom bar rejects stale launcher selections');
  assert.match(native, /if window\.label\(\) != BOTTOM_BAR_LABEL/, 'existing bottom-bar authorization must be retained or explicitly reworked for overlay');
});

test('bottom menu uses measured menu height and source anchor instead of fixed top offset', () => {
  const overlay = read(`src/components/${overlayName}`);
  const native = read('src-tauri/src/context_menu_overlay.rs');
  const request = read('src/lib/contextMenuOverlay.ts');
  assert.match(request, /(?:anchor|placement|y):/, 'show request must carry an anchor or placement hint');
  assert.match(native, /request\.y/, 'native overlay position must account for the clicked row coordinate');
  assert.doesNotMatch(overlay, /\{\s*x:\s*8,\s*y:\s*8\s*\}/, 'no fixed local top-left menu anchor');
  assert.match(overlay, /request\.source[\s\S]{0,250}(?:bounds\.height|menuElement|placement)|(?:bounds\.height|menuElement|placement)[\s\S]{0,250}request\.source/, 'bottom vs top placement must use measured menu height');
});

test('unused transparent popup area dismisses on pointerdown while menu clicks remain interactive', () => {
  const overlay = read(`src/components/${overlayName}`);
  assert.match(overlay, /on:pointerdown=\{[^}]*dismiss|onPointerDown=\{[^}]*dismiss/, 'overlay backdrop must dismiss on pointerdown');
  assert.match(overlay, /<ContextMenu\b[\s\S]*?on:click=\{\(event\) => event\.stopPropagation\(\)\}/, 'click inside menu must not dismiss before action');
});

test('task overlay keeps originating minimized and PID state for disabled actions and dynamic focus label', () => {
  const bottom = read('src/components/BottomBar.svelte');
  const overlay = read(`src/components/${overlayName}`);
  const contract = read('src/lib/contextMenuOverlay.ts');
  assert.match(bottom, /showContextMenuOverlay\(\{[^}]*kind: 'task-window'[^}]*isMinimized:\s*taskWindow\.isMinimized/, 'origin sends minimized presentation state');
  assert.match(bottom, /showContextMenuOverlay\(\{[^}]*kind: 'task-window'[^}]*processId:\s*normalizeTaskGalleryProcessId\(taskWindow\.processId\)/, 'origin sends only presentation PID, not HWND');
  assert.match(contract, /isMinimized\??:|(?:canMinimize|isMinimized)\??:/, 'request has minimized state');
  assert.match(overlay, /disabled=\{[^}]*request\.(?:isMinimized|canMinimize)[^}]*\}[^>]*>Minimize/, 'minimize disabled when already minimized');
  assert.match(overlay, /disabled=\{[^}]*request\.(?:processId|hasProcess)[^}]*\}[^>]*>.*Process Manager/, 'process action disabled without valid PID');
  assert.match(overlay, /request\.isMinimized[^\n]*Restore[^\n]*Switch|request\.isMinimized[^\n]*Switch[^\n]*Restore/, 'focus label reflects minimized state');
});

test('every origin uses unpredictable opaque tokens without leaking path or HWND to overlay request', () => {
  for (const path of ['src/components/TopBar.svelte', 'src/components/BottomBar.svelte']) {
    const bar = read(path);
    assert.match(bar, /crypto\.randomUUID\(\)/, `${path} generates unpredictable token`);
    assert.doesNotMatch(bar, /const token\s*=\s*`[^`]*(?:Date\.now|pin\.path|taskWindow\.hwnd|launcher\.shortcutPath)/, `${path} must not encode a sensitive target in token`);
  }
  const contract = read('src/lib/contextMenuOverlay.ts');
  assert.doesNotMatch(contract, /\b(?:hwnd|shortcutPath|path):\s*string/, 'overlay request must not receive sensitive action targets');
});

test('opening overlay focuses first enabled menu action after Svelte renders it', () => {
  const overlay = read(`src/components/${overlayName}`);
  const openHandler = overlay.match(/listen<ContextMenuOverlayRequest>\(CONTEXT_MENU_OVERLAY_OPEN_EVENT,[\s\S]*?\n\s*\}\);/)?.[0] ?? '';
  assert.match(openHandler, /positionMenu\(\)/, 'open event must schedule positioning');
  assert.match(overlay, /(?:menuElement\?\.querySelector|menuElement\.querySelector|menuElement\?\.querySelectorAll|menuElement\.querySelectorAll)[\s\S]{0,160}(?:menuitem|context-menu-item|button)/, 'focus target must be an actual menu item, not menu container');
  assert.match(overlay, /(?:button:not\(:disabled\)|:not\(\[disabled\]\)|\.disabled\s*===?\s*false|!\w+\.disabled)/, 'first focus target must exclude disabled actions');
  const positionHandler = overlay.slice(overlay.indexOf('async function positionMenu()'), overlay.indexOf('function dismiss()'));
  assert.match(positionHandler, /(?:focus\(|focusFirst|focusEnabled|focusMenu)/, 'focus follows render and measured positioning');
});

test('overlay keyboard navigation supports arrows and boundaries, skips disabled rows, and restores origin focus on Escape', () => {
  const overlay = read(`src/components/${overlayName}`);
  const keyHandler = overlay.slice(overlay.indexOf('function handleKeydown(event: KeyboardEvent)'), overlay.indexOf('onMount(() =>'));
  for (const key of ['ArrowDown', 'ArrowUp', 'Home', 'End']) {
    assert.match(keyHandler, new RegExp(key), `${key} must navigate menu items`);
  }
  assert.match(overlay, /(?:button:not\(:disabled\)|:not\(\[disabled\]\)|!\w+\.disabled)/, 'navigation excludes disabled items');
  assert.match(overlay, /\.focus\(\)/, 'keyboard navigation moves real DOM focus');
  assert.match(keyHandler, /Escape[\s\S]*dismiss\(/, 'Escape dismisses menu');
  const top = read('src/components/TopBar.svelte');
  const bottom = read('src/components/BottomBar.svelte');
  assert.match(overlay + top + bottom, /(?:restore|return|focus).*?(?:origin|trigger)|(?:origin|trigger).*?(?:restore|return|focus)/i, 'dismiss lifecycle restores origin trigger focus where available');
});
