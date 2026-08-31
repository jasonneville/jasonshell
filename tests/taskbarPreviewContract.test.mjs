import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  isNativeLiveTaskPreviewPayload,
  TASK_PREVIEW_SOURCES
} from '../dist-tests/lib/taskbarPreview.js';

const taskbarPreviewSource = readFileSync(new URL('../src/lib/taskbarPreview.ts', import.meta.url), 'utf8');
const taskPreviewSurfaceSource = readFileSync(
  new URL('../src/components/TaskPreviewSurface.svelte', import.meta.url),
  'utf8'
);
const taskPreviewCssSource = readFileSync(
  new URL('../src/components/TaskPreviewSurface.css', import.meta.url),
  'utf8'
);
const shellWindowsSource = readFileSync(new URL('../src-tauri/src/shell_windows.rs', import.meta.url), 'utf8');
const taskPreviewRustSource = readFileSync(new URL('../src-tauri/src/task_preview.rs', import.meta.url), 'utf8');
const taskWindowsRustSource = readFileSync(new URL('../src-tauri/src/task_windows/mod.rs', import.meta.url), 'utf8');
const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');

const basePayload = {
  hwnd: '1234',
  title: 'Example',
  processName: 'example.exe',
  iconDataUrl: 'data:image/png;base64,icon',
  isMinimized: false
};

test('regular task previews allocate freshness ids from shared native state', () => {
  assert.match(taskbarPreviewSource, /allocateTaskPreviewRequestId/);
  assert.match(bottomBarSource, /allocateTaskPreviewRequestId/);
  assert.match(taskPreviewRustSource, /pub fn allocate_task_preview_request_id/);
  assert.match(taskPreviewRustSource, /allocated_request_id/);
});

function extractRustFunction(source, functionName) {
  const signatureStart = source.indexOf(`fn ${functionName}(`);
  assert.notEqual(signatureStart, -1, `${functionName} function should exist`);
  const bodyStart = source.indexOf('{', signatureStart);
  assert.notEqual(bodyStart, -1, `${functionName} should have a body`);
  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') {
      depth += 1;
    } else if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        return source.slice(signatureStart, index + 1);
      }
    }
  }
  assert.fail(`${functionName} body should close`);
}

test('task preview payload contract exposes native live thumbnail flag and source', () => {
  assert.deepEqual(TASK_PREVIEW_SOURCES, {
    capturedImage: 'captured-image',
    nativeDwmThumbnail: 'native-dwm-thumbnail',
    unavailable: 'unavailable'
  });

  assert.match(taskbarPreviewSource, /previewSource\?: TaskPreviewSource \| null/);
  assert.match(taskbarPreviewSource, /nativeLiveThumbnailActive\?: boolean \| null/);
  assert.equal(
    isNativeLiveTaskPreviewPayload({
      ...basePayload,
      previewSource: TASK_PREVIEW_SOURCES.nativeDwmThumbnail,
      nativeLiveThumbnailActive: false
    }),
    true
  );
  assert.equal(
    isNativeLiveTaskPreviewPayload({
      ...basePayload,
      previewSource: TASK_PREVIEW_SOURCES.capturedImage,
      nativeLiveThumbnailActive: true
    }),
    true
  );
  assert.equal(
    isNativeLiveTaskPreviewPayload({
      ...basePayload,
      previewSource: TASK_PREVIEW_SOURCES.capturedImage,
      imageDataUrl: 'data:image/png;base64,capture'
    }),
    false
  );
});

test('task preview surface gives native DWM thumbnails an unobstructed frame', () => {
  assert.match(taskPreviewSurfaceSource, /isNativeLiveTaskPreviewPayload/);
  assert.match(taskPreviewSurfaceSource, /TASK_PREVIEW_HIDE_REQUEST_EVENT/);
  assert.match(taskPreviewSurfaceSource, /requestPreviewHide\('schedule'\)/);
  assert.match(taskPreviewSurfaceSource, /requestPreviewHide\('immediate'\)/);
  assert.match(
    taskPreviewSurfaceSource,
    /previewSurfaceClass = `surface preview-surface\$\{isNativeLivePreview \? ' preview-surface-native' : ''\}`/
  );
  assert.match(taskPreviewSurfaceSource, /class=\{previewSurfaceClass\}/);
  assert.match(
    taskPreviewSurfaceSource,
    /\{#if isNativeLivePreview\}[\s\S]*class="preview-frame preview-frame-native"[\s\S]*\{:else if preview\.imageDataUrl\}/
  );
  assert.doesNotMatch(
    taskPreviewSurfaceSource,
    /\{#if preview\.imageDataUrl\}[\s\S]*\{:else if isNativeLivePreview\}/
  );
  assert.match(taskPreviewSurfaceSource, /<div class="preview-frame preview-frame-native" aria-hidden="true"><\/div>/);
  assert.match(taskPreviewCssSource, /\.preview-surface-native\s*\{[\s\S]*background:\s*transparent/);
  assert.match(taskPreviewCssSource, /\.preview-surface-native \.preview-header\s*\{[\s\S]*background:\s*var\(--js-bg-surface\)/);
  assert.match(shellWindowsSource, /TASK_PREVIEW_LABEL[\s\S]*\.transparent\(true\)[\s\S]*\.visible\(false\)/);
  assert.match(taskPreviewSurfaceSource, /await maximizeTaskWindow\(preview\.hwnd\)/);
  assert.match(taskbarPreviewSource, /galleryNonce\?: string \| null/);
  assert.match(taskPreviewSurfaceSource, /closePreviewedTaskWindow\(preview\.hwnd, preview\.galleryNonce\)/);
  assert.match(taskPreviewSurfaceSource, /event\.key !== 'Enter' && event\.key !== ' '/);
});

test('task preview publish path rechecks request freshness before emitting native state', () => {
  assert.match(taskPreviewRustSource, /show_task_window_preview_with_host/);
  assert.match(taskPreviewRustSource, /preview_position_from_host/);
  assert.match(
    taskPreviewRustSource,
    /publish_and_show_preview\([\s\S]*&preview_window,[\s\S]*payload,[\s\S]*preview_x,[\s\S]*preview_y,[\s\S]*&state,[\s\S]*request_id,[\s\S]*\)/
  );
  assert.match(
    taskPreviewRustSource,
    /fn publish_and_show_preview\([\s\S]*state: &tauri::State<'_, Mutex<TaskPreviewRuntimeState>>,[\s\S]*request_id: u64/
  );
  assert.match(
    taskPreviewRustSource,
    /fn ensure_preview_request_is_current\([\s\S]*preview_request_is_current\(&state, request_id\)/
  );
  assert.match(
    taskPreviewRustSource,
    /fn clear_active_live_thumbnail_if_current_locked\([\s\S]*if preview_request_is_current\(state, request_id\) \{[\s\S]*clear_active_live_thumbnail\(state\);[\s\S]*}/
  );
});

test('task preview publish path does not hold runtime mutex across Tauri window operations', () => {
  assert.match(taskPreviewRustSource, /fn ensure_preview_request_is_current\(/);
  assert.match(taskPreviewRustSource, /fn clear_active_live_thumbnail_if_current_locked\(/);

  const publishFunction = extractRustFunction(taskPreviewRustSource, 'publish_and_show_preview');

  for (const windowCall of ['emit', 'set_position', 'show']) {
    assert.doesNotMatch(
      publishFunction,
      new RegExp(`let mut state = state[\\s\\S]*?preview_window\\s*\\.\\s*${windowCall}\\s*\\(`),
      `publish_and_show_preview must drop runtime state lock before preview_window.${windowCall}()`
    );
  }

  assert.match(
    publishFunction,
    /if !ensure_preview_request_is_current\(state, request_id\)\? \{[\s\S]*?return Ok\(\(\)\);[\s\S]*?\}\s*if let Err\(error\) = preview_window\s*\.\s*emit\(/,
    'freshness should be checked immediately before emit'
  );
  assert.match(
    publishFunction,
    /if !ensure_preview_request_is_current\(state, request_id\)\? \{[\s\S]*?return Ok\(\(\)\);[\s\S]*?\}\s*if let Err\(error\) = preview_window\s*\.\s*set_position\(/,
    'freshness should be checked immediately before set_position'
  );
  assert.match(
    publishFunction,
    /if !ensure_preview_request_is_current\(state, request_id\)\? \{[\s\S]*?return Ok\(\(\)\);[\s\S]*?\}\s*if let Err\(error\) = preview_window\s*\.\s*show\(/,
    'freshness should be checked immediately before show'
  );

  const hideFunction = extractRustFunction(taskPreviewRustSource, 'hide_task_window_preview');
  assert.match(
    hideFunction,
    /let mut state = state[\s\S]*?begin_task_preview_hide\(&mut state, request_id\)[\s\S]*?\}\s*;\s*if !should_hide[\s\S]*?let preview_window = app_handle/,
    'hide_task_window_preview should close the runtime state lock scope before reading/using the preview window'
  );
  assert.match(hideFunction, /if !ensure_preview_request_is_current\(&state, request_id\)\? \{[\s\S]*?return Ok\(\(\)\);[\s\S]*?\}\s*preview_window\s*\.\s*emit\(/);
  assert.match(hideFunction, /if !ensure_preview_request_is_current\(&state, request_id\)\? \{[\s\S]*?return Ok\(\(\)\);[\s\S]*?\}\s*preview_window\s*\.\s*hide\(/);
  assert.match(taskPreviewRustSource, /fn begin_task_preview_hide\([\s\S]*?clear_active_live_thumbnail\(state\);/);
});

test('task window close path skips preview validator while preview capture still uses it', () => {
  const closeFunction = extractRustFunction(taskWindowsRustSource, 'close_task_window');
  assert.doesNotMatch(closeFunction, /validate_task_window_preview_source/);
  assert.match(taskPreviewRustSource, /validate_task_window_preview_source\(source_hwnd\)\?/);
});
