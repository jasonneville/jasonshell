import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
const read = path => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const native = read('src-tauri/src/task_gallery.rs');
const thumbnails = read('src-tauri/src/task_gallery/thumbnails.rs');
const commands = read('src/ipc/commands.ts');
const contracts = read('src-tauri/src/contracts.rs');
const main = read('src-tauri/src/main.rs');
const shellWindows = read('src-tauri/src/shell_windows.rs').replace(/\r\n/g, '\n');

// Static integration guards supplement executable pure admission/geometry tests;
// they are NOT evidence of live Win32/DWM behavior or fake-resource lifecycle.
test('sync command uses frontend/native inventories and actual registration', () => {
  assert.match(commands, /syncTaskGalleryThumbnails:\s*'sync_task_gallery_thumbnails'/);
  assert.match(contracts, /SYNC_TASK_GALLERY_THUMBNAILS:\s*&str\s*=\s*"sync_task_gallery_thumbnails"/);
  assert.match(main, /task_gallery::sync_task_gallery_thumbnails/);
  // build.rs owns permission generation; no hand-authored ACL requirement here.
});
test('inline native admission checks caller, session/revision/request and captured membership before OS work', () => {
  const command = native.match(/pub async fn sync_task_gallery_thumbnails[\s\S]*?\n\}/)?.[0] ?? '';
  assert.ok(command);
  assert.match(command, /window\.label\(\) != TASK_GALLERY_LABEL/);
  assert.match(command, /admit_request\(runtime\.nonce\.as_deref\(\), runtime\.revision, runtime\.latest_thumbnail_request_id, &args\)/);
  assert.match(command, /validate_frames\(&args/);
  assert.match(command, /snapshot_window_from_runtime\(&runtime, &args\.nonce, &frame\.hwnd\)/);
  assert.match(command, /runtime\.latest_thumbnail_request_id = args\.request_id/);
  assert.match(native, /current != authorized\.identity/);
  assert.match(native, /fn thumbnail_work_is_current[\s\S]*?runtime\.nonce\.as_deref\(\) != Some\(args\.nonce\.as_str\(\)\)[\s\S]*?runtime\.revision != args\.revision[\s\S]*?runtime\.latest_thumbnail_request_id != args\.request_id/);
});
test('DWM registration is destination-owned and each owned handle unregisters on Drop', () => {
  assert.match(thumbnails, /let destination = window\.hwnd\(\)/);
  assert.match(thumbnails, /DwmRegisterThumbnail\(destination, source\)/);
  assert.match(thumbnails, /impl Drop for LiveThumbnail[\s\S]*?DwmUnregisterThumbnail\(self\.handle\)/);
  assert.match(thumbnails, /DwmQueryThumbnailSourceSize\(self\.handle\)/);
  assert.match(thumbnails, /DwmUpdateThumbnailProperties\(self\.handle/);
  assert.match(thumbnails, /bounds\.left \* scale/);
  assert.match(thumbnails, /bounds\.top \* scale/);
});
test('snapshot and sync replace/drop prior live sets and do not publish stale staged resources', () => {
  assert.match(native, /thumbnails: ResourceOwner<ThumbnailGeneration, Resource>/);
  assert.match(native, /runtime\.thumbnails\.clear\(\)/);
  assert.match(native, /runtime\.thumbnails\.take_if_current\(&thumbnail_generation\(args\)\)/);
  assert.match(native, /drop\(old\);[\s\S]*?gallery\.set_size/);
  assert.match(native, /drop\(old\.retain_visible\(args\.frames\.iter\(\)\.map/);
  assert.match(native, /runtime\.thumbnails\.finish\(&thumbnail_generation\(args\), staged\)/);
  assert.match(native, /drop\(completion\.retired\);\s*if !accepted \{ return Err\("Stale task gallery thumbnail work/);
  assert.match(native, /runtime\.thumbnails\.cancel\(\)/);
  assert.match(native, /capture_task_window_preview\(frame\.hwnd\.clone\(\)\)/);
  assert.match(native, /source: "unavailable", image_data_url: None, error: Some\(error\)/);
});
test('native monitor placement converts layout once, clamps above live bottom bar, and preserves nonactivating hover', () => {
  assert.match(native, /bottom_bar\.scale_factor\(\)/);
  assert.match(native, /monitor\.size\(\)\.width as f64 \/ scale_factor/);
  assert.match(native, /layout\.panel_width \* scale_factor/);
  assert.match(native, /layout\.panel_height \* scale_factor/);
  assert.match(native, /\.clamp\(min_x, max_x\.max\(min_x\)\)/);
  assert.match(native, /bottom_position\.y - padding - height as i32/);
  assert.match(native, /ShowWindow\(hwnd, SW_SHOWNOACTIVATE\)/);
  assert.match(native, /if args\.focus_gallery && !same_session/);
});

test('same-session snapshot must not unconditionally clear all committed native registrations', () => {
  // Narrow integration regression, not proof of actual DWM rendering. Executable
  // ResourceOwner probes establish that revision advancement supports reuse.
  const show = native.slice(native.indexOf('let mut allowed_hwnds = BTreeMap::new();'), native.search(/#\[tauri::command\]\s*pub async fn sync_task_gallery_thumbnails/));
  assert.ok(show.length > 0);
  assert.doesNotMatch(show, /\(runtime\.revision, runtime\.thumbnails\.clear\(\)\)/,
    'Unconditional snapshot clear drops every DWM handle even for same-session unchanged geometry/membership');
});

// Tauri's Windows shadow(true) gives undecorated windows a 1px white native
// border. CSS already owns the rounded bevel; native frame must not paint past
// that curve. These builder guards do not certify actual DWM pixels.
for (const [surface, builder] of [['gallery', 'build_task_gallery_window'], ['single preview', 'build_preview_window']]) {
  test(`${surface} builder disables native shadow while retaining transparent undecorated chrome`, () => {
    const body = shellWindows.match(new RegExp(`fn ${builder}\\([^]*?\\n\\}`))?.[0];
    assert.ok(body, `${builder} must exist`);
    assert.match(body, /\.decorations\(false\)/, `${surface} remains undecorated`);
    assert.match(body, /\.transparent\(true\)/, `${surface} retains transparent rounded corners`);
    assert.match(body, /\.shadow\(false\)/, `${surface} must disable the Windows native 1px border/shadow`);
    assert.doesNotMatch(body, /\.shadow\(true\)/, `${surface} must not re-enable native shadow`);
  });
}
