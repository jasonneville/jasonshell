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
  const start = source.search(new RegExp(`fn ${name}(?:<[^>]+>)?\\(`));
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

test('folder import command is registered Settings-owned and renderer supplies no source path', () => {
  assert.match(commands, /importSpeechModelFolder: 'import_speech_model_folder'/);
  assert.match(main, /speech_runtime::import_speech_model_folder/);
  assert.match(body(runtime, 'import_speech_model_folder'), /require_model_settings\(&window\)\?/);
  const start = runtime.indexOf('fn import_speech_model_folder(');
  const signature = runtime.slice(start, runtime.indexOf(') ->', start));
  assert.doesNotMatch(signature, /(?:path|source|directory|request)\s*:/);
  assert.match(api, /export function importSpeechModelFolder\(\)/);
  assert.match(api, /invoke<[^>]+>\(IPC_COMMANDS\.importSpeechModelFolder\)/);
  // Inventory/registration checks only; actual generated ACL is shell_app_acl.
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
  const adapter = body(runtime, 'import_model_blocking');
  assert.match(adapter, /import_model_with\(\s*&state,\s*kind,/);
  assert.match(adapter, /app\.dialog\(\)\.file\(\)\.set_parent\(&window\)/);
  assert.match(adapter, /ModelImportKind::Archive => picker\s*\.add_filter\("Parakeet model archive", &\["tar", "tar.gz", "tgz"\]\)\s*\.blocking_pick_file\(\)/);
  assert.match(adapter, /ModelImportKind::Folder => picker\.blocking_pick_folder\(\)/);
  assert.match(adapter, /app_local_data_dir\(\)/);
  assert.match(adapter, /\|path\|\s*\{\s*crate::speech_model::load_parakeet_tdt\(path\)/);
  assert.match(body(runtime, 'import_speech_model'), /import_model_blocking\(app, window, ModelImportKind::Archive\)/);
  assert.match(body(runtime, 'import_speech_model_folder'), /import_model_blocking\(app, window, ModelImportKind::Folder\)/);
  const transaction = body(runtime, 'import_model_with');
  assert.match(transaction, /let validate = \|path: &std::path::Path\|\s*\{\s*loaded = Some\(load\(path\)\?\)/);
  assert.match(transaction, /ModelImportKind::Archive => crate::speech_model_install::install_archive\(\s*&source, &data, limits, validate,/);
  assert.match(transaction, /ModelImportKind::Folder => crate::speech_model_install::install_directory\(\s*&source, &data, limits, validate,/);
  assert.ok(transaction.indexOf('load(path)?') < transaction.indexOf('install_archive('));
  assert.ok(transaction.indexOf('install_directory(') < transaction.indexOf('loaded.ok_or_else'));
  assert.ok(transaction.indexOf('loaded.ok_or_else') < transaction.indexOf('WarmModelState::Ready(model)'));
});

test('import/capture admission is serialized and late warmup epoch cannot publish', () => {
  const importer = body(runtime, 'import_model_with');
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
  assert.match(warmup, /finish_warm_model\(&state, epoch, source, missing, loaded\)/);
  const finish = body(runtime, 'finish_warm_model');
  assert.match(finish, /if operation\.epoch != epoch[\s\S]*operation\.importing[\s\S]*shutting_down[\s\S]*return;/);
  assert.ok(finish.indexOf('operation.epoch != epoch') < finish.indexOf('std::mem::replace('));
});

test('missing/unloadable status stays truthful and cancelled/failed warmup can retry without replacing ready', () => {
  const status = body(runtime, 'model_status');
  assert.match(status, /WarmModelState::Failed if operation\.missing => "missing"/);
  assert.match(status, /WarmModelState::Failed => "error"/);
  assert.match(status, /WarmModelState::Loading => "loading"/);
  const importer = body(runtime, 'import_model_with');
  assert.match(importer, /if matches!\(\*slot, WarmModelState::Loading \| WarmModelState::Failed\)\s*\{\s*\*slot = WarmModelState::NotStarted;/);
  assert.match(runtime, /struct ImportReservation[^]*state: &'a SpeechRuntimeState/);
  assert.match(runtime, /Drop for ImportReservation[\s\S]*operation\.importing = false;[\s\S]*retry\(\)/);
  assert.match(body(runtime, 'import_model_blocking'), /\|\| spawn_warm_model_async\(app\.clone\(\)\)/);
  assert.match(importer, /drop\(reservation\);\s*let model = model_status\(&?state\)\?/);
});
