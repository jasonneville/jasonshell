import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const source = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const ui = source('../src/components/StackPopupSurface.svelte');
const wrapper = source('../src/lib/stackPopup.ts');
const events = source('../src/ipc/events.ts');
const contracts = source('../src-tauri/src/contracts.rs');
const commands = source('../src-tauri/src/stack_popup.rs');
const clipboard = source('../src-tauri/src/stack_popup/clipboard.rs');
const fileOps = source('../src-tauri/src/stack_popup/file_ops.rs');
const models = source('../src-tauri/src/stack_popup/models.rs');
const auth = source('../src-tauri/src/stack_popup/auth.rs');

function handler(name) {
  const start = commands.indexOf(`fn ${name}(`);
  assert.ok(start >= 0, `${name} command exists`);
  const end = commands.indexOf('\n#[tauri::command]', start + 1);
  return commands.slice(start, end < 0 ? undefined : end);
}

test('one shared typed Stack file-operation event connects native emitter to popup-local listener', () => {
  assert.match(events, /stackFileOperationProgress:\s*'stack-operation:progress'/);
  assert.match(contracts, /stack-operation:progress/);
  assert.match(models + events, /operation_id|operationId/);
  assert.match(ui, /getCurrentWindow\(\)\.listen|currentWindow\.listen/);
  assert.match(ui, /stackFileOperationProgress/);
  assert.doesNotMatch(ui, /\blisten\s*\(\s*IPC_EVENTS\.stackFileOperationProgress/,
    'do not use global listen for private operation events');
});

test('popup creates operation IDs, passes them to each slow mutation, and discards prior operation updates', () => {
  assert.match(ui, /operationId/);
  assert.match(wrapper, /operationId/);
  for (const name of ['paste_stack_items', 'delete_stack_item', 'extract_stack_archive']) {
    assert.match(handler(name), /operation_id:\s*String/);
    assert.match(handler(name), /authorize_stack_command/);
  }
  assert.match(ui, /(?:payload|progress)\.operationId\s*!==\s*\w+|\w+\s*!==\s*(?:payload|progress)\.operationId/,
    'late event for previous operation must not overwrite current status');
});

test('progress is emitted only to authorized originating window, not application-wide or other callers', () => {
  for (const name of ['paste_stack_items', 'delete_stack_item', 'extract_stack_archive']) {
    const body = handler(name);
    assert.match(body, /authorize_stack_command/);
    assert.match(body, /window:\s*WebviewWindow/);
    assert.doesNotMatch(body, /app_handle\.emit\s*\(/);
  }
  assert.match(commands + clipboard + fileOps, /emit_to\s*\(|window\.emit\s*\(/);
  assert.doesNotMatch(auth, /PASTE_STACK_ITEMS[^\n]*TOP_BAR|DELETE_STACK_ITEM[^\n]*TOP_BAR|EXTRACT_STACK_ARCHIVE[^\n]*TOP_BAR/);
});

test('paste reports planning, copy/move, refresh and completion or failure without guessing totals', () => {
  const implementation = commands + clipboard + fileOps + models;
  for (const stage of ['planning', 'copy', 'move', 'complete', 'fail']) {
    assert.match(implementation.toLowerCase(), new RegExp(stage), `${stage} lifecycle missing`);
  }
  assert.match(ui, /refreshFileOperation\(operationId\)/, 'frontend owns listing refresh phase');
  assert.match(implementation, /(?:total_files|totalFiles)/);
  assert.match(implementation, /(?:completed_files|completedFiles)/);
  assert.match(implementation, /(?:total_bytes|totalBytes)/);
  assert.match(implementation, /(?:completed_bytes|completedBytes)/);
  assert.match(implementation, /(?:current_path|currentPath)/);
  assert.match(implementation, /(?:pre.scan|scan.*before|scan.*total)/i);
  assert.match(implementation, /(?:rename|fallback)/i);
  assert.match(ui, /(?:completedFiles|completed_files|totalFiles|total_files)/);
});

test('delete keeps Recycle Bin semantics and shows indeterminate folder and top-level selection progress', () => {
  assert.match(fileOps, /delete_path\s*\(/);
  assert.match(fileOps, /FOF_ALLOWUNDO|FOFX_RECYCLEONDELETE|RecycleBin|recycle/i);
  assert.match(ui, /deleteConfirmation/);
  assert.match(ui, /(?:completedItems|completed_items|completedPaths|completed_paths|deletedCount)/);
  assert.match(ui, /(?:totalItems|total_items|totalPaths|total_paths|paths\.length)/);
  assert.match(ui, /(?:indeterminate|folder)/i);
});

test('extract reports archive lifecycle and elapsed time, not invented percentages or file totals', () => {
  const extraction = handler('extract_stack_archive');
  assert.match(extraction + models + ui, /(?:archive_path|archivePath|archive\.path)/);
  assert.match(extraction + models + ui, /(?:elapsed_ms|elapsedMs|elapsed)/);
  assert.match(ui, /(?:indeterminate|extracting)/i);
  assert.doesNotMatch(extraction, /(?:percent|total_files|total_bytes)/i);
});

test('clipboard copy/cut gives immediate item-count feedback in accessible toolbar status', () => {
  assert.match(ui, /class="stack-status[^\"]*"[^>]*role="status"[^>]*aria-live="polite"/);
  assert.match(ui, /Copied.*(?:length|count)|(?:length|count).*Copied/s);
  assert.match(ui, /Cut.*(?:length|count)|(?:length|count).*Cut/s);
  assert.doesNotMatch(ui, /role="progressbar"|role="dialog"[^>]*progress/i);
});

test('native completion waits for frontend listing before reporting finished', () => {
  assert.doesNotMatch(commands + clipboard, /progress\.phase\(\s*"refreshing"\s*\)/,
    'native commands must not report a frontend-only listing phase');
  assert.match(ui, /if\s*\(payload\.phase\s*===\s*'completed'\)\s*\{\s*refreshFileOperation\(payload\.operationId\)/);
  for (const functionName of ['pasteIntoCurrentFolder', 'confirmDeleteSelection', 'extractSelectedArchive', 'pasteDroppedPaths']) {
    const start = ui.indexOf(`function ${functionName}(`);
    assert.ok(start >= 0, `${functionName} exists`);
    const finish = ui.indexOf('finishFileOperation(operationId', start);
    assert.ok(finish > start, `${functionName} finishes`);
    const operation = ui.slice(start, finish);
    assert.match(operation, /(?:await listStackFolder\(|await invokePaste\()/,
      `${functionName} must await listing before finishing`);
  }
});

test('delete success count advances after successful target, and elapsed clock ticks between events', () => {
  assert.match(ui, /await invokeDelete\(path, operationId\);\s*updateDeleteProgress\(operationId, index \+ 1,/);
  assert.match(ui, /updateDeleteProgress\(operationId, index, pendingDelete\.paths\.length, path\)/);
  assert.match(ui, /operationClockTimer\s*=\s*window\.setInterval\(\(\)\s*=>\s*\{\s*operationClock\s*=\s*Date\.now\(\)/);
  assert.match(ui, /stopOperationClock\(\)/);
});

test('failed native delete target keeps elapsed clock alive for next target until batch terminates', () => {
  const listener = ui.slice(ui.indexOf('listen<StackFileOperationProgress>'), ui.indexOf('}).then((unlisten)', ui.indexOf('listen<StackFileOperationProgress>')));
  assert.match(listener, /payload\.operationId !== activeOperation\.operationId/);
  assert.match(listener, /if\s*\(payload\.phase === 'failed'\)\s*\{[\s\S]*?if\s*\(payload\.operation !== 'delete'\)\s*\{\s*stopOperationClock\(\);\s*\}/,
    'native delete failure must not stop clock; other native failures still do');
  const batch = ui.slice(ui.indexOf('async function confirmDeleteSelection()'), ui.indexOf('async function ', ui.indexOf('async function confirmDeleteSelection()') + 1));
  assert.match(batch, /for\s*\(const \[index, path\] of pendingDelete\.paths\.entries\(\)\)\s*\{[\s\S]*?await invokeDelete\(path, operationId\);[\s\S]*?catch\s*\(error\)\s*\{\s*failures\.push\(/,
    'failed target is recorded and loop continues to next target');
  assert.match(batch, /finishFileOperation\(operationId, failures\.length \? errorMessage : undefined\)/,
    'batch termination stops clock after processing all targets');
});
