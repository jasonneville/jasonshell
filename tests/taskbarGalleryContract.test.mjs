import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  acknowledgeTaskGalleryTransition,
  shouldDeferTaskGalleryClose,
  shouldCancelTaskGalleryClose
} from '../dist-tests/lib/taskbarUi.js';

const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const bottomBarCssSource = readFileSync(new URL('../src/components/BottomBar.css', import.meta.url), 'utf8');
const gallerySource = readFileSync(new URL('../src/components/TaskGallerySurface.svelte', import.meta.url), 'utf8');
const galleryCssSource = readFileSync(new URL('../src/components/TaskGallerySurface.css', import.meta.url), 'utf8');
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

test('capsule retains native gallery affordance and exposes shared group context menu', () => {
  assert.match(bottomBarSource, /class={`task-group \$\{taskGroupDisplayClass\(group\)\}`}/);
  assert.match(bottomBarSource, /class={`task-button task-capsule/);
  assert.match(bottomBarSource, /ariaHaspopup="dialog"/);
  assert.match(bottomBarSource, /ariaExpanded=\{taskGalleryOpenGroupKey === group.key\}/);
  const capsuleButton = bottomBarSource.match(/class={`task-button task-capsule[\s\S]*?<\/MeltActionButton>/)?.[0] ?? '';
  assert.ok(capsuleButton);
  assert.match(capsuleButton, /onContextMenu=\{\(event\) => (?:void )?openTaskGroupMenu\(group, event\)\}/,
    'capsule right-click must open group menu, not a representative single-window menu');
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

test('rapid capsule click opens focused gallery from captured pointer release without dwell', () => {
  assert.match(bottomBarSource, /pendingTaskGalleryGroupKey = pendingTaskGalleryPointer\([\s\S]*?taskGroupDisplay\(group\) === 'capsule'/);
  assert.match(bottomBarSource, /resolveTaskGalleryPointerRelease\([\s\S]*?pendingTaskGalleryGroupKey,[\s\S]*?taskGroupDragStarted/);
  assert.match(bottomBarSource, /if \(galleryReleaseResult\.openGroupKey\)[\s\S]*?openTaskGallery\(galleryGroup, galleryAnchor, true\)/);
});

test('click-open guard survives native show resolution until nonce-matched gallery entry', () => {
  assert.match(bottomBarSource, /taskGalleryClickTransitionNonce = focusGallery \? nonce : null;[\s\S]*?await showTaskGalleryNative\(/);
  assert.match(bottomBarSource, /function\s+scheduleTaskGalleryClose\s*\(groupKey: string\)[\s\S]*?shouldDeferTaskGalleryClose\(taskGalleryClickTransitionNonce, taskGalleryOpenNonce\)/);
  assert.doesNotMatch(bottomBarSource, /await showTaskGalleryNative\([\s\S]*?finally \{[\s\S]*?taskGalleryClickTransitionNonce === nonce[\s\S]*?taskGalleryClickTransitionNonce = null;/);
  assert.match(gallerySource, /emit(?:<TaskPreviewHoverEnter>)?\(TASK_PREVIEW_HOVER_ENTER_EVENT, \{ source: 'gallery', nonce: payload\.nonce \}\)/);
  assert.match(bottomBarSource, /openTaskGallery\(group, event\.currentTarget as HTMLElement \| null, true\)/);

  const clickNonce = 'click-session';
  let transitionNonce = clickNonce;

  // Native show Promise resolution is intentionally not a lifecycle input.
  assert.equal(shouldDeferTaskGalleryClose(transitionNonce, clickNonce), true);
  transitionNonce = acknowledgeTaskGalleryTransition(transitionNonce, 'stale-session');
  assert.equal(shouldDeferTaskGalleryClose(transitionNonce, clickNonce), true);

  // Actual pointer entry acknowledges only its own session and hands off normal departure closing.
  transitionNonce = acknowledgeTaskGalleryTransition(transitionNonce, clickNonce);
  assert.equal(shouldDeferTaskGalleryClose(transitionNonce, clickNonce), false);
});

test('gallery close cancellation requires current gallery nonce but preserves preview entry', () => {
  const currentNonce = 'current-session';

  assert.equal(
    shouldCancelTaskGalleryClose({ source: 'gallery', nonce: 'stale-session' }, currentNonce),
    false
  );
  assert.equal(
    shouldCancelTaskGalleryClose({ source: 'gallery', nonce: currentNonce }, currentNonce),
    true
  );
  assert.equal(shouldCancelTaskGalleryClose({ source: 'preview' }, currentNonce), true);
  assert.match(
    bottomBarSource,
    /if \(!shouldCancelTaskGalleryClose\(event\.payload, taskGalleryOpenNonce\)\) return;\s*cancelTaskGalleryClose\(\);/
  );
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

test('inline gallery uses guarded sync rather than on-demand separate preview', () => {
  assert.match(gallerySource, /syncTaskGalleryThumbnails/);
  assert.doesNotMatch(gallerySource, /showTaskGalleryWindowPreview\s*\(/);
  assert.match(gallerySource, /revision/);
  assert.match(gallerySource, /requestId/);
  // Legacy native preview routes still must pass caller generations through safely.
  assert.match(nativeGallerySource, /request_id: args\.request_id/);
  assert.match(nativeGallerySource, /hide_task_window_preview\(app_handle, state, request_id\)/);
  assert.doesNotMatch(nativeGallerySource, /let request_id = task_preview::next_task_preview_request_id\(&state\)\?/);
  assert.match(gallerySource, /panelElement\?\.focus\(\)/);
  assert.doesNotMatch(gallerySource, /rowButtons\[focusedIndex\]\?\.focus\(\);\s*\n\s*\}\);/);
});

test('gallery blur honors native context-menu focus hold', () => {
  assert.match(gallerySource, /const blurHandler = \(\) => \{[\s\S]*?if \(disposed\) return;[\s\S]*?clearNativeFrames\(\);[\s\S]*?hideTaskGalleryOnFocusLoss\(\)/);
});

test('gallery preview close is nonce-scoped to the exact window', () => {
  assert.match(taskPreviewSurfaceSource, /closePreviewedTaskWindow\(preview\.hwnd, preview\.galleryNonce\)/);
  assert.match(nativeGallerySource, /gallery_nonce: Some\(args\.nonce\)/);
  assert.match(nativeGallerySource, /pub (?:async )?fn close_task_gallery_previewed_window/);
  assert.match(nativeGallerySource, /snapshot_window\(&args\.nonce, &args\.hwnd\)/);
  assert.match(nativeGallerySource, /task_windows::close_task_window_with_identity\(authorized\.row\.hwnd, authorized\.identity\)/);
  assert.match(nativeGallerySource, /TaskGalleryAuthorizedWindow/);
  assert.match(nativeGallerySource, /current != authorized\.identity/);
});

test('closing a gallery preview retains the gallery until snapshot count leaves capsule mode', () => {
  assert.match(taskPreviewSurfaceSource, /const preserveGallery = Boolean\(preview\.galleryNonce\);[\s\S]*?requestPreviewHide\('immediate', preserveGallery\)/);
  assert.match(gallerySource, /if \(event\.payload\.mode === 'immediate'\) \{[\s\S]*?if \(!event\.payload\.preserveGallery\) void closeTaskGallery\(\)/);
  const closeCommand = nativeGallerySource.match(/pub (?:async )?fn close_task_gallery_previewed_window[\s\S]*?\n\}/)?.[0] ?? '';
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
  assert.match(nativeGallerySource, /if args\.focus_gallery && !same_session \{[\s\S]*?gallery\.set_focus\(\)/);
  assert.match(gallerySource, /if \(!sameNonce && incoming\.focusGallery\) panelElement\?\.focus\(\)/);
});

test('inline gallery keeps pointer-crossing lifetime without separate preview dependency', () => {
  assert.match(gallerySource, /function\s+scheduleGalleryHoverClose\s*\(/);
  assert.match(gallerySource, /function\s+cancelGalleryHoverClose\s*\(/);
  assert.match(gallerySource, /TASK_PREVIEW_HOVER_ENTER_EVENT/);
  assert.match(gallerySource, /TASK_PREVIEW_HIDE_REQUEST_EVENT/);
  assert.match(bottomBarSource, /onMouseLeave=\{\(\) => scheduleTaskGalleryClose\(group\.key\)\}/);
  assert.match(gallerySource, /emit(?:<TaskPreviewHoverEnter>)?\(TASK_PREVIEW_HOVER_ENTER_EVENT, \{ source: 'gallery', nonce: payload\.nonce \}\)/);
  assert.match(gallerySource, /function\s+handleGalleryPointerEnter\s*\([\s\S]*?cancelGalleryHoverClose\(\);[\s\S]*?emit(?:<TaskPreviewHoverEnter>)?\(TASK_PREVIEW_HOVER_ENTER_EVENT, \{ source: 'gallery', nonce: payload\.nonce \}\)/);
  assert.match(taskPreviewSurfaceSource, /emit(?:<TaskPreviewHoverEnter>)?\(TASK_PREVIEW_HOVER_ENTER_EVENT, \{ source: 'preview' \}\)/);
  assert.match(bottomBarSource, /type TaskPreviewHoverEnter/);
  assert.match(bottomBarSource, /listen<\s*TaskPreviewHoverEnter\s*>\(TASK_PREVIEW_HOVER_ENTER_EVENT, \(event\) => \{/s);
  assert.doesNotMatch(gallerySource, /showTaskGalleryWindowPreview\s*\(/);
  assert.doesNotMatch(gallerySource, /listen\(TASK_PREVIEW_HOVER_ENTER_EVENT, cancelGalleryHoverClose\)/);
  assert.match(gallerySource, /on:pointerenter=\{handleGalleryPointerEnter\}/);
  assert.match(gallerySource, /on:pointerleave=\{scheduleGalleryHoverClose\}/);
});

test('image-first gallery retains exact labels and sibling action controls', () => {
  assert.match(gallerySource, /role="group"[\s\S]*?aria-label=\{taskGalleryTabLabel\(item\)\}/);
  assert.match(gallerySource, /class="task-gallery-activate"/);
  assert.match(gallerySource, /class="task-gallery-close"/);
  assert.match(gallerySource, /aria-label=\{`Close \$\{item\.title\}`\}/);
  assert.match(gallerySource, /class:active=\{item\.isActive\}/);
  assert.match(gallerySource, /class:minimized=\{item\.isMinimized\}/);
  assert.match(gallerySource, /title=\{item\.title\}/);
  assert.match(gallerySource, /const parts = \[item\.title, item\.processName\]/);
  assert.doesNotMatch(gallerySource, /HWND \$\{item\.hwnd\}/);
  assert.match(galleryCssSource, /min-width: 0;/);
  assert.match(gallerySource, /task-gallery-tab-title/);
  assert.match(gallerySource, /thumbnail/i);
  assert.match(gallerySource, /layout/);
});

test('gallery uses shared theme control and text tokens, not fixed palette or bar-height targets', () => {
  assert.match(galleryCssSource, /background:\s*var\(--js-color-control\)/);
  assert.match(galleryCssSource, /border:\s*0;/, 'activation and close controls keep their independent borderless layout');
  assert.match(galleryCssSource, /var\(--js-color-border/);
  assert.match(galleryCssSource, /var\(--js-color-text/);
  // No motion is also valid reduced-motion behavior; browser acceptance measures it.
  for (const declaration of galleryCssSource.matchAll(/(?:animation|transition)\s*:\s*([^;]+);/g)) {
    assert.equal(declaration[1].trim(), 'none', 'gallery motion declarations must explicitly disable animation/transition');
  }
  assert.doesNotMatch(galleryCssSource, /height:\s*min\(24px,\s*100%\)/,
    'gallery Close targets must not shrink with the bottom bar');
  assert.match(nativeGallerySource, /bottom_bar\.outer_position\(\)/);
  assert.match(nativeGallerySource, /bottom_position\.y - padding - height as i32/);
});

test('same-session refresh preserves focus and updates native authoritative geometry', () => {
  assert.match(gallerySource, /const next = reconcileTaskGalleryFocus\(sameNonce \? focusedHwnd : null, incoming\.windows\);/);
  assert.match(gallerySource, /panelElement\?\.focus\(\)/);
  assert.match(nativeGallerySource, /task_gallery_layout\(/);
  assert.match(nativeGallerySource, /revision/);
  assert.match(nativeGallerySource, /let same_session[\s\S]*gallery\.set_position[\s\S]*if !same_session/);
  assert.match(bottomBarSource, /refreshExisting: true/);
  assert.match(nativeGallerySource, /args\.refresh_existing && \(runtime\.nonce\.as_deref\(\) != Some\(args\.nonce\.as_str\(\)\) \|\| runtime\.revision != expected_revision\)[\s\S]*?return Ok\(None\)/);
  assert.match(nativeGallerySource, /let same_session[\s\S]*?runtime\.nonce = Some\(args\.nonce\.clone\(\)\)[\s\S]*?runtime\.windows_by_hwnd = allowed_hwnds/);
  assert.match(nativeGallerySource, /gallery\.set_position[\s\S]*?validate_nonce\(&args\.nonce\)\?;[\s\S]*?show_task_gallery_window/);
});

test('snapshot closes gallery when its group leaves capsule mode', () => {
  assert.match(bottomBarSource, /!galleryGroup \|\| galleryGroup\.windows\.length < 2 \|\| taskGroupDisplay\(galleryGroup\) !== 'capsule'/);
});
