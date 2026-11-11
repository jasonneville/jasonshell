import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

// These are wiring/compatibility guards only, not memory or native runtime proof.
test('existing chunks poll stays registered and target-authorized for both terminal surfaces', () => {
  const command = read('src-tauri/src/stack_popup.rs').split('pub fn poll_stack_terminal_session(')[1].split('\n#[tauri::command]')[0];
  assert.match(command, /StackCommandAuth::TerminalSessionTarget/);
  assert.match(command, /POLL_STACK_TERMINAL_SESSION/);
  assert.match(command, /STACK_POPUP_LABEL/);
  assert.match(command, /TERMINAL_PANEL_LABEL/);
  assert.match(command, /terminal::poll_stack_terminal_session\([^;]*window\.label\(\)/);
  assert.match(read('src-tauri/src/main.rs'), /stack_popup::poll_stack_terminal_session/);
  for (const surface of ['stack-popup', 'terminal-panel']) {
    const capability = JSON.parse(read(`src-tauri/capabilities/${surface}.json`));
    assert.ok(capability.windows.includes(surface));
    assert.ok(capability.permissions.includes('core:default'));
  }
});

test('legacy native read retains aggregate output and chunks while modern poll carries chunks only', () => {
  const native = read('src-tauri/src/stack_popup/terminal.rs');
  const legacy = native.split('pub(crate) fn read_stack_terminal(')[1].split('pub(crate) fn poll_stack_terminal_session(')[0];
  assert.match(legacy, /poll_stack_terminal_session/);
  assert.match(legacy, /\.collect::<String>\(\)/);
  assert.match(legacy, /output,/);
  assert.match(legacy, /chunks: result\.chunks/);
  assert.match(legacy, /exited: !result\.running/);
  const pollShape = native.split('pub struct StackTerminalPollResult {')[1].split('\n}')[0];
  assert.match(pollShape, /pub chunks: Vec<StackTerminalOutputChunk>/);
  assert.doesNotMatch(pollShape, /pub output:/);
});
