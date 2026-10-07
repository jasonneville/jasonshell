import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

// Source safety contracts, not live Win32 evidence. No commands are executed.
const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const menu = read('src-tauri/src/taskbar_menu.rs');
const windows = read('src-tauri/src/task_windows/mod.rs');
const actions = read('src-tauri/src/task_windows/actions.rs');
function functionBody(source, name) {
  const signature = new RegExp(`\\bfn\\s+${name}\\s*\\(`).exec(source);
  assert.ok(signature, `missing native function ${name}`);
  const start = source.indexOf('{', signature.index);
  let depth = 1;
  for (let i = start + 1; i < source.length; i++) {
    if (source[i] === '{') depth++;
    if (source[i] === '}' && --depth === 0) return source.slice(start + 1, i);
  }
  assert.fail(`unterminated native function ${name}`);
}

test('request-close retains bottom-bar authorization and routes HWND/PID to request-only native helper', () => {
  const body = functionBody(menu, 'run_task_window_action');
  const guard = body.indexOf('if window.label() != BOTTOM_BAR_LABEL');
  const branch = body.match(/"request-close"\s*=>\s*([\s\S]*?)(?=\n\s*(?:"[\w-]+"|_)\s*=>)/)?.[1];
  assert.ok(branch, 'run_task_window_action must accept request-close separately from force-capable close');
  assert.ok(guard >= 0 && guard < body.indexOf('"request-close"'), 'authorization must precede dispatch');
  assert.match(branch, /task_windows::\w+\(\s*request\.hwnd\s*,\s*request\.process_id\s*\)/);
  assert.doesNotMatch(branch, /TaskWindowAction::Close|perform_task_window_action|terminate|kill/i);
  const helperName = branch.match(/task_windows::(\w+)\(/)?.[1];
  const helper = functionBody(windows, helperName);
  assert.match(helper, /task_window_identity\(/, 'capture native identity, do not trust renderer PID alone');
  assert.match(helper, /expected_process_id/, 'require caller expected PID');
  assert.match(helper, /(?:filter\([\s\S]*?!=\s*0|==\s*0|!=\s*0)/, 'reject zero expected PID');
  assert.match(helper, /(?:process_id\s*!=|!=\s*identity\.process_id|process_id\s*==|==\s*identity\.process_id)/, 'match captured native PID');
  assert.match(helper, /request_close_task_window_with_identity\(/);
  assert.doesNotMatch(helper, /(?<!request_)close_task_window_with_identity\(|TaskWindowAction::Close|perform_task_window_action|TerminateProcess/);
});

test('request-only identity helper validates then posts WM_CLOSE without forceful fallback', () => {
  const body = functionBody(actions, 'request_close_task_window_with_identity');
  assert.match(body, /revalidate_close_target\(/);
  assert.match(body, /PostMessageW\([\s\S]*?WM_CLOSE/);
  assert.doesNotMatch(body, /close_window_with_identity|TerminateProcess|spawn_task_window_helper|wait_for_window/);
  const dispatch = functionBody(actions, 'request_close_with');
  assert.match(dispatch, /validate\(\)\?;\s*dispatch\(\)/);
});

test('frontend action wrapper allows request-close without replacing existing standalone close', () => {
  const wrapper = read('src/lib/taskbarMenus.ts');
  const union = wrapper.match(/export type TaskWindowAction\s*=([^;]+);/)?.[1] ?? '';
  assert.match(union, /'request-close'/);
  assert.match(union, /'close'/);
  assert.match(wrapper, /invoke\(IPC_COMMANDS\.runTaskWindowAction, \{ request: \{ hwnd, action, processId \} \}\)/);
});
