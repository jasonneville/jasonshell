import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { test } from 'node:test';

function readRepoFile(path) {
  const url = new URL(`../${path}`, import.meta.url);
  return existsSync(url) ? readFileSync(url, 'utf8') : '';
}

const commands = readRepoFile('src/ipc/commands.ts');
const api = readRepoFile('src/lib/stackPopup.ts');
const surface = readRepoFile('src/components/StackPopupSurface.svelte');
const panel = readRepoFile('src/components/StackGitPanel.svelte');
const panelState = readRepoFile('src/lib/stackGitPanelState.ts');
const models = readRepoFile('src-tauri/src/stack_popup/models.rs');
const stackPopupRs = readRepoFile('src-tauri/src/stack_popup.rs');
const gitStatusRs = readRepoFile('src-tauri/src/stack_popup/git_status.rs');
const mainRs = readRepoFile('src-tauri/src/main.rs');
const contracts = readRepoFile('src-tauri/src/contracts.rs');

function stripRustTestBlocks(source) {
  return source.replace(/\n#\[cfg\(test\)\][\s\S]*$/m, '\n');
}

const productionGitStatusRs = stripRustTestBlocks(gitStatusRs);

test('stack git API exposes no-AI diff, unstage, and destructive revert command contracts', () => {
  for (const [jsName, commandId] of [
    ['stackGitUnstagePaths', 'stack_git_unstage_paths'],
    ['stackGitRevertPaths', 'stack_git_revert_paths'],
    ['stackGitDiff', 'stack_git_diff']
  ]) {
    assert.match(commands, new RegExp(`${jsName}:\\s*'${commandId}'`));
    assert.match(api, new RegExp(`export function ${jsName}\\(`));
    assert.match(api, new RegExp(`IPC_COMMANDS\\.${jsName}`));
    assert.match(contracts, new RegExp(`pub const ${commandId.toUpperCase()}: &str = "${commandId}"`));
    assert.match(stackPopupRs, new RegExp(`pub async fn ${commandId}\\(`));
    assert.match(mainRs, new RegExp(`stack_popup::${commandId}`));
  }
  assert.match(api, /export type StackGitDiff = \{/);
  assert.match(api, /path: string;/);
  assert.match(api, /content: string/);
  assert.match(api, /export type StackGitCommitFiles = \{/);
  assert.match(api, /export type StackGitCommitFile = \{/);
  assert.match(api, /export type StackGitCommitFileDiff = \{/);
  assert.match(api, /export type StackGitRevertRequest = \{/);
  assert.match(api, /paths: string\[\];/);
});

test('stack git commit history exposes commit file list and per-file diff contracts', () => {
  for (const [jsName, commandId] of [
    ['stackGitCommitFiles', 'stack_git_commit_files'],
    ['stackGitCommitFileDiff', 'stack_git_commit_file_diff']
  ]) {
    assert.match(commands, new RegExp(`${jsName}:\\s*'${commandId}'`));
    assert.match(api, new RegExp(`export function ${jsName}\\(`));
    assert.match(api, new RegExp(`IPC_COMMANDS\\.${jsName}`));
    assert.match(contracts, new RegExp(`pub const ${commandId.toUpperCase()}: &str = "${commandId}"`));
    assert.match(stackPopupRs, new RegExp(`pub async fn ${commandId}\\(`));
    assert.match(mainRs, new RegExp(`stack_popup::${commandId}`));
  }
  assert.match(panel, /stackGitCommitFiles\(/);
  assert.match(panel, /stackGitCommitFileDiff\(/);
});

test('stack git status tracks staged and unstaged state plus optional ahead behind counts', () => {
  assert.match(api, /export type StackGitFileStatus = \{[\s\S]*staged: boolean;[\s\S]*unstaged: boolean;/);
  assert.match(api, /export type StackGitStatus = \{[\s\S]*ahead\?: number \| null;[\s\S]*behind\?: number \| null;/);
  assert.match(models, /pub struct StackGitFileStatus \{[\s\S]*pub staged: bool,[\s\S]*pub unstaged: bool,/);
  assert.match(models, /pub struct StackGitStatus \{[\s\S]*pub ahead: Option<usize>,[\s\S]*pub behind: Option<usize>,/);
  assert.match(gitStatusRs, /"rev-list"[\s\S]*"--left-right"[\s\S]*"--count"/);
});

test('extracted StackGitPanel owns OpenChamber-like no-AI sections and controls', () => {
  assert.ok(panel.length > 0, 'src/components/StackGitPanel.svelte must exist');
  assert.match(surface, /import StackGitPanel from '\.\/StackGitPanel\.svelte';/);
  assert.match(surface, /<StackGitPanel\b/);
  assert.match(panel, /class="stack-git-panel"/);
  for (const label of ['Changes', 'Staged', 'Unstaged', 'Diff', 'History', 'Branches']) {
    assert.match(panel, new RegExp(`>${label}<|aria-label="[^"]*${label}`));
  }
  for (const control of ['Stage', 'Unstage', 'Discard', 'Commit', 'Refresh', 'Fetch', 'Pull', 'Push']) {
    assert.match(panel, new RegExp(`>${control}<|aria-label="[^"]*${control}`));
  }
  assert.doesNotMatch(panel, /\bAI\b|assistant|chat|prompt|LLM|model/i);
});

test('stack git pure state groups paths and gates safe operations', async () => {
  const state = await import('../dist-tests/lib/stackGitPanelState.js');
  const entries = [
    { path: 'src/staged.ts', relativePath: 'src/staged.ts', status: 'modified', staged: true, unstaged: false },
    { path: 'src/unstaged.ts', relativePath: 'src/unstaged.ts', status: 'modified', staged: false, unstaged: true },
    { path: 'src/both.ts', relativePath: 'src/both.ts', status: 'modified', staged: true, unstaged: true },
    { path: 'src/new.ts', relativePath: 'src/new.ts', status: 'untracked', staged: false, unstaged: true }
  ];

  assert.deepEqual(state.groupStackGitEntries(entries), {
    staged: [entries[0], entries[2]],
    unstaged: [entries[1], entries[2], entries[3]],
    stagedCount: 2,
    unstagedCount: 3,
    totalCount: 5
  });
  assert.doesNotMatch(panelState, /stagedAndUnstaged/);
  assert.equal(state.canStageGitSelection([entries[1]]), true);
  assert.equal(state.canStageGitSelection([entries[2]]), true);
  assert.equal(state.canUnstageGitSelection([entries[0]]), true);
  assert.equal(state.canCommitGitStatus({ entries, conflicts: 0 }), true);
  assert.equal(state.canCommitGitStatus({ entries: entries.filter((entry) => !entry.staged), conflicts: 0 }), false);
  assert.equal(state.canCommitGitStatus({ entries, conflicts: 1 }), false);
  assert.equal(state.canDiscardGitSelection([entries[3]]), false, 'untracked discard must be blocked');
  assert.equal(state.canDiscardGitSelection([entries[1]]), true);
  assert.equal(state.canDiscardGitSelection([entries[2]]), true);
  assert.equal(state.canDiscardGitSelection([entries[0]]), false);
});

test('destructive discard requires explicit confirmation and never targets untracked files', () => {
  assert.match(panelState, /function confirmStackGitDiscard|export function confirmStackGitDiscard/);
  assert.match(panelState, /untracked[\s\S]*(return false|blocked|throw)/i);
  assert.match(panel, /confirmStackGitDiscard\(entries\)/);
  assert.match(panelState, /This reverts working-tree changes and cannot be undone\./);
  assert.match(panel, /stackGitRevertPaths/);
  assert.doesNotMatch(panel, /stackGitRevertPaths\([^)]*untracked/i);
});

test('stack git API exposes first-class history and stash command contracts without AI', () => {
  assert.match(panel, /History/);
  assert.match(panel, /Stashes/);
  assert.match(panel, /stackGitLog/);

  for (const [jsName, commandId] of [
    ['stackGitStashes', 'stack_git_stashes'],
    ['stackGitStash', 'stack_git_stash'],
    ['stackGitStashApply', 'stack_git_stash_apply'],
    ['stackGitStashPop', 'stack_git_stash_pop'],
    ['stackGitStashDrop', 'stack_git_stash_drop']
  ]) {
    assert.match(commands, new RegExp(`${jsName}:\\s*'${commandId}'`));
    assert.match(api, new RegExp(`export function ${jsName}\\(`));
    assert.match(api, new RegExp(`IPC_COMMANDS\\.${jsName}`));
    assert.match(contracts, new RegExp(`pub const ${commandId.toUpperCase()}: &str = "${commandId}"`));
    assert.match(stackPopupRs, new RegExp(`pub async fn ${commandId}\\(`));
    assert.match(mainRs, new RegExp(`stack_popup::${commandId}`));
  }

  assert.match(api, /export type StackGitStashEntry = \{/);
  assert.match(api, /ref: string;/);
  assert.match(api, /index: number;/);
  assert.match(api, /branch\?: string \| null;/);
  assert.match(api, /message: string;/);
  assert.match(api, /export type StackGitStashRequest = \{/);
  assert.match(api, /message\?: string;/);
  assert.match(api, /includeUntracked: boolean/);
  assert.doesNotMatch(panel, /\bAI\b|assistant|chat|prompt|LLM|model/i);
});

test('stack git history files and diffs use stale-safe loading, friendly status labels, and accessible disclosure', () => {
  assert.match(panel, /historyFilesLoading/);
  assert.match(panel, /historyFilesError/);
  assert.match(panel, /No files changed/);
  assert.match(panel, /No diff content\./);
  assert.match(panel, /aria-controls=\{`stack-git-history-files-\$\{entry\.commitHash\}`\}/);
  assert.match(panel, /aria-controls=\{`stack-git-history-diff-\$\{entry\.commitHash\}-\$\{fileIndex\}`\}/);
  assert.match(panel, /const token = \+\+diffToken;[\s\S]*token !== diffToken/);
  assert.match(panel, /commitFileStatusLabel\(file\.status\)/);
  assert.match(panel, /commitFileStatusClass\(file\.status\)/);
  assert.match(panel, /stack-git-path__dir/);
  assert.match(panel, /stack-git-path__name/);
  assert.match(panel, /handleEscape\(event: KeyboardEvent\)/);
  assert.doesNotMatch(panel, /ensureHistorySelection\(/);
  assert.doesNotMatch(panel, /diffDrawerHistory/);
});

test('stack git stash backend uses fixed argv, bounded message, explicit untracked flag, and spawn_blocking', () => {
  assert.match(productionGitStatusRs, /stack_git_stashes_async[\s\S]*spawn_blocking/);
  assert.match(productionGitStatusRs, /stack_git_stash_async[\s\S]*spawn_blocking/);
  assert.match(productionGitStatusRs, /stack_git_stash_apply_async[\s\S]*spawn_blocking/);
  assert.match(productionGitStatusRs, /stack_git_stash_pop_async[\s\S]*spawn_blocking/);
  assert.match(productionGitStatusRs, /stack_git_stash_drop_async[\s\S]*spawn_blocking/);
  assert.match(productionGitStatusRs, /validate_git_stash_ref\([^)]*\)/);
  assert.match(productionGitStatusRs, /stash@\{N\}|stash@\{\d\+\}|stash@\{[0-9]\+\}/);
  assert.match(productionGitStatusRs, /"stash"[\s\S]*"list"[\s\S]*"--format=/);
  assert.match(productionGitStatusRs, /"stash"[\s\S]*"push"[\s\S]*"-m"/);
  assert.match(productionGitStatusRs, /include_untracked[\s\S]*"--include-untracked"/);
  assert.match(productionGitStatusRs, /MAX_GIT_STASH_MESSAGE/);
  assert.match(productionGitStatusRs, /"stash"[\s\S]*"apply"[\s\S]*stash_ref/);
  assert.match(productionGitStatusRs, /"stash"[\s\S]*"pop"[\s\S]*stash_ref/);
  assert.match(productionGitStatusRs, /"stash"[\s\S]*"drop"[\s\S]*stash_ref/);
  assert.match(productionGitStatusRs, /ProcessRunSpec\s*\{/);
  assert.match(productionGitStatusRs, /program:\s*trusted_git_path\(\)[\s\S]*\.into_owned\(\)/);
  assert.match(productionGitStatusRs, /args:\s*std::iter::once\("-C"\.to_string\(\)\)/);
  assert.match(productionGitStatusRs, /run_process\(spec\)/);
  assert.doesNotMatch(productionGitStatusRs, /Command::new\("git"\)/);
});

test('StackGitPanel renders visible history and stash UI with destructive confirmations', () => {
  assert.match(panel, /type StackGit\w*View = 'changes' \| 'history' \| 'stashes' \| 'branches'/);
  assert.match(panel, /History/);
  assert.match(panel, /Stashes/);
  assert.match(panel, /gitLog\.entries|stackGitLog\(/);
  assert.match(panel, /stackPopup\.stackGitStashes\(/);
  assert.match(panel, /stackPopup\.stackGitStash\(/);
  assert.match(panel, /includeUntracked|stashMessage/);
  assert.match(panel, /Apply|stackGitStashApply/);
  assert.match(panel, /Pop|stackGitStashPop/);
  assert.match(panel, /Drop|stackGitStashDrop/);
  assert.match(panel, /title: 'Pop stash\?'/);
  assert.match(panel, /title: 'Drop stash\?'/);
  assert.match(panel, /Pop applies the stash and removes it from the list\./);
  assert.match(panel, /Drop removes this stash permanently\./);
});

test('stack git commit push failure clears stale success and stash button follows dirty tree', () => {
  assert.match(panel, /statusMessage = '';/);
  assert.match(panel, /stackGitStash\(folderPath, stashMessage\.trim\(\) \|\| 'WIP', status\.entries\.some\(/);
  assert.match(panel, /disabled=\{!status \|\| operationBusy \|\| !hasDirtyWorkingTree\}/);
});

test('Stack Browser navigation shortcuts never capture Backspace from editable controls', () => {
  assert.match(surface, /function isEditableKeyTarget\(target: EventTarget \| null\)/);
  assert.match(surface, /HTMLInputElement|HTMLTextAreaElement/);
  assert.match(surface, /isContentEditable/);
  assert.match(surface, /if \(gitStatusPopupOpen \|\| isEditableKeyTarget\(event\.target\)\) \{\s*return;\s*\}/);
  assert.match(panel, /id="stack-git-commit-message"[\s\S]*on:keydown\|stopPropagation/);
});

test('StackGitPanel uses OpenChamber Git display geometry instead of card tabs and split panes', () => {
  const headerBlock = panel.match(/<div class="stack-git-panel-row stack-git-panel-row--secondary">[\s\S]*?<\/header>/)?.[0] ?? '';
  assert.match(panel, /class="stack-git-branch-selector"/);
  assert.doesNotMatch(panel, /class="stack-git-repository-menu"/);
  assert.doesNotMatch(panel, /<details[^>]*>\s*<summary[^>]*aria-label="Repository views"/);
  assert.match(headerBlock, /<div[^>]*role="tablist"/);
  assert.match(headerBlock, /role="tab"/);
  assert.match(headerBlock, /aria-selected=/);
  assert.match(headerBlock, /Fetch[\s\S]*Pull[\s\S]*Push[\s\S]*Changes[\s\S]*History[\s\S]*Stashes[\s\S]*Branches/);
  assert.match(panel, /let activeView: StackGitView = 'changes';/);
  assert.match(panel, /class="stack-git-change-group-header"/);
  assert.match(panel, /class="stack-git-change-row/);
  assert.match(panel, /rows="1"/);
  assert.match(panel, /min-height:\s*38px/);
  assert.match(panel, /height:\s*34px/);
  assert.match(panel, /width:\s*32px/);
  assert.match(panel, /padding:\s*8px 12px/);
  assert.doesNotMatch(panel, /class="stack-git-panel-tabs"/);
  assert.doesNotMatch(panel, /grid-template-columns:\s*minmax\([^;]+\)\s+minmax\(/);
  assert.doesNotMatch(panel, /inset:\s*var\(--js-space-4\)/);
});

test('StackGitPanel replaces the file grid instead of overlaying it', () => {
  assert.match(surface, /\{#if gitStatusPopupOpen\}[\s\S]*<StackGitPanel\b[\s\S]*\{:else\}[\s\S]*class="details-table"/);
  assert.match(panel, /\.stack-git-panel\s*\{[^}]*background:\s*var\(--js-bg-surface\)/);
  assert.doesNotMatch(panel, /\.stack-git-panel\s*\{[^}]*position:\s*absolute/);
  assert.match(surface, /function handleBackgroundContextMenu\(event: MouseEvent\) \{\s*if \(gitStatusPopupOpen \|\|/);
  assert.match(surface, /function handleKeydown\(event: KeyboardEvent\) \{\s*if \(gitStatusPopupOpen \|\|/);
});

test('StackGitPanel branch button opens a grouped branch dropdown, not the Branches view', () => {
  const headerBlock = panel.match(/<header class="stack-git-panel-header">[\s\S]*?<\/header>/)?.[0] ?? '';
  const branchSelectorBlock = headerBlock.match(/<button[\s\S]*?class="stack-git-branch-selector"[\s\S]*?<\/button>/)?.[0] ?? '';
  assert.match(branchSelectorBlock, /aria-expanded=\{branchDropdownOpen\}/);
  assert.match(branchSelectorBlock, /aria-controls="stack-git-branch-dropdown"/);
  assert.match(branchSelectorBlock, /on:click=\{toggleBranchDropdown\}/);
  assert.doesNotMatch(branchSelectorBlock, /handleTabChange\('branches'\)/);
  assert.match(panel, /let branchDropdownOpen = false;/);
  assert.match(panel, /function toggleBranchDropdown\(\)[\s\S]*branchDropdownOpen = !branchDropdownOpen;[\s\S]*void refreshBranches\(\);/);
  assert.match(panel, /id="stack-git-branch-dropdown"[\s\S]*aria-label="Branch picker"/);
  assert.match(panel, /class="stack-git-branch-dropdown"/);
  assert.match(panel, />Local branches</);
  assert.match(panel, />Remote branches</);
  assert.match(panel, /localBranches = branches\.filter\(\(branch\) => !branch\.remote\)/);
  assert.match(panel, /remoteBranches = branches\.filter\(\(branch\) => branch\.remote\)/);
});

test('StackGitPanel branch dropdown rows checkout and create branch from selected source ref', () => {
  assert.match(panel, /\{#each localBranches as branch/);
  assert.match(panel, /\{#each remoteBranches as branch/);
  assert.match(panel, /on:click=\{\(\) => void checkoutBranch\(branch\.name\)\}/);
  assert.match(panel, /class="[^"]*stack-git-branch-dropdown__new-branch[^"]*"/);
  assert.match(panel, /on:submit\|preventDefault=\{\(\) => void createBranch\(\)\}/);
  assert.match(panel, /bind:value=\{newBranchSource\}/);
  assert.match(panel, /\{#each sourceBranches as branch \(branch\.name\)\}/);
  assert.match(panel, /let newBranchSource = '';/);
  assert.match(panel, /sourceBranches = \[\.\.\.localBranches, \.\.\.remoteBranches\]/);
  assert.match(panel, /stackGitCreateBranch\(folderPath, name, true, newBranchSource \|\| undefined\)/);
  assert.match(panel, /stackGitCreateBranch\(folderPath, action\.branchName, true, action\.sourceBranch\)/);
  assert.match(panel, /sourceBranch\?: string;/);
  assert.match(panel, /<form class="stack-git-branch-form" on:submit\|preventDefault=\{\(\) => void createBranch\(\)\}>[\s\S]*<span>Source branch<\/span>[\s\S]*bind:value=\{newBranchSource\}/);
  assert.match(panel, /let branchLoading = false;/);
  assert.match(panel, /let branchToken = 0;/);
  assert.match(panel, /const token = \+\+branchToken;/);
  assert.match(panel, /aria-busy=\{statusLoading \|\| viewLoading \|\| branchLoading \|\| diffLoading \? 'true' : 'false'\}/);
});

test('StackGitPanel marks local branches checked out in another worktree', () => {
  assert.match(panel, /branch\.checkedOutElsewhere/);
  assert.match(panel, />Other worktree</);
  assert.match(panel, /Checked out in another worktree:/);
  assert.match(panel, /branch\.checkedOutElsewherePath/);
  assert.match(panel, /on:click=\{\(\) => void checkoutBranch\(branch\.name\)\}/);
  assert.match(panel, /data-stack-git-branch-delete=\{branch\.name\}/);
});

test('stack git branches checkout prefers local branch for matching remote tail', () => {
  assert.match(panel, /function matchLocalBranchName\(branchName: string\)/);
  assert.ok(panel.includes("const tail = branchName.replace(/^(?:refs\\/)?remotes\\/[^/]+\\//, '').replace(/^refs\\/heads\\//, '');"));
  assert.match(panel, /normalizeBranchLabel\(branch\) === tail/);
  assert.match(panel, /branch = matchLocalBranchName\(branch\);[\s\S]*stackPopup\.stackGitCheckoutBranch\(folderPath, branch\)/);
});

test('stack git checkout refreshes changed files and resets dropdown state to the checked out branch', () => {
  assert.match(panel, /async function refreshBranches\(resetSelection = false\)/);
  assert.match(panel, /if \(resetSelection\) \{[\s\S]*selectedBranchName = next\.currentBranch \?\? status\.branch \?\? branches\[0\]\?\.name \?\? '';[\s\S]*newBranchSource = selectedBranchName;/);
  assert.match(panel, /async function refreshAfterMutation\(refreshBranchState = false\) \{[\s\S]*await refreshStatus\(\);[\s\S]*if \(refreshBranchState\) await refreshBranches\(true\);[\s\S]*onRefresh\?\.\(\);/);
  assert.match(panel, /stackPopup\.stackGitCheckoutBranch\(folderPath, branch\)[\s\S]*await refreshAfterMutation\(true\);/);
  assert.match(panel, /if \(resetSelection && next\.currentBranch && status\) \{[\s\S]*status = \{ \.\.\.status, branch: next\.currentBranch \};[\s\S]*\}/);
});

test('stack git dropdown current branch uses the same live status source as the header label', () => {
  assert.match(panel, /\$: currentBranchLabel = status\?\.branch \?\? 'Detached';/);
  assert.match(panel, /stack-git-branch-selector__label">\{currentBranchLabel\}<\/span>/);
  assert.doesNotMatch(panel, /\{currentBranchLabel\(\)\}/);
  assert.match(panel, /function isCurrentBranch\(branch: StackGitBranches\['branches'\]\[number\], currentBranch: string\) \{[\s\S]*normalizeBranchLabel\(branch\) === currentBranch/);
  assert.match(panel, /class:current=\{isCurrentBranch\(branch, currentBranchLabel\)\}/);
  assert.match(panel, /function applyCheckedOutBranch\(branchName: string\)[\s\S]*status = \{ \.\.\.status, branch: branchName \};[\s\S]*selectedBranchName = branchName;[\s\S]*newBranchSource = branchName;/);
  assert.match(panel, /stackPopup\.stackGitCheckoutBranch\(folderPath, branch\)[\s\S]*applyCheckedOutBranch\(branch\);[\s\S]*await refreshAfterMutation\(true\);/);
});

test('stack git dropdown confirms local branch deletion and protects current and remote branches', () => {
  assert.match(panel, /type StackGitConfirmKind = [^;]*'delete-branch'/);
  assert.match(panel, /function deleteLocalBranch\(branch: StackGitBranches\['branches'\]\[number\]\)/);
  assert.match(panel, /function operationErrorMessage\(error: unknown, fallback: string\)/);
  assert.match(panel, /typeof error === 'string' && error\.trim\(\)/);
  assert.match(panel, /error instanceof Error && error\.message/);
  assert.match(panel, /if \(branch\.remote \|\| isCurrentBranch\(branch, currentBranchLabel\) \|\| operationBusy\) return;/);
  assert.match(panel, /kind: 'delete-branch'/);
  assert.match(panel, /Delete local branch/);
  assert.match(panel, /bind:this=\{branchPickerButton\}/);
  assert.match(panel, /data-stack-git-branch-delete=\{branch\.name\}/);
  assert.match(panel, /branchDeleteInProgress = true;/);
  assert.match(panel, /branchDeleteInProgress = false;/);
  assert.match(panel, /await stackPopup\.stackGitDeleteBranch\(folderPath, action\.branchName, action\.force \?\? false, action\.removeWorktree \?\? false, action\.worktreePath\)/);
  assert.match(panel, /force: true/);
  assert.match(panel, /branch delete error|deleteError|Git branch deletion failed|not fully merged/);
  assert.match(panel, /aria-label=\{`Delete local branch \$\{normalizeBranchLabel\(branch\)\}`\}/);
  assert.match(panel, /class="stack-git-branch-delete"/);
  assert.match(panel, /await focusBranchPickerButton\(\);/);
});

test('linked-worktree branch deletion confirms once then removes optimistically and forces backend cleanup', () => {
  assert.match(panel, /function deleteLocalBranch\([\s\S]*force: true/);
  assert.match(panel, /removeWorktree: branch\.checkedOutElsewhere/);
  assert.match(panel, /worktreePath: branch\.checkedOutElsewherePath/);
  assert.match(panel, /branch\.checkedOutElsewherePath/);
  assert.match(panel, /action\.removeWorktree \?\? false/);
  assert.match(panel, /branches = branches\.filter\(\(branch\) => branch\.name !== action\.branchName\)/);
  assert.match(panel, /permanently remove its linked worktree directory/);
  assert.match(panel, /uncommitted worktree changes/i);
  assert.match(panel, /unmerged branch commits/i);
  assert.match(panel, /catch \(error\) \{[\s\S]*await refreshBranches\(true\);/);
  assert.doesNotMatch(panel, /function forceDeleteLocalBranch/);
  assert.doesNotMatch(panel, /function removeWorktreeAndDeleteLocalBranch/);
});

test('stack git create branch frontend contract forwards optional source branch', () => {
  assert.match(api, /export function stackGitCreateBranch\(folderPath: string, branchName: string, checkout = true, sourceBranch\?: string\): Promise<StackGitBranchOperationResult>/);
  assert.match(api, /request: \{ folderPath, branchName, checkout, sourceBranch \}/);
});

test('StackGitPanel inserts staged and unstaged diff drawers directly below the clicked row', () => {
  const selectChangePathBlock = panel.match(/function selectChangePath\([\s\S]*?\n  }/)?.[0] ?? '';
  const openChangeDiffBlock = panel.match(/function openChangeDiff\([\s\S]*?\n  }/)?.[0] ?? '';
  const handleTabChangeBlock = panel.match(/function handleTabChange\([\s\S]*?\n  }/)?.[0] ?? '';
  const closeDiffDrawerBlock = panel.match(/function closeDiffDrawer\([\s\S]*?\n  }/)?.[0] ?? '';
  const refreshStatusBlock = panel.match(/async function refreshStatus\([\s\S]*?\n  }/)?.[0] ?? '';
  const loadDiffBlock = panel.match(/async function loadDiff\([\s\S]*?\n  }/)?.[0] ?? '';
  assert.match(panel, /let diffDrawerOpen = false;/);
  assert.match(panel, /let diffDrawerStaged = false;/);
  assert.match(openChangeDiffBlock, /if \(diffDrawerOpen && selectedChangePaths\.includes\(entry\.path\) && diffDrawerStaged === staged\) \{\s*closeDiffDrawer\(\);\s*return;/);
  assert.match(openChangeDiffBlock, /diffDrawerOpen = true;/);
  assert.match(openChangeDiffBlock, /diffDrawerStaged = staged;/);
  assert.match(openChangeDiffBlock, /void loadDiff\(entry\.path, staged, entry\.status\)/);
  assert.doesNotMatch(selectChangePathBlock, /refreshChangesDiff\(/);
  assert.match(handleTabChangeBlock, /if \(activeView === 'changes' && view !== 'changes'\) closeDiffDrawer\(\);/);
  assert.match(handleTabChangeBlock, /closeDiffDrawer\(\);\s*activeView = view;/);
  assert.match(closeDiffDrawerBlock, /diffToken \+= 1;/);
  assert.match(closeDiffDrawerBlock, /diffLoading = false;/);
  assert.match(refreshStatusBlock, /const drawerRowExists = nextStatus\.entries\.some/);
  assert.match(refreshStatusBlock, /if \(drawerRowExists\) void refreshChangesDiff\(\);\s*else closeDiffDrawer\(\);/);
  assert.match(loadDiffBlock, /const requestFolderPath = folderPath;/);
  assert.match(loadDiffBlock, /token !== diffToken \|\| requestFolderPath !== folderPath/);
  assert.match(panel, /entry\.path === selectedChangePaths\[0\] && diffDrawerStaged[\s\S]*class="stack-git-change-diff-drawer"[\s\S]*role="region"/);
  assert.match(panel, /entry\.path === selectedChangePaths\[0\] && !diffDrawerStaged[\s\S]*class="stack-git-change-diff-drawer"[\s\S]*role="region"/);
  assert.match(panel, /class="stack-git-change-diff-drawer" role="region"[^>]*>\s*<pre class="stack-git-diff-view"/);
  assert.match(panel, /stack-git-diff-line--\$\{rendered\.kind\}|data-kind=\{rendered\.kind\}/);
  assert.match(panel, /stack-git-diff-line__prefix/);
  assert.match(panel, /stack-git-diff-line__body/);
  assert.match(panel, /stack-git-diff-line__meta/);
  assert.doesNotMatch(panel, /\{index < diffLines\.length - 1 \? '\\n' : ''\}/);
  assert.match(panel, /const maxRenderedDiffLines = 4_000;/);
  assert.match(panel, /Diff truncated after \$\{maxRenderedDiffLines\.toLocaleString\(\)\} lines/);
  assert.match(panel, /lines\.slice\(0, maxRenderedDiffLines\)/);
  assert.doesNotMatch(panel, /class="stack-git-diff-header"/);
  assert.doesNotMatch(panel, /aria-label="Close diff"/);
  assert.match(panel, /aria-expanded=\{diffDrawerOpen && selectedChangePaths\.includes\(entry\.path\) && diffDrawerStaged\}/);
  assert.match(panel, /aria-expanded=\{diffDrawerOpen && selectedChangePaths\.includes\(entry\.path\) && !diffDrawerStaged\}/);
  assert.match(panel, /aria-pressed=\{diffDrawerOpen && selectedChangePaths\.includes\(entry\.path\) && diffDrawerStaged\}/);
  assert.match(panel, /aria-pressed=\{diffDrawerOpen && selectedChangePaths\.includes\(entry\.path\) && !diffDrawerStaged\}/);
  assert.match(panel, /handleEscape\(event: KeyboardEvent\)[\s\S]*if \(diffDrawerOpen\) \{[\s\S]*closeDiffDrawer\(\);[\s\S]*return;[\s\S]*\}[\s\S]*closePanel\(\);/);
  const drawerStyleBlock = panel.match(/\.stack-git-change-diff-drawer\s*\{[\s\S]*?\.stack-git-stream-header h3\s*\{/ )?.[0] ?? '';
  const drawerPreBlock = panel.match(/\.stack-git-change-diff-drawer pre\s*\{[\s\S]*?\n  \}/)?.[0] ?? '';
  const lineStyleBlock = panel.match(/\.stack-git-diff-line\s*\{[\s\S]*?\.stack-git-commit\s*\{/ )?.[0] ?? '';
  const bareActionStyles = panel.match(/\.stack-git-panel button\.stack-git-change-group-header__bulk,[\s\S]*?button\.stack-git-change-row__discard:focus-visible\s*\{[\s\S]*?\n  \}/)?.[0] ?? '';
  const changeRowStyles = panel.match(/\.stack-git-change-row\s*\{[\s\S]*?\.stack-git-change-row__status\s*\{/ )?.[0] ?? '';
  const changeHeaderStyles = panel.match(/\.stack-git-change-group-header\s*\{[\s\S]*?\n  \}/)?.[0] ?? '';
  assert.match(drawerStyleBlock, /background:\s*var\(--js-color-surface\);/);
  assert.match(drawerStyleBlock, /border-left:\s*2px solid var\(--js-color-accent-border\);/);
  assert.match(drawerStyleBlock, /font:\s*0\.67rem\/1\.45 ui-monospace, "Cascadia Mono", Consolas, monospace;/);
  assert.match(drawerStyleBlock, /overflow:\s*auto;/);
  assert.match(drawerStyleBlock, /padding:\s*0\.45rem;/);
  assert.match(drawerStyleBlock, /user-select:\s*text;/);
  assert.match(drawerStyleBlock, /white-space:\s*pre-wrap;/);
  assert.match(drawerStyleBlock, /cursor:\s*text;/);
  assert.match(drawerPreBlock, /background:\s*transparent;/);
  assert.match(drawerPreBlock, /border:\s*0;/);
  assert.match(drawerPreBlock, /font:\s*inherit;/);
  assert.match(drawerPreBlock, /overflow:\s*auto;/);
  assert.match(drawerPreBlock, /white-space:\s*inherit;/);
  assert.match(lineStyleBlock, /border-left:\s*3px solid transparent;/);
  assert.match(lineStyleBlock, /grid-template-columns:\s*1\.25ch minmax\(0, 1fr\);/);
  assert.match(lineStyleBlock, /padding:\s*0\.06rem 0\.35rem 0\.06rem 0\.45rem;/);
  assert.doesNotMatch(panel, /background:\s*#06080b/);
  assert.match(lineStyleBlock, /\.stack-git-diff-line--addition\s*\{[\s\S]*background:\s*var\(--js-color-success\);[\s\S]*border-left-color:\s*var\(--js-color-success-border\);[\s\S]*color:\s*var\(--js-color-text-strong\);/);
  assert.match(lineStyleBlock, /\.stack-git-diff-line--deletion\s*\{[\s\S]*background:\s*var\(--js-color-error\);[\s\S]*border-left-color:\s*var\(--js-color-error-border\);[\s\S]*color:\s*var\(--js-color-text-strong\);/);
  assert.match(lineStyleBlock, /\.stack-git-diff-line--hunk\s*\{[\s\S]*background:\s*var\(--js-color-warning\);[\s\S]*border-left-color:\s*var\(--js-color-warning-border\);[\s\S]*color:\s*var\(--js-color-text-strong\);/);
  assert.match(lineStyleBlock, /\.stack-git-diff-line--addition \.stack-git-diff-line__prefix[\s\S]*color-mix\(in srgb, var\(--js-color-success-border\) 75%, var\(--js-color-text-strong\)\)/);
  assert.match(bareActionStyles, /appearance:\s*none;/);
  assert.match(bareActionStyles, /background:\s*transparent;/);
  assert.match(bareActionStyles, /border:\s*0;/);
  assert.match(bareActionStyles, /color:\s*inherit;/);
  assert.match(bareActionStyles, /min-height:\s*0;/);
  assert.match(bareActionStyles, /:hover:not\(:disabled\)[\s\S]*background:\s*transparent;[\s\S]*color:\s*inherit;/);
  assert.match(bareActionStyles, /:focus-visible[\s\S]*box-shadow:\s*var\(--js-focus-ring\);/);
  assert.match(changeRowStyles, /background:\s*color-mix\(in srgb, var\(--js-color-surface-overlay\) 68%, transparent\);/);
  assert.match(changeRowStyles, /\.stack-git-change-row:hover,[\s\S]*\.stack-git-change-row:focus-within[\s\S]*background:\s*color-mix\(in srgb, var\(--js-color-control-hover\) 84%, var\(--js-color-accent-border\)\);/);
  assert.match(changeRowStyles, /\.stack-git-panel button\.stack-git-change-row__content,[\s\S]*background:\s*transparent;/);
  assert.match(changeHeaderStyles, /padding:\s*8px;/);
  assert.match(changeRowStyles, /padding:\s*0 8px;/);
  assert.doesNotMatch(panel, /stack-git-diff-line__prefix" aria-hidden="true"/);
  const diffPreStyles = panel.match(/\.stack-git-change-diff-drawer pre\s*\{[^}]*\}/)?.[0] ?? '';
  assert.doesNotMatch(diffPreStyles, /max-height:/);
  assert.doesNotMatch(drawerStyleBlock, /linear-gradient\(180deg,/);
  assert.doesNotMatch(drawerStyleBlock, /box-shadow:\s*inset 0 1px 0/);
  assert.match(panel, /@media \(prefers-reduced-motion: reduce\)[\s\S]*\.stack-git-change-diff-drawer\s*\{[\s\S]*animation:\s*none;/);
  assert.match(panel, /@container \(max-width: 32rem\)/);
  assert.doesNotMatch(panel, /stack-git-diff-drawer-backdrop/);
  assert.doesNotMatch(panel, /\{#if diffText \|\| diffLoading\}\s*<aside class="stack-git-diff-panel"/);
});
