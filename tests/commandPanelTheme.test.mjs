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
    '--js-color-success-border',
    '--js-color-surface-overlay',
    '--js-color-border',
    '--js-color-error',
    '--js-color-error-border',
    '--js-color-text-strong'
  ]) {
    assert.match(commandPanelCss, new RegExp(`var\\(${token}(?:\\)|,)`), `missing ${token}`);
  }

  assert.match(commandPanelCss, /\.command-panel-close-button \{[\s\S]*background: #dc2626;/);
  assert.match(commandPanelCss, /\.command-panel-close-button:hover,\s*\.command-panel-close-button:focus-visible \{[\s\S]*background: #ef4444;/);
  assert.match(commandPanelCss, /\.command-panel-close-button:active \{[\s\S]*background: #b91c1c;/);
  const rawHexWithoutCloseButton = commandPanelCss
    .replace(/\.command-panel-close-button \{[\s\S]*?\n\}/, '')
    .replace(/\.command-panel-close-button:hover,\s*\.command-panel-close-button:focus-visible \{[\s\S]*?\n\}/, '')
    .replace(/\.command-panel-close-button:active \{[\s\S]*?\n\}/, '');
  assert.doesNotMatch(rawHexWithoutCloseButton, /#[0-9a-f]{3,8}\b/i, 'raw hex in themed surface');
  assert.match(commandPanelCss, /border-radius: 0/);
  assert.match(commandPanelCss, /\.delete-confirm-backdrop \{/);
  assert.match(commandPanelCss, /position: fixed;/);
  assert.match(commandPanelCss, /inset: 0;/);
  assert.match(commandPanelCss, /z-index: 80;/);
  assert.match(commandPanelCss, /\.delete-confirm-dialog \{[\s\S]*box-shadow: var\(--js-shadow-raised\);[\s\S]*width: min\(28rem, calc\(100vw - 1.5rem\)\);/);
  assert.match(commandPanelCss, /\.delete-confirm-actions :global\(button\):focus-visible \{[\s\S]*box-shadow: var\(--js-focus-ring\);/);
  assert.match(commandPanelCss, /command-transcript-token--url/);
});

test('quick commands keep compact icon controls and context-only history/edit actions', () => {
  assert.match(commandPanelSource, /import MeltActionButton/);
  assert.match(commandPanelSource, /command-run-button/);
  assert.match(commandPanelSource, /command-stop-button/);
  assert.match(commandPanelSource, /class="command-icon-button command-delete-button"/);
  assert.match(commandPanelSource, /ariaLabel=\{isCommandStopping\(entry\.id\) \? `\$\{entry\.label\} is stopping` : activeCommandIds\.has\(entry\.id\) \? `Stop \$\{entry\.label\}` : `Run \$\{entry\.label\}`\}/);
  assert.match(commandPanelSource, /ariaLabel=\{`Delete \$\{entry\.label\}`\}/);
  assert.match(commandPanelSource, /View output history/);
  assert.match(commandPanelSource, /Edit command/);
  assert.match(commandPanelSource, /command-spinner/);
  assert.match(commandPanelSource, /command-input-panel/);
  assert.match(commandPanelSource, /command-transcript-shell/);
  assert.match(commandPanelSource, /command-transcript-line/);
  assert.match(commandPanelSource, /Configuration/);
  assert.match(commandPanelSource, /Previous runs/);
  assert.match(commandPanelSource, /listQuickCommandHistory\(\)/);
  assert.match(commandPanelSource, /const allRuns = await listQuickCommandHistory\(\);/);
  assert.match(commandPanelSource, /ariaLabel=\{isCommandStopping\(entry\.id\) \? `\$\{entry\.label\} is stopping` : activeCommandIds\.has\(entry\.id\) \? `Stop \$\{entry\.label\}` : `Run \$\{entry\.label\}`\}/);
  assert.match(commandPanelSource, /ariaLabel="Save command"[\s\S]*>\s*\{saving \? 'Saving…' : 'Save'\}/);
  assert.match(commandPanelSource, /ariaLabel="Cancel command editing"[\s\S]*>\s*Clear\s*</);
  assert.match(commandPanelSource, /onClick=\{\(\) => void \(activeCommandIds\.has\(entry\.id\) \? stopEntry\(entry\.id\) : runEntry\(entry\)\)\}/);
  assert.match(commandPanelSource, /onClick=\{\(event\) => void deleteEntry\(entry\.id, event\)\}/);
});
