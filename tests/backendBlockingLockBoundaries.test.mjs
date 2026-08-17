import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const stackPopup = readFileSync('src-tauri/src/stack_popup.rs', 'utf8');
const stackClipboard = readFileSync('src-tauri/src/stack_popup/clipboard.rs', 'utf8');
const processManager = readFileSync('src-tauri/src/process_manager.rs', 'utf8');
const taskPreview = readFileSync('src-tauri/src/task_preview.rs', 'utf8');

function extractFunction(source, name) {
  const marker = new RegExp(`(?:pub(?:\\(crate\\))?\\s+)?(?:async\\s+)?fn\\s+${name}(?:<[^>]+>)?\\s*\\(`);
  const match = marker.exec(source);
  assert.ok(match, `missing function ${name}`);
  const start = match.index;
  const bodyStart = source.indexOf('{', start);
  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0) {
      return source.slice(start, index + 1);
    }
  }
  throw new Error(`could not extract ${name}`);
}

test('stack archive extraction and recursive file ops run behind blocking task boundaries', () => {
  const extractStackArchive = extractFunction(stackPopup, 'extract_stack_archive');
  assert.match(extractStackArchive, /pub\s+async\s+fn\s+extract_stack_archive/);
  assert.match(extractStackArchive, /tauri::async_runtime::spawn_blocking\(move \|\| run_archive_extraction_plan\(plan\)\)/);
  const runArchiveExtractionPlan = extractFunction(stackPopup, 'run_archive_extraction_plan');
  assert.match(runArchiveExtractionPlan, /process_runner::run_process\(spec\)/);
  assert.doesNotMatch(extractStackArchive, /\.status\(\)/, 'archive status wait must not remain on the async command path');

  const pasteStackItems = extractFunction(stackPopup, 'paste_stack_items');
  assert.match(pasteStackItems, /pub\s+async\s+fn\s+paste_stack_items/);
  assert.match(pasteStackItems, /paste_stack_clipboard_items_async\(&app_handle, &state, destination\)\.await/);

  const pasteAsync = extractFunction(stackClipboard, 'paste_stack_clipboard_items_async');
  assert.match(pasteAsync, /tauri::async_runtime::spawn_blocking\(move \|\| \{\s*paste_clipboard_items\(&clipboard, &destination, journal_dir\.as_deref\(\)\)/);

  const deleteStackItem = extractFunction(stackPopup, 'delete_stack_item');
  assert.match(deleteStackItem, /pub\s+async\s+fn\s+delete_stack_item/);
  assert.match(deleteStackItem, /file_ops::delete_stack_item_path_async\(path\)\.await/);
  const deleteAsync = extractFunction(readFileSync('src-tauri/src/stack_popup/file_ops.rs', 'utf8'), 'delete_stack_item_path_async');
  assert.match(deleteAsync, /spawn_blocking\(move \|\| delete_path\(&target\)\)/);
});

test('process manager icon extraction is outside cache mutex guard', () => {
  const body = extractFunction(processManager, 'process_icon_data_url');
  assert.match(body, /process_icon_data_url_from_cache_with_extractor\(\s*cache,\s*executable_path,/);
  const helper = extractFunction(processManager, 'process_icon_data_url_from_cache_with_extractor');
  assert.match(helper, /cached_process_icon_data_url\(cache, &icon_cache_key\)/);
  assert.match(helper, /let icon_data_url\s*=\s*extractor\(Path::new\(&icon_cache_key\)\);/);
  assert.match(helper, /store_process_icon_data_url\(cache, &icon_cache_key, icon_data_url\.clone\(\)\);/);
  assert.doesNotMatch(body, /cache\.lock\(\)[\s\S]*shell_file_icon_data_url/);
  assert.doesNotMatch(helper, /cache\.lock\(\)[\s\S]*shell_file_icon_data_url/);
});

test('bounded icon cache helper enforces ttl and lru-like capacity', () => {
  const helper = readFileSync('src-tauri/src/task_windows/bounded_string_cache.rs', 'utf8');
  assert.match(helper, /capacity: usize/);
  assert.match(helper, /positive_ttl: Duration/);
  assert.match(helper, /negative_ttl: Duration/);
  assert.match(helper, /evict_over_capacity/);
  assert.match(helper, /evict_expired/);
  assert.match(helper, /caches_positive_and_negative_values/);
  assert.match(helper, /evicts_oldest_entry_at_capacity/);
});

test('task preview window operations happen after runtime guard is dropped', () => {
  const body = extractFunction(taskPreview, 'publish_and_show_preview');
  assert.match(body, /ensure_preview_request_is_current\(state, request_id\)\?/);
  const guardHelper = extractFunction(taskPreview, 'ensure_preview_request_is_current');
  assert.match(guardHelper, /state\s*\.lock\(\)[\s\S]*preview_request_is_current/);
  assert.match(body, /preview_window\s*\.emit\(/);
  assert.doesNotMatch(body, /let\s+mut\s+state\s*=\s*state\s*\.lock\(\)[\s\S]*preview_window\s*\.(emit|set_position|show|hide|set_focus)\(/);
});
