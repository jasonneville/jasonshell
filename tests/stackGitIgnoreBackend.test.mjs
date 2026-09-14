import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const commandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const facadeSource = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');
const contractsSource = readFileSync(new URL('../src-tauri/src/contracts.rs', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const stackPopupSource = readFileSync(new URL('../src-tauri/src/stack_popup.rs', import.meta.url), 'utf8');
const modelsSource = readFileSync(new URL('../src-tauri/src/stack_popup/models.rs', import.meta.url), 'utf8');
const gitStatusSource = readFileSync(new URL('../src-tauri/src/stack_popup/git_status.rs', import.meta.url), 'utf8');

test('git-ignore mutation is registered across frontend and backend IPC contracts', () => {
  assert.match(commandsSource, /stackGitIgnorePath:\s*'stack_git_ignore_path'/);
  assert.match(contractsSource, /STACK_GIT_IGNORE_PATH:\s*&str\s*=\s*"stack_git_ignore_path"/);
  assert.match(contractsSource, /STACK_GIT_IGNORE_PATH,/);
  assert.match(mainSource, /stack_popup::stack_git_ignore_path,/);
});

test('frontend wrapper sends fixed request model', () => {
  assert.match(
    facadeSource,
    /export function stackGitIgnorePath\(folderPath: string, path: string\): Promise<StackGitOperationResult>[\s\S]*?IPC_COMMANDS\.stackGitIgnorePath,[\s\S]*?request:\s*\{ folderPath, path \}/
  );
  assert.match(modelsSource, /pub struct StackGitIgnorePathRequest\s*\{\s*pub folder_path: String,\s*pub path: String,\s*\}/);
});

test('backend facade authorizes stack-popup and delegates to async blocking implementation', () => {
  assert.match(
    stackPopupSource,
    /pub async fn stack_git_ignore_path[\s\S]*?STACK_GIT_IGNORE_PATH[\s\S]*?callers:\s*&\[crate::shell_windows::STACK_POPUP_LABEL\][\s\S]*?stack_git_ignore_path_async\(request\)\.await/
  );
  assert.match(
    gitStatusSource,
    /pub\(crate\) async fn stack_git_ignore_path_async[\s\S]*?spawn_blocking\(move \|\| stack_git_ignore_path\(request\)\)/
  );
});
