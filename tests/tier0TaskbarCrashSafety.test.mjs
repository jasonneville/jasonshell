import assert from 'node:assert/strict';
import test from 'node:test';
import { functionBody, productionPart, readRust } from './support/rustSource.mjs';

const main = productionPart(readRust('src-tauri/src/main.rs'));
const explorer = productionPart(readRust('src-tauri/src/explorer.rs'));
const appbar = productionPart(readRust('src-tauri/src/appbar.rs'));

function markerBeforeHide(name) {
  const body = functionBody(explorer, name);
  assert.ok(body, `${name} body not found`);
  const hideAt = body.indexOf('set_taskbar_visibility(');
  assert.notEqual(hideAt, -1, `${name} must hide through set_taskbar_visibility`);
  const markerAt = Math.min(
    ...['write_hidden_marker', 'persist_hidden_marker'].map((t) => {
      const i = body.indexOf(t);
      return i === -1 ? Infinity : i;
    }),
  );
  assert.ok(markerAt < hideAt, `${name} must persist the hidden marker before hiding`);
}

// WIN-04: a crash must not leave the Explorer taskbar permanently hidden.
test('main.rs installs a panic hook that restores the taskbar', () => {
  const idx = main.indexOf('std::panic::set_hook');
  assert.notEqual(idx, -1, 'std::panic::set_hook not installed in main.rs');
  assert.match(main.slice(idx, idx + 2500), /explorer::\w*restore\w*/i);
});

test('panic hook is installed AFTER helper-argument dispatch so helper subprocess panics never touch the taskbar', () => {
  const hookAt = main.indexOf('std::panic::set_hook');
  const lastHelper = main.lastIndexOf('"--internal-');
  assert.notEqual(lastHelper, -1, 'helper dispatch not found');
  assert.ok(hookAt > lastHelper, 'set_hook must come after the last --internal-* helper dispatch');
});

test('panic hook never clears the marker', () => {
  const hook = main.slice(main.indexOf('std::panic::set_hook'), main.indexOf('std::panic::set_hook') + 2500);
  assert.ok(!/clear_hidden_marker|finish_taskbar_restore/.test(hook), 'hook must not clear the marker');
  const emergency = functionBody(explorer, 'emergency_restore_taskbar');
  assert.ok(emergency, 'emergency_restore_taskbar body not found');
  assert.ok(!/clear_hidden_marker|finish_taskbar_restore/.test(emergency), 'emergency restore must not clear the marker');
});

test('single-snapshot restore_taskbar does not clear the marker; finish_taskbar_restore is the only clear site', () => {
  const restore = functionBody(explorer, 'restore_taskbar');
  assert.ok(restore, 'restore_taskbar body not found');
  assert.ok(!/clear_hidden_marker/.test(restore), 'restore_taskbar must not clear the marker (first success != all restored)');
  const finish = functionBody(explorer, 'finish_taskbar_restore');
  assert.ok(finish, 'finish_taskbar_restore body not found');
  assert.match(finish, /should_clear_hidden_marker\(/);
  assert.match(finish, /clear_hidden_marker\(/);
  const exact = /(?<![A-Za-z0-9_])clear_hidden_marker\(/g;
  assert.equal((explorer.match(exact) ?? []).length, 2, 'exactly: one definition + one call');
  assert.equal((finish.match(exact) ?? []).length, 1, 'the single call must be inside finish_taskbar_restore');
});

test('appbar cleanup finishes restore through finish_taskbar_restore after all snapshots', () => {
  assert.match(appbar, /finish_taskbar_restore\(/);
});

test('initial hide persists the marker before hiding', () => {
  markerBeforeHide('hide_primary_taskbar_if_needed');
});

test('re-hide paths (enforce / reconcile) persist the marker before hiding', () => {
  markerBeforeHide('enforce_taskbar_hidden');
  markerBeforeHide('reconcile_owned_taskbars');
});

test('startup classification consults the marker via effective_originally_visible', () => {
  assert.ok((explorer.match(/effective_originally_visible\(/g) ?? []).length >= 2);
  assert.match(explorer, /hidden_marker_present\(/);
});
