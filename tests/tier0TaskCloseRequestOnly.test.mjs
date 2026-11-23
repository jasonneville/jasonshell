import assert from 'node:assert/strict';
import test from 'node:test';
import { countOf, functionBody, productionPart, readRust } from './support/rustSource.mjs';

// WIN-16 / SEC-04: single-window Close is request-only. It must never
// terminate the owning process, spawn the elevated helper, or send a second
// WM_CLOSE after a SendMessageTimeout timeout.
const actions = productionPart(readRust('src-tauri/src/task_windows/actions.rs'));
const helper = productionPart(readRust('src-tauri/src/task_windows/helper.rs'));
const banned = [
  'TerminateProcess',
  'terminate_window_process',
  'elevate_close_target',
  'spawn_task_window_helper',
  'should_elevate_after_access_denied',
];

test('actions.rs production code has no terminate/elevate path', () => {
  for (const token of banned) {
    assert.ok(!actions.includes(token), `actions.rs must not reference ${token}`);
  }
});

test('close_window_with_identity exists, decides via close_followup, and posts only on PostFallback', () => {
  const body = functionBody(actions, 'close_window_with_identity');
  assert.ok(body, 'close_window_with_identity must exist (the single close path)');
  assert.match(body, /close_followup\(/);
  assert.match(body, /WM_CLOSE|SC_CLOSE/);
  assert.match(body, /CloseFollowup::PostFallback/);
  assert.equal(countOf(body, 'PostMessageW('), 1, 'at most one fallback post');
  assert.ok(
    body.indexOf('PostMessageW(') > body.indexOf('CloseFollowup::PostFallback'),
    'PostMessageW must live inside the PostFallback arm',
  );
  assert.ok(!/close_request_result\(/.test(body), 'tautological close_request_result is replaced by close_followup');
});

test('close_task_window_with_identity exists and does not escalate or kill', () => {
  const body = functionBody(actions, 'close_task_window_with_identity');
  assert.ok(body, 'close_task_window_with_identity body must exist');
  for (const token of banned) assert.ok(!body.includes(token), `${token} reachable from close_task_window_with_identity`);
});

test('elevated helper no longer terminates processes on close', () => {
  assert.ok(!helper.includes('TerminateProcess'), 'helper.rs must not call TerminateProcess');
});

test('Process Manager kill path stays untouched', () => {
  const pm = readRust('src-tauri/src/process_manager.rs');
  assert.match(pm, /TerminateProcess|kill/i, 'process_manager.rs must keep its explicit kill');
});
