import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const css = readFileSync(new URL('../src/components/StackPopupSurface.css', import.meta.url), 'utf8');
const api = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');
const models = readFileSync(new URL('../src-tauri/src/stack_popup/models.rs', import.meta.url), 'utf8');
const paging = readFileSync(new URL('../src-tauri/src/stack_popup/paging.rs', import.meta.url), 'utf8');
const commands = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const stackPopupRs = readFileSync(new URL('../src-tauri/src/stack_popup.rs', import.meta.url), 'utf8');

test('stack git status backend and API use git porcelain outside folder listing', () => {
  assert.match(models, /pub struct StackGitStatus/);
  assert.match(commands, /getStackGitStatus:\s*'get_stack_git_status'/);
  assert.match(stackPopupRs, /pub async fn get_stack_git_status/);
  assert.match(api, /export type StackGitFileStatusKind = 'modified' \| 'added' \| 'deleted' \| 'untracked' \| 'conflict'/);
  assert.match(api, /export function getStackGitStatus\(folderPath: string\): Promise<StackGitStatus \| null>/);
  assert.doesNotMatch(paging, /get_stack_git_status/);
});

test('stack browser renders minimal git summary and row badges', () => {
  assert.match(surface, /class="stack-git-summary"/);
  assert.match(surface, /stackGitSummaryParts/);
  assert.match(surface, /stackGitStatusForEntry/);
  assert.match(surface, /data-git-status=\{gitEntryStatus \?\? undefined\}/);
  assert.match(surface, /git-status-badge/);
  assert.match(css, /\.stack-git-summary/);
  assert.match(css, /\.details-body button\.git-modified/);
  assert.match(css, /\.git-status-badge/);
});

test('stack browser file-list git status matches workbench glyphs and bare colors', () => {
  assert.match(surface, /if \(status === 'added'\) return 'A';/);
  assert.match(surface, /if \(status === 'deleted'\) return 'D';/);
  assert.match(surface, /if \(status === 'modified'\) return 'M';/);
  assert.match(surface, /if \(status === 'untracked'\) return '\?';/);
  assert.match(surface, /if \(status === 'conflict'\) return 'M';/);
  assert.match(css, /\.git-status-badge \{[\s\S]*background: transparent;[\s\S]*border: 0;[\s\S]*font-size: 0\.65rem;[\s\S]*font-weight: 600;/);
  assert.match(css, /\.git-status-badge \{[\s\S]*position: relative;[\s\S]*top: 1px;/);
  assert.match(css, /\.git-status-badge\.git-status-added \{ color: #76ad4f; \}/);
  assert.match(css, /\.git-status-badge\.git-status-deleted \{ color: #da5b4a; \}/);
  assert.match(css, /\.git-status-badge\.git-status-modified,[\s\S]*\.git-status-badge\.git-status-conflict \{\s*color: #c67f13;\s*\}/);
  assert.match(css, /\.git-status-badge\.git-status-untracked \{ color: #479fe6; \}/);
  assert.match(css, /\.details-body button\.git-added \{\s*box-shadow: inset 5px 0 0 #76ad4f;\s*\}/);
  assert.match(css, /\.details-body button\.git-modified \{\s*box-shadow: inset 5px 0 0 #c67f13;\s*\}/);
  assert.match(css, /\.details-body button\.git-deleted \{\s*box-shadow: inset 5px 0 0 #da5b4a;\s*\}/);
  assert.match(css, /\.details-body button\.git-conflicted \{\s*box-shadow: inset 5px 0 0 #c67f13;\s*\}/);
  assert.match(css, /\.details-body button\.git-untracked \{\s*box-shadow: inset 5px 0 0 #479fe6;\s*\}/);
});
