import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { shouldAnimateTerminalCommand } from '../dist-tests/features/top-bar/topBarUxState.js';
import { MATERIAL_SYMBOL_ICON_NAMES } from '../dist-tests/components/icons/materialSymbolIcons.js';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

const app = read('src/App.svelte');
const surfaceLoader = read('src/lib/surfaceLoader.ts');
const topBar = read('src/components/TopBar.svelte');
const topBarCss = read('src/components/TopBar.css');
const materialSymbolIcon = read('src/components/icons/MaterialSymbolIcon.svelte');
const materialSymbolIcons = read('src/components/icons/materialSymbolIcons.ts');
const stackPopup = read('src/components/StackPopupSurface.svelte');
const terminalPanel = read('src/components/TerminalPanelSurface.svelte');
const terminalPanelCss = read('src/components/TerminalPanelSurface.css');
const bottomBar = read('src/components/BottomBar.svelte');
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

test('persistent terminal is its own shell surface and uses delayed first-open startup', () => {
  assert.match(app, /loadSurfaceComponent\(surface\)/);
  assert.match(surfaceLoader, /'terminal-panel': \(\) => import\('\.\.\/components\/TerminalPanelSurface\.svelte'\)/);
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
  assert.match(topBar, /class=\{`terminal-button\$\{terminalCompletionPending \? ' terminal-complete' : ''\}`/);
  assert.match(topBar, /toggleTerminalPanel\(event\.currentTarget\)/);
  assert.match(topBar, /class="terminal-control"[\s\S]*class="command-control"/);
  assert.match(topBar, /TOP_BAR_TERMINAL_ACTIVITY_EVENT = 'terminal-panel:activity'/);
  assert.match(topBar, /if \(event\.payload\?\.active === false\) \{[\s\S]*terminalCompletionPending = true;[\s\S]*playTerminalCompletionSound\(\)/);
  assert.match(topBar, /terminalCompletionPending = false;/);
  assert.match(topBar, /<MaterialSymbolIcon name="terminal" \/>/);
  assert.match(topBar, /<MaterialSymbolIcon name="code" \/>/);
  assert.match(topBarCss, /\.top-bar \.terminal-button/);
  assert.match(topBarCss, /\.top-bar \.terminal-button\.terminal-complete/);
  assert.match(terminalPanel, /notifyTopBarForSubmittedCommand/);
  assert.match(terminalPanel, /shouldAnimateTerminalCommand\(commandText\)/);
  assert.match(terminalPanel, /importantTerminalActivitySessions\.add\(sessionId\)/);
  assert.match(terminalPanel, /emitTopBarTerminalActivity\(sessionId, true\)/);
  assert.match(terminalPanel, /emitTopBarTerminalActivity\(sessionId, false, completed\)/);
  assert.match(terminalPanel, /listen<TerminalOutputPayload>\('stack-terminal:output'[\s\S]{0,120}notifyTopBarForImportantTerminalOutput\(event\.payload\.sessionId\)/);
  assert.match(terminalPanel, /marker\.kind === 'end'[\s\S]{0,180}clearImportantTerminalActivity\([^\n]+, true\)/);
  assert.doesNotMatch(terminalPanel, /listen<TerminalOutputPayload>\('stack-terminal:output'[\s\S]{0,180}emitTo\(TOP_BAR_EVENT_TARGET, TOP_BAR_TERMINAL_ACTIVITY_EVENT/);
  assert.match(terminalPanelBackend, /TERMINAL_PANEL_OPEN_EVENT: &str = "terminal-panel:open"/);
  assert.match(terminalPanelBackend, /emit_to\(TERMINAL_PANEL_LABEL, TERMINAL_PANEL_OPEN_EVENT/);
});

test('terminal top bar icon keeps completion state without prompt glyph text', () => {
  assert.match(topBar, /terminalCompletionPending \? ' terminal-complete' : ''/);
  assert.match(topBar, /<MaterialSymbolIcon name="terminal" \/>/);
  assert.doesNotMatch(topBar, /terminalActivityGlyph|terminalCompletionGlyph|terminal-glyph/);
  assert.match(topBarCss, /\.top-bar \.terminal-button\.terminal-complete \{[\s\S]*color: var\(--js-color-accent\);/);
});

test('terminal top bar animation is reserved for important submitted commands', () => {
  assert.equal(shouldAnimateTerminalCommand('ls'), false);
  assert.equal(shouldAnimateTerminalCommand('cat package.json'), false);
  assert.equal(shouldAnimateTerminalCommand('echo codex'), false);
  assert.equal(shouldAnimateTerminalCommand('mvn test'), true);
  assert.equal(shouldAnimateTerminalCommand('./mvnw clean install'), true);
  assert.equal(shouldAnimateTerminalCommand('codex'), true);
  assert.equal(shouldAnimateTerminalCommand('npx codex --prompt "review"'), true);
  assert.equal(shouldAnimateTerminalCommand('pi ask'), true);
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
  assert.doesNotMatch(terminalPanel, /void startTerminal\(\)/);
  assert.match(terminalPanel, /scheduleIdlePrewarm\(\)/);
  assert.match(terminalPanel, /TERMINAL_IDLE_PREWARM_DELAY_MS = 5_000/);
  assert.match(terminalPanel, /terminalStartPromise/);
  assert.match(terminalPanel, /startPersistentTerminal\(\)/);
  assert.match(terminalPanel, /TERMINAL_PANEL_OPEN_EVENT = 'terminal-panel:open'/);
  assert.match(terminalPanel, /listen\(TERMINAL_PANEL_OPEN_EVENT/);
  assert.match(terminalPanel, /window\.addEventListener\('focus', handlePanelOpen\)/);
  assert.match(terminalPanel, /function handlePanelOpen[\s\S]*startTerminal\('first-open'\)/);
  assert.match(terminalPanel, /function startTerminalOnce\(intent: TerminalStartupIntent\)[\s\S]*if \(intent !== 'idle-prewarm'\) ensureTerminalView\(\)/);
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
  assert.match(
    terminalPanel,
    /event\.ctrlKey && event\.key\.toLowerCase\(\) === 'v'\) \{\s*event\.preventDefault\(\);\s*event\.stopPropagation\(\);\s*void pasteClipboard\(\);\s*return false;/,
    'primary persistent Ctrl+V must suppress native browser\/xterm paste before JasonShell clipboard write'
  );
  assert.match(
    terminalPanel,
    /event\.ctrlKey && event\.key\.toLowerCase\(\) === 'v'\) \{ event\.preventDefault\(\); event\.stopPropagation\(\); void pasteClipboardForRuntime\(runtime\); return false; \}/,
    'split-pane Ctrl+V must suppress native paste and route through the pane runtime'
  );
  assert.match(terminalPanel, /async function pasteClipboardForRuntime\(runtime: TerminalPaneRuntime\)[\s\S]*await writeTerminalDataForRuntime\(runtime, text\)/);
  assert.doesNotMatch(terminalPanel, /pasteClipboardForRuntime\(runtime: TerminalPaneRuntime\)[\s\S]*await writeTerminalData\(text\)/);
  assert.match(terminalPanel, /import \{ buildTerminalTabTitle \} from '\.\.\/features\/terminal\/terminalTabTitle'/);
  assert.match(terminalPanel, /const manuallyRenamedTerminalSessions = new Set<string>\(\)/);
  assert.match(terminalPanel, /let terminalTitleStates = new Map<string, TerminalTitleState>\(\)/);
  assert.match(terminalPanel, /function rememberTerminalTitleStateForRuntime\(runtime: TerminalPaneRuntime\)[\s\S]*terminalTitleStates = new Map\(terminalTitleStates\)\.set\(runtime\.session\.sessionId/);
  assert.match(terminalPanel, /function rememberTerminalTitleOutput\(sessionId: string, output: string\)[\s\S]*recentOutputText: `\$\{previous\.recentOutputText \?\? ''\}\$\{stripTerminalAnsiControls\(output\)\}`\.slice\(-20000\)/);
  assert.match(terminalPanel, /rememberTerminalTitleOutput\(event\.payload\.sessionId, event\.payload\.text\)/);
  assert.match(terminalPanel, /manuallyRenamedTerminalSessions\.add\(renamed\.sessionId\)/);
  assert.match(terminalPanel, /function terminalManualTitle\(sessionId: string, title\?: string\)[\s\S]*manuallyRenamedTerminalSessions\.has\(sessionId\)[\s\S]*isDefaultTerminalProfileTitle\(title\) \? undefined : title/);
  assert.match(terminalPanel, /function isDefaultTerminalProfileTitle\(title: string\)[\s\S]*Windows Terminal[\s\S]*PowerShell[\s\S]*Git Bash/);
  assert.match(terminalPanel, /function terminalDisplayTitle\(terminalSession: StackTerminalSession\)[\s\S]*titleState = terminalTitleStates\.get\(terminalSession\.sessionId\)[\s\S]*manualTitle: terminalManualTitle\(terminalSession\.sessionId, terminalSession\.title\)/);
  assert.match(terminalPanel, /aria-label=\{`Switch to terminal session \$\{displayTitle\}`\}/);
  assert.match(terminalPanel, /<span>\{displayTitle\}<\/span>/);
  assert.match(terminalPanel, /aria-label=\{`Terminal pane \$\{displayTitle\}`\}/);
  assert.match(terminalPanel, /<div class="terminal-pane-chrome" title=\{displayTitle\}>/);
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

test('material symbol icon set keeps official 960-grid paths and equal inline sizing', () => {
  assert.deepEqual(MATERIAL_SYMBOL_ICON_NAMES, ['close', 'settings', 'search', 'folder', 'terminal', 'code', 'speaker', 'monitor_heart']);
  assert.match(materialSymbolIcons, /close:\s*'m256-200-56-56 224-224-224-224 56-56 224 224 224-224 56 56-224 224 224 224-56 56-224-224-224 224Z'/);
  assert.match(materialSymbolIcons, /search:\s*'M784-120 532-372q-30 24-69 38t-83 14q-109 0-184\.5-75\.5T120-580q0-109 75\.5-184\.5T380-840q109 0 184\.5 75\.5T640-580q0 44-14 83t-38 69l252 252-56 56ZM380-400q75 0 127\.5-52\.5T560-580q0-75-52\.5-127\.5T380-760q-75 0-127\.5 52\.5T200-580q0 75 52\.5 127\.5T380-400Z'/);
  assert.match(materialSymbolIcons, /settings:[\s\S]*'m370-80-16-128q-13-5-24\.5-12T307-235l-119 50L78-375l103-78q-1-7-1-13\.5v-27q0-6\.5 1-13\.5L78-585l110-190 119 50q11-8 23-15t24-12l16-128h220l16 128q13 5 24\.5 12t22\.5 15l119-50 110 190-103 78q1 7 1 13\.5v27q0 6\.5-2 13\.5l103 78-110 190-118-50q-11 8-23 15t-24 12L590-80H370Zm70-80h79l14-106q31-8 57\.5-23\.5T639-327l99 41 39-68-86-65q5-14 7-29\.5t2-31\.5q0-16-2-31\.5t-7-29\.5l86-65-39-68-99 42q-22-23-48\.5-38\.5T533-694l-13-106h-79l-14 106q-31 8-57\.5 23\.5T321-633l-99-41-39 68 86 64q-5 15-7 30t-2 32q0 16 2 31t7 30l-86 65 39 68 99-42q22 23 48\.5 38\.5T427-266l13 106Zm42-180q58 0 99-41t41-99q0-58-41-99t-99-41q-59 0-99\.5 41T342-480q0 58 40\.5 99t99\.5 41Zm-2-140Z'/);
  assert.match(materialSymbolIcons, /code: 'M320-240 80-480l240-240 57 57-184 184 183 183-56 56Zm320 0-57-57 184-184-183-183 56-56 240 240-240 240Z'/);
  assert.match(materialSymbolIcons, /speaker: 'M560-131v-82q90-26 145-100t55-168q0-94-55-168T560-749v-82q124 28 202 125\.5T840-481q0 127-78 224\.5T560-131ZM120-360v-240h160l200-200v640L280-360H120Zm440 40v-322q47 22 73\.5 66t26\.5 96q0 51-26\.5 94\.5T560-320ZM400-606l-86 86H200v80h114l86 86v-252ZM300-480Z'/);
  assert.match(materialSymbolIcons, /folder: 'M160-160q-33 0-56\.5-23\.5T80-240v-480q0-33 23\.5-56\.5T160-800h240l80 80h320q33 0 56\.5 23\.5T880-640v400q0 33-23\.5 56\.5T800-160H160Zm0-80h640v-400H447l-80-80H160v480Zm0 0v-480 480Z'/);
  assert.match(materialSymbolIcons, /terminal: 'M160-160q-33 0-56\.5-23\.5T80-240v-480q0-33 23\.5-56\.5T160-800h640q33 0 56\.5 23\.5T880-720v480q0 33-23\.5 56\.5T800-160H160Zm0-80h640v-400H160v400Zm140-40-56-56 103-104-104-104 57-56 160 160-160 160Zm180 0v-80h240v80H480Z'/);
  assert.match(materialSymbolIcons, /monitor_heart: 'M80-600v-120q0-33 23\.5-56\.5T160-800h640q33 0 56\.5 23\.5T880-720v120h-80v-120H160v120H80Zm80 440q-33 0-56\.5-23\.5T80-240v-120h80v120h640v-120h80v120q0 33-23\.5 56\.5T800-160H160Zm261-125\.5q10-5\.5 15-16\.5l124-248 44 88q5 11 15 16\.5t21 5\.5h240v-80H665l-69-138q-5-11-15-15\.5t-21-4\.5q-11 0-21 4\.5T524-658L400-410l-44-88q-5-11-15-16\.5t-21-5\.5H80v80h215l69 138q5 11 15 16\.5t21 5\.5q11 0 21-5\.5ZM480-480Z'/);
  assert.match(terminalPanel, /import MaterialSymbolIcon from '\.\/icons\/MaterialSymbolIcon\.svelte'/);
  assert.match(terminalPanel, /<MaterialSymbolIcon name="close" \/>/);
  assert.match(materialSymbolIcon, /viewBox="0 -960 960 960"/);
  assert.match(materialSymbolIcon, /fill="currentColor"/);
  assert.match(materialSymbolIcon, /display: inline-block;/);
  assert.match(materialSymbolIcon, /vertical-align: middle;/);
  assert.match(materialSymbolIcon, /width: 1rem;/);
  assert.match(materialSymbolIcon, /height: 1rem;/);
  assert.match(topBar, /<MaterialSymbolIcon name="settings" \/>/);
  assert.match(topBar, /<MaterialSymbolIcon name="folder" \/>/);
  assert.match(topBar, /<MaterialSymbolIcon name="terminal" \/>/);
  assert.match(topBar, /<MaterialSymbolIcon name="code" \/>/);
  assert.match(topBar, /<MaterialSymbolIcon name="speaker" \/>/);
  assert.match(bottomBar, /<MaterialSymbolIcon name="monitor_heart" \/>/);
  assert.match(topBarCss, /\.top-bar \.terminal-button\.terminal-complete \{[\s\S]*color: var\(--js-color-accent\);/);
});

test('terminal tabs are backend-session authoritative and are not capped at four', () => {
  const terminalBackend = read('src-tauri/src/stack_popup/terminal.rs');
  assert.doesNotMatch(terminalBackend, /MAX_STACK_TERMINAL_SESSIONS\b|limited to \{MAX_STACK_TERMINAL_SESSIONS\}|can_start_session/);
  assert.doesNotMatch(terminalPanel, /terminalSessions\.length < 4/);
  assert.match(terminalPanel, /canCreateSession:\s*!sessionCreationInFlight/);
  assert.match(terminalPanel, /let terminalSessionCreationInFlight = false/);
  assert.match(terminalPanel, /await refreshTerminalSessionList\(\);[\s\S]*currentVisibleTerminalTabs\(\)[\s\S]*startPersistentTerminal\(\)/);
  assert.match(terminalPanel, /let terminalTabSessionIds = new Set<string>\(\)/);
  assert.match(terminalPanel, /let paneOnlyTerminalSessionIds = new Set<string>\(\)/);
  assert.match(terminalPanel, /\$: visibleTerminalTabs = terminalSessions\.filter\(\(item\) => terminalTabSessionIds\.has\(item\.sessionId\) && !paneOnlyTerminalSessionIds\.has\(item\.sessionId\)\)/);
  assert.match(terminalPanel, /const backendSessions = await listStackTerminals\('terminal-panel'\)/);
  assert.match(terminalPanel, /const backendSessionIds = new Set\(backendSessions\.map\(\(item\) => item\.sessionId\)\)/);
  assert.doesNotMatch(terminalPanel, /terminalPanes\.length >= 2|terminalPanes\.length < 2|Math\.min\(terminalPanes\.length \+ 1, 2\)/);
  assert.doesNotMatch(terminalPanel, /removePaneRuntime\(removedPane\.paneId/);
});

test('terminal tab plus creates whole-page tabs while split right and down are separate toolbar buttons', () => {
  assert.match(terminalPanel, /class="terminal-tab-new"[\s\S]*aria-label="New terminal tab"[\s\S]*runTerminalAction\('newSession'\)/);
  assert.match(terminalPanel, /title="Split pane right"[\s\S]*aria-label="Split terminal pane right"[\s\S]*runTerminalAction\('splitVertical'\)/);
  assert.match(terminalPanel, /title="Split pane down"[\s\S]*aria-label="Split terminal pane down"[\s\S]*runTerminalAction\('splitHorizontal'\)/);
  assert.match(terminalPanel, /import \{[\s\S]*planActivateTerminalTabWorkbench[\s\S]*planCloseTerminalTabWorkbench[\s\S]*planCreateTerminalTabWorkbench[\s\S]*TerminalTabWorkbenchSummary[\s\S]*\} from '\.\.\/features\/terminal\/terminalWorkbenchState'/);
  assert.match(terminalPanel, /let terminalTabWorkbenches = new Map<string, TerminalTabWorkbenchSummary>\(\)/);
  assert.doesNotMatch(terminalPanel, /\$: activeTerminalTabSessionId = currentWorkbenchTabSessionId\(\)/);
  assert.match(terminalPanel, /function activateTerminalSession\(nextSession: StackTerminalSession\)[\s\S]*activateTerminalTabWorkbench\(nextSession\)/);
  assert.match(terminalPanel, /function detachPaneRuntimeViewForHiddenTab\(runtime: TerminalPaneRuntime\)[\s\S]*disposePaneRuntime\(runtime\)[\s\S]*runtime\.host = null[\s\S]*runtime\.replayedSessionOutput = false/);
  assert.doesNotMatch(terminalPanel, /function detachPaneRuntimeViewForHiddenTab[\s\S]{0,700}markPaneRuntimeDisposed/);
  assert.doesNotMatch(terminalPanel, /function detachPaneRuntimeViewForHiddenTab[\s\S]{0,700}stopStackTerminal/);
  assert.match(terminalPanel, /function activateTerminalTabWorkbench\(nextSession: StackTerminalSession\)[\s\S]*saveCurrentTerminalWorkbench\(\)[\s\S]*planActivateTerminalTabWorkbench\([\s\S]*detachVisibleTerminalWorkbenchViewsForHiddenTab\(\)[\s\S]*setTerminalPaneTree\(plan\.visibleTree\)[\s\S]*restoreVisibleTerminalWorkbenchRuntimes\(\)/);
  assert.doesNotMatch(terminalPanel, /function openSessionAsTab\(/);
  assert.match(terminalPanel, /function clearStoppedTerminalSessionState\(sessionId: string\)[\s\S]*forgetTerminalSessionOwnership\(sessionId\)[\s\S]*sessionReplayBuffers\.delete\(sessionId\)[\s\S]*renderedSequenceKeysBySession\.delete\(sessionId\)[\s\S]*terminalTitleStates\.delete\(sessionId\)/);
  assert.match(terminalPanel, /async function createTerminalSession\(\)[\s\S]*if \(terminalSessionCreationInFlight\) return;[\s\S]*terminalSessionCreationInFlight = true;[\s\S]*saveCurrentTerminalWorkbench\(tabCreationWorkbenchSessionId\)[\s\S]*startTerminalPanelSessionInActiveCwd\(\)[\s\S]*planCreateTerminalTabWorkbench\([\s\S]*replaceTerminalTabWorkbenches\(plan\.workbenches\)[\s\S]*detachVisibleTerminalWorkbenchViewsForHiddenTab\(\)[\s\S]*setTerminalPaneTree\(plan\.visibleTree\)[\s\S]*restoreVisibleTerminalWorkbenchRuntimes\(\)[\s\S]*terminalSessionCreationInFlight = false/);
  assert.doesNotMatch(terminalPanel, /async function createTerminalSession\(\)[\s\S]*shouldActivateNewTab/);
  assert.match(terminalPanel, /async function createSplitPaneSession\(direction: TerminalSplitDirection\)[\s\S]*const splitStartGeneration = terminalWorkbenchGeneration[\s\S]*startTerminalPanelSessionInActiveCwd\(\)[\s\S]*isSplitStartStale\([\s\S]*await stopStackTerminal\(nextSession\.sessionId\)[\s\S]*markTerminalSessionAsPaneOnly\(nextSession\.sessionId\)[\s\S]*setTerminalPaneTree\(splitPaneTreeAtLeaf\(terminalPaneTree, targetPaneId, pane, direction\)\)[\s\S]*ensureTerminalViewForPane\(runtime\)[\s\S]*await tick\(\)[\s\S]*ensureTerminalViewForPane\(runtime\)/);
  assert.match(terminalPanel, /\{#each visibleTerminalTabs as terminalSession, index \(terminalSession\.sessionId\)\}/);
  assert.match(terminalPanel, /function startTerminalPanelSessionInActiveCwd\(\)[\s\S]*startStackTerminal\(cwd, terminalPanelStartupProfile\(\), 'terminal-panel'\)/);
  assert.match(terminalPanel, /async function splitTerminal[\s\S]*orientation === 'vertical' \? 'right' : 'down'[\s\S]*createSplitPaneSession\(direction\)/);
  assert.doesNotMatch(terminalPanel, /async function splitTerminal[\s\S]{0,220}terminalPanes\.length < 2/);
  assert.doesNotMatch(terminalPanel, /<button type="button" title="New terminal session"/);
  assert.match(terminalPanelCss, /\.terminal-session-tabs \.terminal-tab-new/);
});

test('terminal recursive split panes keep a source-contract tree and pane focus does not collapse layout', () => {
  assert.match(terminalPanel, /type TerminalPaneTreeNode =/);
  assert.match(terminalPanel, /let terminalPaneTree: TerminalPaneTreeNode \| null = null/);
  assert.match(terminalPanel, /function flattenPaneTree\(node: TerminalPaneTreeNode \| null = terminalPaneTree\): TerminalPaneModel\[\]/);
  assert.match(terminalPanel, /function splitPaneTreeAtLeaf\([\s\S]*direction: TerminalSplitDirection[\s\S]*kind: 'split'[\s\S]*first:[\s\S]*second:/);
  assert.match(terminalPanel, /return \{\s*\.\.\.node,[\s\S]*first: splitPaneTreeAtLeaf\(node\.first, targetPaneId, nextPane, direction\),[\s\S]*second: splitPaneTreeAtLeaf\(node\.second, targetPaneId, nextPane, direction\)[\s\S]*\};/);
  assert.match(terminalPanel, /setTerminalPaneTree\(splitPaneTreeAtLeaf\(terminalPaneTree, targetPaneId, pane, direction\)\)/);
  assert.match(terminalPanel, /async function splitTerminal[\s\S]*await createSplitPaneSession\(direction\)/);
  assert.doesNotMatch(terminalPanel, /async function splitTerminal[\s\S]{0,260}terminalPanes\.length < 2/);
  assert.doesNotMatch(terminalPanel, /async function createSplitPaneSession[\s\S]{0,520}ensurePrimaryPaneForSession\(/);
  assert.match(terminalPanel, /\{#snippet renderPaneTree\(node: TerminalPaneTreeNode\)\}[\s\S]*class="terminal-pane-split"[\s\S]*data-split-direction=\{node\.direction\}[\s\S]*@render renderPaneTree\(node\.first\)[\s\S]*@render renderPaneTree\(node\.second\)/);
  const focusNextPaneBody = terminalPanel.match(/function focusNextPane\(direction = 1\) \{[\s\S]*?\n  \}/)?.[0] ?? '';
  assert.match(focusNextPaneBody, /activatePane\(pane\.paneId\)[\s\S]*focusTerminal\(\)/);
  assert.doesNotMatch(focusNextPaneBody, /activateTerminalSession\(/);
  assert.match(terminalPanel, /async function splitTerminal[\s\S]*await startTerminal\('user-action'\);[\s\S]*await createSplitPaneSession\(direction\)/);
  assert.match(terminalPanelCss, /\.terminal-pane-split\[data-split-direction="right"\][\s\S]*flex-direction:\s*row;/);
  assert.match(terminalPanelCss, /\.terminal-pane-split\[data-split-direction="down"\][\s\S]*flex-direction:\s*column;/);
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
  assert.match(terminalPanel, /if \(!terminalPanes\.length\) \{[\s\S]*const nextSession = currentVisibleTerminalTabs\(\)\.find\(\(item\) => item\.running\) \?\? currentVisibleTerminalTabs\(\)\[0\] \?\? null;[\s\S]*activateTerminalTabWorkbench\(nextSession\)/);
  assert.doesNotMatch(terminalPanel, /async function closeVisibleTerminalWorkbench\(/);
  assert.match(terminalPanel, /async function closeTerminalSessionTab\(sessionId: string\)[\s\S]*planCloseTerminalTabWorkbench\([\s\S]*replaceTerminalTabWorkbenches\(plan\.workbenches\)[\s\S]*if \(closingActiveTab\) \{[\s\S]*setTerminalPaneTree\(plan\.visibleTree, true, false\)[\s\S]*restoreVisibleTerminalWorkbenchRuntimes\(\)[\s\S]*stopAndForgetTerminalSessionsInBackground\(plan\.stopBackendSessionIds\.length \? plan\.stopBackendSessionIds : \[sessionId\]\)/);
  assert.doesNotMatch(terminalPanel, /async function closeTerminalSessionTab\(sessionId: string\)[\s\S]{0,900}await stopStackTerminal/);
  assert.match(terminalPanel, /function stopAndForgetTerminalSessionsInBackground\(sessionIds: Iterable<string>\)[\s\S]*terminalSessions = terminalSessions\.filter\(\(item\) => !stoppedSessionIds\.has\(item\.sessionId\)\)[\s\S]*clearStoppedTerminalSessionState\(stoppedSessionId\)[\s\S]*void Promise\.all\(\[\.\.\.stoppedSessionIds\]\.map[\s\S]*stopStackTerminal\(stoppedSessionId\)/);
  assert.match(terminalPanel, /async function stopTerminal\(\)[\s\S]*if \(terminalTabSessionIds\.has\(sessionId\)\) \{[\s\S]*await closeTerminalSessionTab\(sessionId\)/);
  assert.match(terminalPanel, /async function restartTerminal\(\)[\s\S]*const clearOldStoppedSession = \(\) => \{[\s\S]*clearStoppedTerminalSessionState\(oldSession\)[\s\S]*await stopStackTerminal\(oldSession\)[\s\S]*clearOldStoppedSession\(\)/);
  assert.match(terminalPanel, /async function restartTerminal\(\)[\s\S]*Persistent terminal stale restart cleanup unavailable[\s\S]*clearStoppedTerminalSessionState\(nextSession\.sessionId\)/);
  assert.doesNotMatch(terminalPanel, /if \(!terminalPanes\.length\) \{[\s\S]{0,160}await createTerminalSession\(\)/);
});

test('terminal split and tab lifecycle guards stale runtimes and DOM identity', () => {
  assert.match(terminalPanel, /runtimeId: string/);
  assert.match(terminalPanel, /disposed: boolean/);
  assert.match(terminalPanel, /let terminalWorkbenchGeneration = 0/);
  assert.match(terminalPanel, /function nextTerminalRuntimeId\(\)/);
  assert.match(terminalPanel, /function isRuntimeCurrent\(runtime: TerminalPaneRuntime\)[\s\S]*runtime\.disposed[\s\S]*runtimeMatchesCurrentPane\(runtime\)/);
  assert.match(terminalPanel, /function commitRuntime\(runtime: TerminalPaneRuntime\)[\s\S]*if \(!isRuntimeCurrent\(runtime\)\) return false/);
  assert.match(terminalPanel, /function markPaneRuntimeDisposed\(runtime: TerminalPaneRuntime\)[\s\S]*runtime\.disposed = true/);
  assert.match(terminalPanel, /async function createSplitPaneSession\(direction: TerminalSplitDirection\)[\s\S]*const splitStartGeneration = terminalWorkbenchGeneration[\s\S]*if \(isSplitStartStale\([\s\S]*await stopStackTerminal\(nextSession\.sessionId\)/);
  assert.match(terminalPanel, /function isSplitStartStale\([\s\S]*splitStartGeneration !== terminalWorkbenchGeneration[\s\S]*currentWorkbenchTabSessionId\(\) !== splitStartTabSessionId/);
  assert.match(terminalPanel, /function attachPaneHost\(node: HTMLDivElement, pane: TerminalPaneModel\)[\s\S]*runtime\.session\.sessionId !== pane\.sessionId/);
  assert.match(terminalPanel, /function bindPaneHost\(node: HTMLDivElement, pane: TerminalPaneModel\)[\s\S]*let boundRuntimeId: string \| null = null[\s\S]*runtime\.runtimeId === boundRuntimeId/);
  assert.match(terminalPanel, /function writeTerminalOutputForRuntime\(runtime: TerminalPaneRuntime, output: string\)[\s\S]*const hasAttachedTerminal = runtimeHasAttachedTerminal\(runtime\)[\s\S]*hasVisibleOutput && hasAttachedTerminal[\s\S]*hasVisibleOutput && !hasAttachedTerminal[\s\S]*Attaching terminal view/);
  assert.match(terminalPanel, /\{#key node\.splitId\}[\s\S]*class="terminal-pane-split"/);
  assert.match(terminalPanel, /\{#key paneDomKey\(pane\)\}[\s\S]*use:bindPaneHost=\{pane\}/);
});

test('terminal tabs replay retained output and remember hidden-tab chunks', () => {
  assert.match(terminalPanel, /let sessionReplayBuffers = new Map<string, string>\(\)/);
  assert.match(terminalPanel, /let renderedSequenceKeysBySession = new Map<string, Set<string>>\(\)/);
  assert.match(terminalPanel, /if \(!runtime\) \{[\s\S]*rememberTerminalChunkForSession\(event\.payload\)[\s\S]*return;[\s\S]*\}/);
  assert.match(terminalPanel, /function rememberTerminalChunkForSession\(chunk: StackTerminalOutputChunk\)[\s\S]*appendSessionReplayBuffer\(chunk\.sessionId, chunk\.text\)/);
  assert.match(terminalPanel, /function appendSessionReplayBuffer\(sessionId: string, output: string\)[\s\S]*\.slice\(-262144\)/);
  assert.match(terminalPanel, /replayedSessionOutput: boolean/);
  assert.match(terminalPanel, /replayedSessionOutput: false/);
  assert.match(terminalPanel, /function runtimeHasAttachedTerminal\(runtime: TerminalPaneRuntime\)[\s\S]*runtime\.host\?\.contains\(element\)/);
  assert.match(terminalPanel, /function replayTerminalSessionOutput\(runtime: TerminalPaneRuntime\)[\s\S]*!runtimeHasAttachedTerminal\(runtime\)[\s\S]*runtime\.terminal\?\.write\(replay\)/);
  assert.match(terminalPanel, /function ensureTerminalViewForPane\(runtime: TerminalPaneRuntime\)[\s\S]*runtime\.terminal\?\.element && runtime\.host\.contains\(runtime\.terminal\.element\)[\s\S]*return;[\s\S]*disposePaneRuntime\(runtime\)/);
  assert.match(terminalPanel, /function disposePaneRuntime\(runtime: TerminalPaneRuntime\)[\s\S]*const shouldReplayRetainedOutputOnReattach = !runtime\.disposed && Boolean\(runtime\.terminal\);[\s\S]*runtime\.terminal\?\.dispose\(\);[\s\S]*if \(shouldReplayRetainedOutputOnReattach\) runtime\.replayedSessionOutput = false;/);
  assert.doesNotMatch(terminalPanel, /function disposePaneRuntime\(runtime: TerminalPaneRuntime\)[\s\S]*renderedSequences\s*=\s*new Set/);
  assert.match(terminalPanel, /function attachPaneHost\(node: HTMLDivElement, pane: TerminalPaneModel\)[\s\S]*ensureTerminalViewForPane\(runtime\);[\s\S]*replayTerminalSessionOutput\(runtime\);[\s\S]*scheduleFitForRuntime\(runtime\)/);
  assert.match(terminalPanel, /update\(nextPane: TerminalPaneModel\)[\s\S]*previousPane\.sessionId !== nextPane\.sessionId[\s\S]*attachPaneHost\(node, boundPane\)/);
  assert.match(terminalPanel, /\{#snippet renderPaneTree\(node: TerminalPaneTreeNode\)\}/);
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
  assert.match(terminalPanel, /function splitTerminal\(orientation: Exclude<TerminalSplitOrientation, 'single' \| 'mixed'>\)/);
  assert.match(terminalPanel, /use:bindPaneHost=\{pane\}/);
  assert.match(terminalPanel, /class="terminal-pane-tree"/);
  assert.match(terminalPanel, /function closeTerminalSessionTab\(sessionId: string\)/);
  assert.doesNotMatch(terminalPanel, /class="terminal-split-grid"/);
  assert.doesNotMatch(terminalPanel, /class="terminal-pane-title"/);
  assert.match(terminalPanelCss, /\.terminal-pane-tree,/);
  assert.match(terminalPanelCss, /\.terminal-pane-split/);
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
