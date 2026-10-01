// Static Tauri boundary checks only: not native picker/capture/process-restart E2E.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const runtime = read('src-tauri/src/speech_runtime.rs');
const main = read('src-tauri/src/main.rs');
const api = read('src/lib/speech.ts');
const commands = read('src/ipc/commands.ts');

function body(source, name) {
  const start = source.indexOf(`fn ${name}(`);
  assert.ok(start >= 0, `function ${name} exists`);
  const open = source.indexOf('{', start);
  let depth = 0;
  for (let index = open; index < source.length; index++) {
    if (source[index] === '{') depth++;
    if (source[index] === '}' && --depth === 0) return source.slice(open + 1, index);
  }
  assert.fail(`function ${name} has balanced body`);
}

test('registered native model commands authorize Settings and accept no renderer file path', () => {
  for (const [camel, snake] of [['getSpeechModelStatus', 'get_speech_model_status'], ['importSpeechModel', 'import_speech_model']]) {
    assert.match(commands, new RegExp(`${camel}: '${snake}'`));
    assert.match(main, new RegExp(`speech_runtime::${snake}`));
    assert.match(body(runtime, snake), /require_model_settings\(&window\)\?/);
    assert.match(api, new RegExp(`export function ${camel}\\(\\)`));
    assert.match(api, new RegExp(`invoke<[^>]+>\\(IPC_COMMANDS\\.${camel}\\)`));
  }
  assert.match(body(runtime, 'require_model_settings'), /window\.label\(\) != crate::shell_windows::SETTINGS_PANEL_LABEL/);
  const signature = runtime.slice(runtime.indexOf('async fn import_speech_model('), runtime.indexOf('fn import_model_blocking('));
  assert.doesNotMatch(signature, /archive:\s|path:\s|request:\s/);
  assert.match(main, /tauri_plugin_dialog::init\(\)/);
});

test('warmup resolves app-local installation before optional bundle and uses actual model loader', () => {
  const resolver = body(runtime, 'resolve_model_resource');
  assert.ok(resolver.indexOf('resolve_installed_model(&data)') < resolver.indexOf('resolve(MODEL_RESOURCE'));
  assert.match(resolver, /app_local_data_dir\(\)/);
  assert.match(resolver, /return Ok\(\(path, "installed"\)\)/);
  const warmup = body(runtime, 'spawn_warm_model_async');
  assert.match(warmup, /resolve_model_resource\(&app\)/);
  assert.match(warmup, /preload_parakeet_tdt\(&path\)/);
  assert.match(body(runtime, 'preload_parakeet_tdt'), /speech_model::load_parakeet_tdt\(model_path\)/);
});

test('import uses native filtered picker and ONNX loader callback before ready pool assignment', () => {
  const importer = body(runtime, 'import_model_blocking');
  assert.match(importer, /\.dialog\(\)[\s\S]*\.set_parent\(&window\)[\s\S]*\.add_filter\("Parakeet model archive", &\["tar", "tar.gz", "tgz"\]\)[\s\S]*\.blocking_pick_file\(\)/);
  assert.match(importer, /app_local_data_dir\(\)/);
  assert.match(importer, /install_archive\([\s\S]*\|path\|\s*\{[\s\S]*speech_model::load_parakeet_tdt\(path\)/);
  assert.ok(importer.indexOf('speech_model::load_parakeet_tdt(path)') < importer.indexOf('WarmModelState::Ready(model)'));
});

test('import/capture admission is serialized and late warmup epoch cannot publish', () => {
  const importer = body(runtime, 'import_model_blocking');
  assert.match(importer, /state\.commit\.lock\(\)/);
  for (const guard of [/operation\.importing/, /WarmModelState::InUse/, /inner\.capture\.is_some\(\)/, /SpeechStatusKind::Recording \| SpeechStatusKind::Transcribing/]) assert.match(importer, guard);
  assert.match(importer, /operation\.epoch = operation\.epoch\.wrapping_add\(1\)/);
  // Admission lexical block ends before picker and extraction side effects.
  assert.match(importer, /operation\.epoch = [^;]+;\s*\}\s*let reservation/);
  const capture = body(runtime, 'start_speech_capture');
  assert.match(capture, /state\.commit\.lock\(\)/);
  assert.ok(capture.indexOf('.importing') < capture.indexOf('create_capture_stream'));
  const warmup = body(runtime, 'spawn_warm_model_async');
  assert.match(warmup, /drop\(operation\);[\s\S]*spawn_blocking/);
  assert.match(warmup, /if operation\.epoch != epoch[\s\S]*operation\.importing[\s\S]*shutting_down[\s\S]*return;/);
  assert.ok(warmup.indexOf('operation.epoch != epoch') < warmup.indexOf('*slot = match loaded'));
});

test('missing/unloadable status stays truthful and cancelled/failed warmup can retry without replacing ready', () => {
  const status = body(runtime, 'model_status');
  assert.match(status, /WarmModelState::Failed if operation\.missing => "missing"/);
  assert.match(status, /WarmModelState::Failed => "error"/);
  assert.match(status, /WarmModelState::Loading => "loading"/);
  const importer = body(runtime, 'import_model_blocking');
  assert.match(importer, /if matches!\(\*slot, WarmModelState::Loading \| WarmModelState::Failed\)\s*\{\s*\*slot = WarmModelState::NotStarted;/);
  assert.match(runtime, /impl Drop for ImportReservation[\s\S]*operation\.importing = false;[\s\S]*spawn_warm_model_async/);
});
