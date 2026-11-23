import assert from 'node:assert/strict';
import test from 'node:test';
import { functionBody, productionPart, readRust } from './support/rustSource.mjs';

// WIN-06: the ShellRuntimeState guard must not be held across SendMessage /
// SPI broadcast / cross-thread SetWindowPos, AND cleanup must not race an
// in-flight transition once the guard is dropped. Behaviour of the race is
// unit-tested in src-tauri/src/appbar_tier0_tests.rs; this file pins the
// structural wiring the unit seams cannot see.
const appbar = productionPart(readRust('src-tauri/src/appbar.rs'));

function firstIndexOf(body, tokens) {
  const found = tokens.map((t) => body.indexOf(t)).filter((i) => i !== -1);
  return found.length ? Math.min(...found) : -1;
}

test('hide_shell_for_fullscreen drops the state guard before cross-thread side effects', () => {
  const body = functionBody(appbar, 'hide_shell_for_fullscreen');
  assert.ok(body, 'hide_shell_for_fullscreen body not found');
  const dropAt = body.indexOf('drop(state)');
  assert.notEqual(dropAt, -1, 'expected explicit drop(state)');
  const sideEffect = firstIndexOf(body, [
    'unregister_tracked_appbars_with(',
    'unregister_appbar',
    'sync_work_area_best_effort_with(',
    'run_fullscreen_hide_effects_with(',
    'park_shell_surface_windows(',
  ]);
  assert.ok(sideEffect !== -1 && dropAt < sideEffect, 'drop(state) must precede unregister/SPI/park calls');
});

test('restore_shell_after_fullscreen drops the state guard before cross-thread side effects', () => {
  const body = functionBody(appbar, 'restore_shell_after_fullscreen');
  assert.ok(body, 'restore_shell_after_fullscreen body not found');
  const dropAt = body.indexOf('drop(state)');
  assert.notEqual(dropAt, -1, 'expected explicit drop(state)');
  const sideEffect = firstIndexOf(body, [
    'prepare_fullscreen_restore_retry_with(',
    'sync_work_area_best_effort(',
    'apply_reserved_work_area_with(',
    'register_tracked_appbar(',
    'register_appbar_and_track_vec(',
    'park_shell_surface_windows(',
    'unregister_tracked_appbars_with(',
    'unregister_appbar',
  ]);
  assert.ok(sideEffect !== -1 && dropAt < sideEffect, 'drop(state) must precede register/unregister/SPI/park calls');
});

test('worker-thread parking of shell HWNDs uses SWP_ASYNCWINDOWPOS', () => {
  const park = functionBody(appbar, 'park_window_below_virtual_desktop');
  assert.ok(park, 'park_window_below_virtual_desktop body not found');
  assert.match(park, /SWP_ASYNCWINDOWPOS/);
});

test('both transitions snapshot the baseline work area before dropping the lock and route SPI through re-validating seams', () => {
  const hide = functionBody(appbar, 'hide_shell_for_fullscreen');
  const hideDrop = hide.indexOf('drop(state)');
  const hideBaseline = hide.indexOf('baseline_work_area');
  assert.ok(hideBaseline !== -1 && hideBaseline < hideDrop, 'hide must copy baseline_work_area before drop(state)');
  assert.match(hide, /run_fullscreen_hide_effects_with\(/);
  assert.match(hide, /fullscreen_transition_is_current\(/);
  assert.ok(
    !/sync_work_area_best_effort_with\(\s*layout\.monitor_rect/.test(hide),
    'hide must not set the full-monitor work area outside run_fullscreen_hide_effects_with',
  );

  const restore = functionBody(appbar, 'restore_shell_after_fullscreen');
  const restoreDrop = restore.indexOf('drop(state)');
  const restoreBaseline = restore.indexOf('baseline_work_area');
  assert.ok(restoreBaseline !== -1 && restoreBaseline < restoreDrop, 'restore must copy baseline_work_area before drop(state)');
  assert.match(restore, /apply_reserved_work_area_with\(/);
  assert.match(restore, /fullscreen_transition_is_current\(/);
  assert.ok(
    !/sync_work_area_best_effort\(\s*reserved_work_area\(/.test(restore),
    'restore must not apply the reserved work area outside apply_reserved_work_area_with',
  );
});

test('restore: the stale-after-register branch resets the SPI work area to the baseline', () => {
  const restore = functionBody(appbar, 'restore_shell_after_fullscreen');
  const stale = restore.indexOf('!fullscreen_transition_is_current(');
  assert.notEqual(stale, -1, 'stale branch not found');
  const branch = restore.slice(stale, stale + 900);
  assert.match(branch, /baseline|set_work_area|sync_work_area_best_effort/, 'stale branch must restore the baseline work area, not only unregister');
});

test('restore: Reserved/shell_layout is committed only when the full transition-current check passes', () => {
  const restore = functionBody(appbar, 'restore_shell_after_fullscreen');
  const matchAt = restore.indexOf('match position_result');
  assert.notEqual(matchAt, -1, 'match position_result not found');
  const okArm = restore.slice(matchAt, restore.indexOf('Err(error) =>', matchAt));
  assert.match(okArm, /fullscreen_transition_is_current\(/, 'success arm must use fullscreen_transition_is_current, not just cleaned_up/CleanupInProgress');
  assert.ok(okArm.indexOf('fullscreen_transition_is_current(') < okArm.indexOf('FullscreenAppBarState::Reserved'));
});

test('begin_cleanup_plan bounded-waits (<= 2 s) for an active fullscreen transition before taking state', () => {
  const body = functionBody(appbar, 'begin_cleanup_plan');
  assert.ok(body, 'begin_cleanup_plan body not found');
  const call = /wait_for_fullscreen_transition_idle\(\s*([^)]*\)?)\s*\)/.exec(body);
  assert.ok(call, 'begin_cleanup_plan must call wait_for_fullscreen_transition_idle(');
  let arg = call[1].trim();
  const constant = /^[A-Z_][A-Z0-9_]*$/.test(arg)
    ? new RegExp(`const ${arg}: Duration = (Duration::from_(?:millis|secs)\\(\\d+\\))`).exec(appbar)
    : null;
  if (constant) arg = constant[1];
  const m = /Duration::from_(millis|secs)\((\d+)\)/.exec(arg);
  assert.ok(m, `timeout must be a literal Duration or a const Duration, got: ${call[1]}`);
  const ms = m[1] === 'secs' ? Number(m[2]) * 1000 : Number(m[2]);
  assert.ok(ms > 0 && ms <= 2000, `timeout ${ms}ms must be within (0, 2000]`);
  assert.ok(
    body.indexOf('wait_for_fullscreen_transition_idle(') < body.indexOf('state.lock()'),
    'wait must happen before cleanup takes the runtime state',
  );
});
