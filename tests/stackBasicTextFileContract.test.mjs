import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

test('basic text read command is wired across frontend and backend contracts', () => {
  const commands = read('src/ipc/commands.ts');
  const api = read('src/lib/stackPopup.ts');
  const contracts = read('src-tauri/src/contracts.rs');
  const main = read('src-tauri/src/main.rs');
  const backend = read('src-tauri/src/stack_popup.rs');

  assert.match(commands, /readStackBasicTextFile: 'read_stack_basic_text_file'/);
  assert.match(api, /export type StackBasicTextFile = \{[\s\S]*path: string;[\s\S]*content: string;[\s\S]*byteLength: number;[\s\S]*\}/);
  assert.match(api, /readStackBasicTextFile\(path: string\): Promise<StackBasicTextFile>[\s\S]*IPC_COMMANDS\.readStackBasicTextFile, \{ path \}/);
  assert.match(contracts, /READ_STACK_BASIC_TEXT_FILE: &str = "read_stack_basic_text_file"/);
  assert.match(main, /stack_popup::read_stack_basic_text_file,/);
  assert.match(backend, /pub async fn read_stack_basic_text_file[\s\S]*READ_STACK_BASIC_TEXT_FILE[\s\S]*callers:\s*&\[crate::shell_windows::STACK_POPUP_LABEL\]/);
});

test('basic text backend stays bounded, read-only, and off the UI thread', () => {
  const backend = read('src-tauri/src/stack_popup.rs');

  assert.match(backend, /STACK_BASIC_TEXT_FILE_MAX_BYTES:\s*u64\s*=\s*1024 \* 1024/);
  assert.match(backend, /spawn_blocking/);
  assert.match(backend, /FILE_FLAG_OPEN_REPARSE_POINT/);
  assert.match(backend, /stack_basic_text_file_final_path\(&file/);
  assert.match(backend, /GetFinalPathNameByHandleW/);
  assert.match(backend, /is_reparse_point/);
  assert.match(backend, /contains\(&0\)/);
  assert.doesNotMatch(backend, /fn write_stack_basic_text_file|save_stack_basic_text_file/);
});
