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

function stripRustTestBlocks(source) {
  return source.replace(/\n#\[cfg\(test\)\][\s\S]*$/m, '\n');
}

test('stack git status backend is a separate non-listing command using git porcelain', () => {
  const productionRustGitStatus = stripRustTestBlocks(rustGitStatus);
  assert.match(commandsSource, /getStackGitStatus:\s*'get_stack_git_status'/);
  assert.match(commandsSource, /stackGitAddPaths:\s*'stack_git_add_paths'/);
  assert.match(commandsSource, /stackGitCommit:\s*'stack_git_commit'/);
  assert.match(rustContracts, /GET_STACK_GIT_STATUS:\s*&str\s*=\s*"get_stack_git_status"/);
  assert.match(rustContracts, /OPEN_STACK_GIT_REMOTE_URL:\s*&str\s*=\s*"open_stack_git_remote_url"/);
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
  assert.match(productionRustGitStatus, /process_runner::run_process|run_process\(/);
  assert.match(productionRustGitStatus, /GIT_TIMEOUT_ENV_VAR/);
  assert.match(productionRustGitStatus, /spawn_blocking/);
  assert.match(productionRustGitStatus, /git_stdout_bytes/);
  assert.match(productionRustGitStatus, /"--porcelain=v1"/);
  assert.match(productionRustGitStatus, /"-z"/);
  assert.match(productionRustGitStatus, /"rev-parse"/);
  assert.doesNotMatch(productionRustGitStatus, /Command::new\("git"\)/);
  assert.match(productionRustGitStatus, /"--pathspec-from-file=-"/);
  assert.match(productionRustGitStatus, /"--pathspec-file-nul"/);
  assert.match(productionRustGitStatus, /"commit"/);
  assert.match(masterSpec, /Stack popup:[\s\S]*`get_stack_git_status`/);
});

test('stack popup API exposes typed git branch counts and per-path status entries', () => {
  assert.match(stackPopupApi, /export type StackGitFileStatusKind = 'modified' \| 'added' \| 'deleted' \| 'untracked' \| 'conflict';/);
  assert.match(stackPopupApi, /export type StackGitStatus = \{/);
  assert.match(stackPopupApi, /branch: string;/);
  assert.match(stackPopupApi, /remoteRepositoryUrl\?: string \| null;/);
  assert.match(commandsSource, /openStackGitRemoteUrl:\s*'open_stack_git_remote_url'/);
  assert.match(rustStackPopup, /pub fn open_stack_git_remote_url\(/);
  assert.match(rustMain, /stack_popup::open_stack_git_remote_url/);
  assert.match(stackPopupApi, /export function openStackGitRemoteUrl\(url: string\): Promise<void> \{/);
  assert.match(stackPopupApi, /invoke\(IPC_COMMANDS\.openStackGitRemoteUrl, \{ url \}\)/);
  assert.match(stackPopupApi, /conflicts: number;/);
  assert.match(stackPopupApi, /staged: boolean;/);
  assert.match(stackPopupApi, /entries: StackGitFileStatus\[\];/);
  assert.match(stackPopupApi, /export function getStackGitStatus\(folderPath: string\): Promise<StackGitStatus \| null>/);
  assert.match(stackPopupApi, /invoke<StackGitStatus \| null>\(IPC_COMMANDS\.getStackGitStatus/);
  assert.match(stackPopupApi, /export function stackGitAddPaths\(folderPath: string, paths: string\[\]\): Promise<StackGitOperationResult>/);
  assert.match(stackPopupApi, /export function stackGitCommit\(folderPath: string, message: string, paths: string\[\]\): Promise<StackGitOperationResult>/);
  assert.match(stackPopupApi, /request: \{ folderPath, message, paths \}/);
});

test('stack popup API exposes typed git workbench command wrappers ahead of backend wiring', () => {
  assert.match(stackPopupApi, /export type StackGitBranchOperationResult = StackGitOperationResult;/);
  assert.match(stackPopupApi, /export function stackGitFetch\(folderPath: string\): Promise<StackGitOperationResult>/);
  assert.match(stackPopupApi, /invoke<StackGitOperationResult>\(IPC_COMMANDS\.stackGitFetch/);
  assert.match(stackPopupApi, /export function stackGitPull\(folderPath: string\): Promise<StackGitOperationResult>/);
  assert.match(stackPopupApi, /invoke<StackGitOperationResult>\(IPC_COMMANDS\.stackGitPull/);
  assert.match(stackPopupApi, /export function stackGitPush\(folderPath: string\): Promise<StackGitOperationResult>/);
  assert.match(stackPopupApi, /invoke<StackGitOperationResult>\(IPC_COMMANDS\.stackGitPush/);
  assert.match(stackPopupApi, /export function stackGitCheckoutBranch\(folderPath: string, branchName: string\): Promise<StackGitBranchOperationResult>/);
  assert.match(stackPopupApi, /invoke<StackGitBranchOperationResult>\(IPC_COMMANDS\.stackGitCheckoutBranch/);
  assert.match(stackPopupApi, /request: \{ folderPath, branchName \}/);
  assert.match(stackPopupApi, /export function stackGitCreateBranch\(folderPath: string, branchName: string, checkout = true\): Promise<StackGitBranchOperationResult>/);
  assert.match(stackPopupApi, /invoke<StackGitBranchOperationResult>\(IPC_COMMANDS\.stackGitCreateBranch/);
  assert.match(stackPopupApi, /request: \{ folderPath, branchName, checkout \}/);
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
  assert.match(stackPopupSurface, /openGitRemoteRepository\(url: string \| null \| undefined\)/);
  assert.match(stackPopupSurface, /await openStackGitRemoteUrl\(url\)/);
  assert.doesNotMatch(stackPopupSurface, /window\.open/);
  assert.match(stackPopupSurface, /Git remote unavailable/);
  assert.match(stackPopupSurface, /gitStatus\.remoteRepositoryUrl/);
  assert.match(stackPopupSurface, /class="stack-git-remote-link"/);
  assert.match(stackPopupSurface, /aria-label="Open remote repository in browser"/);
  assert.match(stackPopupCss, /\.stack-git-summary \.stack-git-remote-link/);
  assert.match(stackPopupSurface, /openGitStatusPopup\('all'\)/);
  assert.match(stackPopupSurface, /openGitStatusPopup\(part\.status\)/);
  assert.match(stackPopupSurface, /class="stack-git-popup"/);
  assert.match(stackPopupSurface, /stackGitAddPaths\(currentPath, paths\)/);
  assert.match(stackPopupSurface, /stackGitCommit\(currentPath, message, paths\)/);
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

test('stack git status dialog is upgraded into a dense developer workbench', () => {
  assert.match(stackPopupSurface, /type StackGitWorkbenchView = 'changes' \| 'log' \| 'tree' \| 'branches';/);
  assert.match(stackPopupSurface, /let gitWorkbenchView: StackGitWorkbenchView = 'changes';/);
  assert.match(stackPopupSurface, /let gitWorkbenchExpanded = false;/);
  assert.match(stackPopupSurface, /class:expanded=\{gitWorkbenchExpanded\}/);
  assert.match(stackPopupSurface, /aria-label=\{gitWorkbenchExpanded \? 'Restore git workbench' : 'Expand git workbench'\}/);
  assert.match(stackPopupSurface, /class="stack-git-workbench"/);
  assert.match(stackPopupSurface, /class="stack-git-workbench-sidebar"/);
  assert.match(stackPopupSurface, /class="stack-git-workbench-main"/);
  assert.match(stackPopupSurface, /class="stack-git-workbench-views"/);
  for (const [view, label] of [['changes', 'Changes'], ['log', 'Log'], ['tree', 'Tree'], ['branches', 'Branches']]) {
    assert.match(stackPopupSurface, new RegExp(`setGitWorkbenchView\\('${view}'\\)[\\s\\S]*>${label}<`));
  }
  assert.match(stackPopupSurface, /class="stack-git-remote-actions"/);
  for (const action of ['Fetch', 'Pull', 'Push']) {
    assert.match(stackPopupSurface, new RegExp(`>${action}<`));
  }
  assert.match(stackPopupSurface, /class="stack-git-branch-controls"/);
  assert.match(stackPopupSurface, /Checkout branch/);
  assert.match(stackPopupSurface, /Create branch/);
  assert.match(stackPopupSurface, /stackGitFetch\(currentPath\)/);
  assert.match(stackPopupSurface, /stackGitPull\(currentPath\)/);
  assert.match(stackPopupSurface, /stackGitPush\(currentPath\)/);
  assert.match(stackPopupSurface, /stackGitLog\(folderPath, 80\)/);
  assert.match(stackPopupSurface, /stackGitTree\(folderPath, 'HEAD'\)/);
  assert.match(stackPopupSurface, /stackGitBranches\(folderPath\)/);
  assert.match(stackPopupSurface, /gitLog\.entries/);
  assert.match(stackPopupSurface, /gitTree\.entries/);
  assert.match(stackPopupSurface, /gitBranches\.branches/);
  assert.match(stackPopupSurface, /stackGitCheckoutBranch\(currentPath, branchName\)/);
  assert.match(stackPopupSurface, /stackGitCreateBranch\(currentPath, branchName, true\)/);
  assert.match(stackPopupSurface, /stagedGitStatusEntries\(\)\.length/);
  assert.match(stackPopupSurface, /filteredGitStatusEntries\(\)\.length/);
});

test('stack git workbench rejects stale async data and confirms mutating git commands', () => {
  assert.match(stackPopupSurface, /let pendingGitMutation:/);
  assert.match(stackPopupSurface, /pendingGitMutation = \{ kind: 'remote', operation \};/);
  assert.match(stackPopupSurface, /pendingGitMutation = \{ kind: 'add', paths: \[\.\.\.gitStatusSelectedPaths\] \};/);
  assert.match(stackPopupSurface, /kind: 'commit'/);
  assert.match(stackPopupSurface, /pendingGitMutation = \{ kind: 'checkout', branchName: gitBranchDraft\.trim\(\) \};/);
  assert.match(stackPopupSurface, /pendingGitMutation = \{ kind: 'createBranch', branchName: gitNewBranchDraft\.trim\(\) \};/);
  assert.match(stackPopupSurface, /function isCurrentGitWorkbenchResponse/);
  assert.match(stackPopupSurface, /requestedLoadSequence === folderLoadSequence/);
  assert.match(stackPopupSurface, /folderPath === stackState\.currentPath/);
  assert.match(stackPopupSurface, /gitStatus\?\.repositoryRoot === repositoryRoot/);
  assert.match(stackPopupSurface, /responseRepositoryRoot === repositoryRoot/);
  assert.match(stackPopupSurface, /operationErrorMessage\(error, `Git \$\{view\} load failed`\)/);
  assert.match(stackPopupSurface, /gitWorkbenchLoading = false;/);
  assert.match(stackPopupSurface, /class="git-confirm-dialog"/);
  assert.match(stackPopupSurface, /Confirm git/);
  assert.match(stackPopupSurface, /Add stages/);
  assert.match(stackPopupSurface, /Commit creates a new local commit/);
  assert.match(stackPopupSurface, /Push sends local commits/);
  assert.match(stackPopupSurface, /Pull updates files in this working tree/);
  assert.match(stackPopupSurface, /void confirmGitMutation\(\)/);
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

test('stack git workbench can resize or expand inside the stack browser surface without card nesting', () => {
  assert.match(stackPopupCss, /\.stack-git-popup \{/);
  assert.match(stackPopupCss, /resize: both;/);
  assert.match(stackPopupCss, /max-width: calc\(100% - 2rem\);/);
  assert.match(stackPopupCss, /max-height: calc\(100% - 2rem\);/);
  assert.match(stackPopupCss, /\.stack-git-popup\.expanded/);
  assert.match(stackPopupCss, /\.stack-git-workbench \{/);
  assert.match(stackPopupCss, /grid-template-columns: minmax\(11rem, 0\.34fr\) minmax\(0, 1fr\);/);
  assert.match(stackPopupCss, /\.stack-git-workbench-main \{/);
  assert.match(stackPopupCss, /\.stack-git-log-list/);
  assert.match(stackPopupSurface, /class="stack-git-log-entry"/);
  assert.match(stackPopupSurface, /class="stack-git-log-subject"/);
  assert.match(stackPopupSurface, /class="stack-git-log-meta"/);
  assert.match(stackPopupCss, /\.stack-git-workbench-main > \.stack-git-log-list/);
  assert.match(stackPopupCss, /grid-template-columns: minmax\(0, 1fr\) minmax\(12rem, 0\.44fr\);/);
  assert.match(stackPopupCss, /@container \(max-width: 42rem\)/);
  assert.match(stackPopupCss, /\.stack-git-tree-list/);
  assert.match(stackPopupCss, /\.stack-git-branch-panel/);
  assert.doesNotMatch(stackPopupCss, /\.stack-git-workbench[\s\S]*box-shadow: var\(--js-shadow-panel\)/);
  assert.doesNotMatch(stackPopupSurface, /class="stack-git-card"/);
});
