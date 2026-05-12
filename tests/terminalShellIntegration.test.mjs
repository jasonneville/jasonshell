import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

function readRepoFile(path) {
  const url = new URL(`../${path}`, import.meta.url);
  return existsSync(url) ? readFileSync(url, 'utf8') : '';
}

const model = readRepoFile('src/features/stack-browser/terminalShellIntegration.ts');
const panel = readRepoFile('src/components/TerminalPanelSurface.svelte');
const rustTerminal = readRepoFile('src-tauri/src/stack_popup/terminal.rs');

test('terminal shell integration parser supports OSC command and cwd markers', () => {
  assert.match(model, /parseTerminalShellSequence/);
  assert.match(model, /prefix === 'A'/);
  assert.match(model, /prefix === 'B'/);
  assert.match(model, /prefix === 'C'/);
  assert.match(model, /prefix === 'D'/);
  assert.match(model, /CurrentDir/);
  assert.match(model, /reduceTerminalShellMarker/);
  assert.match(model, /beginTerminalCommandRecord/);
  assert.match(model, /outputStartMarker: typeof xtermMarkerLine === 'number' \? xtermMarkerLine \+ 1/);
  assert.match(model, /MAX_COMMAND_RECORDS = 200/);
});

test('visible terminal registers frontend OSC parser and command actions', () => {
  assert.match(panel, /registerOscHandler\?\.\(133/);
  assert.match(panel, /registerOscHandler\?\.\(1337/);
  assert.match(panel, /registerOscHandler\?\.\(633/);
  assert.match(panel, /applyAuthoritativeTerminalCwd/);
  assert.match(panel, /shellCwdMarkerSeen/);
  assert.match(panel, /shellParserDisposers/);
  assert.match(panel, /beginTerminalCommandRecord/);
  assert.match(panel, /Alt\+ArrowUp|event\.key === 'ArrowUp'/);
  assert.match(panel, /Alt\+ArrowDown|event\.key === 'ArrowDown'/);
  assert.match(panel, /copySelectedCommandOutput/);
  assert.match(panel, /Copy command output/);
});

test('PowerShell and Git Bash static integration inject markers without dotfile mutation', () => {
  assert.match(rustTerminal, /`e\]133;D;\$last`a/);
  assert.match(rustTerminal, /`e\]133;CurrentDir;\$cwd`a/);
  assert.match(rustTerminal, /`e\]133;A`a/);
  assert.match(rustTerminal, /JASONSHELL_TERMINAL_SHELL_INTEGRATION/);
  assert.match(rustTerminal, /apply_git_bash_shell_integration/);
  assert.match(rustTerminal, /PROMPT_COMMAND/);
  assert.match(rustTerminal, /\\033\]133;CurrentDir;%s\\a/);
  assert.doesNotMatch(rustTerminal, /\.bashrc|\.bash_profile|Microsoft\.PowerShell_profile|profile\.ps1/i);
});
