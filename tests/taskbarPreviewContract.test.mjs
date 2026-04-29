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

const basePayload = {
  hwnd: '1234',
  title: 'Example',
  processName: 'example.exe',
  iconDataUrl: 'data:image/png;base64,icon',
  isMinimized: false
};

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
  assert.match(taskPreviewSurfaceSource, /event\.key !== 'Enter' && event\.key !== ' '/);
});

test('task preview publish path rechecks request freshness before emitting native state', () => {
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
    /if !preview_request_is_current\(&state, request_id\) \{[\s\S]*clear_active_live_thumbnail\(&mut state\);[\s\S]*return Ok\(\(\)\);/
  );
});
