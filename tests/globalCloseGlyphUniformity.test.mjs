import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const iconRegistry = readFileSync(new URL('../src/components/icons/materialSymbolIcons.ts', import.meta.url), 'utf8');
const stackPopup = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const taskPreview = readFileSync(new URL('../src/components/TaskPreviewSurface.svelte', import.meta.url), 'utf8');
const processManager = readFileSync(new URL('../src/components/ProcessManagerSurface.svelte', import.meta.url), 'utf8');
const commandPanel = readFileSync(new URL('../src/components/CommandPanelSurface.svelte', import.meta.url), 'utf8');
const settingsPanel = readFileSync(new URL('../src/components/SettingsPanelSurface.svelte', import.meta.url), 'utf8');
const trayPanel = readFileSync(new URL('../src/components/TrayPanelSurface.svelte', import.meta.url), 'utf8');
const calendarPanel = readFileSync(new URL('../src/components/CalendarPanelSurface.svelte', import.meta.url), 'utf8');
const stackGitPanel = readFileSync(new URL('../src/components/StackGitPanel.svelte', import.meta.url), 'utf8');
const terminalPanel = readFileSync(new URL('../src/components/TerminalPanelSurface.svelte', import.meta.url), 'utf8');
const searchPanel = readFileSync(new URL('../src/components/SearchPanelSurface.svelte', import.meta.url), 'utf8');
const topBar = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');

test('global close glyphs use shared close icon and search icon is official', () => {
  assert.match(iconRegistry, /close:\s*'m256-200-56-56 224-224-224-224 56-56 224 224 224-224 56 56-224 224 224 224-56 56-224-224-224 224Z'/);
  assert.match(iconRegistry, /search:\s*'M784-120 532-372q-30 24-69 38t-83 14q-109 0-184\.5-75\.5T120-580q0-109 75\.5-184\.5T380-840q109 0 184\.5 75\.5T640-580q0 44-14 83t-38 69l252 252-56 56ZM380-400q75 0 127\.5-52\.5T560-580q0-75-52\.5-127\.5T380-760q-75 0-127\.5 52\.5T200-580q0 75 52\.5 127\.5T380-400Z'/);
  assert.match(iconRegistry, /'close',\s*'settings'/);

  for (const [source, className, ariaLabel, glyph] of [
    [stackPopup, 'stack-browser-close-button', 'Close stack browser', '×'],
    [taskPreview, 'preview-close-button', 'Close previewed window', '×'],
    [processManager, 'process-manager-close-button', 'Close process manager', '×'],
    [commandPanel, 'command-panel-close-button', 'Close quick commands', '×'],
    [settingsPanel, 'settings-panel-header', 'Close settings', 'x'],
    [trayPanel, 'tray-close-button', 'Close notification area icons', '×'],
    [calendarPanel, 'calendar-close', 'Close calendar', '×'],
    [stackGitPanel, 'stack-git-icon-button', 'Close git panel', '✕']
  ]) {
    assert.match(source, /import MaterialSymbolIcon from '\.\/icons\/MaterialSymbolIcon\.svelte'/);
    assert.match(source, /<MaterialSymbolIcon name="close" \/>/);
    assert.match(source, new RegExp(`class="${className}"`));
    assert.match(source, new RegExp(`ariaLabel="${ariaLabel}"|aria-label="${ariaLabel}"`));
    assert.doesNotMatch(source, new RegExp(`class="${className}"[\\s\\S]*>${glyph}<`));
    assert.doesNotMatch(source, new RegExp(`ariaLabel="${ariaLabel}"[\\s\\S]*>${glyph}<|aria-label="${ariaLabel}"[\\s\\S]*>${glyph}<`));
  }

  assert.match(terminalPanel, /import MaterialSymbolIcon from '\.\/icons\/MaterialSymbolIcon\.svelte'/);
  assert.match(terminalPanel, /class="terminal-tab-close"/);
  assert.match(terminalPanel, /aria-label=\{`Close terminal session \$\{displayTitle\}`\}/);
  assert.match(terminalPanel, /<MaterialSymbolIcon name="close" \/>/);
  assert.doesNotMatch(terminalPanel, /class="terminal-tab-close"[\s\S]*>×<\/button>/);

  assert.match(searchPanel, /class="search-panel-clear-button"/);
  assert.match(searchPanel, /ariaLabel="Clear search"/);
  assert.match(searchPanel, /class="search-panel-clear-button"[\s\S]*?×[\s\S]*?<\/MeltActionButton>/);
  assert.match(topBar, /class="search-button"/);
  assert.match(topBar, /ariaLabel="Open search"/);
  assert.match(topBar, /<MaterialSymbolIcon name="search" \/>/);
  assert.doesNotMatch(searchPanel, /class="search-panel-clear-button"[\s\S]*<MaterialSymbolIcon name="close" \/>/);
  assert.doesNotMatch(topBar, /search-clear-button|<MaterialSymbolIcon name="close" \/>/);
});
