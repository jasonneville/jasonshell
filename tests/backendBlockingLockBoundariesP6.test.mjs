import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const stackPopupSource = readFileSync(new URL('../src-tauri/src/stack_popup.rs', import.meta.url), 'utf8');
const stackFileOpsSource = readFileSync(new URL('../src-tauri/src/stack_popup/file_ops.rs', import.meta.url), 'utf8');
const stackClipboardSource = readFileSync(new URL('../src-tauri/src/stack_popup/clipboard.rs', import.meta.url), 'utf8');

test('P6 phase 1 Stack archive extraction runs process status behind spawn_blocking', () => {
  assert.match(stackPopupSource, /pub\s+async\s+fn\s+extract_stack_archive\(/);
  assert.match(stackPopupSource, /tauri::async_runtime::spawn_blocking\(move \|\| run_archive_extraction_plan\(plan\)\)/);

  const commandBody = stackPopupSource.slice(
    stackPopupSource.indexOf('pub async fn extract_stack_archive'),
    stackPopupSource.indexOf('fn run_archive_extraction_plan')
  );
  assert.doesNotMatch(commandBody, /\.status\(\)/);

  const runnerBody = stackPopupSource.slice(
    stackPopupSource.indexOf('fn run_archive_extraction_plan'),
    stackPopupSource.indexOf('pub fn show_stack_item_properties')
  );
  assert.match(runnerBody, /Command::new\(&plan\.executable\)[\s\S]*\.status\(\)/);
  assert.match(runnerBody, /Failed to extract archive: \{error\}/);
  assert.match(runnerBody, /Archive extraction failed with status \{status\}/);
});

test('P6 phase 1 Stack recursive paste and delete commands use async blocking boundaries', () => {
  assert.match(stackPopupSource, /pub\s+async\s+fn\s+paste_stack_items\(/);
  assert.match(stackPopupSource, /clipboard::paste_stack_clipboard_items_async\(&state,\s*destination\)\.await/);
  assert.match(stackClipboardSource, /pub\(crate\)\s+async\s+fn\s+paste_stack_clipboard_items_async/);
  assert.match(stackClipboardSource, /tauri::async_runtime::spawn_blocking\(move \|\| \{\s*paste_clipboard_items\(&clipboard,\s*&destination\)\s*\}\)/);
  assert.match(stackClipboardSource, /update_cut_clipboard_after_paste\(state,\s*used_internal_clipboard,\s*&result\)/);

  assert.match(stackPopupSource, /pub\s+async\s+fn\s+delete_stack_item\(/);
  assert.match(stackPopupSource, /file_ops::delete_stack_item_path_async\(path\)\.await/);
  assert.match(stackFileOpsSource, /pub\(crate\)\s+async\s+fn\s+delete_stack_item_path_async/);
  assert.match(stackFileOpsSource, /tauri::async_runtime::spawn_blocking\(move \|\| delete_path\(&target\)\)/);
});

test('P6 phase 1 keeps cheap stack commands off the new blocking job boundary', () => {
  assert.match(stackPopupSource, /pub\s+fn\s+read_stack_folder\(/);
  assert.match(stackPopupSource, /pub\s+fn\s+suggest_stack_paths\(/);
  assert.doesNotMatch(stackPopupSource, /pub\s+async\s+fn\s+read_stack_folder\(/);
  assert.doesNotMatch(stackPopupSource, /pub\s+async\s+fn\s+suggest_stack_paths\(/);
});
