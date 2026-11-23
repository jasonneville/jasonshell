import assert from 'node:assert/strict';
import test from 'node:test';
import { countOf, functionBody, productionPart, readRust } from './support/rustSource.mjs';

// GUI-subsystem release builds: console-subsystem children (powershell,
// sc.exe, task children, shutdown.exe) flash a console window unless spawned
// with CREATE_NO_WINDOW. Required shared helper (new file
// src-tauri/src/process_flags.rs, declared `mod process_flags;` in main.rs):
//   pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;
//   pub(crate) fn no_console_window(command: &mut std::process::Command) -> &mut std::process::Command
// GUI launches (explorer / code / control) and the terminal launch must NOT use it.
const flags = productionPart(readRust('src-tauri/src/process_flags.rs'));
const main = productionPart(readRust('src-tauri/src/main.rs'));

test('shared helper applies CREATE_NO_WINDOW via creation_flags and is declared in main.rs', () => {
  assert.match(flags, /CREATE_NO_WINDOW:\s*u32\s*=\s*0x0?800_?0000/);
  const body = functionBody(flags, 'no_console_window');
  assert.ok(body, 'no_console_window body not found');
  assert.match(body, /creation_flags\(\s*CREATE_NO_WINDOW\s*\)/);
  assert.match(main, /^mod process_flags;/m);
});

const SITES = [
  ['src-tauri/src/launchers.rs', 'copy_text_to_clipboard'],
  ['src-tauri/src/search_sources/everything_install.rs', 'everything_service_running'],
  ['src-tauri/src/dev_tools/task_runner.rs', 'spawn_task_child'],
];

for (const [file, fn] of SITES) {
  test(`${file} ${fn} spawns with no_console_window`, () => {
    const text = productionPart(readRust(file));
    const body = functionBody(text, fn);
    assert.ok(body, `${fn} body not found`);
    assert.match(body, /no_console_window\(/);
  });
}

test('system_power.rs shutdown/restart child spawns with no_console_window', () => {
  const text = productionPart(readRust('src-tauri/src/system_power.rs'));
  const at = text.indexOf('power_action_plan(request.action)');
  assert.notEqual(at, -1, 'power action spawn site not found');
  const site = text.slice(at, at + 600);
  assert.match(site, /no_console_window\(/);
});

test('launchers.rs applies it ONLY to the clipboard helper (GUI launches keep their windows)', () => {
  const text = productionPart(readRust('src-tauri/src/launchers.rs'));
  assert.equal(countOf(text.replace(/^use .*$/gm, ''), 'no_console_window('), 1);
});

test('the terminal launch path never uses CREATE_NO_WINDOW (it must show its console)', () => {
  const text = productionPart(readRust('src-tauri/src/stack_popup/file_ops.rs'));
  assert.ok(!/no_console_window|CREATE_NO_WINDOW|creation_flags/.test(text), 'file_ops.rs terminal launch must keep a visible console');
});
