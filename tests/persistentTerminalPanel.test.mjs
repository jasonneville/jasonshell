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
  assert.match(capability, /list_stack_terminals/);
  assert.match(capability, /rename_stack_terminal/);
  assert.match(capability, /stop_terminal_panel_sessions/);
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

test('Stack Browser no longer owns the visible terminal panel xterm internals', () => {
  assert.doesNotMatch(stackPopup, /class="stack-view-toggle"/);
  assert.doesNotMatch(stackPopup, /CLI<\/MeltActionButton>/);
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
  assert.match(terminalPanel, /visibleResizePromise = resizeAllVisiblePanes\(\)/);
  assert.match(terminalPanel, /window\.setTimeout\(\(\) => scheduleFit\(\), 60\)/);
  assert.match(terminalPanel, /function ensureVisibleResizeBeforeInput/);
  assert.match(terminalPanel, /await ensureVisibleResizeBeforeInput\(\);[\s\S]{0,120}writeStackTerminal\(sessionId, data\)/);
  assert.match(terminalPanel, /terminal-panel-status/);
  assert.match(terminalPanel, /role=\{paneRuntime\.lifecycle === 'failed' \? 'alert' : 'status'\}/);
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
  assert.match(terminalPanel, /event\.preventDefault\(\);\s*\r?\n\s*event\.stopPropagation\(\);\s*\r?\n\s*void copySelection\(\)/);
  assert.match(terminalPanel, /function isTerminalFontZoomKey\(event: KeyboardEvent\)[\s\S]*event\.key === '-' && !event\.shiftKey[\s\S]*event\.key === '\+' \|\| event\.key === '='/);
  assert.match(terminalPanel, /function handleTerminalFontZoomWheel\(event: WheelEvent\)/);
  assert.match(terminalPanel, /function setTerminalFontSize\(nextSize: number\)/);
  assert.match(terminalPanel, /const clamped = clampTerminalFontSize\(nextSize\)/);
  assert.match(terminalPanel, /event\.preventDefault\(\);\s*\r?\n\s*event\.stopPropagation\(\);\s*\r?\n\s*zoomTerminalFont\(event\.key === '-' \? -1 : 1\)/);
  assert.match(terminalPanel, /function applyTerminalFontSizeToXterm\(xterm: Terminal\)/);
  assert.match(terminalPanel, /xterm\.options\.fontSize = terminalFontSize/);
  assert.doesNotMatch(terminalPanel, /xterm\.refresh\(0, Math\.max\(0, xterm\.rows - 1\)\)/);
  assert.match(terminalPanel, /if \(runtime\.terminal\) applyTerminalFontSizeToXterm\(runtime\.terminal\)/);
  assert.match(terminalPanel, /scheduleFitForRuntime\(runtime\)/);
  assert.match(terminalPanel, /void resizeAllVisiblePanes\(\)/);
  assert.match(terminalPanel, /on:wheel\|capture\|nonpassive=\{handleTerminalFontZoomWheel\}/);
  assert.match(terminalPanel, /event\.preventDefault\(\);\s*\r?\n\s*event\.stopImmediatePropagation\?\.\(\);\s*\r?\n\s*event\.stopPropagation\(\);/);
  assert.doesNotMatch(terminalPanel, /slice\(0, midpoint\)|normalizeTerminalClipboardSelection/);
  assert.match(terminalPanel, /navigator\.clipboard\?\.writeText\(selection\)/);
  assert.match(terminalPanel, /navigator\.clipboard\?\.readText\(\)/);
  assert.match(terminalPanel, /function handleTerminalMouseDown\(event: MouseEvent\)/);
  assert.match(terminalPanel, /event\.detail < 3/);
  assert.match(terminalPanel, /terminal\.select\(startColumn, row, currentInputText\.length\)/);
  assert.match(terminalPanel, /currentInputSelectionActive = true/);
  assert.match(terminalPanel, /function deleteSelectedCurrentInput\(\)/);
  assert.match(terminalPanel, /'\\u007f'\.repeat\(length\)/);
  assert.match(terminalPanel, /event\.key === 'Backspace' \|\| event\.key === 'Delete'/);
  assert.match(terminalPanel, /on:mousedown\|capture=\{\(event\) => \{ activatePane\(pane\.paneId\); handleTerminalMouseDown\(event\); \}\}/);
  assert.match(terminalPanel, /on:contextmenu=\{\(event\) => \{ activatePane\(pane\.paneId\); openTerminalContextMenu\(event\); \}\}/);
  assert.match(terminalPanel, /class="terminal-panel-context-menu"/);
  assert.match(terminalPanel, /fontFamily: TERMINAL_PANEL_FONT_FAMILY/);
  assert.match(terminalPanel, /const TERMINAL_PANEL_DEFAULT_FONT_SIZE = 13/);
  assert.match(terminalPanel, /fontSize: terminalFontSize/);
  assert.match(terminalPanel, /lineHeight: 1\.25/);
  assert.match(terminalPanel, /scrollback: 8000/);
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
  assert.match(terminalPanelCss, /height: 1px !important;/);
  assert.match(terminalPanelCss, /\.terminal-panel-output :global\(\.xterm-rows\)/);
});

test('terminal tabs are backend-session authoritative and are not capped at four', () => {
  const terminalBackend = read('src-tauri/src/stack_popup/terminal.rs');
  assert.doesNotMatch(terminalBackend, /MAX_STACK_TERMINAL_SESSIONS\b|limited to \{MAX_STACK_TERMINAL_SESSIONS\}|can_start_session/);
  assert.doesNotMatch(terminalPanel, /terminalSessions\.length < 4/);
  assert.match(terminalPanel, /canCreateSession:\s*true/);
  assert.match(terminalPanel, /await refreshTerminalSessionList\(\);[\s\S]*terminalSessions\.find\(\(candidate\) => candidate\.running\)[\s\S]*startPersistentTerminal\(\)/);
  assert.match(terminalPanel, /const backendSessions = await listStackTerminals\('terminal-panel'\)/);
  assert.match(terminalPanel, /const backendSessionIds = new Set\(backendSessions\.map\(\(item\) => item\.sessionId\)\)/);
  assert.match(terminalPanel, /removePaneRuntime\(removedPane\.paneId, false, true\)/);
  assert.doesNotMatch(terminalPanel, /removePaneRuntime\(removedPane\.paneId, true\)/);
});

test('terminal tab plus creates whole-page tabs while split is a separate toolbar button', () => {
  assert.match(terminalPanel, /class="terminal-tab-new"[\s\S]*aria-label="New terminal tab"[\s\S]*runTerminalAction\('newSession'\)/);
  assert.match(terminalPanel, /title="Split terminal pane"[\s\S]*aria-label="Split terminal pane"[\s\S]*runTerminalAction\('splitVertical'\)/);
  assert.match(terminalPanel, /function openSessionAsTab\(nextSession: StackTerminalSession\): TerminalPaneRuntime[\s\S]*removePaneRuntime\(pane\.paneId, false, true\)[\s\S]*splitOrientation = 'single'/);
  assert.match(terminalPanel, /function openSessionAsTab\(nextSession: StackTerminalSession\): TerminalPaneRuntime[\s\S]*replayTerminalSessionOutput\(runtime\)[\s\S]*startPollingForRuntime\(runtime\)[\s\S]*pollTerminalOutputForRuntime\(runtime\)/);
  assert.match(terminalPanel, /async function createTerminalSession\(\)[\s\S]*openSessionAsTab\(nextSession\)/);
  assert.match(terminalPanel, /async function createSplitPaneSession\(orientation: Exclude<TerminalSplitOrientation, 'single'>\)[\s\S]*ensurePrimaryPaneForSession\(nextSession\)/);
  assert.match(terminalPanel, /async function splitTerminal[\s\S]*createSplitPaneSession\(orientation\)/);
  assert.doesNotMatch(terminalPanel, /async function splitTerminal[\s\S]{0,180}createTerminalSession\(\)/);
  assert.doesNotMatch(terminalPanel, /<button type="button" title="New terminal session"/);
  assert.match(terminalPanelCss, /\.terminal-session-tabs \.terminal-tab-new/);
});

test('terminal tabs are horizontal rectangular tabs', () => {
  assert.match(terminalPanelCss, /\.terminal-panel-header \{[\s\S]*display:\s*grid;[\s\S]*grid-template-columns:\s*minmax\(0, 1fr\) max-content;/);
  assert.doesNotMatch(terminalPanelCss, /\.terminal-panel-header \{[\s\S]{0,260}flex-wrap:\s*wrap/);
  assert.doesNotMatch(terminalPanel, /class="terminal-panel-title"/);
  assert.doesNotMatch(terminalPanelCss, /\.terminal-panel-header div \{/);
  assert.doesNotMatch(terminalPanelCss, /\.terminal-panel-title \{/);
  assert.match(terminalPanelCss, /\.terminal-session-tabs \{[\s\S]*display:\s*flex;[\s\S]*flex-direction:\s*row;[\s\S]*flex-wrap:\s*nowrap;[\s\S]*justify-self:\s*stretch;[\s\S]*overflow-x:\s*auto;[\s\S]*width:\s*100%;/);
  assert.match(terminalPanelCss, /\.terminal-toolbar \{[\s\S]*flex-wrap:\s*nowrap;/);
  assert.match(terminalPanelCss, /\.terminal-tab-shell \{[\s\S]*border-radius:\s*0;/);
  assert.doesNotMatch(terminalPanelCss, /\.terminal-tab-shell \{[\s\S]{0,260}border-radius:\s*999px/);
});

test('terminal tab close lives in the header and replaces the status dot on hover', () => {
  assert.match(terminalPanel, /function closeTerminalSessionTab\(sessionId: string\)/);
  assert.match(terminalPanel, /class="terminal-tab-shell"[\s\S]*role="presentation"/);
  assert.match(terminalPanel, /class="terminal-tab-button"[\s\S]*role="tab"[\s\S]*activateTerminalSession\(terminalSession\)/);
  assert.match(terminalPanel, /class="terminal-tab-status"[\s\S]*terminalSession\.running \? '●' : '○'/);
  assert.match(terminalPanel, /class="terminal-tab-close"[\s\S]*aria-label=\{`Close terminal session/);
  assert.match(terminalPanel, /on:click\|stopPropagation=\{\(\) => void closeTerminalSessionTab\(terminalSession\.sessionId\)\}/);
  assert.doesNotMatch(terminalPanel, /class="terminal-pane-close"/);
  assert.match(terminalPanelCss, /\.terminal-tab-shell:hover \.terminal-tab-status[\s\S]*display:\s*none;/);
  assert.match(terminalPanelCss, /\.terminal-tab-shell:hover \.terminal-tab-close[\s\S]*display:\s*flex;/);
  assert.match(terminalPanelCss, /\.terminal-pane-chrome \{[\s\S]*grid-template-rows/);
  assert.doesNotMatch(terminalPanelCss, /\.terminal-pane-close/);
});

test('terminal tab close does not recreate a replacement session while other tabs exist', () => {
  assert.match(terminalPanel, /if \(!terminalPanes\.length\) \{[\s\S]*const nextSession = terminalSessions\.find\(\(item\) => item\.running\) \?\? terminalSessions\[0\] \?\? null;[\s\S]*openSessionAsTab\(nextSession\)/);
  assert.doesNotMatch(terminalPanel, /if \(!terminalPanes\.length\) \{[\s\S]{0,160}await createTerminalSession\(\)/);
});

test('terminal tabs replay retained output and remember hidden-tab chunks', () => {
  assert.match(terminalPanel, /let sessionReplayBuffers = new Map<string, string>\(\)/);
  assert.match(terminalPanel, /let renderedSequenceKeysBySession = new Map<string, Set<string>>\(\)/);
  assert.match(terminalPanel, /if \(!runtime\) \{[\s\S]*rememberTerminalChunkForSession\(event\.payload\)[\s\S]*return;[\s\S]*\}/);
  assert.match(terminalPanel, /function rememberTerminalChunkForSession\(chunk: StackTerminalOutputChunk\)[\s\S]*appendSessionReplayBuffer\(chunk\.sessionId, chunk\.text\)/);
  assert.match(terminalPanel, /function appendSessionReplayBuffer\(sessionId: string, output: string\)[\s\S]*\.slice\(-262144\)/);
  assert.match(terminalPanel, /replayedSessionOutput: boolean/);
  assert.match(terminalPanel, /replayedSessionOutput: false/);
  assert.match(terminalPanel, /function replayTerminalSessionOutput\(runtime: TerminalPaneRuntime\)[\s\S]*if \(runtime\.replayedSessionOutput \|\| !runtime\.terminal\) return;[\s\S]*runtime\.terminal\.write\(replay\)/);
  assert.match(terminalPanel, /function attachPaneHost\(node: HTMLDivElement, pane: TerminalPaneModel\)[\s\S]*ensureTerminalViewForPane\(runtime\);[\s\S]*replayTerminalSessionOutput\(runtime\);[\s\S]*scheduleFitForRuntime\(runtime\)/);
  assert.match(terminalPanel, /update\(nextPane: TerminalPaneModel\)[\s\S]*previousPane\.sessionId !== nextPane\.sessionId[\s\S]*attachPaneHost\(node, boundPane\)/);
  assert.match(terminalPanel, /\{#each terminalPanes as pane \(pane\.sessionId\)\}/);
  assert.match(terminalPanel, /renderedSequences: new Set<string>\(renderedSequenceKeysBySession\.get\(nextSession\.sessionId\) \?\? \[\]\)/);
});

test('phase 7 terminal panel owns real per-pane xterm runtimes and split resize', () => {
  assert.match(terminalPanel, /type TerminalPaneRuntime = \{/);
  assert.match(terminalPanel, /terminal: Terminal \| null/);
  assert.match(terminalPanel, /fitAddon: FitAddon \| null/);
  assert.match(terminalPanel, /searchAddon: SearchAddon \| null/);
  assert.match(terminalPanel, /session: StackTerminalSession/);
  assert.match(terminalPanel, /pollInFlight: boolean/);
  assert.match(terminalPanel, /renderedSequences: Set<string>/);
  assert.match(terminalPanel, /let paneRuntimes = new Map<string, TerminalPaneRuntime>\(\)/);
  assert.match(terminalPanel, /function runtimeForSession\(sessionId: string\)/);
  assert.match(terminalPanel, /listen<TerminalOutputPayload>\('stack-terminal:output'[\s\S]*runtimeForSession\(event\.payload\.sessionId\)[\s\S]*writeTerminalChunkForRuntime/);
  assert.match(terminalPanel, /listen<TerminalCwdPayload>\('stack-terminal:cwd'[\s\S]*applyAuthoritativeTerminalCwdForRuntime/);
  assert.match(terminalPanel, /listen<TerminalClosedPayload>\('stack-terminal:closed'[\s\S]*stopPollingForRuntime/);
  assert.match(terminalPanel, /function ensureTerminalViewForPane\(runtime: TerminalPaneRuntime\)/);
  assert.match(terminalPanel, /paneTerminal\.loadAddon\(runtime\.fitAddon\)/);
  assert.match(terminalPanel, /paneTerminal\.loadAddon\(runtime\.searchAddon\)/);
  assert.match(terminalPanel, /runtime\.resizeObserver = new ResizeObserver\(\(\) => scheduleFitForRuntime\(runtime\)\)/);
  assert.match(terminalPanel, /function resizeAllVisiblePanes\(\)[\s\S]*paneRuntimes\.values\(\)[\s\S]*resizeTerminalToFitForRuntime/);
  assert.match(terminalPanel, /resizeStackTerminal\(sessionId, cols, rows, width, height\)/);
  assert.match(terminalPanel, /function splitTerminal\(orientation: Exclude<TerminalSplitOrientation, 'single'>\)/);
  assert.match(terminalPanel, /use:bindPaneHost=\{pane\}/);
  assert.match(terminalPanel, /class="terminal-pane-grid"/);
  assert.match(terminalPanel, /function closeTerminalSessionTab\(sessionId: string\)/);
  assert.doesNotMatch(terminalPanel, /class="terminal-split-grid"/);
  assert.doesNotMatch(terminalPanel, /class="terminal-pane-title"/);
  assert.match(terminalPanelCss, /\.terminal-pane-grid\[data-split-orientation="vertical"\]/);
  assert.match(terminalPanelCss, /\.terminal-pane-grid\[data-split-orientation="horizontal"\]/);
  assert.match(terminalPanelCss, /\.terminal-pane\.focused/);
  assert.match(terminalPanelCss, /box-sizing:\s*border-box;/);
  assert.match(terminalPanelCss, /overflow:\s*hidden;/);
  assert.doesNotMatch(terminalPanelCss, /clip-path: inset/);
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
