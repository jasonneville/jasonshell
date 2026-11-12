import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
const read = (path) => readFileSync(path, 'utf8');
test('System Info commands have a single canonical TS/Rust inventory and registered handlers', () => {
  const ts = read('src/ipc/commands.ts'); const rust = read('src-tauri/src/contracts.rs'); const main = read('src-tauri/src/main.rs');
  for (const command of ['get_process_manager_context','set_process_manager_tab','get_system_info_snapshot']) {
    assert.ok(ts.includes(`'${command}'`) || ts.includes(`"${command}"`), `TS inventory ${command}`);
    assert.ok(rust.includes(`"${command}"`), `Rust inventory ${command}`);
    assert.ok(main.includes(`process_manager::${command}`), `registered handler ${command}`);
  }
});
test('renderer bridge exposes typed context, epoch-bound tab changes and cache-only snapshot reads', () => {
  const source = read('src/lib/processManager.ts');
  for (const name of ['ProcessManagerContext','ProcessManagerTab','SystemInfoSnapshot','getProcessManagerContext','setProcessManagerTab','getSystemInfoSnapshot']) assert.ok(source.includes(name), name);
  for (const key of ['openGeneration','tabRevision','snapshotRevision','afterRevision','focusPid']) assert.ok(source.includes(key),key);
  assert.match(source, /system_info/);
});
test('native System Info handlers use injected window and shared registered-instance authorization', () => {
  const source = read('src-tauri/src/process_manager.rs');
  for (const command of ['get_process_manager_context','set_process_manager_tab','get_system_info_snapshot']) {
    const match = source.match(new RegExp(`pub (?:async )?fn ${command}\\s*\\(([\\s\\S]*?)\\n\\}`));
    assert.ok(match,command); assert.match(match[1], /window:\s*WebviewWindow/);
    assert.match(match[1], /authorize_(?:process_manager|manager)/);
  }
});

function rustBody(source, name) {
  const signature = source.indexOf(`fn ${name}(`);
  assert.notEqual(signature,-1,`${name} exists`);
  const start = source.indexOf('{',signature); let depth = 0;
  for (let i = start; i < source.length; i++) {
    if (source[i] === '{') depth++;
    if (source[i] === '}' && --depth === 0) return source.slice(start+1,i);
  }
  assert.fail(`${name} body closes`);
}
test('snapshot handler is a guarded read-only clone path, not a hardware or work-admission path', () => {
  const body = rustBody(read('src-tauri/src/process_manager.rs'),'get_system_info_snapshot');
  const guard = body.indexOf('authorize_process_manager(&window)?');
  assert.ok(guard >= 0,'concrete caller guard required');
  const stateAccess = body.indexOf('.state::<');
  assert.ok(stateAccess > guard,'guard before runtime/cache access');
  assert.doesNotMatch(body,/\b(?:spawn|collect|admit|reserve|warm|start|invalidate)\s*\(/,'cache reads must not collect/enqueue/mutate epochs');
  assert.doesNotMatch(body,/let\s+mut\s|\.as_mut\s*\(/,'no cache mutation through reader');
  assert.match(body,/cloned\s*\(|clone\s*\(|read_snapshot\s*\(/,'return a detached cache value');
});
test('manager authorization resolves the registered concrete instance rather than accepting label equality', () => {
  const source = read('src-tauri/src/process_manager.rs');
  const adapter = rustBody(source,'same_registered_instance');
  assert.match(adapter,/get_webview_window\(label\)/);
  assert.match(adapter,/window\.hwnd\(\)/); assert.match(adapter,/registered\.hwnd\(\)/);
  assert.match(adapter,/actual\s*==\s*expected/,'current HWND identities compared');
  const bridge = rustBody(source,'authorize_process_manager');
  assert.match(bridge,/authorize_manager\(window\.label\(\)/);
  assert.match(bridge,/same_registered_instance\(window,\s*PROCESS_MANAGER_LABEL\)/);
});

test('native closed-event producers carry retired context so delayed closes are generation/revision scoped', () => {
  const bridge=read('src-tauri/src/process_manager.rs');const main=read('src-tauri/src/main.rs');
  for(const [path,source] of [['process_manager.rs',bridge],['main.rs',main]]) {
    assert.doesNotMatch(source,/\.emit\(\s*(?:process_manager::)?PROCESS_MANAGER_CLOSED_EVENT\s*,\s*\(\)\s*\)/,`${path}: unversioned close can retire a later opening`);
  }
  assert.match(bridge,/\.emit\(\s*PROCESS_MANAGER_CLOSED_EVENT\s*,[\s\S]*?\bcontext\b/,'closed payload includes authoritative retired context');
});
