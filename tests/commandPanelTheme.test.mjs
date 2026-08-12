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
    '--js-color-success-border'
  ]) {
    assert.match(commandPanelCss, new RegExp(`var\\(${token}(?:\\)|,)`), `missing ${token}`);
  }

  assert.doesNotMatch(commandPanelCss, /#[0-9a-f]{3,8}\b/i, 'raw hex in themed surface');
  assert.match(commandPanelCss, /border-radius: 0/);
});

test('quick commands keep compact icon controls and context-only history/edit actions', () => {
  assert.match(commandPanelSource, /import MeltActionButton/);
  assert.match(commandPanelSource, /command-run-button/);
  assert.match(commandPanelSource, /command-stop-button/);
  assert.match(commandPanelSource, /class="command-icon-button command-delete-button"/);
  assert.match(commandPanelSource, /ariaLabel=\{activeRunIds\.has\(entry\.id\) \? `Stop \$\{entry\.label\}` : `Run \$\{entry\.label\}`\}/);
  assert.match(commandPanelSource, /ariaLabel=\{`Delete \$\{entry\.label\}`\}/);
  assert.match(commandPanelSource, /View output history/);
  assert.match(commandPanelSource, /Edit command/);
  assert.match(commandPanelSource, /command-spinner/);
  assert.match(commandPanelSource, /Configuration/);
  assert.match(commandPanelSource, /Previous runs/);
  assert.match(commandPanelSource, /listQuickCommandHistory\(\)/);
  assert.match(commandPanelSource, /ariaLabel=\{activeRunIds\.has\(entry\.id\) \? `Stop \$\{entry\.label\}` : `Run \$\{entry\.label\}`\}/);
  assert.match(commandPanelSource, /ariaLabel="Save command"[\s\S]*>\s*\{saving \? 'Saving…' : 'Save'\}/);
  assert.match(commandPanelSource, /ariaLabel="Cancel command editing"[\s\S]*>\s*Clear\s*</);
  assert.match(commandPanelSource, /onClick=\{\(\) => void \(activeRunIds\.has\(entry\.id\) \? stopEntry\(entry\.id\) : runEntry\(entry\.id\)\)\}/);
  assert.match(commandPanelSource, /onClick=\{\(\) => void deleteEntry\(entry\.id\)\}/);
});
