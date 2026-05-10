import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

const app = read('src/App.svelte');
const topBar = read('src/components/TopBar.svelte');
const topBarCss = read('src/components/TopBar.css');
const stackPopup = read('src/components/StackPopupSurface.svelte');
const terminalPanel = read('src/components/TerminalPanelSurface.svelte');
const terminalPanelCss = read('src/components/TerminalPanelSurface.css');
const shellSurface = read('src/lib/shellSurface.ts');
const ipcSurfaces = read('src/ipc/surfaces.ts');
const ipcCommands = read('src/ipc/commands.ts');
const terminalApi = read('src/lib/persistentTerminal.ts');
const terminalPanelApi = read('src/lib/terminalPanel.ts');
const terminalPanelBackend = read('src-tauri/src/terminal_panel.rs');
const shellWindows = read('src-tauri/src/shell_windows.rs');
const main = read('src-tauri/src/main.rs');
const contracts = read('src-tauri/src/contracts.rs');
const capability = read('src-tauri/capabilities/terminal-panel.json');

test('persistent terminal is its own shell surface and starts at app startup', () => {
  assert.match(app, /import TerminalPanelSurface/);
  assert.match(app, /surface === 'terminal-panel'[\s\S]*<TerminalPanelSurface \/>/);
  assert.match(shellSurface, /\| 'terminal-panel'/);
  assert.match(ipcSurfaces, /terminalPanel: 'terminal-panel'/);
  assert.match(shellWindows, /TERMINAL_PANEL_LABEL: &str = "terminal-panel"/);
  assert.match(shellWindows, /build_terminal_panel_window/);
  assert.match(main, /terminal_panel::show_terminal_panel/);
  assert.match(main, /terminal_panel::hide_terminal_panel/);
  assert.match(main, /stack_popup::start_persistent_terminal/);
  assert.match(main, /stack_popup::read_stack_terminal/);
  assert.match(capability, /"terminal-panel"/);
});

test('top bar terminal button sits before quick commands and toggles terminal panel', () => {
  assert.match(ipcCommands, /showTerminalPanel: 'show_terminal_panel'/);
  assert.match(ipcCommands, /hideTerminalPanel: 'hide_terminal_panel'/);
  assert.match(terminalPanelApi, /showTerminalPanel/);
  assert.match(terminalPanelApi, /hideTerminalPanel/);
  assert.match(topBar, /TERMINAL_PANEL_ID = 'terminal-panel'/);
  assert.match(topBar, /class="terminal-button"/);
  assert.match(topBar, /toggleTerminalPanel\(event\.currentTarget\)/);
  assert.match(topBar, /class="terminal-control"[\s\S]*class="command-control"/);
  assert.match(topBarCss, /\.top-bar \.terminal-button/);
  assert.match(terminalPanelBackend, /TERMINAL_PANEL_OPEN_EVENT: &str = "terminal-panel:open"/);
  assert.match(terminalPanelBackend, /emit_to\(TERMINAL_PANEL_LABEL, TERMINAL_PANEL_OPEN_EVENT/);
});

test('Stack Browser embedded CLI is isolated in extracted terminal pane', () => {
  assert.match(stackPopup, /import StackTerminalPane from '\.\/StackTerminalPane\.svelte'/);
  assert.match(stackPopup, /class="stack-view-toggle"/);
  assert.match(stackPopup, /CLI/);
  assert.match(stackPopup, /class:terminal-mode/);
  assert.doesNotMatch(stackPopup, /from '@xterm\/xterm'/);
  assert.doesNotMatch(stackPopup, /from '@xterm\/addon-fit'/);
});

test('terminal panel owns xterm, startup status, errors, and poll fallback', () => {
  assert.match(terminalPanel, /import \{ Terminal \} from '@xterm\/xterm'/);
  assert.match(terminalPanel, /import \{ FitAddon \} from '@xterm\/addon-fit'/);
  assert.match(terminalPanel, /onMount\(\(\) => \{/);
  assert.match(terminalPanel, /void startTerminal\(\)/);
  assert.match(terminalPanel, /startPersistentTerminal\(\)/);
  assert.match(terminalPanel, /TERMINAL_PANEL_OPEN_EVENT = 'terminal-panel:open'/);
  assert.match(terminalPanel, /listen\(TERMINAL_PANEL_OPEN_EVENT/);
  assert.match(terminalPanel, /window\.addEventListener\('focus', handlePanelOpen\)/);
  assert.match(terminalPanel, /function handlePanelOpen/);
  assert.match(terminalPanel, /function scheduleFitAfterPanelOpen/);
  assert.match(terminalPanel, /visibleResizePromise = resizeTerminalToFit\(\)/);
  assert.match(terminalPanel, /window\.setTimeout\(\(\) => scheduleFit\(\), 60\)/);
  assert.match(terminalPanel, /function ensureVisibleResizeBeforeInput/);
  assert.match(terminalPanel, /await ensureVisibleResizeBeforeInput\(\);[\s\S]{0,120}writeStackTerminal\(sessionId, data\)/);
  assert.match(terminalPanel, /terminal-panel-status/);
  assert.match(terminalPanel, /role=\{lifecycle === 'failed' \? 'alert' : 'status'\}/);
  assert.match(terminalPanel, /readStackTerminal\(sessionId\)/);
  assert.match(terminalPanel, /writeStackTerminal\(sessionId, data\)/);
  assert.doesNotMatch(terminalPanel, /writeTerminalOutput\(result\.output\)/);
  assert.match(terminalPanel, /const sequenceKey = `\$\{chunk\.sessionId\}:\$\{chunk\.stream \?\? 'stdout'\}:\$\{chunk\.sequence\}`/);
  assert.match(terminalPanel, /let writeQueue: Promise<void> = Promise\.resolve\(\)/);
  assert.match(terminalPanel, /function enqueueTerminalWrite/);
  assert.match(terminalPanel, /enqueueTerminalWrite\(\(\) => writeStackTerminal\(sessionId, data\)\)/);
  assert.doesNotMatch(
    terminalPanel,
    /async function writeTerminalData[\s\S]{0,360}pollTerminalOutput\(\)/,
    'normal terminal input must not wait for a read/poll roundtrip'
  );
  assert.match(terminalPanel, /convertEol:\s*false/);
  assert.match(terminalPanel, /windowsPty:\s*\{\s*backend:\s*'conpty'\s*\}/);
  assert.doesNotMatch(
    terminalPanel,
    /function writeTerminalOutput[\s\S]{0,460}terminal\?\.scrollToBottom\(\)/,
    'full-screen TUI redraws must not be followed by forced scroll pinning'
  );
  assert.match(terminalPanel, /trackTerminalInput\(data\)/);
  assert.match(terminalPanel, /resizeStackTerminal\(/);
  assert.match(terminalPanel, /terminal\.attachCustomKeyEventHandler/);
  assert.match(terminalPanel, /isAltBackquoteHotkey\(event\)/);
  assert.match(terminalPanel, /hideTerminalPanel\(\)/);
  assert.match(terminalPanel, /<svelte:window on:keydown\|capture/);
  assert.match(terminalPanel, /return false;\s*}\s*if \(event\.type === 'keyup'/);
  assert.match(terminalPanel, /navigator\.clipboard\?\.writeText\(selection\)/);
  assert.match(terminalPanel, /navigator\.clipboard\?\.readText\(\)/);
  assert.match(terminalPanel, /function handleTerminalMouseDown\(event: MouseEvent\)/);
  assert.match(terminalPanel, /event\.detail < 3/);
  assert.match(terminalPanel, /terminal\.select\(startColumn, row, currentInputText\.length\)/);
  assert.match(terminalPanel, /currentInputSelectionActive = true/);
  assert.match(terminalPanel, /function deleteSelectedCurrentInput\(\)/);
  assert.match(terminalPanel, /'\\u007f'\.repeat\(length\)/);
  assert.match(terminalPanel, /event\.key === 'Backspace' \|\| event\.key === 'Delete'/);
  assert.match(terminalPanel, /on:mousedown\|capture=\{handleTerminalMouseDown\}/);
  assert.match(terminalPanel, /on:contextmenu=\{openTerminalContextMenu\}/);
  assert.match(terminalPanel, /class="terminal-panel-context-menu"/);
  assert.match(terminalPanel, /TERMINAL_PANEL_FONT_FAMILY/);
  assert.match(terminalPanel, /fontFamily: TERMINAL_PANEL_FONT_FAMILY/);
  assert.match(terminalPanel, /letterSpacing: 0/);
  assert.doesNotMatch(terminalPanel, /function anchorCommandLineToLastRow\(\)/);
  assert.doesNotMatch(terminalPanel, /terminal\.write\(`\\x1b\[\$\{terminal\.rows\};1H`\)/);
  assert.doesNotMatch(terminalPanel, /terminalOutputHasClear/);
  assert.match(terminalPanel, /Still waiting for terminal output/);
  assert.match(terminalPanelCss, /\.terminal-panel/);
  assert.match(terminalPanelCss, /\.terminal-panel-output/);
  assert.match(terminalPanelCss, /\.terminal-panel-context-menu/);
  assert.match(terminalPanelCss, /font-feature-settings: "liga" 0, "calt" 0, "tnum" 1;/);
  assert.match(terminalPanelCss, /opacity: 0 !important;/);
  assert.match(terminalPanelCss, /caret-color: transparent !important;/);
  assert.match(terminalPanelCss, /left: -10000px !important;/);
  assert.match(terminalPanelCss, /\.terminal-panel-output :global\(\.xterm-rows\)/);
});

test('persistent terminal output is routed to the terminal panel window', () => {
  const terminalBackend = read('src-tauri/src/stack_popup/terminal.rs');
  const stackPopupBackend = read('src-tauri/src/stack_popup.rs');
  assert.match(terminalBackend, /target_label: Option<String>/);
  assert.match(terminalBackend, /terminal_event_target_label/);
  assert.match(terminalBackend, /shell_windows::TERMINAL_PANEL_LABEL/);
  assert.match(terminalBackend, /emit_to\(\s*target_label\.as_str\(\),\s*crate::contracts::events::STACK_TERMINAL_OUTPUT/);
  assert.match(stackPopupBackend, /target_label: Some\(crate::shell_windows::TERMINAL_PANEL_LABEL\.to_string\(\)\)/);
});

test('contracts list terminal panel surface and commands', () => {
  assert.match(contracts, /TERMINAL_PANEL: &str = "terminal-panel"/);
  assert.match(contracts, /TERMINAL_PANEL_OPEN: &str = "terminal-panel:open"/);
  assert.match(contracts, /SHOW_TERMINAL_PANEL/);
  assert.match(contracts, /HIDE_TERMINAL_PANEL/);
  assert.match(contracts, /START_PERSISTENT_TERMINAL/);
});
