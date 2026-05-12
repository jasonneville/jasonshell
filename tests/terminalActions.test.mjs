import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

function readRepoFile(path) {
  const url = new URL(`../${path}`, import.meta.url);
  return existsSync(url) ? readFileSync(url, 'utf8') : '';
}

const actions = readRepoFile('src/features/terminal/terminalActions.ts');
const history = readRepoFile('src/features/terminal/terminalHistory.ts');
const fixes = readRepoFile('src/features/terminal/terminalQuickFixes.ts');
const panel = readRepoFile('src/components/TerminalPanelSurface.svelte');

test('terminal menus close on outside pointer without eating internal menu clicks', () => {
  assert.match(panel, /document\.addEventListener\('pointerdown', closeTerminalMenusOnOutsidePointer, true\)/);
  assert.match(panel, /target\.closest\('\.terminal-toolbar, \.terminal-toolbar-menu, \.terminal-panel-context-menu, \.terminal-search, \.terminal-quick-select'\)/);
  assert.match(panel, /closeTerminalMenus\(\)/);
  assert.match(panel, /actionMenuOpen = false/);
  assert.match(panel, /recentMenuOpen = false/);
  assert.match(panel, /quickSelectOpen = false/);
  assert.match(panel, /searchOpen = false/);
  assert.match(panel, /on:pointerdown\|stopPropagation/);
});

test('terminal action gating avoids scanning xterm buffer during reactive state updates', () => {
  assert.match(panel, /function commandRecordHasOutput\(record: TerminalCommandRecord\)/);
  assert.match(panel, /hasCommandOutput: Boolean\(record && commandRecordHasOutput\(record\)\)/);
  assert.doesNotMatch(panel, /hasCommandOutput: Boolean\(record && commandOutputText\(record\)\)/);
  assert.doesNotMatch(panel, /detectTerminalQuickFixes|quickFixScanText|terminal-quick-fixes/);
});

test('terminal cwd actions use effective cwd fallback instead of session-only state', () => {
  assert.match(panel, /function effectiveTerminalCwd\(\)/);
  assert.match(panel, /session\?\.cwd \|\| commandState\?\.cwd \|\| selectedCommand\(\)\?\.cwd/);
  assert.match(panel, /hasCwd: Boolean\(cwd\)/);
  assert.match(panel, /openStackTerminalHere\(cwd\)/);
  assert.match(panel, /openStackFolderInVscode\(cwd\)/);
  assert.match(panel, /revealStackItem\(cwd\)/);
});

test('terminal action registry defines state-gated Phase 6 actions', () => {
  for (const id of [
    'copySelection',
    'copyCommand',
    'copyCommandOutput',
    'rerunCommand',
    'search',
    'clear',
    'openCwdInFiles',
    'openExternalTerminalHere',
    'restartTerminal',
    'stopTerminal'
  ]) {
    assert.match(actions, new RegExp(`id: '${id}'`));
  }
  assert.match(actions, /destructive: true/);
  assert.match(actions, /id: 'search'[\s\S]*isEnabled: \(\) => true/);
  assert.match(actions, /id: 'restartTerminal'[\s\S]*isEnabled: \(\) => true/);
  assert.match(actions, /hasCommandOutput/);
  assert.match(actions, /hasDetectedTarget/);
  assert.match(panel, /getTerminalAction/);
  assert.match(panel, /runTerminalAction/);
});

test('recent terminal history is bounded in memory and not persisted', () => {
  assert.match(history, /recentTerminalCommands/);
  assert.match(history, /recentTerminalDirectories/);
  assert.match(history, /limit = 12/);
  assert.match(history, /limit = 8/);
  assert.doesNotMatch(history, /localStorage|settings|invoke|writeFile|persist/i);
  assert.match(panel, /recentTerminalCommands\(activeRuntime\(\)\?\.commandState \?\? commandState\)/);
  assert.match(panel, /recentTerminalDirectories\(activeRuntime\(\)\?\.commandState \?\? commandState\)/);
  assert.match(panel, /cdCommandForDirectory/);
  assert.match(panel, /quoteShellPath/);
  assert.doesNotMatch(panel, /`cd \$\{dir\}`/);
});

test('quick-fix parser remains pure but terminal overlay is hidden for now', () => {
  assert.match(fixes, /autoExecute: false/);
  assert.match(fixes, /git push --set-upstream origin/);
  assert.match(fixes, /EADDRINUSE|address already in use/);
  assert.match(fixes, /command not found|not recognized/);
  assert.doesNotMatch(fixes, /writeStackTerminal|writeTerminalData|invoke\(/);
  assert.doesNotMatch(panel, /terminal-quick-fixes|handleQuickFix|detectTerminalQuickFixes/);
});

