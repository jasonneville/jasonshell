import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const appSource = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
const surfaceLoaderSource = readFileSync(new URL('../src/lib/surfaceLoader.ts', import.meta.url), 'utf8');
const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const topBarCss = readFileSync(new URL('../src/components/TopBar.css', import.meta.url), 'utf8');
const commandPanelSource = readFileSync(new URL('../src/components/CommandPanelSurface.svelte', import.meta.url), 'utf8');
const commandPanelCss = readFileSync(new URL('../src/components/CommandPanelSurface.css', import.meta.url), 'utf8');
const shellSurfaceSource = readFileSync(new URL('../src/lib/shellSurface.ts', import.meta.url), 'utf8');
const ipcSurfacesSource = readFileSync(new URL('../src/ipc/surfaces.ts', import.meta.url), 'utf8');
const ipcEventsSource = readFileSync(new URL('../src/ipc/events.ts', import.meta.url), 'utf8');
const ipcCommandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const commandPanelWrapper = readFileSync(new URL('../src/lib/commandPanel.ts', import.meta.url), 'utf8');
const shellWindowsSource = readFileSync(new URL('../src-tauri/src/shell_windows.rs', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const commandPanelRs = readFileSync(new URL('../src-tauri/src/command_panel.rs', import.meta.url), 'utf8');
const contractsSource = readFileSync(new URL('../src-tauri/src/contracts.rs', import.meta.url), 'utf8');
const capabilitySource = readFileSync(new URL('../src-tauri/capabilities/command-panel.json', import.meta.url), 'utf8');

test('command panel is routed as a dedicated auxiliary shell surface', () => {
  assert.match(appSource, /loadSurfaceComponent\(surface\)/);
  assert.match(surfaceLoaderSource, /'command-panel': \(\) => import\('\.\.\/components\/CommandPanelSurface\.svelte'\)/);
  assert.match(shellSurfaceSource, /\| 'command-panel'/);
  assert.match(shellWindowsSource, /COMMAND_PANEL_LABEL: &str = "command-panel"/);
  assert.match(shellWindowsSource, /COMMAND_PANEL_WIDTH_LOGICAL: f64 = 460\.0/);
  assert.match(shellWindowsSource, /COMMAND_PANEL_HEIGHT_LOGICAL: f64 = 420\.0/);
  assert.match(shellWindowsSource, /build_command_panel_window\(app\)/);
  assert.match(shellWindowsSource, /fn build_command_panel_window\(app: &App\)[\s\S]*?\.resizable\(true\)/);
  assert.match(mainSource, /mod command_panel;/);
  assert.match(mainSource, /command_panel::show_command_panel/);
  assert.match(mainSource, /command_panel::hide_command_panel/);
  assert.match(mainSource, /shell_windows::COMMAND_PANEL_LABEL[\s\S]*WindowEvent::Focused\(false\)/);
  assert.match(mainSource, /emit_to\(\s*shell_windows::TOP_BAR_LABEL,\s*command_panel::COMMAND_PANEL_CLOSED_EVENT/);
  assert.match(commandPanelRs, /pub fn show_command_panel/);
  assert.match(commandPanelRs, /pub fn hide_command_panel/);
  assert.match(capabilitySource, /"command-panel"/);
});

test('command panel contracts and wrappers use constant-backed IPC and event names', () => {
  assert.match(ipcCommandsSource, /showCommandPanel: 'show_command_panel'/);
  assert.match(ipcCommandsSource, /hideCommandPanel: 'hide_command_panel'/);
  assert.match(ipcEventsSource, /commandPanelClosed: 'command-panel:closed'/);
  assert.match(ipcSurfacesSource, /commandPanel: 'command-panel'/);
  assert.match(contractsSource, /COMMAND_PANEL/);
  assert.match(contractsSource, /SHOW_COMMAND_PANEL/);
  assert.match(contractsSource, /HIDE_COMMAND_PANEL/);
  assert.match(contractsSource, /COMMAND_PANEL_CLOSED/);
  assert.match(commandPanelWrapper, /invoke\(IPC_COMMANDS\.showCommandPanel/);
  assert.match(commandPanelWrapper, /invoke\(IPC_COMMANDS\.hideCommandPanel/);
  assert.doesNotMatch(commandPanelWrapper, /invoke\('show_command_panel'/);
  assert.doesNotMatch(commandPanelWrapper, /invoke\('hide_command_panel'/);
});

test('top bar command button is left of tray button and enforces popup exclusivity', () => {
  assert.match(topBarSource, /from '\.\.\/lib\/commandPanel'/);
  assert.match(topBarSource, /COMMAND_PANEL_CLOSED_EVENT/);
  assert.match(topBarSource, /class="command-control"[\s\S]*class="tray-control"/);
  assert.match(topBarSource, /class="command-button"[\s\S]*ariaControls=\{COMMAND_PANEL_ID\}/);
  assert.match(topBarSource, /ariaLabel="Open quick commands"/);
  assert.match(topBarSource, /await closePanel\(\);[\s\S]*await closeAudioPanel\(\);[\s\S]*await closeTrayPanel\(\);[\s\S]*await showCommandPanel\(\{/);
  assert.match(topBarSource, /if \(commandOpen && \(!target \|\| !commandControl\?\.contains\(target\)\)\) \{[\s\S]*void closeCommandPanel\(\);/);
  assert.match(topBarSource, /(?:void listen|registerAsyncUnlistener\(listen)\(COMMAND_PANEL_CLOSED_EVENT, \(\) => \{[\s\S]*commandOpen = false;/);
  assert.match(topBarCss, /\.top-bar \.command-button \{/);
  assert.match(topBarSource, /<span class="command-glyph" aria-hidden="true">⌘<\/span>/);
});

test('command panel surface includes compact list actions, resize controls, and command-block editor flow', () => {
  assert.match(commandPanelSource, /id="command-panel"[\s\S]*role="dialog"/);
  assert.match(commandPanelSource, /command-run-button/);
  assert.match(commandPanelSource, /Edit/);
  assert.match(commandPanelSource, /command-delete-button/);
  assert.match(commandPanelSource, /Label/);
  assert.match(commandPanelSource, /Mode/);
  assert.match(commandPanelSource, /Program/);
  assert.match(commandPanelSource, /Working directory/);
  assert.match(commandPanelSource, /Arguments \(one per line\)/);
  assert.match(commandPanelSource, /Commands \(one per line\)/);
  assert.match(commandPanelSource, /parseQuickCommandArgsTextarea/);
  assert.match(commandPanelSource, /formatQuickCommandArgsTextarea/);
  assert.match(commandPanelSource, /parseQuickCommandCommandsTextarea/);
  assert.match(commandPanelSource, /formatQuickCommandCommandsTextarea/);
  assert.match(commandPanelSource, /saveQuickCommandsSettings/);
  assert.match(commandPanelSource, /runQuickCommand/);
  assert.match(commandPanelSource, /listQuickCommandHistory/);
  assert.match(commandPanelSource, /on:contextmenu/);
  assert.match(commandPanelSource, /openKeyboardContextMenu/);
  assert.match(commandPanelSource, /on:keydown=\{dismissContextMenuOnEscape\}/);
  assert.match(commandPanelSource, /View output history/);
  assert.match(commandPanelSource, /Output stays local in settings/);
  assert.match(commandPanelSource, /command-list-resize-grip/);
  assert.match(commandPanelSource, /loadHistory/);
  assert.match(commandPanelSource, /run\.running \? 'Running'/);
  assert.match(commandPanelSource, /command-history-run/);
  assert.match(commandPanelCss, /grid-template-columns: minmax\(8rem, var\(--command-list-width\)\)/);
  assert.match(commandPanelSource, /hideCommandPanel/);
  assert.match(commandPanelCss, /\.command-panel \{/);
});

test('command panel Rust placement anchors to right edge and clamps within top bar bounds', () => {
  assert.match(commandPanelRs, /anchors_command_panel_to_button_right_edge/);
  assert.match(commandPanelRs, /clamps_command_panel_inside_top_bar_edges/);
  assert.match(commandPanelRs, /COMMAND_PANEL_EDGE_PADDING_PHYSICAL/);
  assert.match(commandPanelRs, /COMMAND_PANEL_MARGIN_PHYSICAL/);
  assert.match(commandPanelRs, /emit_to\(TOP_BAR_LABEL, COMMAND_PANEL_CLOSED_EVENT/);
});
