import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

// Source guards are NOT evidence of actual Windows dispatch or caller rejection.
const source = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const gallery = source('src-tauri/src/task_gallery.rs');
const actions = source('src-tauri/src/task_windows/actions.rs');
const facade = source('src-tauri/src/task_windows/mod.rs');
const wrapper = source('src/lib/taskGallery.ts');
const commands = source('src/ipc/commands.ts');
const main = source('src-tauri/src/main.rs');
const surface = source('src/components/TaskGallerySurface.svelte');

function functionBody(text, name) {
  const start = text.search(new RegExp(`(?:pub(?:\\([^)]*\\))?\\s+)?(?:async\\s+)?fn\\s+${name}\\s*\\(`));
  assert.notEqual(start, -1, `missing native acceptance seam: ${name}`);
  const brace = text.indexOf('{', start);
  let depth = 1;
  let end = brace + 1;
  for (; depth && end < text.length; end++) {
    if (text[end] === '{') depth++;
    else if (text[end] === '}') depth--;
  }
  return text.slice(start, end);
}

test('gallery tile close has a dedicated nonce/HWND IPC route, registered separately from preview close', () => {
  assert.match(wrapper, /export function closeTaskGalleryWindow\(hwnd: string, nonce: string\): Promise<void>/);
  assert.match(wrapper, /invoke\(IPC_COMMANDS\.closeTaskGalleryWindow, \{ args: \{ hwnd, nonce \} \}\)/);
  assert.match(commands, /closeTaskGalleryWindow:\s*['"]close_task_gallery_window['"]/);
  assert.match(main, /task_gallery::close_task_gallery_window\s*,/);
});

test('gallery tile command authorizes gallery caller and session target before request-only dispatch', () => {
  const body = functionBody(gallery, 'close_task_gallery_window');
  const caller = body.indexOf('window.label()');
  const snapshot = body.indexOf('snapshot_window(&args.nonce, &args.hwnd)');
  const dispatch = body.indexOf('request_close_task_window_with_identity');
  assert.ok(caller >= 0 && snapshot > caller && dispatch > snapshot);
  assert.match(body, /request_gallery_close_with/);
  assert.match(body, /request_close_task_window_with_identity\(authorized\.row\.hwnd, authorized\.identity\)/);
  assert.doesNotMatch(body, /windows_by_hwnd\.remove|hide_gallery_and_reset|\bclose_task_window_with_identity\(/);
});

test('request-only primitive validates shell/exact identity before one fallible WM_CLOSE, no force or elevation', () => {
  const body = functionBody(actions, 'request_close_task_window_with_identity');
  const validate = body.indexOf('revalidate_close_target');
  const dispatch = body.search(/PostMessageW|SendMessageTimeoutW/);
  assert.ok(validate >= 0 && dispatch > validate, 'reuse immutable identity revalidation before dispatch');
  assert.equal((body.match(/PostMessageW|SendMessageTimeoutW/g) ?? []).length, 1);
  assert.match(body, /WM_CLOSE/);
  assert.match(body.slice(dispatch), /\?|map_err|Err\(/, 'dispatch failure must be surfaced, not only validation failure');
  assert.doesNotMatch(body, /terminate_window_process|TerminateProcess|PROCESS_TERMINATE|elevate_close_target|close_window_with_identity|wait_for_window_close|taskkill|SetForegroundWindow/);
  assert.match(facade, /request_close_task_window_with_identity/);
  const facadeBody = functionBody(facade, 'request_close_task_window_with_identity');
  assert.match(facadeBody, /reject_internal_shell_hwnd\(&hwnd\)\?/);
  assert.ok(facadeBody.indexOf('reject_internal_shell_hwnd') < facadeBody.indexOf('actions::request_close_task_window_with_identity'));
});

test('preview X keeps legacy close policy and existing authorization removal semantics', () => {
  const body = functionBody(gallery, 'close_task_gallery_previewed_window');
  assert.match(body, /TASK_PREVIEW_LABEL/);
  assert.match(body, /close_task_window_with_identity\(authorized\.row\.hwnd, authorized\.identity\)/);
  assert.match(body, /windows_by_hwnd\.remove\(&args\.hwnd\)/);
});

test('tile X permanently reserves a 24px slot and reveals on hover and keyboard focus', () => {
  assert.match(surface, /(?:width|min-width|flex-basis|grid-template-columns|flex):[^;\n]*24px/);
  assert.match(surface, /:hover[^{]*[\s\S]*?(?:opacity|visibility)/);
  assert.match(surface, /:focus-within[^{]*[\s\S]*?(?:opacity|visibility)/);
  assert.doesNotMatch(surface, /\.task-gallery[^{}]*close[^{}]*\{[^}]*display:\s*none/s);
});
