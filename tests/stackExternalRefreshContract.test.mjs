import assert from 'node:assert/strict';
import test from 'node:test';
import { readRepoFile as source } from './support/repoSource.mjs';
test('external refresh has bounded watcher IPC separate from Git status collection', () => {
  const commands = source('src/ipc/commands.ts'); const api = source('src/lib/stackPopup.ts'); const native = source('src-tauri/src/stack_popup.rs');
  for (const [name, command] of [['start', 'start_stack_folder_watch'], ['stop', 'stop_stack_folder_watch']]) {
    assert.ok(commands.includes(command)); assert.match(api, new RegExp(`export (?:async )?function ${name}StackFolderWatch\\(`));
    assert.match(native, new RegExp(`pub (?:async )?fn ${command}\\(`));
  }
  assert.match(api, /STACK_FOLDER_CHANGED_EVENT\s*=\s*'stack-popup:folder-changed'/);
  assert.match(api, /STACK_FOLDER_WATCH_ERROR_EVENT\s*=\s*'stack-popup:folder-watch-error'/);
  const surface = source('src/components/StackPopupSurface.svelte');
  assert.match(surface, /\.listen<StackFolderWatchChange>\(STACK_FOLDER_CHANGED_EVENT/);
  assert.match(surface, /\.listen<StackFolderWatchError>\(STACK_FOLDER_WATCH_ERROR_EVENT/);
  assert.match(source('src-tauri/src/contracts.rs'), /stack-popup:folder-changed/);
});
test('native watcher implementation includes the runner-owned real filesystem acceptance', () => {
  const watcher = source('src-tauri/src/stack_popup/folder_watch.rs');
  assert.match(watcher, /ReadDirectoryChangesW/);
  assert.match(watcher, /stack_folder_watch\.rs/);
  assert.doesNotMatch(watcher, /notify::|std::thread::sleep\([^)]*status/);
});
test('successful native hide retires watch and publishes closed to the popup as well as top bar', () => {
  const popup = source('src-tauri/src/stack_popup/popup_window.rs');
  const start = popup.indexOf('pub(crate) fn hide_stack_popup_window');
  const end = popup.indexOf('pub(crate) fn latest_stack_popup_request', start);
  const hide = popup.slice(start, end);
  assert.match(hide, /popup\.emit_to\(\s*tauri::EventTarget::WebviewWindow\s*\{\s*label:\s*popup\.label\(\)\.into\(\),?\s*\},\s*"stack-popup:closed"\s*,\s*\(\)\s*\)/, 'hidden popup must receive its own explicit WebviewWindow-targeted closed event, not only top bar');
  assert.match(hide, /\.emit_to\(TOP_BAR_LABEL,\s*"stack-popup:closed"/, 'top-bar closed publication remains intact');
  assert.ok(hide.indexOf('.hide()') < hide.indexOf('folder_watch::retire'), 'only successful hide retires native watch');
  assert.ok(hide.indexOf('folder_watch::retire') < hide.search(/popup\.emit_to/), 'retire before telling popup to discard its old token');
});
