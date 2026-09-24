import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

test('basic text read and save commands are wired across frontend and backend contracts', () => {
  const commands = read('src/ipc/commands.ts');
  const api = read('src/lib/stackPopup.ts');
  const contracts = read('src-tauri/src/contracts.rs');
  const main = read('src-tauri/src/main.rs');
  const backend = read('src-tauri/src/stack_popup.rs');

  assert.match(commands, /readStackBasicTextFile: 'read_stack_basic_text_file'/);
  assert.match(commands, /saveStackBasicTextFile: 'save_stack_basic_text_file'/);
  assert.match(api, /export type StackBasicTextFile = \{[\s\S]*path: string;[\s\S]*content: string;[\s\S]*byteLength: number;[\s\S]*identity: string;[\s\S]*\}/);
  assert.match(api, /readStackBasicTextFile\(path: string\): Promise<StackBasicTextFile>[\s\S]*IPC_COMMANDS\.readStackBasicTextFile, \{ path \}/);
  assert.match(api, /saveStackBasicTextFile\([\s\S]*path: string,[\s\S]*content: string,[\s\S]*expectedContent: string,[\s\S]*expectedIdentity: string[\s\S]*IPC_COMMANDS\.saveStackBasicTextFile,[\s\S]*expectedContent,[\s\S]*expectedIdentity/);
  assert.match(contracts, /READ_STACK_BASIC_TEXT_FILE: &str = "read_stack_basic_text_file"/);
  assert.match(contracts, /SAVE_STACK_BASIC_TEXT_FILE: &str = "save_stack_basic_text_file"/);
  assert.match(main, /stack_popup::read_stack_basic_text_file,/);
  assert.match(main, /stack_popup::save_stack_basic_text_file,/);
  assert.match(backend, /pub async fn read_stack_basic_text_file[\s\S]*READ_STACK_BASIC_TEXT_FILE[\s\S]*callers:\s*&\[crate::shell_windows::STACK_POPUP_LABEL\]/);
  assert.match(backend, /pub async fn save_stack_basic_text_file[\s\S]*SAVE_STACK_BASIC_TEXT_FILE[\s\S]*callers:\s*&\[crate::shell_windows::STACK_POPUP_LABEL\]/);
  const auth = read('src-tauri/src/stack_popup/auth.rs');
  assert.match(auth, /SAVE_STACK_BASIC_TEXT_FILE,[\s\S]*callers: &\[contracts::surfaces::STACK_POPUP\]/);
});

test('basic text backend stays bounded, safe for regular UTF-8 files, and off the UI thread', () => {
  const backend = read('src-tauri/src/stack_popup.rs');

  assert.match(backend, /STACK_BASIC_TEXT_FILE_MAX_BYTES:\s*u64\s*=\s*1024 \* 1024/);
  assert.match(backend, /spawn_blocking/);
  assert.match(backend, /FILE_FLAG_OPEN_REPARSE_POINT/);
  assert.match(backend, /stack_basic_text_file_final_path\(&file/);
  assert.match(backend, /GetFinalPathNameByHandleW/);
  assert.match(backend, /is_reparse_point/);
  assert.match(backend, /contains\(&0\)/);
  assert.match(backend, /fn save_stack_basic_text_file_blocking[\s\S]*validate_stack_basic_text_content\(content\)[\s\S]*validate_stack_basic_text_content\(expected_content\)[\s\S]*create_stack_basic_text_staging\(parent\)/);
  assert.match(backend, /write_stage\(&mut stage_file, content\.as_bytes\(\)\)[\s\S]*stage_file[\s\S]*sync_all\(\)/);
  assert.match(backend, /fn create_stack_basic_text_staging[\s\S]*create_new\(true\)/);
  assert.match(backend, /open_stack_basic_text_file_for_save\(&candidate\)[\s\S]*stack_basic_text_file_matches\(&mut target_guard[\s\S]*open_stack_basic_text_file\(&candidate\)[\s\S]*stack_basic_text_file_matches\(&mut current_target[\s\S]*publish_stack_basic_text_staging/);
  assert.match(backend, /ReplaceFileW[\s\S]*REPLACEFILE_WRITE_THROUGH/);
  assert.match(backend, /share_mode\(FILE_SHARE_READ \| FILE_SHARE_DELETE\)/);
  assert.doesNotMatch(backend, /set_len\(0\)/);
});
