import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('stack browser delete confirmation stays inside stack popup webview', () => {
  const stackSurface = readFileSync('src/components/StackPopupSurface.svelte', 'utf8');
  const stackRust = readFileSync('src-tauri/src/stack_popup.rs', 'utf8');
  const main = readFileSync('src-tauri/src/main.rs', 'utf8');

  assert.doesNotMatch(stackSurface, /window\.confirm/);
  assert.match(stackSurface, /deleteConfirmation/);
  assert.match(stackSurface, /role="dialog"/);
  assert.match(stackSurface, /confirmDeleteSelection/);
  assert.match(stackRust, /begin_stack_popup_focus_hold/);
  assert.match(stackRust, /end_stack_popup_focus_hold/);
  assert.match(stackRust, /begin_stack_popup_focus_loss_hold/);
  assert.match(stackRust, /end_stack_popup_focus_loss_hold/);
  assert.match(main, /suppress_stack_popup_focus_loss/);
  assert.match(stackSurface, /await beginStackPopupFocusLossHold\(\)/);
  assert.match(stackSurface, /await deleteStackItem\(path\)/);
  assert.match(stackSurface, /applyStackFolderListing\(stackState, pendingDelete\.folderPath, listing\)/);
  assert.match(stackSurface, /await endStackPopupFocusLossHold\(\)/);
});

test('stack browser exposes persisted resize grip and resize command wiring', () => {
  const stackSurface = readFileSync('src/components/StackPopupSurface.svelte', 'utf8');
  const stackCss = readFileSync('src/components/StackPopupSurface.css', 'utf8');
  const stackWrapper = readFileSync('src/lib/stackPopup.ts', 'utf8');
  const commands = readFileSync('src/ipc/commands.ts', 'utf8');
  const main = readFileSync('src-tauri/src/main.rs', 'utf8');
  const popupWindow = readFileSync('src-tauri/src/stack_popup/popup_window.rs', 'utf8');

  assert.match(commands, /resizeStackPopup: 'resize_stack_popup'/);
  assert.match(commands, /beginStackPopupFocusLossHold: 'begin_stack_popup_focus_loss_hold'/);
  assert.match(commands, /endStackPopupFocusLossHold: 'end_stack_popup_focus_loss_hold'/);
  assert.match(stackWrapper, /resizeStackPopup\(width: number, height: number, persist = false\)/);
  assert.match(stackWrapper, /beginStackPopupFocusLossHold\(\)/);
  assert.match(stackWrapper, /endStackPopupFocusLossHold\(\)/);
  assert.match(main, /stack_popup::resize_stack_popup/);
  assert.match(main, /stack_popup::begin_stack_popup_focus_loss_hold/);
  assert.match(main, /stack_popup::end_stack_popup_focus_loss_hold/);
  assert.match(stackSurface, /class="stack-resize-grip"/);
  assert.match(stackSurface, /on:pointerdown=\{beginResize\}/);
  assert.match(stackSurface, /resizeRequestChain/);
  assert.match(stackSurface, /resizeStackPopup\(request\.width, request\.height, request\.persist\)/);
  assert.match(stackCss, /\.stack-resize-grip/);
  assert.match(popupWindow, /stack-popup-geometry-v1\.json/);
  assert.match(popupWindow, /save_stack_popup_size/);
  assert.match(popupWindow, /load_stack_popup_size/);
});

test('search panel has outside-dismiss and result-interaction handshake', () => {
  const topBar = readFileSync('src/components/TopBar.svelte', 'utf8');
  const searchSurface = readFileSync('src/components/SearchPanelSurface.svelte', 'utf8');
  const searchPanel = readFileSync('src/lib/searchPanel.ts', 'utf8');
  const main = readFileSync('src-tauri/src/main.rs', 'utf8');

  assert.match(searchPanel, /SEARCH_PANEL_INTERACTION_EVENT = 'search-panel:interaction'/);
  assert.match(searchPanel, /SEARCH_PANEL_CLOSED_EVENT = 'search-panel:closed'/);
  assert.match(searchSurface, /<svelte:window on:mousedown=\{markPanelInteraction\}/);
  assert.match(topBar, /on:pointerdown=\{handleTopBarPointerDown\}/);
  assert.match(topBar, /on:blur=\{scheduleSearchBlurClose\}/);
  assert.match(topBar, /SEARCH_PANEL_INTERACTION_EVENT/);
  assert.match(topBar, /SEARCH_PANEL_CLOSED_EVENT/);
  assert.match(main, /window\.label\(\) == shell_windows::SEARCH_PANEL_LABEL/);
  assert.match(main, /WindowEvent::Focused\(true\)/);
  assert.match(main, /search_panel::SEARCH_PANEL_INTERACTION_EVENT/);
  assert.match(main, /search_panel::emit_search_panel_closed_to_top_bar\(window\.app_handle\(\)\)/);
});

test('outside-click and blur search dismissal route through reset-closing path', () => {
  const topBar = readFileSync('src/components/TopBar.svelte', 'utf8');

  assert.match(topBar, /function resetActiveSearchState\(\)/);
  assert.match(topBar, /on:pointerdown=\{handleTopBarPointerDown\}/);
  assert.match(topBar, /on:blur=\{scheduleSearchBlurClose\}/);
  assert.match(topBar, /function handleTopBarPointerDown[\s\S]*void closePanel\(\);[\s\S]*function openSettingsPanel/);
  assert.match(topBar, /function scheduleSearchBlurClose[\s\S]*void closePanel\(\);[\s\S]*function handleTopBarPointerDown/);
  assert.match(topBar, /async function closePanel\(\) \{[\s\S]*resetActiveSearchState\(\)/);
});
