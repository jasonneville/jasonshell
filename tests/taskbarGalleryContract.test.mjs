import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const bottomBarCssSource = readFileSync(new URL('../src/components/BottomBar.css', import.meta.url), 'utf8');
const gallerySource = readFileSync(new URL('../src/components/TaskGallerySurface.svelte', import.meta.url), 'utf8');
const taskPreviewSurfaceSource = readFileSync(new URL('../src/components/TaskPreviewSurface.svelte', import.meta.url), 'utf8');
const nativeGallerySource = readFileSync(new URL('../src-tauri/src/task_gallery.rs', import.meta.url), 'utf8');

function extractCssRuleBody(source, selector) {
  const rule = source.match(new RegExp(`${selector} \\{([\\s\\S]*?)\\n\\}`, 'm'));
  return rule?.[1] ?? '';
}

test('native capsule wiring imports and uses gallery helpers', () => {
  assert.match(bottomBarSource, /taskGroupDisplayMode/);
  assert.match(bottomBarSource, /taskGroupGalleryItems/);
  assert.match(bottomBarSource, /taskbarStripPressureState/);
  assert.match(bottomBarSource, /showTaskGalleryNative/);
  assert.match(bottomBarSource, /hideTaskGalleryNative/);
});

test('capsule renders native gallery affordance only', () => {
  assert.match(bottomBarSource, /class={`task-group \$\{taskGroupDisplayClass\(group\)\}`}/);
  assert.match(bottomBarSource, /class={`task-button task-capsule/);
  assert.match(bottomBarSource, /ariaHaspopup="dialog"/);
  assert.match(bottomBarSource, /ariaExpanded=\{taskGalleryOpenGroupKey === group.key\}/);
  const capsuleButton = bottomBarSource.match(/class={`task-button task-capsule[\s\S]*?<\/MeltActionButton>/)?.[0] ?? '';
  assert.ok(capsuleButton);
  assert.doesNotMatch(capsuleButton, /onContextMenu/);
});

test('capsule outer width follows the shared equal flex contract', () => {
  const capsuleRule = extractCssRuleBody(bottomBarCssSource, '\\.bottom-bar \\.task-group-capsule');
  assert.ok(capsuleRule);
  assert.match(capsuleRule, /flex:\s*1 1 0;/);
  assert.match(capsuleRule, /max-width:\s*160px;/);
  assert.doesNotMatch(capsuleRule, /max-width:\s*calc\(160px \* var\(--task-window-count, 1\)\);/);
  assert.doesNotMatch(capsuleRule, /min-width:\s*96px;|max-width:\s*96px;/);
});

test('capsule hover opens gallery after a cancellable dwell', () => {
  assert.match(bottomBarSource, /function\s+scheduleTaskGalleryOpen\s*\(/);
  assert.match(bottomBarSource, /function\s+cancelTaskGalleryOpen\s*\(/);
  assert.match(bottomBarSource, /function\s+resolveTaskGalleryOpenGroup\s*\(/);
  assert.match(bottomBarSource, /const nextGroup = resolveTaskGalleryOpenGroup\(group\.key\);/);
  const capsuleButton = bottomBarSource.match(/class={`task-button task-capsule[\s\S]*?<\/MeltActionButton>/)?.[0] ?? '';
  assert.match(capsuleButton, /onMouseEnter=\{\(event\) => scheduleTaskGalleryOpen\(group, event\)\}/);
  assert.match(capsuleButton, /onMouseLeave=\{\(\) => scheduleTaskGalleryClose\(group\.key\)\}/);
});

test('gallery lifecycle closes on escape and stale snapshot', () => {
  assert.match(bottomBarSource, /function\s+closeTaskGallery\s*\([\s\S]*?hideTaskGalleryNative/);
  assert.match(bottomBarSource, /hideTaskGalleryNative\(nonce\)/);
  assert.match(nativeGallerySource, /cancelled_nonces[\s\S]*?take_cancelled_nonce\(&mut runtime, &args\.nonce\)/);
  assert.match(gallerySource, /hideTaskGalleryNative\(nonce\)/);
  assert.match(nativeGallerySource, /ShowWindow\(hwnd, SW_HIDE\)/);
  assert.match(gallerySource, /\{#if payload\}[\s\S]*task-gallery-panel[\s\S]*\{\/if\}/);
  assert.match(bottomBarSource, /handleGlobalKeydown[\s\S]*if \(event\.key !== 'Escape'\)/);
  assert.match(bottomBarSource, /taskbar:windows-snapshot[\s\S]*taskGalleryOpenGroupKey[\s\S]*closeTaskGallery\(\)/);
  assert.match(bottomBarSource, /return \(\) => \{[\s\S]*hideTaskGalleryNative\(\)\.catch/);
});

test('direct task buttons keep existing action semantics', () => {
  assert.match(bottomBarSource, /onClick=\{\(event\) => handleTaskWindowClick\(taskWindow, event\)\}/);
  assert.match(bottomBarSource, /onContextMenu=\{\(event\) => void openTaskMenu\(taskWindow, event\)\}/);
  assert.match(bottomBarSource, /onMouseEnter=\{\(event\) => queuePreview\(taskWindow, event\)\}/);
});

test('gallery preview generations use shared allocation and native pass-through', () => {
  assert.match(gallerySource, /allocateTaskPreviewRequestId/);
  assert.doesNotMatch(gallerySource, /let previewSequence\s*=/);
  assert.match(gallerySource, /hideTaskGalleryWindowPreview\(\{ nonce, requestId, hwnd \}\)/);
  assert.match(nativeGallerySource, /request_id: args\.request_id/);
  assert.match(nativeGallerySource, /hide_task_window_preview\(app_handle, state, request_id\)/);
  assert.doesNotMatch(nativeGallerySource, /let request_id = task_preview::next_task_preview_request_id\(&state\)\?/);
  assert.match(gallerySource, /panelElement\?\.focus\(\)/);
  assert.doesNotMatch(gallerySource, /rowButtons\[focusedIndex\]\?\.focus\(\);\s*\n\s*\}\);/);
});

test('gallery blur honors native context-menu focus hold', () => {
  assert.match(gallerySource, /const blurHandler = \(\) => \{ if \(!disposed\) void hideTaskGalleryOnFocusLoss\(\); \}/);
});

test('gallery preview close is nonce-scoped to the exact window', () => {
  assert.match(taskPreviewSurfaceSource, /closePreviewedTaskWindow\(preview\.hwnd, preview\.galleryNonce\)/);
  assert.match(nativeGallerySource, /gallery_nonce: Some\(args\.nonce\)/);
  assert.match(nativeGallerySource, /pub fn close_task_gallery_previewed_window/);
  assert.match(nativeGallerySource, /snapshot_window\(&args\.nonce, &args\.hwnd\)/);
  assert.match(nativeGallerySource, /task_windows::close_task_window_with_identity\(authorized\.row\.hwnd, authorized\.identity\)/);
  assert.match(nativeGallerySource, /TaskGalleryAuthorizedWindow/);
  assert.match(nativeGallerySource, /current != authorized\.identity/);
});

test('closing a gallery preview retains the gallery until snapshot count leaves capsule mode', () => {
  assert.match(taskPreviewSurfaceSource, /const preserveGallery = Boolean\(preview\.galleryNonce\);[\s\S]*?requestPreviewHide\('immediate', preserveGallery\)/);
  assert.match(gallerySource, /if \(event\.payload\.mode === 'immediate'\) \{[\s\S]*?if \(!event\.payload\.preserveGallery\) void closeTaskGallery\(\)/);
  const closeCommand = nativeGallerySource.match(/pub fn close_task_gallery_previewed_window[\s\S]*?\n\}/)?.[0] ?? '';
  assert.ok(closeCommand);
  assert.match(closeCommand, /windows_by_hwnd\.remove\(&args\.hwnd\)/);
  assert.doesNotMatch(closeCommand, /hide_gallery_and_reset/);
  assert.match(bottomBarSource, /!galleryGroup \|\| galleryGroup\.windows\.length < 2 \|\| taskGroupDisplay\(galleryGroup\) !== 'capsule'/);
  assert.match(bottomBarSource, /windows: taskGroupGalleryItems\(galleryGroup\)/);
});

test('gallery menu resolves the clicked window pid natively', () => {
  assert.match(nativeGallerySource, /process_id: Some\(authorized\.identity\.process_id\)/);
  assert.match(nativeGallerySource, /task_windows::task_window_identity\(hwnd\)/);
  assert.doesNotMatch(nativeGallerySource, /process_id:\s*row\.process_id/);
});

test('hover-open gallery does not steal native focus', () => {
  assert.match(bottomBarSource, /openTaskGallery\(nextGroup, anchor, false\)/);
  assert.match(bottomBarSource, /openTaskGallery\(group, event\.currentTarget as HTMLElement \| null, true\)/);
  assert.match(nativeGallerySource, /ShowWindow\(hwnd, SW_SHOWNOACTIVATE\)/);
  assert.match(nativeGallerySource, /show_task_gallery_window\(&gallery, args\.focus_gallery\)/);
  assert.match(nativeGallerySource, /if args\.focus_gallery \{[\s\S]*?gallery\.set_focus\(\)/);
  assert.match(gallerySource, /if \(event\.payload\.focusGallery\) panelElement\?\.focus\(\)/);
});

test('gallery closes after pointer leaves both tabs and task preview', () => {
  assert.match(gallerySource, /function\s+scheduleGalleryHoverClose\s*\(/);
  assert.match(gallerySource, /function\s+cancelGalleryHoverClose\s*\(/);
  assert.match(gallerySource, /TASK_PREVIEW_HOVER_ENTER_EVENT/);
  assert.match(gallerySource, /TASK_PREVIEW_HIDE_REQUEST_EVENT/);
  assert.match(bottomBarSource, /onMouseLeave=\{\(\) => scheduleTaskGalleryClose\(group\.key\)\}/);
  assert.match(gallerySource, /emit(?:<TaskPreviewHoverEnter>)?\(TASK_PREVIEW_HOVER_ENTER_EVENT, \{ source: 'gallery' \}\)/);
  assert.match(gallerySource, /function\s+handleGalleryPointerEnter\s*\([\s\S]*?cancelGalleryHoverClose\(\);[\s\S]*?emit(?:<TaskPreviewHoverEnter>)?\(TASK_PREVIEW_HOVER_ENTER_EVENT, \{ source: 'gallery' \}\)/);
  assert.match(taskPreviewSurfaceSource, /emit(?:<TaskPreviewHoverEnter>)?\(TASK_PREVIEW_HOVER_ENTER_EVENT, \{ source: 'preview' \}\)/);
  assert.match(bottomBarSource, /type TaskPreviewHoverEnter/);
  assert.match(bottomBarSource, /listen<\s*TaskPreviewHoverEnter\s*>\(TASK_PREVIEW_HOVER_ENTER_EVENT, \(\) => \{/s);
  assert.match(gallerySource, /if \(event\.payload\.source === 'preview'\) \{/);
  assert.doesNotMatch(gallerySource, /listen\(TASK_PREVIEW_HOVER_ENTER_EVENT, cancelGalleryHoverClose\)/);
  assert.match(gallerySource, /on:pointerenter=\{handleGalleryPointerEnter\}/);
  assert.match(gallerySource, /on:pointerleave=\{scheduleGalleryHoverClose\}/);
});

test('gallery surface is one horizontal tab strip with exact window labels', () => {
  assert.match(gallerySource, /class="task-gallery-strip"/);
  assert.match(gallerySource, /role="listbox" aria-orientation="horizontal"/);
  assert.match(gallerySource, /role="option"/);
  assert.match(gallerySource, /class:active=\{item\.isActive\}/);
  assert.match(gallerySource, /class:minimized=\{item\.isMinimized\}/);
  assert.match(gallerySource, /title=\{item\.title\}/);
  assert.match(gallerySource, /const parts = \[item\.title, item\.processName\]/);
  assert.doesNotMatch(gallerySource, /HWND \$\{item\.hwnd\}/);
  assert.match(gallerySource, /min-width: 0;/);
  assert.match(gallerySource, /task-gallery-tab-title/);
  assert.doesNotMatch(gallerySource, /task-gallery-search|task-gallery-heading|task-gallery-meta|task-gallery-list/);
});

test('gallery tabs match bottom-bar task button styling', () => {
  assert.match(gallerySource, /background:\s*var\(--js-color-control\)/);
  assert.match(gallerySource, /border:\s*0;/);
  assert.match(gallerySource, /border-left:\s*1px solid var\(--js-color-border-soft\)/);
  assert.match(gallerySource, /border-radius:\s*0;/);
  assert.match(gallerySource, /font-size:\s*0\.62rem;/);
  assert.match(gallerySource, /font-weight:\s*600;/);
  assert.match(gallerySource, /gap:\s*0\.28rem;/);
  assert.match(gallerySource, /padding:\s*0 0\.38rem;/);
  assert.match(nativeGallerySource, /bottom_bar[\s\S]*outer_size\(\)[\s\S]*size\.height/);
});

test('same-session refresh preserves focus and updates native geometry from tab count', () => {
  assert.match(gallerySource, /const next = reconcileTaskGalleryFocus\(sameNonce \? focusedHwnd : null, event\.payload\.windows\);/);
  assert.match(gallerySource, /panelElement\?\.focus\(\)/);
  assert.match(nativeGallerySource, /task_gallery_width_logical\(/);
  assert.match(nativeGallerySource, /TASK_GALLERY_HEIGHT_LOGICAL/);
  assert.match(nativeGallerySource, /let same_session[\s\S]*gallery\.set_position[\s\S]*if same_session/);
  assert.match(bottomBarSource, /refreshExisting: true/);
  assert.match(nativeGallerySource, /if args\.refresh_existing[\s\S]*?nonce\.as_deref\(\) != Some\(args\.nonce\.as_str\(\)\)[\s\S]*?return Ok\(\(\)\)/);
  assert.match(nativeGallerySource, /let same_session[\s\S]*?runtime\.nonce = Some\(args\.nonce\.clone\(\)\)[\s\S]*?runtime\.windows_by_hwnd = allowed_hwnds\.clone\(\)/);
  assert.match(nativeGallerySource, /gallery\.set_position[\s\S]*?nonce\.as_deref\(\) != Some\(args\.nonce\.as_str\(\)\)[\s\S]*?return Ok\(\(\)\)[\s\S]*?show_task_gallery_window/);
});

test('snapshot closes gallery when its group leaves capsule mode', () => {
  assert.match(bottomBarSource, /!galleryGroup \|\| galleryGroup\.windows\.length < 2 \|\| taskGroupDisplay\(galleryGroup\) !== 'capsule'/);
});
