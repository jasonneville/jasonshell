import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { test } from 'node:test';
import ts from 'typescript';

function readRepoFile(path) {
  const url = new URL(`../${path}`, import.meta.url);
  return existsSync(url) ? readFileSync(url, 'utf8') : '';
}

const panel = readRepoFile('src/components/StackGitPanel.svelte');
const popup = readRepoFile('src/components/StackPopupSurface.svelte');
const dialog = readRepoFile('src/components/StackConfirmDialog.svelte');

async function importPanelHelpers() {
  const source = panel.match(/<script context="module" lang="ts">([\s\S]*?)<\/script>/)?.[1] ?? '';
  const javascript = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 }
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
}

test('gitignore hierarchy normalizes separators, excludes repo root, and defaults exact file', async () => {
  const { stackGitIgnoreChoices, canStackGitIgnoreEntry } = await importPanelHelpers();
  assert.deepEqual(stackGitIgnoreChoices('C:\\repo', 'src\\generated/out.log'), [
    { relativePath: 'src', absolutePath: 'C:\\repo\\src', kind: 'folder' },
    { relativePath: 'src/generated', absolutePath: 'C:\\repo\\src\\generated', kind: 'folder' },
    { relativePath: 'src/generated/out.log', absolutePath: 'C:\\repo\\src\\generated\\out.log', kind: 'file' }
  ]);
  assert.equal(canStackGitIgnoreEntry({ status: 'untracked', unstaged: true }), true);
  assert.equal(canStackGitIgnoreEntry({ status: 'untracked', unstaged: false }), false);
  assert.equal(canStackGitIgnoreEntry({ status: 'modified', unstaged: true }), false);
});

test('untracked Changes rows expose isolated right-click ignore flow and authoritative refresh', () => {
  assert.match(panel, /on:contextmenu\|preventDefault\|stopPropagation=.*openGitIgnoreMenu/);
  assert.match(panel, />Add to \.gitignore</);
  assert.match(panel, /selectedGitIgnorePath = choices\[choices\.length - 1\]\.absolutePath/);
  assert.match(panel, /\.stackGitIgnorePath\(folderPath, selectedGitIgnorePath\)/);
  assert.match(panel, /await refreshAfterMutation\(\)/);
  assert.match(panel, /window\.innerWidth[\s\S]*window\.innerHeight/);
});

test('shared Stack confirmation dialog owns modal a11y and both Stack surfaces use it', () => {
  assert.ok(dialog, 'shared StackConfirmDialog must exist');
  assert.match(dialog, /role="alertdialog"/);
  assert.match(dialog, /aria-modal="true"/);
  assert.match(dialog, /event\.key !== 'Tab'/);
  assert.match(dialog, /event\.key === 'Escape'/);
  assert.match(dialog, /initialFocus/);
  assert.match(dialog, /returnFocus/);
  assert.match(panel, /<StackConfirmDialog/);
  assert.match(popup, /<StackConfirmDialog/);
  assert.doesNotMatch(panel, /class="stack-git-confirm-dialog"/);
  assert.doesNotMatch(popup, /class="delete-confirm-dialog"/);
});

test('shared confirmation focus trap discovers visible enabled descendants in DOM order', () => {
  assert.match(dialog, /dialog\.querySelectorAll<HTMLElement>\(/);
  assert.match(dialog, /input:not\(\[type="hidden"\]\)/);
  assert.match(dialog, /control\.disabled/);
  assert.match(dialog, /element\.matches\(':disabled'\)/);
  assert.match(dialog, /element\.getClientRects\(\)\.length/);
  assert.doesNotMatch(dialog, /const focusables = \[cancelButton, confirmButton\]/);
  assert.match(panel, /<input type="radio" name="stack-git-ignore-target"/);
});
