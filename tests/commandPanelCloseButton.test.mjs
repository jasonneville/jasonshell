import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const commandPanelSource = readFileSync(new URL('../src/components/CommandPanelSurface.svelte', import.meta.url), 'utf8');
const commandPanelCss = readFileSync(new URL('../src/components/CommandPanelSurface.css', import.meta.url), 'utf8');

test('command panel close button is accessible and styled like destructive shell close controls', () => {
  assert.match(commandPanelSource, /function closePanel\(\) \{[\s\S]*hideCommandPanel\(\)/);
  assert.match(commandPanelSource, /import MaterialSymbolIcon from '\.\/icons\/MaterialSymbolIcon\.svelte'/);
  assert.match(commandPanelSource, /class="command-panel-close-button"/);
  assert.match(commandPanelSource, /ariaLabel="Close quick commands"/);
  assert.match(commandPanelSource, /onClick=\{closePanel\}/);
  assert.match(commandPanelSource, /<MaterialSymbolIcon name="close" \/><\/MeltActionButton>/);

  assert.match(commandPanelCss, /\.command-panel-header \{/);
  assert.match(commandPanelCss, /position: relative;/);
  assert.match(commandPanelCss, /padding-right: 3rem;/);
  assert.match(commandPanelCss, /\.command-panel-close-button \{/);
  assert.match(commandPanelCss, /align-items: center;/);
  assert.match(commandPanelCss, /background: #dc2626;/);
  assert.match(commandPanelCss, /border: 1px solid rgba\(255, 255, 255, 0\.35\);/);
  assert.match(commandPanelCss, /border-radius: var\(--js-radius-xs\);/);
  assert.match(commandPanelCss, /color: #fff;/);
  assert.match(commandPanelCss, /cursor: pointer;/);
  assert.match(commandPanelCss, /display: inline-flex;/);
  assert.match(commandPanelCss, /font: inherit;/);
  assert.match(commandPanelCss, /font-size: 0\.75rem;/);
  assert.match(commandPanelCss, /font-weight: 800;/);
  assert.match(commandPanelCss, /height: 1\.35rem;/);
  assert.match(commandPanelCss, /justify-content: center;/);
  assert.match(commandPanelCss, /line-height: 1;/);
  assert.match(commandPanelCss, /min-width: 2\.1rem;/);
  assert.match(commandPanelCss, /padding: 0 0\.42rem;/);
  assert.match(commandPanelCss, /position: absolute;/);
  assert.match(commandPanelCss, /right: 0\.42rem;/);
  assert.match(commandPanelCss, /top: 0\.42rem;/);
  assert.match(commandPanelCss, /z-index: 4;/);
  assert.match(commandPanelCss, /\.command-panel-close-button:hover,\s*\.command-panel-close-button:focus-visible \{[\s\S]*background: #ef4444;/);
  assert.match(commandPanelCss, /\.command-panel-close-button:active \{[\s\S]*background: #b91c1c;[\s\S]*transform: translateY\(1px\);/);
  assert.match(commandPanelCss, /box-shadow: var\(--js-focus-ring\);/);
});
