import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const contractsSource = readFileSync('src-tauri/src/contracts.rs', 'utf8');
const stackPopupSource = readFileSync('src-tauri/src/stack_popup.rs', 'utf8');
const openWithSource = readFileSync('src-tauri/src/stack_popup/open_with.rs', 'utf8');

function extractFunction(source, name) {
  const marker = new RegExp(`(?:pub(?:\\(crate\\))?\\s+)?(?:async\\s+)?fn\\s+${name}(?:<[^>]+>)?\\s*\\(`);
  const match = marker.exec(source);
  assert.ok(match, `missing function ${name}`);
  const start = match.index;
  const bodyStart = source.indexOf('{', start);
  assert.ok(bodyStart >= 0, `missing body for ${name}`);
  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0) return source.slice(start, index + 1);
  }
  throw new Error(`could not extract ${name}`);
}

function extractSpawnBlockingClosure(functionSource, functionName) {
  const spawnIndex = functionSource.indexOf('spawn_blocking');
  assert.ok(spawnIndex >= 0, `${functionName} missing spawn_blocking`);
  const closureMarker = /spawn_blocking\s*\(\s*move\s*\|\|\s*\{/g;
  closureMarker.lastIndex = spawnIndex;
  const match = closureMarker.exec(functionSource);
  assert.ok(match, `${functionName} missing move spawn_blocking closure`);
  const bodyStart = match.index + match[0].length - 1;
  let depth = 0;
  for (let index = bodyStart; index < functionSource.length; index += 1) {
    const char = functionSource[index];
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0) return functionSource.slice(bodyStart, index + 1);
  }
  throw new Error(`could not extract spawn_blocking closure for ${functionName}`);
}

function assertDirectoryRejectedBeforeWork(closureSource, functionName, workPattern) {
  const directoryMessage = 'Open with is only available for files';
  const messageIndex = closureSource.indexOf(directoryMessage);
  assert.ok(messageIndex >= 0, `${functionName} must reject directories with exact Open With file-only error`);
  assert.match(closureSource, /\.is_dir\s*\(\s*\)/, `${functionName} must perform explicit directory check inside spawn_blocking`);
  const workMatch = workPattern.exec(closureSource);
  assert.ok(workMatch, `${functionName} missing expected picker/candidate/app work`);
  assert.ok(
    messageIndex < workMatch.index,
    `${functionName} must reject directories before picker/candidate/app work`,
  );
}

const openWithCommands = [
  ['OPEN_STACK_ITEM_WITH_PICKER', 'open_stack_item_with_picker'],
  ['LIST_STACK_OPEN_WITH_CANDIDATES', 'list_stack_open_with_candidates'],
  ['OPEN_STACK_ITEM_WITH_APP', 'open_stack_item_with_app'],
];

test('stack Open With command names remain stable', () => {
  for (const [constantName, commandName] of openWithCommands) {
    assert.match(
      contractsSource,
      new RegExp(`pub const ${constantName}: &str = "${commandName}";`),
      `${constantName} must keep exact IPC command name`,
    );
  }
});

test('stack Open With commands are async and authorize before blocking work', () => {
  for (const [constantName, functionName] of openWithCommands) {
    const body = extractFunction(stackPopupSource, functionName);
    assert.match(body, new RegExp(`pub\\s+async\\s+fn\\s+${functionName}\\s*\\(`));
    assert.match(body, new RegExp(`command:\\s*crate::contracts::commands::${constantName}`));
    assert.ok(body.indexOf('authorize_stack_command') >= 0, `${functionName} missing auth`);
    assert.ok(body.indexOf('spawn_blocking') >= 0, `${functionName} missing spawn_blocking`);
    assert.ok(
      body.indexOf('authorize_stack_command') < body.indexOf('spawn_blocking'),
      `${functionName} must authorize before spawn_blocking`,
    );
  }
});

test('stack Open With filesystem and launch work is routed inside spawn_blocking', () => {
  const picker = extractFunction(stackPopupSource, 'open_stack_item_with_picker');
  const pickerClosure = extractSpawnBlockingClosure(picker, 'open_stack_item_with_picker');
  assert.match(pickerClosure, /normalize_existing_path\(&path\)\?[\s\S]*open_shell_path_with_picker\(path\)/);
  assert.doesNotMatch(picker.slice(0, picker.indexOf('spawn_blocking')), /normalize_existing_path\(&path\)|open_shell_path_with_picker\(/);

  const candidates = extractFunction(stackPopupSource, 'list_stack_open_with_candidates');
  const candidatesClosure = extractSpawnBlockingClosure(candidates, 'list_stack_open_with_candidates');
  assert.match(candidatesClosure, /normalize_existing_path\(&path\)\?[\s\S]*open_with_candidates_for_path\(Path::new\(&path\)\)/);
  assert.doesNotMatch(candidates.slice(0, candidates.indexOf('spawn_blocking')), /normalize_existing_path\(&path\)|open_with_candidates_for_path\(/);

  const app = extractFunction(stackPopupSource, 'open_stack_item_with_app');
  const appClosure = extractSpawnBlockingClosure(app, 'open_stack_item_with_app');
  assert.match(appClosure, /normalize_existing_path\(&path\)\?[\s\S]*open_with_app\(Path::new\(&path\),\s*&app_id\)/);
  assert.doesNotMatch(app.slice(0, app.indexOf('spawn_blocking')), /normalize_existing_path\(&path\)|open_with_app\(/);

  assertDirectoryRejectedBeforeWork(
    pickerClosure,
    'open_stack_item_with_picker',
    /open_shell_path_with_picker\(path\)/,
  );
  assertDirectoryRejectedBeforeWork(
    candidatesClosure,
    'list_stack_open_with_candidates',
    /open_with_candidates_for_path\(Path::new\(&path\)\)/,
  );
  assertDirectoryRejectedBeforeWork(
    appClosure,
    'open_stack_item_with_app',
    /open_with_app\(Path::new\(&path\),\s*&app_id\)/,
  );

  const openWithApp = extractFunction(openWithSource, 'open_with_app');
  assert.match(openWithApp, /open_with_candidates_for_path\(path\)\?/);
  assert.match(openWithApp, /find\(\|candidate\| candidate\.id == app_id\)/);
  assert.match(openWithApp, /Command::new\(&candidate\.executable\)[\s\S]*\.spawn\(\)/);
});
