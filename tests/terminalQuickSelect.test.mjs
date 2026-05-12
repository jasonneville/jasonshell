import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

function readRepoFile(path) {
  const url = new URL(`../${path}`, import.meta.url);
  return existsSync(url) ? readFileSync(url, 'utf8') : '';
}

const quickSelect = readRepoFile('src/features/terminal/terminalQuickSelect.ts');
const panel = readRepoFile('src/components/TerminalPanelSurface.svelte');

test('terminal quick select detects safe daily developer targets', () => {
  assert.match(quickSelect, /TerminalQuickSelectKind = 'url' \| 'localhost' \| 'windowsPath' \| 'relativePath' \| 'fileLine' \| 'gitHash' \| 'branch'/);
  assert.match(quickSelect, /URL_PATTERN/);
  assert.match(quickSelect, /LOCALHOST_PATTERN/);
  assert.match(quickSelect, /WINDOWS_PATH_PATTERN/);
  assert.match(quickSelect, /\[\^\\s\\r\\n<>\|"\?\*\]\+/);
  assert.match(quickSelect, /RELATIVE_PATH_PATTERN/);
  assert.match(quickSelect, /GIT_HASH_PATTERN/);
  assert.match(quickSelect, /BRANCH_PATTERN/);
  assert.match(quickSelect, /assignQuickSelectLabels/);
});

test('terminal quick select safety rejects unsafe open targets', () => {
  assert.match(quickSelect, /isSafeTerminalQuickSelectTarget/);
  assert.match(quickSelect, /\^https\?:\\\/\\\//);
  assert.match(quickSelect, /javascript:/);
  assert.match(quickSelect, /\[<>\|"\?\*\]/);
  assert.match(quickSelect, /isSafeBranchName/);
  assert.match(quickSelect, /!value\.includes\('\.\.'\)/);
});

test('persistent terminal wires quick select overlay and keyboard cancellation', () => {
  assert.match(panel, /detectTerminalQuickSelectTargets/);
  assert.match(panel, /quickSelectOpen/);
  assert.match(panel, /terminal-quick-select/);
  assert.match(panel, /event\.key === 'Escape'/);
  assert.match(panel, /openQuickSelectTarget/);
  assert.match(panel, /copyQuickSelectTarget/);
});
