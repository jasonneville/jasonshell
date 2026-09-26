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
const bottomBarCssSource = readFileSync(new URL('../src/components/BottomBar.css', import.meta.url), 'utf8');
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

test('task preview native window has no rectangular shadow behind rounded surface', () => {
  const previewBuilder = extractRustFunction(shellWindowsSource, 'build_preview_window');
  assert.match(previewBuilder, /WebviewWindowBuilder::new\(\s*app,\s*TASK_PREVIEW_LABEL,/);
  assert.match(previewBuilder, /\.transparent\(true\)/);
  assert.match(previewBuilder, /\.decorations\(false\)/);
  assert.match(previewBuilder, /\.shadow\(false\)/);
  assert.match(taskPreviewCssSource, /\.preview-surface\s*\{[^}]*border-radius:\s*var\(--js-radius-sm\);[^}]*overflow:\s*hidden;/);
});

test('taskbar preview connector keeps only the source tile seamless', () => {
  const queuePreview = bottomBarSource.match(/function queuePreview\([\s\S]*?(?=\n  async function refreshTaskbarWindows\()/)?.[0];
  const hidePreview = bottomBarSource.match(/async function hidePreview\([\s\S]*?(?=\n  function schedulePreviewHide\()/)?.[0];
  assert.ok(queuePreview, 'queuePreview should exist');
  assert.ok(hidePreview, 'hidePreview should exist');
  const barMarkup = bottomBarSource.slice(bottomBarSource.indexOf('<div class="surface bottom-bar"'));
  const barRule = bottomBarCssSource.match(/\.bottom-bar\.surface\s*\{([^}]*)\}/)?.[1];

  assert.match(bottomBarSource, /let\s+bottomBarEl\s*:/, 'bind the bar root for tile-relative coordinates');
  assert.match(barMarkup, /class="surface bottom-bar"[^>]*bind:this=\{bottomBarEl\}/);
  assert.match(queuePreview, /button\.getBoundingClientRect\(\)/);
  assert.match(queuePreview, /bottomBarEl\.getBoundingClientRect\(\)/);
  assert.match(queuePreview, /getComputedStyle\(button\)\.backgroundColor/, 'mask uses source tile color, including active tiles');
  assert.match(
    queuePreview,
    /await showTaskWindowPreview\([\s\S]*?\);[\s\S]*?previewConnector\s*=/,
    'only a successfully shown preview receives a connector'
  );
  assert.match(queuePreview, /left:\s*(?:rect|buttonRect)\.left\s*-\s*(?:barRect|bottomBarRect)\.left/);
  assert.match(queuePreview, /width:\s*(?:rect|buttonRect)\.width/);
  assert.match(hidePreview, /previewConnector\s*=\s*null/, 'hiding removes the connector');
  assert.match(barMarkup, /--preview-connector-left:/);
  assert.match(barMarkup, /--preview-connector-width:/);
  assert.match(barMarkup, /--preview-connector-color:/);
  assert.match(bottomBarCssSource, /\.bottom-bar\.surface::(?:before|after)\s*\{[^}]*pointer-events:\s*none;[^}]*\}/);
  assert.match(bottomBarCssSource, /--preview-connector-left/);
  assert.match(bottomBarCssSource, /--preview-connector-width/);
  assert.match(bottomBarCssSource, /--preview-connector-color/);
  assert.match(bottomBarCssSource, /(?:height:\s*1px|border-top:\s*1px)/);
  assert.match(barRule ?? '', /border-top:\s*1px solid var\(--js-color-border-soft\)/);
  assert.doesNotMatch(barRule ?? '', /border-top:\s*(?:0|none)|border:\s*(?:0|none)/);
});

test('stale task preview hide cannot clear a newer tile connector', () => {
  const queuePreview = bottomBarSource.match(/function queuePreview\([\s\S]*?(?=\n  async function refreshTaskbarWindows\()/)?.[0];
  const hidePreview = bottomBarSource.match(/async function hidePreview\([\s\S]*?(?=\n  function schedulePreviewHide\()/)?.[0];
  assert.ok(queuePreview);
  assert.ok(hidePreview);
  assert.match(bottomBarSource, /let previewGeneration\s*=\s*0/);
  assert.match(queuePreview, /await showTaskWindowPreview\([\s\S]*?\);[\s\S]*?previewConnector\s*=/);
  assert.match(queuePreview, /(?:\+\+previewGeneration|previewGeneration\s*\+=\s*1)/, 'new preview must advance connector generation');
  assert.match(
    hidePreview,
    /const hideGeneration\s*=\s*previewGeneration\s*;[\s\S]*?await allocateTaskPreviewRequestId\(\)[\s\S]*?await hideTaskWindowPreview\(requestId\)[\s\S]*?if\s*\(hideGeneration\s*===\s*previewGeneration\)\s*\{\s*previewConnector\s*=\s*null\s*;/,
    'hide must retain the seam until its native hide completes, then clear only its own generation'
  );
});

test('task preview source tile removes its own top edge', () => {
  const directTile = bottomBarSource.match(/\{#each group\.windows as taskWindow \(taskWindow\.hwnd\)\}([\s\S]*?)\{\/each\}/)?.[1];
  assert.ok(directTile, 'source task tile markup exists');
  assert.match(directTile, /class=\{`task-button[^`]*task-button-preview-connected[^`]*`\}/,
    'direct task tile receives a dedicated preview-connected class');
  assert.match(directTile, /previewConnector\?\.hwnd\s*===\s*taskWindow\.hwnd|previewConnector\s*&&\s*previewConnector\.hwnd\s*===\s*taskWindow\.hwnd/,
    'only the task that owns the shown preview gets the connected class');
  assert.match(bottomBarSource, /await showTaskWindowPreview\([\s\S]*?previewConnector\s*=\s*\{[^}]*hwnd:\s*taskWindow\.hwnd/,
    'connector identity is stored after native preview show');

  const ordinary = bottomBarCssSource.match(/\.bottom-bar \.task-button\s*\{([^}]*)\}/)?.[1];
  const active = bottomBarCssSource.match(/\.bottom-bar \.task-button-active\s*\{([^}]*)\}/)?.[1];
  const connected = bottomBarCssSource.match(/\.bottom-bar \.task-button\.task-button-preview-connected\s*\{([^}]*)\}/)?.[1];
  const connectedActive = bottomBarCssSource.match(/\.bottom-bar \.task-button(?:\.task-button-active)?\.task-button-preview-connected\.task-button-active\s*\{([^}]*)\}|\.bottom-bar \.task-button\.task-button-active\.task-button-preview-connected\s*\{([^}]*)\}/)?.slice(1).find(Boolean);
  assert.match(ordinary ?? '', /box-shadow:\s*var\(--js-inset-highlight\)/, 'ordinary tiles retain top highlight');
  assert.match(active ?? '', /box-shadow:[^;]*var\(--js-inset-highlight\)[^;]*var\(--js-color-accent-soft\)/,
    'ordinary active tiles retain highlight and accent');
  assert.ok(connected, 'connected tile overrides its own top inset highlight');
  assert.match(connected, /box-shadow:\s*[^;]+;/);
  assert.doesNotMatch(connected, /var\(--js-inset-highlight\)/);
  assert.match(connectedActive ?? '', /box-shadow:[^;]*var\(--js-color-accent-soft\)/,
    'connected active tile retains accent outline');
  assert.doesNotMatch(connectedActive ?? '', /var\(--js-inset-highlight\)/);
});

test('preview-connected task tile preserves attention styling', () => {
  const attention = bottomBarCssSource.match(/\.bottom-bar \.task-button\.task-window-attention\s*\{([^}]*)\}/)?.[1];
  const connectedAttention = bottomBarCssSource.match(/\.bottom-bar \.task-button\.task-window-attention\.task-button-preview-connected\s*\{([^}]*)\}/)?.[1];
  assert.match(attention ?? '', /box-shadow:\s*inset 0 4px 0 #ffd54f/);
  assert.match(connectedAttention ?? '', /box-shadow:[^;]*#ffd54f/, 'connected attention tile keeps warning highlight');
  assert.doesNotMatch(connectedAttention ?? '', /var\(--js-inset-highlight\)/, 'only top inset highlight is removed');
  const connectedActiveAttention = bottomBarCssSource.match(/\.bottom-bar \.task-button\.task-button-active\.task-window-attention\.task-button-preview-connected\s*\{([^}]*)\}/)?.[1];
  assert.match(connectedActiveAttention ?? '', /box-shadow:[^;]*#ffd54f[^;]*var\(--js-color-accent-soft\)|box-shadow:[^;]*var\(--js-color-accent-soft\)[^;]*#ffd54f/,
    'active connected attention tile keeps both warning and accent');
  assert.doesNotMatch(connectedActiveAttention ?? '', /var\(--js-inset-highlight\)/);
});

test('stale preview show cannot leave an unconnected native preview', () => {
  const queuePreview = bottomBarSource.match(/function queuePreview\([\s\S]*?(?=\n  async function refreshTaskbarWindows\()/)?.[0];
  assert.ok(queuePreview);
  const afterShow = queuePreview.split(/await showTaskWindowPreview\(\{[\s\S]*?\}\);/)[1];
  assert.ok(afterShow, 'inspect continuation after native show resolves');
  assert.match(afterShow,
    /if\s*\(generation\s*!==\s*previewGeneration\)\s*\{[\s\S]*?(?:await\s+(?:hideTaskWindowPreview|hidePreview)\(|previewConnector\s*=\s*\{)/,
    'stale native show must explicitly clean up or establish connected ownership');
  assert.match(afterShow, /if\s*\(generation\s*===\s*previewGeneration\s*&&\s*bottomBarEl\s*&&\s*button\.isConnected\)/,
    'current preview retains its existing generation and connected-button guard');
});

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
