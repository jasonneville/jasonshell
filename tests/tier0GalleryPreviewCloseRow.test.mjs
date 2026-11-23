import assert from 'node:assert/strict';
import test from 'node:test';
import { functionBody, productionPart, readRust } from './support/rustSource.mjs';

// Request-only close: the window may survive (Save prompt). The gallery must
// not drop its row/thumbnail optimistically; the next refresh reconciles.
const gallery = productionPart(readRust('src-tauri/src/task_gallery.rs'));

test('close_task_gallery_previewed_window keeps the row and thumbnail after a close request', () => {
  const body = functionBody(gallery, 'close_task_gallery_previewed_window');
  assert.ok(body, 'close_task_gallery_previewed_window body not found');
  assert.match(body, /close_task_window_with_identity\(|request_close_task_window_with_identity\(/);
  assert.ok(!/windows_by_hwnd\.remove\(/.test(body), 'must not remove the row after a request-only close');
  assert.ok(!/thumbnails\.remove\(/.test(body), 'must not drop the thumbnail after a request-only close');
});
