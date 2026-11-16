import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const read = path => readFileSync(path, 'utf8').replace(/\r\n/g, '\n');
function block(source, start) {
  const from = source.indexOf(start); assert.ok(from >= 0, `Missing actual-source seam: ${start}`);
  let depth = 0; const brace = source.indexOf('{', from);
  for (let i = brace; i < source.length; i++) {
    if (source[i] === '{') depth++;
    if (source[i] === '}' && --depth === 0) return source.slice(from, i + 1);
  }
  throw new Error(`Unbalanced production seam ${start}`);
}
test('actual native delayed branch + PM runtime survive queued refocus/reopen', () => {
  const root = resolve('.');
  const base = join(process.env.LOCALAPPDATA, 'Temp', 'opencode', 'panel-blur-actual-source'); mkdirSync(base, { recursive: true });
  const directory = mkdtempSync(join(base, 'run-')); mkdirSync(join(directory, 'src'));
  const main = read('src-tauri/src/main.rs');
  const delayed = block(main, 'if let (Some(revision), Some(expected))');
  const retire = block(read('src-tauri/src/process_manager.rs'), 'pub fn retire_system_info(');
  const suspend = block(read('src-tauri/src/process_manager.rs'), 'pub fn suspend_system_info(');
  const scheduler = block(read('src-tauri/src/process_manager/native_runtime.rs'), 'if let Ok(mut state) = inner.lock()');
  const inactiveCollector = block(read('src-tauri/src/process_manager/native_runtime.rs'), 'if !active && state.coordinator.context().visible');
  const fixture = read('tests/native/panel-blur-facade.rs')
    .replace('/* ACTUAL_SYSTEM_INFO_MODULE */', `#[path = ${JSON.stringify(join(root, 'src-tauri/src/process_manager/system_info.rs'))}] mod system_info;`)
    .replace('/* ACTUAL_RETIRE_FUNCTION */', retire)
    .replace('/* ACTUAL_SUSPEND_FUNCTION */', suspend)
    .replace('/* ACTUAL_SCHEDULER_TICK */', scheduler)
    .replace('/* ACTUAL_COLLECTOR_INACTIVE */', inactiveCollector)
    .replace('/* ACTUAL_DEFERRED_BRANCH */', delayed);
  writeFileSync(join(directory, 'src/lib.rs'), fixture);
  writeFileSync(join(directory, 'Cargo.toml'), '[package]\nname="panel-blur-actual-source"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nserde={version="1",features=["derive"]}\n');
  const result = spawnSync('cargo', ['test', '--offline', '--manifest-path', join(directory, 'Cargo.toml'), '--target-dir', join(base, 'target'), '--', '--nocapture'], { encoding: 'utf8', timeout: 120000, env: { ...process.env, RUSTC_WRAPPER: '', RUSTC_WORKSPACE_WRAPPER: '' } });
  const output = `${result.stdout ?? ''}\n${result.stderr ?? ''}`;
  writeFileSync(join(base, 'latest.log'), output);
  writeFileSync(join(base, 'latest-artifact.txt'), directory);
  assert.equal(result.status, 0, `Actual-source Rust facade failed (artifacts ${directory}):\n${output}`);
});
