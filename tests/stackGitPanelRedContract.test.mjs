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
  assert.match(api, /export type StackGitRevertRequest = \{/);
  assert.match(api, /paths: string\[\];/);
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
  assert.match(panel, /class="stack-git-branch-selector"/);
  assert.match(panel, /class="stack-git-repository-menu"/);
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
  assert.match(panel, /class="stack-git-change-diff-drawer" role="region"[^>]*>\s*<pre>/);
  assert.doesNotMatch(panel, /class="stack-git-diff-header"/);
  assert.doesNotMatch(panel, /aria-label="Close diff"/);
  assert.match(panel, /aria-expanded=\{diffDrawerOpen && selectedChangePaths\.includes\(entry\.path\) && diffDrawerStaged\}/);
  assert.match(panel, /aria-expanded=\{diffDrawerOpen && selectedChangePaths\.includes\(entry\.path\) && !diffDrawerStaged\}/);
  assert.match(panel, /aria-pressed=\{diffDrawerOpen && selectedChangePaths\.includes\(entry\.path\) && diffDrawerStaged\}/);
  assert.match(panel, /aria-pressed=\{diffDrawerOpen && selectedChangePaths\.includes\(entry\.path\) && !diffDrawerStaged\}/);
  assert.match(panel, /handleEscape\(event: KeyboardEvent\)[\s\S]*if \(diffDrawerOpen\) \{[\s\S]*closeDiffDrawer\(\);[\s\S]*return;[\s\S]*\}[\s\S]*closePanel\(\);/);
  assert.match(panel, /\.stack-git-change-diff-drawer\s*\{[\s\S]*width:\s*100%;[\s\S]*animation:\s*stack-git-drawer-in 160ms/);
  assert.match(panel, /\.stack-git-panel-scroll\s*\{[^}]*overflow-x:\s*hidden;[^}]*overflow-y:\s*auto;[^}]*overscroll-behavior:\s*contain;/);
  assert.match(panel, /\.stack-git-change-diff-drawer\s*\{[^}]*box-sizing:\s*border-box;/);
  assert.match(panel, /\.stack-git-change-diff-drawer\s*\{[^}]*padding:\s*0 12px 12px 44px;/);
  assert.match(panel, /\.stack-git-change-diff-drawer pre\s*\{[^}]*overflow-x:\s*auto;[^}]*overflow-y:\s*visible;[^}]*white-space:\s*pre;/);
  const diffPreStyles = panel.match(/\.stack-git-change-diff-drawer pre\s*\{[^}]*\}/)?.[0] ?? '';
  assert.doesNotMatch(diffPreStyles, /max-height:/);
  assert.match(panel, /@media \(prefers-reduced-motion: reduce\)[\s\S]*\.stack-git-change-diff-drawer\s*\{[\s\S]*animation:\s*none;/);
  assert.doesNotMatch(panel, /stack-git-diff-drawer-backdrop/);
  assert.doesNotMatch(panel, /\{#if diffText \|\| diffLoading\}\s*<aside class="stack-git-diff-panel"/);
});
