import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { test } from 'node:test';

function readRepoFile(path) {
  const url = new URL(`../${path}`, import.meta.url);
  return existsSync(url) ? readFileSync(url, 'utf8') : '';
}

const commandsSource = readRepoFile('src/ipc/commands.ts');
const stackPopupApi = readRepoFile('src/lib/stackPopup.ts');
const stackPopupSurface = readRepoFile('src/components/StackPopupSurface.svelte');
const stackPopupCss = readRepoFile('src/components/StackPopupSurface.css');
const rustStackPopup = readRepoFile('src-tauri/src/stack_popup.rs');
const rustGitStatus = readRepoFile('src-tauri/src/stack_popup/git_status.rs');
const rustMain = readRepoFile('src-tauri/src/main.rs');
const rustContracts = readRepoFile('src-tauri/src/contracts.rs');
const masterSpec = readRepoFile('master_spec.md');

test('stack git status backend is a separate non-listing command using git porcelain', () => {
  assert.match(commandsSource, /getStackGitStatus:\s*'get_stack_git_status'/);
  assert.match(commandsSource, /stackGitAddPaths:\s*'stack_git_add_paths'/);
  assert.match(commandsSource, /stackGitCommit:\s*'stack_git_commit'/);
  assert.match(rustContracts, /GET_STACK_GIT_STATUS:\s*&str\s*=\s*"get_stack_git_status"/);
  assert.match(rustContracts, /STACK_GIT_ADD_PATHS:\s*&str\s*=\s*"stack_git_add_paths"/);
  assert.match(rustContracts, /STACK_GIT_COMMIT:\s*&str\s*=\s*"stack_git_commit"/);
  assert.match(rustStackPopup, /mod git_status;/);
  assert.match(rustStackPopup, /pub async fn get_stack_git_status\(/);
  assert.match(rustStackPopup, /pub async fn stack_git_add_paths\(/);
  assert.match(rustStackPopup, /pub async fn stack_git_commit\(/);
  assert.match(rustStackPopup, /git_status::stack_git_status_for_path_async\(path\)\.await/);
  assert.match(rustMain, /stack_popup::get_stack_git_status/);
  assert.match(rustMain, /stack_popup::stack_git_add_paths/);
  assert.match(rustMain, /stack_popup::stack_git_commit/);
  assert.match(rustGitStatus, /Command::new\("git"\)/);
  assert.match(rustGitStatus, /"--porcelain=v1"/);
  assert.match(rustGitStatus, /"-z"/);
  assert.match(rustGitStatus, /"rev-parse"/);
  assert.match(rustGitStatus, /tauri::async_runtime::spawn_blocking/);
  assert.match(rustGitStatus, /"--pathspec-from-file=-"/);
  assert.match(rustGitStatus, /"--pathspec-file-nul"/);
  assert.match(rustGitStatus, /"commit"/);
  assert.match(masterSpec, /Stack popup:[\s\S]*`get_stack_git_status`/);
});

test('stack popup API exposes typed git branch counts and per-path status entries', () => {
  assert.match(stackPopupApi, /export type StackGitFileStatusKind = 'modified' \| 'added' \| 'deleted' \| 'untracked' \| 'conflict';/);
  assert.match(stackPopupApi, /export type StackGitStatus = \{/);
  assert.match(stackPopupApi, /branch: string;/);
  assert.match(stackPopupApi, /conflicts: number;/);
  assert.match(stackPopupApi, /staged: boolean;/);
  assert.match(stackPopupApi, /entries: StackGitFileStatus\[\];/);
  assert.match(stackPopupApi, /export function getStackGitStatus\(folderPath: string\): Promise<StackGitStatus \| null>/);
  assert.match(stackPopupApi, /invoke<StackGitStatus \| null>\(IPC_COMMANDS\.getStackGitStatus/);
  assert.match(stackPopupApi, /export function stackGitAddPaths\(folderPath: string, paths: string\[\]\): Promise<StackGitOperationResult>/);
  assert.match(stackPopupApi, /export function stackGitCommit\(folderPath: string, message: string, paths: string\[\]\): Promise<StackGitOperationResult>/);
  assert.match(stackPopupApi, /request: \{ folderPath, message, paths \}/);
});

test('stack popup loads git status outside folder listing and guards stale responses', () => {
  assert.match(stackPopupSurface, /getStackGitStatus/);
  assert.match(stackPopupSurface, /let gitStatus: StackGitStatus \| null = null;/);
  assert.match(stackPopupSurface, /let gitStatusRequestSequence = 0;/);
  assert.match(stackPopupSurface, /void refreshStackGitStatus\(folderPath,\s*loadSequence\);/);
  assert.match(stackPopupSurface, /async function refreshStackGitStatus\(folderPath: string,\s*loadSequence: number\)/);
  assert.match(stackPopupSurface, /requestSequence !== gitStatusRequestSequence/);
  assert.match(stackPopupSurface, /loadSequence !== folderLoadSequence/);

  const loadFolderBody = stackPopupSurface.slice(
    stackPopupSurface.indexOf('async function loadFolder'),
    stackPopupSurface.indexOf('function startNewIconHydrationSession')
  );
  assert.match(loadFolderBody, /void refreshStackGitStatus\(folderPath,\s*loadSequence\);/);
  assert.match(loadFolderBody, /const listing = await listStackFolder/);

  const typedSubmitBody = stackPopupSurface.slice(
    stackPopupSurface.indexOf('async function submitPathDraft'),
    stackPopupSurface.indexOf('function resetPathDraft')
  );
  assert.match(typedSubmitBody, /stackState = commitValidatedStackFolderListing\(stackState, folderPath, listing\);[\s\S]*void refreshStackGitStatus\(folderPath,\s*loadSequence\);/);
});

test('stack popup renders branch summary and minimal row git badges', () => {
  assert.match(stackPopupSurface, /stack-git-summary/);
  assert.match(stackPopupSurface, /openGitStatusPopup\('all'\)/);
  assert.match(stackPopupSurface, /openGitStatusPopup\(part\.status\)/);
  assert.match(stackPopupSurface, /class="stack-git-popup"/);
  assert.match(stackPopupSurface, /stackGitAddPaths\(currentPath, gitStatusSelectedPaths\)/);
  assert.match(stackPopupSurface, /stackGitCommit\(currentPath, gitCommitMessage, stagedGitStatusEntries\(\)\.map\(\(entry\) => entry\.path\)\)/);
  assert.match(stackPopupSurface, /entry\.staged \? 'Staged'/);
  assert.doesNotMatch(stackPopupSurface, /role="tablist"/);
  assert.match(stackPopupSurface, /<button type="button" disabled=\{!filteredGitStatusEntries\(\)\.length \|\| gitOperationPending\} on:click=\{\(\) => setAllGitPathsSelected\(true\)\}>Select all<\/button>/);
  assert.match(stackPopupSurface, /<button type="button" disabled=\{!gitStatusSelectedPaths\.length \|\| gitOperationPending\} on:click=\{\(\) => setAllGitPathsSelected\(false\)\}>Clear<\/button>/);
  assert.match(stackPopupSurface, /<button type="button" disabled=\{!gitStatusSelectedPaths\.length \|\| gitOperationPending\} on:click=\{\(\) => void addSelectedGitPaths\(\)\}>Add selected<\/button>/);
  assert.match(stackPopupSurface, /<button type="submit" disabled=\{!gitCommitMessage\.trim\(\) \|\| gitOperationPending \|\| !stagedGitStatusEntries\(\)\.length\}>Commit<\/button>/);
  assert.match(stackPopupSurface, /\{gitStatusSelectedPaths\.length\} selected/);
  assert.match(stackPopupSurface, /<input type="checkbox" bind:group=\{gitStatusSelectedPaths\} value=\{entry\.path\} \/>/);
  assert.doesNotMatch(stackPopupSurface, /checked=\{isGitPathSelected\(entry\.path\)\}/);
  assert.match(stackPopupSurface, /Add selected/);
  assert.match(stackPopupSurface, /Commit message/);
  assert.match(stackPopupSurface, /stackGitSummaryParts\(gitStatus\)/);
  assert.match(stackPopupSurface, /stackGitStatusForEntry\(entry\)/);
  assert.match(stackPopupSurface, /data-git-status=\{gitEntryStatus \?\? undefined\}/);
  assert.match(stackPopupSurface, /git-status-badge/);
  assert.match(stackPopupSurface, /stackGitStatusLabel\(gitEntryStatus\)/);
});

test('stack git badges have distinct compact colors without changing row layout columns', () => {
  assert.match(stackPopupCss, /\.stack-git-summary/);
  assert.match(stackPopupCss, /\.git-status-badge/);
  for (const status of ['modified', 'added', 'deleted', 'untracked', 'conflict']) {
    assert.match(stackPopupCss, new RegExp(`\\.git-status-badge\\.git-status-${status}`));
  }
  assert.match(stackPopupCss, /box-shadow: inset 5px 0 0 #52f28a;/);
  assert.match(stackPopupCss, /box-shadow: inset 5px 0 0 #ffd84d;/);
  assert.match(stackPopupCss, /box-shadow: inset 5px 0 0 #ff6464;/);
  assert.match(stackPopupCss, /box-shadow: inset 5px 0 0 #8ab4ff;/);
  assert.match(stackPopupCss, /grid-template-columns: minmax\(10rem, 1fr\) 5\.5rem 5rem 8\.5rem;/);
});
