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
});

test('Stack Browser no longer exposes or warms embedded CLI view', () => {
  assert.doesNotMatch(stackPopup, /class="stack-view-toggle"/);
  assert.doesNotMatch(stackPopup, /CLI/);
  assert.doesNotMatch(stackPopup, /void warmStackTerminalForCurrentFolder\(\)/);
  assert.doesNotMatch(stackPopup, /class:terminal-mode/);
});

test('terminal panel owns xterm, startup status, errors, and poll fallback', () => {
  assert.match(terminalPanel, /import \{ Terminal \} from '@xterm\/xterm'/);
  assert.match(terminalPanel, /import \{ FitAddon \} from '@xterm\/addon-fit'/);
  assert.match(terminalPanel, /onMount\(\(\) => \{/);
  assert.match(terminalPanel, /void startTerminal\(\)/);
  assert.match(terminalPanel, /startPersistentTerminal\(\)/);
  assert.match(terminalPanel, /terminal-panel-status/);
  assert.match(terminalPanel, /role=\{lifecycle === 'failed' \? 'alert' : 'status'\}/);
  assert.match(terminalPanel, /readStackTerminal\(sessionId\)/);
  assert.match(terminalPanel, /writeStackTerminal\(sessionId, data\)/);
  assert.match(terminalPanel, /resizeStackTerminal\(/);
  assert.match(terminalPanel, /Still waiting for terminal output/);
  assert.match(terminalPanelCss, /\.terminal-panel/);
  assert.match(terminalPanelCss, /\.terminal-panel-output/);
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
  assert.match(contracts, /SHOW_TERMINAL_PANEL/);
  assert.match(contracts, /HIDE_TERMINAL_PANEL/);
  assert.match(contracts, /START_PERSISTENT_TERMINAL/);
});
