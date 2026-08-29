import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const previewSource = readFileSync(new URL('../src/components/TaskPreviewSurface.svelte', import.meta.url), 'utf8');
const previewCss = readFileSync(new URL('../src/components/TaskPreviewSurface.css', import.meta.url), 'utf8');
const previewWrapper = readFileSync(new URL('../src/lib/taskbarPreview.ts', import.meta.url), 'utf8');
const taskbarWindowsWrapper = readFileSync(new URL('../src/lib/taskbarWindows.ts', import.meta.url), 'utf8');
const ipcCommands = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const taskWindowsRs = readFileSync(new URL('../src-tauri/src/task_windows/mod.rs', import.meta.url), 'utf8');
const taskWindowActionsRs = readFileSync(new URL('../src-tauri/src/task_windows/actions.rs', import.meta.url), 'utf8');
const taskWindowHelperRs = readFileSync(new URL('../src-tauri/src/task_windows/helper.rs', import.meta.url), 'utf8');
const mainRs = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

function functionBody(source, name) {
  const start = source.indexOf(`function ${name}`);
  assert.notEqual(start, -1, `${name} exists`);
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let index = braceStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0) return source.slice(braceStart + 1, index);
  }
  assert.fail(`${name} body closes`);
}

function cssRule(source, selector) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = source.match(new RegExp(`${escapedSelector}\\s*\\{([\\s\\S]*?)\\n\\}`));
  assert.ok(match, `${selector} rule exists`);
  return match[1];
}

test('preview outer root owns full-bounds hover retention handlers', () => {
  assert.match(previewSource, /function handlePreviewPointerEnter\(/);
  assert.match(previewSource, /function handlePreviewPointerLeave\(/);
  assert.match(previewSource, /on:pointerenter=\{handlePreviewPointerEnter\}/);
  assert.match(previewSource, /on:pointerleave=\{\(event\) => void handlePreviewPointerLeave\(event\)\}/);
  assert.doesNotMatch(previewSource, /onMouseEnter=\{\(\) => void emit\(TASK_PREVIEW_HOVER_ENTER_EVENT\)\}/);
  assert.doesNotMatch(previewSource, /hideTaskWindowPreview\(/);
  assert.match(previewSource, /TASK_PREVIEW_HIDE_REQUEST_EVENT/);
});

test('preview pointer leave ignores top-half/internal transitions and hides only outside root', () => {
  const leaveBody = functionBody(previewSource, 'handlePreviewPointerLeave');
  assert.match(leaveBody, /event\.currentTarget/);
  assert.match(leaveBody, /event\.relatedTarget/);
  assert.match(leaveBody, /contains\(relatedTarget\)/);
  assert.match(leaveBody, /return;/);
  assert.match(leaveBody, /await requestPreviewHide\('schedule'\)/);
});

test('scheduled hide keeps preview alive until backend hide event arrives', () => {
  const hideBody = functionBody(previewSource, 'requestPreviewHide');
  assert.match(hideBody, /if \(mode === 'immediate'\)/);
  assert.match(hideBody, /preview = null;/);
  assert.match(hideBody, /await emit\(TASK_PREVIEW_HIDE_REQUEST_EVENT, \{ mode \}\);/);
  assert.doesNotMatch(hideBody, /if \(mode === 'schedule'\)[\s\S]*preview = null;/);
});

test('preview close button is accessible red X and does not activate preview', () => {
  const closeBody = functionBody(previewSource, 'handlePreviewClose');
  const closeButtonRule = cssRule(previewCss, '.preview-close-button');
  assert.match(previewSource, /ariaLabel="Close previewed window"/);
  assert.match(previewSource, /class="preview-close-button"/);
  assert.match(previewSource, />×<|>✕</);
  assert.match(closeBody, /event\.preventDefault\(\)/);
  assert.match(closeBody, /event\.stopPropagation\(\)/);
  assert.match(closeBody, /await closePreviewedTaskWindow\(preview\.hwnd\)/);
  assert.match(closeBody, /await requestPreviewHide\('immediate'\)/);
  assert.match(closeButtonRule, /position:\s*absolute/);
  assert.match(closeButtonRule, /top:/);
  assert.match(closeButtonRule, /right:/);
  assert.match(closeButtonRule, /(red|danger|#dc2626|#ef4444|--js-color-danger)/);
  assert.match(closeButtonRule, /border-radius:\s*(?:0|[234]px|var\(--js-radius-xs\)|var\(--js-radius-sm\))/);
  assert.doesNotMatch(closeButtonRule, /border-radius:\s*999px/);
  assert.match(closeButtonRule, /(?:min-width:\s*(?:2\.[0-9]+|[3-9])rem|padding:\s*0\s+(?:0\.[1-9]|[1-9])\d*rem)/);
});

test('close previewed task window wrapper validates external hwnd and command wiring', () => {
  assert.match(previewWrapper, /export function closePreviewedTaskWindow\(hwnd: string\): Promise<void>/);
  assert.match(previewWrapper, /if \(!hwnd\.trim\(\)\)/);
  assert.match(previewWrapper, /invoke\(IPC_COMMANDS\.closeTaskWindow, \{ hwnd \}\)/);
  assert.match(ipcCommands, /closeTaskWindow: 'close_task_window'/);
  assert.match(taskbarWindowsWrapper, /closeTaskWindow\(hwnd: string\): Promise<void>/);
  assert.match(taskWindowsRs, /pub fn close_task_window\(hwnd: String\) -> Result<\(\), String>/);
  assert.match(taskWindowsRs, /reject_internal_shell_hwnd|is_jasonshell_window/);
  assert.match(mainRs, /task_windows::close_task_window/);
});

test('task window close falls back to terminating the owning process', () => {
  assert.match(taskWindowActionsRs, /SendMessageTimeoutW/);
  assert.match(taskWindowActionsRs, /PostMessageW/);
  assert.match(taskWindowActionsRs, /revalidate_close_target/);
  assert.match(taskWindowActionsRs, /GetWindowThreadProcessId/);
  assert.match(taskWindowActionsRs, /OpenProcess\(PROCESS_TERMINATE, false, process_id\)/);
  assert.match(taskWindowActionsRs, /spawn_task_window_helper/);
  assert.match(taskWindowHelperRs, /WM_CLOSE/);
  assert.match(taskWindowHelperRs, /PostMessageW\(Some\(hwnd\), WM_CLOSE/);
  assert.match(taskWindowHelperRs, /OpenProcess\(\s*PROCESS_TERMINATE \| PROCESS_QUERY_LIMITED_INFORMATION,\s*false,\s*pid,?\s*\)/);
  assert.match(taskWindowHelperRs, /TerminateProcess\(process_handle, 1\)/);
  assert.match(taskWindowHelperRs, /creation_time/);
  assert.match(taskWindowHelperRs, /canonical_image_path/);
  assert.match(taskWindowHelperRs, /IsWindow\(Some\(hwnd\)\)\.as_bool\(\)/);
  assert.match(taskWindowHelperRs, /utf16hex:/);
  assert.match(taskWindowHelperRs, /decode_canonical_path/);
  assert.match(taskWindowHelperRs, /std::process::exit\(exit_code\)/);
  assert.doesNotMatch(taskWindowHelperRs, /SeDebugPrivilege|taskkill|kill_tree|kill-tree/);
  assert.match(taskWindowHelperRs, /--task-window-helper/);
  assert.match(mainRs, /task_windows::handle_task_window_helper_args/);
  assert.match(mainRs, /match launchers::handle_launch_pinned_taskbar_helper_args\(\)/);
  assert.match(mainRs, /Err\(error\) => \{\r?\n\s*eprintln!/);
  assert.doesNotMatch(mainRs, /tauri::Builder::default\(\)[\s\S]*handle_launch_pinned_taskbar_helper_args/);
});
