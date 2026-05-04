import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const commandPanelSource = readFileSync(new URL('../src/components/CommandPanelSurface.svelte', import.meta.url), 'utf8');
const commandPanelCss = readFileSync(new URL('../src/components/CommandPanelSurface.css', import.meta.url), 'utf8');

test('quick commands surface uses shared shell theme tokens for primary surfaces', () => {
  for (const token of [
    '--js-bg-panel',
    '--js-color-surface-raised',
    '--js-color-surface-sunken',
    '--js-color-border-soft',
    '--js-color-text',
    '--js-color-text-muted',
    '--js-color-accent-border',
    '--js-shadow-raised',
    '--js-radius-md'
  ]) {
    assert.match(commandPanelCss, new RegExp(`var\\(${token}(?:\\)|,)`), `missing ${token}`);
  }

  const primarySurfaceBlocks = commandPanelCss.match(
    /\.(?:command-panel|command-list|command-editor|command-list li|command-editor input,\n\.command-editor select,\n\.command-editor textarea) \{[\s\S]*?\n\}/g
  ) ?? [];

  assert.ok(primarySurfaceBlocks.length >= 4, 'expected command panel primary surface blocks');
  for (const block of primarySurfaceBlocks) {
    assert.doesNotMatch(block, /#[0-9a-f]{3,8}\b/i, `raw hex in primary themed surface:\n${block}`);
  }
});

test('quick commands controls keep button semantics and existing action labels', () => {
  assert.match(commandPanelSource, /import MeltActionButton/);
  assert.match(commandPanelSource, /ariaLabel=\{`Run \$\{entry\.label\}`\}[\s\S]*>\s*\{runningId === entry\.id \? 'Running…' : 'Run'\}/);
  assert.match(commandPanelSource, /ariaLabel=\{`Edit \$\{entry\.label\}`\}[\s\S]*>\s*Edit\s*</);
  assert.match(commandPanelSource, /ariaLabel=\{`Delete \$\{entry\.label\}`\}[\s\S]*>\s*Delete\s*</);
  assert.match(commandPanelSource, /ariaLabel="Save command"[\s\S]*>\s*\{saving \? 'Saving…' : 'Save'\}/);
  assert.match(commandPanelSource, /ariaLabel="Cancel command editing"[\s\S]*>\s*Clear\s*</);
  assert.match(commandPanelSource, /onClick=\{\(\) => void runEntry\(entry\.id\)\}/);
  assert.match(commandPanelSource, /onClick=\{\(\) => startEditEntry\(entry\)\}/);
  assert.match(commandPanelSource, /onClick=\{\(\) => void deleteEntry\(entry\.id\)\}/);
});
