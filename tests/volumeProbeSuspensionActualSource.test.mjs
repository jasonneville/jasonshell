import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
const read = path => readFileSync(path, 'utf8').replace(/\r\n/g, '\n');
function block(source, start) {
  const from = source.indexOf(start); assert.ok(from >= 0, `Missing actual-source seam ${start}`);
  let depth = 0;
  for (let i = source.indexOf('{', from); i < source.length; i++) {
    if (source[i] === '{') depth++;
    if (source[i] === '}' && --depth === 0) return source.slice(from, i + 1);
  }
  throw new Error(`Unbalanced source ${start}`);
}
test('actual volume collector rejects suspended work and cancels retired running probes with real reap', { skip: process.platform !== 'win32' }, () => {
  const base = join(process.env.LOCALAPPDATA, 'Temp/opencode/volume-probe-suspension'); mkdirSync(base, { recursive: true });
  const directory = mkdtempSync(join(base, 'run-')); mkdirSync(join(directory, 'src'));
  const source = read('src-tauri/src/process_manager/volume_probe.rs');
  const modules = ['system_info', 'probe_executor'].map(name => `#[path = ${JSON.stringify(resolve(`src-tauri/src/process_manager/${name}.rs`))}] mod ${name};`).join('\n');
  const fixture = read('tests/native/volume-probe-suspension.rs')
    .replace('/* ACTUAL_MODULES */', modules)
    .replace('/* ACTUAL_UNAVAILABLE */', block(source, 'fn unavailable('))
    .replace('/* ACTUAL_COLLECT */', block(source, 'pub fn collect('))
    .replace('/* HELPER_SOURCE */', JSON.stringify(resolve('src-tauri/tests/fixtures/system_info_probe_child.rs')));
  writeFileSync(join(directory, 'src/lib.rs'), fixture);
  writeFileSync(join(directory, 'Cargo.toml'), '[package]\nname="volume-probe-suspension"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nserde={version="1",features=["derive"]}\nserde_json="1"\nwindows={version="0.61.3",features=["Win32_Foundation","Win32_System_JobObjects","Win32_System_Pipes","Win32_System_Threading","Win32_Security"]}\n');
  const result = spawnSync('cargo', ['test', '--offline', '--manifest-path', join(directory, 'Cargo.toml'), '--target-dir', join(base, 'target'), '--', '--nocapture'], { encoding: 'utf8', timeout: 120000, env: { ...process.env, RUSTC_WRAPPER: '', RUSTC_WORKSPACE_WRAPPER: '' } });
  const output = `${result.stdout ?? ''}\n${result.stderr ?? ''}`; writeFileSync(join(base, 'latest.log'), output); writeFileSync(join(base, 'latest-artifact.txt'), directory);
  assert.equal(result.status, 0, `Actual volume collector/executor failed (${directory}):\n${output}`);
});
